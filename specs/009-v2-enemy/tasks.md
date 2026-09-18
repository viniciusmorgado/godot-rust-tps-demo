# Tasks: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

**Input**: Design documents from `/specs/009-v2-enemy/` (spec.md, plan.md, research.md R1–R9,
data-model.md, contracts/enemy-api.md, contracts/zz_enemy_parity.gd, quickstart.md)

**Branch**: `v2` — local commits only, **never** `git push`. **Baseline**: `541de80`.

**Tests**: requested by the spec (Principle III: pure logic MUST be `#[cfg(test)]`-covered).
Named test cases below are the MINIMUM the data-model pins; additional tests are welcome.

**Organization**: one task group per commit of `plan.md`'s Commit Plan, in order. The two user
visual checkpoints are STOP tasks — confirmed by the user, not the implementer.

## Phase 1: Setup

- [ ] T001 Add the `v1` worktree: `git worktree add ../oxide-godot-v1 v1`, then
  `cd ../oxide-godot-v1/oxide_godot_core && cargo build` and
  `cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .`.
  Verification: worktree builds clean, extension loads.
- [ ] T002 On `v2` at `541de80`, confirm the baseline gate: from `oxide_godot_core/`,
  `cargo build && cargo clippy && cargo test`. Verification: zero warnings, **62 tests pass**
  (spec.md Context — this is the number SC-001's "≥ 77 total" is measured against).

**Checkpoint**: baseline confirmed on both trees; US1 work can start.

---

## Phase 2: User Story 1 — `red_robot.rs` typed state machine, pure `step`, one raycast helper (Priority: P1)

**Goal**: FR-001…FR-012. **Independent Test**: per spec.md US1.

### Commit 1 — `red_robot/model.rs` (pure)

- [ ] T003 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/red_robot.rs`, add `mod model;`
  (matching `player.rs`'s `mod model;`). Create `oxide_godot_core/oxide_godot_lib/src/red_robot/
  model.rs` with `RobotTuning` (`#[derive(Clone, Copy, Debug)]`, 8 fields per data-model.md:
  `player_aim_tolerance: f32 = 15.0_f32.to_radians()` — RENAMED from
  `PLAYER_AIM_TOLERANCE_DEGREES`, value unchanged, still radians; `shoot_wait: f32 = 6.0`;
  `aim_time: f32 = 1.0`; `aim_prepare_time: f32 = 0.5`; `blend_aim_speed: f32 = 0.05`;
  `removal_delay: f32 = 10.0`; `trauma_delay: f32 = 0.1`; `trauma_amount: f64 = 13.0`) and its
  `impl Default` with exactly these `v1` literals (`red_robot.rs:21,23,24,26,27` + the inline
  literals at `:305,395,399`).
- [ ] T004 [P] [US1] In the same file, add `RobotCounters` (`aim_preparing: f32,
  shoot_countdown: f32, aim_countdown: f32`, `#[derive(Clone, Copy, Debug)]`), `RobotInputs`
  (`has_player: bool, angle_to_player: Option<f32>, sees_player: Option<bool>`, `#[derive(Clone,
  Copy, Debug)]`), and `Cmd` (`RpcPlayShoot, ResumeApproach`, `#[derive(Clone, Copy, Debug,
  PartialEq)]`).
- [ ] T005 [US1] Add the two raycast-timing predicates `shoot_countdown_will_expire(
  shoot_countdown: f32, dt: f32) -> bool` (`shoot_countdown - dt < 0.0`) and
  `aim_countdown_will_expire(aim_countdown: f32, dt: f32) -> bool` (`aim_countdown - dt < 0.0`)
  (research.md R3 — the exact arithmetic `step` itself independently performs); `facing(angle:
  f32, tolerance: f32) -> bool` (`angle > -tolerance && angle < tolerance` — `red_robot.rs:
  152-154`); `angle_to_player(local_target: Vector3) -> f32` (`local_target.x.atan2(
  local_target.z)` — `red_robot.rs:151`, the transposed-basis-local vector is computed by GLUE
  and passed in here as `local_target`).
