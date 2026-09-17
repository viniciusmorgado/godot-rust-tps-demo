// Temporary: this module is not wired into `player.rs`'s glue until commit 2 of this milestone
// (`specs/008-v2-player-bullet-door`), so nothing outside `#[cfg(test)]` calls these items yet.
// Removed in that commit.
#![allow(dead_code)]

//! gdext builtin math is pure only when its body does not go through `as_inner()` (the
//! engine) — e.g. `Quaternion::slerp` and `Basis::looking_at` do (their real computation is
//! delegated to the engine's own implementation via FFI); operators, `from_quaternion`,
//! `get_quaternion`, `from_euler`, and `orthonormalized` do not (they use `glam`, a pure Rust
//! math crate). A `#[test]` that calls an engine-backed method panics with "Godot engine not
//! available", which is the practical check. `slerp` and `looking_at` therefore stay in glue
//! (`player.rs`), called directly where `v1` calls them; nothing in this module wraps them.

use godot::prelude::*;

/// Tuning constants for `Player`'s motion/orientation/jump math — the exact `v1` literals from
/// `player.rs`.
#[derive(Clone, Copy, Debug)]
pub struct PlayerTuning {
    pub motion_interpolate_speed: f32,
    pub rotation_interpolate_speed: f32,
    pub min_airborne_time: f32,
    pub jump_speed: f32,
    pub land_threshold: f32,
    pub respawn_below_y: f32,
}

impl Default for PlayerTuning {
    fn default() -> Self {
        Self {
            motion_interpolate_speed: 10.0,
            rotation_interpolate_speed: 10.0,
            min_airborne_time: 0.1,
            jump_speed: 5.0,
            land_threshold: 0.5,
            respawn_below_y: -40.0,
        }
    }
}

/// One physics frame's snapshot of `player_input`, read through a single
/// `bind()`/`bind_mut()` acquisition.
#[derive(Clone, Copy, Debug)]
pub struct InputFrame {
    pub motion: Vector2,
    pub aiming: bool,
    pub shooting: bool,
    pub jumping: bool,
    pub shoot_target: Vector3,
    pub camera_rotation_basis: Basis,
    pub camera_base_quaternion: Quaternion,
    pub aim_rotation: f64,
}

/// The pure result of `airborne_step`. `land` and `jump` are independent flags — `v1` can fire
/// both RPCs in the same frame (landing resets `airborne_time`, which can immediately satisfy
/// the jump condition if the jump button is held that same frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirborneOutcome {
    pub airborne_time: f32,
    pub on_air: bool,
    pub land: bool,
    pub jump: bool,
    pub jump_velocity_y: Option<f32>,
}

/// Literal transcription of `v1`'s jump/land/airborne logic (`player.rs:196-214`), in v1's
/// exact order. Reordering these steps changes the observable result — do not "simplify".
pub fn airborne_step(
    airborne_time: f32,
    dt: f32,
    is_on_floor: bool,
    jump_pressed: bool,
    tuning: &PlayerTuning,
) -> AirborneOutcome {
    // v1: `self.airborne_time += delta as f32;`
    let mut airborne_time = airborne_time + dt;
    let mut land = false;

    // v1: `if self.base().is_on_floor() { if self.airborne_time > 0.5 { rpc("land") }
    //       self.airborne_time = 0.0; }`
    if is_on_floor {
        land = airborne_time > tuning.land_threshold;
        airborne_time = 0.0;
    }

    // v1: `let mut on_air: bool = self.airborne_time > MIN_AIRBORNE_TIME;` (recomputed AFTER
    // the possible reset above)
    let mut on_air = airborne_time > tuning.min_airborne_time;

    let mut jump = false;
    let mut jump_velocity_y = None;
    // v1: `if !on_air && self.player_input.bind().jumping { ...; on_air = true;
    //       self.airborne_time = MIN_AIRBORNE_TIME; rpc("jump") }`
    if !on_air && jump_pressed {
        jump = true;
        jump_velocity_y = Some(tuning.jump_speed);
        on_air = true;
        airborne_time = tuning.min_airborne_time;
    }

    AirborneOutcome {
        airborne_time,
        on_air,
        land,
        jump,
        jump_velocity_y,
    }
}

