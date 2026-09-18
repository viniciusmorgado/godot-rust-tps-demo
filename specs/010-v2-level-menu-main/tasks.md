---

description: "Task list for Milestone V2-E — level, menu, main: the scene manager and the end of v2"
---

# Tasks: Milestone V2-E — level, menu, main: the scene manager and the end of v2

**Input**: Design documents from `/specs/010-v2-level-menu-main/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R9), data-model.md, contracts/, quickstart.md
**Branch**: `v2` (work directly, no feature branch). Local commits only — **NEVER `git push`**.
**Baseline**: `dab4470`.

**Tests**: pure-module unit tests are REQUIRED by this milestone's own success criteria
(SC-001, ≥ 20 new, total ≥ 112) — included below, one `#[cfg(test)] mod tests` task per pure
module/row-group.

**Organization**: one phase per user story (US1 `main_scene.rs`, US2 `level.rs` +
`flying_forklift.rs`, US3 `menu.rs`), each ending in its own commit(s) + gates + headless
validation, per plan.md's 5-commit Commit Plan. 🛑 STOP tasks are confirmed by the user only —
do not proceed past one without their explicit go-ahead.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an incomplete task in this list
- **[Story]**: US1 / US2 / US3 — omitted for Setup and Polish tasks
- All paths below are relative to the repo root (`/home/morgado/Experimento/oxide-godot`)

---

## Phase 1: Setup

- [X] T001 Add the `v1` worktree and build it: `git worktree add ../oxide-godot-v1 v1`; `cd ../oxide-godot-v1/oxide_godot_core && cargo build`; `cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .` (per `CLAUDE.md`'s parity-harness convention and quickstart.md §3)
- [X] T002 Confirm baseline gates on `v2` from `oxide_godot_core/`: `cargo build && cargo clippy -- -D warnings && cargo test` — expect clean, 92 tests passing (spec.md's stated baseline)
- [X] T003 [P] Recreate the V2-A settings harness for reuse as US3's harness case (a): copy `specs/006-v2-typed-settings/contracts/zz_settings_parity.gd` to `oxide-godot/oxide-godot/zz_settings_parity.gd` and to `../oxide-godot-v1/oxide-godot/zz_settings_parity.gd` (identical on both trees, per quickstart.md §4's note that it is reused UNCHANGED, not modified); create the matching `zz_settings_parity.tscn` root scene on both trees if not already present in the contract

**Checkpoint**: worktree ready, baseline gates green, US3's harness pre-staged.

---

## Phase 2: User Story 1 - `main_scene.rs`: the scene manager (Priority: P1) 🎯 Commit 1

**Goal**: `Main` reaches `Menu`/`Level` through typed `try_cast` + `connect_other` instead of
`has_signal` + `Callable::from_object_method`; both by-name deferred calls in the crate become
`Callable::from_fn(...).call_deferred(&[])`.

**Independent Test**: headless `main/main.tscn` boots to the menu with no new errors; in the
running game, Play → level loads; Esc in the level → back to the menu; Settings' Apply button
still returns to the menu.

### Implementation for User Story 1

- [X] T004 [US1] In `oxide_godot_core/oxide_godot_lib/src/main_scene.rs`, rewrite `change_scene_to_packed(&mut self, resource: Gd<PackedScene>)` per research.md R2's exact body: after `instantiate()` and clearing existing children, `try_cast::<Level>` the new node and, on success, connect `level.signals().quit().connect_other(&this, |main: &mut Main| main.go_to_main_menu())`; else `try_cast::<Menu>` and connect `menu.signals().replace_main_scene().connect_other(&this, |main: &mut Main, scene: Gd<PackedScene>| main.replace_main_scene(scene))`; `add_child(&node)` after the cast/connect (order matters — connect before the node enters the tree, matching R2's sketch); remove `has_signal`/`Callable::from_object_method` entirely (FR-001)
- [X] T005 [US1] In the same file, replace `replace_main_scene`'s `self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()])` with `Callable::from_fn("change_scene_to_packed", move |_args| { this.bind_mut().change_scene_to_packed(resource.clone()); Variant::nil() }).call_deferred(&[])` per research.md R1's exact closure shape (capturing `Gd<Main>` + the resource by move) — same deferred (end-of-current-frame) timing, no `Gd<Main>`/`Gd<PackedScene>` state kept beyond the closure (FR-002)
- [X] T006 [US1] Drop `#[func]` from `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed`; make all three plain `pub(crate) fn` (FR-003) — confirmed by spec.md's Context grep that nothing calls any of the three by name after T004/T005 land
- [X] T007 [US1] Gates (`cargo build && cargo clippy -- -D warnings && cargo test`) + headless `main/main.tscn --quit-after 120`, `menu/menu.tscn --quit-after 120`, `level/level.tscn --quit-after 120` per quickstart.md §2 — zero new errors relative to `CLAUDE.md`'s documented baseline; commit with message `main_scene: typed try_cast + connect_other scene dispatch, Callable::call_deferred instead of call_deferred("name"); closes backlog #3, #23, #24` (research.md R9); stage only `main_scene.rs`

