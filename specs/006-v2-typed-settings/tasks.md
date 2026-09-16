# Tasks: Milestone V2-A — Typed `Settings` and its consumers

**Input**: Design documents from `specs/006-v2-typed-settings/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: pure-logic unit tests ARE requested (spec FR-004/FR-017, contracts/settings-api.md) —
included below inside Phase 2 (US1). No web/API contract-test scaffolding applies to this project.

**Branch**: `v2`, worked directly. **Local commits only — never push.** Each commit message is
given verbatim in its closing task; it must match plan.md's Commit Plan.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an incomplete task — safe to work in parallel
- **[USn]**: which user story phase the task belongs to (spec.md priorities P1/P2/P3)
- Setup/Polish tasks carry no `[USn]` label
- Every task names its exact file path(s) and the verification step that closes it (gate command,
  headless run, grep, or harness diff) — per constitution Principles II/III this verification is
  not optional polish, it is part of the task

## Path Conventions

Single Godot-extension crate. Rust: `oxide_godot_core/oxide_godot_lib/src/*.rs`. Godot project:
`oxide-godot/oxide-godot/` (scenes, `project.godot`). Docs: `docs/*.md`, `CLAUDE.md`, `README.md`
at repo root. Spec artifacts already exist under `specs/006-v2-typed-settings/`.

---

## Phase 1: Setup

**Purpose**: parity baseline + the crate-wide clippy fix that must land before any other v2 commit
so Principle III's `cargo clippy` gate holds literally from here on (research.md R9's explicit
ordering rationale — this is Commit 0 of plan.md's Commit Plan, run FIRST).

- [x] T001 [P] Create the `v1` parity worktree: from the repo root, `git worktree add ../oxide-godot-v1 v1`; then `cd ../oxide-godot-v1/oxide_godot_core && cargo build`. **Verify**: build succeeds with no errors; `../oxide-godot-v1/` is a clean checkout of branch `v1`, never modified by this milestone.
- [x] T002 [P] Fix clippy `collapsible_if` in `oxide_godot_core/oxide_godot_lib/src/blast.rs:30` — collapse `if let Some(camera) = &self.camera { if camera.is_instance_valid() { ... } }` into `if let Some(camera) = &self.camera && camera.is_instance_valid() { ... }` (stable let-chains, edition 2024). No other change to this file.
- [x] T003 [P] Fix clippy `collapsible_if` in `oxide_godot_core/oxide_godot_lib/src/bullet.rs:50` — collapse `if let Some(mut collider) = collider { if collider.has_method("hit") { collider.rpc("hit", &[]); } }` into `if let Some(mut collider) = collider && collider.has_method("hit") { collider.rpc("hit", &[]); }`. No other change to this file.
- [x] T004 [P] Fix both clippy warnings in `oxide_godot_core/oxide_godot_lib/src/red_robot.rs`: (a) `:325` `cmp_owned` — `body.get_name() == StringName::from("Target")` → `body.get_name() == "Target"`; (b) `:391` `collapsible_if` — collapse `if hit_player { if let Ok(player) = player.try_cast::<Player>() { ... } }` into `if hit_player && let Ok(player) = player.try_cast::<Player>() { ... }`. This is a **1:1 syntax rewrite only** (research.md R9 caution) — do not touch anything else in this branch or the surrounding state machine.
- [x] T005 [P] Fix all three clippy warnings in `oxide_godot_core/oxide_godot_lib/src/menu.rs`: (a) `:28` `cmp_owned` — `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")` → `== "metal"`; (b) `:213` `cmp_owned` — `DisplayServer::singleton().get_name() == GString::from("headless")` → `== "headless"`; (c) `:379,381` `if_same_then_else` — merge `} else if scale_filter == Scaling3DMode::METALFX_TEMPORAL.ord() as i64 { self.scale_filter_metalfx_temporal.set_pressed(true); } else if self.metalfx_supported { self.scale_filter_metalfx_temporal.set_pressed(true); } else { ... }` into `} else if scale_filter == Scaling3DMode::METALFX_TEMPORAL.ord() as i64 || self.metalfx_supported { self.scale_filter_metalfx_temporal.set_pressed(true); } else { ... }`.
- [x] T006 [P] Fix clippy `cmp_owned` in `oxide_godot_core/oxide_godot_lib/src/main_scene.rs:23` — `DisplayServer::singleton().get_name() == GString::from("headless")` → `== "headless"`.
- [x] T007 [P] Fix clippy `cmp_owned` in `oxide_godot_core/oxide_godot_lib/src/settings.rs:31` — `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")` → `== "metal"`.
- [x] T008 Gate, validate and commit 0 (depends on T002–T007): in `oxide_godot_core/`, run `cargo build && cargo clippy --quiet && cargo test` (quickstart.md §1) — **zero warnings, all green**; headless-validate per quickstart.md §2 (`main.tscn`, `menu/menu.tscn`, `level/level.tscn`, no new errors beyond the documented baseline); commit with message `Fix 9 clippy warnings crate-wide (no behavior change)`.

**Checkpoint**: crate is clippy-clean; every later commit in this milestone can rely on the gate holding literally.

---

## Phase 2: User Story 1 — `settings.rs` remodeled into a typed, testable model (Priority: P1)

**Goal**: the 15 graphics options exist as a typed, `Copy` model parsed once per call from
`user://settings.ini`, with the apply-decision logic split into a pure, unit-tested `plan()` and
a thin `apply()` glue step — while `Settings`' compatibility surface stays intact so the 5
consumers (unmigrated until Phase 3) keep working unchanged. This is Commit 1 of the Commit Plan.

**Independent Test** (quickstart.md, matches spec.md's US1 Independent Test): `cargo test` passes
with the new unit tests, no Godot binary; headless `main.tscn`/`menu.tscn`/`level.tscn` show no
new errors; with `user://settings.ini` removed, boot produces defaults and creates no file, and
an Apply writes a file byte-identical to what a `v1` build would write for the same choices.

### Pure model — `oxide_godot_core/oxide_godot_lib/src/settings/graphics.rs` (NEW file)

- [x] T009 [US1] Create `oxide_godot_lib/src/settings/graphics.rs` with the 5 project enums and `WireValue`, exactly per `contracts/settings-api.md`: `GiType { Sdfgi, VoxelGi, LightmapGi }` (wire `0/1/2`); `GiQuality { Disabled, Low, High }` (wire `0/1/2`); `SsaoQuality { Disabled, Medium, High }` and `SsilQuality { Disabled, Medium, High }` (wire `-1/2/3`, kept as two distinct types per data-model.md's rationale — do not merge them); `ScaleFilter { Nearest, Bilinear, Fsr1, MetalFxSpatial, Fsr2, MetalFxTemporal }` (wire `0..=5`) with `to_engine(self) -> Scaling3DMode` and `from_wire(code: i64) -> Option<Self>`; `WireValue { Int(i64), Real(f64), Bool(bool) }`. All `#[derive(Copy, Clone, Eq, PartialEq, Debug)]` (or `PartialEq` only where a field is `f64`/uses a non-`Eq` type, per contracts/settings-api.md).
- [x] T010 [US1] In the same file, add `ScaleFilter::to_engine()`'s body with the api-gap comment on the exact line that maps `Nearest`: `// api-gap(godot-4.7): Scaling3DMode::NEAREST — absent from the gdext 0.5.5 prebuilt API (4.6); replace when the binding ships it` immediately above `Scaling3DMode::from_ord(5)` (research.md R2). `from_wire` matches `0..=5` directly, independent of the engine binding.
- [x] T011 [US1] In the same file, add `pub struct GraphicsSettings` with the 15 fields exactly as data-model.md's table (note: `resolution_scale: f64`, **not** `f32` — SC-003 byte-identical round trip depends on this, research.md R3), plus `default_for(metalfx_supported: bool) -> Self` with the exact default values listed in data-model.md, `to_wire(&self) -> [WireValue; 15]`, and `from_wire(present: [Option<WireValue>; 15], metalfx_supported: bool) -> (Self, Vec<&'static str>)` per the semantics in data-model.md (`None` → silent default; `Some` that fails to parse → default + name pushed to the `Vec`). Field/key order MUST match the fixed order in research.md R3 (`video`: display_mode, vsync, max_fps, resolution_scale, scale_filter; `rendering`: taa, msaa, screen_space_aa, shadow_mapping, gi_type, gi_quality, ssao_quality, ssil_quality, bloom, volumetric_fog).
- [x] T012 [US1] In the same file, add `AoDecision<Q> { enabled: bool, quality: Q, half_size: bool }`, `ApplyPlan` (fields per data-model.md), and `pub fn plan(settings: &GraphicsSettings) -> ApplyPlan`. Reproduce `v1` exactly per research.md R4: SSAO `Disabled` → `{ enabled: false, .. }` — keep the `// upstream bug fix: settings.gd used \`if\` instead of \`elif\` — "SSAO: Disabled" (-1) was re-enabled by the else` comment on the `else if` that follows; `Medium` → `{ enabled: true, quality: EnvironmentSsaoQuality::HIGH, half_size: false }`; `High` → `{ enabled: true, quality: EnvironmentSsaoQuality::MEDIUM, half_size: true }` (quirk, backlog #25, untouched). SSIL `Disabled` → off; `Medium` → `{ MEDIUM, false }`; `High` → `{ HIGH, true }`.
- [x] T013 [US1] In the same file, add `#[cfg(test)] mod tests` with at least these 8+ tests: (1) `to_wire`/`from_wire` round-trip for each of the 15 fields individually; (2) `default_for(true)` matches data-model.md's MetalFX-temporal default; (3) `default_for(false)` matches the FSR2 default; (4) a partial wire array (some slots `None`) keeps the present values and defaults only the missing ones; (5) a malformed `Some(WireValue)` on an enum-backed field (e.g. an out-of-range int for `gi_type`) defaults that field AND appends its name to the returned `Vec`; (6) `plan()` for `SsaoQuality::Disabled/Medium/High` matches the R4 table; (7) `plan()` for `SsilQuality::Disabled/Medium/High` matches the R4 table. **Verify**: `cargo test` in `oxide_godot_core/` passes, ≥ 8 tests, no Godot binary involved.

### Glue — `oxide_godot_core/oxide_godot_lib/src/settings.rs` (MODIFIED, transitional shape)

- [x] T014 [US1] Rewrite `settings.rs` to the **transitional shape** (research.md R7, contracts/settings-api.md "Transitional shape"): add `mod graphics;` and `pub use graphics::{...}`; KEEP `#[var] config_file: Gd<ConfigFile>` and `#[func]` on `load_settings`, `save_settings`, `apply_graphics_settings` exactly as today; REMOVE `#[constant] GI_TYPE_*`/`GI_QUALITY_*` and the `#[var]` exposure of `metalfx_supported` — the field itself STAYS, private, because `default_for`/`from_wire` need it on every transitional re-parse and in the final `ready` (re-run the R8 grep `grep -rn "GI_TYPE_\|GI_QUALITY_\|metalfx_supported\|config_file" --include='*.tscn' --include='*.gd' oxide-godot/` and record that it is still zero). ADD the two typed accessors in their TRANSITIONAL form so T019–T023 compile and behave before T024: `pub fn graphics(&self) -> GraphicsSettings` = read `config_file` into `[Option<WireValue>; 15]` → `from_wire` (warn per malformed name) at each call; `pub fn set_graphics(&mut self, graphics: GraphicsSettings)` = write `graphics.to_wire()`'s 15 values into `config_file` via `set_value` (so the menu's `set_graphics` → `apply` → `save` sequence works while apply/save still parse from `config_file`). `apply_graphics_settings` and `save_settings` internally read `config_file` into a `[Option<WireValue>; 15]`, call `GraphicsSettings::from_wire`, then `plan()`, then a new private `fn apply(plan: &ApplyPlan, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)` glue method that pushes every `ApplyPlan` field to the engine AND replaces the dynamic shadow call with a typed recursive walk over `scene_root.get_children_ex().include_internal(true)`, `try_cast::<Light3D>`, `set_shadow(false)` (research.md R5 — walk ALL children including internal ones, matching `propagate_call`'s own reach). `input`'s `toggle_fullscreen` handling is unchanged.
- [ ] T015 [US1] Gate, validate and commit 1 (depends on T009–T014): `cargo build && cargo clippy --quiet && cargo test` clean in `oxide_godot_core/`; headless-validate `main.tscn`/`menu/menu.tscn`/`level/level.tscn` (no new errors); commit with message `Remodel settings.rs: typed GraphicsSettings model + plan/apply split`.

### Parity checkpoint (US1)

- [x] T016 [US1] Turn `specs/006-v2-typed-settings/contracts/zz_settings_parity.gd`'s skeleton into a real harness: create `oxide-godot/oxide-godot/zz_settings_parity.tscn` (root `Node`, script = the filled-in `.gd`) implementing all three cases from research.md R10 — case (a) delete `user://settings.ini`, boot, assert absent, one no-op Apply, copy the file; case (b) for each of the 15 rows × option, press the button + `_on_apply_pressed()`, dump ini bytes + window mode/vsync/max_fps/scaling scale+mode/taa/msaa/screen_space_aa/ssao_enabled/ssil_enabled/glow_enabled/volumetric_fog_enabled + `has_shadow()` of the two `SpotLight3D` in `menu.tscn` (the direct check of the menu's Apply, whose `scene_root` is the `Menu` node) and, for the shadow-off case, additionally instance `level/level.tscn` after the Apply (its `Level.ready` calls `apply_graphics_settings` with itself as `scene_root`) and dump `has_shadow()` of every `Light3D` beneath it, then free it; case (c) hand-edit `gi_type = 7`, boot, capture whether a warning was logged and the effective `GiType`. Known coverage gap, stated here so nobody expects it from the harness: the SSAO/SSIL quality constant and `half_size` passed to `RenderingServer` have NO getter and are not dumped — they are covered by the `plan()` unit tests (T013) plus a side-by-side read of the constants against `v1` (research.md R10). Copy the same two files into `../oxide-godot-v1/oxide-godot/`.
- [x] T017 [US1] Run the harness on both trees per quickstart.md §4 (`XDG_DATA_HOME=/tmp/parity-v1` for the `v1` worktree, `/tmp/parity-v2` for this branch) and diff the two `zz_parity_dump.json` files plus the copied `settings.ini`s. **Verify**: `case_a_boot` and `case_b_options` are byte-identical between the two runs (at this checkpoint the 5 consumers on `v2` still call `Settings` dynamically, unchanged, so behavior is expected to match `v1` exactly); `case_c_malformed` is expected to differ (not a failure) — its concrete PASS condition on `v2`: the run's stdout/stderr contains the `godot_warn!` line naming `gi_type`, and the effective GI setup is the default `VoxelGi` (the `Level`'s VoxelGI node visible, SDFGI disabled); on `v1` the value falls through to the LightmapGI `else` branch, recorded, not diffed.

### 🛑 USER VISUAL CHECKPOINT — STOP and wait

- [ ] T018 [US1] **STOP.** Ask the user to confirm, in the running game (not headless): the game boots with the default graphics settings; the Settings menu shows those defaults; pressing Apply persists the choice across a full restart of the game (SC-007, "after US1"). **Do not start Phase 3 until the user confirms this checkpoint.**

**Checkpoint**: User Story 1 is complete and independently verified — typed model, pure `plan()`,
unit tests, parity, and user confirmation all in place; the 5 consumers are untouched and still
work exactly as in `v1`.

---

## Phase 3: User Story 2 — the 5 consumers move to typed access (Priority: P2)

**Goal**: every consumer resolves `Gd<Settings>` once and reads/writes typed fields/methods
instead of dynamic `.get`/`.call` by name; `Settings`' compatibility surface is then removed.
Closes `docs/v2-backlog.md` item #1. Depends on Phase 2's checkpoint (T018) being confirmed.

**Independent Test**: headless `main.tscn`/`menu.tscn`/`level.tscn` with no new errors; the
quickstart.md §5 grep returns nothing; SDFGI/VoxelGI/LightmapGI each render as before; turning
shadows off in the menu still turns them off in the level.

- [ ] T019 [P] [US2] `oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs`: add `#[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))] settings: OnReady<Gd<Settings>>`; replace the `get_node_as::<Node>("/root/Settings").get("config_file")` read (line 19) with `self.settings.bind().graphics().shadow_mapping` (or the equivalent field it actually reads — check the current line before editing). No other behavior change. Gate + headless-validate; commit `flying_forklift.rs: typed Settings access` (Commit 2).
- [ ] T020 [P] [US2] `oxide_godot_core/oxide_godot_lib/src/bullet.rs`: add the same `OnReady<Gd<Settings>>` field, but resolve/read it from `ready()` — **NOT inside `explode()`** (research.md R6, spec Edge Cases). Replace the dynamic read inside `explode` (line 72, `get("config_file")` → `shadow_mapping`) with `self.settings.bind().graphics().shadow_mapping` at `explode`-time: the `OnReady` handle is resolved once at `ready`, and dereferencing it per event is a field access, not a tree lookup (research.md R6). Do NOT snapshot the value at `ready` — that would change behavior if settings change between spawn and explosion. Gate + headless-validate; commit `bullet.rs: typed Settings access` (Commit 3).
- [ ] T021 [P] [US2] `oxide_godot_core/oxide_godot_lib/src/main_scene.rs`: add the same field; replace the `get_node_as::<Node>("/root/Settings").get("config_file")…get_value("video", "display_mode").to::<i64>()` read (lines 27-33) with `self.settings.bind().graphics().display_mode`, and pass it straight to `set_mode(...)` — the `WindowMode::from_ord(display_mode as i32)` conversion (line 37) is DELETED, not retyped, because the field is already a `WindowMode`. No other behavior change. Gate + headless-validate; commit `main_scene.rs: typed Settings access` (Commit 4).
- [ ] T022 [P] [US2] `oxide_godot_core/oxide_godot_lib/src/level.rs`: add the same field; replace the `.call("apply_graphics_settings", …)` (line 43-45) with `self.settings.bind_mut().apply_graphics_settings(window, environment, self.to_gd().upcast())`; replace the `.get("config_file")` + `if gi_type == SDFGI {…} else if gi_type == VOXEL_GI {…} else {…}` chain with `match self.settings.bind().graphics().gi_type { GiType::Sdfgi => self.setup_sdfgi(), GiType::VoxelGi => self.setup_voxelgi(), GiType::LightmapGi => /* the existing third branch */ }`; remove the 5 local constants `SDFGI`/`VOXEL_GI`/`GI_DISABLED`/`GI_LOW`/`GI_HIGH`. No other change to `level.rs` (its own `gi_plan` pure extraction is out of scope, V2-E). Gate + headless-validate; commit `level.rs: typed Settings access + GI match` (Commit 5).
- [ ] T023 [P] [US2] `oxide_godot_core/oxide_godot_lib/src/menu.rs`: add the same field; in `_on_settings_pressed`, replace the 15 `config_file.get_value(...)` reads with reads of `self.settings.bind().graphics()`'s fields to set each row's button state; in `_on_apply_pressed`, replace the ~52 `config_file.set_value(...)` writes with building one `GraphicsSettings` value from the button states and calling `self.settings.bind_mut().set_graphics(graphics)`, then the already-present-but-now-typed `apply_graphics_settings(...)` and `save_settings()` calls (no more `.call("name", ...)`); remove the local `const SCALING_3D_MODE_NEAREST` and set that field to `ScaleFilter::Nearest` directly. The button `if`/`else if` chains that decide WHICH value to set stay as they are (backlog #22, deferred to V2-E) — only how the chosen value reaches/leaves the model changes. **Verify** (SC-005): `grep -n SCALING_3D_MODE_NEAREST oxide_godot_core/oxide_godot_lib/src/menu.rs` returns zero matches. Gate + headless-validate; commit `menu.rs: typed Settings access, drop SCALING_3D_MODE_NEAREST const` (Commit 6).
- [ ] T024 [US2] (depends on T019–T023 ALL landing) `oxide_godot_core/oxide_godot_lib/src/settings.rs`: remove `#[var]` from `config_file` (keep the field itself, private, no macro exposure) and remove `#[func]` from `load_settings`, `save_settings`, `apply_graphics_settings` (methods themselves stay, plain now); add a `graphics: GraphicsSettings` field; change `ready()` to load `config_file` once and call `GraphicsSettings::from_wire` once into `graphics` (stop calling the old `load_settings`); `graphics()` becomes a plain copy of the field and `set_graphics` a plain field write (the transitional re-parse/`set_value` bodies from T014 are deleted); `save_settings` now writes `graphics.to_wire()`'s 15 values into the SAME retained `config_file` object via `set_value` (not a fresh `ConfigFile`, per research.md R7 — preserves unknown keys/order of a hand-edited file) and saves it; `apply_graphics_settings`/`save_settings`/etc. now operate on the stored `graphics` field, no per-call re-parsing. **Verify** (quickstart.md §5): `grep -rn 'get_node_as::<Node>("/root/Settings")\|\.get("config_file")\|\.call("apply_graphics_settings\|\.call("save_settings' oxide_godot_core/oxide_godot_lib/src` returns **zero matches**. Gate + headless-validate; commit `settings.rs: remove compatibility surface, parse once at ready` (Commit 7).

