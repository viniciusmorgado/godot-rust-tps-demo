//! gdext builtin math is pure only when its body does not go through `as_inner()` or a
//! generated `out/builtin_classes/**` method (constitution 1.4.1). Every builtin call in this
//! module (`Basis::transposed`, the `Basis`/`Transform3D` `Mul` operators, `orthonormalized`,
//! `Vector3`/`Vector2` arithmetic, `f32::atan2`/`to_degrees`) is glam/std-based and pure —
//! verified against the gdext 0.5.5 source (research.md R1). Nothing in this module calls the
//! engine.

use godot::prelude::*;

use super::State;

/// Tuning constants for `EnemyRobot`'s state machine/aim/laser math — the exact `v1` literals
/// from `red_robot.rs`.
#[derive(Clone, Copy, Debug)]
pub struct RobotTuning {
    /// v1: `PLAYER_AIM_TOLERANCE_DEGREES = 15.0.to_radians()` (`:21`) — RENAMED (the constant
    /// holds radians despite its old name), value unchanged.
    pub player_aim_tolerance: f32,
    pub shoot_wait: f32,       // v1: SHOOT_WAIT = 6.0 (:23)
    pub aim_time: f32,         // v1: AIM_TIME = 1.0 (:24)
    pub aim_prepare_time: f32, // v1: AIM_PREPARE_TIME = 0.5 (:26)
    pub blend_aim_speed: f32,  // v1: BLEND_AIM_SPEED = 0.05 (:27)
    pub removal_delay: f32,    // v1: literal 10.0 in hit's create_timer (:305)
    pub trauma_delay: f32,     // v1: literal 0.1 in shoot's create_timer (:395)
    pub trauma_amount: f64,    // v1: literal 13.0 passed to add_camera_shake_trauma (:399)
}

impl Default for RobotTuning {
    fn default() -> Self {
        Self {
            player_aim_tolerance: 15.0_f32.to_radians(),
            shoot_wait: 6.0,
            aim_time: 1.0,
            aim_prepare_time: 0.5,
            blend_aim_speed: 0.05,
            removal_delay: 10.0,
            trauma_delay: 0.1,
            trauma_amount: 13.0,
        }
    }
}

/// The three non-replicated counters `v1` keeps as flat fields (`aim_preparing`,
/// `shoot_countdown`, `aim_countdown`). Kept FLAT here too — plan.md's Complexity Tracking:
/// `v1`'s counters are not cleanly one-per-state (they outlive `Idle`, and `resume_approach()`
/// resets them unconditionally from any state), so a richer per-variant enum would have to
/// special-case those irregularities without eliminating any invalid combination the flat shape
/// doesn't already avoid (backlog #31).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RobotCounters {
    pub aim_preparing: f32,
    pub shoot_countdown: f32,
    pub aim_countdown: f32,
}

/// One physics frame's snapshot of what `step` needs beyond the counters/state themselves.
/// `has_player` was in the original sketch but turned out redundant once implemented: `step`
/// only ever consults `angle_to_player` inside the `Approach` branch (where a player is always
/// present by construction — glue's no-player branch returns before `step` is ever called), and
/// never treats `angle_to_player.is_some()` as a general "is a player tracked" proxy elsewhere
/// (in `Aim`/`Shooting` it is deliberately `None` regardless of whether a player exists).
#[derive(Clone, Copy, Debug)]
pub struct RobotInputs {
    /// Radians — `atan2` of the transposed-basis-local target vector (glue computes the
    /// vector, `angle_to_player` below turns it into this angle). Only meaningful/consulted in
    /// the `Approach` branch.
    pub angle_to_player: Option<f32>,
    /// `Some(...)` only on a frame glue actually raycasted (research.md R3 — glue only raycasts
    /// when the relevant countdown predicate says it's about to expire, exactly mirroring
    /// `v1`'s lazy raycast timing); `None` otherwise. `step` never consults it when `None`.
    pub sees_player: Option<bool>,
}

/// The observable engine effects `step` decides to trigger — glue applies them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    /// v1: `self.base_mut().rpc("play_shoot", &[])` (`:230`).
    RpcPlayShoot,
    /// v1: the `else` branch calling `self.resume_approach()` (`:232`).
    ResumeApproach,
}

/// v1: `red_robot.rs:152-157` — only while facing the player does `shoot_countdown` count down
/// and (on expiry) trigger a raycast.
pub fn facing(angle: f32, tolerance: f32) -> bool {
    angle > -tolerance && angle < tolerance
}

