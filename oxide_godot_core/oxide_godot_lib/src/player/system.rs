//! The player's pure core as gameplay systems (constitution 1.5.1, Principle III; specs/012
//! research R6/R7): v2's `apply_input` steps (2)–(6) and (9) on the entity's components, calling
//! `player/model.rs` unchanged. No `Gd`, no `NonSend`, no engine.

use bevy_ecs::prelude::*;
use godot::builtin::Vector2;

use super::Animations;
use super::model::{self, AnimPlan, PlayerTuning};
use crate::ecs::markers::{
    AirborneTime, BodyState, FixedDelta, InputFrameC, Motion, Orientation, OrientTarget,
    ReplayState, RootMotion, Simulates, TickIntents, Tuning, Velocity,
};

/// `tick_decide`'s query (a `type` so clippy's `type_complexity` stays quiet).
type TickDecideQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static InputFrameC,
        &'static BodyState,
        &'static Orientation,
        &'static mut Motion,
        &'static mut AirborneTime,
        &'static mut TickIntents,
    ),
    With<Simulates>,
>;

/// `Phase::Gameplay` of the FIXED schedule: v2 steps (2), (3), (4) and the branch decision of
/// step (5) (`player.rs:226-258`, `:266`, `:274`, `:294-303`) → `TickIntents`, consumed by
/// `orient_and_anim`, `move_body` and `sync_out_player` in the same run.
pub fn tick_decide(
    tuning: Res<Tuning<PlayerTuning>>,
    dt: Res<FixedDelta>,
    mut players: TickDecideQuery,
) {
    let tuning = &tuning.0;
    let dt = dt.0 as f32;
    for (frame, body, _orientation, mut motion, mut airborne, mut intents) in &mut players {
        let frame = frame.0;

        // (2) motion lerp.
        motion.0 = model::lerp_motion(motion.0, frame.motion, dt, tuning);

        // (3) flattened camera axes.
        let (camera_x, camera_z) = model::flatten_camera_axes(frame.camera_rotation_basis);

        // (4) airborne step (`is_on_floor()` was read at `SyncIn`).
        let outcome = model::airborne_step(airborne.0, dt, body.on_floor, frame.jumping, tuning);
        airborne.0 = outcome.airborne_time;

        let mut next = TickIntents {
            land: outcome.land,
            jump: outcome.jump,
            jump_velocity_y: outcome.jump_velocity_y,
            ..TickIntents::default()
        };

        // (5) the branch decision.
        if outcome.on_air {
            // v2 wrote the jump velocity (`:240-244`) and re-read it (`:255`) before `anim_plan`:
            // the jump step plans from the POST-jump velocity.
            let velocity_y = outcome.jump_velocity_y.unwrap_or(body.velocity.y);
            next.plan = Some(model::anim_plan(true, velocity_y, false, motion.0, 0.0));
            // root motion is NOT re-read while airborne (v1's own field-persistence).
        } else if frame.aiming {
            next.orient = Some(OrientTarget::Camera(frame.camera_base_quaternion));
            next.plan = Some(model::anim_plan(false, 0.0, true, motion.0, frame.aim_rotation));
            next.read_root_motion = true;
            next.shoot = frame.shooting && body.cooldown_left == 0.0;
        } else {
            // Not in air or aiming, idle.
            next.orient = model::walk_target(camera_x, camera_z, motion.0).map(OrientTarget::Walk);
            next.plan = Some(model::anim_plan(false, 0.0, false, motion.0, 0.0));
            next.read_root_motion = true;
        }
        // `respawn` stays false until `tick_settle`.
        *intents = next;
    }
}

/// `Phase::GameplayIntegrate`: v2 step (6) (`player.rs:313-317`). The velocity in is the body's
/// with `y` replaced by the jump velocity when the step jumped (`:240-244` preceded `:313`).
pub fn tick_integrate(
    dt: Res<FixedDelta>,
    mut players: Query<
        (&BodyState, &RootMotion, &mut Orientation, &mut Velocity, &TickIntents),
        With<Simulates>,
    >,
) {
    let dt = dt.0 as f32;
    for (body, root_motion, mut orientation, mut velocity, intents) in &mut players {
        let mut velocity_in = body.velocity;
        if let Some(jump_velocity_y) = intents.jump_velocity_y {
            velocity_in.y = jump_velocity_y;
        }
        let (next_orientation, next_velocity) =
            model::integrate_root_motion(orientation.0, root_motion.0, dt, body.gravity, velocity_in);
        orientation.0 = next_orientation;
        velocity.0 = next_velocity;
    }
}

