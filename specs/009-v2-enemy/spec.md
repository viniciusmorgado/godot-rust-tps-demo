# Feature Specification: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

**Feature Branch**: `v2` (work directly, per constitution 1.4.1 Principle II — `red_robot.rs` is
the last gameplay hub; `part.rs` is its dependent, instantiated three times per robot). No
per-milestone feature branch. Baseline: `5d0a32a` (constitution 1.4.1). Local commits only, never
pushed.

**Created**: 2026-09-17

**Status**: Draft

**Phase**: v2 — Idiomatic Rust (constitution 1.4.1, Principles I, II and III — the 1.4.1 wording:
builtin math TYPES and their Rust-implemented methods are pure; a method whose body goes through
`as_inner()` or that is generated under the bindings' `out/builtin_classes/**` calls the engine
and belongs in glue). Both pillars apply, done together: (1) idiomatic Rust leaning on the type
system (a typed robot state machine replacing four loose `f32`/`bool` fields and three `if
self.state ==` checks, `HitTarget` reused instead of a fresh ad-hoc typed check, tuning structs
instead of module consts) and (2) FFI reduction through Principle III (three duplicated
`PhysicsRayQueryParameters3D`/`intersect_ray` blocks collapsed into one helper, a per-frame
`Os::has_feature("dedicated_server")` read once at `ready`, per-shot `get_node_as`/`load`
resolved once via `OnReady`, the two `connect_other` timer chains in `part.rs` and the one in
`red_robot.rs::hit` replaced by the `godot::task::spawn` + `SceneTreeTimer` async pattern already
established in `part_disappear.rs`/`blast.rs`).

- **Backlog items closed by this spec**: #16 (the dead `body.get_name() == "Target"` branch
  removed; `player: Option<Gd<Node3D>>` becomes `Option<Gd<Player>>`), #17 (the 10 s
  post-death removal and the 0.1 s post-laser-hit trauma delay move from `connect_other` chains
  to the `godot::task::spawn` async pattern — same delays, same observable timing), #18
  (`aim_preparing` and `test_shoot` both lose `#[export]`, kept as plain `#[var]` — neither
  scene stores an override value, confirmed by grep; see FR-009), #15 (the death puff's parent
  changes from `Death` — freed 10 s after the robot dies — to the robot's own parent, removing
  the fragile lifetime coupling; see FR-018 and the Edge Cases note on this being the one
  structural harness difference).
- **Backlog items explicitly deferred**: none new. #3, #6, #19–#25, #29, #30 remain untouched,
  out of scope for this milestone (different modules or already-deferred decisions).
- **Residual dynamic access left in the touched modules after this milestone**: `red_robot.rs`
  `self.base_mut().rpc("play_shoot", &[])` (fired by `red_robot.rs` on itself) and the inherited
  `HitTarget::rpc_hit()`'s `.rpc("hit", &[])` (unchanged, from V2-C — `EnemyRobot` is a
  `HitTarget::Robot` variant, resolved by `bullet.rs`, not by this module); `part.rs`
  `self.base_mut().rpc("destroy", &[])` — **all permanent**, gdext exposes RPC dispatch only by
  name (the engine-limitation residual case the constitution names explicitly).
- **#28 stays open, other half addressed**: this milestone preloads `impact_effect.tscn` (US1)
  closing the per-shot `load::<PackedScene>` site named in #28's hypothesis for the ROBOT's
  laser hits (V2-C already closed the PLAYER's-bullet half). The checkpoint after US1 asks the
  user to note, without acting on it, whether the hitch on the robot's first laser shot changed.
  `docs/v2-backlog.md`'s existing #28 row gets a second annotation; it stays open regardless.

**Input**: User description: "Milestone V2-D — enemy: `part.rs` and `red_robot.rs`. Apply the
V2-B/V2-C patterns (snapshot→step→apply, `HitTarget` reuse, `OnReady`/preloaded resources, the
`godot::task::spawn` async pattern, tuning structs) to the last gameplay hub — a 4-state machine
with loose counters, three duplicated raycasts, per-frame `has_feature`, per-shot `load`/
`get_node_as`, a 10 s `await` inside an RPC, and a dead branch — and to its `Part` dependent,
which has its own per-event `get_node_as`/`load` and two `await` chains. Preserve every
name/signature `level.rs` calls today and everything `bullet.rs`'s `HitTarget` already expects
from `EnemyRobot`. Behavioral parity with `v1` is mandatory except the closed backlog items
above, verified headless and with a parity harness against a `v1` worktree."

## Context

`EnemyRobot` is consumed **typed** by one module: `level.rs::spawn_robot`
(`load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>()`,
then `robot.signals().exploded().connect_other(&*self, |this: &mut Level| this._respawn_robot(...))`
— a typed signal connection, already compliant). `bullet.rs`'s `HitTarget` (introduced V2-C,
`hittable.rs`) already resolves an `EnemyRobot` collider via `try_cast` and calls
`.rpc("hit", &[])` on it — `EnemyRobot::hit` MUST keep its exact `#[rpc(authority, call_local,
unreliable)]` signature and observable effect for that to keep working; nothing in this milestone
changes `hittable.rs` itself. `Part` is consumed **typed** by `EnemyRobot` alone: three fields
(`death_shield1`, `death_shield2`, `death_head`, all `OnReady<Gd<Part>>`) and one call site per
field, `self.death_shieldN.bind_mut().explode()`.

