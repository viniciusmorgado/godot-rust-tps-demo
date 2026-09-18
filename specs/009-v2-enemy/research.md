# Research: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

## R1 — Purity audit of every builtin method the pure model calls

Evidence gathered by reading the gdext 0.5.5 source directly (cargo registry cache,
`~/.cargo/registry/src/.../godot-core-0.5.5/src/builtin/`), the same methodology V2-C's
research.md R2 amendments used, per the 1.4.1 constitution rule (pure ⟺ body does not go
through `as_inner()` nor a generated `out/builtin_classes/**` method).

| Method | File:line | Body | Verdict |
|---|---|---|---|
| `Basis::transposed` | `matrices/basis.rs:373-375` | `Self::from_cols(self.rows[0], self.rows[1], self.rows[2])` — plain array reorg | **Pure** |
| `Basis * Vector3` (`Mul<Vector3> for Basis`) | `matrices/basis.rs:595-601` | `self.glam2(&rhs, \|a, b\| a * b)` | **Pure** (glam2, see below) |
| `Basis::orthonormalized` | `matrices/basis.rs:387-...` | Gram-Schmidt over `self.rows`/`self.determinant()`, plain float math | **Pure** |
| `Basis::from_quaternion` | `matrices/basis.rs:128-130` | `RMat3::from_quat(quat.to_glam()).to_front()` | **Pure** (glam) |
| `Transform3D * Transform3D` (`Mul for Transform3D`) | `matrices/transform3d.rs:291-297` | `self.glam2(&rhs, \|a, b\| a * b)` | **Pure** |
| `Transform3D * Vector3` | `matrices/transform3d.rs:299-305` | `self.glam2(&rhs, \|t, v\| t.transform_point3(v))` | **Pure** |
| `Vector3::length` / `Vector2::length` | `vectors/vector_macros.rs:453-456` | `(self.length_squared() as real).sqrt()`, `length_squared` → `self.to_glam().length_squared()` | **Pure** |
| `Vector3::normalized` / `Vector2::normalized` | `vectors/vector_macros.rs:795-804` | `try_normalized()` → `self / self.length()`, no engine | **Pure** |
| `Vector3::distance_to` / `Vector2::distance_to` | `vectors/vector_macros.rs:738-744` | `(to - self).length()` | **Pure** |
| `Vector2::clamp` | `vectors/vector_macros.rs:442-449` | `Self::from_glam(self.to_glam().clamp(min.to_glam(), max.to_glam()))` | **Pure** |
| `f32::atan2`, `f32::to_degrees` | n/a — `v1`/`red_robot.rs` calls these on plain `f32` SCALAR components (`to_player_local.x.atan2(to_player_local.z)`), not on a `Vector` type at all | `std::f32` | **Pure** (not even a gdext type involved) |
| `GlamConv::glam2`/`glam` helpers (shared by all the above) | `math/glam_helpers.rs:19-51` | pure type conversion (`to_glam`/`from_front`) + calling the supplied closure — no `sys::`, no FFI | **Pure** |

**Confirmed engine-backed (stay in glue, unchanged from `v1`'s call sites)**: `get_global_transform()`, `get_rid()`, `get_multiplayer()`, `get_gravity()`, `get_velocity()`/`set_velocity()`, `set_up_direction()`, `move_and_slide()`, `set_global_basis()`, `animation_tree.get_root_motion_position/rotation()`, `animation_tree.set(...)`/`.get(...)` (`Variant`-typed `AnimationTree` params), `get_world_3d().get_direct_space_state().intersect_ray(...)`, `Os::singleton().has_feature(...)`, `ray_mesh.get_surface_override_material(...).set_shader_parameter(...)`, `randi()`/`randf()` (global RNG singletons). None of these appear inside any pure function this milestone introduces.

**Conclusion**: every builtin math operation `red_robot.rs`'s pure model needs (facing-angle
`atan2`, the transposed-basis local-space transform, the aim-blend clamp, the root-motion
`Transform3D` composition/`orthonormalized`, `Vector3`/`Vector2` arithmetic) is confirmed pure
under the 1.4.1 rule — no `walk_target`/`slerp_toward`-style surprise like V2-C hit. `part.rs`'s
pure surface (`Vector3::new(...).normalized()`, arithmetic) is likewise all glam-based.

