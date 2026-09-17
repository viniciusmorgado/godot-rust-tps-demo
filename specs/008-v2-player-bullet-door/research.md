# Research: Milestone V2-C — player, bullet, door

Evidence from the current working tree (`player.rs`, `bullet.rs`, `door.rs`, `player_input.rs`,
`red_robot.rs`, `level.rs`, `lib.rs`, `player.tscn`, `bullet.tscn`, `door.tscn`), gdext 0.5.5
sources (`~/.cargo/registry/src/*/godot-core-0.5.5/`), and `specs/007-v2-leaves-and-input/`'s
already-verified precedents (reused, not re-derived, where the same fact applies).

## R1 — `InputFrame` acquisition: one `bind_mut()` per authority frame

**Decision**: `apply_input` acquires `self.player_input.bind_mut()` exactly once, reads every
field/method it needs through that single guard into an `InputFrame`, writes `jumping = false`
through the SAME guard, then drops it — one critical section, one lock, matching FR-004/SC-003.
The three `#[func]` getters on `PlayerInputSynchronizer` (`get_aim_rotation`,
`get_camera_rotation_basis`, `get_camera_base_quaternion`) LOSE `#[func]`, becoming plain
`pub(crate)` methods: `grep -rn "get_aim_rotation\|get_camera_rotation_basis\|
get_camera_base_quaternion" --include='*.gd' --include='*.tscn' .` returns zero matches (no
GDScript or `.tscn` method-track/connection calls them by name) — the same closure V2-B applied
to `CameraNoiseShake::add_trauma` (backlog #13 of that milestone). They stay callable from
`player.rs` because that call is typed (`self.player_input.bind().get_aim_rotation()`), not
by-name dispatch.

**Evidence**: all three getters are `&self` methods (confirmed by reading `player_input.rs:
161-175` directly) — callable through a `GdRef`/`GdMut` guard obtained via `bind()`/`bind_mut()`
without any additional borrow gymnastics, exactly like V2-B's `player_input.rs` glue already
calls its own `&self` methods internally. Using `bind_mut()` (not `bind()`) for the WHOLE
acquisition is required only because the SAME critical section also needs to write `jumping =
false`; every other field read is a shared read that would be satisfiable by `bind()` alone, but
splitting into a `bind()` then a separate `bind_mut()` would be TWO acquisitions, violating
FR-004. One `bind_mut()` covering both the reads and the one write is the minimal-acquisition
shape.

**Non-authority path**: `physics_process`'s `else` branch calls `self.animate(anim, delta)` with
the REPLICATED `current_animation`; `animate`'s `Strafe` arm is the only place that still needs
an externally-sourced value (`aim_rotation`), fetched via one `self.player_input.bind()` call
made ONLY when the plan being applied is `Strafe` — `JumpUp`/`JumpDown`/`Walk` never touch
`player_input` at all on this path. `self.motion` is read locally (it is `Player`'s own
replicated field, confirmed in the spec's Context table via `player.tscn`'s
`SceneReplicationConfig`), not fetched from `player_input`.

**Alternatives considered**: keeping the three getters `#[func]` "just in case" — rejected,
since Principle III's spirit (glue is thin, `#[godot_api] impl X` is only what the
engine/scenes need by name) argues for removing dead exposure once grep confirms it is unused,
exactly as V2-B did.

## R2 — Pure model `player/model.rs`

**Decision**: one pure module, mirroring `player_input/model.rs`'s shape (a `Default`-carrying
tuning struct, several small pure functions, `#[cfg(test)] mod tests`). Every formula below is
checked against the exact `v1` source line range cited.

```rust
pub struct PlayerTuning {
    pub motion_interpolate_speed: f32,   // v1: 10.0  (player.rs:19)
    pub rotation_interpolate_speed: f32, // v1: 10.0  (player.rs:20)
    pub min_airborne_time: f32,          // v1: 0.1   (player.rs:22)
    pub jump_speed: f32,                 // v1: 5.0   (player.rs:23)
    pub land_threshold: f32,             // v1: 0.5   (player.rs:198, inline literal)
    pub respawn_below_y: f32,            // v1: -40.0 (player.rs:303, inline literal)
}
```