Confirmed by reading both files in full (`red_robot.rs` 470 lines, `part.rs` 120) and their
scenes:

| Module | Base | What consumers call today |
|---|---|---|
| `red_robot.rs` | `CharacterBody3D` | `level.rs`: `instantiate_as::<EnemyRobot>()`, `signals().exploded().connect_other(...)` (typed). `bullet.rs`/`hittable.rs`: `try_cast::<EnemyRobot>()`, `.rpc("hit", &[])` (V2-C, unchanged). `red_robot.tscn`'s `[connection]`s call `_on_area_body_entered`/`_on_area_body_exited` (`#[func]`, kept); a method track on the shoot animation calls `shoot_check` (`#[func]`, kept) and another on the aim/return transition calls `resume_approach` (`#[func]`, kept). RPCs `hit`/`play_shoot` are fired by `red_robot.rs` on itself. |
| `part.rs` | `RigidBody3D` | `red_robot.rs`'s `hit` RPC calls `self.death_shieldN.bind_mut().explode()` (typed, ×3); `part.tscn`'s (embedded inside `red_robot.tscn`, no standalone scene file) `explode` animation's method track calls `destroy` by name (`#[rpc]`, must stay reachable by that name). |

`red_robot.tscn`'s `SceneReplicationConfig_h6xi0` (5 properties): `.:global_transform` (mode 1,
continuous), `.:health` (mode 0, spawn-only), `.:state` (mode 1, continuous), `.:target_position`
(mode 1, continuous), `.:dead` (mode 0, spawn-only) — confirming `aim_preparing` is exported
(`#[export]`) but NOT part of this list; grepping the scene for stored override values on
`aim_preparing`/`test_shoot` (both `#[export]` fields) finds none — neither the top-level
`EnemyRobot` node nor any other node in the file sets either property, confirming backlog #18's
premise that both attributes expose no real designer-tunable surface today.

`PlayerDetectionArea`'s two `[connection]`s (`red_robot.tscn:11044-11045`) are same-node-style
(`from="PlayerDetectionArea" to="."`), same pattern as `door.tscn`'s. `red_robot.rs`'s
`_on_area_body_entered` checks `body.clone().try_cast::<Player>().is_ok() || body.get_name() ==
"Target"` — grepping every `.tscn` in the project for `name="Target"` finds none; the branch has
never fired and cannot fire, confirming backlog #16's premise exactly (matching V2-C's identical
finding for `door.tscn`'s never-instanced state, and V2-B's `player_input.rs` precedent of a dead
branch removed without behavior change).

`Part`'s own `SceneReplicationConfig_hqtbc` (5 properties, all mode 1/continuous):
`.:fade_value`, `.:position`, `.:rotation`, `.:linear_velocity`, `.:angular_velocity`. Grepping
the three `Part` node blocks (`Death/PartShield1`, `PartShield2`, `PartHead`) for
`lifetime`/`lifetime_random`/`disappearing_time` overrides finds none — every instance uses the
Rust `#[init(val = ...)]` defaults (`3.0`, `3.0`, `0.5`); these three `#[export]`s are kept as-is
(no backlog item asks otherwise, and unlike `aim_preparing`/`test_shoot` they are genuinely
designer-tunable knobs, just unused by the current scene).

The async pattern (backlog #5's precedent, established in `part_disappear.rs`/`blast.rs`) has two
concrete shapes already in the codebase: `part_disappear.rs`'s two sequential
`get_tree().create_timer(t).signals().timeout().to_future().await` calls (the `SceneTree` itself
owns the `SceneTreeTimer`, so its emitter is guaranteed to outlive the wait — `to_future()`, not
fallible), and `blast.rs`'s single `to_fallible_future()` wait on its own child
`AnimationPlayer`'s `animation_finished` signal (a node that could in principle be freed
independently, hence the fallible variant). Every wait this milestone touches
(`part.rs::explode`'s pre-`set_process` wait, `part.rs::destroy`'s pre-`queue_free` wait,
`red_robot.rs::hit`'s pre-removal wait, `red_robot.rs::shoot`'s pre-trauma wait) is a
`get_tree().create_timer(...)` wait — the `SceneTree`-owned shape, `to_future()` throughout, no
fallible variant needed anywhere in this milestone.

Baseline gates at the start of this milestone (inherited from V2-C, unchanged): `cargo build` /
`cargo clippy` / `cargo test` all clean, 62 tests. Behavior baseline for parity is branch `v1` (a
separate worktree, built and run independently, as in V2-A/B/C).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `red_robot.rs`: typed state machine, pure `step`, one raycast helper (Priority: P1)

The robot still detects the player entering its area, turns to face them, prepares, aims, fires
its laser (clipping the beam at whatever it actually hits, spawning the impact effect and
shaking the camera if it hits the player), takes damage, and explodes into its three parts after
the fifth hit — all exactly as before, except the dead `"Target"` branch is gone, the 10 s
removal and 0.1 s trauma delay use the established async pattern instead of `connect_other`
chains, and the three duplicated raycast blocks become one helper.

**Why this priority**: `EnemyRobot` is the largest and last gameplay hub; its 4-state machine
with counters is the richest state shape in v2 so far, and `bullet.rs`'s `HitTarget` already
depends on its `hit` RPC keeping its exact signature.