## R2 — Internal state model

Transcribing `v1`'s ACTUAL counter usage (`red_robot.rs:140-235`, `:270-274`, `:319-337`):

| Trigger | Effect on counters/state |
|---|---|
| `_on_area_body_entered` (Player enters) | `state = Approach`. Counters NOT reset — `aim_preparing`/`shoot_countdown` keep whatever value they last held (a `v1` quirk: re-entering the area after leaving mid-`Aim` resumes with a stale `shoot_countdown`, not a fresh `SHOOT_WAIT`). **Kept verbatim.** |
| `_on_area_body_exited` (Player leaves) | `state = Idle`, `player = None`. Counters untouched (irrelevant once `Idle`). |
| `Approach`, every frame | `aim_preparing` counts DOWN toward `0.0` (clamped there) if `> 0.0` — this is the ONLY place it decreases; `shoot_countdown` counts down only while facing the player within tolerance; on `shoot_countdown < 0.0` AND the raycast confirms sight: `state = Aim`, `aim_countdown = AIM_TIME`, `aim_preparing = 0.0` (forced, not just left at whatever the countdown reached — `red_robot.rs:183`); on `shoot_countdown < 0.0` and the raycast does NOT confirm sight: `shoot_countdown = SHOOT_WAIT` (reset, retry later), `aim_preparing` unaffected. |
| `Aim`/`Shooting`, every frame | `aim_preparing` counts UP toward `AIM_PREPARE_TIME` (clamped there) if `< AIM_PREPARE_TIME` — the ONLY place it increases; `aim_countdown` counts down UNCONDITIONALLY in BOTH `Aim` and `Shooting` (`red_robot.rs:205`, outside the `if ... state == State::Aim` that gates the RAYCAST check, so `Shooting`'s `aim_countdown` keeps draining below zero with no effect — a `v1` quirk since nothing reads it again once `Shooting`; **kept verbatim, harmless**); the raycast-gated transition (`aim_countdown < 0.0 && state == Aim` — note the explicit `state == Aim` guard, so this branch NEVER fires while `Shooting`) either sets `state = Shooting`, `shoot_countdown = SHOOT_WAIT`, RPCs `play_shoot` (sight confirmed), or calls `resume_approach()` (not confirmed). |
| `resume_approach()` (`#[func]`, called by the animation method track, NOT gated on current state) | `state = Approach`, `aim_preparing = AIM_PREPARE_TIME`, `shoot_countdown = SHOOT_WAIT` — an unconditional reset, callable at any time regardless of what state the robot was actually in when the track fires. |
| `ready`, if `test_shoot` | `shoot_countdown = 0.0` (forces an immediate shoot-countdown expiry on the next `Approach` frame — but only takes effect once `Approach` is entered; if never entered, has no observable effect, matching `v1`). |

**Decision**: `aim_preparing` and `aim_countdown`/`shoot_countdown` are NOT cleanly one-per-state
(both `aim_preparing`'s direction of travel AND `aim_countdown`'s liveness span `Aim` AND
`Shooting` together, and `resume_approach` resets counters unconditionally regardless of the
CURRENT state, including from `Idle` if the animation track somehow fired there — `v1` allows
this, so v2 must too). An enum with per-variant DATA that must be pattern-matched and
reconstructed on every transition (including `resume_approach`'s unconditional jump) adds
ceremony without eliminating an actual invalid combination `v1` avoids today (there is no
invalid combination in the FLAT representation either — every counter simply free-floats,
`v1`-style, and only the STATE `#[export]` enum needs to stay flat for the wire). **Internal
representation stays flat, matching `v1`'s own shape**: `aim_preparing: f32`, `shoot_countdown:
f32`, `aim_countdown: f32` as three plain (non-`#[export]`, non-replicated) struct fields
alongside the replicated `state: State`, exactly mirroring `v1`'s own field layout one-for-one.
This is a DELIBERATE departure from the milestone brief's sketched `RobotState { Approach {
aim_preparing, shoot_countdown }, ... }` enum: that shape would need to (a) special-case
`resume_approach`'s unconditional, state-agnostic reset, and (b) special-case `Aim`/`Shooting`
sharing one live `aim_countdown` span despite the raycast-gate's asymmetric `state == Aim`
check — both are irregularities the ENUM shape would have to encode as extra branches, buying
back nothing an already-flat, already-`v1`-faithful representation doesn't already have. The
type-system win this milestone delivers instead is the PURE, TESTABLE `step` function itself
(FR-007) — a `match self.state { ... }` over the flat `State`, not a richer enum. `aim_preparing`
still loses `#[export]` (FR-003, backlog #18) regardless of this decision.

