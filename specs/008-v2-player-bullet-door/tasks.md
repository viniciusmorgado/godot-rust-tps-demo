# Tasks: Milestone V2-C — player, bullet, door

**Input**: Design documents from `/specs/008-v2-player-bullet-door/` (spec.md, plan.md,
research.md R1–R9, data-model.md, contracts/player-bullet-door-api.md,
contracts/zz_player_parity.gd, quickstart.md)

**Branch**: `v2` — local commits only, **never** `git push`. **Baseline**: `43c4e57`.

**Tests**: requested by the spec (Principle III: pure logic MUST be `#[cfg(test)]`-covered).
Named test cases below are the MINIMUM the data-model pins; additional tests are welcome.

**Organization**: one task group per commit of `plan.md`'s Commit Plan, in order. The two user
visual checkpoints are STOP tasks — confirmed by the user, not the implementer.

## Phase 1: Setup

- [ ] T001 Add the `v1` worktree: `git worktree add ../oxide-godot-v1 v1`, then
  `cd ../oxide-godot-v1/oxide_godot_core && cargo build` and
  `cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .`.
  Verification: worktree builds clean, extension loads.
- [ ] T002 On `v2` at `43c4e57`, confirm the baseline gate: from `oxide_godot_core/`,
  `cargo build && cargo clippy && cargo test`. Verification: zero warnings, **41 tests pass**
  (spec.md Context — this is the number SC-001's "≥ 56 total" is measured against).

**Checkpoint**: baseline confirmed on both trees; US1 work can start.

---

## Phase 2: User Story 1 — `player.rs` reads one `InputFrame` (Priority: P1)

**Goal**: FR-001…FR-012. **Independent Test**: per spec.md US1.

### Commit 1 — `player/model.rs` (pure)

- [x] T003 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/player.rs`, add `mod model;`
  (matching `settings.rs`'s `mod graphics;` / `player_input.rs`'s `mod model;`). Create
  `oxide_godot_core/oxide_godot_lib/src/player/model.rs` with `PlayerTuning` (`#[derive(Clone,
  Copy, Debug)]`, 6 `f32` fields per data-model.md: `motion_interpolate_speed=10.0`,
  `rotation_interpolate_speed=10.0`, `min_airborne_time=0.1`, `jump_speed=5.0`,
  `land_threshold=0.5`, `respawn_below_y=-40.0`) and its `impl Default` with exactly these v1
  literals (`player.rs:19,20,22,23` + the two inline literals at `:198`/`:303`).
- [x] T004 [P] [US1] In the same file, add `InputFrame` (`motion: Vector2, aiming: bool,
  shooting: bool, jumping: bool, shoot_target: Vector3, camera_rotation_basis: Basis,
  camera_base_quaternion: Quaternion, aim_rotation: f64`, `#[derive(Clone, Copy, Debug)]`) and
  `AirborneOutcome` (`airborne_time: f32, on_air: bool, land: bool, jump: bool,
  jump_velocity_y: Option<f32>`, `#[derive(Clone, Copy, Debug, PartialEq)]`).
- [x] T005 [US1] Add `airborne_step(airborne_time: f32, dt: f32, is_on_floor: bool,
  jump_pressed: bool, tuning: &PlayerTuning) -> AirborneOutcome` — the LITERAL transcription of
  `player.rs:196-214` in v1's exact order (data-model.md's 4-step derivation): (1)
  `airborne_time += dt`; (2) if `is_on_floor`: `land = airborne_time > tuning.land_threshold`,
  then `airborne_time = 0.0` UNCONDITIONALLY; (3) `on_air = airborne_time >
  tuning.min_airborne_time` (recomputed AFTER the possible reset); (4) if `!on_air &&
  jump_pressed`: `jump = true`, `jump_velocity_y = Some(tuning.jump_speed)`, `on_air = true`,
  `airborne_time = tuning.min_airborne_time`. `land` and `jump` MUST be independent `bool`
  fields (NOT an enum/Option that forces mutual exclusion) — a frame can set both. Depends on
  T003, T004.
