# Data Model: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

Every signature below carries the exact `v1` line range it reproduces (research.md's R1-R7
evidence). Pure functions are `Gd`-free, `Variant`-free, tested with `cargo test`.

## `red_robot/model.rs`

```rust
pub struct RobotTuning {
    pub player_aim_tolerance: f32,  // v1: PLAYER_AIM_TOLERANCE_DEGREES = 15.0.to_radians() (:21) — RENAMED, value unchanged (radians)
    pub shoot_wait: f32,            // v1: SHOOT_WAIT = 6.0 (:23)
    pub aim_time: f32,              // v1: AIM_TIME = 1.0 (:24)
    pub aim_prepare_time: f32,      // v1: AIM_PREPARE_TIME = 0.5 (:26)
    pub blend_aim_speed: f32,       // v1: BLEND_AIM_SPEED = 0.05 (:27)
    pub removal_delay: f32,         // v1: literal 10.0 in hit's create_timer (:305)
    pub trauma_delay: f32,          // v1: literal 0.1 in shoot's create_timer (:395)
    pub trauma_amount: f64,         // v1: literal 13.0 passed to add_camera_shake_trauma (:399)
}
// Default matches every v1 literal above.
```

```rust
/// v1: red_robot.rs:140-235's raycast timing (the pre-check only, NOT the raycast itself —
/// research.md R3). Both one-line, both unit-tested.
pub fn shoot_countdown_will_expire(shoot_countdown: f32, dt: f32) -> bool {
    shoot_countdown - dt < 0.0
}
pub fn aim_countdown_will_expire(aim_countdown: f32, dt: f32) -> bool {
    aim_countdown - dt < 0.0
}
```

```rust
pub struct RobotInputs {
    pub has_player: bool,
    pub angle_to_player: Option<f32>,  // None iff !has_player; radians, atan2 of the
                                        // transposed-basis-local target vector (glue computes)
    pub sees_player: Option<bool>,     // Some(...) only on a frame glue raycasted: Approach ⇒
                                        // `facing(angle, tol) && shoot_countdown_will_expire`;
                                        // Aim ⇒ `aim_countdown_will_expire` (v1 raycasts only
                                        // there: :152-157 and :206). Some(...) when `..._will_expire` returned
                                        // true for the CURRENT state (research.md R3); None
                                        // otherwise — step() never consults it when None.
}

pub enum Cmd {
    RpcPlayShoot,     // v1: self.base_mut().rpc("play_shoot", &[]) (:230)
    ResumeApproach,   // v1: else branch calling self.resume_approach() (:232)
}

pub struct RobotCounters {
    pub aim_preparing: f32,
    pub shoot_countdown: f32,
    pub aim_countdown: f32,
}

/// v1: red_robot.rs:140-235, exact order and thresholds.
pub fn step(
    state: State,
    counters: &mut RobotCounters,
    dt: f32,
    inputs: &RobotInputs,
    tuning: &RobotTuning,
) -> (State, Vec<Cmd>) {
    // Approach (:140-189):
    //   if counters.aim_preparing > 0.0 { counters.aim_preparing -= dt; clamp to 0.0 }
    //   if let Some(angle) = inputs.angle_to_player
    //       && angle > -tuning.player_aim_tolerance && angle < tuning.player_aim_tolerance {
    //     counters.shoot_countdown -= dt;
    //     if counters.shoot_countdown < 0.0 {
    //       match inputs.sees_player {
    //         Some(true) => { state = Aim; counters.aim_countdown = tuning.aim_time;
    //                         counters.aim_preparing = 0.0; }               // :181-183, forced
    //         _ => { counters.shoot_countdown = tuning.shoot_wait; }        // :186, retry later
    //       }
    //     }
    //   }
    // Aim | Shooting (:190-234):
    //   if counters.aim_preparing < tuning.aim_prepare_time { += dt; clamp to aim_prepare_time }
    //   counters.aim_countdown -= dt;                         // UNCONDITIONAL, both states (:205)
    //   if counters.aim_countdown < 0.0 && state == Aim {      // gate explicitly excludes Shooting
    //     match inputs.sees_player {
    //       Some(true) => { state = Shooting; counters.shoot_countdown = tuning.shoot_wait;
    //                        cmds.push(Cmd::RpcPlayShoot); }
    //       _ => { let (p, s) = resume_approach_reset(tuning);
    //              counters.aim_preparing = p; counters.shoot_countdown = s; state = Approach;
    //              cmds.push(Cmd::ResumeApproach); }
    //     }
    //   }
    // Idle: no-op (matches v1 — no `if self.state == State::Idle` branch exists at all).
}

/// v1: resume_approach()'s counter reset (:271-273), shared by the #[func] and step()'s
/// Cmd::ResumeApproach path so the reset formula lives in exactly one place.
pub fn resume_approach_reset(tuning: &RobotTuning) -> (f32, f32) {
    (tuning.aim_prepare_time, tuning.shoot_wait)
}
```

