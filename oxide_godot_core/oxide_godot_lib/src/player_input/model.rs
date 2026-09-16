// Temporary: this module is not wired into `player_input.rs`'s glue until commit 2 of this
// milestone (`specs/007-v2-leaves-and-input`), so nothing outside `#[cfg(test)]` calls these
// items yet. Removed in that commit.
#![allow(dead_code)]

use godot::prelude::*;

/// Tuning constants for `PlayerInputSynchronizer`'s camera rotation, aim hold/toggle threshold
/// and fall-to-black fade — the exact `v1` literals from `player_input.rs`.
#[derive(Clone, Copy, Debug)]
pub struct PlayerInputTuning {
    pub camera_controller_speed: f32,
    pub camera_mouse_speed: f32,
    pub camera_x_rot_min: f32,
    pub camera_x_rot_max: f32,
    pub aim_speed_scale: f32,
    pub aim_mouse_scale: f32,
    pub aim_hold_threshold: f32,
    pub fall_fade_start_y: f32,
    pub fall_fade_span: f32,
    pub fall_fade_recover_rate: f32,
}

impl Default for PlayerInputTuning {
    fn default() -> Self {
        Self {
            camera_controller_speed: 3.0,
            camera_mouse_speed: 0.001,
            // A minimum angle lower than or equal to -90 breaks movement if the player is
            // looking upward.
            camera_x_rot_min: (-89.9_f32).to_radians(),
            camera_x_rot_max: 70.0_f32.to_radians(),
            aim_speed_scale: 0.5,
            aim_mouse_scale: 0.75,
            aim_hold_threshold: 0.4,
            fall_fade_start_y: -17.0,
            fall_fade_span: 15.0,
            fall_fade_recover_rate: 4.0,
        }
    }
}

/// Replaces `v1`'s three independently-mutable fields (`aiming`, `toggled_aim`,
/// `aiming_timer`), which admitted combinations `v1`'s own logic never produces. `seconds` IS
/// `v1`'s `aiming_timer`, carried by both aiming variants because `v1` keeps counting while
/// toggled (it only resets when aiming stops).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AimState {
    Idle,
    Held { seconds: f32 },
    Toggled { seconds: f32 },
}

impl AimState {
    pub fn is_aiming(self) -> bool {
        !matches!(self, AimState::Idle)
    }
}

/// The pure aim-transition output driving `camera_animation`'s play calls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraCue {
    Shoot,
    Far,
}

/// One frame's worth of `Input` reads, built once at the top of `process`.
#[derive(Clone, Copy, Debug)]
pub struct InputSnapshot {
    pub motion: Vector2,
    pub camera_move: Vector2,
    pub aim_just_pressed: bool,
    pub aim_pressed: bool,
    pub aim_just_released: bool,
    pub jump_just_pressed: bool,
    pub shoot_pressed: bool,
}

/// Literal transcription of `v1`'s `process` aim logic (`player_input.rs:88-114`), in `v1`'s
/// exact evaluation order. This is a direct port, not a redesigned state machine — do not
/// "simplify" the branching.
pub fn step_aim(
    state: AimState,
    snap: &InputSnapshot,
    dt: f32,
    tuning: &PlayerInputTuning,
) -> (AimState, Option<CameraCue>) {
    let (was_aiming, toggled, seconds) = match state {
        AimState::Idle => (false, false, 0.0),
        AimState::Held { seconds } => (true, false, seconds),
        AimState::Toggled { seconds } => (true, true, seconds),
    };

    // v1: `if just_released && aiming_timer <= AIM_HOLD_THRESHOLD { current_aim = true; toggled_aim = true }`
    let (aim_now, toggled_next) = if snap.aim_just_released && seconds <= tuning.aim_hold_threshold
    {
        (true, true)
    } else {
        // v1: `current_aim = toggled_aim || pressed; if just_pressed { toggled_aim = false }`
        (
            toggled || snap.aim_pressed,
            if snap.aim_just_pressed { false } else { toggled },
        )
    };

    // v1: `if current_aim { aiming_timer += dt } else { aiming_timer = 0.0 }`
    let seconds_next = if aim_now { seconds + dt } else { 0.0 };

    let next = match (aim_now, toggled_next) {
        (false, _) => AimState::Idle,
        (true, true) => AimState::Toggled {
            seconds: seconds_next,
        },
        (true, false) => AimState::Held {
            seconds: seconds_next,
        },
    };

    // v1: `if self.aiming != current_aim { play("shoot" | "far") }`
    let cue = match (was_aiming, aim_now) {
        (false, true) => Some(CameraCue::Shoot),
        (true, false) => Some(CameraCue::Far),
        _ => None,
    };

    (next, cue)
}