- [x] T006 [US1] In the same file, add: `lerp_motion(current: Vector2, target: Vector2, dt: f32,
  tuning: &PlayerTuning) -> Vector2` (`current.lerp(target, tuning.motion_interpolate_speed *
  dt)` — `player.rs:182-184`); `flatten_camera_axes(basis: Basis) -> (Vector3, Vector3)`
  (returns `(camera_x, camera_z)` = `(basis.col_a(), basis.col_c())`, each with `.y = 0.0` then
  `.normalized()` — `player.rs:187-193`); `slerp_toward(current: Basis, target: Quaternion, dt:
  f32, speed: f32) -> Basis` (`Basis::from_quaternion(current.get_quaternion().slerp(target, dt
  * speed))` — shared formula for `player.rs:226-231` and `:266-271`); `walk_target(camera_x:
  Vector3, camera_z: Vector3, motion: Vector2) -> Option<Basis>` (`target = camera_x*motion.x +
  camera_z*motion.y`; `Some(Basis::looking_at(target))` if `target.length() > 0.001`, else
  `None` — `player.rs:264-267`).
- [x] T007 [US1] In the same file, add: `integrate_root_motion(orientation: Transform3D,
  root_motion: Transform3D, dt: f32, gravity: Vector3, velocity_in: Vector3) -> (Transform3D,
  Vector3)` (`orientation *= root_motion`; `h = orientation.origin / dt`; `velocity.x/z =
  h.x/h.z` off `velocity_in`; `velocity += gravity * dt`; `orientation.origin = Vector3::ZERO`;
  `orientation = orientation.orthonormalized()`; return `(orientation, velocity)` —
  `player.rs:283-297`); `should_respawn(y: f32, tuning: &PlayerTuning) -> bool` (`y <
  tuning.respawn_below_y` — `player.rs:303`); `AnimPlan` enum (`JumpUp, JumpDown, Strafe {
  aim_rotation: f64, blend_position: Vector2 }, Walk { blend_position: Vector2 }`,
  `#[derive(Clone, Copy, Debug, PartialEq)]`); `anim_plan(on_air: bool, velocity_y: f32, aiming:
  bool, motion: Vector2, aim_rotation: f64) -> AnimPlan` (airborne → `JumpUp` if `velocity_y >
  0.0` else `JumpDown`; grounded+aiming → `Strafe { aim_rotation, blend_position:
  Vector2::new(motion.x, -motion.y) }`; grounded+not aiming → `Walk { blend_position:
  Vector2::new(motion.length(), 0.0) }` — `player.rs:218-234`/`:261-274`).
- [x] T008 [US1] Add `#[cfg(test)] mod tests` to `player/model.rs` covering `airborne_step`: (a)
  landing after exceeding the `0.5` s threshold → `land: true`, `airborne_time` reset to `0`;
  (b) floor contact at or under the threshold → `land: false`; (c) jumping while grounded (not
  on_air) → `jump: true`, `jump_velocity_y: Some(tuning.jump_speed)`, `airborne_time ==
  tuning.min_airborne_time`; (d) already airborne + `jump_pressed` → no second jump (`jump:
  false`); (e) **landing AND jumping in the SAME frame** (on floor with `airborne_time` past the
  threshold, AND `jumping` held) → both `land: true` AND `jump: true` on the same
  `AirborneOutcome` (spec Scenario 4 — this test MUST fail if the implementation collapses the
  two into a mutually-exclusive enum); (f) the exact accumulate→check→reset order (a case where
  reordering would change the observable `land`/`on_air` result).
- [x] T009 [US1] Add to the same `mod tests`: `lerp_motion` moves partway toward the target
  (not equal to either endpoint) for a mid-range `dt`; `flatten_camera_axes` zeroes `.y` and
  normalizes both returned vectors; `slerp_toward` moves the basis's quaternion toward (not
  equal to) the target for a mid-range `dt*speed`; `walk_target` returns `None` for a motion
  vector whose flattened length is `≤ 0.001` and `Some` (matching `Basis::looking_at`) above it.
- [x] T010 [US1] Add to the same `mod tests`: `integrate_root_motion` — horizontal velocity
  equals `root_motion.origin / dt`, gravity added, returned orientation's origin is
  `Vector3::ZERO` and its basis is orthonormal; `should_respawn` true below `-40.0`, false at or
  above; `anim_plan` covers all 4 branches with their exact payloads (`Strafe`'s
  `Vector2::new(motion.x, -motion.y)`, `Walk`'s `Vector2::new(motion.length(), 0.0)`, and both
  `JumpUp`/`JumpDown` selected correctly by the sign of `velocity_y`).
