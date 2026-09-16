# Data Model: Milestone V2-B — Leaves and player input

All types below are engine-free (no `Gd<T>`, no engine singleton, no `Variant`/`GString`/
`StringName`) except where noted — `Vector2`/`Vector3` are gdext's pure-Rust math builtins
(Principle III's named exception) and appear freely.

## `player_input/model.rs`

### `PlayerInputTuning`

```rust
#[derive(Clone, Copy, Debug)]
pub struct PlayerInputTuning {
    pub camera_controller_speed: f32,  // v1: 3.0 (rad/s-ish, rotate_y multiplier)
    pub camera_mouse_speed: f32,       // v1: 0.001
    pub camera_x_rot_min: f32,         // v1: deg_to_rad(-89.9)
    pub camera_x_rot_max: f32,         // v1: deg_to_rad(70.0)
    pub aim_speed_scale: f32,          // v1: 0.5 (controller, while aiming)
    pub aim_mouse_scale: f32,          // v1: 0.75 (mouse, while aiming)
    pub aim_hold_threshold: f32,       // v1: 0.4 (seconds before a hold becomes a toggle-eligible press)
    pub fall_fade_start_y: f32,        // v1: -17.0
    pub fall_fade_span: f32,           // v1: 15.0 (alpha reaches 1.0 — fully black — at fall_fade_start_y - fall_fade_span = -32)
    pub fall_fade_recover_rate: f32,   // v1: 4.0 (alpha *= 1 - rate*dt while above the threshold)
}
impl Default for PlayerInputTuning { /* the v1 literals above */ }
```

### `AimState`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AimState {
    Idle,
    Held { seconds: f32 },     // aim button held down; `seconds` = v1's `aiming_timer`
    Toggled { seconds: f32 },  // aiming without holding (a short tap toggled it on); the timer KEEPS counting
}
impl AimState {
    pub fn is_aiming(self) -> bool { !matches!(self, AimState::Idle) }
}
```

Replaces v1's three independently-mutable fields (`aiming: bool`, `toggled_aim: bool`,
`aiming_timer: f32`), which admitted combinations v1's own logic never produces (e.g. `aiming =
false` with `toggled_aim = true`). `seconds` IS v1's `aiming_timer`, carried by BOTH aiming
variants because v1 keeps counting while toggled (it only resets when aiming stops) — that
accumulated time is what decides whether a later press-and-release while toggled ends the aim
(accumulated > 0.4 s) or re-toggles it (≤ 0.4 s). There is no separate `aiming_timer` field in v2. The replicated `aiming: bool` field is the per-frame
projection `AimState::is_aiming`, written by the apply step every frame (research.md R4).

### `CameraCue`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraCue {
    Shoot,  // v1: camera_animation.play("shoot") — aiming just started
    Far,    // v1: camera_animation.play("far")   — aiming just stopped
}
```

Returned by `step_aim` on the frame `is_aiming` flips (v1: `if self.aiming != current_aim`),
`None` otherwise — the apply step plays the animation only on `Some`, never every frame.

### `InputSnapshot`

```rust
#[derive(Clone, Copy, Debug)]
pub struct InputSnapshot {
    pub motion: Vector2,             // (move_right - move_left, move_back - move_forward)
    pub camera_move: Vector2,        // (view_right - view_left, view_up - view_down)
    pub aim_just_pressed: bool,
    pub aim_pressed: bool,
    pub aim_just_released: bool,
    pub jump_just_pressed: bool,
    pub shoot_pressed: bool,
}
```

Built once per `process` call from 8 `Input::singleton().get_action_strength`/
`is_action_*_pressed` reads (glue, in `player_input.rs`, not here). The `input()` mouse-motion
callback does NOT populate an `InputSnapshot` (it fires on a different event, not every frame);
it reads `event.get_screen_relative()` directly and feeds the SAME pure rotation functions
(`scaled_mouse_look`, `clamp_pitch`) `InputSnapshot`'s `camera_move`-driven path also uses.

### Pure functions