/// `v1`: `self.motion.lerp(input_motion, MOTION_INTERPOLATE_SPEED * delta as f32)`
/// (`player.rs:182-184`).
pub fn lerp_motion(current: Vector2, target: Vector2, dt: f32, tuning: &PlayerTuning) -> Vector2 {
    current.lerp(target, tuning.motion_interpolate_speed * dt)
}

/// `v1`: `camera_z = camera_basis.col_c()`, `camera_x = camera_basis.col_a()`, both with `.y =
/// 0.0` then `.normalized()` (`player.rs:187-193`). Returns `(camera_x, camera_z)`.
pub fn flatten_camera_axes(basis: Basis) -> (Vector3, Vector3) {
    let mut camera_x = basis.col_a();
    camera_x.y = 0.0;
    let camera_x = camera_x.normalized();

    let mut camera_z = basis.col_c();
    camera_z.y = 0.0;
    let camera_z = camera_z.normalized();

    (camera_x, camera_z)
}

/// `v1`: `target = camera_x*motion.x + camera_z*motion.y`; `Some(target)` if
/// `target.length() > 0.001`, else `None` (no rotation this frame) — `player.rs:264-267`. Only
/// the guard is the domain decision; turning `target` into a basis is `Basis::looking_at`, an
/// engine-required call (see this module's doc comment) left to glue.
pub fn walk_target(camera_x: Vector3, camera_z: Vector3, motion: Vector2) -> Option<Vector3> {
    let target = camera_x * motion.x + camera_z * motion.y;
    if target.length() > 0.001 { Some(target) } else { None }
}

/// `v1`'s root-motion integration (`player.rs:283-297`): apply root motion, extract horizontal
/// velocity, add gravity, clear the accumulated displacement, orthonormalize.
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

/// `v1`: `if self.base().get_transform().origin.y < -40.0` (`player.rs:303`).
pub fn should_respawn(y: f32, tuning: &PlayerTuning) -> bool {
    y < tuning.respawn_below_y
}

/// The pure animation decision, applied once by glue (`player.rs:218-234`/`:261-274`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimPlan {
    JumpUp,
    JumpDown,
    Strafe { aim_rotation: f64, blend_position: Vector2 },
    Walk { blend_position: Vector2 },
}

