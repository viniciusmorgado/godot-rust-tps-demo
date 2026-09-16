# Feature Specification: Milestone V2-A — Typed `Settings` and its consumers

**Feature Branch**: `v2` (work directly on `v2`, per constitution 1.4.0 Principle II — v2 follows an "infrastructure first" order, not a per-milestone feature branch; local commits only, never pushed)

**Created**: 2026-09-16

**Status**: Draft

**Phase**: v2 — Idiomatic Rust (constitution 1.4.0, Principles I, II and III). Both pillars apply, done together, to the final result of every user story: (1) idiomatic Rust leaning on the type system (typed model, enums, parse-don't-validate) and (2) FFI reduction through the interface/implementation separation of Principle III. Behavioral parity with branch `v1` is mandatory for every observable effect except the single documented deviation in User Story 1.

- **Backlog items closed by this spec**: #1 (typed `Gd<Settings>` access, `docs/v2-backlog.md`) and one new item added by this spec for the malformed-wire-value deviation (see User Story 1, "deliberate deviation").
- **Backlog items explicitly deferred** (left as-is, target milestone per `docs/v2-catalog.md`): #22 and #23 (`menu.rs` button chains and scene-manager typed connections — V2-E), #25 (SSAO/SSIL naming quirk — parity first, never fixed as a side effect of this milestone).
- **Residual dynamic access left in the touched modules after this milestone** (not changed here, Principle II residual-case list): `bullet.rs:51` `has_method("hit")` (removed by the typed `Hittable` dispatch in V2-C); `bullet.rs:42,52,56` `.rpc("explode")` / `.rpc("hit")` by name (PERMANENT — gdext exposes RPC only by name, the engine-limitation case the constitution names); `menu.rs:214` `call_deferred("_on_host_pressed")` and `main_scene.rs:59` `call_deferred("change_scene_to_packed")` by name (V2-E); `main_scene.rs:70-76` `has_signal("quit")` / `has_signal("replace_main_scene")` / `Callable::from_object_method` (removed when the typed scene manager lands in V2-E). Modules not touched by this milestone list their own residual cases in their own spec.
- **Transitional contract between US1 and US2**: the typed model lands in US1, but the 5 consumers still reach `Settings` by name until US2 migrates them. Therefore US1 KEEPS the compatibility surface (`#[var] config_file`, `#[func]` on `apply_graphics_settings` / `save_settings` / `load_settings`) and, in US1 only, `apply_graphics_settings` and `save_settings` re-parse the typed model from `config_file` at their boundary (parse-at-boundary), so the menu's `set_value` writes still take effect. "Parse ONCE at `ready`" and the removal of the compatibility surface are the LAST commit of US2, after all 5 consumers are typed. This is the intermediate step Principle I allows, declared here so the US1 checkpoint stays independently testable.

**Input**: User description: "Milestone V2-A — typed `Settings` and its consumers. Remodel `settings.rs` (188 l.) into a typed `GraphicsSettings`-style model under constitution 1.4.0 Principles I and III (parse-don't-validate, enums instead of raw ints, pure logic separated from engine glue, unit-tested); move the 5 consumers (`flying_forklift.rs`, `bullet.rs`, `level.rs`, `menu.rs`, `main_scene.rs`) off dynamic `/root/Settings` access onto typed `Gd<Settings>` access, closing v2-backlog item 1; bring the crate's `cargo build`/`cargo clippy`/`cargo test` gates to green, catalogue the `Scaling3DMode::NEAREST` API gap in `docs/api-gaps.md`, and update `CLAUDE.md`/`docs/v2-backlog.md`/`README.md` accordingly. Behavioral parity with branch `v1` is mandatory, verified by headless validation and a parity harness run on a `v1` worktree, with exactly one documented behavior deviation (malformed wire values fall back to the default with a warning instead of the original's ambiguous `else`-chain fallthrough)."

## Context

`Settings` (`oxide_godot_lib/src/settings.rs`, 188 lines) is the root of 5 consumers, all of which access it dynamically today — the v1-only exception of constitution Principle II ("Settings autoload ported last"). Current sites (10 `get_node_as::<Node>("/root/Settings")`, 12 dynamic accesses total, unchanged since `specs/005-v1-settings-autoload`):

| Consumer | Line(s) | Access |
|---|---|---|
| `flying_forklift.rs` | 19 | `.get("config_file")` |
| `bullet.rs` | 72 | `.get("config_file")` (inside `explode`, per event) |
| `level.rs` | 43, 116, 144, 173 | `.call("apply_graphics_settings", …)` once (43) + `.get("config_file")` ×4 |
| `menu.rs` | 208, 308, 479 | `.call("apply_graphics_settings", …)` ×2 (208, 606) + `.get("config_file")` ×2 (308, 480) + `.call("save_settings")` ×1 (611) |
| `main_scene.rs` | 29 | `.get("config_file")` |

Constitution 1.4.0 (Principle II, "v2 lifts that exception") forbids all of the above in v2 and requires the infrastructure-first order: `Settings` is remodeled before its consumers, so every later milestone builds on a typed API instead of `ConfigFile` string lookups.

Current `cargo clippy` baseline (verified against the working tree at spec time) reports exactly 9 warnings across the crate: `blast.rs:30`, `bullet.rs:50`, `red_robot.rs:325`, `red_robot.rs:391`, `menu.rs:28`, `menu.rs:213`, `menu.rs:379`, `main_scene.rs:23`, `settings.rs:31`. `cargo test` currently links and runs with 0 tests. Neither gate is enforced yet — this milestone makes both green and keeps them green going forward (constitution Principle III, "mandatory gates").

Behavior baseline for parity is branch `v1` (a separate worktree, built and run independently — never merged into or referenced from `v2`'s build).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `settings.rs` remodeled into a typed, testable model (Priority: P1)

The `Settings` autoload keeps loading, applying and saving the same 15 graphics options with the exact same observable effect as `v1`, but internally the raw `ConfigFile` string/int lookups are replaced by one typed model that is parsed once from the file and read everywhere else as typed fields, with the model's load/merge/apply-decision logic covered by engine-free unit tests.

**Why this priority**: `Settings` is the root of every other consumer; nothing in User Story 2 can start until the typed API this story creates exists. It is also where Principle III first bites for real (pure model vs. engine-applying glue) and where the crate's first `cargo test` coverage lands.

**Independent Test**: `cargo test` passes with the new unit tests and no Godot binary involved; headless `main.tscn` / `menu.tscn` / `level.tscn` show no new errors; with `user://settings.ini` removed, boot produces defaults and no file is created, and an Apply writes a file byte-identical to the one a `v1` build would write for the same choices.

**Acceptance Scenarios**:

1. **Given** the 15 settings currently read as raw `ConfigFile` values, **When** `settings.rs` is remodeled, **Then** they exist as typed fields of one model type: engine enums where gdext already has them (`WindowMode`, `VSyncMode`, `Msaa`, `ScreenSpaceAa`), project enums where the original used bare ints or sentinel values (`GiType { Sdfgi, VoxelGi, LightmapGi }`, `GiQuality { Disabled, Low, High }`, a three-state SSAO enum, a three-state SSIL enum, `ScaleFilter` for `scale_filter`), and plain typed fields for `max_fps`, `resolution_scale`, `taa`, `shadow_mapping`, `bloom`, `volumetric_fog`.
2. **Given** `user://settings.ini` does or does not exist, **When** `ready` runs, **Then** the file is loaded into the typed model (missing keys resolve to the same defaults as `v1`, in the same section/key order) and the file is NOT created by this read. In US1 (transitional contract, see top of spec) the compatibility surface stays and `apply_graphics_settings` / `save_settings` re-parse the model from `config_file` at their boundary, so consumers that still write `set_value` keep working; after the last commit of US2 the file is parsed exactly ONCE at `ready`, no code downstream of `ready` touches `ConfigFile` directly, and every consumer reads typed fields.
3. **Given** the typed model, **When** it is saved, **Then** the wire format is preserved byte-for-byte relative to `v1`: same sections, same keys, same key order, same value types per key (ints for enum codes using the SAME integer codes as `v1`, `-1` for SSAO/SSIL disabled, `0` int for `max_fps`, `1.0` real for `resolution_scale`, booleans as booleans) — a `settings.ini` written by `v1` is read unchanged by this model and vice versa.
4. **Given** the typed⇄wire conversion and the defaults (including the `metalfx_supported`-dependent default of `scale_filter`), **When** they are implemented, **Then** they live in plain Rust with no `Gd<T>`, `Variant`, `GString` or other engine-backed type, and are covered by `cargo test` (round-trip per field, defaults, merge of a partial file, SSAO/SSIL mapping) — these tests run and pass with no Godot binary present.
5. **Given** `apply_graphics_settings`, **When** it is decomposed, **Then** a pure function computes what to apply from the typed model (including the SSAO/SSIL quality-and-half-size decision) and a separate glue method applies that decision to `Window` / `Environment` / `RenderingServer` / the scene root, with the observable result IDENTICAL to `v1` — including both preserved SSAO/SSIL naming quirks (backlog #25, still deferred) and the existing upstream bug fix (SSAO `-1` branch using `else if`, kept with its `// upstream bug fix: ...` comment intact and unmoved in meaning).
6. **Given** the shadow-disabling call (`v1`: `scene_root.propagate_call_ex("set").args(&varray!["shadow_enabled", false])`, a dynamic call by string name), **When** it is ported, **Then** it is EITHER replaced by a typed walk over `Light3D` descendants calling `Light3D::set_shadow(false)` (the `shadow_enabled` property) with the same observable effect, OR kept as-is and listed in this spec as a residual dynamic-access case with justification — one of the two, decided at plan time, but not silently left unaddressed.
7. **Given** `#[constant] GI_TYPE_*` / `GI_QUALITY_*` and `#[var] metalfx_supported`, **When** US1 lands, **Then** they are removed after confirming (and recording in the plan) that no remaining `.gd` and no `.tscn` in the project references them by name (the 5 consumers never used them). `#[var] config_file` and the `#[func]` exposure of `load_settings` / `save_settings` / `apply_graphics_settings` are KEPT in US1 (the consumers still call them by name) and removed in the LAST commit of User Story 2, once nothing calls them by name.
8. **Given** Principle III's interface/implementation split, **When** the module is laid out, **Then** `impl INode for Settings` contains only the lifecycle callbacks (`init`, `ready`, `input`) and each delegates to the model or the glue; `#[godot_api] impl Settings` contains only what still must be exposed to the engine/scenes (possibly empty); the typed model, its defaults, its wire conversion and the pure apply-decision function live in plain `impl`/free functions/a submodule with no macro and no engine type; the engine-applying method is a clearly separated glue section. `CONFIG_FILE_PATH` stays a module `const` (it is a path); every other constant that is a tuning/domain value belongs to the model or the relevant type.
9. **Given** a hand-edited `settings.ini` with a value outside the known set for an enum-backed key (e.g. `gi_type = 7`), **When** it is loaded, **Then** that key resolves to its DEFAULT and a `godot_warn!` is emitted — this is the ONE deliberate behavior deviation from `v1` (which fell through to whichever `else` branch happened to match) and it is recorded as a new one-entry item in `docs/v2-backlog.md` (origin `menu/settings.gd` / `settings.rs`, motivation "parse, don't validate") and marked closed by this spec in the same commit.

---

### User Story 2 - The 5 consumers move to typed access (Priority: P2)

Every consumer that today reaches `/root/Settings` dynamically instead resolves a typed `Gd<Settings>` once and reads/writes typed fields or calls typed methods, with the same observable behavior as `v1` in every other respect.

**Why this priority**: Depends entirely on User Story 1's typed API existing first (infrastructure-first order, constitution Principle II); closes backlog item #1, the single largest source of untyped, unchecked-at-compile-time code left in the crate.

**Independent Test**: headless `main.tscn` / `menu.tscn` / `level.tscn` with no new errors; `grep -rn 'get_node_as::<Node>("/root/Settings")\|\.call("apply_graphics_settings\|\.call("save_settings' oxide_godot_core/oxide_godot_lib/src` returns nothing; in the running game, SDFGI/VoxelGI/LightmapGI each render as before and turning shadows off in the menu still turns them off in the level.

**Acceptance Scenarios**:

1. **Given** each of the 5 consumer modules, **When** it needs `Settings`, **Then** it resolves `Gd<Settings>` exactly ONCE — at `ready` (e.g. `OnReady`; `init` cannot, the node is not in the tree yet), including `bullet.rs`, whose access today happens inside the per-event `explode` — never re-resolved per frame or per event.
2. **Given** `level.rs`'s three `if gi_type ==` chains and its 5 local constants `SDFGI` / `VOXEL_GI` / `GI_DISABLED` / `GI_LOW` / `GI_HIGH`, **When** it moves to typed access, **Then** the constants are removed and the chains become `match` on the typed `GiType`/`GiQuality` enums, with no other change to `level.rs`'s behavior (its pure `gi_plan` extraction is out of scope, deferred to V2-E).
3. **Given** `menu.rs`'s 15 reads in `_on_settings_pressed` and its ~52 `set_value` writes in `_on_apply_pressed`, **When** it moves to typed access, **Then** each read/write goes through the typed model (read a copy, or mutate through `bind_mut`) instead of `ConfigFile::get_value`/`set_value`; the button `if`/`else if` chains that decide WHICH value to read/write stay exactly as they are (backlog #22, deferred to V2-E — this story only changes how the chosen value reaches/leaves the model); the local `const SCALING_3D_MODE_NEAREST` in `menu.rs` is removed and replaced by setting `ScaleFilter::Nearest` on the typed model.
4. **Given** `flying_forklift.rs`, `bullet.rs` and `main_scene.rs`, **When** they move to typed access, **Then** only the access path changes (dynamic lookup → typed field read); no other line of behavior changes.
5. **Given** the crate after this story, **When** it is searched for `get_node_as::<Node>("/root/Settings")`, `.get("config_file")` or `.call("apply_graphics_settings"/"save_settings", …)`, **Then** ZERO matches remain — this grep is the acceptance check for backlog item #1's closure — and, in the same last commit, the compatibility surface of `Settings` (`#[var] config_file`, `#[func]` ×3) is removed and `ready` becomes the single parse point (FR-002, FR-007).
6. **Given** the residual dynamic accesses of the touched modules (`bullet.rs` `has_method("hit")` and `.rpc("explode"/"hit")`; `menu.rs:214` and `main_scene.rs:59` `call_deferred` by name; `main_scene.rs` `has_signal`/`Callable::from_object_method`), **When** this story completes, **Then** they are left untouched (out of scope here) and are listed at the top of this spec with their removal milestone (V2-C for `has_method`, V2-E for the rest) or marked permanent (`.rpc` by name, gdext limitation), per Principle II's residual-case rule.
7. **Given** `menu.rs`'s button chains and `level.rs`'s own `gi_plan` extraction are unchanged by this story, **When** a reviewer checks Principle III compliance, **Then** the review does not reject this story for those chains — Principle III applies in full to `settings.rs` in User Story 1 and to each consumer's OWN remaining glue/logic split in that consumer's later milestone (V2-B for `flying_forklift`'s pure logic, V2-C/D/E for the others), not here.

---

### User Story 3 - Gates, API-gap catalog and operational docs (Priority: P3)

The crate's `cargo build` / `cargo clippy` / `cargo test` gates are all green with no behavior change, the one known engine API gap is catalogued per the constitution's protocol, and the operational guide and backlog/README reflect the milestone's outcome.

**Why this priority**: Lowest risk, no observable-behavior surface, but required before this milestone (or any later v2 milestone) can be declared done — the constitution makes all three gates mandatory from v2 onward.

**Independent Test**: `cargo clippy` and `cargo test` both exit clean on the `v2` branch; `docs/api-gaps.md` exists with exactly one entry; `CLAUDE.md`'s work cycle section mentions `cargo clippy` and `cargo test`.

**Acceptance Scenarios**:

1. **Given** the 9 current `cargo clippy` warnings (`blast.rs:30`, `bullet.rs:50`, `red_robot.rs:325,391`, `menu.rs:28,213,379`, `main_scene.rs:23`, `settings.rs:31`), **When** they are fixed, **Then** `cargo clippy` reports zero warnings crate-wide with no change to observable behavior.
2. **Given** `Scaling3DMode::NEAREST` is absent from the gdext 0.5.5 prebuilt API (Godot 4.6) but present in the local Godot 4.7.2, **When** the gap is catalogued, **Then** `docs/api-gaps.md` is created with one entry (symbol, introducing version, workaround, location) and the workaround stays isolated behind `ScaleFilter`'s typed conversion, marked at the exact spot with the constitution's `// api-gap(godot-4.7): Scaling3DMode::NEAREST — …; replace when the binding ships it` comment. The crate's API feature set (`api-custom`, `api-4-x`) is NOT changed — that remains a user decision, recorded as such.
3. **Given** `CLAUDE.md`'s current work cycle (build only), **When** it is updated, **Then** it gains `cargo clippy` and `cargo test` as mandatory steps after `cargo build`, a short note on the Principle III module layout, the `// api-gap(...)` comment convention with a pointer to `docs/api-gaps.md`, and a note on using a `v1` worktree for the parity harness — with no rule content added (rules stay in the constitution).
4. **Given** `docs/v2-backlog.md`, **When** this milestone closes, **Then** item #1 is marked done (with the closing commit), the new malformed-values item (User Story 1, scenario 9) is added and marked done, and items #22, #23, #25 remain open with a note pointing at their target milestone (V2-E, V2-E, and "parity first" respectively).
5. **Given** `README.md`'s Versions table, **When** this milestone lands, **Then** the `v2` row reads "In progress" instead of its previous state.

### Edge Cases

- A `settings.ini` with an unknown enum-coded value for a known key → default + `godot_warn!` (User Story 1, scenario 9; the one deviation).
- No `settings.ini` present at boot → defaults in memory, no file created, matching `v1` exactly (User Story 1, scenario 2).
- `bullet.rs` reads settings inside a per-event handler (`explode`), not per-frame — Principle III's "resolve once" still applies: the `Gd<Settings>` handle is resolved once at `ready`, not re-resolved inside `explode` on every explosion.
- The two SSAO/SSIL naming quirks (backlog #25) and the existing upstream bug fix (SSAO `-1` handling) are behavior, not structure — this milestone MUST reproduce them exactly, not "fix" or "clean up" either while restructuring the surrounding code.
- Headless intermittent errors on `main.tscn` (documented in `CLAUDE.md` as a known race between level loading and the dummy renderer) are not regressions if they disappear on a second run — unchanged by this milestone.
- The shadow-disabling dynamic call by name (User Story 1, scenario 6) may end this milestone either replaced by a typed walk or listed as a residual case — both are acceptable outcomes of this spec; which one is a plan decision.

## Requirements *(mandatory)*

### Functional Requirements

**Typed model (US1)**

- **FR-001**: `Settings` MUST expose its 15 graphics options through one typed model (project-chosen name) instead of raw `ConfigFile` lookups, using engine enums where gdext already provides them and project-defined enums for `gi_type`, `gi_quality`, `ssao_quality` (three states), `ssil_quality` (three states) and `scale_filter`.
- **FR-002**: The typed model MUST be loaded from `user://settings.ini` at `ready` with the same default-merge semantics as `v1` (missing keys filled from defaults in memory; the file is NOT created by this read). During US1 (transitional contract) `apply_graphics_settings` / `save_settings` MUST re-parse the model from `config_file` at their boundary; from the last commit of US2 on, the file MUST be parsed exactly ONCE, at `ready`.
- **FR-003**: The wire format (sections, keys, key order, value types and integer codes for enums) MUST be preserved exactly as in `v1`, so files are interchangeable in both directions between `v1` and this milestone's build.
- **FR-004**: The typed⇄wire conversion and the defaults (including the `metalfx_supported`-dependent default of `scale_filter`) MUST be pure Rust (no `Gd<T>`, `Variant`, `GString`, or other engine-backed builtin) and MUST be covered by `cargo test`: at minimum, a round-trip test per field, a defaults test, a partial-file-merge test, and an SSAO/SSIL-mapping test (≥ 8 tests total).
- **FR-005**: `apply_graphics_settings` MUST be decomposed into a pure decision step (reads the typed model, returns what to apply) and a glue step (applies the decision to `Window`/`Environment`/`RenderingServer`/the scene root); the combined observable effect MUST be identical to `v1`, including both preserved SSAO/SSIL naming quirks and the existing upstream bug fix comment.
- **FR-006**: The shadow-disabling dynamic call by name MUST be either replaced by a typed equivalent with the same effect (recursive walk, `Light3D::set_shadow(false)`), or explicitly listed in this spec as a residual dynamic-access case with justification.
- **FR-007**: `#[constant] GI_TYPE_*`/`GI_QUALITY_*` and `#[var] metalfx_supported` MUST be removed in US1 once it is confirmed (and recorded) that no remaining `.gd` or `.tscn` references them by name. `#[var] config_file` and the `#[func]` exposure of `load_settings`/`save_settings`/`apply_graphics_settings` MUST be kept through US1 and dropped in the last commit of User Story 2, when nothing calls them by name any more.
- **FR-008**: The module MUST follow Principle III's layout: `impl INode for Settings` = delegating lifecycle callbacks only; `#[godot_api] impl Settings` = only the residual exposed API; the typed model, defaults, wire conversion and pure apply-decision logic = plain Rust with unit tests; the engine-applying method = a clearly separated glue section. `CONFIG_FILE_PATH` stays a module `const`.
- **FR-009**: A wire value outside the known set for an enum-backed key MUST resolve to that key's default and emit a `godot_warn!`; this deviation MUST be recorded as one new entry in `docs/v2-backlog.md` and marked closed by this spec.

**Typed consumers (US2, closes backlog #1)**

- **FR-010**: Each of the 5 consumers MUST resolve `Gd<Settings>` exactly once (at `ready`, never per frame or per event) and read/write typed fields or call typed methods instead of dynamic `.get`/`.call` by name.
- **FR-011**: `level.rs`'s 5 local GI constants MUST be removed and its `if gi_type ==` chains MUST become `match` on the typed enums, with no other behavior change.
- **FR-012**: `menu.rs`'s reads in `_on_settings_pressed` and writes in `_on_apply_pressed` MUST go through the typed model; its button `if`/`else if` chains themselves are OUT OF SCOPE (backlog #22, deferred); its local `SCALING_3D_MODE_NEAREST` constant MUST be removed in favor of setting `ScaleFilter::Nearest` on the model.
- **FR-013**: `flying_forklift.rs`, `bullet.rs` and `main_scene.rs` MUST change only their access path to `Settings`, with no other behavior change.
- **FR-014**: After this story, the crate MUST contain zero occurrences of `get_node_as::<Node>("/root/Settings")`, `.get("config_file")`, `.call("apply_graphics_settings", …)` or `.call("save_settings")` (verifiable by grep), and the last commit of the story MUST remove `Settings`' compatibility surface (`#[var] config_file`, `#[func]` on the three methods) and make `ready` the single parse point.
- **FR-015**: Residual dynamic access NOT removed by this milestone in the touched modules (`bullet.rs:51` `has_method("hit")` → V2-C; `bullet.rs:42,52,56` `.rpc("explode"/"hit")` → permanent, gdext exposes RPC only by name; `menu.rs:214` and `main_scene.rs:59` `call_deferred` by name → V2-E; `main_scene.rs:70-76` `has_signal`/`Callable::from_object_method` → V2-E) MUST be listed in this spec together with the milestone that removes each one or the reason it is permanent.

**Gates and documentation (US3)**

- **FR-016**: `cargo clippy` MUST report zero warnings crate-wide after this milestone, with no change to observable behavior in fixing the 9 current warnings.
- **FR-017**: `cargo test` MUST pass and include the unit tests required by FR-004.
- **FR-018**: `docs/api-gaps.md` MUST be created with exactly one entry for `Scaling3DMode::NEAREST` (symbol, introducing version, workaround, location), and the workaround MUST carry the `// api-gap(godot-4.7): ...` comment at its exact spot, per constitution Principle I. The crate's API feature set MUST NOT be changed as part of this milestone.
- **FR-019**: `CLAUDE.md` MUST gain the `cargo clippy`/`cargo test` steps in the work cycle, a note on the Principle III module layout, the `// api-gap(...)` convention with a pointer to `docs/api-gaps.md`, and a note on the `v1`-worktree parity harness — operational content only, no rule duplicated from the constitution.
- **FR-020**: `docs/v2-backlog.md` MUST have item #1 and the new malformed-values item marked done in the closing commit(s); items #22, #23 and #25 MUST remain open with a note of their target milestone.
- **FR-021**: `README.md`'s Versions table MUST show `v2` as "In progress" after this milestone.

**Verification (all user stories)**

- **FR-022**: Every user story MUST be validated headless (import + `main.tscn` + `menu.tscn` + `level.tscn`, no new errors beyond the documented baseline) before being considered complete.
- **FR-023**: Behavioral parity with a `v1` worktree build MUST be verified by a parity harness for: (a) no-file-at-boot / file-created-on-Apply / byte-identical output for defaults; (b) every menu option, Apply → identical observable state (window mode, vsync, max fps, scaling scale/mode, TAA, MSAA, SSAA, SSAO/SSIL enabled flags, glow, fog, level light shadow flag) on both branches; (c) a hand-edited unknown value → the one documented deviation (default + warning) on this branch only.
- **FR-024**: Each unit of work MUST be one local commit naming what was remodeled (US1: `settings.rs`; US2: one commit per consumer module is acceptable; US3: clippy/docs), never pushed.

### Key Entities

- **Typed graphics-settings model** (name decided at plan time, e.g. `GraphicsSettings`): the 15 settings as typed fields; owns `Default` (with the `metalfx_supported`-dependent `scale_filter` default) and the wire conversion to/from `ConfigFile`; pure, unit-tested.
- **`GiType` / `GiQuality` / SSAO quality / SSIL quality / `ScaleFilter`**: project enums replacing raw ints/sentinels in the model; `ScaleFilter` additionally owns the `Scaling3DMode::NEAREST` API-gap workaround.
- **`Settings` (Node, `/root/Settings`)**: glue node — owns the typed model, the load/save boundary (`ConfigFile`, confined to that boundary), the engine-applying step, and the `toggle_fullscreen` input callback (unchanged).
- **`user://settings.ini`**: the wire file; format unchanged and interchangeable with `v1`, except the one documented malformed-value deviation.
- **`docs/api-gaps.md`**: new catalog, one entry (`Scaling3DMode::NEAREST`).
- **`docs/v2-backlog.md`**: item #1 and the new malformed-values item closed; #22, #23, #25 stay open.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Zero dynamic accesses to `Settings` remain in the crate (`get_node_as::<Node>("/root/Settings")`, `.get("config_file")`, `.call("apply_graphics_settings"/"save_settings", …)` — verified by grep).
- **SC-002**: `cargo build`, `cargo clippy` and `cargo test` all exit clean, with at least 8 unit tests covering round-trip, defaults, partial-file merge and the SSAO/SSIL mapping.
- **SC-003**: `settings.ini` produced by this milestone's build is byte-identical to the one a `v1` build produces, for the defaults and for every individual menu option, per the parity harness.
- **SC-004**: Headless validation (import + `main.tscn` + `menu.tscn` + `level.tscn`) shows zero new errors relative to the documented baseline.
- **SC-005**: `docs/api-gaps.md` exists with exactly 1 entry; `menu.rs` no longer contains a numeric scaling-mode constant.
- **SC-006**: `docs/v2-backlog.md` item #1 is marked done; the milestone's one documented deviation is recorded as a new, closed item in the same file.
- **SC-007**: In a side-by-side session, the user confirms: game boots with defaults, the Settings menu shows them, Apply persists across a restart (after US1); SDFGI/VoxelGI/LightmapGI each look as before and turning shadows off in the menu actually disables shadows in the level (after US2); nothing visually changes after US3.

## Assumptions

- Phase v2 (constitution 1.4.0); the milestone's ONLY behavior deviation from `v1` is the malformed-wire-value fallback (User Story 1, scenario 9); everything else is byte-for-byte / pixel-for-pixel parity.
- The typed model's exact struct/enum names, its exact `Gd<Settings>` resolution mechanism for consumers (a resolved `OnReady` field vs. a helper that walks the tree), the exact submodule layout inside `settings.rs`, and whether the shadow-disabling call is replaced by a typed walk or kept as a justified residual case are all plan-time decisions, not fixed by this spec.
- The parity harness runs against a separate `v1` git worktree, built independently; it is a validation tool for this milestone, not a crate dependency or a build-time reference from `v2`.
- Visual checkpoints (SC-007) are confirmed by the user; automated validation is headless plus the parity harness.
- Out of scope for this milestone: `menu.rs`'s button `if`/`else if` chains (backlog #22), the typed scene-manager connections in `main_scene.rs` (backlog #23), fixing the SSAO/SSIL naming quirk (backlog #25), `bullet.rs`'s `Hittable` dispatch, and any pure-logic extraction in the 5 consumers beyond the access path itself (each belongs to that consumer's own later milestone, per `docs/v2-catalog.md`'s suggested milestones V2-B…E).
