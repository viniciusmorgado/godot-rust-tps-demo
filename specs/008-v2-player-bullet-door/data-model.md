# Data Model: Milestone V2-C — player, bullet, door

All types below are engine-free (no `Gd<T>`, no engine singleton, no `Variant`/`GString`/
`StringName`) except `HitTarget` (explicitly glue, see `hittable.rs`) — `Vector2`/`Vector3`/
`Basis`/`Quaternion`/`Transform3D` are gdext's pure-Rust math builtins (Principle III's named
exception) and appear freely.

## `player/model.rs`

### `PlayerTuning`

```rust
#[derive(Clone, Copy, Debug)]
pub struct PlayerTuning {
    pub motion_interpolate_speed: f32,   // v1: 10.0 (player.rs:19)
    pub rotation_interpolate_speed: f32, // v1: 10.0 (player.rs:20)
    pub min_airborne_time: f32,          // v1: 0.1  (player.rs:22)
    pub jump_speed: f32,                 // v1: 5.0  (player.rs:23)
    pub land_threshold: f32,             // v1: 0.5  (player.rs:198, inline)
    pub respawn_below_y: f32,            // v1: -40.0 (player.rs:303, inline)
}
impl Default for PlayerTuning { /* the v1 literals above */ }
```

### `InputFrame`

```rust
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
```
Built by glue from ONE `self.player_input.bind_mut()` acquisition (research.md R1); the guard
also writes `jumping = false` back before being dropped. Not constructed on the non-authority
path (which only ever needs `aim_rotation`, fetched directly when required — see
`player.rs`'s glue design in research.md R3).

### `AirborneOutcome` and `airborne_step`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirborneOutcome {
    pub airborne_time: f32,
    pub on_air: bool,
    pub land: bool,             // v1: player.rs:199 `rpc("land")`
    pub jump: bool,             // v1: player.rs:213 `rpc("jump")`
    pub jump_velocity_y: Option<f32>, // Some(tuning.jump_speed) iff `jump`
}

pub fn airborne_step(
    airborne_time: f32,
    dt: f32,
    is_on_floor: bool,
    jump_pressed: bool,
    tuning: &PlayerTuning,
) -> AirborneOutcome;
```
Reproduces `player.rs:196-214` in v1's exact order — see research.md R2 for the 4-step
derivation. **`land` and `jump` are independent flags, not mutually exclusive**: a frame that
lands (floor contact after exceeding `land_threshold`, which unconditionally resets
`airborne_time` to `0`) can, in the SAME frame, also satisfy the post-reset `on_air == false`
jump condition if `jumping` is held — both `rpc`s fire in `v1`, so the pure function must expose
both as independently-`true`-able fields (spec Scenario 4, FR-005's dedicated test case).

### Motion, camera, and orientation helpers

```rust
pub fn lerp_motion(current: Vector2, target: Vector2, dt: f32, tuning: &PlayerTuning) -> Vector2;
// v1: player.rs:182-184 — current.lerp(target, tuning.motion_interpolate_speed * dt)

pub fn flatten_camera_axes(basis: Basis) -> (Vector3, Vector3);
// v1: player.rs:187-193 — returns (camera_x, camera_z), each with .y = 0.0, normalized()

pub fn slerp_toward(current: Basis, target: Quaternion, dt: f32, speed: f32) -> Basis;
// v1: player.rs:226-231 (aiming) / :266-271 (walking), same shared formula:
// Basis::from_quaternion(current.get_quaternion().slerp(target, dt * speed))

pub fn walk_target(camera_x: Vector3, camera_z: Vector3, motion: Vector2) -> Option<Basis>;
// v1: player.rs:264-267 — target = camera_x*motion.x + camera_z*motion.y;
// Some(Basis::looking_at(target)) if target.length() > 0.001, else None (no rotation this frame)
```

### Root motion and respawn

```rust
pub fn integrate_root_motion(
    orientation: Transform3D,
    root_motion: Transform3D,
    dt: f32,
    gravity: Vector3,
    velocity_in: Vector3,
) -> (Transform3D, Vector3);
// v1: player.rs:283-297 — orientation *= root_motion; h = orientation.origin / dt;
// velocity.x/z = h.x/h.z; velocity += gravity * dt; orientation.origin = ZERO;
// orientation = orientation.orthonormalized(); returns (orientation, velocity)