/// v1: `red_robot.rs:151` — `to_player_local.x.atan2(to_player_local.z)`. The comment there
/// explains the axis choice: the robot's front is +Z, and `atan2` is zero at +X, so `z` is the
/// second (X) parameter. `local_target` is the transposed-basis-local target vector, computed
/// by glue (an engine-backed `Transform3D` read) and handed in here.
pub fn angle_to_player(local_target: Vector3) -> f32 {
    local_target.x.atan2(local_target.z)
}

/// v1: `red_robot.rs:157`/`:206`'s raycast-timing pre-checks — the exact arithmetic `step`
/// itself independently performs when it actually decrements. Glue calls these BEFORE `step` to
/// decide whether to raycast at all this frame (research.md R3), preserving `v1`'s lazy raycast
/// frequency without a two-phase `step` design.
pub fn shoot_countdown_will_expire(shoot_countdown: f32, dt: f32) -> bool {
    shoot_countdown - dt < 0.0
}
pub fn aim_countdown_will_expire(aim_countdown: f32, dt: f32) -> bool {
    aim_countdown - dt < 0.0
}

/// v1: `resume_approach()`'s counter reset (`red_robot.rs:271-273`), shared by the `#[func]`
/// glue and `step`'s `Cmd::ResumeApproach` path so the reset formula lives in exactly one place.
pub fn resume_approach_reset(tuning: &RobotTuning) -> (f32, f32) {
    (tuning.aim_prepare_time, tuning.shoot_wait)
}

/// Literal transcription of `v1`'s state-transition logic (`red_robot.rs:140-235`), in `v1`'s
/// exact order. Reordering these steps changes the observable result — do not "simplify".
pub fn step(
    mut state: State,
    counters: &mut RobotCounters,
    dt: f32,
    inputs: &RobotInputs,
    tuning: &RobotTuning,
) -> (State, Vec<Cmd>) {
    let mut cmds = Vec::new();

    if state == State::Approach {
        // v1:141-146
        if counters.aim_preparing > 0.0 {
            counters.aim_preparing -= dt;
            if counters.aim_preparing < 0.0 {
                counters.aim_preparing = 0.0;
            }
        }

        // v1:148-154 (facing gate) + :156-188 (countdown/raycast decision)
        if let Some(angle) = inputs.angle_to_player
            && facing(angle, tuning.player_aim_tolerance)
        {
            counters.shoot_countdown -= dt;
            if counters.shoot_countdown < 0.0 {
                match inputs.sees_player {
                    Some(true) => {
                        // v1:181-183 — aim_preparing FORCED to 0.0, not just left as-is.
                        state = State::Aim;
                        counters.aim_countdown = tuning.aim_time;
                        counters.aim_preparing = 0.0;
                    }
                    _ => {
                        // v1:186 — player not in sight, retry later.
                        counters.shoot_countdown = tuning.shoot_wait;
                    }
                }
            }
        }
    } else if state == State::Aim || state == State::Shooting {
        // v1:198-203
        if counters.aim_preparing < tuning.aim_prepare_time {
            counters.aim_preparing += dt;
            if counters.aim_preparing > tuning.aim_prepare_time {
                counters.aim_preparing = tuning.aim_prepare_time;
            }
        }

        // v1:205 — UNCONDITIONAL in both Aim and Shooting.
        counters.aim_countdown -= dt;
        // v1:206 — the `state == State::Aim` gate EXPLICITLY excludes Shooting; Shooting's
        // aim_countdown keeps draining below zero with no further effect (v1 quirk, kept).
        if counters.aim_countdown < 0.0 && state == State::Aim {
            match inputs.sees_player {
                Some(true) => {
                    // v1:227-230
                    state = State::Shooting;
                    counters.shoot_countdown = tuning.shoot_wait;
                    cmds.push(Cmd::RpcPlayShoot);
                }
                _ => {
                    // v1:232 — resume_approach()'s reset, shared formula.
                    let (aim_preparing, shoot_countdown) = resume_approach_reset(tuning);
                    counters.aim_preparing = aim_preparing;
                    counters.shoot_countdown = shoot_countdown;
                    state = State::Approach;
                    cmds.push(Cmd::ResumeApproach);
                }
            }
        }
    }
    // Idle: no-op — v1 has no `if self.state == State::Idle` branch at all.

    (state, cmds)
}

