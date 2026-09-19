//! The robot entity's pure gameplay systems (constitution 1.5.2 "ECS shape (v3)"; specs/013
//! research R6): `robot_decide` (`Gameplay`), `robot_step_and_animate` and `robot_replay`
//! (`GameplayIntegrate`), `robot_hit_apply` (`GameplaySettle`, after the bullet's settle) of the
//! FIXED schedule, and `robot_timers` (`Gameplay`) of the FRAME schedule. Every decision of v2's
//! `physics_process`/`animate`/`hit` (`red_robot.rs:128-259`, `:276-290`, `:456-490`) lives
//! here; every engine call is in `red_robot/sync.rs`. `model.rs` is untouched.

use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use godot::builtin::{Transform3D, Vector2, Vector3};

use super::State;
use super::model::{self, Cmd, RobotInputs, RobotTuning};
use crate::ecs::event::RobotHitLocal;
use crate::ecs::index::EntityIndex;
use crate::ecs::markers::{
    AimBlend, AnimDecision, Dead, FixedDelta, FrameDelta, Health, Orientation, PendingTrauma,
    RaycastAnswers, RemovalTimer, Remove, ReplayRobot, RobotCountersC, RobotFrame, RobotIntents,
    RobotState, RobotTag, RootMotion, ShootRequested, Simulates, TargetPosition, TrackedPlayer,
    TraumaDue, Tuning, Velocity,
};

/// v2 `red_robot.rs:199` and `:403` — the laser/shoot ray length when nothing is hit.
pub const DEFAULT_MAX_DIST: f32 = 1000.0;

/// v2 `:164-166` / `:461-463`: the transposed-basis-local target vector's angle.
fn angle_to(global_transform: Transform3D, target: Vector3) -> f32 {
    let local: Vector3 = global_transform.basis.transposed() * (target - global_transform.origin);
    model::angle_to_player(local)
}

/// `robot_decide`'s query (a `type` so clippy's `type_complexity` stays quiet).
type DecideQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static RobotFrame,
        &'static RobotState,
        &'static RobotCountersC,
        &'static TrackedPlayer,
        &'static mut TargetPosition,
        &'static mut RobotIntents,
        Has<ShootRequested>,
    ),
    (With<Simulates>, Without<Dead>),
>;

/// `Gameplay`, FIXED schedule, `Simulates` live robots: the intents are rebuilt; the
/// `ShootRequested` marker becomes the `shoot` intent (v2 `:138-141`, consumed once); no tracked
/// player → the no-player branch (`:143-151`: target zero, `idle_branch`); else `target =
/// player_origin` (`:153`) and the raycast decision of the state branch — `Approach`: facing and
/// the countdown about to expire (`:177-183`); `Aim | Shooting`: the laser clip from the snapshot
/// (`:199-205`) and, in `Aim`, the countdown
/// gate (`:216-222`). The laser values are the previous run's read (research R8's one-step
/// buffer, `LaserBuffer`): the shoot animation enables the ray, so it is live in Aim/Shooting.
pub fn robot_decide(
    tuning: Res<Tuning<RobotTuning>>,
    dt: Res<FixedDelta>,
    mut robots: DecideQuery,
    mut commands: Commands,
) {
    let dt = dt.0 as f32;
    for (entity, frame, state, counters, tracked, mut target, mut intents, shoot_requested) in &mut robots {
        *intents = RobotIntents::default();
        if shoot_requested {
            intents.shoot = true;
            commands.entity(entity).remove::<ShootRequested>();
        }
        let Some(player_origin) = frame.player_origin.filter(|_| tracked.0.is_some()) else {
            target.0 = Vector3::ZERO;
            intents.idle_branch = true;
            continue;
        };
        target.0 = player_origin;
        match state.0 {
            State::Approach => {
                let angle = angle_to(frame.global_transform, target.0);
                // research.md R3 (V2-D): raycast only when facing AND the countdown is about to
                // expire — the same gate v1 applies before ever decrementing shoot_countdown.
                if model::facing(angle, tuning.0.player_aim_tolerance)
                    && model::shoot_countdown_will_expire(counters.0.shoot_countdown, dt)
                {
                    intents.raycast = Some((frame.ray_from.origin, target.0 + Vector3::UP));
                }
            }
            State::Aim | State::Shooting => {
                let max_dist = if frame.laser_colliding {
                    (frame.ray_from.origin - frame.laser_point).length()
                } else {
                    DEFAULT_MAX_DIST
                };
                intents.clip = Some(max_dist);
                // v1's `:206` gate excludes Shooting explicitly.
                if state.0 == State::Aim && model::aim_countdown_will_expire(counters.0.aim_countdown, dt) {
                    intents.raycast = Some((frame.ray_from.origin, target.0 + Vector3::UP));
                }
            }
            State::Idle => {}
        }
    }
}