**Independent Test**: `cargo test` passes for the new pure module with no Godot binary; headless
`main.tscn`/`level.tscn` show no new errors; in the running game, a robot approaches, turns to
face, prepares, aims, shoots its laser, hits or misses the player identically to `v1`, takes
damage and dies after 5 hits, and is removed from the scene roughly 10 seconds later.

**Acceptance Scenarios**:

1. **Given** `red_robot.tscn`'s replication config and its `[connection]`s/method tracks, **When**
   `red_robot.rs` is remodeled, **Then** the replicated `#[export]` fields `target_position:
   Vector3`, `health: i32`, `state: State` (the flat wire enum, `i64`-backed codes unchanged:
   `Idle=0, Approach=1, Aim=2, Shooting=3`), `dead: bool` keep their exact names, types and
   `SceneReplicationConfig` modes; the `#[signal] exploded()` (`level.rs`'s typed connection),
   the RPCs `hit`/`play_shoot`, and the `#[func]`s `resume_approach`, `shoot_check`,
   `_on_area_body_entered`, `_on_area_body_exited` all keep their exact names, signatures and
   `#[rpc]`/`#[func]` attributes.
2. **Given** `aim_preparing: f32` and `test_shoot: bool`, both currently `#[export]` with no
   stored override in `red_robot.tscn` (verified by grep — Context), **When** remodeled, **Then**
   both lose `#[export]` and become plain `#[var]` fields (closing backlog #18 as "do not
   export", the option the backlog row itself leads with): `test_shoot` stays settable by
   `shoot_check()` (called via the shoot animation's method track) and read once per physics
   frame; `aim_preparing`'s live value moves into the richer internal state representation (see
   FR-002) rather than staying a bare flat field, since it is only ever meaningful while
   `Approach`/`Aim`/`Shooting` is the current state — `Idle` has no `aim_preparing` value in
   `v1` either (the field simply sits unused at whatever it was last set to).
3. **Given** the dead `body.get_name() == "Target"` branch in `_on_area_body_entered`
   (`red_robot.rs:325`) and `player: Option<Gd<Node3D>>`, **When** remodeled, **Then** the branch
   is removed entirely (backlog #16 — confirmed unreachable: no `.tscn` in the project has a
   node named `"Target"`, verified by grep) and `player` becomes `Option<Gd<Player>>`, removing
   every `try_cast::<Player>()` call this module previously needed on its own field (the
   `try_cast` still happens exactly once, at the moment a body enters the detection area, to
   decide whether to accept it as `self.player` — this is the SAME single conversion point
   `v1` had, just moved: `v1` tries the cast against a bare `Node3D` parameter every read site
   that needed a `Player`-typed call; v2 tries it once at entry and stores the typed result).
4. **Given** the three duplicated `PhysicsRayQueryParameters3D::create_ex(...).collision_mask
   (0xFFFFFFFF).exclude(&array![rid]).done()` + `intersect_ray` blocks (the Approach state's
   line-of-sight check before transitioning to Aim, the Aim state's line-of-sight check before
   transitioning to Shooting, and `shoot`'s actual laser raycast), **When** remodeled, **Then**
   they become ONE glue helper `fn raycast_to(&self, from: Vector3, to: Vector3) -> RayHit`
   (or equivalent — the exact return shape is a plan-time decision) that builds the exclusion
   array from `self.base().get_rid()` and calls `intersect_ray` once; all three call sites use
   it; the collision mask (`0xFFFFFFFF`) and exclusion behavior are unchanged.
5. **Given** `_clip_ray`'s per-frame `Os::singleton().has_feature("dedicated_server")` check,
   **When** remodeled, **Then** the check happens ONCE, at `ready`, into a stored `bool` field;
   every frame's shader-parameter write (unchanged: v1's own per-frame write, not gated on
   whether the length changed, is kept verbatim for parity) reads that field instead of calling
   `has_feature` again.
6. **Given** the state transition logic (`physics_process`'s `Approach`/`Aim`/`Shooting`
   branches: aim-preparing countdown, the facing-angle tolerance check, the shoot-countdown
   raycast gating entry into `Aim`, the aim-countdown raycast gating entry into `Shooting` or a
   return to `Approach` via `resume_approach`), **When** a pure step function runs (inputs: the
   current internal state, `dt`, whether a player is currently tracked, the facing angle to that
   player if any, and the two raycast "did I actually see the player" outcomes needed this
   frame; outputs: the new internal state plus a small list of commands — "RPC `play_shoot`",
   "call `resume_approach`'s reset", "clip the ray to this length"), **Then** it reproduces
   `red_robot.rs:140-235` exactly, in the same order and with the same thresholds
   (`PLAYER_AIM_TOLERANCE` — renamed from `PLAYER_AIM_TOLERANCE_DEGREES`, since the constant
   holds `15.0_f32.to_radians()`, i.e. RADIANS despite its old name, unchanged value;
   `SHOOT_WAIT = 6.0`; `AIM_TIME = 1.0`; `AIM_PREPARE_TIME = 0.5`); glue performs the two raycasts
   the step function needs (via the ONE helper from Scenario 4) and feeds their boolean outcome
   in, exactly where `v1` performs them inline.
7. **Given** the animation decision (`animate`'s transition-request choice —
   `turn_left`/`turn_right`/`idle`/`walk` from the facing angle and whether a target is tracked
   — and the aim blend step — `blend_position` nudged toward the cannon-to-target angle at
   `BLEND_AIM_SPEED = 0.05`, clamped to `[-1.0, 1.0]` on both axes, plus `aiming/blend_amount`
   from `aim_preparing / AIM_PREPARE_TIME` clamped to `[0.0, 1.0]`), **When** pure functions run
   (`Basis`/`Vector2`/`Vector3`/`f32` only, verified against the 1.4.1 purity rule — `atan2` is
   `std`, `Transform3D`/`Basis` operators and `.transposed()` are glam-based per
   `godot-core/src/builtin/matrices/**`, none of this path calls `as_inner()` or a generated
   `out/builtin_classes/**` method; the plan cites the exact source lines, as V2-C's research.md
   R2 amendments did for `Basis::looking_at`/`Quaternion::slerp`), **Then** they reproduce
   `red_robot.rs:406-456` exactly, including the `target_position == Vector3::ZERO` check as the
   "no target tracked" sentinel (unchanged from `v1` — not remodeled into an `Option` in this
   pass, since `target_position` is the REPLICATED flat field and changing its sentinel
   semantics would be a wire-format change, out of scope).
8. **Given** the root-motion integration inside `physics_process` (`orientation = orientation *
   root_motion`; horizontal velocity from `orientation.origin / delta`; gravity added; origin
   cleared; basis orthonormalized) and the no-player idle branch (`target_position =
   Vector3::ZERO`; velocity from gravity alone; `move_and_slide`), **When** remodeled, **Then**
   both reproduce `red_robot.rs:128-136` and `:238-260` exactly — this is the same shape
   `player.rs`'s `integrate_root_motion` (V2-C) already established and tested; `red_robot.rs`'s
   version may reuse that exact pure function if its signature fits, or a twin with the same
   body, a plan-time decision.
9. **Given** backlog #17 (the `hit` RPC's 10 s `create_timer(10.0).signals().timeout()
   .connect_other(...)` removal wait, and `shoot`'s 0.1 s `create_timer(0.1)...connect_other(...)`
   trauma-delay wait), **When** closed, **Then** both become `godot::task::spawn` async blocks
   using `to_future()` on the `SceneTree`-owned timer (Context — no fallible variant needed),
   checking `is_instance_valid()` after the wait before doing anything with `self`; the delays
   (10.0, 0.1) and their effects (`queue_free()`; `add_camera_shake_trauma(13.0)`) are unchanged.
10. **Given** the laser hit on the player specifically (`shoot`'s `hit_player` check against
    `self.player`, followed by `try_cast::<Player>()` and the 0.1 s-delayed
    `add_camera_shake_trauma(13.0)` call), **When** remodeled, **Then** it KEEPS the direct
    `try_cast::<Player>()` + typed `add_camera_shake_trauma` call — NOT routed through
    `HitTarget`: `HitTarget`'s two variants exist to unify "does this collider have a `hit` RPC
    to call", a question `EnemyRobot` never asks about its OWN target (`self.player` is already
    known to be a `Player`, stored typed since Scenario 3); `add_camera_shake_trauma` has no
    `EnemyRobot`-side equivalent, so folding this through `HitTarget` would need to immediately
    discard its `Robot` variant — strictly more code for no type-safety gain.
11. **Given** `hit`'s sequence in `v1` (`red_robot.rs:276-311`, in this order): `if dead {
    return }`; on EVERY hit of a live robot, set `parameters/hit{1..3}/request = 1` (random pick)
    and play the hit sound; THEN `health -= 1`; THEN, only if `health` reached exactly `0`: mark
    `dead`, deactivate the `AnimationTree`, hide the model, show `Death`, disable the collision
    shape, start the two detach sparks, call `explode()` on the three parts, play the explosion
    sound, emit `exploded`, and (server only) start the 10 s removal wait — **When** remodeled,
    **Then** the glue keeps that exact order: the hit-reaction animation parameter (RNG pick
    `randi() % 3 + 1`, sampled in glue as `part.rs` already does) and the hit sound fire on
    every non-dead hit BEFORE the decrement, not only on the killing blow; a pure `hit_step(
    health: i32) -> (i32, bool)` (decrement, report whether health JUST reached zero — `health
    > 0 && new_health == 0`) is unit-tested and decides ONLY whether the death sequence runs;
    the death sequence's engine effects (visibility, `AnimationTree`, sparks, `explode()` ×3,
    explosion sound, signal, timer) stay in glue, applied only on that transition.

---

### User Story 2 - `part.rs`: pure fade/lifetime, async waits, the puff's parent (Priority: P2)

A destroyed robot part still flies off with the same initial velocity and spin, fades out over
the same curve, disappears at the same moment, and leaves the same dissipating-particle puff
behind — except the puff is no longer a child of the dying robot's `Death` node (closing backlog
#15), and both waits use the established async pattern instead of `connect_other` chains.

**Why this priority**: smaller and self-contained; depends on nothing from US1 (in fact US1
depends on `Part::explode`'s exact signature staying `#[func] fn explode(&mut self)`, called
typed by `red_robot.rs::hit`), independently testable and deployable.

**Independent Test**: `cargo test` passes for the pure fade/lifetime functions; in the running
game, a robot's three parts fly apart with visible spin and gravity, visibly fade near the end of
their lifetime, disappear, and leave a brief particle puff behind at the correct world position.

**Acceptance Scenarios**:

1. **Given** `_disappearing_counter: f32` accumulating every `process` frame once `explode` has
   armed it, and `fade_value`'s curve (`(_disappearing_counter / disappearing_time).powi(2)`),
   **When** a pure `fade_curve(counter: f32, disappearing_time: f32) -> f32` runs, **Then** it
   reproduces `part.rs:54` exactly (squared ratio, unclamped — `v1` never clamps this either,
   since `process` stops itself before the counter can overshoot meaningfully past the destroy
   threshold below).
2. **Given** the destroy-trigger check (`_disappearing_counter >= disappearing_time - 0.2`),
   **When** a pure `should_destroy(counter: f32, disappearing_time: f32) -> bool` runs, **Then**
   it reproduces `part.rs:57` exactly; glue fires the `destroy` RPC and stops `process` the
   instant it returns `true`, unchanged from `v1`.
3. **Given** `explode`'s random initial spin (`(Vector3::new(randf(), randf(), randf())
   .normalized() * 2.0 - Vector3::ONE) * 10.0`), **When** a pure `random_angular_velocity(r1: f32,
   r2: f32, r3: f32) -> Vector3` runs (taking three already-sampled `[0.0, 1.0)` values — the
   RNG sampling itself stays in glue via `randf()`, matching `part.rs`'s own established
   precedent and `flying_forklift.rs`'s catalogued pattern), **Then** it reproduces
   `part.rs:90-93` exactly, including the un-normalized-after-shift result (the final `* 2.0 -
   Vector3::ONE` step is NOT renormalized, matching `v1` bit for bit).
4. **Given** the initial linear velocity (`3.0 * Vector3::UP`, a fixed constant) and the wait
   duration before arming `process` (`lifetime + lifetime_random * randf()`), **When** remodeled,
   **Then** the linear velocity constant and a pure `wait_time(lifetime: f32, lifetime_random:
   f32, r: f32) -> f32` (taking an already-sampled `r`) both reproduce `part.rs:89,95` exactly.
5. **Given** `explode`'s `create_timer(wait).signals().timeout().connect_other(...)` (arms
   `process`) and `destroy`'s `create_timer(0.2).signals().timeout().connect_other(...)` (frees
   the node), **When** backlog #5's async pattern closes both, **Then** each becomes a
   `godot::task::spawn` block using `to_future()` on the `SceneTree`-owned timer, checking
   `is_instance_valid()` before touching `self` afterward — same delays, same effects.
6. **Given** `destroy`'s puff spawn (`load::<PackedScene>("part_disappear.tscn")
   .instantiate_as::<CpuParticles3D>()`, added as a child of `self.base().get_parent()` — the
   `Death` node for a robot's parts — then positioned at the part's own global origin) and
   backlog #15 (the puff's lifetime is coupled to `Death`'s lifetime, which is freed 10 s after
   the robot's death, even though the puff itself finishes fading well before that in every
   observed case), **When** closed, **Then** the puff is instantiated as a child of the ROBOT's
   own parent (the node `EnemyRobot` itself sits under, e.g. `Level`'s spawn container) instead
   of `Death`; `part_disappear.tscn` is preloaded (`#[init(val = load(...))]`, matching V2-C's
   `bullet_scene` precedent) instead of loaded on every `destroy`; the puff's WORLD position is
   set identically (`set_global_position(origin)`), so this is a scene-tree-structure change
   with no visible difference under normal play, documented as the one intentional structural
   difference the parity harness's part scenario must account for (Edge Cases).
