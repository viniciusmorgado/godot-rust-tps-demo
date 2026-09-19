//! Sync-layer application of the non-registration events (research.md R3): pure in the
//! constitution's sense — `&mut World`, no `Gd`, no engine — and tested without Godot.

use bevy_ecs::prelude::{Messages, World};

use super::event::{DoorBodyEntered, InboundEvent, PlayerFx};
use super::index::EntityIndex;
use super::markers::{
    JumpQueued, PendingBulletFx, PendingFx, PendingMouseLook, PendingPartFx, PendingRobotFx,
    PendingRobotHits, Remove, RobotCountersC, RobotState, ShootRequested, Simulates, TrackedPlayer,
    Trauma, Tuning,
};
use super::NodeHandles;
use crate::camera_noise_shake::model::{self, CameraShakeTuning};
use crate::red_robot::model::{resume_approach_reset, RobotTuning};
use crate::red_robot::State;

/// Applies every non-`Register` event directly to the World (`Unregister`, `DoorBodyEntered`,
/// `BlastAnimationFinished`, and the player's `JumpPressed`/`MouseLook`/`AddTrauma`/`PlayerFx`
/// — specs/012 data-model.md "Drain arms"). `Register` needs the engine handles and is routed
/// by the driver to `ecs::apply_register`.
pub fn apply_non_register(world: &mut World, event: InboundEvent) {
    match event {
        InboundEvent::Register { .. } => {
            debug_assert!(false, "Register is routed to apply_register by the driver");
        }
        InboundEvent::Unregister { id } => {
            let removed = world.resource_mut::<EntityIndex>().unregister(id);
            if let Some(e) = removed {
                // The `Option`-returning accessor: tests need not insert `NodeHandles`.
                if let Some(mut handles) = world.get_non_send_mut::<NodeHandles>() {
                    handles.by_entity.remove(&e);
                }
                world.despawn(e);
            }
        }
        InboundEvent::DoorBodyEntered { id, is_player } => {
            // FR-016: the drain resolves the id; an unknown id is dropped silently.
            if let Some(entity) = world.resource::<EntityIndex>().entity(id) {
                world
                    .resource_mut::<Messages<DoorBodyEntered>>()
                    .write(DoorBodyEntered { entity, is_player });
            }
        }
        InboundEvent::BlastAnimationFinished { id } => {
            // FR-026: no decision to make — mark for removal; unknown id dropped.
            if let Some(entity) = world.resource::<EntityIndex>().entity(id) {
                world.entity_mut(entity).insert(Remove);
            }
        }
        // v2 `player_input.rs:163`: `self.jumping = true`, consumed and cleared by the next tick.
        InboundEvent::JumpPressed { root_id } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut queued) = world.get_mut::<JumpQueued>(entity)
            {
                queued.0 = true;
            }
        }
        // v2 `player_input.rs:151-154`: applied in arrival order by the frame run.
        InboundEvent::MouseLook { root_id, screen_relative } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut pending) = world.get_mut::<PendingMouseLook>(entity)
            {
                pending.0.push(screen_relative);
            }
        }
        // v2 `player.rs:163-164` → `camera_noise_shake.rs:64-67`: clamped at `max_trauma`.
        InboundEvent::AddTrauma { root_id, amount } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id) {
                add_trauma(world, entity, amount as f32);
            }
        }
        // v2 `player.rs:133-154` on a REMOTE peer; `Shoot`'s trauma (`:153`) is applied at drain
        // time so the same run's `shake_decide` sees it (research R7).
        InboundEvent::PlayerFx { root_id, fx } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id) {
                if let Some(mut pending) = world.get_mut::<PendingFx>(entity) {
                    pending.0.push(fx);
                }
                if fx == PlayerFx::Shoot {
                    add_trauma(world, entity, 0.35);
                }
            }
        }
        // ---- specs/013 (the enemy): data-model.md "Drain arms" ----
        // v2 `bullet.rs:137-146` on a REMOTE peer: applied by the frame `SyncOut`.
        InboundEvent::BulletFx { root_id, fx } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut pending) = world.get_mut::<PendingBulletFx>(entity)
            {
                pending.0.push(fx);
            }
        }
        // v2 `bullet.rs:150-153`: the method track fires on every peer, the server frees.
        InboundEvent::BulletDestroy { root_id } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && world.get::<Simulates>(entity).is_some()
            {
                world.entity_mut(entity).insert(Remove);
            }
        }
        // v2 `part.rs:194-215` on a REMOTE peer: applied by the frame `SyncOut`.
        InboundEvent::PartFx { root_id, fx } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut pending) = world.get_mut::<PendingPartFx>(entity)
            {
                pending.0.push(fx);
            }
        }
        // v2 `red_robot.rs:276-328` on a REMOTE peer (option (B): the local path is
        // `RobotHitLocal`); the client's frame run applies v2's client-side handler per hit.
        InboundEvent::RobotHit { root_id } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut pending) = world.get_mut::<PendingRobotHits>(entity)
            {
                pending.0 += 1;
            }
        }
        // v2 `red_robot.rs:330-333` on a REMOTE peer.
        InboundEvent::RobotFx { root_id, fx } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id)
                && let Some(mut pending) = world.get_mut::<PendingRobotFx>(entity)
            {
                pending.0.push(fx);
            }
        }
        // v2 `red_robot.rs:335-338`: consumed by the next fixed run's `robot_decide`.
        InboundEvent::ShootRequested { root_id } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id) {
                world.entity_mut(entity).insert(ShootRequested);
            }
        }
        // v2 `red_robot.rs:267-274`: the method track's reset, `model.rs:110-113`.
        InboundEvent::ResumeApproachRequested { root_id } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id) {
                reset_counters(world, entity);
                if let Some(mut state) = world.get_mut::<RobotState>(entity) {
                    state.0 = State::Approach;
                }
            }
        }
        // v2 `red_robot.rs:340-358`: the detection area. On entry the counters are reset FIRST
        // (backlog #31 CLOSED, spec FR-023 — the one sanctioned behavior change), then `Approach`.
        InboundEvent::RobotPlayerSeen { root_id, player } => {
            if let Some(entity) = world.resource::<EntityIndex>().entity(root_id) {
                if let Some(mut tracked) = world.get_mut::<TrackedPlayer>(entity) {
                    tracked.0 = player;
                }
                let next = if player.is_some() {
                    reset_counters(world, entity);
                    State::Approach
                } else {
                    State::Idle
                };
                if let Some(mut state) = world.get_mut::<RobotState>(entity) {
                    state.0 = next;
                }
            }
        }
    }
}

