//! The bullet entity's pure gameplay systems (constitution 1.5.2 "ECS shape (v3)"; specs/013
//! research R7): `bullet_step` in `Gameplay` and `bullet_settle` in `GameplaySettle` of the FIXED
//! schedule, on `Simulates` entities only (the server, v2 `bullet.rs:89`). Every decision of v2's
//! `physics_process` (`:95-132`) lives here; every engine call is in `bullet/sync.rs`.

use bevy_ecs::prelude::*;

use super::pure::{self, BulletState};
use crate::ecs::event::RobotHitLocal;
use crate::ecs::markers::{BulletIntents, BulletStateC, Collided, FixedDelta, Simulates};
use crate::hittable::HitKind;

/// `Gameplay`: v2 `bullet.rs:98-107` — an already-exploded bullet is inactive for the run (`:98-100`,
/// so `move_bullet` skips it); otherwise `pure::step` counts `time_alive` down and expiry raises the
/// explode intent (`:103-107`). The intents are rebuilt from scratch every run.
pub fn bullet_step(
    dt: Res<FixedDelta>,
    mut bullets: Query<(&mut BulletStateC, &mut BulletIntents), With<Simulates>>,
) {
    let dt = dt.0 as f32;
    for (mut state, mut intents) in &mut bullets {
        *intents = BulletIntents { active: state.0 != BulletState::Exploded, ..Default::default() };
        if !intents.active {
            continue;
        }
        let (new_state, expired) = pure::step(state.0, dt);
        state.0 = new_state;
        intents.explode = expired;
    }
}

/// `GameplaySettle`: v2 `bullet.rs:114-131` on `move_bullet`'s answer — the hit target (`:117-121`),
/// the collision shape off (`:122`), the collision-driven explode ONLY when this run's expiry did
/// not already explode the bullet (backlog #13, `:123-127`), then `Exploded` unconditionally
/// (`:130`). A robot hit is also written as `RobotHitLocal` for the robot's `robot_hit_apply` of the
/// SAME run (spec option (B), research R4).
pub fn bullet_settle(
    mut bullets: Query<(&Collided, &mut BulletStateC, &mut BulletIntents), With<Simulates>>,
    mut hits: MessageWriter<RobotHitLocal>,
) {
    for (collided, mut state, mut intents) in &mut bullets {
        if !collided.collided {
            continue;
        }
        intents.hit = collided.hit;
        intents.disable_collision = true;
        intents.explode |= matches!(state.0, BulletState::Flying { .. });
        state.0 = BulletState::Exploded;
        if let Some(robot) = intents.hit.and_then(HitKind::robot_id) {
            hits.write(RobotHitLocal { robot });
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use godot::obj::InstanceId;

    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn world_with(state: BulletState, collided: Collided) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(FixedDelta(DT));
        world.insert_resource(Messages::<RobotHitLocal>::default());
        let e = world
            .spawn((BulletStateC(state), BulletIntents::default(), collided, Simulates))
            .id();
        (world, e)
    }

    fn intents(world: &World, e: Entity) -> BulletIntents {
        *world.get::<BulletIntents>(e).unwrap()
    }

    fn state(world: &World, e: Entity) -> BulletState {
        world.get::<BulletStateC>(e).unwrap().0
    }

    #[test]
    fn step_keeps_flying_and_counts_down() {
        let (mut world, e) = world_with(BulletState::Flying { time_alive: 5.0 }, Collided::default());
        world.run_system_once(bullet_step).unwrap();
        assert_eq!(state(&world, e), BulletState::Flying { time_alive: 5.0 - DT as f32 });
        let i = intents(&world, e);
        assert!(i.active);
        assert!(!i.explode);
    }

    #[test]
    fn expiry_sets_explode_intent() {
        let (mut world, e) = world_with(BulletState::Flying { time_alive: 0.01 }, Collided::default());
        world.run_system_once(bullet_step).unwrap();
        assert_eq!(state(&world, e), BulletState::Exploded);
        let i = intents(&world, e);
        assert!(i.active);
        assert!(i.explode);
    }

    #[test]
    fn collision_sets_hit_disables_and_explodes() {
        let robot = InstanceId::from_i64(7);
        let collided = Collided { hit: Some(HitKind::Robot(robot)), collided: true };
        let (mut world, e) = world_with(BulletState::Flying { time_alive: 4.0 }, collided);
        world.run_system_once(bullet_step).unwrap();
        world.run_system_once(bullet_settle).unwrap();
        let i = intents(&world, e);
        assert_eq!(i.hit, Some(HitKind::Robot(robot)));
        assert!(i.disable_collision);
        assert!(i.explode);
        assert_eq!(state(&world, e), BulletState::Exploded);
        let hits = world.resource::<Messages<RobotHitLocal>>();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits.iter_current_update_messages().next().unwrap().robot, robot);
    }

    #[test]
    fn expiry_and_collision_same_step_explode_once() {
        let collided = Collided { hit: None, collided: true };
        let (mut world, e) = world_with(BulletState::Flying { time_alive: 0.01 }, collided);
        world.run_system_once(bullet_step).unwrap();
        // Expiry already exploded the bullet this run …
        assert!(intents(&world, e).explode);
        assert_eq!(state(&world, e), BulletState::Exploded);
        world.run_system_once(bullet_settle).unwrap();
        // … so the settle sees `Exploded` and does not re-flag: still ONE explode intent
        // (backlog #13), the state unchanged, the collision still disables the shape.
        let i = intents(&world, e);
        assert!(i.explode);
        assert!(i.disable_collision);
        assert_eq!(i.hit, None);
        assert_eq!(state(&world, e), BulletState::Exploded);
        assert!(world.resource::<Messages<RobotHitLocal>>().is_empty());
    }

    #[test]
    fn exploded_bullet_is_inactive() {
        let (mut world, e) = world_with(BulletState::Exploded, Collided::default());
        world.run_system_once(bullet_step).unwrap();
        let i = intents(&world, e);
        assert!(!i.active);
        assert!(!i.explode);
        assert_eq!(state(&world, e), BulletState::Exploded);
    }
}