**Checkpoint**: `Main`'s scene manager is fully typed; no user-visible STOP here — the visual
checkpoint for US1 lands jointly with US2 after commit 3 (below).

---

## Phase 3: User Story 2 - `level.rs` + `flying_forklift.rs`: spawning, GI setup, the respawn hitch (Priority: P1) 🎯 Commits 2–3

**Goal**: preloaded spawn scenes, `OnReady` GI nodes, one pure `gi_plan` replacing three
near-identical `setup_*` functions, the 15 s robot respawn on the established async pattern, a
pure `pick_spawn`/`pick_model`, both redundant `randomize()` calls removed, uniform
`add_child_ex(...).force_readable_name(true)`.

**Independent Test**: `cargo test` passes for `gi_plan`/`pick_spawn`/`pick_model`; headless
`level/level.tscn` and the auto-hosted `main/main.tscn` show no new errors; in the running
game, a level boots with the correct GI technique, robots/players spawn at their points,
killing a robot brings a fresh one back ~15 s later, forklifts show one of three models.

### Implementation for User Story 2 — commit 2: `level.rs`

- [X] T008 [US2] Create `oxide_godot_core/oxide_godot_lib/src/level/model.rs` with the `GiPlan` struct (`sdfgi_enabled: bool`, `voxel_visible: bool`, `probes_visible: bool`, `free_lightmap: bool`, `create_lightmap: bool`, `lightmap_visible: Option<bool>`, `sdfgi_rays: Option<EnvironmentSdfgiRayCount>`, `voxel_quality: Option<VoxelGiQuality>`) and `pub fn gi_plan(gi_type: GiType, gi_quality: GiQuality, has_lightmap: bool) -> GiPlan`, transcribing research.md R3's 9-cell table exactly (all 3 `Sdfgi`/`VoxelGi`/`LightmapGi` × `High`/`Low`/`Disabled` combinations), including the `create_lightmap = !has_lightmap` gating on the `LightmapGi` rows and the `lightmap_visible: Some(false)` ONLY on the `LightmapGi`×`Disabled` row (`None` elsewhere — the asymmetry Acceptance Scenario 2b pins)
- [X] T009 [US2] In the same file, add `pub fn pick_spawn(r: i64, count: i64) -> i64 { r % count }` (research.md R4 — the literal transcription of `level.rs:199`'s `randi() % count as i64`; `randi()` is never negative, so `%` and `rem_euclid` coincide, and `%` is the v1 text).
- [X] T010 [US2] In the same file, add `#[cfg(test)] mod tests`: one test per `(gi_type, gi_quality)` cell with `has_lightmap = false` (`ready()`'s actual single-call path, 9 tests), at least 2 tests with `has_lightmap = true` on `LightmapGi` rows covering the `create_lightmap` gating (Edge Cases — correctness/testability completion, not an observed-in-game path), plus `pick_spawn` tests at `r = 0`, `r = count - 1`, `r = count` (wraps to 0) — ≥ 14 tests total
- [X] T011 [US2] In `oxide_godot_core/oxide_godot_lib/src/level.rs` (NOT `lib.rs`), add `mod model;` at the top so the file resolves to `level/model.rs` — the same shape `player.rs`/`red_robot.rs`/`camera_noise_shake.rs` use for their `model.rs`; `lib.rs` is unchanged.
- [X] T012 [US2] In `level.rs`, replace `spawn_robot`'s per-spawn `load::<PackedScene>("res://enemies/red_robot/red_robot.tscn")` (`:166`) and `add_player`'s per-spawn `load::<PackedScene>("res://player/player.tscn")` (`:204`) with `#[init(val = load("res://enemies/red_robot/red_robot.tscn"))] robot_scene: Gd<PackedScene>` and `#[init(val = load("res://player/player.tscn"))] player_scene: Gd<PackedScene>` bare fields, resolved once (V2-C's `bullet_scene` precedent); both spawn sites keep calling `.instantiate_as::<T>()` on the field, still creating a fresh instance per spawn (FR-005)
- [X] T013 [US2] In `level.rs`, replace the 5 `get_node_as::<Node3D>("VoxelGI"/"ReflectionProbes")` call sites across `setup_sdfgi`/`setup_voxelgi`/`setup_lightmapgi` (`:96-97,122-123,139,146-147,160`) with `#[init(node = "VoxelGI")] voxel_gi: OnReady<Gd<Node3D>>` and `#[init(node = "ReflectionProbes")] reflection_probes: OnReady<Gd<Node3D>>` fields
- [X] T014 [US2] In `level.rs`'s `ready()`, replace the `match gi_type { GiType::Sdfgi => self.setup_sdfgi(), ... }` dispatch and delete the three `setup_*` methods; call `model::gi_plan(gi_type, gi_quality, self.lightmap_gi.is_some())` once and apply it via a new `apply_gi_plan(&mut self, plan: level::model::GiPlan)` glue method following research.md R3's exact 7-step order: (1) `world_environment`'s `sdfgi_enabled`; (2) `voxel_gi.set_visible`; (3) `reflection_probes.set_visible`; (4) free `self.lightmap_gi` if `plan.free_lightmap` and `Some`; (5) if `plan.create_lightmap`, allocate + `load::<LightmapGiData>("res://level/level.lmbake")` + name + store in `self.lightmap_gi` + `add_child` (the ONE remaining per-level-instance `load`, not per-spawn — spec's Assumptions explicitly excludes it from FR-005); (6) if `plan.lightmap_visible == Some(v)`, set `self.lightmap_gi`'s visibility to `v`; (7) if `plan.sdfgi_rays`/`plan.voxel_quality` are `Some`, write via `RenderingServer::singleton()` (FR-006)
- [X] T015 [US2] In `level.rs`'s `ready()`, remove the pre-shuffle `randomize()` call (`:57`) — the spawn-point shuffle now draws from whatever RNG state `Main`'s single boot-time `randomize()` left it in (FR-010, Acceptance Scenario 6)
- [X] T016 [US2] In `level.rs`'s `add_player`, replace the inline `(randi() % count as i64) as i32` pick (`:196-202`) with `model::pick_spawn(randi() as i64, count as i64) as i32` — RNG sampled in glue, formula in pure (FR-008)
- [X] T017 [US2] In `level.rs`'s `add_player`, change the plain `self.spawned_nodes.add_child(&player)` (`:208`) to `self.spawned_nodes.add_child_ex(&player).force_readable_name(true).done()`, matching `spawn_robot`'s existing form — verified a no-op for the player since `set_name` already runs first (FR-009, Acceptance Scenario 5)
- [X] T018 [US2] In `level.rs`, rewrite `_respawn_robot` (`:178-185`) as a `godot::task::spawn` block capturing `self.to_gd()` (`Gd<Level>`) and a cloned `spawn_point: Gd<Node3D>`, awaiting `self.base().get_tree().create_timer(15.0).signals().timeout().to_future()`, then checking `is_instance_valid()` on the captured `Gd<Level>` before calling `spawn_robot` again — same 15 s delay, same effect (FR-007)
- [X] T019 [US2] Gates + headless (`level/level.tscn --quit-after 120`, `main/main.tscn --quit-after 120`) per quickstart.md §2; grep SC-003 (`grep -n 'load(\|get_node_as' oxide_godot_core/oxide_godot_lib/src/level.rs` — no matches inside `spawn_robot`/`add_player`/`ready` bodies, only the T014-step-5 `.lmbake` load, reviewed and kept per R3) and SC-004 (`grep -n 'connect_other' oxide_godot_core/oxide_godot_lib/src/level.rs` — only the already-typed `peer_connected`/`peer_disconnected`, no `create_timer(...).connect_other` chain left); commit with message `level: extract GiPlan, pick_spawn into level/model.rs; preloaded scenes, OnReady GI nodes, uniform add_child, async respawn, redundant randomize() removed; closes backlog #19 (level half), #20, #21, #32 (load half)` (research.md R9); stage `level/model.rs`, `level.rs`, `lib.rs`

### Implementation for User Story 2 — commit 3: `flying_forklift.rs`

- [X] T020 [P] [US2] In `oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs`, add an inline `mod pure { pub fn pick_model(r: f64, count: usize) -> usize { (r * count as f64).floor() as usize } #[cfg(test)] mod tests { ... } }` transcribing `:30`'s formula exactly (research.md R5), with tests at `r = 0.0` (index 0), `r` just below `1.0` (last index), and `count = 1` (always index 0) — ≥ 3 tests (matches `bullet.rs`/`door.rs`'s inline-`mod pure` precedent)
- [X] T021 [US2] In `flying_forklift.rs`'s `ready()`, remove the per-instance `randomize()` call (`:27`) (FR-010, Acceptance Scenario 7)
- [X] T022 [US2] In `flying_forklift.rs`'s `ready()`, replace the model-pick line (`:30`, `(randf() * child_count as f64).floor() as usize`) with `pure::pick_model(randf(), child_count)` — RNG sampled in glue, the visibility-toggle loop (`:31-33`) unchanged, driven by the pure function's returned index
- [X] T023 [US2] Gates + headless (`level/level.tscn --quit-after 120` — instantiates a `FlyingForklift`, or `level/forklift/flying_forklift.tscn` standalone if it validates independently; `main/main.tscn --quit-after 120`); commit with message `flying_forklift: pick_model, redundant randomize() removed; closes backlog #19 (forklift half)` (research.md R9); stage only `flying_forklift.rs`

### Parity harness for User Story 2 — cases (b)/(c)/(d)

- [X] T024 [US2] Build `oxide-godot/oxide-godot/zz_end_parity.gd`/`.tscn` from `specs/010-v2-level-menu-main/contracts/zz_end_parity.gd`'s skeleton (throwaway, not committed): fill case (b) — for each of the 9 `(gi_type, gi_quality)` cells, configure `Settings` via the SAME path on both trees (either the menu buttons + Apply, as V2-A's own harness does, or writing `settings.ini` before boot — record which one quickstart.md §4 ends up using), instance `level/level.tscn` standalone, await a frame, dump `sdfgi_enabled`, `VoxelGI`/`ReflectionProbes` `.visible`, whether a `LightmapGI` child exists and its `.visible`, free the level, repeat for the next cell
- [X] T025 [US2] In the same harness file, fill case (c): `seed(12345)` as the very first statement (matching where `Main`'s boot-time `randomize()` would already have run — this harness bypasses `Main` entirely, per Edge Cases), instance `level/level.tscn` standalone, await a frame, dump `SpawnedNodes`' children (count, names), trigger a robot's death (`robot.rpc("hit")` ×5, or `exploded.emit()` directly on the harness's own authority) and count frames until the NEXT robot appears under `SpawnedNodes` (~900 @ 60 fps for the 15 s respawn), locate the `FlyingForklift` instance (if `level.tscn` has one) and dump which model child ends up visible — the INDEX is excluded from the diff per FR-019(c)'s documented #19 divergence (`v1` unseeded); only "exactly one visible" is compared
- [X] T026 [US2] Fill case (d) WITHOUT touching production code: the harness scene instances `main/main.tscn` as its own child (`Main::ready` runs normally; in headless the menu auto-hosts and loads the level by itself) and polls, every `process_frame`, `main.get_child_count()` and `main.get_child(0).get_class()` (expect `Menu` → then `Level`), recording the frame of each change; then it injects the quit action through the event path `Level::input` actually listens to — `var ev := InputEventAction.new(); ev.action = "quit"; ev.pressed = true; Input.parse_input_event(ev)` (`Input.action_press` alone does NOT deliver an `InputEvent` to `_input`) — and records the frame the child becomes `Menu` again. Dump the `[(frame, class)]` sequence. No `godot_print!`/debug hook in `main_scene.rs` — the harness observes from outside.
- [X] T027 [US2] Copy the filled `zz_end_parity.gd`/`.tscn` to `../oxide-godot-v1/oxide-godot/`; run cases (b)/(c)/(d) on both trees with `XDG_DATA_HOME=/tmp/parity-v1` / `/tmp/parity-v2`, `--fixed-fps 60`, per quickstart.md §4; diff the dumps — identical outside the documented forklift-model-index exclusion (case c) — report the actual `diff` output, not a claimed match (per the standing "verify before claiming parity" discipline)