/// `Phase::GameplaySettle`: v2 step (9)'s decision (`player.rs:329`) on the post-move origin.
pub fn tick_settle(
    tuning: Res<Tuning<PlayerTuning>>,
    mut players: Query<(&BodyState, &mut TickIntents), With<Simulates>>,
) {
    for (body, mut intents) in &mut players {
        intents.respawn = model::should_respawn(body.origin_y, &tuning.0);
    }
}

/// The non-`Simulates` replay plan (v2 `player.rs:104-117`): the replicated `current_animation`
/// and `motion` back into an `AnimPlan`.
pub fn replay_plan(state: &ReplayState) -> AnimPlan {
    match state.current_animation {
        Animations::JumpUp => AnimPlan::JumpUp,
        Animations::JumpDown => AnimPlan::JumpDown,
        Animations::Strafe => AnimPlan::Strafe {
            aim_rotation: state.aim_rotation,
            blend_position: Vector2::new(state.motion.x, -state.motion.y),
        },
        Animations::Walk => AnimPlan::Walk {
            blend_position: Vector2::new(state.motion.length(), 0.0),
        },
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use godot::builtin::{Basis, Quaternion, Transform3D, Vector3};

    use super::*;
    use crate::player::model::InputFrame;

    const DT: f64 = 1.0 / 60.0;

    fn frame() -> InputFrame {
        InputFrame {
            motion: Vector2::ZERO,
            aiming: false,
            shooting: false,
            jumping: false,
            shoot_target: Vector3::ZERO,
            camera_rotation_basis: Basis::IDENTITY,
            camera_base_quaternion: Quaternion::new(0.0, 0.0, 0.0, 1.0),
            aim_rotation: 0.25,
        }
    }

    fn body() -> BodyState {
        BodyState {
            on_floor: true,
            velocity: Vector3::ZERO,
            gravity: Vector3::new(0.0, -9.8, 0.0),
            cooldown_left: 0.4,
            origin_y: 0.0,
        }
    }

    /// research R9's recipe: a bare World with the resources the systems need and one
    /// `Simulates` player.
    fn world_with(frame: InputFrame, body: BodyState, airborne: f32) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Tuning(PlayerTuning::default()));
        world.insert_resource(FixedDelta(DT));
        let player = world
            .spawn((
                InputFrameC(frame),
                body,
                Orientation(Transform3D::IDENTITY),
                RootMotion(Transform3D::IDENTITY),
                Motion(Vector2::ZERO),
                AirborneTime(airborne),
                TickIntents::default(),
                Velocity(Vector3::ZERO),
                Simulates,
            ))
            .id();
        (world, player)
    }

    fn decide(world: &mut World, player: Entity) -> TickIntents {
        world.run_system_once(tick_decide).unwrap();
        *world.get::<TickIntents>(player).unwrap()
    }

    #[test]
    fn walk_decides_walk_plan_and_walk_target() {
        let mut f = frame();
        f.motion = Vector2::new(0.0, -1.0);
        let (mut world, player) = world_with(f, body(), 0.0);
        let intents = decide(&mut world, player);
        let motion = world.get::<Motion>(player).unwrap().0;
        assert_eq!(intents.plan, Some(AnimPlan::Walk { blend_position: Vector2::new(motion.length(), 0.0) }));
        assert!(matches!(intents.orient, Some(OrientTarget::Walk(_))));
        assert!(intents.read_root_motion);
        assert!(!intents.shoot && !intents.land && !intents.jump && !intents.respawn);
    }

    #[test]
    fn aim_decides_strafe_plan_and_camera_orient() {
        let mut f = frame();
        f.aiming = true;
        let (mut world, player) = world_with(f, body(), 0.0);
        let intents = decide(&mut world, player);
        assert!(matches!(intents.plan, Some(AnimPlan::Strafe { aim_rotation, .. }) if aim_rotation == 0.25));
        assert_eq!(intents.orient, Some(OrientTarget::Camera(f.camera_base_quaternion)));
        assert!(intents.read_root_motion);
    }

    #[test]
    fn airborne_decides_jump_plan_and_skips_orient_and_root_motion() {
        let mut b = body();
        b.on_floor = false;
        b.velocity.y = -1.0;
        let (mut world, player) = world_with(frame(), b, 0.5);
        let intents = decide(&mut world, player);
        assert_eq!(intents.plan, Some(AnimPlan::JumpDown));
        assert_eq!(intents.orient, None);
        assert!(!intents.read_root_motion);

        b.velocity.y = 1.0;
        let (mut world, player) = world_with(frame(), b, 0.5);
        assert_eq!(decide(&mut world, player).plan, Some(AnimPlan::JumpUp));
    }

    #[test]
    fn jump_step_plans_jump_up_from_the_post_jump_velocity() {
        let mut f = frame();
        f.jumping = true;
        let (mut world, player) = world_with(f, body(), 0.0);
        let intents = decide(&mut world, player);
        assert_eq!(intents.jump_velocity_y, Some(PlayerTuning::default().jump_speed));
        assert!(intents.jump);
        assert_eq!(intents.plan, Some(AnimPlan::JumpUp));
        assert_ne!(intents.plan, Some(AnimPlan::JumpDown));
    }

    #[test]
    fn land_and_jump_intents_can_both_fire() {
        let mut f = frame();
        f.jumping = true;
        let (mut world, player) = world_with(f, body(), 0.6);
        let intents = decide(&mut world, player);
        assert!(intents.land);
        assert!(intents.jump);
    }

    #[test]
    fn shoot_fires_only_when_aiming_and_cooldown_zero() {
        let mut f = frame();
        f.aiming = true;
        f.shooting = true;
        let mut b = body();
        b.cooldown_left = 0.1;
        let (mut world, player) = world_with(f, b, 0.0);
        assert!(!decide(&mut world, player).shoot);

        b.cooldown_left = 0.0;
        let (mut world, player) = world_with(f, b, 0.0);
        assert!(decide(&mut world, player).shoot);
    }

    #[test]
    fn integrate_updates_orientation_and_velocity() {
        let mut b = body();
        b.velocity = Vector3::new(0.5, -0.2, 0.1);
        let (mut world, player) = world_with(frame(), b, 0.0);
        let root_motion = Transform3D::new(Basis::IDENTITY, Vector3::new(0.0, 0.0, 0.02));
        world.entity_mut(player).insert(RootMotion(root_motion));
        world.entity_mut(player).insert(TickIntents { jump_velocity_y: Some(5.0), ..TickIntents::default() });
        world.run_system_once(tick_integrate).unwrap();

        let mut velocity_in = b.velocity;
        velocity_in.y = 5.0;
        let (expected_orientation, expected_velocity) =
            model::integrate_root_motion(Transform3D::IDENTITY, root_motion, DT as f32, b.gravity, velocity_in);
        assert_eq!(world.get::<Orientation>(player).unwrap().0, expected_orientation);
        assert_eq!(world.get::<Velocity>(player).unwrap().0, expected_velocity);
    }

    #[test]
    fn settle_flags_respawn_below_threshold() {
        let mut b = body();
        b.origin_y = -41.0;
        let (mut world, player) = world_with(frame(), b, 0.0);
        world.run_system_once(tick_settle).unwrap();
        assert!(world.get::<TickIntents>(player).unwrap().respawn);

        b.origin_y = -39.0;
        let (mut world, player) = world_with(frame(), b, 0.0);
        world.run_system_once(tick_settle).unwrap();
        assert!(!world.get::<TickIntents>(player).unwrap().respawn);
    }

    #[test]
    fn replay_builds_v2_plans_for_all_four_animations() {
        let state = |current_animation| ReplayState {
            current_animation,
            motion: Vector2::new(0.3, -0.4),
            aim_rotation: 0.7,
        };
        assert_eq!(replay_plan(&state(Animations::JumpUp)), AnimPlan::JumpUp);
        assert_eq!(replay_plan(&state(Animations::JumpDown)), AnimPlan::JumpDown);
        assert_eq!(
            replay_plan(&state(Animations::Strafe)),
            AnimPlan::Strafe { aim_rotation: 0.7, blend_position: Vector2::new(0.3, 0.4) }
        );
        assert_eq!(
            replay_plan(&state(Animations::Walk)),
            AnimPlan::Walk { blend_position: Vector2::new(0.5, 0.0) }
        );
    }
}