- [x] T011 [US1] Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`
  (zero warnings; all T008–T010 tests pass; ≥ 12 new tests in this file per spec SC-001).
  Commit: `player: extract PlayerTuning, InputFrame, AirborneOutcome, AnimPlan and pure
  motion/orientation/root-motion math into player/model.rs`.

### Commit 2 — `player.rs` glue + `player_input.rs` getter visibility

- [x] T012 [US1] In `oxide_godot_core/oxide_godot_lib/src/player.rs`: change
  `#[init(val = 100.0)] airborne_time: f32` to `#[init(val = 0.0)]` (backlog #10); remove the
  `crosshair: OnReady<Gd<TextureRect>>` field entirely (backlog #12, confirmed unused by grep);
  add `#[init(val = load("res://player/bullet/bullet.tscn"))] bullet_scene: Gd<PackedScene>`
  (a BARE field, no `OnReady` wrapper — research.md R4: `load()` has no tree dependency, unlike
  `OnReady`'s reason for existing); add `#[init(node =
  "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle")] shoot_particle:
  OnReady<Gd<CpuParticles3D>>` and the equivalent `muzzle_particle` for `.../MuzzleFlash`
  (same paths `player.rs:116-123` use today, resolved once instead of via `get_node_as` per
  shot).
- [x] T013 [US1] Rewrite `apply_input(&mut self, delta: f64)` per research.md R3's exact
  sequence: (1) ONE `self.player_input.bind_mut()` acquisition building an `InputFrame` from
  every field/method it needs, THEN writing `jumping = false` back through the SAME guard
  before dropping it (FR-004/SC-003 — no second acquisition anywhere else in this function);
  (2) `model::lerp_motion` → write `self.motion`; (3) `model::flatten_camera_axes` from
  `frame.camera_rotation_basis`; (4) `model::airborne_step` (engine read: `is_on_floor()`) →
  RPC `self.base_mut().rpc("land", &[])` if `land`, RPC `self.base_mut().rpc("jump", &[])` if
  `jump` — **both may fire in the same call**, do not use `else if`; if `jump_velocity_y` is
  `Some`, `set_velocity` with it immediately (matching `player.rs:207-209`'s two-write shape);
  (5) branch on `on_air`: **airborne** → `model::anim_plan` with `velocity_y` from the current
  `get_velocity().y`, apply via the shared apply-step (T014), do NOT touch `self.root_motion`;
  **grounded + `frame.aiming`** → `model::slerp_toward` (target =
  `frame.camera_base_quaternion`) → `self.orientation.basis`; `model::anim_plan` → `Strafe`;
  apply; THEN read `animation_tree.get_root_motion_rotation()`/`get_root_motion_position()` and
  reassign `self.root_motion` (v1's order: animate-then-read-root-motion, `player.rs:234-239`);
  if `frame.shooting && fire_cooldown.get_time_left() == 0.0`, spawn a bullet via
  `self.bullet_scene.instantiate_as::<CharacterBody3D>()` (same `set_global_position`/
  `look_at`/`add_collision_exception_with`/parent-`add_child_ex` sequence as
  `player.rs:246-258`) and RPC `shoot`; **grounded + not aiming** → `model::walk_target` →
  `model::slerp_toward` if `Some`; `model::anim_plan` → `Walk`; apply; THEN read root motion and
  reassign `self.root_motion` (same ordering note, `player.rs:274-279`); (6)
  `model::integrate_root_motion` (engine reads: `get_gravity()`, current `get_velocity()`) →
  new `self.orientation`, new velocity x/z; (7) `set_velocity`, `set_up_direction(Vector3::UP)`,
  `move_and_slide()`; (8) `player_model.set_global_basis(self.orientation.basis)`; (9)
  `model::should_respawn` on `get_transform().origin.y` — if true, set `transform.origin =
  initial_position` AND zero `velocity` (backlog #11, the one added write vs. `v1`).
- [x] T014 [US1] Add ONE apply-step function (e.g. `fn apply_anim(&mut self, plan: AnimPlan)`)
  that sets `self.current_animation` to the matching `Animations` variant, then writes the
  `AnimationTree` parameters per data-model.md's pinned per-variant order: `JumpUp` →
  `TRANSITION_REQUEST = "jump_up"`; `JumpDown` → `TRANSITION_REQUEST = "jump_down"`; `Strafe` →
  `TRANSITION_REQUEST = "strafe"`, THEN `AIM_ADD_AMOUNT = aim_rotation`, THEN `STRAFE_BLEND =
  blend_position`; `Walk` → `AIM_ADD_AMOUNT = 0` FIRST, THEN `TRANSITION_REQUEST = "walk"`, THEN
  `WALK_BLEND = blend_position` (note the Walk/Strafe write-order difference is v1's own,
  `player.rs:144-177`, kept verbatim). Add the 4 `AnimationTree` path-string constants
  (`TRANSITION_REQUEST`, `AIM_ADD_AMOUNT`, `STRAFE_BLEND`, `WALK_BLEND`) plus the 4
  transition-request value constants (`"jump_up"`/`"jump_down"`/`"strafe"`/`"walk"`) as named
  `&str` consts in `player.rs`. `jump`/`land`'s RPC handlers call this SAME function with the
  trivial `AnimPlan::JumpUp`/`JumpDown` variants (replacing their direct `self.animate(...)`
  calls) plus still play their own sound effect afterward.
- [x] T015 [US1] Rewrite `physics_process`'s non-authority (`else`) branch: build the
  `AnimPlan` from `self.current_animation` (replicated) — for `Strafe`, acquire
  `self.player_input.bind()` ONCE (read-only) for `get_aim_rotation()` ONLY in this case; for
  `JumpUp`/`JumpDown`/`Walk`, do not touch `player_input` at all; `self.motion` is read locally
  (already `Player`'s own replicated field). Call the T014 apply-step function with the result.
- [x] T016 [US1] In `oxide_godot_core/oxide_godot_lib/src/player_input.rs`, remove `#[func]`
  from `get_aim_rotation`, `get_camera_rotation_basis`, `get_camera_base_quaternion` (research.md
  R1 — confirmed by `grep -rn "get_aim_rotation\|get_camera_rotation_basis\|
  get_camera_base_quaternion" --include='*.gd' --include='*.tscn' .` returning zero matches;
  they stay `pub(crate)`, called only via typed `player_input.bind()...` from `player.rs`).