/// v2 `animate` (`red_robot.rs:456-490`) as a decision: the transition request and, when the
/// target is set, the aim blend amount and the stepped `AimBlend` (the component replaces the
/// tree `get` of `:482-485`, spec FR-016). Called on the POST-step state and counters (v2 called
/// `animate` after `step`, `:238`), on the idle branch (`:145`) and on the replay path (`:134`).
fn animation_decision(
    state: State,
    target: Vector3,
    frame: &RobotFrame,
    aim_preparing: f32,
    aim_blend: &mut Vector2,
    dt: f32,
    tuning: &RobotTuning,
) -> AnimDecision {
    let angle = (state == State::Approach).then(|| angle_to(frame.global_transform, target));
    let request = model::transition_request(state, angle, target == Vector3::ZERO, tuning);
    // Aiming or shooting.
    let aim = (target != Vector3::ZERO).then(|| {
        let blend_amount = model::aim_blend_amount(aim_preparing, tuning);
        let mt = frame.ray_mesh;
        let to_cannon_local: Vector3 = mt.basis.transposed() * (target + Vector3::UP - mt.origin);
        let (h_angle, v_angle) = model::cannon_angles(to_cannon_local);
        *aim_blend = model::aim_blend_step(*aim_blend, h_angle, v_angle, dt, tuning);
        (blend_amount, *aim_blend)
    });
    AnimDecision { request, aim }
}

/// `robot_step_and_animate`'s query.
type StepQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RobotFrame,
        &'static RaycastAnswers,
        &'static mut RobotState,
        &'static mut RobotCountersC,
        &'static TargetPosition,
        &'static mut AimBlend,
        &'static mut RobotIntents,
        &'static RootMotion,
        &'static mut Orientation,
        &'static mut Velocity,
    ),
    (With<Simulates>, Without<Dead>),
>;

/// `GameplayIntegrate`, FIXED schedule, `Simulates` live robots: on the idle branch the
/// animation decision and `idle_velocity` (v2 `:145-146`); otherwise `model::step` on the state
/// at run start with the raycast answer (`:186-196`, `:225-235`; `RpcPlayShoot` → the
/// `play_shoot` intent, `ResumeApproach` already applied by `step`), THEN the animation decision
/// on the NEW state and counters (`:238`), THEN `integrate_root_motion` (`:245-251`) into
/// `Orientation`/`Velocity`.
pub fn robot_step_and_animate(tuning: Res<Tuning<RobotTuning>>, dt: Res<FixedDelta>, mut robots: StepQuery) {
    let dt = dt.0 as f32;
    for (frame, answers, mut state, mut counters, target, mut aim_blend, mut intents, root_motion, mut orientation, mut velocity) in
        &mut robots
    {
        if intents.idle_branch {
            intents.anim = Some(animation_decision(
                state.0,
                target.0,
                frame,
                counters.0.aim_preparing,
                &mut aim_blend.0,
                dt,
                &tuning.0,
            ));
            velocity.0 = model::idle_velocity(frame.gravity, dt);
            continue;
        }
        // v1's if/else-if is ONE mutually-exclusive decision keyed to the state at the START
        // of this step; `Idle` has no branch (`model::step` treats it as a no-op).
        let state_at_start = state.0;
        if state_at_start != State::Idle {
            let angle = (state_at_start == State::Approach).then(|| angle_to(frame.global_transform, target.0));
            let inputs = RobotInputs { angle_to_player: angle, sees_player: answers.sees_player };
            let (new_state, cmds) = model::step(state_at_start, &mut counters.0, dt, &inputs, &tuning.0);
            state.0 = new_state;
            for cmd in cmds {
                match cmd {
                    Cmd::RpcPlayShoot => intents.play_shoot = true,
                    Cmd::ResumeApproach => {}
                }
            }
        }
        intents.anim = Some(animation_decision(
            state.0,
            target.0,
            frame,
            counters.0.aim_preparing,
            &mut aim_blend.0,
            dt,
            &tuning.0,
        ));
        let (new_orientation, new_velocity) =
            model::integrate_root_motion(orientation.0, root_motion.0, dt, frame.gravity, frame.velocity);
        orientation.0 = new_orientation;
        velocity.0 = new_velocity;
    }
}