**Projection to the replicated surface**: `state: State` (FR-001) IS the flat field directly
(no separate projection step needed, since the internal representation already uses `State` as
its discriminant) — this differs from `player.rs`'s `AnimPlan`-vs-`current_animation` split
(where the internal decision type is richer than the wire enum) precisely because `v1`'s own
robot state never carried richer per-state data to begin with.

## R3 — Pure `step` and raycast timing

`v1` raycasts LAZILY: the `Approach`→`Aim` raycast only runs on the exact frame
`shoot_countdown` crosses below `0.0` (`red_robot.rs:157`); the `Aim`→`Shooting`/`resume_approach`
raycast only runs on the exact frame `aim_countdown` crosses below `0.0` while `state == Aim`
(`:206`). Raycasting every frame regardless (feeding the pure step an unconditional "did the ray
hit" input) would change `v1`'s actual physics-query frequency — not faithful, and wasteful.

**Decision**: glue performs a cheap, pure PRE-CHECK before deciding whether to raycast at all —
`fn shoot_countdown_will_expire(shoot_countdown: f32, dt: f32) -> bool { shoot_countdown - dt <
0.0 }` and `fn aim_countdown_will_expire(aim_countdown: f32, dt: f32) -> bool { aim_countdown -
dt < 0.0 }` (two one-line pure predicates, unit-tested, the exact arithmetic `step` itself will
independently perform when it actually decrements). Glue calls the relevant predicate (based on
the CURRENT `state`, read before calling `step`) and only invokes `raycast_to` (R4) when it
returns `true` AND the same gate v1 applies before decrementing that counter holds: for
`Approach`, `facing(angle_to_player, tolerance) && shoot_countdown_will_expire(..)` (v1 only
counts down and raycasts while facing, `red_robot.rs:152-157`); for `Aim`,
`aim_countdown_will_expire(..)` (v1 `:205-206`; never in `Shooting`), passing the outcome into `step` as `sees_player: Option<bool>` (`None` on every
frame the predicate said no raycast was needed — `step`'s own internal branches only ever
consult `sees_player` on the exact same frames the predicate identified, so `None` is never
actually needed by `step`'s logic, but the type still allows it defensively). This keeps the
raycast frequency IDENTICAL to `v1` with no risk of the predicate and `step`'s internal
decrement-and-compare ever disagreeing (both are the same one-line formula, tested together).

```rust
pub struct RobotInputs {
    pub has_player: bool,
    pub angle_to_player: Option<f32>,       // None if !has_player
    pub sees_player: Option<bool>,          // Some only on a predicted-expiry frame (see above)
}

pub enum Cmd {
    RpcPlayShoot,
    ResumeApproach,           // glue calls the SAME reset resume_approach() itself performs
}

pub fn step(
    state: State,
    counters: &mut RobotCounters,   // { aim_preparing, shoot_countdown, aim_countdown } — &mut,
                                     // since v1 mutates them in place; returned commands describe
                                     // the OBSERABLE engine effects, not every counter write
    dt: f32,
    inputs: &RobotInputs,
    tuning: &RobotTuning,
) -> (State, Vec<Cmd>)
```

Transcribed line-for-line from `red_robot.rs:140-235` in `data-model.md` (full body, not
summarized here). `resume_approach()` itself STAYS a `#[func]` in glue (FR-001 — the animation
method track calls it BY NAME, unconditionally, regardless of `step`); glue's `Cmd::ResumeApproach`
(emitted by `step` when the `Aim`-raycast fails) calls the SAME counter-reset logic
`resume_approach()`'s body performs — to avoid duplicating that reset in two places, `step`
itself calls a shared PURE `fn resume_approach_reset(tuning: &RobotTuning) -> (f32, f32)` (`->
(aim_preparing, shoot_countdown)` new values) that both `step`'s `Cmd::ResumeApproach` PATH and
the glue `#[func] resume_approach()` call identically.

`animate` decomposed (transcribed from `red_robot.rs:406-456`):
- `pub fn transition_request(state: State, angle_to_player: Option<f32>, target_is_zero: bool,
  tuning: &RobotTuning) -> &'static str` — `Approach` branch: `turn_left`/`turn_right` outside
  `±PLAYER_AIM_TOLERANCE`, else `idle` if `target_is_zero` else `walk`; any other state: `idle`
  unconditionally (`v1`'s `else` branch, `:425-428`).
- `pub fn aim_blend_amount(aim_preparing: f32, tuning: &RobotTuning) -> f32` —
  `(aim_preparing / tuning.aim_prepare_time).clamp(0.0, 1.0)`.
- `pub fn cannon_angles(to_cannon_local: Vector3) -> (f32, f32)` — `(h_angle, v_angle)` in
  DEGREES: `h = to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees()`, `v =
  to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees()` (glue computes `to_cannon_local`
  itself, an engine-read `Transform3D`-local vector, and passes it in).
- `pub fn aim_blend_step(blend: Vector2, h_angle: f32, v_angle: f32, dt: f32, tuning:
  &RobotTuning) -> Vector2` — `blend.x += tuning.blend_aim_speed * dt * -h_angle`, clamped
  `[-1.0, 1.0]`; `blend.y += tuning.blend_aim_speed * dt * v_angle`, clamped `[-1.0, 1.0]`.
- `pub fn ember_position(max_dist: f32, mesh_offset: f32) -> Vector3` — `Vector3::new(0.0, 0.0,
  -max_dist / 2.0 - mesh_offset)`.
- `pub fn ember_extents(current: Vector3, max_dist: f32, mesh_offset: f32) -> Vector3` — `{
  extents.z = (max_dist - mesh_offset.abs()) / 2.0; extents }` (x/y unchanged from `current`,
  matching `v1`'s partial mutation of the existing extents value, `red_robot.rs:373-375`).

## R4 — `shoot()` glue

- `impact_effect_scene: OnReady<Gd<PackedScene>>` via `#[init(val = load("res://enemies/
  red_robot/laser/impact_effect/impact_effect.tscn"))]` (bare field, matching V2-C's
  `bullet_scene` precedent — `godot::tools::load` needs no tree, confirmed then).
- `laser_ember: OnReady<Gd<CpuParticles3D>>` via `#[init(node = "RedRobotModel/Armature/
  Skeleton3D/RayFrom/LaserEmber")]` (fixed path, unlike `part.rs`'s `Model` child — confirmed:
  `red_robot.tscn:10698`, a stable named node, not an indexed one).
- ONE helper: `fn raycast_to(&self, from: Vector3, to: Vector3) -> Option<RayHit>` where
  `RayHit { position: Vector3, collider: Option<Gd<Object>> }` (the collider handle, not yet
  resolved to an `InstanceId` — comparing `collider.instance_id() == player.instance_id()` stays
  the caller's job, exactly matching `v1`'s three call sites' identical comparison, just no
  longer duplicating the `PhysicsRayQueryParameters3D::create_ex(...).collision_mask(0xFFFFFFFF)
  .exclude(&array![rid]).done()` + `VarDictionary` unpacking three times). Builds the exclusion
  array from `self.base().get_rid()` internally.
- The 0.1 s trauma delay: `godot::task::spawn` capturing `Gd<Self>` AND the already-resolved
  `Gd<Player>` (cloned before the spawn, exactly as `v1`'s closure captures `player` by move);
  after `to_future()`, check `is_instance_valid()` on BOTH the robot's `Gd<Self>` (in case the
  robot itself was freed — not currently possible mid-frame, but matches the established
  guard-after-every-await convention) and the captured `Gd<Player>` (the player CAN disconnect/
  despawn independently) before calling `player.bind_mut().add_camera_shake_trauma(13.0)`.
- `Os::singleton().has_feature("dedicated_server")` read once into `is_dedicated_server: bool`
  at `ready`, replacing the per-frame `_clip_ray` check.

## R5 — `hit` RPC glue order

Transcribed from `red_robot.rs:276-311`, confirmed against the CORRECTED spec (Acceptance
Scenario 11 / FR-012 — hit reaction fires on EVERY live hit, before the decrement, not only on
the killing blow):

```text
1. if self.dead { return }                                    // unchanged guard, first
2. let param = format!("parameters/hit{}/request", randi() % 3 + 1);   // RNG in glue
   self.animation_tree.set(&param, &1.to_variant());
   self.hit_sound.play();
3. let (new_health, just_died) = pure::hit_step(self.health);  // pure, tested
   self.health = new_health;
4. if just_died {
       self.dead = true;
       self.animation_tree.set_active(false);
       self.model.set_visible(false);
       self.death.set_visible(true);
       self.collision_shape.set_disabled(true);
       self.death_detach_spark1.set_emitting(true);
       self.death_detach_spark2.set_emitting(true);
       self.death_shield1.bind_mut().explode();
       self.death_shield2.bind_mut().explode();
       self.death_head.bind_mut().explode();
       self.explosion_sound.play();
       self.signals().exploded().emit();
       if self.base().get_multiplayer().unwrap().is_server() {
           // godot::task::spawn + create_timer(10.0).to_future() + is_instance_valid() (FR-010)
       }
   }
```

`pure::hit_step(health: i32) -> (i32, bool)`: `let new = health - 1; (new, health > 0 && new ==
0)` — matches FR-012 exactly; a call with `health <= 0` is glue's responsibility to never make
(the `if self.dead { return }` guard at step 1 already ensures this, since `dead` and `health ==
0` become `true` together in the same RPC invocation and stay `true` forever after).

## R6 — Root motion: twin function, not cross-module reuse

`player::model::integrate_root_motion` (V2-C, `player/model.rs:137-156`) has the EXACT signature
`red_robot.rs` needs (`(orientation, root_motion, dt, gravity, velocity_in) -> (Transform3D,
Vector3)`) and an identical body to what `red_robot.rs:238-257` needs. However, `player.rs`'s
`mod model;` (`player.rs:10`) is declared PRIVATE (no `pub`/`pub(crate)`) — reusing it would
require a visibility edit to `player.rs`, a file this milestone's own spec Assumptions name as
OUT OF SCOPE / untouched ("`player.rs` (V2-C, untouched)"). **Decision: a TWIN pure function in
`red_robot/model.rs`** with the identical body (verified byte-for-byte equivalent against
`player/model.rs:137-156` at implementation time) — keeps `red_robot/model.rs` fully
self-contained, avoids a new cross-milestone module dependency for a ~15-line function, and
keeps `player.rs` genuinely untouched as the spec promises. Both twins are independently unit-
tested (no shared test burden either way).

No-player branch (`red_robot.rs:128-136`): pure `fn idle_velocity(gravity: Vector3, dt: f32) ->
Vector3 { gravity * dt }` (trivial, but named and tested for symmetry with the tracked-player
path) — glue sets `target_position = Vector3::ZERO`, calls `animate`/`transition_request` (which
already handles `target_is_zero` as an input, R3), applies the velocity, `move_and_slide()`.

## R7 — `part.rs`

- `OnReady<Gd<MultiplayerSynchronizer>>` (`"MultiplayerSynchronizer"`), `OnReady<Gd<
  CollisionShape3D>>` ×2 (`"Col1"`, `"Col2"`) — fixed named paths, straightforward `#[init(node
  = "...")]`.
- `Model`'s first child: NOT a fixed path — `PartShield1`/`PartShield2` instance
  `part_shield.glb` and `PartHead` instances `part_head.glb` (confirmed:
  `red_robot.tscn`'s three `Part` nodes' `RedRobotModel`... no — checked directly: each `Part`
  node's own `Model` child wraps a DIFFERENT `.glb`, so `get_child(0)`'s NAME differs per
  instance; only the INDEX is stable). `OnReady::from_base_fn(|base: &Gd<Node>| base.get_node_as::<Node>
  ("Model").get_child(0).unwrap().cast::<MeshInstance3D>())` (exact closure signature confirmed
  against gdext 0.5.5's source, `obj/on_ready.rs:192-199` — `from_base_fn<F>(init_fn: F) where F:
  FnOnce(&Gd<Node>) -> T + 'static`, called once, before `ready()` runs, identical timing to
  every other `OnReady` field, per spec FR-017).
- `part_disappear_scene: OnReady<Gd<PackedScene>>` via `#[init(val = load("res://enemies/
  red_robot/parts/part_disappear_effect/part_disappear.tscn"))]` (bare field, V2-C precedent).
- Pure: `fade_curve`, `should_destroy`, `random_angular_velocity`, `wait_time` — signatures
  exactly as spec FR-014/FR-015 states, transcribed from `part.rs:54,57,90-93,95`.
- Async: `explode`'s arm-`process` wait and `destroy`'s pre-`queue_free` wait both become
  `godot::task::spawn` + `to_future()` on the `SceneTree`-owned timer (`part_disappear.rs`'s
  established shape — Context, spec), `is_instance_valid()` checked before touching `self`.
- **Backlog #15's traversal** (puff parent = the ROBOT's own parent, not `Death`): from a
  `Part`, the chain is `self.base().get_parent()` (→ `Death`) → `.get_parent()` (→ the
  `EnemyRobot` itself, since `red_robot.tscn` parents `Death` directly under the robot root) →
  `.get_parent()` (→ whatever the ROBOT is parented under — `level.rs::spawn_robot`'s
  `self.spawned_nodes: OnReady<Gd<Node3D>>`, confirmed at `level.rs:172-175`, in the real game).
  Glue performs this 3-hop walk with `Option`-chaining (`get_parent()` on a node with no parent
  returns `None`); if the chain is SHORTER than 3 hops (e.g. a standalone `Part` instanced
  directly into the harness scene, with no `Death`/`EnemyRobot` ancestors at all — the parity
  harness's part-alone scenario, R8), it falls back to the LAST successfully-resolved parent
  (worst case, `self.base().get_parent()` itself, i.e. `Part`'s own immediate parent) — never
  panics, never silently drops the puff.

## R8 — Parity harness `zz_enemy_parity.tscn`/`.gd`

`red_robot.tscn`'s `PlayerDetectionArea` (Area3D): `collision_layer = 2`, `collision_mask = 2`.
`player.tscn`'s body: `collision_layer = 6`. Godot's area-detects-body rule (confirmed
empirically in V2-C, [[godot-area3d-mask-detection]] — `area.mask & body.layer`): `2 & 6 = 2`
(bit 1 set in both) → **detection works natively**, no harness-side mask widening needed (unlike
`door.tscn`'s mismatch in V2-C). `EnemyRobot::ready` reads nothing from `Settings` (grepped:
neither `red_robot.rs` nor `part.rs` mentions `Settings` at all) — both instantiate standalone
without a level.

Three cases, run on both trees with `XDG_DATA_HOME` + `--fixed-fps 60`:

- **(a) robot + tracked player**: `red_robot.tscn` on a harness floor; `player.tscn` teleported
  into `PlayerDetectionArea`'s volume, then later OUT of it. Dump per frame: `state`,
  `target_position`, `aim_preparing`, `animation_tree.get("parameters/aim/blend_position")`,
  the `LaserEmber`'s local position/extents once shooting starts, the frame `play_shoot` fires
  (observed via the `ShootAnimation` player's `current_animation`), and — for the trauma call —
  the PLAYER's camera rotation delta (the `CameraNoiseShake` node's rotation changing is the
  only externally-observable proxy for `add_camera_shake_trauma` actually firing, since trauma
  itself is a private field with no getter; comparing camera rotation across the expected
  trauma frame on both trees is sufficient for parity without adding a test-only accessor).
- **(b) `hit` ×5 to death**: a bare `red_robot.tscn` (no player needed), `robot.rpc("hit")`
  called 5 times a few frames apart (mirroring a real firefight's cadence, not same-tick, to
  avoid the RPC's own `unreliable` transport dropping calls sent faster than the network layer
  can flush in a single frame — a robustness precaution, not a spec requirement). Dump `dead`,
  `Death`'s visibility, each of the 3 parts' `fade_value` per frame once `explode()` fires
  (confirming US2 is exercised for free through US1's own scenario, exactly as V2-C's case (b)
  exercised `hittable.rs` through `bullet.rs`), the puff's resolved parent path (documented #15
  difference, NOT asserted equal) and world position (asserted equal), the `exploded` signal
  fire count (exactly 1), and the frame `queue_free()` actually removes the node (≈600 frames
  at 60fps for the 10s delay — the harness polls `is_instance_valid()` in a loop with a generous
  timeout, same pattern as V2-C's bullet-expiry-frame-counting case).
- **No separate standalone-part case**: case (b) already exercises `Part::explode`→fade→
  `destroy`→puff for all three parts typed through `EnemyRobot::hit` exactly as the real game
  does; a THIRD, separately-instanced bare `Part` (with no `Death`/`EnemyRobot` ancestors) is
  added ONLY to exercise the #15 traversal's fallback path (R7) — this is a harness-only,
  never-happens-in-the-real-game case, dumped once to confirm the puff still spawns (at the
  part's own immediate parent, the fallback) without panicking, not compared for equality
  against `v1` (this exact bare-part-with-no-robot-ancestor scenario cannot be constructed in
  `v1` either, since `v1`'s `part.gd` has the identical `get_parent()` call with no such
  fallback needed — there is nothing to diff, only a "does not crash" check).

Exclusions: none of #15/#16/#17/#18 produce an equality-breaking divergence in (a)/(b) (#16/#18
are unobservable by construction; #17 preserves timing; #15's puff PARENT differs but its
POSITION/fade timing don't — the harness records the parent path as informational, per spec
Edge Cases, while still asserting position/fade equality).

## R9 — Module layout and commit plan

- `red_robot.rs` (glue) + `red_robot/model.rs` (pure: `RobotTuning`, `State` re-exported from
  glue or defined in model — decided: `State` stays in `red_robot.rs` since it is the
  `#[derive(GodotConvert, Var, Export)]` wire type, `red_robot/model.rs` imports it), `step`,
  `RobotInputs`, `Cmd`, the `animate`-decomposition functions, `hit_step`, `integrate_root_motion`
  twin, `idle_velocity`, the two countdown-expiry predicates, `resume_approach_reset` — plus
  `#[cfg(test)] mod tests`.
- `part.rs` + inline `mod pure` (small surface: `fade_curve`, `should_destroy`,
  `random_angular_velocity`, `wait_time` — matches `bullet.rs`'s V2-C precedent exactly, smaller
  than `red_robot`'s pure surface).

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `red_robot: extract RobotTuning, State-projected step, animate decomposition, hit_step and a root-motion integration twin into red_robot/model.rs` | `red_robot/model.rs` (new) | gates |
| 2 | `red_robot: raycast_to helper, OnReady/preloaded resources, step-driven physics_process, async removal/trauma waits; closes backlog #16, #17, #18` | `red_robot.rs` | gates + headless |
| 3 | `part: pure fade/lifetime, OnReady resources incl. from_base_fn Model child, async waits, puff parented under the robot's own parent; closes backlog #15` | `part.rs` | gates + headless |
| 4 | `docs: close backlog #15, #16, #17, #18 citing this milestone's commits; annotate #28 with the robot-laser half; correct docs/v2-catalog.md's stale pure-builtins claim to the 1.4.1 rule` | `docs/v2-backlog.md`, `docs/v2-catalog.md` | none (docs-only) |

User visual checkpoints: after commit 2 (US1: robot approach/aim/shoot/death/removal) and after
commit 3 (US2: parts flying/fading/puff — though largely already visible from commit 2's
checkpoint, per US2's Independent Test; the checkpoint after commit 3 confirms the closed #15
puff-parent change specifically, plus asks the user to note the robot's-first-laser-shot hitch
per backlog #28's other half).