/// Controller look-delta scaling: `v1`'s `process` (`player_input.rs:83-87`).
pub fn scaled_look(raw: Vector2, aiming: bool, dt: f32, tuning: &PlayerInputTuning) -> Vector2 {
    let mut speed = dt * tuning.camera_controller_speed;
    if aiming {
        speed *= tuning.aim_speed_scale;
    }
    raw * speed
}

/// Mouse look-delta scaling: `v1`'s `input` (`player_input.rs:172-176`). No `dt` factor.
pub fn scaled_mouse_look(raw: Vector2, aiming: bool, tuning: &PlayerInputTuning) -> Vector2 {
    let mut speed = tuning.camera_mouse_speed;
    if aiming {
        speed *= tuning.aim_mouse_scale;
    }
    raw * speed
}

/// `v1`'s `rotate_camera` pitch clamp (`player_input.rs:230-231`).
pub fn clamp_pitch(current: f32, delta_y: f32, tuning: &PlayerInputTuning) -> f32 {
    (current + delta_y).clamp(tuning.camera_x_rot_min, tuning.camera_x_rot_max)
}

/// Fall-to-black fade: `v1`'s `process` (`player_input.rs:150-167`). Byte-identical, including
/// the absence of a `.max(0.0)` floor on the decay branch.
pub fn alpha_for_height(y: f32, prev_alpha: f32, dt: f32, tuning: &PlayerInputTuning) -> f32 {
    if y < tuning.fall_fade_start_y {
        ((tuning.fall_fade_start_y - y) / tuning.fall_fade_span).min(1.0)
    } else {
        prev_alpha * (1.0 - tuning.fall_fade_recover_rate * dt)
    }
}