7. **Given** `ready`'s per-instance material override (upstream bug fix #2 — the duplicated
   surface material is installed via `set_surface_override_material` instead of mutating the
   shared `Mesh` resource, so `Death/PartShield1` and `PartShield2`'s independent fades both
   render), **When** remodeled, **Then** the fix and its `// upstream bug fix` comment stay
   EXACTLY where they are, untouched by this milestone (constitution Principle I: a v1 bug fix
   is not v2 work to revisit).
8. **Given** `explode`'s `MultiplayerSynchronizer`/`Col1`/`Col2` per-call `get_node_as` lookups
   and `ready`'s `get_node_as::<Node>("Model").get_child(0)` lookup, **When** remodeled, **Then**
   all four become `OnReady` fields resolved once (`OnReady<Gd<MultiplayerSynchronizer>>`,
   `OnReady<Gd<CollisionShape3D>>` ×2, and the `Model`'s first child as an `OnReady<Gd<
   MeshInstance3D>>` resolved with `OnReady::from_base_fn(|b| b.get_node_as::<Node>("Model")
   .get_child(0)…)` — NOT a fixed `#[init(node = "Model/<name>")]` path, because the three
   `Part` instances carry different models (`PartShield1/2` → `part_shield.glb`, `PartHead` →
   `part_head.glb`), so the child's name differs per instance exactly as `v1`'s `get_child(0)`
   already tolerates) — `explode`'s early non-server return (`if
   !self.base().get_multiplayer().unwrap().is_server() { return; }`) stays exactly where it is,
   AFTER the synchronizer visibility toggle, unchanged ordering from `v1`.

### Edge Cases

- **Dead `"Target"` branch (backlog #16)**: no `.tscn` in the project has ever had a node named
  `"Target"`; removing the branch changes nothing observable. Not a parity exclusion — there is
  nothing to exercise on either branch.
- **`aim_preparing`/`test_shoot` losing `#[export]` (backlog #18)**: no scene stores an override
  for either; removing `#[export]` changes nothing observable in the running game. If a future
  editor session had manually set either in the inspector (none currently do, verified), that
  value would be lost — documented here as the one theoretical (not actual) risk.