pub fn should_respawn(y: f32, tuning: &PlayerTuning) -> bool;
// v1: player.rs:303 — y < tuning.respawn_below_y
```

### `AnimPlan` and `anim_plan`

```rust
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
) -> AnimPlan;
```
Reproduces `player.rs:218-234`/`:261-274`: airborne → `JumpUp` if `velocity_y > 0.0` else
`JumpDown`; grounded + aiming → `Strafe { aim_rotation, blend_position: Vector2::new(motion.x,
-motion.y) }` (the animation's forward/backward axis is reversed, kept verbatim — `player.rs:
161-164`'s comment); grounded + not aiming → `Walk { blend_position: Vector2::new(motion.length(),
0.0) }`.

**What the ONE apply-step function writes per variant** (v1 `animate()`, `player.rs:144-177`,
in this order, plus `self.current_animation = <variant's Animations value>` first):
- `JumpUp` → `TRANSITION_REQUEST = "jump_up"`.
- `JumpDown` → `TRANSITION_REQUEST = "jump_down"`.
- `Strafe` → `TRANSITION_REQUEST = "strafe"`, then `AIM_ADD_AMOUNT = aim_rotation`, then
  `STRAFE_BLEND = blend_position`.
- `Walk` → `AIM_ADD_AMOUNT = 0` (aim to zero — no aiming while walking), then
  `TRANSITION_REQUEST = "walk"`, then `WALK_BLEND = blend_position`.
Note the `Walk` variant writes `add_amount` BEFORE the transition request while `Strafe` writes
it AFTER — v1's own order in each branch, kept verbatim.

**`AnimationTree` parameter constants** (glue, `player.rs`, NOT the pure module — Godot
property-path strings): `TRANSITION_REQUEST = "parameters/state/transition_request"` (values
`"jump_up"`/`"jump_down"`/`"strafe"`/`"walk"`), `AIM_ADD_AMOUNT = "parameters/aim/add_amount"`,
`STRAFE_BLEND = "parameters/strafe/blend_position"`, `WALK_BLEND =
"parameters/walk/blend_position"`.

## `hittable.rs` (glue — crate-level, new)

```rust
pub enum HitTarget {
    Player(Gd<Player>),
    Robot(Gd<EnemyRobot>),
}

pub fn resolve(node: Gd<Node3D>) -> Option<HitTarget>;
// two try_cast()s, order irrelevant (a node cannot be both types)

impl HitTarget {
    pub fn rpc_hit(&mut self);
    // .rpc("hit", &[]) on the inner Gd, via Deref to the shared Node ancestor
}
```
Introduced by `bullet.rs` (US2), reused unmodified by V2-D's `red_robot.rs`. No unit tests
(glue, no pure decision beyond what `try_cast` itself guarantees — research.md R5).

## `bullet.rs` (inline `mod pure`)

```rust
mod pure {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum BulletState {
        Flying { time_alive: f32 },
        Exploded,
    }

    pub fn step(state: BulletState, dt: f32) -> (BulletState, bool) {
        // bool = "explode now due to expiry"; an already-Exploded state is a no-op
    }
}
```
Reproduces `bullet.rs:43-47`. Glue's `physics_process`: call `step`, RPC `explode` if the second
element is `true`; run `move_and_collide` UNCONDITIONALLY afterward (v1 never skips this on the
expiry frame — spec US2 scenario 1); on a collision, resolve `HitTarget` and `rpc_hit()`,
disable the collision shape, RPC `explode` ONLY IF the state coming out of `step` was still
`Flying` (backlog #13 — FR-014's one new gate), and then set the state to `Exploded`
unconditionally (v1's trailing `self.hit = true`, `bullet.rs:57`) so later frames return early. `BULLET_VELOCITY` becomes an associated const
`Bullet::VELOCITY: f32 = 20.0` (simpler than a `BulletTuning` struct for a single value only
ever read by glue, per research.md R6's recommendation — `step` never needs it).

## `door.rs` (inline `mod pure`)

```rust
mod pure {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum DoorState {
        Closed,
        Open,
    }

    pub fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool) {
        // bool = "play the open animation now"
    }
}
```
Reproduces `door.rs:26-29`. `_on_door_body_entered(&mut self, body: Gd<Node3D>)` KEEPS its
signature; its first statement is `body.try_cast::<Player>().is_ok()`, fed into `on_body`.

## Cross-references (unchanged surface, verified by grep/read in spec.md's Context + research.md)

| Consumer | Reads/writes | Type after this milestone | Changed? |
|---|---|---|---|
| `level.rs:203-206` | `instantiate_as::<Player>()`, `bind_mut().set_player_id(id: i32)` | unchanged | no |
| `red_robot.rs` | `try_cast::<Player>()` (×2), `bind_mut().add_camera_shake_trauma(f64)` | unchanged | no |
| `door.rs` (this milestone) | `try_cast::<Player>()` | unchanged | no |
| `bullet.tscn:103` | method-track call to `destroy` | unchanged `#[func]` | no |
| `door.tscn:35` | `[connection] body_entered → _on_door_body_entered` | unchanged, `Gd<Node3D>` param | no |
| `player.tscn` `SceneReplicationConfig` | `player_id`, `motion`, `current_animation` | unchanged names/types | no |
| V2-D (future) `red_robot.rs` | will call `hittable::resolve`/`HitTarget::rpc_hit` | new, additive | n/a (not edited this milestone) |