### Parity checkpoint (US2)

- [ ] T025 [US2] Re-run the parity harness (quickstart.md §4, same `XDG_DATA_HOME` split) on both trees. **Verify**: `case_a_boot` and `case_b_options` still byte-identical (now genuinely exercising both branches' full menu-button → Apply path, since `v2`'s `#[var] config_file` is gone); `case_c_malformed` still shows the one documented deviation on `v2` only (same PASS condition as T017: warning line naming `gi_type` + effective `VoxelGi`).

### 🛑 USER VISUAL CHECKPOINT — STOP and wait

- [ ] T026 [US2] **STOP.** Ask the user to confirm, in the running game: everything from the US1 checkpoint still holds, PLUS each of SDFGI/VoxelGI/LightmapGI renders as before when selected, and turning shadows off in the Settings menu actually disables shadows in the level (SC-007, "after US2"). **Do not start Phase 4 until the user confirms this checkpoint.**

**Checkpoint**: User Story 2 is complete — zero dynamic `Settings` access remains in the crate
(backlog #1 closed), all 5 consumers are typed, parity holds, user confirmed.

---

## Phase 4: User Story 3 — Gates, API-gap catalog and operational docs (Priority: P3)

**Goal**: catalogue the one known engine API gap and bring the operational guide, backlog, and
README up to date with this milestone's outcome. No observable game behavior changes in this
phase. Depends on Phase 3 (T026 confirmed) — though these doc edits could technically start
earlier, they are sequenced last per plan.md's Commit Plan so `docs/v2-backlog.md`'s "closing
commit" references (T028 below) can name real commit hashes from Phases 1–3.

- [ ] T027 [P] [US3] Create `docs/api-gaps.md` with the `Scaling3DMode::NEAREST` entry exactly as pinned in research.md R2: Symbol `Scaling3DMode::NEAREST`; Introducing version Godot 4.7 (absent from gdext 0.5.5's prebuilt API 4.6); Workaround `ScaleFilter::Nearest` maps to `Scaling3DMode::from_ord(5)`, reusing the `MAX` ordinal slot the 4.6 binding leaves unnamed; Location `oxide_godot_lib/src/settings/graphics.rs`, `ScaleFilter::to_engine()`.
- [ ] T028 [P] [US3] Update `CLAUDE.md`'s "Work cycle" section: add `cargo clippy` and `cargo test` as mandatory steps after `cargo build`; add a short note describing the Principle III module layout used here (`settings.rs` = glue, `settings/graphics.rs` = pure model with `#[cfg(test)]`); document the `// api-gap(godot-<version>): <symbol> — <reason>; replace when the binding ships it` comment convention with a pointer to `docs/api-gaps.md`; add a note on using a `v1` git worktree + a separate `XDG_DATA_HOME` per tree for the parity harness (quickstart.md §3–4). Operational content only — no rule duplicated from the constitution.
- [ ] T029 Gate, validate and commit (depends on T027, T028): full gates + headless; commit with message `docs/api-gaps.md + CLAUDE.md: catalogue Scaling3DMode::NEAREST, document v2 gates/layout` (Commit 9).
- [ ] T030 [P] [US3] Update `docs/v2-backlog.md`: mark item **#1** done, citing the commit hash of T024/Commit 7 (the compatibility-surface removal — the commit that actually made access typed everywhere); add ONE new entry for the malformed-wire-value deviation (origin `menu/settings.gd` / `settings.rs`; improvement "parse, don't validate — out-of-range enum code defaults + warns instead of falling through an `else` chain"; motivation as stated in spec.md US1 scenario 9) and mark it done, citing T015/Commit 1's hash; add a one-line note next to items **#22** and **#23** ("target: V2-E") and **#25** ("parity first — this milestone reproduces both SSAO/SSIL quirks verbatim, still deferred").
- [ ] T031 [P] [US3] Update `README.md`'s Versions table: change the v2 column's **Status** row from "Not started" to "In progress".
- [ ] T032 Gate, validate and commit (depends on T030, T031): full gates + headless; commit with message `docs/v2-backlog.md + README.md: close #1 and the malformed-values item, v2 → In progress` (Commit 10).

**Checkpoint**: all documentation reflects the milestone's actual, committed outcome; no
observable behavior changed in this phase (nothing to re-confirm visually).

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: final end-to-end confirmation and removal of validation scaffolding — nothing here
is part of the crate or the Godot project's normal contents.

- [ ] T033 Run the parity harness one final time (quickstart.md §4) on both trees, all three cases, as an end-of-milestone confirmation that nothing drifted across Phases 3–4's doc-only commits.
- [ ] T034 Remove validation scaffolding: delete `zz_settings_parity.tscn`/`.gd` from both `oxide-godot/oxide-godot/` (this branch) and `../oxide-godot-v1/oxide-godot/`; then `git worktree remove ../oxide-godot-v1` from the `v2` checkout. **Verify**: `git status` in both trees shows no `zz_*` files; `git worktree list` no longer shows the `v1` worktree.
- [ ] T035 Final full gate + headless pass: `cargo build && cargo clippy --quiet && cargo test` clean in `oxide_godot_core/`; headless import + `main.tscn`/`menu/menu.tscn`/`level/level.tscn`, no new errors beyond the documented baseline.
- [ ] T036 Report the residual-dynamic-access list to the user, unchanged from spec.md's top block and confirmed still accurate by grep, with the line numbers of spec.md/FR-015 (re-resolved after this milestone's edits, since they shift): `bullet.rs:51` `has_method("hit")` (removed in V2-C when the typed `Hittable` dispatch lands); `bullet.rs:42,52,56` `.rpc("explode")`/`.rpc("hit")` by name (PERMANENT — gdext exposes RPC only by name); `menu.rs:214` `call_deferred("_on_host_pressed")` and `main_scene.rs:59` `call_deferred("change_scene_to_packed")` by name (V2-E); `main_scene.rs:70-76` `has_signal("quit")`/`has_signal("replace_main_scene")`/`Callable::from_object_method` (V2-E). Confirm none of these were touched by this milestone and none of the 5 consumer commits (T019–T024) accidentally introduced a new one.

**Checkpoint**: Milestone V2-A is complete — `docs/v2-backlog.md` item #1 closed, one new item
added and closed, `docs/api-gaps.md` started, all gates green, parity evidenced, both user
checkpoints confirmed.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately. Commit 0 (clippy) MUST land before
  any other Rust edit in this milestone (Principle III's gate must hold for every commit after).
- **User Story 1 (Phase 2)**: depends on Phase 1 (T008) — the crate must be clippy-clean first.
- **User Story 2 (Phase 3)**: depends on Phase 2's user checkpoint (T018) being CONFIRMED, not
  just T017 finishing — this is the infrastructure-first order the constitution requires
  (`Settings`' typed API must exist before its consumers move to it).
- **User Story 3 (Phase 4)**: depends on Phase 3's user checkpoint (T026) being confirmed (the
  backlog entry in T030 needs Phase 3's real commit hash).
- **Polish (Phase 5)**: depends on Phase 4 (T032).

### Within Phase 3 (User Story 2)

T019–T023 (the 5 consumers) have no dependency on each other — all five depend only on Phase 2
being done, so they are marked `[P]`. T024 (removing `Settings`' compatibility surface) depends
on ALL FIVE of them landing first — it deletes the `#[func]`/`#[var]` surface those five stop
using, and would break anything not yet migrated.

### Parallel Opportunities

- Phase 1: T001–T007 are all `[P]` (a worktree add and 6 independent-file clippy fixes).
- Phase 3: T019–T023 are all `[P]` (5 independent consumer files).
- Phase 4: T027/T028 are `[P]` (different files, one commit); T030/T031 are `[P]` (different
  files, one commit).
- Nothing in Phase 2 is `[P]` against anything else in Phase 2 — T009–T013 all edit the same new
  file (`graphics.rs`) sequentially by design (enums → model → plan → tests, each building on
  the last), and T014 edits `settings.rs` after `graphics.rs` exists.

---

## Parallel Example: Phase 3 (User Story 2)

```bash
# After Phase 2's user checkpoint (T018) is confirmed, launch all 5 consumer migrations together:
Task: "flying_forklift.rs: typed Settings access (T019)"
Task: "bullet.rs: typed Settings access, resolved at ready (T020)"
Task: "main_scene.rs: typed Settings access (T021)"
Task: "level.rs: typed Settings access + GI match (T022)"
Task: "menu.rs: typed Settings access, drop SCALING_3D_MODE_NEAREST const (T023)"

# Only after ALL FIVE commits land:
Task: "settings.rs: remove compatibility surface, parse once at ready (T024)"
```

---

## Implementation Strategy

### MVP scope

User Story 1 (Phase 2) alone is a complete, independently valuable increment: it is where the
typed model, the pure `plan()`, and the crate's first unit tests land, and it is fully backward
compatible with the unmigrated consumers. If the milestone had to stop after Phase 2, `v2` would
still be in a coherent, playable, parity-verified state — just not yet closing backlog item #1.

### Incremental delivery

1. Phase 1 (Setup) → clippy-clean baseline.
2. Phase 2 (US1) → typed model + pure `plan()` + tests, checkpoint confirmed by user → **MVP**.
3. Phase 3 (US2) → all 5 consumers typed, backlog #1 closed, checkpoint confirmed by user.
4. Phase 4 (US3) → docs/backlog/README bookkeeping, no behavior change.
5. Phase 5 (Polish) → final confirmation, scaffolding removed.

Each phase ends with gates green and headless validation clean; Phases 2 and 3 additionally end
with a parity-harness diff and a user-confirmed visual checkpoint before the next phase starts.

---

## Notes

- `[P]` tasks touch different files and have no incomplete-task dependency between them.
- `[USn]` maps every Phase-2/3/4 task back to spec.md's user stories for traceability.
- Commit numbers in parentheses refer to plan.md's Commit Plan table (0–7, 9–10; there is no
  commit "8" — plan.md's table has a pre-existing numbering gap from when the clippy commit was
  moved to the front as commit 0; this does not affect task execution order, only the label).
- The two 🛑 STOP tasks (T018, T026) are confirmed by the USER, not by the implementer — do not
  mark them complete on the implementer's own judgment call.
- Do not push any commit; this branch (`v2`) is worked on locally throughout.