```rust
pub fn step_aim(state: AimState, snap: &InputSnapshot, dt: f32, tuning: &PlayerInputTuning)
    -> (AimState, Option<CameraCue>);

pub fn scaled_look(raw: Vector2, aiming: bool, dt: f32, tuning: &PlayerInputTuning) -> Vector2;
// controller path: raw * dt * tuning.camera_controller_speed, then * tuning.aim_speed_scale if aiming

pub fn scaled_mouse_look(raw: Vector2, aiming: bool, tuning: &PlayerInputTuning) -> Vector2;
// mouse path: raw * tuning.camera_mouse_speed, then * tuning.aim_mouse_scale if aiming (no dt)

pub fn clamp_pitch(current: f32, delta_y: f32, tuning: &PlayerInputTuning) -> f32;
// (current + delta_y).clamp(tuning.camera_x_rot_min, tuning.camera_x_rot_max)

pub fn alpha_for_height(y: f32, prev_alpha: f32, dt: f32, tuning: &PlayerInputTuning) -> f32;
// v1 (player_input.rs:138-146): the black overlay's alpha RISES as the player falls:
// y < fall_fade_start_y (-17)  => ((fall_fade_start_y - y) / fall_fade_span).min(1.0)   // 1.0 (fully black) at -32
// y >= fall_fade_start_y       => prev_alpha * (1.0 - fall_fade_recover_rate * dt)      // fades back out; NO .max(0.0) in v1 (keep byte-identical)

pub fn aim_rotation(camera_x_rot: f32, tuning: &PlayerInputTuning) -> f64;
// v1's get_aim_rotation(): pure function of the current pitch, reused by player.rs unchanged
```

`step_aim` reproduces v1's `process` (`player_input.rs:84-100`) LITERALLY, in v1's evaluation
order — it is a direct transcription, not a redesigned machine:

```rust
let (was_aiming, toggled, seconds) = match state {
    AimState::Idle => (false, false, 0.0),
    AimState::Held { seconds } => (true, false, seconds),
    AimState::Toggled { seconds } => (true, true, seconds),
};
// v1: `if just_released && aiming_timer <= AIM_HOLD_THRESHOLD { current_aim = true; toggled = true }`
let (aim_now, toggled_next) = if snap.aim_just_released && seconds <= tuning.aim_hold_threshold {
    (true, true)                                   // released EARLY → becomes a toggle
} else {
    // v1: `current_aim = toggled_aim || pressed; if just_pressed { toggled_aim = false }`
    (toggled || snap.aim_pressed, if snap.aim_just_pressed { false } else { toggled })
};
// v1: `if current_aim { aiming_timer += dt } else { aiming_timer = 0.0 }`
let seconds_next = if aim_now { seconds + dt } else { 0.0 };
let next = match (aim_now, toggled_next) {
    (false, _) => AimState::Idle,
    (true, true) => AimState::Toggled { seconds: seconds_next },
    (true, false) => AimState::Held { seconds: seconds_next },
};
// v1: `if self.aiming != current_aim { play("shoot" | "far") }`
let cue = match (was_aiming, aim_now) {
    (false, true) => Some(CameraCue::Shoot),
    (true, false) => Some(CameraCue::Far),
    _ => None,
};
(next, cue)
```

Consequences to pin in the unit tests (all v1 behavior, none of it "designed" here):
- press and HOLD → `Held`, aiming while held; release after MORE than 0.4 s → `Idle` (`Far`).
- press and release within ≤ 0.4 s (a tap) → `Toggled` (aim stays on); press again while
  toggled → `Held` (still aiming, no cue), and the release then ends aiming only if the
  ACCUMULATED seconds (toggle time included) exceed 0.4 s — a double tap within 0.4 s total
  re-toggles instead.