/// `resume_approach_reset` on the entity's counters (`aim_countdown` untouched, as v2's
/// `resume_approach` never reset it, `red_robot.rs:268-273`).
fn reset_counters(world: &mut World, entity: bevy_ecs::prelude::Entity) {
    let tuning = world.resource::<Tuning<RobotTuning>>().0;
    if let Some(mut counters) = world.get_mut::<RobotCountersC>(entity) {
        let (aim_preparing, shoot_countdown) = resume_approach_reset(&tuning);
        counters.0.aim_preparing = aim_preparing;
        counters.0.shoot_countdown = shoot_countdown;
    }
}

fn add_trauma(world: &mut World, entity: bevy_ecs::prelude::Entity, amount: f32) {
    let tuning = world.resource::<Tuning<CameraShakeTuning>>().0;
    if let Some(mut trauma) = world.get_mut::<Trauma>(entity) {
        trauma.0 = model::add_trauma(trauma.0, amount, &tuning);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::{Entity, With};
    use godot::obj::InstanceId;

    use super::*;

    fn world_with_index() -> World {
        let mut world = World::new();
        world.insert_resource(EntityIndex::default());
        world.insert_resource(Messages::<DoorBodyEntered>::default());
        world.insert_resource(Tuning(CameraShakeTuning::default()));
        world.insert_resource(Tuning(RobotTuning::default()));
        world
    }

    fn register(world: &mut World, id: i64) -> Entity {
        let entity = world.spawn_empty().id();
        world
            .resource_mut::<EntityIndex>()
            .register_if_absent(InstanceId::from_i64(id), || entity);
        entity
    }

    fn entities_marked_remove(world: &mut World) -> usize {
        world.query_filtered::<Entity, With<Remove>>().iter(world).count()
    }

    #[test]
    fn blast_animation_finished_marks_remove() {
        let mut world = world_with_index();
        let e = register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::BlastAnimationFinished { id: InstanceId::from_i64(7) });
        assert!(world.get::<Remove>(e).is_some());
    }

    #[test]
    fn blast_animation_finished_for_unknown_id_is_dropped() {
        let mut world = world_with_index();
        register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::BlastAnimationFinished { id: InstanceId::from_i64(99) });
        assert_eq!(entities_marked_remove(&mut world), 0);
    }

    #[test]
    fn unregister_despawns_and_drops_index_entry() {
        let mut world = world_with_index();
        let e = register(&mut world, 7);
        apply_non_register(&mut world, InboundEvent::Unregister { id: InstanceId::from_i64(7) });
        // Despawned: the entity is gone (`World::entities().len()` would also count bevy-internal
        // entities in 0.19, so liveness is what is asserted).
        assert!(world.get_entity(e).is_err());
        assert_eq!(world.resource::<EntityIndex>().entity(InstanceId::from_i64(7)), None);
    }

    // ---- The player's drain arms (specs/012 T007) ----

    use godot::builtin::Vector2;

    /// One player entity with the queued components and `Trauma(0.0)`, registered under id 7.
    fn player_world() -> (World, Entity) {
        let mut world = world_with_index();
        let entity = world
            .spawn((JumpQueued::default(), PendingMouseLook::default(), PendingFx::default(), Trauma(0.0)))
            .id();
        world
            .resource_mut::<EntityIndex>()
            .register_if_absent(InstanceId::from_i64(7), || entity);
        (world, entity)
    }

    fn root() -> InstanceId {
        InstanceId::from_i64(7)
    }

    #[test]
    fn jump_pressed_sets_jump_queued() {
        let (mut world, e) = player_world();
        apply_non_register(&mut world, InboundEvent::JumpPressed { root_id: root() });
        assert!(world.get::<JumpQueued>(e).unwrap().0);
    }

    #[test]
    fn mouse_look_is_queued_in_order() {
        let (mut world, e) = player_world();
        let a = Vector2::new(20.0, -10.0);
        let b = Vector2::new(-3.0, 4.0);
        apply_non_register(&mut world, InboundEvent::MouseLook { root_id: root(), screen_relative: a });
        apply_non_register(&mut world, InboundEvent::MouseLook { root_id: root(), screen_relative: b });
        assert_eq!(world.get::<PendingMouseLook>(e).unwrap().0, vec![a, b]);
    }

    #[test]
    fn add_trauma_clamps_at_max() {
        let (mut world, e) = player_world();
        apply_non_register(&mut world, InboundEvent::AddTrauma { root_id: root(), amount: 0.75 });
        assert_eq!(world.get::<Trauma>(e).unwrap().0, 0.75);
        apply_non_register(&mut world, InboundEvent::AddTrauma { root_id: root(), amount: 0.75 });
        // min(1.5, max_trauma = 1.2)
        assert_eq!(world.get::<Trauma>(e).unwrap().0, CameraShakeTuning::default().max_trauma);
        assert_eq!(world.get::<Trauma>(e).unwrap().0, 1.2);
    }

    #[test]
    fn player_fx_is_queued_and_shoot_adds_trauma_at_drain() {
        let (mut world, e) = player_world();
        apply_non_register(&mut world, InboundEvent::PlayerFx { root_id: root(), fx: PlayerFx::Jump });
        assert_eq!(world.get::<Trauma>(e).unwrap().0, 0.0);
        apply_non_register(&mut world, InboundEvent::PlayerFx { root_id: root(), fx: PlayerFx::Shoot });
        assert_eq!(world.get::<PendingFx>(e).unwrap().0, vec![PlayerFx::Jump, PlayerFx::Shoot]);
        assert_eq!(world.get::<Trauma>(e).unwrap().0, 0.35);
    }

    #[test]
    fn player_events_for_unknown_root_are_dropped() {
        let (mut world, e) = player_world();
        let unknown = InstanceId::from_i64(99);
        apply_non_register(&mut world, InboundEvent::JumpPressed { root_id: unknown });
        apply_non_register(
            &mut world,
            InboundEvent::MouseLook { root_id: unknown, screen_relative: Vector2::new(1.0, 1.0) },
        );
        apply_non_register(&mut world, InboundEvent::AddTrauma { root_id: unknown, amount: 0.75 });
        apply_non_register(&mut world, InboundEvent::PlayerFx { root_id: unknown, fx: PlayerFx::Shoot });
        assert!(!world.get::<JumpQueued>(e).unwrap().0);
        assert!(world.get::<PendingMouseLook>(e).unwrap().0.is_empty());
        assert!(world.get::<PendingFx>(e).unwrap().0.is_empty());
        assert_eq!(world.get::<Trauma>(e).unwrap().0, 0.0);
    }

    // ---- The enemy's drain arms (specs/013 T007) ----

    use super::super::event::{BulletFx, PartFx, RobotFx};
    use crate::red_robot::model::RobotCounters;

    fn stale_counters() -> RobotCountersC {
        RobotCountersC(RobotCounters { aim_preparing: 0.1, shoot_countdown: 2.0, aim_countdown: 0.3 })
    }

    /// One robot entity with the drain-touched components and stale counters, registered under id 7.
    fn robot_world() -> (World, Entity) {
        let mut world = world_with_index();
        let entity = world
            .spawn((
                RobotState(State::Idle),
                stale_counters(),
                TrackedPlayer(None),
                PendingRobotFx::default(),
                PendingRobotHits::default(),
            ))
            .id();
        world
            .resource_mut::<EntityIndex>()
            .register_if_absent(InstanceId::from_i64(7), || entity);
        (world, entity)
    }

    #[test]
    fn bullet_fx_explode_is_queued() {
        let mut world = world_with_index();
        let e = world.spawn(PendingBulletFx::default()).id();
        world.resource_mut::<EntityIndex>().register_if_absent(root(), || e);
        apply_non_register(&mut world, InboundEvent::BulletFx { root_id: root(), fx: BulletFx::Explode });
        assert_eq!(world.get::<PendingBulletFx>(e).unwrap().0, vec![BulletFx::Explode]);
    }

    #[test]
    fn bullet_destroy_marks_remove_only_on_simulates() {
        let mut world = world_with_index();
        let server = world.spawn(Simulates).id();
        let client = world.spawn_empty().id();
        world.resource_mut::<EntityIndex>().register_if_absent(InstanceId::from_i64(7), || server);
        world.resource_mut::<EntityIndex>().register_if_absent(InstanceId::from_i64(8), || client);
        apply_non_register(&mut world, InboundEvent::BulletDestroy { root_id: InstanceId::from_i64(7) });
        apply_non_register(&mut world, InboundEvent::BulletDestroy { root_id: InstanceId::from_i64(8) });
        assert!(world.get::<Remove>(server).is_some());
        assert!(world.get::<Remove>(client).is_none());
    }

    #[test]
    fn part_fx_destroy_is_queued() {
        let mut world = world_with_index();
        let e = world.spawn(PendingPartFx::default()).id();
        world.resource_mut::<EntityIndex>().register_if_absent(root(), || e);
        apply_non_register(&mut world, InboundEvent::PartFx { root_id: root(), fx: PartFx::Destroy });
        assert_eq!(world.get::<PendingPartFx>(e).unwrap().0, vec![PartFx::Destroy]);
    }

    #[test]
    fn robot_hit_is_counted_for_remote_peers() {
        let (mut world, e) = robot_world();
        apply_non_register(&mut world, InboundEvent::RobotHit { root_id: root() });
        apply_non_register(&mut world, InboundEvent::RobotHit { root_id: root() });
        assert_eq!(world.get::<PendingRobotHits>(e).unwrap().0, 2);
    }

    #[test]
    fn robot_fx_play_shoot_is_queued() {
        let (mut world, e) = robot_world();
        apply_non_register(&mut world, InboundEvent::RobotFx { root_id: root(), fx: RobotFx::PlayShoot });
        assert_eq!(world.get::<PendingRobotFx>(e).unwrap().0, vec![RobotFx::PlayShoot]);
    }

    #[test]
    fn shoot_requested_inserts_marker() {
        let (mut world, e) = robot_world();
        assert!(world.get::<ShootRequested>(e).is_none());
        apply_non_register(&mut world, InboundEvent::ShootRequested { root_id: root() });
        assert!(world.get::<ShootRequested>(e).is_some());
    }

    #[test]
    fn robot_player_seen_resets_counters_on_entry() {
        let tuning = RobotTuning::default();
        let (mut world, e) = robot_world();
        let player = InstanceId::from_i64(42);
        apply_non_register(&mut world, InboundEvent::RobotPlayerSeen { root_id: root(), player: Some(player) });
        assert_eq!(world.get::<RobotState>(e).unwrap().0, State::Approach);
        let counters = world.get::<RobotCountersC>(e).unwrap().0;
        assert_eq!(counters.aim_preparing, tuning.aim_prepare_time);
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
        assert_eq!(counters.aim_countdown, 0.3);
        assert_eq!(world.get::<TrackedPlayer>(e).unwrap().0, Some(player));

        // `resume_approach_requested_resets_state_and_counters` (T007): the method track's arm
        // uses the same reset formula (v2 red_robot.rs:268-273).
        world.entity_mut(e).insert((RobotState(State::Shooting), stale_counters()));
        apply_non_register(&mut world, InboundEvent::ResumeApproachRequested { root_id: root() });
        assert_eq!(world.get::<RobotState>(e).unwrap().0, State::Approach);
        let counters = world.get::<RobotCountersC>(e).unwrap().0;
        assert_eq!(counters.aim_preparing, tuning.aim_prepare_time);
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
        assert_eq!(counters.aim_countdown, 0.3);
    }

    #[test]
    fn robot_player_seen_none_sets_idle_without_reset() {
        let (mut world, e) = robot_world();
        world.entity_mut(e).insert((RobotState(State::Approach), TrackedPlayer(Some(InstanceId::from_i64(42)))));
        apply_non_register(&mut world, InboundEvent::RobotPlayerSeen { root_id: root(), player: None });
        assert_eq!(world.get::<RobotState>(e).unwrap().0, State::Idle);
        let counters = world.get::<RobotCountersC>(e).unwrap().0;
        assert_eq!(counters.aim_preparing, 0.1);
        assert_eq!(counters.shoot_countdown, 2.0);
        assert_eq!(counters.aim_countdown, 0.3);
        assert_eq!(world.get::<TrackedPlayer>(e).unwrap().0, None);
    }
}