pub fn anim_plan(
    on_air: bool,
    velocity_y: f32,
    aiming: bool,
    motion: Vector2,
    aim_rotation: f64,
) -> AnimPlan {
    if on_air {
        if velocity_y > 0.0 {
            AnimPlan::JumpUp
        } else {
            AnimPlan::JumpDown
        }
    } else if aiming {
        // v1: Vector2::new(self.motion.x, -self.motion.y) — the animation's forward/backward
        // axis is reversed.
        AnimPlan::Strafe {
            aim_rotation,
            blend_position: Vector2::new(motion.x, -motion.y),
        }
    } else {
        AnimPlan::Walk {
            blend_position: Vector2::new(motion.length(), 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.1;

    #[test]
    fn lands_after_exceeding_the_threshold() {
        let tuning = PlayerTuning::default();
        let outcome = airborne_step(0.6, DT, true, false, &tuning);
        assert_eq!(outcome.land, true);
        assert_eq!(outcome.airborne_time, 0.0);
        assert_eq!(outcome.jump, false);
    }

    #[test]
    fn no_land_at_or_under_the_threshold() {
        let tuning = PlayerTuning::default();
        let outcome = airborne_step(0.3, DT, true, false, &tuning);
        assert_eq!(outcome.land, false);
    }

    #[test]
    fn jumps_while_grounded() {
        let tuning = PlayerTuning::default();
        let outcome = airborne_step(0.0, DT, true, true, &tuning);
        assert_eq!(outcome.jump, true);
        assert_eq!(outcome.jump_velocity_y, Some(tuning.jump_speed));
        assert_eq!(outcome.airborne_time, tuning.min_airborne_time);
        assert_eq!(outcome.on_air, true);
    }

    #[test]
    fn no_double_jump_while_already_airborne() {
        let tuning = PlayerTuning::default();
        let outcome = airborne_step(1.0, DT, false, true, &tuning);
        assert_eq!(outcome.on_air, true);
        assert_eq!(outcome.jump, false);
        assert_eq!(outcome.jump_velocity_y, None);
    }

    #[test]
    fn land_and_jump_can_both_fire_in_the_same_frame() {
        let tuning = PlayerTuning::default();
        let outcome = airborne_step(0.6, DT, true, true, &tuning);
        assert_eq!(outcome.land, true);
        assert_eq!(outcome.jump, true);
        assert_eq!(outcome.jump_velocity_y, Some(tuning.jump_speed));
        assert_eq!(outcome.airborne_time, tuning.min_airborne_time);
    }

    #[test]
    fn on_air_is_recomputed_after_the_landing_reset_not_before() {
        let tuning = PlayerTuning::default();
        // Without the reset-before-recompute order, on_air would be (0.6+0.1) > 0.1 = true.
        let outcome = airborne_step(0.6, DT, true, false, &tuning);
        assert_eq!(outcome.on_air, false);
    }

    #[test]
    fn motion_lerp_moves_partway_toward_the_target() {
        let tuning = PlayerTuning::default();
        let current = Vector2::ZERO;
        let target = Vector2::new(1.0, 0.0);
        // A small dt keeps the lerp weight (speed * dt) under 1.0, so the result is partial,
        // not a full snap to the target.
        let result = lerp_motion(current, target, 0.01, &tuning);
        assert!(result.x > 0.0 && result.x < 1.0, "result was: {result:?}");
    }

    #[test]
    fn camera_axes_are_flattened_and_normalized() {
        let basis = Basis::IDENTITY;
        let (x, z) = flatten_camera_axes(basis);
        assert_eq!(x.y, 0.0);
        assert_eq!(z.y, 0.0);
        assert!((x.length() - 1.0).abs() < 1e-5);
        assert!((z.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn walk_target_is_none_below_the_length_guard() {
        let result = walk_target(Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0), Vector2::ZERO);
        assert_eq!(result, None);
    }

    #[test]
    fn walk_target_is_some_above_the_length_guard() {
        let camera_x = Vector3::new(1.0, 0.0, 0.0);
        let camera_z = Vector3::new(0.0, 0.0, 1.0);
        let motion = Vector2::new(1.0, 0.0);
        let result = walk_target(camera_x, camera_z, motion);
        assert_eq!(result, Some(camera_x));
    }

    #[test]
    fn root_motion_integration_computes_horizontal_velocity_and_adds_gravity() {
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
    fn should_respawn_below_threshold_only() {
        let tuning = PlayerTuning::default();
        assert!(should_respawn(-41.0, &tuning));
        assert!(!should_respawn(-40.0, &tuning));
        assert!(!should_respawn(0.0, &tuning));
    }

    #[test]
    fn anim_plan_airborne_picks_jump_up_or_down_by_velocity_sign() {
        assert_eq!(anim_plan(true, 1.0, false, Vector2::ZERO, 0.0), AnimPlan::JumpUp);
        assert_eq!(anim_plan(true, -1.0, false, Vector2::ZERO, 0.0), AnimPlan::JumpDown);
    }

    #[test]
    fn anim_plan_grounded_aiming_is_strafe_with_reversed_y_blend() {
        let motion = Vector2::new(1.0, 2.0);
        let plan = anim_plan(false, 0.0, true, motion, 0.5);
        assert_eq!(
            plan,
            AnimPlan::Strafe {
                aim_rotation: 0.5,
                blend_position: Vector2::new(1.0, -2.0)
            }
        );
    }

    #[test]
    fn anim_plan_grounded_walking_is_walk_with_motion_length_blend() {
        let motion = Vector2::new(3.0, 4.0);
        let plan = anim_plan(false, 0.0, false, motion, 0.0);
        assert_eq!(
            plan,
            AnimPlan::Walk {
                blend_position: Vector2::new(5.0, 0.0)
            }
        );
    }
}