- press and release in the SAME frame (`just_pressed && just_released`) from `Idle` → `Toggled`
  (v1's release check runs first with `aiming_timer = 0`).
- `Toggled` with no input → `Toggled { seconds + dt }` (the timer keeps counting).

## `camera_noise_shake/model.rs`

### `CameraShakeTuning`

```rust
#[derive(Clone, Copy, Debug)]
pub struct CameraShakeTuning {
    pub speed: f32,       // v1: 1.0 — noise sample position advances by dt * speed * 5000.0 per frame (v1's "magic number", camera_noise_shake.rs:66-67)
    pub decay_rate: f32,  // v1: 1.5
    pub max_yaw: f32,     // v1: 0.05
    pub max_pitch: f32,   // v1: 0.05
    pub max_roll: f32,    // v1: 0.1
    pub max_trauma: f32,  // v1: 1.2 — add_trauma's clamp ceiling
}
impl Default for CameraShakeTuning { /* the v1 literals above */ }
```

### Pure functions

```rust
pub fn decay(trauma: f32, dt: f32, tuning: &CameraShakeTuning) -> f32;
// (trauma - tuning.decay_rate * dt).max(0.0)

pub fn advance_time(time: f64, dt: f64, tuning: &CameraShakeTuning) -> f64;
// time + dt * tuning.speed as f64 * 5000.0   (v1: `self.time += delta * SPEED as f64 * 5000.0`, the magic 5000 is part of the formula)

pub fn shake(trauma: f32) -> f32;
// trauma * trauma  (v1: pow(trauma, 2))

pub fn offsets(shake: f32, samples: [f32; 3], tuning: &CameraShakeTuning) -> Vector3;
// samples[0] = noise at `seed`, samples[1] = noise at `seed+1`, samples[2] = noise at `seed+2`
// Vector3 { x: tuning.max_pitch * shake * samples[1],
//           y: tuning.max_yaw   * shake * samples[0],
//           z: tuning.max_roll  * shake * samples[2] }
// (v1's exact, non-alphabetical axis↔seed-offset mapping — research.md R5)

pub fn add_trauma(current: f32, amount: f32, tuning: &CameraShakeTuning) -> f32;
// (current + amount).min(tuning.max_trauma)
```

`add_trauma` here is the pure clamp function; the glue method of the same name in
`camera_noise_shake.rs` (`pub(crate) fn add_trauma(&mut self, amount: f64)`, no `#[func]` per
FR-013) calls it and stores the result — the pure/glue split keeps the name, only one is a
`Gd`-bound method.

## `debug_label.rs` (inline `mod pure`)

```rust
mod pure {
    pub struct DebugStats {
        pub fps: f64,
        pub vsync_enabled: bool,
        pub ram_bytes: u64,
        pub vram_bytes: u64,
        pub multiplayer_id: Option<i64>,  // None = offline (v1's "Online: No"); Some(id) = online
    }

    pub fn compose(stats: &DebugStats) -> String {
        // "FPS: {fps}\nVSync: {on/off}\nMemory: {mib:3.2} MiB\nVRAM: {mib:3.2} MiB\n
        //  Online: {Yes/No}[\nMultiplayer ID: {id}]"
        // fps formatted via format_godot_float (below); ram/vram via format!("{:3.2}", bytes as f64 / 1_048_576.0)
    }

    fn format_godot_float(v: f64) -> String {
        let s = format!("{v}");
        if s.contains('.') { s } else { format!("{s}.0") }
    }
}
```

`format_godot_float` reproduces `Variant::from(f64).stringify()` for whole-number values
(`60.0`) and for short fractions; it is NOT a general reimplementation of Godot's float printing
(Godot prints `0.1 + 0.2` as `0.3`, Rust as `0.30000000000000004`). That is sufficient because
`Engine.get_frames_per_second()` is always integer-valued; the parity harness confirms the bytes.

## `part_disappear.rs`, `blast.rs`

No new pure types — both close backlog #5 (the async pattern), not a state remodel. The one
shared "entity" between them is **the pattern itself**, stated once in `CLAUDE.md`'s "Port
conventions (v2)" (research.md R7's verbatim paragraph) rather than as a reusable Rust type: a
`Gd<Self>` captured at spawn time, an `is_instance_valid()` check after every `.await`, and
`to_future()`/`to_fallible_future()` chosen per the awaited signal's emitter lifetime.

`blast.rs`'s `camera: Option<Gd<Camera3D>>` (already resolved once at `ready` in `v1` — no
per-frame re-resolution to remove) is unchanged in shape by this milestone; only the timer-await
body around it changes.

## Cross-references to `player.rs` (unchanged surface, verified by grep in spec.md's context
table and research.md R9)

| `player.rs` reads/writes | Type after this milestone | Changed? |
|---|---|---|
| `.motion` | `Vector2` | no |
| `.aiming` (read) | `bool` | no (still the `AimState` projection) |
| `.shooting` (read+write) | `bool` | no |
| `.jumping` (read+write) | `bool` | no (name/type preserved; export-vs-replicate decided in US1, see plan.md Constitution Check) |
| `.shoot_target` | `Vector3` | no |
| `.camera_camera` | `OnEditor<Gd<Camera3D>>` (was `Option<Gd<Camera3D>>`) | **yes** — `player.rs:138`'s `.clone().unwrap()` → `.clone()` |
| `.get_aim_rotation()` | `fn(&self) -> f64` | no |
| `.get_camera_rotation_basis()` | unchanged | no |
| `.get_camera_base_quaternion()` | unchanged | no |
| `.add_camera_shake_trauma(f64)` (on `Player`, calls `CameraNoiseShake::add_trauma`) | unchanged `#[rpc]` | no |