/// `v1`'s `get_aim_rotation` (`player_input.rs:184-199`).
pub fn aim_rotation(camera_x_rot: f32, tuning: &PlayerInputTuning) -> f64 {
    let camera_x_rot = camera_x_rot.clamp(tuning.camera_x_rot_min, tuning.camera_x_rot_max);
    if camera_x_rot >= 0.0 {
        // Aim up.
        (-camera_x_rot / tuning.camera_x_rot_max) as f64
    } else {
        // Aim down.
        (camera_x_rot / tuning.camera_x_rot_min) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(
        aim_just_pressed: bool,
        aim_pressed: bool,
        aim_just_released: bool,
    ) -> InputSnapshot {
        InputSnapshot {
            motion: Vector2::ZERO,
            camera_move: Vector2::ZERO,
            aim_just_pressed,
            aim_pressed,
            aim_just_released,
            jump_just_pressed: false,
            shoot_pressed: false,
        }
    }

    fn idle_snap() -> InputSnapshot {
        snap(false, false, false)
    }

    const DT: f32 = 0.1;

    #[test]
    fn idle_plus_press_becomes_held_with_shoot_cue() {
        let tuning = PlayerInputTuning::default();
        let (state, cue) = step_aim(AimState::Idle, &snap(true, true, false), DT, &tuning);
        assert_eq!(state, AimState::Held { seconds: DT });
        assert_eq!(cue, Some(CameraCue::Shoot));
    }

    #[test]
    fn hold_past_threshold_then_release_returns_to_idle_with_far_cue() {
        let tuning = PlayerInputTuning::default();
        // Held for 0.5s (> AIM_HOLD_THRESHOLD 0.4), then released.
        let state = AimState::Held { seconds: 0.5 };
        let (state, cue) = step_aim(state, &snap(false, false, true), DT, &tuning);
        assert_eq!(state, AimState::Idle);
        assert_eq!(cue, Some(CameraCue::Far));
    }

    #[test]
    fn tap_within_threshold_toggles_and_stays_aiming() {
        let tuning = PlayerInputTuning::default();
        // Held for only 0.2s (<= AIM_HOLD_THRESHOLD 0.4), then released: becomes a toggle.
        let state = AimState::Held { seconds: 0.2 };
        let (state, cue) = step_aim(state, &snap(false, false, true), DT, &tuning);
        assert!(state.is_aiming());
        assert!(matches!(state, AimState::Toggled { .. }));
        // Still aiming (was true, is true) -> no cue.
        assert_eq!(cue, None);
    }

    #[test]
    fn press_while_toggled_becomes_held_then_release_past_threshold_ends_aim() {
        let tuning = PlayerInputTuning::default();
        // Press while toggled -> Held (still aiming, no cue), seconds keeps accumulating from
        // the toggled seconds (0.3 -> after this frame's dt).
        let toggled = AimState::Toggled { seconds: 0.3 };
        let (state, cue) = step_aim(toggled, &snap(true, true, false), DT, &tuning);
        assert_eq!(state, AimState::Held { seconds: 0.3 + DT });
        assert_eq!(cue, None);

        // Release with accumulated seconds (0.4) > AIM_HOLD_THRESHOLD (0.4 is not > 0.4, so
        // bump the accumulated seconds past it explicitly for this scenario).
        let held = AimState::Held { seconds: 0.5 };
        let (state, cue) = step_aim(held, &snap(false, false, true), DT, &tuning);
        assert_eq!(state, AimState::Idle);
        assert_eq!(cue, Some(CameraCue::Far));
    }

    #[test]
    fn double_tap_within_threshold_total_re_toggles() {
        let tuning = PlayerInputTuning::default();
        // Toggled at 0.1s total, a press resets toggled->held with the SAME accumulated
        // seconds, then an immediate release with seconds still <= threshold re-toggles.
        let toggled = AimState::Toggled { seconds: 0.1 };
        let (state, _) = step_aim(toggled, &snap(true, true, false), DT, &tuning);
        assert_eq!(state, AimState::Held { seconds: 0.1 + DT });

        let (state, cue) = step_aim(state, &snap(false, false, true), DT, &tuning);
        // 0.1 + DT + DT = 0.3, still <= 0.4 -> re-toggles, no cue (still aiming throughout).
        assert!(matches!(state, AimState::Toggled { .. }));
        assert_eq!(cue, None);
    }

    #[test]
    fn same_frame_press_and_release_from_idle_toggles() {
        let tuning = PlayerInputTuning::default();
        // v1's release check runs first, with aiming_timer == 0.0 (Idle), which satisfies
        // `<= AIM_HOLD_THRESHOLD`.
        let (state, cue) = step_aim(AimState::Idle, &snap(true, true, true), DT, &tuning);
        assert!(matches!(state, AimState::Toggled { .. }));
        assert_eq!(cue, Some(CameraCue::Shoot));
    }

    #[test]
    fn toggled_with_no_input_keeps_counting() {
        let tuning = PlayerInputTuning::default();
        let toggled = AimState::Toggled { seconds: 1.0 };
        let (state, cue) = step_aim(toggled, &idle_snap(), DT, &tuning);
        assert_eq!(
            state,
            AimState::Toggled {
                seconds: 1.0 + DT
            }
        );
        assert_eq!(cue, None);
    }

    #[test]
    fn scaled_look_is_halved_while_aiming() {
        let tuning = PlayerInputTuning::default();
        let raw = Vector2::new(1.0, 0.0);
        let not_aiming = scaled_look(raw, false, 1.0, &tuning);
        let aiming = scaled_look(raw, true, 1.0, &tuning);
        assert_eq!(aiming.x, not_aiming.x * tuning.aim_speed_scale);
    }

    #[test]
    fn scaled_mouse_look_is_scaled_by_0_75_while_aiming() {
        let tuning = PlayerInputTuning::default();
        let raw = Vector2::new(1.0, 0.0);
        let not_aiming = scaled_mouse_look(raw, false, &tuning);
        let aiming = scaled_mouse_look(raw, true, &tuning);
        assert_eq!(aiming.x, not_aiming.x * tuning.aim_mouse_scale);
    }

    #[test]
    fn clamp_pitch_clamps_both_ends() {
        let tuning = PlayerInputTuning::default();
        let too_high = clamp_pitch(tuning.camera_x_rot_max, 10.0, &tuning);
        assert_eq!(too_high, tuning.camera_x_rot_max);
        let too_low = clamp_pitch(tuning.camera_x_rot_min, -10.0, &tuning);
        assert_eq!(too_low, tuning.camera_x_rot_min);
    }

    #[test]
    fn alpha_rises_to_1_0_at_the_bottom_of_the_fall_span() {
        let tuning = PlayerInputTuning::default();
        // fall_fade_start_y - fall_fade_span = -17.0 - 15.0 = -32.0
        let alpha = alpha_for_height(-32.0, 0.0, DT, &tuning);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn alpha_decays_above_the_fall_fade_start_y() {
        let tuning = PlayerInputTuning::default();
        let alpha = alpha_for_height(0.0, 1.0, DT, &tuning);
        assert_eq!(alpha, 1.0 - tuning.fall_fade_recover_rate * DT);
    }

    #[test]
    fn aim_rotation_covers_both_branches() {
        let tuning = PlayerInputTuning::default();
        let up = aim_rotation(0.5, &tuning);
        assert_eq!(up, (-0.5_f32 / tuning.camera_x_rot_max) as f64);
        let down = aim_rotation(-0.5, &tuning);
        assert_eq!(down, (-0.5_f32 / tuning.camera_x_rot_min) as f64);
    }
}