- [X] T028 🛑 **STOP — checkpoint 1 (user visual, confirms both US1 and US2)**: boot the game (`main/main.tscn`); menu appears; Play → level loads; robots and players spawn at their spawn points; a level boots with the GI technique/quality its settings specify (SDFGI/VoxelGI/LightmapGI all look visually correct); kill a robot (5 bullet hits) and confirm a new one appears ~15 s later — **ask the user to note, without acting on it, whether backlog #32's respawn hitch changed**; flying forklifts show one of three models at random; pressing Esc in the level returns to the menu; Play again still works. Wait for the user's explicit confirmation before starting Phase 4.

  **Confirmed by the user**: "Ok, everything works just fine, the FPS drop bug when enemy respawn as also being fix[ed]" — backlog #32 resolved; recorded in memory `v2e-checkpoint1-backlog32-resolved.md` for the Phase 5 docs commit (T050) to close citing `a86b13f`.

---

## Phase 4: User Story 3 - `menu.rs`: declarative option table (Priority: P2) 🎯 Commit 4

**Goal**: `_on_settings_pressed`/`_on_apply_pressed`'s ~250 lines of `if`/`else if` become one
declarative table of 15 pure row functions; `is_equal_approx` (engine-backed) is reimplemented
pure; the loading-status poll becomes a pure `loading_step`; the headless auto-host call uses
the same typed deferred mechanism as US1.