/// v1: `hit`'s decrement + death-transition decision only (`red_robot.rs:284-285`). The RNG
/// hit-reaction pick and every engine effect of the death sequence stay in glue — this function
/// decides NOTHING about them; it only reports whether the death sequence should run this call.
pub fn hit_step(health: i32) -> (i32, bool) {
    let new_health = health - 1;
    (new_health, health > 0 && new_health == 0)
}

/// v1: `animate`'s transition-request choice (`red_robot.rs:407-428`).
pub fn transition_request(
    state: State,
    angle_to_player: Option<f32>,
    target_is_zero: bool,
    tuning: &RobotTuning,
) -> &'static str {
    if state == State::Approach {
        match angle_to_player {
            Some(angle) if angle > tuning.player_aim_tolerance => "turn_left",
            Some(angle) if angle < -tuning.player_aim_tolerance => "turn_right",
            _ => {
                if target_is_zero { "idle" } else { "walk" }
            }
        }
    } else {
        "idle"
    }
}

/// v1: `animate`'s `aiming/blend_amount` (`red_robot.rs:432-435`).
pub fn aim_blend_amount(aim_preparing: f32, tuning: &RobotTuning) -> f32 {
    (aim_preparing / tuning.aim_prepare_time).clamp(0.0, 1.0)
}

/// v1: `animate`'s cannon-local angle computation (`red_robot.rs:440-441`), in DEGREES.
/// Returns `(h_angle, v_angle)`.
pub fn cannon_angles(to_cannon_local: Vector3) -> (f32, f32) {
    (
        to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees(),
        to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees(),
    )
}

/// v1: `animate`'s aim-blend nudge (`red_robot.rs:446-452`).
pub fn aim_blend_step(blend: Vector2, h_angle: f32, v_angle: f32, dt: f32, tuning: &RobotTuning) -> Vector2 {
    let mut blend = blend;
    blend.x += tuning.blend_aim_speed * dt * -h_angle;
    blend.x = blend.x.clamp(-1.0, 1.0);
    blend.y += tuning.blend_aim_speed * dt * v_angle;
    blend.y = blend.y.clamp(-1.0, 1.0);
    blend
}

/// v1: `shoot`'s laser-ember placement (`red_robot.rs:372`).
pub fn ember_position(max_dist: f32, mesh_offset: f32) -> Vector3 {
    Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset)
}

/// v1: `shoot`'s laser-ember extents (`red_robot.rs:373-374`) — only `.z` changes, x/y are
/// untouched from `current` (v1's partial mutation of the existing extents value).
pub fn ember_extents(current: Vector3, max_dist: f32, mesh_offset: f32) -> Vector3 {
    let mut extents = current;
    extents.z = (max_dist - mesh_offset.abs()) / 2.0;
    extents
}

/// TWIN of `player/model.rs::integrate_root_motion` (V2-C) — identical body, kept independent
/// (research.md R6: `player.rs`'s `mod model;` is private, and this milestone leaves `player.rs`
/// untouched). v1: `red_robot.rs:238-257`.
pub fn integrate_root_motion(
    orientation: Transform3D,
    root_motion: Transform3D,
    dt: f32,
    gravity: Vector3,
    velocity_in: Vector3,
) -> (Transform3D, Vector3) {
    let orientation = orientation * root_motion;
    let h_velocity = orientation.origin / dt;

    let mut velocity = velocity_in;
    velocity.x = h_velocity.x;
    velocity.z = h_velocity.z;
    velocity += gravity * dt;

    let mut orientation = orientation;
    orientation.origin = Vector3::ZERO;
    let orientation = orientation.orthonormalized();

    (orientation, velocity)
}