```rust
pub struct AirborneOutcome {
    pub airborne_time: f32,
    pub on_air: bool,
    pub land: bool,   // v1: self.base_mut().rpc("land", &[]) at player.rs:199
    pub jump: bool,   // v1: self.base_mut().rpc("jump", &[]) at player.rs:213
    pub jump_velocity_y: Option<f32>, // Some(tuning.jump_speed) when `jump` is true
}

pub fn airborne_step(airborne_time: f32, dt: f32, is_on_floor: bool, jump_pressed: bool,
    tuning: &PlayerTuning) -> AirborneOutcome
```
Transcribed from `player.rs:196-214` IN v1's EXACT order — this is the one function in this
milestone where getting the order right is load-bearing, since `land` and `jump` are NOT
mutually exclusive (spec Scenario 4):
1. `airborne_time += dt`.
2. If `is_on_floor`: `land = airborne_time > tuning.land_threshold`; then `airborne_time = 0.0`
   UNCONDITIONALLY (v1 resets on every floor contact, land or not).
3. `on_air = airborne_time > tuning.min_airborne_time` (recomputed AFTER the possible reset).
4. If `!on_air && jump_pressed`: `jump = true`, `jump_velocity_y = Some(tuning.jump_speed)`,
   `on_air = true`, `airborne_time = tuning.min_airborne_time` (v1's own comment: "increase
   airborne time so next frame on_air is still true").
Because step 2's reset can make `on_air` false in the same frame step 4 then flips back to
true, `land` (decided in step 2) and `jump` (decided in step 4) CAN both be true together: a
frame where the player is on the floor (fires `land`) AND `jumping` was held (step 4 immediately
re-triggers a jump) — this is `v1`'s actual behavior, not an edge case to "fix". Pinned by a
dedicated unit test (FR-005).

```rust
pub fn lerp_motion(current: Vector2, target: Vector2, dt: f32, tuning: &PlayerTuning) -> Vector2
// v1: player.rs:182-184 — current.lerp(target, tuning.motion_interpolate_speed * dt)

pub fn flatten_camera_axes(basis: Basis) -> (Vector3, Vector3)
// v1: player.rs:187-193 — (x, z) = (col_a, col_c) with .y = 0.0, both .normalized()
// returns (camera_x, camera_z) in that order

pub fn slerp_toward(...)  // DROPPED at implementation — Quaternion::slerp is engine-backed (see amendment below); the slerp stays in glue
// v1: player.rs:226-231 (aiming) and the equivalent basis-to-quaternion form at :266-271
// (walking) — shared: Basis::from_quaternion(current.get_quaternion().slerp(target, dt*speed))

pub fn walk_target(camera_x: Vector3, camera_z: Vector3, motion: Vector2) -> Option<Vector3>  // see the amendment below: looking_at is glue
// v1: player.rs:264-267 — target = camera_x*motion.x + camera_z*motion.y;
// Some(Basis::looking_at(target)) if target.length() > 0.001, else None (orientation unchanged)

pub fn integrate_root_motion(orientation: Transform3D, root_motion: Transform3D, dt: f32,
    gravity: Vector3, velocity_in: Vector3) -> (Transform3D, Vector3)
// v1: player.rs:283-297 — orientation = orientation * root_motion; h_velocity =
// orientation.origin / dt; velocity.x/z = h_velocity.x/z; velocity += gravity*dt; clear
// orientation.origin to ZERO; orientation = orientation.orthonormalized(); returns
// (new_orientation, new_velocity) — velocity.y is passed through unchanged in velocity_in
// (gravity's own .y component is what changes it, exactly as v1's `velocity += gravity*dt`
// only touches whichever axis gravity has a component on — Vector3::UP-relative gravity is
// (0, -g, 0) in this project, confirmed by `set_up_direction(Vector3::UP)` at player.rs:291)

pub fn should_respawn(y: f32, tuning: &PlayerTuning) -> bool
// v1: player.rs:303 — y < tuning.respawn_below_y

pub enum AnimPlan {
    JumpUp,
    JumpDown,
    Strafe { aim_rotation: f64, blend_position: Vector2 },
    Walk { blend_position: Vector2 },
}

pub fn anim_plan(on_air: bool, velocity_y: f32, aiming: bool, motion: Vector2, aim_rotation: f64)
    -> AnimPlan
// v1: player.rs:218-234 (on_air branch: JumpUp if velocity_y > 0.0 else JumpDown) and
// player.rs:224-234/261-274 (not on_air: Strafe if aiming, with blend_position =
// Vector2(motion.x, -motion.y) per player.rs:161-164's reversed forward/backward axis
// comment, else Walk with blend_position = Vector2(motion.length(), 0.0) per player.rs:173-175)
```

**`AnimationTree` parameter constants** (pinned verbatim from `player.rs:149-176`, to become
named `&str` consts in the GLUE file, not the pure module — they are Godot property-path
strings, not domain values):
`"parameters/state/transition_request"` (values `"jump_up"`/`"jump_down"`/`"strafe"`/`"walk"`,
themselves also worth naming as consts since they are Godot-side enum-like strings),
`"parameters/aim/add_amount"`, `"parameters/strafe/blend_position"`,
`"parameters/walk/blend_position"`.

**Purity confirmed**: `Basis::looking_at`, `Quaternion::slerp`, `Basis::from_quaternion`,
`Transform3D`'s `Mul` impl, `.orthonormalized()`, `Vector3`/`Vector2` arithmetic are all
`godot::builtin` value types — the same category the constitution's Principle III names
explicitly ("gdext's pure-Rust math builtins ... they never cross the FFI and need no engine")
and the same category V2-B's `player_input/model.rs` already builds on. `v1`'s own
`apply_input` already calls every one of these exact methods (confirmed by reading
`player.rs:226-297` directly) — extracting them into functions that take the same types as
plain parameters changes nothing about their purity, only where they are called from.

**Amendment (found at implementation, commit 1)**: `Basis::looking_at` is NOT pure Rust. It is
a generated builtin method (`out/builtin_classes/basis.rs:219-227`) dispatched through
`sys::builtin_method_table()` (FFI) and panics under `cargo test` ("Godot engine not
available"). `from_quaternion`, `slerp`, `orthonormalized`, `Basis`/`Transform3D` operators and
`get_quaternion` live in `godot-core/src/builtin/**` and are pure (verified by the passing
tests). Consequence: `walk_target` returns the target `Vector3` (the `> 0.001` decision, pure,
tested) and the `looking_at` call moves to glue.

**Second amendment (same commit)**: `Quaternion::slerp` is ALSO engine-backed — its body is
`self.as_inner().slerp(to, weight)` (`godot-core/src/builtin/quaternion.rs:203-208`); only the
normalization assert is Rust. So `slerp_toward` is dropped from the pure model (it had no
decision logic) and both orientation updates stay in glue as v1 wrote them. General rule for
later milestones (to be added to `CLAUDE.md` in this milestone's docs commit): a builtin math
method is pure only if it does not go through `as_inner()` — glam-based operators,
`from_quaternion`, `get_quaternion`, `from_euler`, `orthonormalized`, `Transform3D` mul are
pure; `Quaternion::slerp*`, `Basis::looking_at` and everything generated under
`out/builtin_classes/**` need the engine. The practical check is a `#[test]`: engine-backed
methods panic with "Godot engine not available".

## R3 — Glue frame shape for `apply_input`

**Decision**: the exact sequence, preserved frame-for-frame:
1. Acquire `InputFrame` (R1), clear `jumping`.
2. `lerp_motion` → write `self.motion`.
3. `flatten_camera_axes` from `frame.camera_rotation_basis`.
4. `airborne_step` (engine reads ONCE here: `is_on_floor()`) → RPC `land`/`jump` per the
   outcome's independent flags (both may fire); if `jump_velocity_y` is `Some`, write it into a
   LOCAL velocity value that carries forward to the final `set_velocity` at step 8 (v1 sets
   `self.base_mut().set_velocity(velocity)` immediately inside the jump branch, THEN reads
   `get_velocity()` again later at line 286 for the horizontal components — since nothing
   between lines 209 and 286 changes `.y`, reading it back is equivalent to threading the value
   through glue-local state; the plan preserves v1's own two-write shape since `move_and_slide`
   hasn't run yet and no intermediate engine code depends on the intermediate `set_velocity`
   call being visible).
5. Branch on `on_air` (from step 4's outcome):
   - **Airborne**: `anim_plan` with `velocity_y` read from the CURRENT `get_velocity().y`
     (post-jump if a jump just happened) → apply the plan (writes `AnimationTree` params +
     `current_animation`, via ONE apply-step function shared with the `jump`/`land` RPC
     handlers' direct calls). `root_motion` is NOT reassigned (v1 does not touch it in this
     branch — R2's note on field persistence).
   - **Not airborne, aiming**: `q_from.slerp(frame.camera_base_quaternion, dt·speed)` in glue (engine call) →
     `self.orientation.basis`; `anim_plan` → `Strafe`; apply; THEN read
     `animation_tree.get_root_motion_rotation()/position()` and reassign `self.root_motion`
     (v1's own order: `animate()` — which sets the AnimationTree's transition request — runs
     BEFORE the root motion read, so the read reflects the newly-requested state's motion,
     not the previous frame's; this ordering is preserved exactly); if `frame.shooting` and
     `fire_cooldown.get_time_left() == 0.0`, spawn a bullet (R4) and RPC `shoot`.
   - **Not airborne, not aiming**: `walk_target` → if `Some(target)`, `Basis::looking_at(target)
     .get_quaternion()` → `q_from.slerp(q_to, dt·speed)` (both glue — engine calls); `anim_plan` →
     `Walk`; apply; THEN read root motion and reassign `self.root_motion` (same ordering note).
6. `integrate_root_motion` (engine reads ONCE here: `get_gravity()`, and `get_velocity()` for
   the value threaded from step 4/5) → new `self.orientation`, new velocity x/z.
7. `set_velocity`, `set_up_direction(Vector3::UP)`, `move_and_slide()` — ONE write each,
   unchanged order from `v1`.
8. `player_model.set_global_basis(self.orientation.basis)`.
9. `should_respawn` on `get_transform().origin.y`; if true, set `transform.origin =
   initial_position` AND zero `velocity` (backlog #11 — the one added engine write this
   milestone introduces relative to `v1`'s respawn branch).

This reproduces `v1`'s `apply_input` (`player.rs:180-308`) exactly except for the two
authorized deviations (#10's `airborne_time` starting value, #11's respawn velocity zeroing).

## R4 — Preloaded resources

**Decision**: the bullet scene becomes a BARE field (no `OnReady` wrapper):
```rust
#[init(val = load("res://player/bullet/bullet.tscn"))]
bullet_scene: Gd<PackedScene>,
```
not `OnReady<Gd<PackedScene>>`. **Rationale**: `godot::tools::load<T>(path) -> Gd<T>`
(`godot-core-0.5.5/src/tools/save_load.rs:29-34`) calls `ResourceLoader` via FFI and has NO
documented dependency on the node being inside the `SceneTree` (unlike `OnReady`'s reason for
existing — deferring until a `Base`/parent/autoload lookup is possible). `OnReady` exists
specifically for initialization that needs to happen "before `ready()` starts" because it
depends on tree state; a resource load has no such dependency, so wrapping it in `OnReady` would
add a layer of indirection (and a `#[cfg(test)]`-unfriendly initialization-order sensitivity)
for no benefit. This mirrors `camera_noise_shake.rs`'s existing
`#[init(val = FastNoiseLite::new_gd())]` precedent — a non-tree-dependent engine construction
called directly in `#[init(val = ...)]`, already an established pattern in this codebase.

The two particle emitters become `#[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/
ShootFrom/ShootParticle")] shoot_particle: OnReady<Gd<CpuParticles3D>>` and the equivalent for
`MuzzleFlash` — genuine node lookups, correctly `OnReady` (same paths `player.rs:116-123`
already use, just resolved once instead of via `get_node_as` inside `shoot`).

`crosshair: OnReady<Gd<TextureRect>>` is deleted entirely (backlog #12, confirmed unused by
grep in the spec's Context). `airborne_time`'s `#[init(val = 100.0)]` becomes
`#[init(val = 0.0)]` (backlog #10) — or simply no `#[init(...)]` at all, since `0.0` is `f32`'s
own `Default`; either is acceptable, the task/implementation picks.

## R5 — `HitTarget`: a new crate-level module

**Decision**: `oxide_godot_lib/src/hittable.rs` (new file, `mod hittable;` added to `lib.rs`):
```rust
use crate::player::Player;
use crate::red_robot::EnemyRobot;
use godot::classes::Node3D;
use godot::prelude::*;

pub enum HitTarget {
    Player(Gd<Player>),
    Robot(Gd<EnemyRobot>),
}

pub fn resolve(node: Gd<Node3D>) -> Option<HitTarget> {
    if let Ok(player) = node.clone().try_cast::<Player>() {
        return Some(HitTarget::Player(player));
    }
    if let Ok(robot) = node.try_cast::<EnemyRobot>() {
        return Some(HitTarget::Robot(robot));
    }
    None
}

impl HitTarget {
    pub fn rpc_hit(&mut self) {
        match self {
            HitTarget::Player(p) => { p.rpc("hit", &[]); }
            HitTarget::Robot(r) => { r.rpc("hit", &[]); }
        }
    }
}
```
`.rpc("hit", &[])` is callable directly on `Gd<Player>`/`Gd<EnemyRobot>` via their `Deref` to
the shared `Node` ancestor (confirmed pattern: `Gd<T>` for a user class derefs to `T::Base`'s
engine chain, established in V2-B's `part_disappear.rs`/`blast.rs` research). Both `Player`
(`player.rs:27`) and `EnemyRobot` (`red_robot.rs:31`) are already `pub struct`, and both
`mod player;`/`mod red_robot;` in `lib.rs` are visible crate-wide to sibling modules (the same
visibility shape `bullet.rs` already relies on for `use crate::settings::Settings;`) — no
visibility change needed anywhere. `EnemyRobot`'s `#[rpc] fn hit(&mut self)` exists unchanged at
`red_robot.rs:276-277`, confirmed present and untouched by this milestone (V2-D owns
`red_robot.rs` itself).

This is GLUE (uses `Gd<T>`), not pure domain logic — Principle III's pure-module ban on `Gd<T>`
does not apply to it, and it carries no dedicated unit tests (there is no decision logic inside
type resolution to test beyond what `try_cast` itself already guarantees).

## R6 — `BulletState` + pure `step`

**Decision**:
```rust
pub enum BulletState { Flying { time_alive: f32 }, Exploded }

pub fn step(state: BulletState, dt: f32) -> (BulletState, bool) {
    match state {
        BulletState::Exploded => (BulletState::Exploded, false),
        BulletState::Flying { time_alive } => {
            let time_alive = time_alive - dt;
            if time_alive < 0.0 {
                (BulletState::Exploded, true)
            } else {
                (BulletState::Flying { time_alive }, false)
            }
        }
    }
}
```
Glue's `physics_process`: `(new_state, expired) = step(self.state, dt); self.state = new_state;
if expired { rpc("explode") }` — THEN, regardless of `expired`, still run `move_and_collide`
(v1 never returns early on the expiry frame itself, only on frames where `state` was ALREADY
`Exploded` at frame start — spec US2 scenario 1); if a collision also occurs, RPC `hit` via
`HitTarget` (R5) and disable the collision shape; RPC `explode` for the collision ONLY IF the
state is STILL `Flying` at that point (i.e., `expired` was false this frame) — this is exactly
backlog #13's fix (FR-014): the state-check gate is the only new logic, not a change to when
`move_and_collide`/`hit` happen. `BULLET_VELOCITY` becomes `BulletTuning { velocity: f32 }`
with `Default = 20.0`, or a bare associated const — both satisfy FR-016 equally; the task
picks based on whether `step` ever needs it (it does not — velocity only affects the
`move_and_collide` displacement, computed in glue), so an associated const
(`Bullet::VELOCITY: f32 = 20.0`) is the simpler of the two and is recommended.

**Module layout**: `bullet.rs` gains an inline `mod pure { BulletState, step, #[cfg(test)] mod
tests }` (not a separate `bullet/model.rs` file) — the pure surface is one enum and one
function, smaller than `camera_noise_shake`'s pure surface (which got its own file) and closer
in size to `debug_label`'s (which stayed inline), per research.md R8's own precedent from V2-B:
file boundary follows the pure surface's size.

`destroy` keeps `#[func]` (`bullet.tscn:103`'s method-call track invokes it by name — confirmed
unchanged, no reason to touch it).

## R7 — `DoorState` + pure `on_body`

**Decision**:
```rust
enum DoorState { Closed, Open }

fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool) {
    match (state, is_player) {
        (DoorState::Closed, true) => (DoorState::Open, true),
        (state, _) => (state, false),
    }
}
```
(the trailing `bool` is "play the open animation now"). `_on_door_body_entered(&mut self, body:
Gd<Node3D>)` keeps its exact signature; its first statement is
`body.try_cast::<Player>().is_ok()`, fed into `on_body`. `door.tscn:35`'s `[connection]` is
untouched (FR-018). Given the tiny size (one 2-line function, one 2-variant enum), this stays a
bare private `fn`/`enum` inside `door.rs` — no `mod pure` wrapper needed (there is no `#[func]`/
engine-callback naming collision risk to guard against with a submodule here, and V2-B's
`debug_label.rs` used a `mod` only because it ALSO needed a nested `#[cfg(test)]`; the same
applies here, so a MINIMAL inline `mod pure { ... #[cfg(test)] mod tests { ... } }` is used
after all, for consistency with every other pure surface in the crate carrying its tests in a
`mod tests` beside it, not because `door.rs`'s surface is large enough to need file separation).

## R8 — Parity harness `zz_player_parity.tscn`/`.gd`

**Decision**: mirrors V2-B's harness shape (`XDG_DATA_HOME` per tree, `--fixed-fps 60`), a NEW
scratch scene (not `level.tscn` — confirmed too heavy/nondeterministic: it spawns robots at
random spawn points via `randi() % count`, shuffles player spawn points, and its `ready()`
branches on `is_server()` to run a full multiplayer bring-up sequence, none of which the
harness needs and all of which adds nondeterminism the harness must not fight).

- **(a) Player movement/animation trace**: the harness builds a flat floor itself
  (`StaticBody3D` + a large `BoxShape3D` `CollisionShape3D`, collision layer 1 — matching
  `bullet.rs`'s/robots' expectations without needing `level.tscn`'s actual geometry), instances
  `player.tscn` above it, drives a scripted `move_*`/`aim`/`jump`/`shoot` sequence (same
  `Input.action_press`/`release` + `--fixed-fps 60` technique as V2-B), and dumps
  `global_position`, `velocity`, `current_animation`, and `PlayerModel`'s global basis per
  frame. `Player::ready()` checks `get_multiplayer().is_server()`: the harness's default
  `OfflineMultiplayerPeer` (confirmed in V2-B's research) makes `is_server()` true for a
  directly-instanced player with no explicit multiplayer setup — same "offline = authority"
  shape V2-B's case (a)/(b) already relied on. Frames 0/1 (backlog #10) and the frames
  immediately after a scripted teleport below `-40` (backlog #11) are DUMPED but excluded from
  the equality assertion in the diff step (documented divergences, not omitted from the trace).
- **(b) Bullet vs. wall / vs. robot**: instance `bullet.tscn` aimed at a static wall, count
  frames until its `AnimationPlayer`'s current animation becomes `"explode"` (or, equivalently,
  until `queue_free` — but `destroy` is gated on `is_server()` and the animation method track,
  which fire deterministically at a fixed `--fixed-fps`, so counting frames to the animation
  change is the more direct, harness-controlled signal, avoiding a dependency on the
  `AnimationPlayer` finishing its full clip before `destroy` runs); instance an `EnemyRobot`
  (standalone `red_robot.tscn`, confirmed by reading `red_robot.rs`'s field list that `ready()`
  has NO `Settings`/level dependency — every `OnReady` field resolves from the robot's own
  scene subtree) and a bullet aimed at it, dump `health` before and after. A THIRD, deliberately
  timed case sets the bullet's initial `time_alive` so it crosses zero on the SAME physics tick
  it collides with the wall — the harness dumps whether `explode`'s animation restarted
  (observable as the `AnimationPlayer`'s local playback position resetting to `0.0` twice
  within one frame vs. once) as the documented `v1`-vs-`v2` divergence point (backlog #13), not
  an equality assertion.
- **(c) Door**: instance `door.tscn`, move a scripted `player.tscn` body into its `Area3D` →
  dump whether `DoorModel2/AnimationPlayer` is playing `"doorsimple_opening"`; separately, move
  an `EnemyRobot` body through the same `Area3D` → dump the same check (expect: not playing) and
  confirm no new stderr line appears (a non-player body must not produce any engine warning on
  either branch, per FR-018's whole point).

**Alternatives considered**: driving the harness through `level.tscn` directly — rejected for
the reasons above (nondeterministic spawn RNG, multiplayer bring-up complexity unrelated to
what this milestone changes); building a full multiplayer pair (two peers) — unnecessary, since
none of the three modules' authority-vs-remote branching is being changed in a way that needs
cross-peer verification beyond what the single-authority-peer trace already exercises (the
non-authority `animate` path is exercised by unit test coverage of `anim_plan`, not the
harness).

## R9 — Module layout and commit plan

One commit per module, `player.rs` split into pure-model-then-glue (matching V2-B's
`player_input` precedent, since `player.rs`'s pure surface is large enough to warrant its own
review-sized commit), `hittable.rs` riding with the `bullet.rs` commit (its first and only
consumer this milestone), `door.rs` alone, backlog bookkeeping last. Full detail in `plan.md`'s
Commit Plan. User checkpoints after the `player.rs` glue commit (US1 complete, checkpoint 1) and
after the `door.rs` commit (US2+US3 complete, checkpoint 2) — two checkpoints, not three,
since `bullet.rs` and `door.rs` are small enough to validate together in one pass, mirroring how
V2-B grouped US2+US3 into a single checkpoint.