- [ ] T006 [US1] Add `step(state: State, counters: &mut RobotCounters, dt: f32, inputs:
  &RobotInputs, tuning: &RobotTuning) -> (State, Vec<Cmd>)` — the LITERAL transcription of
  `red_robot.rs:140-235` per data-model.md's pseudocode, in `v1`'s exact order: **Approach**:
  `aim_preparing` counts down toward `0.0` (clamped) if `> 0.0`; if `inputs.angle_to_player` is
  `Some` and `facing(angle, tuning.player_aim_tolerance)`, decrement `shoot_countdown`; on
  crossing below `0.0`, `match inputs.sees_player`: `Some(true)` → `state = Aim`,
  `aim_countdown = tuning.aim_time`, `aim_preparing` FORCED to `0.0` (`:181-183`); anything else
  → `shoot_countdown = tuning.shoot_wait` (retry later, `:186`). **Aim | Shooting**:
  `aim_preparing` counts UP toward `tuning.aim_prepare_time` (clamped) if `<`; `aim_countdown`
  decrements UNCONDITIONALLY in both states (`:205`); if `aim_countdown < 0.0 && state == Aim`
  (the gate EXPLICITLY excludes `Shooting` — `v1` quirk, kept verbatim, `Shooting`'s
  `aim_countdown` keeps draining below zero with no further effect): `match inputs.sees_player`:
  `Some(true)` → `state = Shooting`, `shoot_countdown = tuning.shoot_wait`, push
  `Cmd::RpcPlayShoot`; anything else → call `resume_approach_reset` (T007), apply its result to
  `counters.aim_preparing`/`shoot_countdown`, `state = Approach`, push `Cmd::ResumeApproach`.
  **Idle**: no-op (matches `v1` — no `if self.state == State::Idle` branch exists at all).
  Depends on T003, T004, T005.
- [ ] T007 [US1] Add `resume_approach_reset(tuning: &RobotTuning) -> (f32, f32)` returning
  `(tuning.aim_prepare_time, tuning.shoot_wait)` — the SAME reset formula `v1`'s `#[func]
  resume_approach()` performs (`red_robot.rs:271-273`), shared by `step`'s `Cmd::ResumeApproach`
  path (T006) and the glue `#[func]` (T020) so the reset lives in exactly one place.
- [ ] T008 [US1] Add `hit_step(health: i32) -> (i32, bool)` — `let new = health - 1; (new,
  health > 0 && new == 0)` (`red_robot.rs:284-285` — the decrement and the "health JUST reached
  zero" transition only; the RNG hit-reaction pick and every engine effect of the death sequence
  stay in glue, T019).
