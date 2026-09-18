//! Sync-layer application of the non-registration events (research.md R3): pure in the
//! constitution's sense — `&mut World`, no `Gd`, no engine — and tested without Godot.

use bevy_ecs::prelude::{Messages, World};

use super::event::{DoorBodyEntered, InboundEvent, PlayerFx};
use super::index::EntityIndex;
use super::markers::{JumpQueued, PendingFx, PendingMouseLook, Remove, Trauma, Tuning};
use super::NodeHandles;
use crate::camera_noise_shake::model::{self, CameraShakeTuning};

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
}