**Independent Test**: `cargo test` passes for the option table; V2-A's settings harness (case
a) produces identical output on both trees; in the running game, every settings row
round-trips (open Settings, change a row, Apply, reopen — the row shows the new selection).

### Implementation for User Story 3

- [ ] T029 [US3] Create `oxide_godot_core/oxide_godot_lib/src/menu/model.rs` with a pure `pub(crate) fn is_equal_approx(a: f64, b: f64) -> bool` using Godot's documented formula (research.md R6): `let tolerance = (CMP_EPSILON * a.abs()).max(CMP_EPSILON); (a - b).abs() < tolerance` with `const CMP_EPSILON: f64 = 1e-5` — used ONLY inside this file's `resolution_scale_button`, NEVER `godot::global::is_equal_approx` (confirmed engine-backed, dispatches through `sys::utility_function_table()`, `target/debug/build/godot-core-*/out/utilities.rs:411-420`)
- [ ] T030 [US3] **Mandatory empirical cross-check** (per research.md R6): from a headless context where the engine is available (NOT `cargo test` — e.g. a temporary `#[func]` on an existing node, or a throwaway harness scene run via `godot.x86_64 --headless`), compare T029's pure `is_equal_approx` against the REAL `godot::global::is_equal_approx` for all six resolution-scale presets (`1/3`, `1/2`, `1/1.7`, `1/1.5`, `1/1.3`, `1.0`) plus a few near-boundary values (each preset ± 1e-6, ± 1e-4) — **STOP and report if ANY disagreement is found; do not proceed to T034 until they match**
- [ ] T031 [US3] In `menu/model.rs`, add `display_mode_button(mode: WindowMode) -> DisplayModeButton` / `display_mode_value(button: DisplayModeButton) -> WindowMode` (`menu.rs:306-310`/`:430-436`) — `WINDOWED`/`MAXIMIZED` both show "Windowed"; Apply on "Windowed" always writes `WINDOWED` (the `MAXIMIZED`→`WINDOWED` collapse MUST be preserved, not fixed)
- [ ] T032 [US3] In `menu/model.rs`, add `vsync_button`/`vsync_value` (`menu.rs:312-317`/`:438-446`) — exhaustive both ways, 4 variants (`Disabled`/`Enabled`/`Adaptive`/`Mailbox`)
- [ ] T033 [US3] In `menu/model.rs`, add `max_fps_button`/`max_fps_value` (`menu.rs:319-328`/`:448-464`) — the 8 literal values `30/40/60/72/90/120/144/Unlimited(0)`, exhaustive show-side with `Unlimited` as the else-branch
- [ ] T034 [US3] In `menu/model.rs`, add `resolution_scale_button(scale: f64) -> ResolutionScaleButton` / `resolution_scale_value(button: ResolutionScaleButton) -> f64` (`menu.rs:330-342`/`:466-478`) using T029's pure `is_equal_approx` against the six literal ratios in the SAME checked order as today (`1/3, 1/2, 1/1.7, 1/1.5, 1/1.3`, else `Native`) — the fallback-to-Native-on-unlisted-value quirk MUST be preserved (depends on T029/T030)
- [ ] T035 [US3] In `menu/model.rs`, add `scale_filter_button`/`scale_filter_value` (`menu.rs:344-351`/`:480-492`) — exhaustive both ways, 6 variants; the `metalfx_supported` button-hiding gate stays in glue, untouched
- [ ] T036 [US3] In `menu/model.rs`, add `gi_type_button`/`gi_type_value` and `gi_quality_button`/`gi_quality_value` (`menu.rs:353-363`/`:494-508`) — exhaustive both ways, 3 variants each
- [ ] T037 [US3] In `menu/model.rs`, add `msaa_button(m: Msaa) -> Option<MsaaButton>` / `msaa_value(b: MsaaButton) -> Msaa` and `screen_space_aa_button(a: ScreenSpaceAa) -> Option<ScreenSpaceAaButton>` / `screen_space_aa_value(b: ScreenSpaceAaButton) -> ScreenSpaceAa` (`menu.rs:371-384`/`:512-528`) — the show-side returns `None` on an unlisted value (v1's own gap, NO catch-all, `:371-377` has no `_ =>` arm — nothing gets pressed; preserve verbatim, do not "fix")
- [ ] T038 [US3] In `menu/model.rs`, add `ssao_quality_button`/`ssao_quality_value` and `ssil_quality_button`/`ssil_quality_value` (`menu.rs:392-402`/`:532-546`) — exhaustive both ways, 3 variants each
- [ ] T039 [US3] In `menu/model.rs`, add `#[cfg(test)] mod tests` covering every row in T031–T038: the `MAXIMIZED`→`Windowed` show + `Windowed`→`WINDOWED` write collapse; `resolution_scale`'s Native fallback on an unlisted value (e.g. `0.6`) AND correct matches on all 6 presets; `msaa`/`screen_space_aa`'s `None` on an unlisted enum ordinal; at least one show+apply round-trip per remaining row — ≥ 15 tests total (spec's stated minimum)
- [ ] T040 [US3] In `menu/model.rs`, add `pub enum LoadingCmd { UpdateProgress(f64), Finished, Failed }` and `pub fn loading_step(status: ThreadLoadStatus, progress: f64) -> LoadingCmd` transcribing `process`'s three-way branch (`menu.rs:246-256`) exactly (`IN_PROGRESS` → `UpdateProgress(progress * 100.0)`, `LOADED` → `Finished`, else → `Failed`), plus 3 tests (one per branch)
- [ ] T041 [US3] In `oxide_godot_core/oxide_godot_lib/src/menu.rs` (NOT `lib.rs`), add `mod model;` at the top so the file resolves to `menu/model.rs` (same precedent as T011); `lib.rs` is unchanged.
- [ ] T042 [US3] In `menu.rs`, rewrite `_on_settings_pressed` (`:298-415`) as a 15-call table-walk: for each row, read the `GraphicsSettings` field, call the row's `_button` function from `model`, `set_pressed(true)` on the `OnReady` button field the result names (an `Option`-returning row's `None` is simply skipped — no button pressed) — target ≤ 40 lines (SC-002)
- [ ] T043 [US3] In `menu.rs`, rewrite `_on_apply_pressed` (`:422-560`) as a 15-call table-walk: for each row, read `is_pressed()` off the row's `OnReady` button field(s), call the row's `_value` function from `model`, write the `GraphicsSettings` field — target ≤ 40 lines (SC-002); keep the trailing `set_graphics`/`apply_graphics_settings`/`save_settings` calls unchanged at the end
- [ ] T044 [US3] In `menu.rs`'s `ready()`, replace `self.base_mut().call_deferred("_on_host_pressed", &[])` (`:212`) with `Callable::from_fn("_on_host_pressed", move |_args| { this.bind_mut()._on_host_pressed(); Variant::nil() }).call_deferred(&[])` per research.md R1/R7 — the SAME mechanism T005 established (FR-014)
- [ ] T045 [US3] In `menu.rs`'s `process`, replace the three-way `if status == ThreadLoadStatus::IN_PROGRESS { ... } else if ... LOADED { ... } else { ... }` (`:246-256`) with a call to `model::loading_step(status, progress)` and a `match` applying the returned `LoadingCmd` to the SAME three effects (progress bar value; stop polling + start the done timer; log the error + show main + hide loading) (FR-015)
- [ ] T046 [US3] Gates + headless (`menu/menu.tscn --quit-after 120`, auto-hosted `main/main.tscn --quit-after 120`) per quickstart.md §2; grep SC-002 (`awk '/fn _on_settings_pressed/,/^    }$/' oxide_godot_core/oxide_godot_lib/src/menu.rs | wc -l` and the same for `_on_apply_pressed` — both ≤ 40) and SC-005 crate-wide (`grep -rn 'has_signal\|has_method\|\.call(\|\.get("\|call_deferred(".*"\|Callable::from_object_method' oxide_godot_core/oxide_godot_lib/src/` — zero matches; `grep -rn '\.rpc(' oxide_godot_core/oxide_godot_lib/src/` — exactly the 7 sites in `contracts/end-of-v2-api.md`); commit with message `menu: declarative option table + loading_step in menu/model.rs, Callable::call_deferred for headless auto-host; closes backlog #22` (research.md R9); stage `menu/model.rs`, `menu.rs`, `lib.rs`

### Parity harness for User Story 3 — case (a)

- [ ] T047 [US3] Run T003's copied `zz_settings_parity.gd`/`.tscn` unchanged on both trees, `XDG_DATA_HOME=/tmp/parity-v1` / `/tmp/parity-v2`, `--fixed-fps 60`; diff `settings.ini` bytes + applied engine state → identical, per V2-A's own original pass/fail criteria; report the actual diff output

- [ ] T048 🛑 **STOP — checkpoint 2 (user visual)**: open Settings; change EVERY row (display mode, vsync, max FPS, resolution scale, scale filter, TAA, MSAA, screen-space AA, shadow mapping, GI type, GI quality, SSAO, SSIL, bloom, volumetric fog); press Apply; reopen Settings — every row shows exactly the change that was just made; F11 toggles fullscreen; headless auto-host still works. Wait for the user's explicit confirmation before starting Phase 5.

---

## Phase 5: Polish — Commit 5 (docs, cleanup, final validation)

- [ ] T049 In `docs/v2-backlog.md`, mark #3, #19 (note the `level.rs` pre-shuffle `randomize()` extension closed alongside it), #20, #21 (note the literal default-parameter premise did not hold — the actual work was extracting `pick_spawn`), #22, #23, #24 as **done**, each citing the closing commit's hash (T007/T019/T023/T046)
- [ ] T050 In `docs/v2-backlog.md`, mark #32 **done** (citing T019's commit) if checkpoint 1 (T028) confirmed the respawn hitch resolved, or leave it **open** with the observation appended (the "record, don't chase further" pattern #28/#29 used) if not
- [ ] T051 In `docs/v2-backlog.md`, add a one-line "post-v2 review" note to #6, #25, #29, #30, #31 (each stays open, unchanged); add a header note "v2 complete on `<today's date>`; open items are post-v2 candidates"
- [ ] T052 [P] In `README.md`, update the Versions table: v2's `Status` cell "In progress" → "**Complete**" with a short summary sentence (modules remodeled: `main_scene.rs`, `level.rs`, `flying_forklift.rs`, `menu.rs` — the last four of fifteen; total test count; patterns: typed `Settings`, snapshot→step→apply, async timers, `HitTarget`, the typed scene manager); update the stale "26 items" backlog count in the "Upstream behavior" row to the current total (32); keep the "real projects: v2/v3 only" note unchanged
- [ ] T053 Confirm no harness instrumentation leaked into production code: `grep -n 'godot_print!\|SCENE_UNDER_MAIN\|zz_' oxide_godot_core/oxide_godot_lib/src/main_scene.rs oxide_godot_core/oxide_godot_lib/src/level.rs oxide_godot_core/oxide_godot_lib/src/menu.rs` returns only lines that existed in v1 (the menu's `Error while loading level` print).
- [ ] T054 Remove the throwaway harness files from BOTH trees: `oxide-godot/oxide-godot/zz_settings_parity.{tscn,gd,gd.uid}`, `oxide-godot/oxide-godot/zz_end_parity.{tscn,gd,gd.uid}`, and their counterparts under `../oxide-godot-v1/oxide-godot/`; `git worktree remove ../oxide-godot-v1`; `git worktree prune`; confirm `git status` is clean on `v2` and `git worktree list` shows only the main checkout
- [ ] T055 Full gates (`cargo build && cargo clippy -- -D warnings && cargo test`, expect ≥ 112 tests total per SC-001) + headless (`main/main.tscn` end-to-end, `menu/menu.tscn`, `level/level.tscn`, each `--quit-after 120`) with zero new errors relative to the documented baseline (SC-006); final crate-wide residual grep `grep -rn '\.rpc(' oxide_godot_core/oxide_godot_lib/src/` — confirm exactly the 7 sites in `contracts/end-of-v2-api.md`'s "Crate-wide residual" table, nothing else; commit with message `docs: close backlog #3, #19, #20, #21, #22, #23, #24 citing this milestone's commits; #32 per the checkpoint observation; #6/#25/#29/#30/#31 post-v2 review notes; header "v2 complete on <date>"; README.md v2 Status -> Complete + summary` (research.md R9); stage `docs/v2-backlog.md`, `README.md`, and `main_scene.rs` only if T053 touched it

**Checkpoint**: v2 is complete — every module in the crate follows Principles I–III; the only
by-name dynamic access left anywhere is `.rpc("name", ...)`.

---

## Dependencies & Execution Order

- **Setup (Phase 1)**: no dependencies, run first.
- **US1 (Phase 2, commit 1)**: depends on Setup only. Independent of US2/US3.
- **US2 (Phase 3, commits 2–3)**: depends on Setup only, NOT on US1 (different files) — but
  T028's checkpoint intentionally confirms US1+US2 together since both must be visually sound
  before the game is playable end-to-end.
- **US3 (Phase 4, commit 4)**: depends on Setup only, NOT on US1/US2 (different files) — T044
  reuses T005's established `Callable::from_fn` pattern but does not require T004–T007 to have
  landed first (the pattern is documented in research.md R1, usable independently).
- **Polish (Phase 5, commit 5)**: depends on T028 and T048 (both checkpoints confirmed) and
  therefore on all of Phase 2–4 being complete.
- Within US2: T008–T011 (level/model.rs + lib.rs) before T012–T018 (level.rs glue, which calls
  `model::gi_plan`/`model::pick_spawn`); T012–T018 before T019 (commit 2's gate); T020 before
  T021–T022 (flying_forklift.rs glue calls `pure::pick_model`); T019 and T023 before T024–T027
  (harness needs both commits' code in place).
- Within US3: T029 (pure `is_equal_approx`) and T030 (its cross-check) before T034
  (`resolution_scale_button` uses it) — T030 MUST pass before T034 is trusted; T031–T040 before
  T041 (`lib.rs`); T041 before T042–T045 (glue calls `model::*`); T042–T045 before T046.

## Parallel Example

```bash
# T003 (harness copy) can run alongside T001-T002 (different files/concerns).
# T020 (flying_forklift.rs) has no file overlap with T008-T019 (level.rs/level/model.rs) —
# the two commits could be prepared in parallel, though the task list above sequences them
# for review-size reasons matching plan.md's Commit Plan.
# T052 (README.md) has no file overlap with T049-T051 (docs/v2-backlog.md) - parallel-safe.
```

Within `menu/model.rs` (T029–T040) and `level/model.rs` (T008–T010), every row/cell is a
separate function in the SAME new file — not marked `[P]` even though logically independent,
since concurrent edits to one file conflict; write them sequentially in one sitting.

## Implementation Strategy

1. Phase 1 Setup.
2. Phase 2 US1 (commit 1) — the scene manager, smallest file, establishes the
   `Callable::from_fn` pattern T044 reuses.
3. Phase 3 US2 (commits 2–3) — spawning/GI/respawn, then the harness (cases b/c/d), then 🛑
   checkpoint 1.
4. Phase 4 US3 (commit 4) — the option table (largest line-count change, lowest behavioral
   risk), then the harness (case a, reusing V2-A's own), then 🛑 checkpoint 2.
5. Phase 5 Polish (commit 5) — backlog/README bookkeeping, cleanup, final gates. v2 is
   complete when this commit lands.
