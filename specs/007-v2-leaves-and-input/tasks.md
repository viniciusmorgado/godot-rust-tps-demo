# Tasks: Milestone V2-B — Leaves and player input

**Input**: Design documents from `/specs/007-v2-leaves-and-input/` (spec.md, plan.md, research.md
R1–R10, data-model.md, contracts/leaves-api.md, contracts/zz_leaves_parity.gd, quickstart.md)

**Branch**: `v2` — local commits only, **never** `git push`. **Baseline**: `3357e18`.

**Tests**: requested by the spec (Principle III: pure logic MUST be `#[cfg(test)]`-covered).
Named test cases below are the MINIMUM the data-model pins; additional tests (e.g. splitting a
"both ends" description into two `#[test]` functions) are expected and welcome.

**Organization**: one task group per commit of `plan.md`'s Commit Plan, in order. The three user
visual checkpoints are STOP tasks — confirmed by the user, not the implementer.

## Phase 1: Setup

- [x] T001 Add the `v1` worktree: `git worktree add ../oxide-godot-v1 v1`, then
  `cd ../oxide-godot-v1/oxide_godot_core && cargo build`. Verification: worktree builds clean.
- [x] T002 On `v2` at `3357e18`, confirm the baseline gate: from `oxide_godot_core/`,
  `cargo build && cargo clippy && cargo test`. Verification: zero warnings, **17 tests pass**
  (spec.md Context — this is the number SC-001's "≥ 29 total" is measured against).

**Checkpoint**: baseline confirmed on both trees; US1 work can start.

---

## Phase 2: User Story 1 — `player_input.rs` pattern exemplar (Priority: P1)

**Goal**: FR-001…FR-010. **Independent Test**: per spec.md US1.

### Commit 1 — `player_input/model.rs` (pure)

- [x] T003 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/player_input.rs`, add `mod model;`
  (private submodule declaration, matching `settings.rs`'s `mod graphics;`). Create
  `oxide_godot_core/oxide_godot_lib/src/player_input/model.rs` with `PlayerInputTuning` (10 `f32`
  fields per data-model.md: `camera_controller_speed=3.0`, `camera_mouse_speed=0.001`,
  `camera_x_rot_min=(-89.9_f32).to_radians()`, `camera_x_rot_max=70.0_f32.to_radians()`,
  `aim_speed_scale=0.5`, `aim_mouse_scale=0.75`, `aim_hold_threshold=0.4`,
  `fall_fade_start_y=-17.0`, `fall_fade_span=15.0`, `fall_fade_recover_rate=4.0`) and its
  `impl Default` with exactly these v1 literals.
- [x] T004 [P] [US1] In the same file, add `AimState { Idle, Held { seconds: f32 }, Toggled {
  seconds: f32 } }` (`#[derive(Clone, Copy, Debug, PartialEq)]`, `is_aiming(self) -> bool`),
  `CameraCue { Shoot, Far }` (same derives), and `InputSnapshot` (`motion: Vector2,
  camera_move: Vector2, aim_just_pressed: bool, aim_pressed: bool, aim_just_released: bool,
  jump_just_pressed: bool, shoot_pressed: bool`, `#[derive(Clone, Copy, Debug)]`).
- [x] T005 [US1] Add `step_aim(state: AimState, snap: &InputSnapshot, dt: f32, tuning:
  &PlayerInputTuning) -> (AimState, Option<CameraCue>)` — the LITERAL transcription of
  `player_input.rs:88-114` given verbatim in data-model.md (destructure `state` into
  `(was_aiming, toggled, seconds)`; early-release-becomes-toggle branch
  `snap.aim_just_released && seconds <= tuning.aim_hold_threshold`; else
  `toggled || snap.aim_pressed` with `toggled_next` cleared on `aim_just_pressed`; `seconds_next
  = if aim_now { seconds + dt } else { 0.0 }`; map `(aim_now, toggled_next)` to the 3 `AimState`
  variants; cue from `(was_aiming, aim_now)` transition). Depends on T003, T004.
- [x] T006 [US1] In the same file, add: `scaled_look(raw: Vector2, aiming: bool, dt: f32, tuning:
  &PlayerInputTuning) -> Vector2` (`raw * dt * tuning.camera_controller_speed`, then
  `* tuning.aim_speed_scale` if `aiming` — reproduces `player_input.rs:83-87`);
  `scaled_mouse_look(raw: Vector2, aiming: bool, tuning: &PlayerInputTuning) -> Vector2`
  (`raw * tuning.camera_mouse_speed`, then `* tuning.aim_mouse_scale` if `aiming`, no `dt` —
  reproduces `player_input.rs:172-176`); `clamp_pitch(current: f32, delta_y: f32, tuning:
  &PlayerInputTuning) -> f32` (`(current + delta_y).clamp(tuning.camera_x_rot_min,
  tuning.camera_x_rot_max)` — reproduces `rotate_camera`'s pitch line, `player_input.rs:230-231`);
  `alpha_for_height(y: f32, prev_alpha: f32, dt: f32, tuning: &PlayerInputTuning) -> f32`
  (`y < tuning.fall_fade_start_y` → `((tuning.fall_fade_start_y - y) /
  tuning.fall_fade_span).min(1.0)`; else → `prev_alpha * (1.0 - tuning.fall_fade_recover_rate *
  dt)`, **no `.max(0.0)`**, byte-identical to `player_input.rs:161-166`); `aim_rotation(
  camera_x_rot: f32, tuning: &PlayerInputTuning) -> f64` (clamp then the up/down branch from
  `player_input.rs:184-199`: `camera_x_rot >= 0.0` → `(-camera_x_rot /
  tuning.camera_x_rot_max) as f64`, else `(camera_x_rot / tuning.camera_x_rot_min) as f64`).
- [x] T007 [US1] Add `#[cfg(test)] mod tests` to `player_input/model.rs` covering `step_aim`
  (data-model.md "Consequences to pin"): (a) press-and-hold past 0.4s then release → `Idle`,
  cue `Some(Far)`; (b) press-and-release within ≤0.4s (a tap) → `Toggled`, `is_aiming() == true`;
  (c) press while `Toggled` → `Held` (no cue), then release with accumulated `seconds > 0.4` →
  `Idle`, cue `Some(Far)`; (d) a double tap where the SECOND release's accumulated seconds is
  still `≤ 0.4` → re-toggles (state stays/returns to `Toggled`, no `Idle` in between observable
  from the outside); (e) `just_pressed && just_released` in the SAME frame from `Idle` →
  `Toggled` (v1's release-check runs first, with `seconds == 0.0`, satisfying `<=
  aim_hold_threshold`); (f) `Toggled { seconds }` with no input this frame →
  `Toggled { seconds: seconds + dt }` (timer keeps counting); (g) `Idle` + `aim_just_pressed` →
  `Held { seconds: dt }`, cue `Some(Shoot)`.
- [x] T008 [US1] Add to the same `#[cfg(test)] mod tests`: `scaled_look` scaled by
  `aim_speed_scale` (0.5) when `aiming == true`, unscaled when `false`; `scaled_mouse_look`
  scaled by `aim_mouse_scale` (0.75) when aiming; `clamp_pitch` clamps at both
  `camera_x_rot_min` and `camera_x_rot_max` (2 tests or 1 parameterized); `alpha_for_height`
  reaches `1.0` at `y = -32.0` (`fall_fade_start_y - fall_fade_span`) and decays by the recover
  factor when `y >= -17.0`; `aim_rotation` covers both the aim-up (`camera_x_rot >= 0`) and
  aim-down (`camera_x_rot < 0`) branches.
- [x] T009 [US1] Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`
  (zero warnings; all T007/T008 tests pass; total test count increases by the number of new
  tests). Commit: `player_input: extract AimState and pure rotation/aim/fade math into
  player_input/model.rs`.

### Commit 2 — `player_input.rs` glue + `player.rs:138`

- [x] T010 [US1] In `oxide_godot_core/oxide_godot_lib/src/player_input.rs`, change the 6 node
  fields from `Option<Gd<T>>` to `#[export] field: OnEditor<Gd<T>>`, same names/types otherwise
  (`camera_animation: OnEditor<Gd<AnimationPlayer>>`, `crosshair: OnEditor<Gd<TextureRect>>`,
  `camera_base: OnEditor<Gd<Node3D>>`, `camera_rot: OnEditor<Gd<Node3D>>`, `camera_camera:
  OnEditor<Gd<Camera3D>>`, `color_rect: OnEditor<Gd<ColorRect>>`). Remove `toggled_aim: bool` and
  `aiming_timer: f32`; add `aim_state: AimState` (private, no `#[export]`) and a private
  `prev_alpha: f32` field if the fade needs one carried across frames beyond what `color_rect`'s
  own `modulate.a` already holds (read it back from `color_rect.get_modulate().a` instead of a
  redundant field, if that is simpler — implementer's call, no behavior difference).
- [x] T011 [US1] Add `parent: OnReady<Gd<CharacterBody3D>>` and `parent_rid: OnReady<Rid>`, both
  via `#[init(val = OnReady::from_base_fn(|base| ...))]`: `parent` casts `base.get_parent()
  .unwrap()` to `CharacterBody3D` (confirmed valid: `InputSynchronizer`'s parent in
  `player.tscn:339` is the `Player` node, registered `#[class(base=CharacterBody3D)]` at
  `player.rs:26`); `parent_rid` reads `.get_rid()` off that same cast handle. This replaces the
  two per-frame `self.base().get_parent().unwrap().cast::<Node3D>()` sites at
  `player_input.rs:135-137` and `:155-157` (SC-002).
- [x] T012 [US1] Remove `#[export]` from `jumping` (keep the field, name, type `bool`, and
  `pub(crate)` visibility — FR-002); leave `aiming`, `shoot_target`, `motion`, `shooting`
  exactly as `#[export] pub(crate)` (FR-001, `SceneReplicationConfig` unedited).
- [x] T013 [US1] Rewrite `process(&mut self, delta: f64)`: build one `InputSnapshot` from
  `Input::singleton()` (8 strength reads + `is_action_just_pressed/is_action_pressed/
  is_action_just_released("aim")`, `is_action_just_pressed("jump")`, `is_action_pressed
  ("shoot")`); write `self.motion` from the snapshot; call `model::scaled_look` then
  `model::clamp_pitch` (via the existing `rotate_camera` engine-write helper, updated to take
  the pre-scaled `Vector2` instead of doing the scaling itself) for camera rotation; call
  `model::step_aim(self.aim_state, &snapshot, delta as f32, &tuning)`, store the returned
  `AimState`, write `self.aiming = new_state.is_aiming()` (every frame, per research.md R4), and
  on `Some(cue)` play `"shoot"`/`"far"` on `camera_animation` exactly as `player_input.rs:110`/
  `:112` do today; on `jump_just_pressed`, `self.base_mut().rpc("jump", &[])` unchanged
  (FR-010); set `self.shooting` from the snapshot and, if shooting, raycast with
  `.exclude(&array![*self.parent_rid])` (FR-009, replacing `array![Rid::Invalid]` at
  `player_input.rs:130`) using `self.parent.get_world_3d().unwrap().get_direct_space_state()
  .unwrap()` (no per-frame `get_parent().cast()` — SC-002); compute the fade via
  `model::alpha_for_height(self.parent.get_global_transform().origin.y, prev_alpha, delta as
  f32, &tuning)` and write it to `color_rect`'s modulate alpha, same as
  `player_input.rs:150-167`.
- [x] T014 [US1] Rewrite `input(&mut self, input_event: Gd<InputEvent>)`: on
  `InputEventMouseMotion`, call `model::scaled_mouse_look(mouse_motion.get_screen_relative(),
  self.aiming, &tuning)` then apply through the same rotation-write path `process` uses
  (`rotate_camera`, taking the pre-scaled delta) — same observable result as
  `player_input.rs:170-178`.
- [x] T015 [US1] `get_aim_rotation(&self) -> f64` becomes a thin wrapper: read
  `self.camera_rot.get_rotation().x` once, call `model::aim_rotation(x, &tuning)`. Keep its
  `#[func] pub(crate)` attribute and exact name (`player.rs` calls it) — FR-006/data-model
  cross-reference table.
- [x] T016 [US1] In `oxide_godot_core/oxide_godot_lib/src/player.rs`, at the line reading
  `self.player_input.bind().camera_camera.clone().unwrap()` (currently line 138), remove
  `.unwrap()` — `OnEditor<Gd<Camera3D>>` derefs to `Gd<Camera3D>`, which is `Clone`. This is the
  ONLY edit to `player.rs` in this milestone (FR-003, data-model cross-reference table).
- [x] T017 [US1] Gates: `cargo build && cargo clippy && cargo test`. Headless:
  `/usr/bin/godot.x86_64 --headless --path oxide-godot/oxide-godot --import` then
  `--headless --path oxide-godot/oxide-godot main.tscn` (auto-hosts and spawns a player — no new
  errors vs. the documented baseline). Grep (SC-002):
  `grep -n 'get_parent()\|\.unwrap()' oxide_godot_core/oxide_godot_lib/src/player_input.rs` and
  confirm no match inside `process`/`input`/`ready` (only inside `OnReady::from_base_fn`'s
  one-time closure, which is not a per-frame site). Commit: `player_input: OnEditor node refs,
  OnReady parent handle, snapshot/step/apply frame shape; player.rs:138 camera_camera unwrap
  removal`.

### Harness + checkpoint

- [x] T018 [US1] From `contracts/zz_leaves_parity.gd`, write `oxide-godot/oxide-godot/
  zz_leaves_parity.tscn` + `.gd` (root `Node`, `--case=` cmdline dispatch). Fill in case (a) with
  the corrected press/release sequence (hold-then-late-release → `Idle`; tap → `Toggled`; press
  while toggled → `Held`; release with accumulated > 0.4s → `Idle`; jump; shoot with the target
  NOT behind the player's own collider — Edge Cases' one excluded scenario), plus the
  fall-to-black window (teleport below `y = -32` and dump alpha across the ramp, teleport back
  above `-17` and dump the decay).
- [x] T019 [US1] Copy the harness files into `../oxide-godot-v1/oxide-godot/`. Run case (a) on
  both trees per quickstart.md §4 (`XDG_DATA_HOME=/tmp/parity-v1` / `/tmp/parity-v2`), `diff` the
  two dumps. Verification: identical (the self-hit raycast scenario is avoided by construction,
  per Edge Cases).
- [x] T020 [US1] 🛑 **STOP — user visual checkpoint 1** (spec.md SC-007's US1 half). Ask the user
  to run the game and confirm: move, aim by holding, aim by tapping (toggle), jump, shoot at a
  target and see the crosshair target track correctly, fall off the map and watch the screen
  fade to black and back. Do not proceed to Phase 3 until confirmed.

**Checkpoint**: User Story 1 complete and independently verified.

---

## Phase 3: User Story 2 + User Story 3 — camera shake and debug label (Priority: P2)

**Goal**: FR-011…FR-018. **Independent Test**: per spec.md US2/US3.

### Commit 3 — `camera_noise_shake.rs`

- [x] T021 [P] [US2] Add `mod model;` to `oxide_godot_core/oxide_godot_lib/src/
  camera_noise_shake.rs`. Create `camera_noise_shake/model.rs` with `CameraShakeTuning`
  (`speed: f32 = 1.0`, `decay_rate: f32 = 1.5`, `max_yaw: f32 = 0.05`, `max_pitch: f32 = 0.05`,
  `max_roll: f32 = 0.1`, `max_trauma: f32 = 1.2`) and its `impl Default`.
- [x] T022 [US2] In the same file, add: `decay(trauma: f32, dt: f32, tuning:
  &CameraShakeTuning) -> f32` (`(trauma - tuning.decay_rate * dt).max(0.0)` — reproduces
  `camera_noise_shake.rs:59-62`); `advance_time(time: f64, dt: f64, tuning:
  &CameraShakeTuning) -> f64` (`time + dt * tuning.speed as f64 * 5000.0` — reproduces the exact
  magic-number line `camera_noise_shake.rs:67`, comment included: "pleasing effect at SPEED
  1.0"); `shake(trauma: f32) -> f32` (`trauma * trauma`); `offsets(shake: f32, samples: [f32; 3],
  tuning: &CameraShakeTuning) -> Vector3` (`samples[0]` = noise at `seed`, `[1]` = `seed+1`,
  `[2]` = `seed+2`; returns `Vector3 { x: tuning.max_pitch * shake * samples[1], y:
  tuning.max_yaw * shake * samples[0], z: tuning.max_roll * shake * samples[2] }` — v1's exact
  non-alphabetical mapping, `camera_noise_shake.rs:69-74`); `add_trauma(current: f32, amount:
  f32, tuning: &CameraShakeTuning) -> f32` (`(current + amount).min(tuning.max_trauma)`).
- [x] T023 [US2] Add `#[cfg(test)] mod tests`: `decay` floors at `0.0` and subtracts
  `decay_rate * dt` otherwise; `advance_time` reproduces the `× 5000.0` formula for a known
  `(time, dt, speed)` triple; `shake` returns `trauma²`; `offsets` with 3 DISTINCT sample values
  (e.g. `[0.1, 0.2, 0.3]`) asserts `x` uses `samples[1]`, `y` uses `samples[0]`, `z` uses
  `samples[2]` — so a swapped axis fails (spec US2 scenario 4); `add_trauma` clamps at
  `max_trauma` when `current + amount` would exceed it, and adds normally otherwise.
- [x] T024 [US2] In `camera_noise_shake.rs`, replace the single `noise: Gd<FastNoiseLite>` field
  with three: `noise_yaw`, `noise_pitch`, `noise_roll` (`#[init(val = FastNoiseLite::new_gd())]`
  each). In `ready()`, seed each exactly once — `noise_yaw.set_seed(self.noise_seed)`,
  `noise_pitch.set_seed(self.noise_seed.wrapping_add(1))`,
  `noise_roll.set_seed(self.noise_seed.wrapping_add(2))` — and set `fractal_octaves(1)` /
  `fractal_lacunarity(1.0)` on all three (replacing the single re-seeded instance and its 3
  per-frame `set_seed` calls in `get_noise_value`, `camera_noise_shake.rs:79-82`). Rewrite
  `process`/`apply_shake` to preserve v1's exact ordering (`camera_noise_shake.rs:40-45`: check
  `self.trauma > 0.0` BEFORE decaying; decay; THEN compute shake from the just-decayed trauma):
  `if self.trauma > 0.0 { self.trauma = model::decay(self.trauma, delta as f32, &tuning);
  self.time = model::advance_time(self.time, delta, &tuning); let shake =
  model::shake(self.trauma); let samples = [self.noise_yaw.get_noise_1d(self.time as f32),
  self.noise_pitch.get_noise_1d(self.time as f32), self.noise_roll.get_noise_1d(self.time as
  f32)]; let offset = model::offsets(shake, samples, &tuning); self.base_mut().set_rotation(
  self.start_rotation + offset); }`. `add_trauma(&mut self, amount: f64)` loses `#[func]`
  (confirmed unused by name — FR-013), becomes a plain `pub(crate) fn` in a non-`#[godot_api]`
  `impl CameraNoiseShake` block, calling `model::add_trauma`.
- [x] T025 [US2] Gates + headless (no new errors). Commit: `camera_noise_shake: extract
  CameraShakeTuning/decay/shake/offsets into camera_noise_shake/model.rs; 3 pre-seeded
  FastNoiseLite instances`.

### Commit 4 — `debug_label.rs`

- [x] T026 [P] [US3] In `oxide_godot_core/oxide_godot_lib/src/debug_label.rs`, add an inline
  `mod pure { ... }` (Principle III's distinct-`mod` requirement, no separate file per research.md
  R8) with `DebugStats { fps: f64, vsync_enabled: bool, ram_bytes: u64, vram_bytes: u64,
  multiplayer_id: Option<i64> }`, `compose(stats: &DebugStats) -> String`, and
  `format_godot_float(v: f64) -> String` (`let s = format!("{v}"); if s.contains('.') { s } else
  { format!("{s}.0") }`).
- [x] T027 [US3] `compose` builds, in order: `"FPS: " + format_godot_float(stats.fps)`;
  `"\nVSync: " + "Enabled"/"Disabled"`; `"\nMemory: " + format!("{:3.2}", stats.ram_bytes as f64
  / 1_048_576.0) + " MiB"`; **NEW** `"\nVRAM: " + format!("{:3.2}", stats.vram_bytes as f64 /
  1_048_576.0) + " MiB"` (FR-016, backlog #26, placed right below Memory); `"\nOnline: " +
  "Yes"/"No"` (`stats.multiplayer_id.is_some()`); if `Some(id)`, `"\nMultiplayer ID: " +
  id.to_string()`. Add `#[cfg(test)] mod tests` inside `mod pure`: whole-number FPS
  (`fps: 60.0`) composes `"FPS: 60.0"` (not `"FPS: 60"`); the VRAM line appears immediately after
  Memory and before Online; `multiplayer_id: None` composes `"Online: No"` with no ID line;
  `multiplayer_id: Some(id)` composes `"Online: Yes\nMultiplayer ID: {id}"`.
- [x] T028 [US3] Rewrite `process(&mut self, _delta: f64)`: check `is_action_just_pressed
  ("toggle_debug")` and flip visibility exactly as `debug_label.rs:14-17` do today; THEN, only if
  `self.base().is_visible()` (backlog #4 — currently text is rebuilt unconditionally every
  frame), build one `DebugStats` (`Engine::singleton().get_frames_per_second()`,
  `DisplayServer::singleton().window_get_vsync_mode() != VSyncMode::DISABLED`,
  `Os::singleton().get_static_memory_usage()`, `RenderingServer::singleton()
  .get_rendering_info(RenderingInfo::VIDEO_MEM_USED)`, and the existing
  `OfflineMultiplayerPeer`-check → `multiplayer_id: Option<i64>` projection from
  `debug_label.rs:36-46`), call `pure::compose`, and `set_text` — on the SAME frame visibility
  turns on (the toggle check and the visible-branch build are sequential in one `process` call,
  not split across frames).
- [x] T029 [US3] Gates + headless. Grep (SC-004): confirm `compose`'s VRAM line exists (code
  review, since headless can't visually confirm text). Commit: `debug_label: DebugStats/compose
  pure mod, VRAM line, hidden-frame skip (closes backlog #4, #26)`.

### Harness + checkpoint

- [x] T030 [US2] Fill in the harness's case (b): `seed(12345)` before instancing `player.tscn`
  (so `CameraNoiseShake::noise_seed`'s `randi()` at `ready` matches on both trees), then
  `player.rpc("add_camera_shake_trauma", 0.75)` (the existing `Player::add_camera_shake_trauma`
  `#[rpc]`, unchanged by this milestone — research.md R9), dump the camera's `.rotation` for 30
  `process_frame`s.
- [x] T031 [US3] Fill in the harness's case (c): instantiate the scene that hosts `DebugLabel`
  (`level.tscn` or `main_scene.tscn` — whichever the running project actually attaches it to;
  confirm via grep for `type="DebugLabel"`), force `set_visible(true)` bypassing the toggle
  action, wait one `process_frame`, dump `.text`.
- [x] T032 [US2] [US3] Copy the updated harness to `../oxide-godot-v1/oxide-godot/`. Run cases
  (b) and (c) on both trees, diff — (b) expects identical rotation dumps (bounded by the
  FastNoiseLite bit-identical assumption, Edge Cases); (c) expects identical text with the
  `VRAM:` line stripped from both sides before diffing (present on `v2` only, by design —
  SC-004, not a parity requirement).
- [x] T033 🛑 **STOP — user visual checkpoint 2**. Ask the user to confirm: shooting/getting hit
  produces camera shake as before; the debug overlay (its toggle key) shows FPS / VSync /
  Memory / VRAM / Online (/ Multiplayer ID when online) and stops updating while hidden,
  refreshing immediately on the frame it's shown again. Do not proceed to Phase 4 until
  confirmed.

**Checkpoint**: User Stories 1, 2 and 3 complete and independently verified.

---

## Phase 4: User Story 4 — the one async timer pattern (Priority: P3)

**Goal**: FR-019…FR-022. **Independent Test**: per spec.md US4.

### Commit 5 — `part_disappear.rs` + `blast.rs` + `CLAUDE.md`

- [x] T034 [P] [US4] Rewrite `oxide_godot_core/oxide_godot_lib/src/part_disappear.rs`'s `ready()`:
  replace the two nested `connect_other` closures (`part_disappear.rs:16-32`) with one `async`
  block spawned via `godot::task::spawn`, capturing `Gd<Self>` (not `&mut self`) at spawn time:
  `self.mini_blasts.set_emitting(true)` (unchanged, before the spawn); inside the task, `await
  create_timer(0.2).signals().timeout().to_future::<()>()` (a `SceneTreeTimer`'s emitter is kept
  alive by the tree — plain `to_future` is correct here per research.md R7), check
  `this.is_instance_valid()` and return early if false; `bind_mut()` and call
  `set_emitting(true)` + read `get_lifetime()`; `await create_timer(lifetime *
  2.0).signals().timeout().to_future::<()>()`, check validity again, then `bind_mut()` and
  `queue_free()`. Same two delays, same two engine actions, as `part_disappear.rs:14-32` today.
- [x] T035 [P] [US4] Rewrite `oxide_godot_core/oxide_godot_lib/src/blast.rs`'s `ready()`: replace
  the `connect_other` on `animation_finished` (`blast.rs:21-26`) with one `async` block spawned
  via `godot::task::spawn`, capturing `Gd<Self>`; `await
  animation_player.signals().animation_finished().to_fallible_future::<StringName>()` (the child
  `AnimationPlayer` dies with `Blast` — its emitter is NOT guaranteed to outlive the wait, so the
  FALLIBLE variant is required per research.md R7/FR-020, treating `Err` the same as an early
  return); on `Ok(_)`, check `this.is_instance_valid()` and `bind_mut().queue_free()`. Leave
  `self.camera` and `process`'s per-frame `look_at` exactly as they are (already resolved once —
  FR-022, no change needed).
- [x] T036 [US4] Add the paragraph from research.md R7 verbatim to `CLAUDE.md`'s "Port
  conventions (v2)" section, as a new bullet after "Parity harness" (FR-021): the `Gd<Self>`
  capture rule, the `is_instance_valid()` guard after every `.await`, and the
  `to_future`/`to_fallible_future` choice criterion (emitter outlives the wait vs. not).
- [x] T037 [US4] Gates + headless (`level.tscn`; a robot's death and the laser impact aren't
  scriptable from a headless run, so this only confirms no new errors on scene load — the
  harness in T038/T039 covers timing). Grep (SC-003): `grep -rn connect_other
  oxide_godot_core/oxide_godot_lib/src/part_disappear.rs
  oxide_godot_core/oxide_godot_lib/src/blast.rs` returns no matches. Commit: `part_disappear +
  blast: async task pattern replaces nested connect_other (closes backlog #5); document the
  pattern in CLAUDE.md`.

### Harness + checkpoint

- [x] T038 [US4] Fill in the harness's case (d): instantiate `part_disappear.tscn` directly,
  count `process_frame` awaits until `is_instance_valid()` on the captured handle is `false`;
  same for `impact_effect.tscn` (the scene hosting `Blast`). Dump both frame counts.
- [x] T039 [US4] Copy the updated harness to `../oxide-godot-v1/oxide-godot/`. Run case (d) on
  both trees, diff. Verification: identical frame counts (the async rewrite must not change
  timing).
- [x] T040 🛑 **STOP — user visual checkpoint 3**. Ask the user to confirm: killing a robot still
  makes its parts puff and vanish on the same schedule; a laser impact's blast still plays and
  vanishes when its animation finishes. Do not proceed to Phase 5 until confirmed.

**Checkpoint**: All four user stories complete and independently verified.

---

## Phase 5: Polish

- [x] T041 Update `docs/v2-backlog.md`: mark items **#4, #5, #7, #8, #9, #26** done, each citing
  the commit that closed it (T009/T017 for #8/#9, T017 for #7, T025/T029 for nothing new — #26 is
  T029, #4 is T029, #5 is T037); leave **#6** open with its existing deferral reason (unchanged —
  this milestone does not touch it). Commit: `docs/v2-backlog.md: close #4, #5, #7, #8, #9, #26
  citing this milestone's commits; #6 stays open`.
- [x] T042 Final harness run: all 4 cases (a/b/c/d) on both trees (`XDG_DATA_HOME` split per
  quickstart.md §4), diff each. Report the diffs (SC-006).
- [x] T043 Remove the harness from both trees:
  `oxide-godot/oxide-godot/zz_leaves_parity.{tscn,gd,gd.uid}` and the `v1` worktree's copies;
  `git worktree remove ../oxide-godot-v1`; `git worktree prune`. Verification: `git status` clean
  on `v2`, `git worktree list` shows only the main checkout.
- [x] T044 Full gates + headless recipe one more time (`cargo build && cargo clippy && cargo
  test`; `--headless --import`; `--headless main.tscn`/`level.tscn`). Report the final test count
  (SC-001: expect ≥ 29, i.e. 17 + at least 12 new).
- [x] T045 Re-grep the residual dynamic-access list: `grep -n '\.rpc(' oxide_godot_core/
  oxide_godot_lib/src/player_input.rs` should show only `jump` (permanent, FR-010); confirm by
  grep that none of T003–T037 introduced a new `.call(`/`.get(`/`has_method`/`has_signal`/
  `call_deferred`/`from_object_method` site anywhere in the 5 touched modules or `player.rs`.
  Report the (unchanged) residual list.

**Checkpoint**: Milestone V2-B complete.

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (US1) → Phase 3 (US2+US3) → Phase 4 (US4) → Phase 5, strictly sequential —
  each commit's gate must pass before the next commit starts, and each STOP task blocks the next
  phase until the user confirms.
- Within Phase 2: T003/T004 (pure types) before T005/T006 (functions that use them) before
  T007/T008 (tests) before T009 (gate/commit); T010–T016 (glue) after T009 (needs the compiled
  pure module); T017 (gate/commit) after T010–T016; T018–T020 (harness/checkpoint) after T017.
- Within Phase 3: US2 (T021–T025) and US3 (T026–T029) touch disjoint files
  (`camera_noise_shake.rs`/`camera_noise_shake/model.rs` vs. `debug_label.rs`) and have no
  functional dependency on each other — they may be done in either order, but both must land
  (with their own gate+commit) before T030–T033.
- Within Phase 4: T034 (`part_disappear.rs`) and T035 (`blast.rs`) are independent files, `[P]`;
  both land before T036 (`CLAUDE.md`, depends on the pattern existing in both files to describe
  it accurately) and T037 (gate/commit covers all three files).

## Parallel Example: Phase 3

```text
# US2 and US3 have no file overlap and no functional dependency — a team of two could run:
Task: "T021-T025 camera_noise_shake/model.rs + camera_noise_shake.rs (US2)"
Task: "T026-T029 debug_label.rs (US3)"
# then both converge on T030-T033 (harness cases b+c, checkpoint 2).
```

## Implementation Strategy

Sequential by design (this is a small, single-implementer milestone, not a team split): Setup →
US1 (the pattern exemplar, P1) → **STOP 1** → US2+US3 (P2, may interleave) → **STOP 2** → US4
(P3) → **STOP 3** → Polish. US1 is the MVP in the sense that it fixes the snapshot→step→apply
shape every other story (and V2-C/D) copies; the milestone is not considered "shippable" partway
through, since all 4 stories are small, already-scoped leaves with no reason to ship separately.