- **10 s removal / 0.1 s trauma delay (backlog #17)**: the async rewrite must produce the SAME
  wall-clock delays as `v1`'s `connect_other` chains — the parity harness's robot-death scenario
  measures frame counts to `queue_free()` and to the trauma call, asserting equality, not just
  "eventually happens."
- **Puff's parent path (backlog #15)**: the ONE documented structural (not behavioral) harness
  difference — `v1`'s puff is a child of `Death`; this branch's puff is a child of the robot's
  own parent. The harness's part-destroy scenario records BOTH parent paths rather than
  asserting they match, while asserting the puff's world position and fade timing DO match.
- **Same-tick hit reaching exactly zero health twice**: `hit`'s existing guard (`if self.dead {
  return; }`, checked FIRST, before any decrement) already prevents a hit landing on an
  already-dead robot from decrementing health further or re-firing the death sequence — this is
  unchanged `v1` behavior, not a new case introduced by this milestone, but the pure `hit_step`
  unit tests MUST cover it (a hit on `health == 0`/`dead == true` is glue's responsibility to
  never call `hit_step` for, mirroring the existing guard's placement).
- **Robot with no tracked player mid-Aim/Shooting**: `_on_area_body_exited` resets `state` to
  `Idle` and clears `player` the instant the player LEAVES the detection area, regardless of
  which state the robot was in (`Aim`/`Shooting` included) — `v1` does this unconditionally, and
  the remodel must not add a "finish the current action first" delay that `v1` never had.
- **First-laser hitch (backlog #28, informational only, other half)**: the checkpoint after US1
  asks the user to note, without any code change in response, whether preloading
  `impact_effect.tscn` changed the previously-observed hitch, this time for the ROBOT's laser
  shots specifically (V2-C already addressed the player's-bullet half). Recorded in the existing
  #28 row, which stays open.

## Requirements *(mandatory)*

### Functional Requirements

**`red_robot.rs` (US1)**

- **FR-001**: The replicated `#[export]` fields `target_position: Vector3`, `health: i32`,
  `state: State`, `dead: bool` MUST keep their exact names, types, `SceneReplicationConfig`
  modes and wire codes; `#[signal] exploded()`, RPCs `hit`/`play_shoot`, and `#[func]`s
  `resume_approach`, `shoot_check`, `_on_area_body_entered`, `_on_area_body_exited` MUST keep
  their exact names, signatures and attributes.