- [ ] T009 [US1] Add the `animate` decomposition (`red_robot.rs:406-456`): `transition_request(
  state: State, angle_to_player: Option<f32>, target_is_zero: bool, tuning: &RobotTuning) ->
  &'static str` (`Approach`: `"turn_left"`/`"turn_right"` outside `±tuning.player_aim_tolerance`,
  else `"idle"` if `target_is_zero` else `"walk"`; any other state: `"idle"` unconditionally,
  `v1`'s `else` branch `:425-428`); `aim_blend_amount(aim_preparing: f32, tuning: &RobotTuning)
  -> f32` (`(aim_preparing / tuning.aim_prepare_time).clamp(0.0, 1.0)`); `cannon_angles(
  to_cannon_local: Vector3) -> (f32, f32)` (`(h, v)` in DEGREES: `h =
  to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees()`, `v =
  to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees()` — `:440-441`); `aim_blend_step(
  blend: Vector2, h_angle: f32, v_angle: f32, dt: f32, tuning: &RobotTuning) -> Vector2`
  (`blend.x += tuning.blend_aim_speed * dt * -h_angle`, clamp `[-1.0, 1.0]`; `blend.y +=
  tuning.blend_aim_speed * dt * v_angle`, clamp `[-1.0, 1.0]` — `:446-452`); `ember_position(
  max_dist: f32, mesh_offset: f32) -> Vector3` (`Vector3::new(0.0, 0.0, -max_dist / 2.0 -
  mesh_offset)` — `:372`); `ember_extents(current: Vector3, max_dist: f32, mesh_offset: f32) ->
  Vector3` (only `.z = (max_dist - mesh_offset.abs()) / 2.0` changes, x/y unchanged from
  `current` — `:373-374`).
- [ ] T010 [US1] Add the root-motion integration TWIN `integrate_root_motion(orientation:
  Transform3D, root_motion: Transform3D, dt: f32, gravity: Vector3, velocity_in: Vector3) ->
  (Transform3D, Vector3)` — body IDENTICAL to `player/model.rs:137-156` (research.md R6: a
  twin, not a cross-module import, since `player.rs`'s `mod model;` is private and this
  milestone leaves `player.rs` untouched); `idle_velocity(gravity: Vector3, dt: f32) -> Vector3`
  (`gravity * dt` — `red_robot.rs:128-136`'s no-player-branch velocity).
- [ ] T011 [US1] Add `#[cfg(test)] mod tests` covering: `shoot_countdown_will_expire`/
  `aim_countdown_will_expire` true/false around the zero crossing; `facing` true inside the
  tolerance, false outside (both signs); `angle_to_player` returns `0.0` for a target straight
  ahead (`+Z` local) and `±π/2` for one 90° to either side; `hit_step(5)` → `(4, false)`;
  `hit_step(1)` → `(0, true)` (health JUST reached zero); a call with `health <= 0` is NOT this
  function's concern — glue's existing `if self.dead { return }` guard (unchanged from `v1`)
  never lets that happen, so no test asserts `hit_step`'s behavior there.
- [ ] T012 [US1] Add to `mod tests`: `step`'s 8 named scenarios, each asserting BOTH the
  returned `State` and the exact `RobotCounters`/`Cmd` values — (a) `Approach`, `aim_preparing >
  0.0`, no raycast this frame: counts DOWN and clamps at `0.0`; (b) `Approach`,
  `angle_to_player` outside tolerance (or `None`): `shoot_countdown` does NOT decrement; (c)
  `Approach`, countdown expires, `sees_player: Some(true)`: → `Aim`, `aim_countdown ==
  tuning.aim_time`, `aim_preparing` FORCED to `0.0` (even if it was non-zero going in); (d)
  `Approach`, countdown expires, `sees_player: Some(false)`: stays `Approach`,
  `shoot_countdown == tuning.shoot_wait` (reset, retry); (e) `Aim`, `aim_countdown` expires,
  `sees_player: Some(true)`: → `Shooting`, `shoot_countdown == tuning.shoot_wait`, `cmds ==
  [Cmd::RpcPlayShoot]`; (f) `Aim`, `aim_countdown` expires, `sees_player: Some(false)`: →
  `Approach`, counters reset via `resume_approach_reset`'s values, `cmds ==
  [Cmd::ResumeApproach]`; (g) `Shooting`, `aim_countdown` crosses below `0.0`: state stays
  `Shooting` (the `state == Aim` gate excludes it — `v1` quirk), `aim_countdown` is negative
  (kept draining, no clamp); (h) `Idle`: state and counters both unchanged, `cmds` empty.
- [ ] T013 [US1] Add to `mod tests`: `transition_request` covering all 4 outcomes
  (`"turn_left"`/`"turn_right"`/`"idle"`/`"walk"`) plus the non-`Approach` `"idle"` fallback;
  `aim_blend_step` clamps BOTH axes at `±1.0` for an over-large `h_angle`/`v_angle` input, and
  moves partway (not equal to either endpoint) for a mid-range input; `cannon_angles` returns
  the correct SIGN for targets to the left/right and above/below, in degrees; `ember_position`/
  `ember_extents` reproduce the exact formulas for a representative `max_dist`/`mesh_offset`
  pair; `integrate_root_motion` (the twin) — horizontal velocity equals `root_motion.origin /
  dt`, gravity added, returned orientation's origin is `Vector3::ZERO` and its basis is
  orthonormal (same assertions as `player/model.rs`'s existing test, confirming the twin
  matches byte-for-byte); `idle_velocity` equals `gravity * dt` for a non-trivial `dt`.
- [ ] T014 [US1] Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`
  (zero warnings; all T011–T013 tests pass; ≥ 15 new tests in this file per spec SC-001).
  Commit: `red_robot: extract RobotTuning, State-projected step, animate decomposition,
  hit_step and a root-motion integration twin into red_robot/model.rs`.

### Commit 2 — `red_robot.rs` glue

- [ ] T015 [US1] In `oxide_godot_core/oxide_godot_lib/src/red_robot.rs`: change `player:
  Option<Gd<Node3D>>` to `player: Option<Gd<Player>>` (backlog #16, part 1); remove `#[export]`
  from `aim_preparing: f32` and `test_shoot: bool`, keeping both as plain `#[var]` fields
  (backlog #18 — confirmed no scene stores an override for either); keep `shoot_countdown`/
  `aim_countdown` as plain (non-`#[export]`, non-`#[var]`) fields exactly as `v1` has them;
  add `is_dedicated_server: bool` and set it ONCE in `ready()` via
  `Os::singleton().has_feature("dedicated_server")` (replacing `_clip_ray`'s per-frame check);
  add `#[init(val = load("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn"))]
  impact_effect_scene: Gd<PackedScene>` (bare field, matching V2-C's `bullet_scene` precedent —
  no tree dependency); add `#[init(node = "RedRobotModel/Armature/Skeleton3D/RayFrom/
  LaserEmber")] laser_ember: OnReady<Gd<CpuParticles3D>>` (was `get_node_as` per shot); add
  `rid: Rid` captured once in `ready()` via `self.base().get_rid()` (was re-read at all three
  raycast call sites).
- [ ] T016 [US1] Add ONE helper `fn raycast_to(&self, from: Vector3, to: Vector3) ->
  Option<RayHit>` (`RayHit { position: Vector3, collider: Option<Gd<Object>> }`, data-model.md)
  building `PhysicsRayQueryParameters3D::create_ex(from, to).collision_mask(0xFFFFFFFF)
  .exclude(&array![self.rid]).done()` and calling `intersect_ray` ONCE, replacing the three
  duplicated blocks at `red_robot.rs:162-172`/`:210-220`/`:349-359`. Returns `None` for an
  empty result `VarDictionary` (matches every `!col.is_empty()` guard in `v1`).
- [ ] T017 [US1] Rewrite `physics_process(&mut self, delta: f64)` per research.md R3/R5's exact
  sequence: (1) `if self.dead { return }` (unchanged, first); (2) non-server: call `animate`'s
  glue equivalent (T017 continues to own this — build the `AnimPlan`-equivalent from replicated
  state and `return`, mirroring `red_robot.rs:118-121`); (3) `if self.test_shoot { self.shoot();
  self.test_shoot = false; }` (unchanged); (4) no-player branch: `target_position =
  Vector3::ZERO`, `model::idle_velocity` for the velocity, `move_and_slide()`, animate, early
  `return` (mirrors `:128-136`); (5) snapshot: `target_position = player.get_global_transform()
  .origin`; for `Approach`, compute `angle_to_player` via `model::angle_to_player` fed the
  transposed-basis-local target vector (glue computes `gt.basis.transposed() * (target_position
  - gt.origin)`, an engine read, then hands the plain `Vector3` to the pure function); (6)
  GATED raycast (research.md R3): in `Approach`, only if `model::facing(angle,
  tuning.player_aim_tolerance) && model::shoot_countdown_will_expire(self.shoot_countdown, dt)`;
  in `Aim`, only if `model::aim_countdown_will_expire(self.aim_countdown, dt)`; NEVER in
  `Shooting`/`Idle`; when gated true, call T016's `raycast_to` and compare
  `result.collider.map(|c| c.instance_id()) == Some(player.instance_id())` for `sees_player`;
  (7) build `RobotCounters`/`RobotInputs`, call `model::step`, write the returned counters back,
  update `self.state`; (8) apply `Cmd`s: `RpcPlayShoot` → `self.base_mut().rpc("play_shoot",
  &[])`; `ResumeApproach` → call `self.resume_approach()` (the SAME `#[func]`, T020, not a
  duplicate reset); (9) in `Aim`/`Shooting`, laser-clip: `max_dist` from
  `self.laser_raycast.is_colliding()` exactly as `:192-196`, call the (unchanged, glue-only)
  shader-parameter write using `self.is_dedicated_server` (T015) instead of a fresh
  `has_feature` call; (10) `animate`'s remaining piece (aim blend, only when `target_position !=
  Vector3::ZERO`): read `ray_mesh`'s global transform, compute `to_cannon_local`, call
  `model::cannon_angles`, `model::aim_blend_step` on the current `parameters/aim/
  blend_position`, write it back; `model::aim_blend_amount` → `parameters/aiming/blend_amount`;
  `model::transition_request` → `parameters/state/transition_request`; (11)
  `model::integrate_root_motion` (T010's twin) fed the `AnimationTree`'s root-motion
  position/rotation; (12) `set_velocity`/`set_up_direction(Vector3::UP)`/`move_and_slide()`;
  (13) clear `orientation.origin`, orthonormalize, `set_global_basis`.
- [ ] T018 [US1] Rewrite `shoot(&mut self)`: use T016's `raycast_to` helper (was the inline
  `PhysicsRayQueryParameters3D` block at `:349-359`); on a hit, `max_dist =
  ray_origin.distance_to(result.position)`, else `1000.0` (unchanged); laser-clip via
  `self.is_dedicated_server` (T015, no fresh `has_feature` call); ember placement via
  `model::ember_position`/`model::ember_extents` (T009) on `self.laser_ember` (T015's `OnReady`
  field, was `get_node_as` per shot); on a hit, instantiate `self.impact_effect_scene` (T015's
  preloaded field, was `load()` per shot) via `.instantiate_as::<Node3D>()`, add it under
  `self.base().get_tree().get_root()` exactly as `:382` does; if the hit collider is
  `self.player` (compared by `instance_id`, unchanged), spawn a `godot::task::spawn` block
  capturing the resolved `Gd<Player>` and `Gd<Self>`, `create_timer(self.tuning-or-const
  0.1).signals().timeout().to_future().await`, then `is_instance_valid()` on BOTH before calling
  `player.bind_mut().add_camera_shake_trauma(13.0)` — NOT routed through `HitTarget`
  (spec FR-011/Acceptance Scenario 10's justification).
- [ ] T019 [US1] Rewrite `hit(&mut self)` in `v1`'s exact order (data-model.md/research.md R5):
  `if self.dead { return }` (unchanged, first); on EVERY live hit, BEFORE the decrement: pick
  the random hit-reaction animation parameter (`randi() % 3 + 1`, RNG stays in glue) and play
  the hit sound (unchanged); call `model::hit_step(self.health)`, store the new `self.health`;
  if the returned `bool` is `true` (health JUST reached zero): `self.dead = true`, deactivate
  `AnimationTree`, hide the model, show `Death`, disable the collision shape, start both detach
  sparks, call `explode()` on all three parts (typed, unchanged signature), play the explosion
  sound, emit `exploded`, and — server only (`self.base().get_multiplayer().unwrap()
  .is_server()`, unchanged) — a `godot::task::spawn` block awaiting `create_timer(10.0)
  .signals().timeout().to_future()`, then `is_instance_valid()` before `queue_free()` (backlog
  #17, replacing the `connect_other` chain at `:305-308`).
- [ ] T020 [US1] Rewrite `_on_area_body_entered(&mut self, body: Gd<Node3D>)`: remove the dead
  `|| body.get_name() == "Target"` branch (backlog #16, part 2 — confirmed unreachable, no
  `.tscn` has a `"Target"` node); the surviving condition is `body.clone()
  .try_cast::<Player>().is_ok()`, and on success store the TYPED cast result into
  `self.player: Option<Gd<Player>>` directly (not the original untyped `body`) — set `state =
  Approach`. `_on_area_body_exited` keeps its exact signature and body (`try_cast::<Player>` +
  clear `self.player`/reset `state`), unaffected by the field's new type. `resume_approach(
  &mut self)` calls `model::resume_approach_reset(&self.tuning)` (T007) and writes both fields,
  replacing its inline `AIM_PREPARE_TIME`/`SHOOT_WAIT` literals — its own `#[func]` signature is
  unchanged. `shoot_check`, `#[signal] exploded`, RPCs `hit`/`play_shoot` keep their exact
  signatures/attributes throughout.
- [ ] T021 [US1] Gates: `cargo build && cargo clippy && cargo test`. Headless:
  `/usr/bin/godot.x86_64 --headless --path oxide-godot/oxide-godot --import` then
  `--headless --path oxide-godot/oxide-godot main/main.tscn --quit-after 120` and
  `level/level.tscn --quit-after 120` (no new errors vs. baseline). Grep (SC-002): `grep -c
  intersect_ray oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → exactly `1`. Grep
  (SC-003): `grep -n 'load(\|get_node_as\|has_feature'
  oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → no matches inside
  `physics_process`/`shoot`/`hit` bodies (review any hits outside them, e.g. `#[init]`
  attributes). Grep (SC-004, this module's half): `grep -n connect_other
  oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → no matches. Commit: `red_robot: raycast_to
  helper, OnReady/preloaded resources, step-driven physics_process, async removal/trauma waits;
  closes backlog #16, #17, #18`.

### Harness + checkpoint

- [ ] T022 [US1] From `contracts/zz_enemy_parity.gd`, write `oxide-godot/oxide-godot/
  zz_enemy_parity.tscn` + `.gd` (root `Node`, `--case=` cmdline dispatch, `_make_floor()`
  helper). Fill in case (a): instance `red_robot.tscn` on the harness floor, instance
  `player.tscn` and teleport it into `PlayerDetectionArea`'s volume (no collision-mask override
  needed — research.md R8: `mask=2` already intersects `player.tscn`'s `layer=6`), await
  frames, dump per frame `state`, `target_position`, `aim_preparing`,
  `animation_tree.get("parameters/aim/blend_position")`, and the frame `ShootAnimation`'s
  `current_animation` becomes `"shoot"`; then teleport the player back OUT of the area and dump
  the `Idle` transition too.
- [ ] T023 [US1] Fill in case (b): a bare `red_robot.tscn` (no player needed), `robot.rpc("hit")`
  called 5 times a few frames apart; dump `dead`, `Death`'s visibility, the `exploded` signal
  fire count (must be exactly 1), and the frame `is_instance_valid(robot)` first turns `false`
  (poll with a generous timeout, ~600 frames @60fps expected for the 10 s delay). `part.rs`
  itself is still `v1`-shaped at this point (commit 3 hasn't landed) — this case exercises US1's
  `hit`/death sequence only; part-level dumps (`fade_value`, puff path) are added in T033 once
  `part.rs` is remodeled.
- [ ] T024 [US1] Copy the harness files into `../oxide-godot-v1/oxide-godot/`. Run cases (a) and
  (b) on both trees per quickstart.md §4 (`XDG_DATA_HOME=/tmp/parity-v1` / `/tmp/parity-v2`,
  `--fixed-fps 60`). Verification: (a) identical dumps on both trees (no backlog item in this
  commit changes any OBSERVABLE trace — #16/#18 are unobservable by construction); (b)
  identical `dead`/signal-count/removal-frame-count on both trees.
- [ ] T025 [US1] 🛑 **STOP — user visual checkpoint 1**. Ask the user to run the game and
  confirm: a robot detects the player, turns to face, prepares, aims, fires its laser (impact
  effect visible; camera shakes if the laser hits the player), and — after 5 hits — dies, its
  three parts fly apart, and it vanishes roughly 10 seconds later. Also ask the user to NOTE —
  without any code change in response — whether the robot's first laser shot of a sequence
  still shows the previously-observed hitch (backlog #28, the robot half — V2-C already closed
  the player's-bullet half). Do not proceed to Phase 3 until confirmed.

**Checkpoint**: User Story 1 complete and independently verified.

---

## Phase 3: User Story 2 — `part.rs` pure fade/lifetime, async waits, the puff's parent (Priority: P2)

**Goal**: FR-013…FR-019. **Independent Test**: per spec.md US2.

### Commit 3 — `part.rs`

- [ ] T026 [P] [US2] In `oxide_godot_core/oxide_godot_lib/src/part.rs`, add an inline
  `mod pure { ... }` with `fade_curve(counter: f32, disappearing_time: f32) -> f32`
  (`(counter / disappearing_time).powi(2)` — `part.rs:54`), `should_destroy(counter: f32,
  disappearing_time: f32) -> bool` (`counter >= disappearing_time - 0.2` — `:57`),
  `random_angular_velocity(r1: f32, r2: f32, r3: f32) -> Vector3` (`(Vector3::new(r1, r2, r3)
  .normalized() * 2.0 - Vector3::ONE) * 10.0` — `:90-93`, three ALREADY-SAMPLED `[0.0, 1.0)`
  inputs, RNG stays in glue), `wait_time(lifetime: f32, lifetime_random: f32, r: f32) -> f32`
  (`lifetime + lifetime_random * r` — `:95`, one already-sampled input).
- [ ] T027 [US2] Add `#[cfg(test)] mod tests` inside `mod pure`: `fade_curve` reproduces the
  squared ratio for a representative counter/time pair; `should_destroy` is `false` just below
  `disappearing_time - 0.2` and `true` at/above it; `random_angular_velocity` reproduces the
  exact formula for representative `r1`/`r2`/`r3` inputs, confirming the result is NOT
  renormalized after the `* 2.0 - Vector3::ONE` shift (matches `v1` bit for bit); `wait_time`
  reproduces the formula for a representative `lifetime`/`lifetime_random`/`r` triple.
- [ ] T028 [US2] Rename `_mat`/`_disappearing_counter` to `material: Option<Gd<Material>>`/
  `disappearing_counter: f32` (Rust naming, `v1`'s GDScript-style names dropped — neither is
  exported/replicated either way, no surface change). Add `synchronizer: OnReady<Gd<
  MultiplayerSynchronizer>>` (`#[init(node = "MultiplayerSynchronizer")]`), `col1`/`col2:
  OnReady<Gd<CollisionShape3D>>` (`#[init(node = "Col1")]`/`"Col2"`), `model_mesh: OnReady<Gd<
  MeshInstance3D>>` via `#[init(val = OnReady::from_base_fn(|base: &Gd<Node>| base
  .get_node_as::<Node>("Model").get_child(0).unwrap().cast::<MeshInstance3D>()))]` (NOT a fixed
  `#[init(node = ...)]` path — research.md R7: the child's NAME differs per instance,
  `PartShield1`/`2` vs `PartHead`, only the INDEX `0` is stable), `part_disappear_scene: Gd<
  PackedScene>` via `#[init(val = load("res://enemies/red_robot/parts/part_disappear_effect/
  part_disappear.tscn"))]` (bare field, V2-C precedent).
- [ ] T029 [US2] Rewrite `ready()`: the per-instance surface-material override (upstream bug fix
  #2) stays EXACTLY where it is, with its `// upstream bug fix` comment, UNMODIFIED — only the
  `MeshInstance3D` lookup it starts from now reads `self.model_mesh` (T028) instead of
  `get_node_as::<Node>("Model").get_child(0)` inline. Rewrite `process(&mut self, delta: f64)`:
  `pure::fade_curve(self.disappearing_counter, self.disappearing_time)` →
  `self.set_fade_value(...)`; `self.disappearing_counter += delta as f32`;
  `pure::should_destroy(...)` → if `true`, RPC `destroy` and `self.base_mut()
  .set_process(false)` (unchanged effects, `:53-61`).
- [ ] T030 [US2] Rewrite `explode(&mut self)`: the synchronizer-visibility toggle and the
  non-server early return stay in their EXACT current order (`:80-86`, unchanged); `self.col1`/
  `self.col2` (T028's `OnReady` fields, were `get_node_as` per call) `.set_disabled(false)`;
  linear velocity `3.0 * Vector3::UP` (unchanged literal); `pure::random_angular_velocity`
  fed three fresh `randf()` samples (RNG stays in glue) → `set_angular_velocity`;
  `pure::wait_time(self.lifetime, self.lifetime_random, randf() as f32)` → a `godot::task::spawn`
  block awaiting `create_timer(wait as f64).signals().timeout().to_future()`, then
  `is_instance_valid()` before `self.base_mut().set_process(true)` (backlog #5-style async,
  replacing the `connect_other` chain at `:96-101`).
- [ ] T031 [US2] Rewrite `destroy(&mut self)`: instantiate `self.part_disappear_scene` (T028's
  preloaded field, was `load()` per call) via `.instantiate_as::<CpuParticles3D>()`; resolve the
  puff's parent per research.md R7's 3-hop walk — `self.base().get_parent()` (→ `Death`)
  `.and_then(|d| d.get_parent())` (→ the `EnemyRobot`) `.and_then(|r| r.get_parent())` (→ the
  robot's own parent), falling back to the LAST successfully-resolved ancestor if any hop
  returns `None` (worst case, `self.base().get_parent()` itself — never panics) — add the puff
  as a child of THAT node instead of `Death` (backlog #15); `puff.set_global_position(self
  .base().get_global_transform().origin)` (unchanged); a `godot::task::spawn` block awaiting
  `create_timer(0.2).signals().timeout().to_future()`, then `is_instance_valid()` before
  `self.base_mut().queue_free()` (replacing the `connect_other` chain at `:113-118`).
- [ ] T032 [US2] Gates + headless. Grep (SC-004, this module's half): `grep -n connect_other
  oxide_godot_core/oxide_godot_lib/src/part.rs` → no matches. Commit: `part: pure fade/lifetime,
  OnReady resources incl. from_base_fn Model child, async waits, puff parented under the robot's
  own parent; closes backlog #15`.

### Harness + checkpoint

- [ ] T033 [US2] Re-run harness case (b) (T023/T024's script, unchanged) on both trees — now
  that `part.rs` is remodeled, extend the dump to include each of the 3 parts' `fade_value`
  trace, the frame each part's `destroy` RPC fires, the puff's resolved PARENT PATH per part,
  and the puff's WORLD POSITION per part. Verification: `fade_value` traces, destroy frames, and
  world positions are IDENTICAL between `v1`/`v2`; the parent path is a DOCUMENTED DIVERGENCE
  (expect `Death` on `v1`, the robot's own parent — e.g. the harness's own root `Node` in this
  standalone scenario — on `v2`) — report it, do not assert equality (spec Edge Cases/SC-006).
- [ ] T034 [US2] 🛑 **STOP — user visual checkpoint 2**. Ask the user to confirm: a destroyed
  robot's parts still fly apart with visible spin and gravity, fade near the end of their
  lifetime, disappear, and leave a brief particle puff behind at the correct position — no
  visible difference from before (the puff's new parent, backlog #15, has no observable effect
  under normal play). Do not proceed to Phase 4 until confirmed.

**Checkpoint**: Both user stories complete and independently verified.

---

## Phase 4: Polish

- [ ] T035 Update `docs/v2-backlog.md`: mark items **#15, #16, #17, #18** done, each citing the
  commit that closed it (T014's commit introduces the pure math but T021's commit is where
  #16/#17/#18 actually take effect in `red_robot.rs`; #15 closes at T032's commit); annotate
  **#28**'s existing row with a SECOND note — the user's checkpoint-1 (T025) observation on the
  robot's-laser-shot half of the hitch (the row stays `open`, only gains a note, exactly as its
  first annotation did in V2-C); add a NEW row **#31** (`open`): origin
  `enemies/red_robot/red_robot.gd`/`red_robot.rs` (found in this milestone's research R2);
  improvement "reset `aim_preparing`/`shoot_countdown`/`aim_countdown` when the player enters
  the detection area (`_on_area_body_entered` → `Approach`) instead of resuming with the stale
  values left by the previous state; once fixed, the internal state can become an enum with
  per-variant counters (spec 009 FR-002's originally-sketched shape)"; motivation "`v1` quirk
  kept for parity in V2-D — the counters outlive `Idle`, so a per-state enum cannot represent
  `v1` faithfully today" (plan.md's Complexity Tracking entry, verbatim reasoning).
- [ ] T036 Correct `docs/v2-catalog.md`'s "Facts that shape the design" section: replace "gdext
  math types are pure Rust. `Vector2`, `Vector3`, `Basis`, `Quaternion`, `Transform3D` (and
  their methods: `slerp`, `looking_at`, `orthonormalized`, `transposed`, ...) live in
  `godot::builtin` and never cross the FFI" with the 1.4.1-accurate rule (the TYPES and their
  RUST-IMPLEMENTED methods are pure; a method whose body goes through `as_inner()` or is
  generated under `out/builtin_classes/**` — e.g. `Quaternion::slerp`, `Basis::looking_at` —
  calls the engine), matching `CLAUDE.md`'s existing wording (commit `cebb383`) and the
  constitution's 1.4.1 amendment. Same commit as T035 (docs-only).
- [ ] T037 Commit T035+T036: `docs: close backlog #15, #16, #17, #18 citing this milestone's
  commits; annotate #28 with the robot-laser half; add #31 (reset counters on Approach entry);
  correct docs/v2-catalog.md's stale pure-builtins claim to the 1.4.1 rule`.
- [ ] T038 Final harness run: both cases (a)/(b) on both trees (`XDG_DATA_HOME` split,
  `--fixed-fps 60`, per quickstart.md §4), diff each. Report the diffs and the documented
  divergences (SC-006 — only #15's puff-parent path).
- [ ] T039 Remove the harness from both trees: `oxide-godot/oxide-godot/
  zz_enemy_parity.{tscn,gd,gd.uid}` and the `v1` worktree's copies; `git worktree remove
  ../oxide-godot-v1`; `git worktree prune`. Verification: `git status` clean on `v2`,
  `git worktree list` shows only the main checkout.
- [ ] T040 Full gates + headless recipe one more time (`cargo build && cargo clippy && cargo
  test`; `--headless --import`; `--headless main/main.tscn --quit-after 120`; `--headless
  level/level.tscn --quit-after 120`). Report the final test count (SC-001: expect ≥ 77).
- [ ] T041 Re-grep the residual dynamic-access list: `grep -n '\.rpc(' oxide_godot_core/
  oxide_godot_lib/src/red_robot.rs oxide_godot_core/oxide_godot_lib/src/part.rs` should show
  only `play_shoot` (`red_robot.rs`) and `destroy` (`part.rs`) — both permanent, pre-existing.
  Confirm `hit` is reached exclusively through `hittable.rs`'s `HitTarget::rpc_hit()` (V2-C,
  unmodified) — not re-spelled anywhere in `red_robot.rs`/`part.rs`. Confirm by grep that
  neither module gained a new `.call(`/`.get(`/`has_method`/`has_signal`/`call_deferred`/
  `from_object_method` site. Report the (unchanged except `has_feature`'s per-frame call and the
  `"Target"` string comparison, both removed) residual list.

**Checkpoint**: Milestone V2-D complete.

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (US1) → Phase 3 (US2) → Phase 4, strictly sequential — each commit's gate
  must pass before the next commit starts, and each STOP task blocks the next phase until the
  user confirms.
- Within Phase 2: T003–T005 (pure types + predicates) before T006 (`step`, needs them) before
  T007–T010 (remaining pure functions, independent of `step`) before T011–T013 (tests) before
  T014 (gate/commit); T015–T020 (glue) after T014 (needs the compiled pure module) — T015 (new
  fields) before T016 (raycast helper, needs `self.rid`) before T017 (`physics_process`, needs
  the helper and every pure function) before T018 (`shoot`, needs the helper); T019/T020 depend
  only on T003–T010, not on T017/T018, but are sequenced after them to keep the glue file's
  edits in one pass; T021 (gate/commit) after all of T015–T020. Harness T022–T024 after T021;
  T025 (STOP) after T024.
- Within Phase 3: T026 (pure types) before T027 (tests) before T028 (fields) before T029–T031
  (glue rewrites, each independent of the others but sequenced for one edit pass) before T032
  (gate/commit). T033 (harness re-run) after T032; T034 (STOP) after T033.
- Phase 4's T035/T036 can run in parallel [P] (different files) but are committed together
  (T037); T038–T041 strictly sequential after T037 (cleanup must follow the final harness run
  that still needs the harness files present).

## Parallel Execution Notes

- T003/T004 [P]: independent type definitions in the same NEW file — safe to draft in parallel,
  though in practice one edit pass is simpler for a brand-new file.
- T026 [P]: independent of everything in Phase 2 (different file), but Phase 3 does not START
  until Phase 2's checkpoint (T025) is confirmed, per the strict phase ordering above — the `[P]`
  marker here only means "no file conflict with concurrent Phase 2 work," not "may start early."