```rust
/// v1: red_robot.rs:406-456 ("animate"), decomposed.
pub fn transition_request(
    state: State,
    angle_to_player: Option<f32>,
    target_is_zero: bool,
    tuning: &RobotTuning,
) -> &'static str {
    // Approach: turn_left / turn_right outside tolerance, else idle if target_is_zero else walk.
    // Any other state: "idle" unconditionally (v1's else branch, :425-428).
}

pub fn aim_blend_amount(aim_preparing: f32, tuning: &RobotTuning) -> f32 {
    (aim_preparing / tuning.aim_prepare_time).clamp(0.0, 1.0)
}

pub fn cannon_angles(to_cannon_local: Vector3) -> (f32, f32) {
    // (h_angle, v_angle) in DEGREES — v1:440-441
    (
        to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees(),
        to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees(),
    )
}

pub fn aim_blend_step(blend: Vector2, h_angle: f32, v_angle: f32, dt: f32, tuning: &RobotTuning) -> Vector2 {
    // v1:446-452 — blend.x += blend_aim_speed*dt*-h_angle, clamp [-1,1]; blend.y += blend_aim_speed*dt*v_angle, clamp [-1,1]
}

pub fn ember_position(max_dist: f32, mesh_offset: f32) -> Vector3 {
    Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset) // v1:372
}

pub fn ember_extents(current: Vector3, max_dist: f32, mesh_offset: f32) -> Vector3 {
    let mut extents = current;
    extents.z = (max_dist - mesh_offset.abs()) / 2.0; // v1:373-374, x/y untouched
    extents
}
```

```rust
/// TWIN of player/model.rs's integrate_root_motion (V2-C) — identical body, kept independent
/// per research.md R6 (player.rs stays untouched by this milestone).
/// v1: red_robot.rs:238-257.
pub fn integrate_root_motion(
    orientation: Transform3D,
    root_motion: Transform3D,
    dt: f32,
    gravity: Vector3,
    velocity_in: Vector3,
) -> (Transform3D, Vector3) { /* identical to player/model.rs:137-156 */ }

/// v1: red_robot.rs:128-136 (the no-player branch's velocity).
pub fn idle_velocity(gravity: Vector3, dt: f32) -> Vector3 {
    gravity * dt
}
```

```rust
/// v1: hit's decrement + death-transition decision only (red_robot.rs:284-285). The RNG hit-
/// reaction pick and every engine effect stay in glue (research.md R5) — this function decides
/// NOTHING about them; it only reports whether the death sequence should run this call.
pub fn hit_step(health: i32) -> (i32, bool) {
    let new = health - 1;
    (new, health > 0 && new == 0)
}
```

**`RayHit`** (glue-facing return of the ONE raycast helper, `red_robot.rs`, not pure — contains
`Gd<Object>`):

```rust
pub struct RayHit {
    pub position: Vector3,
    pub collider: Option<Gd<Object>>,
}
```

## `part.rs`'s inline `mod pure`

```rust
/// v1: part.rs:54.
pub fn fade_curve(counter: f32, disappearing_time: f32) -> f32 {
    (counter / disappearing_time).powi(2)
}

/// v1: part.rs:57.
pub fn should_destroy(counter: f32, disappearing_time: f32) -> bool {
    counter >= disappearing_time - 0.2
}

/// v1: part.rs:90-93. Three already-sampled [0.0, 1.0) inputs; RNG stays in glue (randf()).
pub fn random_angular_velocity(r1: f32, r2: f32, r3: f32) -> Vector3 {
    (Vector3::new(r1, r2, r3).normalized() * 2.0 - Vector3::ONE) * 10.0
}

/// v1: part.rs:95. One already-sampled [0.0, 1.0) input.
pub fn wait_time(lifetime: f32, lifetime_random: f32, r: f32) -> f32 {
    lifetime + lifetime_random * r
}
```

## Cross-reference: what stays exactly as it is

| Surface | Stays |
|---|---|
| `red_robot.rs` replicated `#[export]`s | `target_position: Vector3`, `health: i32`, `state: State`, `dead: bool` — names, types, `SceneReplicationConfig` modes/wire codes unchanged (FR-001) |
| `red_robot.rs` signals/RPCs/funcs | `#[signal] exploded()`, `#[rpc] hit`/`play_shoot`, `#[func] resume_approach`/`shoot_check`/`_on_area_body_entered`/`_on_area_body_exited` — exact signatures (FR-001) |
| `part.rs` replicated/exported surface | `fade_value` (+ `set_fade_value`), `lifetime`, `lifetime_random`, `disappearing_time`, `explode` (`#[func]`), `destroy` (`#[rpc]`) — exact signatures (FR-013) |
| `HitTarget`/`hittable.rs` | Untouched; `EnemyRobot` remains its `Robot` variant, resolved by `bullet.rs` |
| Upstream bug fix #2 (`part.rs::ready`) | Untouched, in place, `// upstream bug fix` comment kept (FR-019) |

## State transition summary (`State`, unchanged wire enum)

```text
Idle --(_on_area_body_entered: Player)--> Approach
Approach --(_on_area_body_exited)--> Idle
Approach --(shoot_countdown expires, sees_player)--> Aim
Approach --(shoot_countdown expires, !sees_player)--> Approach (shoot_countdown reset)
Aim --(aim_countdown expires, sees_player)--> Shooting
Aim --(aim_countdown expires, !sees_player)--> Approach (via resume_approach_reset)
Shooting --(aim_countdown expires)--> no-op (state==Aim guard excludes Shooting, v1 quirk kept)
Shooting --(animation method track fires resume_approach())--> Approach (unconditional)
Aim | Shooting --(_on_area_body_exited)--> Idle
```