- [x] T017 [US1] Gates: `cargo build && cargo clippy && cargo test`. Headless:
  `/usr/bin/godot.x86_64 --headless --path oxide-godot/oxide-godot --import` then
  `--headless --path oxide-godot/oxide-godot main/main.tscn --quit-after 120` (no new errors vs.
  baseline). Grep (SC-002): `grep -n 'get_node_as\|load::<PackedScene>'
  oxide_godot_core/oxide_godot_lib/src/player.rs` → no matches. Review (SC-003): confirm by
  reading `apply_input` and the non-authority branch that `player_input.bind`/`bind_mut`
  appears at most once per code path. Commit: `player: InputFrame snapshot (one player_input
  bind per frame), OnReady/preloaded resources, AnimPlan-driven animate(); closes backlog #10,
  #11, #12`.

### Harness + checkpoint

- [ ] T018 [US1] From `contracts/zz_player_parity.gd`, write `oxide-godot/oxide-godot/
  zz_player_parity.tscn` + `.gd` (root `Node`, `--case=` cmdline dispatch, `_make_floor()`
  helper). Fill in case (a): instance `player.tscn` above the harness-built floor, scripted
  `move_*`/`aim`/`jump`/`shoot` sequence (`Input.action_press`/`release` +
  `await get_tree().process_frame`, remembering V2-B's finding that action flags need a frame
  of buffer before being observable), dump per frame `global_position`, `velocity`,
  `current_animation`, `PlayerModel`'s global basis; include a scripted teleport to `y < -40`
  partway through to exercise the respawn branch (backlog #11).
- [ ] T019 [US1] Copy the harness files into `../oxide-godot-v1/oxide-godot/`. Run case (a) on
  both trees per quickstart.md §4 (`XDG_DATA_HOME=/tmp/parity-v1` / `/tmp/parity-v2`,
  `--fixed-fps 60`), diff the two dumps EXCLUDING frames 0–1 (backlog #10) and the frame(s)
  immediately after the scripted below-`-40` teleport (backlog #11). Verification: identical
  outside those documented exclusions.
- [ ] T020 [US1] 🛑 **STOP — user visual checkpoint 1** (spec.md SC-007's US1 half). Ask the
  user to run the game and confirm: walking, strafing while aiming, jumping, landing (with its
  sound), shooting, and falling below the map then respawning with no lingering fall velocity
  and no spurious landing sound at spawn. Also ask the user to NOTE — without any code change in
  response — whether the ~10-frame hitch on the first shot of a firing sequence (backlog #28)
  changed now that `bullet_scene` is preloaded. Do not proceed to Phase 3 until confirmed.

**Checkpoint**: User Story 1 complete and independently verified.

---

## Phase 3: User Story 2 + User Story 3 — typed hit dispatch, door (Priority: P2/P3)

**Goal**: FR-013…FR-019. **Independent Test**: per spec.md US2/US3.

### Commit 3 — `hittable.rs` + `bullet.rs`

- [ ] T021 [P] [US2] Create `oxide_godot_core/oxide_godot_lib/src/hittable.rs` (new file) with
  `pub enum HitTarget { Player(Gd<crate::player::Player>), Robot(Gd<crate::red_robot::
  EnemyRobot>) }`, `pub fn resolve(node: Gd<Node3D>) -> Option<HitTarget>` (two `try_cast`s,
  order irrelevant — a node cannot satisfy both), and `impl HitTarget { pub fn rpc_hit(&mut
  self) }` calling `.rpc("hit", &[])` on whichever inner `Gd` the variant holds (reached via
  `Deref` to the shared `Node` ancestor — no need to upcast explicitly). Add `mod hittable;` to
  `oxide_godot_core/oxide_godot_lib/src/lib.rs`. No tests (glue, research.md R5 — no pure
  decision beyond what `try_cast` itself guarantees).
- [ ] T022 [US2] In `oxide_godot_core/oxide_godot_lib/src/bullet.rs`, add an inline
  `mod pure { ... }` with `#[derive(Clone, Copy, Debug, PartialEq)] pub enum BulletState {
  Flying { time_alive: f32 }, Exploded }` and `pub fn step(state: BulletState, dt: f32) ->
  (BulletState, bool)` (the `bool` is "explode now due to expiry"): decrement `time_alive`; if
  it drops below `0.0`, return `(BulletState::Exploded, true)`; else `(BulletState::Flying {
  time_alive }, false)`; `BulletState::Exploded` in → `(BulletState::Exploded, false)` (no-op),
  reproducing `bullet.rs:43-47`.
- [ ] T023 [US2] Add `#[cfg(test)] mod tests` inside `mod pure`: `time_alive` decrements by
  `dt` and stays `Flying` while `≥ 0.0`; crossing below `0.0` transitions to `Exploded` and
  returns `true`; an `Exploded` state fed back into `step` stays `Exploded` and returns `false`
  (no re-trigger).
- [ ] T024 [US2] Replace `hit: bool`/`time_alive: f32` with `state: pure::BulletState`
  (`#[init(val = pure::BulletState::Flying { time_alive: 5.0 })]`, matching
  `bullet.rs:15-16`'s literal); replace `const BULLET_VELOCITY: f32 = 20.0` with `impl Bullet {
  pub const VELOCITY: f32 = 20.0; }`. Rewrite `physics_process`: if `self.state` is
  `BulletState::Exploded` at frame start, `return` immediately (mirrors `bullet.rs:40-42`'s
  `if self.hit { return; }`); otherwise call `pure::step(self.state, dt)`, store the new state,
  and RPC `explode` if the returned `bool` is `true`; THEN, UNCONDITIONALLY (even on the expiry
  frame itself — spec US2 scenario 1), compute the displacement using `Self::VELOCITY` and call
  `move_and_collide`; on a collision, resolve the collider via `crate::hittable::resolve` and
  call `.rpc_hit()` if it resolves, disable the collision shape, and RPC `explode` ONLY IF
  `self.state` is STILL `BulletState::Flying` at this point (i.e., the expiry branch above did
  NOT already fire this frame — backlog #13/FR-014, the one new gate; everything else on a
  colliding tick — `move_and_collide`, the `hit` RPC, the collision-shape disable — happens
  exactly as in `v1` regardless); FINALLY, still inside the collision branch, set `self.state =
  pure::BulletState::Exploded` UNCONDITIONALLY — this is v1's trailing `self.hit = true`
  (`bullet.rs:57`); without it a non-expired bullet that collided would keep flying, colliding
  and RPC-ing `hit` on every later frame. `destroy` keeps its exact `#[func]`
  (`bullet.tscn:103`'s method-call track).
- [ ] T025 [US2] Gates + headless. Grep (SC-004): `grep -n has_method
  oxide_godot_core/oxide_godot_lib/src/bullet.rs` → no matches. Commit: `hittable: crate-level
  HitTarget dispatch; bullet: BulletState, no double explode (closes backlog #2, #13)`.

### Commit 4 — `door.rs`

- [ ] T026 [US3] In `oxide_godot_core/oxide_godot_lib/src/door.rs`, add an inline
  `mod pure { ... }` with `#[derive(Clone, Copy, Debug, PartialEq)] pub enum DoorState {
  Closed, Open }` and `pub fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool)`
  (the `bool` is "play the open animation now"): `(Closed, true) → (Open, true)`; every other
  combination (`(Closed, false)`, `(Open, _)`) → `(state unchanged, false)` — reproducing
  `door.rs:26-29`.
- [ ] T027 [US3] Add `#[cfg(test)] mod tests` inside `mod pure`: `Closed` + player body →
  `Open`, play `true`; `Closed` + non-player body → unchanged, play `false`; `Open` + player
  body → unchanged (`Open`), play `false` (no re-trigger).
- [ ] T028 [US3] Replace `open: bool` with `state: pure::DoorState` (`#[init(val =
  pure::DoorState::Closed)]`). Rewrite `_on_door_body_entered(&mut self, body: Gd<Node3D>)`
  KEEPING its exact `Gd<Node3D>` parameter (FR-018 — do NOT type it as `Gd<Player>`): its first
  statement is `body.try_cast::<Player>().is_ok()`, fed into `pure::on_body(self.state,
  is_player)`; if the returned `bool` is `true`, play `"doorsimple_opening"` on
  `animation_player` exactly as `door.rs:27` does; store the new state. `door.tscn`'s
  `[connection]` at line 35 is NOT edited.
- [ ] T029 [US3] Gates + headless. Commit: `door: DoorState, pure on_body decision (closes
  backlog #14 in revised form)`.

### Harness + checkpoint

- [ ] T030 [US2] Fill in the harness's case (b): instantiate `bullet.tscn` aimed at a wall a
  short known distance from the harness floor, count frames until its `AnimationPlayer`'s
  current animation becomes `"explode"`; instantiate `enemies/red_robot/red_robot.tscn`
  (confirmed by reading `red_robot.rs`'s field list that its `ready()` has no `Settings`/level
  dependency — every `OnReady` field resolves from its own scene subtree) and a bullet aimed at
  it, dump its `health` (an `#[export] i32`, directly readable) before and after; construct a
  THIRD bullet whose `time_alive` is set to cross zero on the exact physics tick it collides
  with the wall, and dump whether the explode animation's local playback position reset twice
  within that one frame (the documented `v1`-vs-`v2` divergence for backlog #13 — NOT asserted
  equal).
- [ ] T031 [US3] Fill in the harness's case (c): instantiate `door/door.tscn`, move a scripted
  `player.tscn` body into its `Area3D`, dump whether `DoorModel2/AnimationPlayer` is playing
  `"doorsimple_opening"`; on a FRESH `door.tscn` instance, move an `EnemyRobot` body through the
  same `Area3D`, dump the same check (expect: not playing) — capture stderr separately to
  confirm no new warning/error line appears on either branch.
- [ ] T032 [US2] [US3] Copy the updated harness to `../oxide-godot-v1/oxide-godot/`. Run cases
  (b) and (c) on both trees, diff — (b)'s `wall_frames` and `robot_health_before/after` must
  match exactly; the same-tick case is reported, not asserted equal; (c)'s `player_opens`/
  `robot_opens` must match exactly on both branches, and neither `.stderr` capture may show a
  new line.
- [ ] T033 🛑 **STOP — user visual checkpoint 2**. Ask the user to confirm: shooting a wall and
  a robot still explodes/damages as before; walking into the door still opens it; trying to walk
  a robot into a door does not open it. Do not proceed to Phase 4 until confirmed.

**Checkpoint**: All three user stories complete and independently verified.

---

## Phase 4: Polish

- [ ] T034 Update `docs/v2-backlog.md`: mark items **#2, #10, #11, #12, #13, #14** done, each
  citing the commit that closed it (T011 introduces the pure math but T017's commit is where
  #10/#11/#12 actually take effect in `player.rs`; #2/#13 close at T025's commit; #14 closes at
  T029's commit); annotate **#28**'s existing open row with the user's checkpoint-1 observation
  from T020 (hitch changed / unchanged / not tested) — the row stays `open`, only gains a note.
  Commit: `docs/v2-backlog.md: close #2, #10, #11, #12, #13, #14 citing this milestone's
  commits; annotate #28 with the checkpoint observation`.
- [ ] T035 Final harness run: all 3 cases (a/b/c) on both trees (`XDG_DATA_HOME` split,
  `--fixed-fps 60`, per quickstart.md §4), diff each. Report the diffs and the documented
  divergences (SC-006).
- [ ] T036 Remove the harness from both trees:
  `oxide-godot/oxide-godot/zz_player_parity.{tscn,gd,gd.uid}` and the `v1` worktree's copies;
  `git worktree remove ../oxide-godot-v1`; `git worktree prune`. Verification: `git status`
  clean on `v2`, `git worktree list` shows only the main checkout.
- [ ] T037 Full gates + headless recipe one more time (`cargo build && cargo clippy && cargo
  test`; `--headless --import`; `--headless main/main.tscn --quit-after 120`; `--headless
  level/level.tscn --quit-after 120`). Report the final test count (SC-001: expect ≥ 56).
- [ ] T038 Re-grep the residual dynamic-access list: `grep -n '\.rpc(' oxide_godot_core/
  oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/bullet.rs
  oxide_godot_core/oxide_godot_lib/src/hittable.rs` should show only `jump`/`land`/`shoot`
  (`player.rs`), `explode` (`bullet.rs`), and `hit` (`hittable.rs`) — all permanent, all
  pre-existing or explicitly authorized. Confirm by grep that none of T003–T029 introduced a
  new `.call(`/`.get(`/`has_method`/`has_signal`/`call_deferred`/`from_object_method` site in
  `player.rs`, `bullet.rs`, `door.rs`, or `hittable.rs`. Report the (unchanged except
  `has_method`'s removal) residual list.

**Checkpoint**: Milestone V2-C complete.

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (US1) → Phase 3 (US2+US3) → Phase 4, strictly sequential — each commit's
  gate must pass before the next commit starts, and each STOP task blocks the next phase until
  the user confirms.
- Within Phase 2: T003/T004 (pure types) before T005–T007 (functions that use them) before
  T008–T010 (tests) before T011 (gate/commit); T012–T016 (glue) after T011 (needs the compiled
  pure module); T017 (gate/commit) after T012–T016; T018–T020 (harness/checkpoint) after T017.
- Within Phase 3: T021 (`hittable.rs`) has no dependency on T022–T024 and could be done first or
  in parallel with them (`[P]`), but both must land together in commit 3 (T025); US3
  (T026–T029) touches only `door.rs`, entirely independent of US2's files, and could be done
  before, after, or interleaved with commit 3 — the task order above is the recommended
  sequence, not a hard dependency, except that both must land (with their own gates) before
  T030–T033.
- Within Phase 4: T034 depends on knowing every closing commit's hash (i.e., all of Phase 2/3's
  commits must exist first); T035–T038 depend on the harness still being present (T036 deletes
  it, so it must run last among T035/T036).

## Parallel Example: Phase 3

```text
# hittable.rs (T021) and bullet.rs's pure model (T022-T023) touch different files and could be
# split, but both are needed for commit 3 (T024-T025) before either can be gated/committed:
Task: "T021 hittable.rs — HitTarget, resolve, rpc_hit"
Task: "T022-T023 bullet.rs mod pure — BulletState, step, tests"
# door.rs (T026-T029) is fully independent of the above and could run in parallel with them:
Task: "T026-T029 door.rs — DoorState, on_body, tests, glue"
```

## Implementation Strategy

Sequential by design (small, single-implementer milestone): Setup → US1 (the hub, P1) →
**STOP 1** → US2+US3 (P2/P3, may interleave) → **STOP 2** → Polish. US1 is the load-bearing
story — every other module in the project either already consumes `Player` typed or will in a
future milestone — but the milestone is not shippable partway through, since US2/US3 are small
enough that splitting them into their own checkpoint would add process overhead without
reducing risk.