/// v1: `red_robot.rs:128-136`'s no-player-branch velocity (gravity only).
pub fn idle_velocity(gravity: Vector3, dt: f32) -> Vector3 {
    gravity * dt
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.1;

    #[test]
    fn shoot_countdown_will_expire_true_just_below_zero_crossing() {
        assert!(shoot_countdown_will_expire(0.05, DT));
    }

    #[test]
    fn shoot_countdown_will_expire_false_when_still_positive_after_dt() {
        assert!(!shoot_countdown_will_expire(0.5, DT));
    }

    #[test]
    fn aim_countdown_will_expire_true_just_below_zero_crossing() {
        assert!(aim_countdown_will_expire(0.05, DT));
    }

    #[test]
    fn aim_countdown_will_expire_false_when_still_positive_after_dt() {
        assert!(!aim_countdown_will_expire(0.5, DT));
    }

    #[test]
    fn facing_true_inside_tolerance_both_signs() {
        let tolerance = 15.0_f32.to_radians();
        assert!(facing(0.0, tolerance));
        assert!(facing(tolerance * 0.5, tolerance));
        assert!(facing(-tolerance * 0.5, tolerance));
    }

    #[test]
    fn facing_false_outside_tolerance_both_signs() {
        let tolerance = 15.0_f32.to_radians();
        assert!(!facing(tolerance * 2.0, tolerance));
        assert!(!facing(-tolerance * 2.0, tolerance));
    }

    #[test]
    fn angle_to_player_zero_straight_ahead() {
        // Robot's front is +Z; a target straight ahead is local (0, _, positive_z).
        assert_eq!(angle_to_player(Vector3::new(0.0, 0.0, 1.0)), 0.0);
    }

    #[test]
    fn angle_to_player_quarter_turn_either_side() {
        let right = angle_to_player(Vector3::new(1.0, 0.0, 0.0));
        let left = angle_to_player(Vector3::new(-1.0, 0.0, 0.0));
        assert!((right - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((left + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn hit_step_decrements_without_reaching_zero() {
        assert_eq!(hit_step(5), (4, false));
    }

    #[test]
    fn hit_step_reports_reaching_zero() {
        assert_eq!(hit_step(1), (0, true));
    }

    fn inputs(angle: Option<f32>, sees: Option<bool>) -> RobotInputs {
        RobotInputs {
            angle_to_player: angle,
            sees_player: sees,
        }
    }

    #[test]
    fn approach_counts_aim_preparing_down_and_clamps_at_zero() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.05,
            shoot_countdown: tuning.shoot_wait,
            aim_countdown: 0.0,
        };
        let (state, cmds) = step(State::Approach, &mut counters, DT, &inputs(None, None), &tuning);
        assert_eq!(state, State::Approach);
        assert_eq!(counters.aim_preparing, 0.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn approach_does_not_decrement_shoot_countdown_when_not_facing() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.0,
            shoot_countdown: tuning.shoot_wait,
            aim_countdown: 0.0,
        };
        let out_of_tolerance = tuning.player_aim_tolerance * 2.0;
        let (_, _) = step(
            State::Approach,
            &mut counters,
            DT,
            &inputs(Some(out_of_tolerance), None),
            &tuning,
        );
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
    }

    #[test]
    fn approach_transitions_to_aim_on_expiry_when_sees_player() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.3,
            shoot_countdown: 0.05,
            aim_countdown: 0.0,
        };
        let (state, cmds) = step(
            State::Approach,
            &mut counters,
            DT,
            &inputs(Some(0.0), Some(true)),
            &tuning,
        );
        assert_eq!(state, State::Aim);
        assert_eq!(counters.aim_countdown, tuning.aim_time);
        assert_eq!(counters.aim_preparing, 0.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn approach_retries_on_expiry_when_not_seeing_player() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.0,
            shoot_countdown: 0.05,
            aim_countdown: 0.0,
        };
        let (state, cmds) = step(
            State::Approach,
            &mut counters,
            DT,
            &inputs(Some(0.0), Some(false)),
            &tuning,
        );
        assert_eq!(state, State::Approach);
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
        assert!(cmds.is_empty());
    }

    #[test]
    fn aim_transitions_to_shooting_on_expiry_when_sees_player() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.0,
            shoot_countdown: 0.0,
            aim_countdown: 0.05,
        };
        let (state, cmds) = step(State::Aim, &mut counters, DT, &inputs(None, Some(true)), &tuning);
        assert_eq!(state, State::Shooting);
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
        assert_eq!(cmds, vec![Cmd::RpcPlayShoot]);
    }

    #[test]
    fn aim_resumes_approach_on_expiry_when_not_seeing_player() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.0,
            shoot_countdown: 0.0,
            aim_countdown: 0.05,
        };
        let (state, cmds) = step(State::Aim, &mut counters, DT, &inputs(None, Some(false)), &tuning);
        assert_eq!(state, State::Approach);
        assert_eq!(counters.aim_preparing, tuning.aim_prepare_time);
        assert_eq!(counters.shoot_countdown, tuning.shoot_wait);
        assert_eq!(cmds, vec![Cmd::ResumeApproach]);
    }

    #[test]
    fn shooting_keeps_draining_aim_countdown_without_transitioning() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.0,
            shoot_countdown: tuning.shoot_wait,
            aim_countdown: 0.05,
        };
        let (state, cmds) = step(
            State::Shooting,
            &mut counters,
            DT,
            &inputs(None, Some(true)),
            &tuning,
        );
        assert_eq!(state, State::Shooting);
        assert!(counters.aim_countdown < 0.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn idle_is_a_no_op() {
        let tuning = RobotTuning::default();
        let mut counters = RobotCounters {
            aim_preparing: 0.25,
            shoot_countdown: 1.0,
            aim_countdown: 1.0,
        };
        let before = counters;
        let (state, cmds) = step(State::Idle, &mut counters, DT, &inputs(None, None), &tuning);
        assert_eq!(state, State::Idle);
        assert_eq!(counters, before);
        assert!(cmds.is_empty());
    }

    #[test]
    fn transition_request_covers_all_four_outcomes() {
        let tuning = RobotTuning::default();
        assert_eq!(
            transition_request(State::Approach, Some(tuning.player_aim_tolerance * 2.0), false, &tuning),
            "turn_left"
        );
        assert_eq!(
            transition_request(State::Approach, Some(-tuning.player_aim_tolerance * 2.0), false, &tuning),
            "turn_right"
        );
        assert_eq!(transition_request(State::Approach, Some(0.0), true, &tuning), "idle");
        assert_eq!(transition_request(State::Approach, Some(0.0), false, &tuning), "walk");
        assert_eq!(transition_request(State::Aim, Some(0.0), false, &tuning), "idle");
    }

    #[test]
    fn aim_blend_step_clamps_both_axes() {
        let tuning = RobotTuning::default();
        let result = aim_blend_step(Vector2::ZERO, 1_000_000.0, 1_000_000.0, DT, &tuning);
        assert_eq!(result.x, -1.0);
        assert_eq!(result.y, 1.0);
    }

    #[test]
    fn aim_blend_step_moves_partway_for_a_mid_range_input() {
        let tuning = RobotTuning::default();
        let result = aim_blend_step(Vector2::ZERO, 1.0, 1.0, DT, &tuning);
        assert!(result.x < 0.0 && result.x > -1.0);
        assert!(result.y > 0.0 && result.y < 1.0);
    }

    #[test]
    fn cannon_angles_sign_matches_target_side() {
        let (h_right, _) = cannon_angles(Vector3::new(1.0, 0.0, -1.0));
        let (h_left, _) = cannon_angles(Vector3::new(-1.0, 0.0, -1.0));
        assert!(h_right > 0.0);
        assert!(h_left < 0.0);
        let (_, v_up) = cannon_angles(Vector3::new(0.0, 1.0, -1.0));
        let (_, v_down) = cannon_angles(Vector3::new(0.0, -1.0, -1.0));
        assert!(v_up > 0.0);
        assert!(v_down < 0.0);
    }

    #[test]
    fn ember_position_and_extents_match_the_formulas() {
        let pos = ember_position(10.0, 0.5);
        assert_eq!(pos, Vector3::new(0.0, 0.0, -5.5));
        let extents = ember_extents(Vector3::new(1.0, 2.0, 0.0), 10.0, 0.5);
        assert_eq!(extents.x, 1.0);
        assert_eq!(extents.y, 2.0);
        assert_eq!(extents.z, 4.75);
    }

    #[test]
    fn root_motion_twin_matches_players_integration() {
        let orientation = Transform3D::IDENTITY;
        let root_motion = Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 2.0));
        let gravity = Vector3::new(0.0, -9.8, 0.0);
        let velocity_in = Vector3::new(0.0, 3.0, 0.0);
        let (new_orientation, velocity) =
            integrate_root_motion(orientation, root_motion, DT, gravity, velocity_in);
        assert_eq!(velocity.x, 1.0 / DT);
        assert_eq!(velocity.z, 2.0 / DT);
        assert_eq!(velocity.y, 3.0 + gravity.y * DT);
        assert_eq!(new_orientation.origin, Vector3::ZERO);
    }

    #[test]
    fn idle_velocity_is_gravity_times_dt() {
        let gravity = Vector3::new(0.0, -9.8, 0.0);
        assert_eq!(idle_velocity(gravity, DT), gravity * DT);
    }
}