/// `GameplayIntegrate`, FIXED schedule, non-`Simulates` live robots: v2's client path (`:133-135`)
/// — the animation decision from the replicated `state`/`target_position`/`aim_preparing`.
type ReplayQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ReplayRobot, &'static RobotFrame, &'static mut AimBlend, &'static mut RobotIntents),
    (Without<Simulates>, Without<Dead>),
>;

pub fn robot_replay(tuning: Res<Tuning<RobotTuning>>, dt: Res<FixedDelta>, mut robots: ReplayQuery) {
    let dt = dt.0 as f32;
    for (replay, frame, mut aim_blend, mut intents) in &mut robots {
        *intents = RobotIntents::default();
        intents.anim = Some(animation_decision(
            replay.state,
            replay.target_position,
            frame,
            replay.aim_preparing,
            &mut aim_blend.0,
            dt,
            &tuning.0,
        ));
    }
}

/// `GameplaySettle`, FIXED schedule, `.after(bullet_settle)` (spec option (B), research R4): the
/// same run's `RobotHitLocal` messages — v2 `hit`'s `dead` guard (`:278-280`; a robot that died
/// earlier in this run counts as dead) and `hit_step` (`:289-290`) → `hit`, `just_died`.
pub fn robot_hit_apply(
    mut hits: MessageReader<RobotHitLocal>,
    index: Res<EntityIndex>,
    mut robots: Query<(&mut Health, &mut RobotIntents, Has<Dead>), With<RobotTag>>,
) {
    for hit in hits.read() {
        let Some(entity) = index.entity(hit.robot) else {
            continue;
        };
        let Ok((mut health, mut intents, dead)) = robots.get_mut(entity) else {
            continue;
        };
        if dead || intents.just_died {
            continue;
        }
        let (new_health, just_died) = model::hit_step(health.0);
        health.0 = new_health;
        intents.hit = true;
        intents.just_died |= just_died;
    }
}

/// `Gameplay`, FRAME schedule, every robot: the two `SceneTreeTimer`s as tick timers —
/// `PendingTrauma` (v2 `:437-453`, `trauma_delay`) → `TraumaDue(player)`; `RemovalTimer`
/// (`:309-326`, `removal_delay`) → `Remove`.
pub fn robot_timers(
    dt: Res<FrameDelta>,
    mut robots: Query<(Entity, &mut PendingTrauma, &mut RemovalTimer)>,
    mut commands: Commands,
) {
    for (entity, mut pending, mut removal) in &mut robots {
        if let Some((timer, player)) = pending.0.as_mut()
            && timer.step(dt.0)
        {
            let player = *player;
            pending.0 = None;
            commands.entity(entity).insert(TraumaDue(player));
        }
        if let Some(timer) = removal.0.as_mut()
            && timer.step(dt.0)
        {
            removal.0 = None;
            commands.entity(entity).insert(Remove);
        }
    }
}