- **FR-002**: The internal (non-replicated) state carried by `aim_preparing`, `shoot_countdown`,
  `aim_countdown` MUST be represented so that each counter's validity is tied to the states in
  which `v1` actually uses it (`shoot_countdown` only meaningful during `Approach`;
  `aim_countdown` only during `Aim`; `aim_preparing` live during `Approach`/`Aim`/`Shooting`,
  meaningless during `Idle`) — the exact Rust encoding (an enum with per-variant data, or another
  shape preserving the same invariants) is a plan-time decision; the flat replicated `state:
  State` (FR-001) stays a projection of it, unchanged wire codes.
- **FR-003**: `aim_preparing: f32` and `test_shoot: bool` MUST lose `#[export]` (backlog #18,
  "do not export" — verified no scene stores an override for either) and remain plain `#[var]`
  fields; `test_shoot` stays settable by `shoot_check()` and consumed once per physics frame.
- **FR-004**: The dead `body.get_name() == "Target"` branch in `_on_area_body_entered` MUST be
  removed (backlog #16); `player: Option<Gd<Node3D>>` MUST become `Option<Gd<Player>>`.
- **FR-005**: The three duplicated `PhysicsRayQueryParameters3D`/`intersect_ray` blocks MUST
  become exactly ONE helper function, used at all three call sites, with the same collision
  mask (`0xFFFFFFFF`) and self-exclusion behavior.
- **FR-006**: The per-frame `Os::singleton().has_feature("dedicated_server")` check inside
  `_clip_ray` MUST be read exactly ONCE, at `ready`, into a stored field; every subsequent frame
  reads that field.
- **FR-007**: A pure state-transition step function (no `Gd`, no engine call) MUST reproduce
  `red_robot.rs:140-235` exactly: the `Approach` state's aim-preparing countdown and facing-angle
  gate before the shoot-countdown raycast check; the `Aim`/`Shooting` states' laser clip length,
  aim-preparing ramp, and aim-countdown raycast check gating the transition to `Shooting` (with
  `play_shoot`) or the return to `Approach` (via `resume_approach`'s reset); glue performs the
  raycasts (via FR-005's helper) and feeds their boolean outcome in.
- **FR-008**: Pure functions (no `Gd`; `Basis`/`Vector2`/`Vector3`/`f32` only, each verified
  against the 1.4.1 purity rule with source citations at plan time) MUST reproduce, unit-tested:
  the facing-angle computation (`atan2` of the transposed-basis-local target vector), the
  animation transition-request choice, the aim-blend step (`BLEND_AIM_SPEED`-scaled, clamped to
  `[-1.0, 1.0]` on both axes), and the `aiming/blend_amount` ramp.
- **FR-009**: Tunable constants (`PLAYER_AIM_TOLERANCE` — renamed from
  `PLAYER_AIM_TOLERANCE_DEGREES`, value unchanged, still radians; `SHOOT_WAIT`, `AIM_TIME`,
  `AIM_PREPARE_TIME`, `BLEND_AIM_SPEED`, the 10 s removal delay, the 0.1 s trauma delay, the
  13.0 trauma amount) MUST become a `RobotTuning` struct (or associated consts, per plan-time
  choice) with defaults matching `v1`'s literals.
- **FR-010**: Backlog #17 MUST be closed for BOTH waits in this module (`hit`'s 10 s removal,
  `shoot`'s 0.1 s trauma delay): each becomes a `godot::task::spawn` block awaiting a
  `SceneTree`-owned timer via `to_future()`, checking `is_instance_valid()` before acting,
  producing the same delay and the same effect as `v1`.
- **FR-011**: The laser hit on the player (`try_cast::<Player>()` + `add_camera_shake_trauma
  (13.0)`) MUST stay a direct typed call, NOT routed through `HitTarget` (Acceptance Scenario
  10's justification: `HitTarget` unifies "has a `hit` RPC", a question not being asked here).
- **FR-012**: The `hit` RPC MUST keep `v1`'s order: dead-guard → hit-reaction animation
  parameter (RNG pick) + hit sound on EVERY live hit → decrement → death sequence only when
  health just reached zero. A pure `hit_step(health: i32) -> (i32, bool)` MUST reproduce the
  decrement and the "health just reached zero" transition exactly; the death sequence's engine
  effects (visibility, `AnimationTree` off, sparks, the three `explode()`s, explosion sound,
  signal emission, the FR-010 removal wait) stay in glue, applied only on that transition.

**`part.rs` (US2)**

- **FR-013**: The replicated `fade_value: f32` (with its `#[func] set_fade_value` setter) and
  the `#[export]`s `lifetime`, `lifetime_random`, `disappearing_time` MUST keep their exact
  names, types and attributes; `explode` MUST keep its exact `#[func] fn explode(&mut self)`
  signature (called typed by `red_robot.rs::hit`); `destroy` MUST keep its exact `#[rpc]`
  signature (called by name from the animation method track).
- **FR-014**: A pure `fade_curve(counter: f32, disappearing_time: f32) -> f32` and a pure
  `should_destroy(counter: f32, disappearing_time: f32) -> bool` MUST reproduce `part.rs:54,57`
  exactly.
- **FR-015**: A pure `random_angular_velocity(r1: f32, r2: f32, r3: f32) -> Vector3` (three
  already-sampled inputs) and a pure `wait_time(lifetime: f32, lifetime_random: f32, r: f32) ->
  f32` (one already-sampled input) MUST reproduce `part.rs:90-93,95` exactly; RNG sampling itself
  (`randf()`) stays in glue.
- **FR-016**: Both `create_timer(...).signals().timeout().connect_other(...)` chains (`explode`'s
  arm-`process` wait, `destroy`'s pre-`queue_free` wait) MUST become `godot::task::spawn` blocks
  using `to_future()`, checking `is_instance_valid()` before acting — same delays, same effects.
- **FR-017**: The four per-event `get_node_as` lookups (`MultiplayerSynchronizer`, `Col1`,
  `Col2`, `Model`'s first child) MUST become `OnReady` fields resolved once; the `Model` child
  MUST be resolved by index (`get_child(0)` inside `OnReady::from_base_fn`), never by a fixed
  name, since it differs between the shield and head parts.
- **FR-018**: Backlog #15 MUST be closed: `destroy`'s puff MUST be instantiated as a child of
  the robot's own parent instead of `Death`, at the same world position (`set_global_position`
  unchanged); `part_disappear.tscn`'s load MUST become a preloaded `OnReady<Gd<PackedScene>>`
  field instead of a per-`destroy` `load::<PackedScene>` call.
- **FR-019**: Upstream bug fix #2 (the per-instance surface material override in `ready`) and
  its `// upstream bug fix` comment MUST remain untouched, in place, unmodified by this
  milestone.

**Cross-cutting (both modules)**

- **FR-020**: Each module's `#[godot_api] impl I<Base>`/`impl X` blocks MUST contain only
  lifecycle callbacks and exposed API, delegating to plain `impl`/free functions for domain logic
  (Principle III); `red_robot.rs` gains a pure `red_robot/model.rs` submodule; `part.rs` gains an
  inline `mod pure` (its pure surface is small, matching `bullet.rs`'s V2-C precedent).
- **FR-021**: `cargo build`/`cargo clippy`/`cargo test` MUST be clean before every commit;
  headless validation (`CLAUDE.md`'s recipe) MUST show no new errors after every commit.
- **FR-022**: A parity harness run against a `v1` worktree (separate `XDG_DATA_HOME` per tree,
  `--fixed-fps 60`) MUST cover: a robot alone with a scripted `Player` placed in and later
  removed from its detection area (per-frame `state`, `target_position`, aim blend, facing
  animation transition); the robot's laser hitting a wall vs. the tracked player (impact
  position, trauma-call frame count); `hit` ×5 driving the robot to `dead`, its three parts
  exploding, the `exploded` signal, and removal frame count; a `Part` alone driven through
  `explode`→fade→`destroy` (fade values per frame, destroy frame, puff world position) with its
  parent-path difference recorded per Edge Cases, not asserted equal.
- **FR-023**: One local commit per module (US1 may be two: pure model, then glue), each message
  naming what was remodeled; never pushed.

### Key Entities

- **Robot internal state** (`red_robot.rs`): the richer, non-replicated representation of
  `aim_preparing`/`shoot_countdown`/`aim_countdown` tied to the current `State`, per FR-002; the
  flat replicated `state: State` (`Idle | Approach | Aim | Shooting`, unchanged wire codes) is
  its projection.
- **`RobotTuning`** (`red_robot.rs`): the 7 tunable constants named in FR-009.
- **`RayHit`** (or equivalent, `red_robot.rs`): the one raycast helper's typed result, replacing
  three duplicated raw `VarDictionary` inspections (FR-005).
- **`Player` (typed)** (`red_robot.rs`): `player: Option<Gd<Player>>`, replacing `Option<Gd<
  Node3D>>` (FR-004).
- **`BulletState`/`HitTarget`**: unchanged from V2-C; `EnemyRobot` remains a `HitTarget::Robot`
  variant consumed by `bullet.rs`, not modified by this milestone.
- **Part fade/lifetime pure functions** (`part.rs`): `fade_curve`, `should_destroy`,
  `random_angular_velocity`, `wait_time`, per FR-014/FR-015.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build`/`cargo clippy`/`cargo test` clean, with at least 15 new unit tests
  across the pure functions/modules introduced by this milestone (the robot's state-transition
  step, facing angle, aim blend, `hit_step`, and `part.rs`'s `fade_curve`/`should_destroy`/
  `random_angular_velocity`/`wait_time`) — total test count at or above 77 (62 inherited from
  V2-C + 15).
- **SC-002**: Exactly ONE `intersect_ray` call site remains in `red_robot.rs` (grep).
- **SC-003**: Zero `load(`/`get_node_as`/`has_feature` calls remain inside
  `physics_process`/`shoot`/`hit`/`explode`/`destroy` bodies across both modules (grep + code
  review) — all such lookups moved to `OnReady` fields or fields read once at `ready`.
- **SC-004**: Zero nested `connect_other` timer chains remain in either module (grep) — both of
  `part.rs`'s and both of `red_robot.rs`'s replaced by the `godot::task::spawn` async pattern.
- **SC-005**: Headless validation (import + `main.tscn`/`level.tscn`) shows zero new errors
  relative to the documented baseline, after every commit.
- **SC-006**: The parity harness's non-excluded cases (robot state/facing/aim trace; laser
  impact and trauma-call frame counts; hit×5-to-death-to-removal frame count; part
  fade/destroy/puff-position trace) produce identical dumps between `v1` and this branch; the
  puff's parent-path difference (backlog #15) is a documented divergence, not an equality
  assertion.
- **SC-007**: `docs/v2-backlog.md` items #15, #16, #17, #18 are marked done citing this
  milestone's closing commits; #28 remains open, its existing row gaining a second annotation
  for the robot's-laser-shot half of the checkpoint observation.

## Assumptions

- Phase v2 (constitution 1.4.1). The behavior deviations from `v1` beyond the closed backlog
  items are exactly the four named in the top block (#15, #16, #17, #18) — all pre-authorized,
  all already listed. #16 and #18 are, by verification, not observable at all (dead code /
  unused export); #15 is a scene-tree-structure change with no visible difference under normal
  play; #17 is a timing-preserving implementation swap.
- **Robot internal state's exact Rust encoding** (an enum with per-variant counter data, as
  sketched in the milestone brief, vs. another shape preserving the same invariants) is a
  plan-time decision, per FR-002 and per V2-C's identical precedent for `HitTarget`'s shape and
  `bullet.rs`/`door.rs`'s module-layout choice — the constitution requires the invariants (which
  counter is live in which state), not a specific encoding.
- **The raycast helper's exact return shape** (`RayHit` struct vs. a plain `Option<(Vector3,
  InstanceId)>` or similar) is likewise a plan-time decision.
- **`red_robot.rs`'s root-motion integration** may literally reuse `player.rs`'s
  `integrate_root_motion` (V2-C) if the signature fits without contorting either caller, or
  define a twin pure function with the identical body — a plan-time decision, not fixed here,
  since both produce the same tested behavior either way.
- The parity harness reuses V2-B/C's `v1`-worktree + `XDG_DATA_HOME` + `--fixed-fps 60` pattern;
  building the robot-alone and part-alone scenarios (spawning `red_robot.tscn`/instancing a bare
  `Part` outside its usual `Death` parent for the standalone case) is new work for this
  milestone.
- Out of scope: `level.rs`/`main.rs`/`menu.rs` (unaffected, only consumers, verified unchanged
  call sites); `bullet.rs`/`hittable.rs`/`door.rs`/`player.rs` (V2-C, untouched — this milestone
  only CONSUMES `HitTarget`'s existing shape, never modifies it); backlog #3, #6, #19-#25, #29,
  #30 (out of scope, different modules or already-deferred decisions); any change to
  `red_robot.tscn` beyond what a name-preserving Rust remodel requires (none is expected — every
  `[connection]` and replicated property name stays exactly as it is today).