/// Spec FR-022 as amended: the tree is advanced for every live robot EXCEPT in the run it dies
/// (v2 set the tree inactive at death, `:294`, and `AnimationMixer::advance` ignores `active`).
pub fn should_advance(dead: bool, just_died: bool) -> bool {
    !dead && !just_died
}

/// The client's own `hit` handler logic (v2 `:278-290` ran on every peer through `call_local`;
/// `health`/`dead` are spawn-only replicated, analyze BLOCKER 1): `hits` remote hits applied to
/// the client's `health` → `(new_health, reactions, just_died)` — one reaction per LIVE hit, the
/// hits after the death ignored (`:278-280`).
pub fn remote_hits(hits: u32, health: i32, dead: bool) -> (i32, u32, bool) {
    let mut health = health;
    let mut dead = dead;
    let mut reactions = 0;
    let mut died = false;
    for _ in 0..hits {
        if dead {
            break;
        }
        reactions += 1;
        let (new_health, just_died) = model::hit_step(health);
        health = new_health;
        if just_died {
            dead = true;
            died = true;
        }
    }
    (health, reactions, died)
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use godot::builtin::Basis;
    use godot::obj::InstanceId;

    use super::*;
    use crate::ecs::markers::AnimDecision;
    use crate::ecs::timer::Timer;
    use crate::red_robot::model::RobotCounters;

    const DT: f64 = 1.0 / 60.0;
    const GRAVITY: Vector3 = Vector3::new(0.0, -9.8, 0.0);

    fn player() -> InstanceId {
        InstanceId::from_i64(42)
    }

    /// The robot at the origin facing +Z (identity), the player 8 m in front (R1's `ZZ_PZ=8`).
    fn frame(player_origin: Option<Vector3>) -> RobotFrame {
        RobotFrame {
            global_transform: Transform3D::IDENTITY,
            gravity: GRAVITY,
            velocity: Vector3::ZERO,
            player_origin,
            ray_from: Transform3D::new(Basis::IDENTITY, Vector3::new(0.0, 1.5, 0.3)),
            ray_mesh: Transform3D::new(Basis::IDENTITY, Vector3::new(0.0, 1.5, 0.5)),
            ray_mesh_z: -0.5,
            laser_colliding: false,
            laser_point: Vector3::ZERO,
        }
    }

    fn counters(aim_preparing: f32, shoot_countdown: f32, aim_countdown: f32) -> RobotCountersC {
        RobotCountersC(RobotCounters { aim_preparing, shoot_countdown, aim_countdown })
    }

    /// A `Simulates` robot with every component the fixed tick reads.
    fn world_with(state: State, counters: RobotCountersC, tracked: Option<InstanceId>, frame: RobotFrame) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Tuning(RobotTuning::default()));
        world.insert_resource(FixedDelta(DT));
        world.insert_resource(FrameDelta(DT));
        world.insert_resource(Messages::<RobotHitLocal>::default());
        world.insert_resource(EntityIndex::default());
        let e = world
            .spawn((
                RobotTag,
                RobotState(state),
                Health(5),
                TargetPosition(Vector3::ZERO),
                counters,
                TrackedPlayer(tracked),
                Orientation(Transform3D::IDENTITY),
                RootMotion(Transform3D::IDENTITY),
                Velocity(Vector3::ZERO),
                AimBlend(Vector2::ZERO),
                frame,
            ))
            .id();
        world.entity_mut(e).insert((
            RobotIntents::default(),
            RaycastAnswers::default(),
            PendingTrauma::default(),
            RemovalTimer::default(),
            Simulates,
        ));
        (world, e)
    }

    fn in_front() -> Option<Vector3> {
        Some(Vector3::new(0.0, 0.05, 8.0))
    }

    fn intents(world: &World, e: Entity) -> RobotIntents {
        world.get::<RobotIntents>(e).unwrap().clone()
    }

    fn state(world: &World, e: Entity) -> State {
        world.get::<RobotState>(e).unwrap().0
    }

    fn get_counters(world: &World, e: Entity) -> RobotCounters {
        world.get::<RobotCountersC>(e).unwrap().0
    }

    fn decide_and_step(world: &mut World) {
        world.run_system_once(robot_decide).unwrap();
        world.run_system_once(robot_step_and_animate).unwrap();
    }

    #[test]
    fn no_player_branch_uses_idle_velocity_and_zero_target() {
        let (mut world, e) = world_with(State::Idle, counters(0.5, 6.0, 1.0), None, frame(None));
        decide_and_step(&mut world);
        let i = intents(&world, e);
        assert!(i.idle_branch);
        assert_eq!(i.raycast, None);
        assert_eq!(world.get::<TargetPosition>(e).unwrap().0, Vector3::ZERO);
        assert_eq!(world.get::<Velocity>(e).unwrap().0, model::idle_velocity(GRAVITY, DT as f32));
        assert_eq!(i.anim, Some(AnimDecision { request: "idle", aim: None }));
    }

    #[test]
    fn approach_requests_raycast_only_when_facing_and_countdown_expiring() {
        // Facing (the player straight ahead) and the countdown about to expire → a raycast.
        let (mut world, e) = world_with(State::Approach, counters(0.0, 0.01, 1.0), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        let ray_from = frame(None).ray_from.origin;
        assert_eq!(intents(&world, e).raycast, Some((ray_from, Vector3::new(0.0, 0.05, 8.0) + Vector3::UP)));
        // Facing, countdown far from expiry → none.
        let (mut world, e) = world_with(State::Approach, counters(0.0, 2.0, 1.0), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        assert_eq!(intents(&world, e).raycast, None);
        // Not facing (the player at the robot's side), countdown expiring → none.
        let side = Some(Vector3::new(8.0, 0.05, 0.0));
        let (mut world, e) = world_with(State::Approach, counters(0.0, 0.01, 1.0), Some(player()), frame(side));
        world.run_system_once(robot_decide).unwrap();
        assert_eq!(intents(&world, e).raycast, None);
    }

    #[test]
    fn approach_to_aim_when_raycast_sees_player() {
        let (mut world, e) = world_with(State::Approach, counters(0.0, 0.01, 1.0), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        world.entity_mut(e).insert(RaycastAnswers { sees_player: Some(true), shot: None });
        world.run_system_once(robot_step_and_animate).unwrap();
        assert_eq!(state(&world, e), State::Aim);
        let c = get_counters(&world, e);
        assert_eq!(c.aim_countdown, RobotTuning::default().aim_time);
        assert_eq!(c.aim_preparing, 0.0);
    }

    #[test]
    fn aim_branch_clips_at_1000_when_laser_not_colliding() {
        let (mut world, e) = world_with(State::Aim, counters(0.0, 6.0, 1.0), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        let i = intents(&world, e);
        assert_eq!(i.clip, Some(DEFAULT_MAX_DIST));
        assert_eq!(i.raycast, None);
        // Shooting clips too, never raycasts.
        let (mut world, e) = world_with(State::Shooting, counters(0.5, 6.0, 0.01), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        let i = intents(&world, e);
        assert_eq!(i.clip, Some(DEFAULT_MAX_DIST));
        assert_eq!(i.raycast, None);
    }

    #[test]
    fn aim_to_shooting_emits_play_shoot() {
        let (mut world, e) = world_with(State::Aim, counters(0.5, 6.0, 0.01), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        assert!(intents(&world, e).raycast.is_some());
        world.entity_mut(e).insert(RaycastAnswers { sees_player: Some(true), shot: None });
        world.run_system_once(robot_step_and_animate).unwrap();
        assert_eq!(state(&world, e), State::Shooting);
        assert!(intents(&world, e).play_shoot);
    }

    #[test]
    fn aim_lost_resumes_approach() {
        let (mut world, e) = world_with(State::Aim, counters(0.3, 2.0, 0.01), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        world.entity_mut(e).insert(RaycastAnswers { sees_player: Some(false), shot: None });
        world.run_system_once(robot_step_and_animate).unwrap();
        assert_eq!(state(&world, e), State::Approach);
        let c = get_counters(&world, e);
        let t = RobotTuning::default();
        assert_eq!(c.aim_preparing, t.aim_prepare_time);
        assert_eq!(c.shoot_countdown, t.shoot_wait);
        assert!(!intents(&world, e).play_shoot);
    }

    #[test]
    fn animation_decision_uses_the_post_step_state() {
        // Approach → Aim this step: the request is `Aim`'s "idle", not `Approach`'s "walk".
        let (mut world, e) = world_with(State::Approach, counters(0.0, 0.01, 1.0), Some(player()), frame(in_front()));
        world.run_system_once(robot_decide).unwrap();
        world.entity_mut(e).insert(RaycastAnswers { sees_player: Some(true), shot: None });
        world.run_system_once(robot_step_and_animate).unwrap();
        let anim = intents(&world, e).anim.unwrap();
        assert_eq!(anim.request, "idle");
        // The aim blend amount comes from the post-step `aim_preparing` (forced to 0.0).
        assert_eq!(anim.aim.map(|(amount, _)| amount), Some(0.0));
        // The same robot staying in Approach requests "walk".
        let (mut world, e) = world_with(State::Approach, counters(0.0, 2.0, 1.0), Some(player()), frame(in_front()));
        decide_and_step(&mut world);
        assert_eq!(intents(&world, e).anim.unwrap().request, "walk");
    }

    #[test]
    fn aim_blend_steps_from_the_component() {
        let t = RobotTuning::default();
        let target = Vector3::new(0.0, 0.05, 8.0);
        let (mut world, e) = world_with(State::Aim, counters(0.5, 6.0, 1.0), Some(player()), frame(Some(target)));
        let f = frame(None);
        let local: Vector3 = f.ray_mesh.basis.transposed() * (target + Vector3::UP - f.ray_mesh.origin);
        let (h, v) = model::cannon_angles(local);
        let once = model::aim_blend_step(Vector2::ZERO, h, v, DT as f32, &t);
        let twice = model::aim_blend_step(once, h, v, DT as f32, &t);
        decide_and_step(&mut world);
        assert_eq!(world.get::<AimBlend>(e).unwrap().0, once);
        assert_eq!(intents(&world, e).anim.unwrap().aim, Some((1.0, once)));
        decide_and_step(&mut world);
        assert_eq!(world.get::<AimBlend>(e).unwrap().0, twice);
        assert_eq!(intents(&world, e).anim.unwrap().aim, Some((1.0, twice)));
    }

    #[test]
    fn integrate_matches_model_twin() {
        let (mut world, e) = world_with(State::Approach, counters(0.0, 6.0, 1.0), Some(player()), frame(in_front()));
        let root_motion = Transform3D::new(Basis::IDENTITY, Vector3::new(0.0, 0.0, 0.02));
        world.entity_mut(e).insert(RootMotion(root_motion));
        decide_and_step(&mut world);
        let (expected_orientation, expected_velocity) =
            model::integrate_root_motion(Transform3D::IDENTITY, root_motion, DT as f32, GRAVITY, Vector3::ZERO);
        assert_eq!(world.get::<Orientation>(e).unwrap().0, expected_orientation);
        assert_eq!(world.get::<Velocity>(e).unwrap().0, expected_velocity);
    }

    #[test]
    fn shoot_requested_sets_shoot_intent_once() {
        let (mut world, e) = world_with(State::Idle, counters(0.5, 0.0, 1.0), None, frame(None));
        world.entity_mut(e).insert(ShootRequested);
        world.run_system_once(robot_decide).unwrap();
        assert!(intents(&world, e).shoot);
        assert!(world.get::<ShootRequested>(e).is_none());
        world.run_system_once(robot_decide).unwrap();
        assert!(!intents(&world, e).shoot);
    }

    #[test]
    fn replay_builds_animation_from_replicated_fields() {
        let target = Vector3::new(0.0, 0.05, 8.0);
        let (mut world, e) = world_with(State::Idle, counters(0.5, 6.0, 1.0), None, frame(None));
        world.entity_mut(e).remove::<Simulates>();
        world.entity_mut(e).insert(ReplayRobot { state: State::Approach, target_position: target, aim_preparing: 0.25 });
        world.run_system_once(robot_replay).unwrap();
        let anim = intents(&world, e).anim.unwrap();
        assert_eq!(anim.request, "walk");
        let (amount, blend) = anim.aim.unwrap();
        assert_eq!(amount, model::aim_blend_amount(0.25, &RobotTuning::default()));
        assert_eq!(blend, world.get::<AimBlend>(e).unwrap().0);
        assert_ne!(blend, Vector2::ZERO);
    }

    #[test]
    fn robot_hit_apply_ignores_dead_robot() {
        let robot = InstanceId::from_i64(7);
        let (mut world, e) = world_with(State::Idle, counters(0.5, 6.0, 1.0), None, frame(None));
        world.entity_mut(e).insert((Health(3), Dead));
        world.resource_mut::<EntityIndex>().register_if_absent(robot, || e);
        world.resource_mut::<Messages<RobotHitLocal>>().write(RobotHitLocal { robot });
        world.run_system_once(robot_hit_apply).unwrap();
        assert_eq!(world.get::<Health>(e).unwrap().0, 3);
        let i = intents(&world, e);
        assert!(!i.hit);
        assert!(!i.just_died);
        // A live robot takes it: 3 → 2, no death (the buffer is cleared: `run_system_once`
        // builds a fresh reader cursor each time, which would re-read the first message).
        world.entity_mut(e).remove::<Dead>();
        world.resource_mut::<Messages<RobotHitLocal>>().clear();
        world.resource_mut::<Messages<RobotHitLocal>>().write(RobotHitLocal { robot });
        world.run_system_once(robot_hit_apply).unwrap();
        assert_eq!(world.get::<Health>(e).unwrap().0, 2);
        let i = intents(&world, e);
        assert!(i.hit);
        assert!(!i.just_died);
    }

    #[test]
    fn pending_trauma_expiry_flags_trauma_due() {
        let (mut world, e) = world_with(State::Idle, counters(0.5, 6.0, 1.0), None, frame(None));
        world.entity_mut(e).insert(PendingTrauma(Some((Timer::new(DT), player()))));
        world.run_system_once(robot_timers).unwrap();
        assert_eq!(world.get::<TraumaDue>(e).map(|t| t.0), Some(player()));
        assert!(world.get::<PendingTrauma>(e).unwrap().0.is_none());
        assert!(world.get::<Remove>(e).is_none());
    }

    #[test]
    fn removal_timer_expiry_marks_remove() {
        let (mut world, e) = world_with(State::Idle, counters(0.5, 6.0, 1.0), None, frame(None));
        world.entity_mut(e).insert(RemovalTimer(Some(Timer::new(DT))));
        world.run_system_once(robot_timers).unwrap();
        assert!(world.get::<Remove>(e).is_some());
        assert!(world.get::<RemovalTimer>(e).unwrap().0.is_none());
        assert!(world.get::<TraumaDue>(e).is_none());
    }

    #[test]
    fn robot_that_just_died_is_not_advanced() {
        assert!(should_advance(false, false));
        assert!(!should_advance(false, true));
        assert!(!should_advance(true, false));
        assert!(!should_advance(true, true));
    }

    #[test]
    fn remote_hit_decrements_client_health_and_dies_at_zero() {
        // Four hits: 5 → 1, four reactions, alive.
        assert_eq!(remote_hits(4, 5, false), (1, 4, false));
        // The fifth kills: exactly one death.
        assert_eq!(remote_hits(1, 1, false), (0, 1, true));
        // A sixth on a dead robot is ignored (no reaction, no decrement).
        assert_eq!(remote_hits(1, 0, true), (0, 0, false));
        // Six at once: the sixth is ignored inside the same frame too.
        assert_eq!(remote_hits(6, 5, false), (0, 5, true));
    }
}
