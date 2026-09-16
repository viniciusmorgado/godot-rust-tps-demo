# Feature Specification: Milestone E — Settings autoload (v1 raw port, last script)

**Feature Branch**: `005-v1-settings-autoload` (work on `main`, as in Milestones A–D)

**Created**: 2026-09-16

**Status**: Draft

**Phase**: v1 — Raw Port (Principle I of constitution v1.3.1). No abstraction, refactoring or optimization; noticed improvements go to `docs/v2-backlog.md`. No upstream bug is known in `settings.gd`; two **quirks** (SSAO, see Edge Cases) are preserved and recorded in the backlog — they are not a fix. If an objective defect surfaces during the port, the conservative fix clause applies (declare in the spec, isolate with a comment, mention in the commit, record in `docs/upstream-bugs.md`) — otherwise, nothing changes.

**Input**: User description: "Port the Settings autoload (menu/settings.gd, 107 l., Node) to Rust, completing v1 with zero .gd in the project and the whole game in Rust. Special autoload binding: minimal scene menu/settings.tscn + swap of the project.godot line. The 5 Rust consumers (bullet, flying_forklift, level, menu, main_scene) keep accessing /root/Settings dynamically without editing. Milestone E, item 15 of docs/port-order.md."

## Context

Last milestone of v1. Milestones A–D (`specs/001`–`004`, commits up to `6f2fc7f`) ported 14 of the 15 scripts; `menu/settings.gd` is the **only** `.gd` in the project (with its `.gd.uid`). It is the `Settings` autoload (`project.godot:25`, `[autoload] Settings="*res://menu/settings.gd"`), accessible at `/root/Settings`, and is not attached to any scene.

Consumers: **no GDScript remains**. The 5 Rust modules access the autoload **dynamically** (documented exception of Principle II, constitution l.79; v2 backlog item 1) in 10 `get_node_as("/root/Settings")` (12 accesses): `flying_forklift.rs:19` (`get("config_file")`), `bullet.rs:72` (`get("config_file")`), `level.rs:43,116,144,173` (`call("apply_graphics_settings", …)` + 4× `get("config_file")`), `menu.rs:208,308,479` (`call("apply_graphics_settings", …)` ×2, `get("config_file")` ×2, `call("save_settings")`), `main_scene.rs:29` (`get("config_file")`). These accesses MUST keep resolving by name against the new class, **without editing the 5 modules**. Typed access (`Gd<Settings>`) is left for v2 (backlog item 1) — the exception closes in the new code (the class is born in Rust), not in the consumers.

Class name checked (`CLAUDE.md` rule): `Settings` is not an engine class (there are `ProjectSettings`, `EditorSettings`, `LabelSettings` — distinct names) and there is no remaining `.gd` besides the script itself, which is to be deleted.

| # | Original script | Base | Binding today | Binding after the port | Lines | Consumers |
|---|---|---|---|---|---|---|
| 1 | `menu/settings.gd` | `Node` | autoload by **script** (`project.godot:25`) | autoload by **scene**: new `menu/settings.tscn` (root `[node name="Settings" type="Settings"]`) + `project.godot:25` → `Settings="*res://menu/settings.tscn"` | 107 | 5 Rust modules, dynamic, untouched |

Behavior reference: the untouched original project in `../oxide_godot_origins/` (`menu/settings.gd` identical to the repo's).

### Binding by type of an autoload (rule declared in this spec)

Principle II requires binding by `type` swap in the scene, without a bridge `.gd`. An autoload is registered as **script or scene**; a native class cannot be a direct autoload. The exact equivalent of the "`type` swap" for an autoload is: (1) create a minimal scene whose root has `type` = ported class and `name` = autoload key; (2) point the `[autoload]` entry to that scene; (3) delete `.gd` + `.gd.uid` in the same commit. The node remains at `/root/Settings` (the name comes from the autoload key, not from the scene), with `*` (singleton enabled) preserved. This is the **first and only edit of `project.godot` in v1** — exactly one line; nothing else in the file changes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Settings autoload in Rust: loading and persistence (Priority: P1)

The game keeps reading and writing the settings exactly as before: on startup, the autoload loads `user://settings.ini` (if it exists) and fills in memory whatever is missing with the defaults, so that no consumer needs a default value; Apply in the menu writes the file. The five already-ported modules keep working without any change.

**Why this priority**: It is the autoload's contract (property `config_file`, methods `load_settings`/`save_settings`) on which the whole game depends; without it nothing else works. Closes v1: zero `.gd`.

**Independent Test**: `main.tscn` headless (full boot: autoload in scene → `Main` reads `display_mode` → `Menu` applies → automatic host → `Level` applies) without new errors; `menu.tscn` and `level.tscn` headless likewise; in the game, with `user://settings.ini` removed, boot happens with the defaults and the Settings menu shows the defaults; after Apply the file exists with the same names, order and values as the original; restarting the game preserves the choices.

**Acceptance Scenarios**:

1. **Given** `project.godot` registers `Settings="*res://menu/settings.tscn"` and the scene has only the root `Settings` of the ported type, **When** the game starts, **Then** `/root/Settings` exists, is the ported class, and the 10 dynamic accesses of the 5 modules (`get("config_file")`, `call("apply_graphics_settings", window, environment, root)`, `call("save_settings")`) resolve without error (`Invalid get`/`Invalid call` absent in headless).
2. **Given** the autoload enters the tree, **When** `ready` runs, **Then** `load_settings` runs: `config_file.load("user://settings.ini")` (error ignored if the file does not exist) and, for each (section, key) of the defaults that does **not** exist in the file, the default is written **in memory** — in the order of the original: `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) and `rendering` (`taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `gi_type`, `gi_quality`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`).
3. **Given** `user://settings.ini` does not exist, **When** the game starts, **Then** **no file is created** by the load (the original does not save in `load_settings`); the file only comes into existence after `save_settings` (Apply in the menu) — and then contains the 15 keys with the defaults (or the chosen values), with the same types as the original (`max_fps` integer `0`, `resolution_scale` real `1.0`, booleans, enum integers).
4. **Given** the file exists with partial values (missing keys), **When** `load_settings` runs, **Then** the present keys are preserved and only the missing ones receive a default.
5. **Given** the defaults: `video/display_mode` = exclusive fullscreen; `vsync` = enabled; `max_fps` = 0; `resolution_scale` = 1.0; `scale_filter` = MetalFX temporal if `metalfx_supported` else FSR2; `rendering/taa` = false; `msaa` = disabled; `screen_space_aa` = disabled; `shadow_mapping` = true; `gi_type` = VoxelGI (1); `gi_quality` = low (1); `ssao_quality` = medium; `ssil_quality` = −1; `bloom` = true; `volumetric_fog` = true, **When** written/read, **Then** they are the **same integers/booleans/reals** that the original writes — the `settings.ini` written by the original `settings.gd` is read by the port without change and vice versa (user file compatibility).
6. **Given** `metalfx_supported`, **When** the autoload is constructed, **Then** it equals (current rendering driver name == "metal") — false on this platform; the default of `scale_filter` is FSR2.
7. **Given** `save_settings` is called by the menu (dynamic), **When** it runs, **Then** `config_file.save("user://settings.ini")` writes the current state.
8. **Given** `config_file` is an exposed property with that name, **When** a consumer does `get("config_file")`, **Then** it receives the **same** object the autoload uses internally (menu changes via `set_value` are seen by `save_settings` and `apply_graphics_settings`).
9. **Given** `settings.gd` and `settings.gd.uid` are deleted in the commit, **When** the project is imported, **Then** no reference to `res://menu/settings.gd` or to the uid `uid://b04fekxdgdq0k` remains outside `.godot/`; the project has **zero** `.gd` and **zero** `.gd.uid`.

---

### User Story 2 - Applying the graphics settings (Priority: P2)

On entering the menu or the level, the settings are applied exactly as before: window mode, vsync, fps limit, resolution scale and filter, TAA, MSAA, screen-space AA, shadows (disabled on all lights of the scene when the option is off), SSAO, SSIL, bloom and volumetric fog — including the original's two SSAO quirks.

**Why this priority**: It is the 3-argument method called dynamically by `Level` and `Menu`; it determines the look of the game and is where fidelity is most visible.

**Independent Test**: `level.tscn`/`menu.tscn`/`main.tscn` headless without error in the 3-argument call; in the game, toggling each option in Settings → Apply produces the same visual effect as in the original (side by side), including disabled shadows and SSAO/SSIL.

**Acceptance Scenarios**:

1. **Given** `apply_graphics_settings(window, environment, scene_root)` is called by name with 3 arguments, **When** it runs, **Then** the window mode receives `video/display_mode` — applied to the **autoload's own window** (`get_window()`), not to the `window` parameter (preserve); the display server's vsync mode receives `video/vsync`; the engine's fps limit receives `video/max_fps`.
2. **Given** the `window` parameter, **When** it runs, **Then** `scaling_3d_scale` = `video/resolution_scale`, `scaling_3d_mode` = `video/scale_filter`, `use_taa` = `rendering/taa`, `msaa_3d` = `rendering/msaa`, `screen_space_aa` = `rendering/screen_space_aa`.
3. **Given** `rendering/shadow_mapping` false, **When** it runs, **Then** `scene_root.propagate_call("set", ["shadow_enabled", false])` disables the shadow of all lights in the scene (base engine API; call by name that the original already does); with true nothing is done (shadows are **not** re-enabled — the original's `FIXME` limitation, preserved).
4. **Given** `rendering/ssao_quality`, **When** it runs, **Then** if it is −1 → `environment.ssao_enabled = false` **and nothing else** (conservative upstream bug fix, FR-020–FR-024: in the original the following `if` is not `elif` and re-enabled SSAO); else, if it is medium → `ssao_enabled = true` and the rendering server receives **high** quality, `half_size` false, 0.5, 2, 50, 300 (**quirk preserved**: "medium" applies high — ambiguous intent, v2 backlog); else → `ssao_enabled = true` and **medium** quality, `half_size` true, 0.5, 2, 50, 300.
5. **Given** `rendering/ssil_quality` (`if`/`elif`/`else`, correct): **When** it runs, **Then** −1 → `ssil_enabled = false`; medium → `true` + medium quality, `half_size` false, 0.5, 2, 50, 300; else → `true` + high quality, `half_size` true, 0.5, 2, 50, 300.
6. **Given** `rendering/bloom` and `rendering/volumetric_fog`, **When** it runs, **Then** `environment.glow_enabled` and `environment.volumetric_fog_enabled` receive the booleans.
7. **Given** the original reads the values through `Settings.config_file` (the autoload itself via global name), **When** the port reads through its own `config_file` property, **Then** it is the same object — nothing observable changes.
8. **Given** the menu calls `apply_graphics_settings` twice (on entering and on Apply) and the level once, **When** they occur, **Then** each call applies the current state of `config_file` (no cache).

---

### User Story 3 - Toggling fullscreen (Priority: P3)

F11 or Alt+Enter toggle between exclusive fullscreen and windowed at any point in the game, as before.

**Why this priority**: The only input handled by the autoload; independent of the rest, verifiable in seconds by the user.

**Independent Test**: In the game, F11 toggles exclusive fullscreen ↔ windowed; Alt+Enter likewise; in headless it is not exercised (no window).

**Acceptance Scenarios**:

1. **Given** the `toggle_fullscreen` action (`project.godot`, F11 and Alt+Enter — untouched) is pressed, **When** `_input` receives the event, **Then** if the window is **not** in fullscreen nor in exclusive fullscreen → mode = exclusive fullscreen; else → mode = windowed; and the event is marked as handled in the viewport.
2. **Given** any other event, **When** `_input` receives it, **Then** nothing happens.

---

### Edge Cases

- **Autoload → scene**: the only way to bind a native class as an autoload; `settings.tscn` has exactly one node (`name="Settings"`, `type="Settings"`), no script, no `ext_resource`. The name at `/root/Settings` comes from the autoload key. `project.godot` changes in **one** line (`:25`); `run/main_scene`, input actions and everything else stay untouched (checked by diff).
- **Autoload scene run in isolation**: `godot --path . menu/settings.tscn` would instantiate the autoload **and** the scene (two `Settings` nodes under `/root`, one renamed by the engine) — it is not a game scenario; headless validation uses `main.tscn`, `menu.tscn`, `level.tscn`, never `settings.tscn` alone.
- **`GIType`/`GIQuality` enums**: they were consumed as `Settings.GIType.X` by already-ported scripts that today use integer literals (0/1/2). No consumer reads them by name; the spec **does not require** exposing them — exposing them as integer constants of the class (with the same values) or keeping only the integers in the defaults is the plan's decision. The written values are 0/1/2.
- **`load_settings` does not save**: the file is born only on the first `save_settings` (Apply). In headless, `main.tscn` never presses Apply → boot does not create the file (original behavior). Not a defect.
- **`config_file.load` on a missing file**: the return error is ignored (like the original); no engine error message is expected (`ConfigFile.load` does not print an error for a nonexistent file).
- **Conditional defaults**: `scale_filter` depends on `metalfx_supported`, evaluated once at the construction of the autoload (driver name == "metal"). On this platform: FSR2.
- **Order and types in the `.ini`**: the order of sections/keys in the written file follows the insertion order of the defaults (the original's dictionary) — the file written by the port MUST have the same order and the same types (`0` integer, `1.0` real, `true/false`, enum integers) as the original, so that the user's existing `settings.ini` remains valid and byte-for-byte identical after an Apply without changes.
- **`apply_graphics_settings`: own window vs. parameter** — the window mode goes to the autoload's `get_window()` (the root window), and scale/filter/TAA/MSAA/AA go to the `window` parameter (in practice the same root window). Preserve the distinction.
- **SSAO — bug and quirk** (US2 scenario 4): (1) `if == -1` followed by `if == medium / else` **without `elif`** → choosing "SSAO: Disabled" in the menu does not turn SSAO off (the `else` re-enables it at medium + `half_size`). Unambiguous intent (turn off) contradicted by the result, and the SSIL block right below — identical in structure — uses `elif` correctly: it is a **bug**, fixed under the constitution's clause (FR-020–FR-024). (2) "medium" applies **high** quality without `half_size`, and the `else` applies medium with `half_size`: ambiguous intent (may be a deliberate swap of names) → **preserved**, v2 backlog (item 25).
- **The original's `FIXME` (shadows not re-enabled in the menu)**: upstream comment at `settings.gd:83-85`; known limitation, preserved (not an objective defect of this feature).
- **`propagate_call("set", …)`**: call by name to the base engine API (not to a custom script) — allowed, it is what the original does.
- **Headless**: window mode, vsync and fullscreen have no effect on the headless display server; the calls MUST occur without error.
- **Self-reference `Settings.config_file`**: inside `apply_graphics_settings` the original uses the autoload's global name to read its own `config_file`; the port uses its own property — same object.

## Requirements *(mandatory)*

### Functional Requirements

**Behavior (US1)**

- **FR-001**: The autoload MUST become exactly one native class `Settings` with base `Node`, containing: constant `CONFIG_FILE_PATH = "user://settings.ini"`; state `metalfx_supported` (current rendering driver name == "metal", evaluated at construction); the defaults of `settings.gd:20-40` (same sections, keys, order, types and values); and the **exposed** property `config_file` (a `ConfigFile` created at construction).
- **FR-002**: On entering the tree it MUST call `load_settings`.
- **FR-003**: `load_settings` MUST be exposed with that name and reproduce `settings.gd:55-62`: load `CONFIG_FILE_PATH` ignoring the return error; for each (section, key) of the defaults missing from `config_file`, write the default in memory; MUST NOT save the file.
- **FR-004**: `save_settings` MUST be exposed with that name and save the `config_file` to `CONFIG_FILE_PATH`.
- **FR-005**: The values of the defaults MUST be the same integers/booleans/reals as the original (engine enums for window mode, vsync, scale filter, MSAA, screen-space AA, SSAO quality; `GIType`/`GIQuality` as 1/1; `−1` for SSIL; `0` for fps; `1.0` for scale), so that a `settings.ini` written by the original is read by the port and vice versa without changing values.

**Behavior (US2)**

- **FR-006**: `apply_graphics_settings(window, environment, scene_root)` MUST be exposed with that name and that arity (3 arguments, dynamic call from `level.rs:44` and `menu.rs:208,606`) and reproduce `settings.gd:69-107` line by line: mode of the **own** window (`get_window()`) = `video/display_mode`; display server vsync = `video/vsync`; engine max fps = `video/max_fps`; on the `window` parameter: `scaling_3d_scale`, `scaling_3d_mode`, `use_taa`, `msaa_3d`, `screen_space_aa`; if `rendering/shadow_mapping` false → `scene_root.propagate_call("set", ["shadow_enabled", false])`.
- **FR-007**: SSAO MUST reproduce `settings.gd:88-95` with the structure `if` / `else if` / `else` (−1 → turns off and nothing else — fix FR-020–FR-024; medium → high with `half_size` false — quirk preserved; else → medium with `half_size` true; parameters 0.5, 2, 50, 300). SSIL MUST reproduce `settings.gd:97-104` (`if`/`elif`/`else`: −1 turns off; medium → medium without `half_size`; else high with `half_size`; same parameters). Then `glow_enabled` = `rendering/bloom` and `volumetric_fog_enabled` = `rendering/volumetric_fog`.
- **FR-008**: The values read from `config_file` MUST be interpreted as in the original (enum integers converted to the corresponding engine enums; booleans; real for the scale).

**Behavior (US3)**

- **FR-009**: `_input` MUST, when the `toggle_fullscreen` action is pressed, put the window in exclusive fullscreen if it is neither in fullscreen nor in exclusive fullscreen, else in windowed mode, and mark the input as handled in the viewport (`settings.gd:49-52`).

**Port cycle (Principle II adapted to the autoload)**

- **FR-010**: The name `Settings` MUST be checked against engine classes and top-level identifiers of the remaining `.gd`s (`CLAUDE.md` rule) before the port; collision → stop.
- **FR-011**: The binding MUST be: new scene `oxide-godot/menu/settings.tscn` with a **single** root node `[node name="Settings" type="Settings"]` (no script, no `ext_resource`), and the line `project.godot:25` changed to `Settings="*res://menu/settings.tscn"` (singleton `*` preserved). No other line of `project.godot` changes; no bridge `.gd`.
- **FR-012**: `settings.gd` and `settings.gd.uid` MUST be deleted in the same commit; no reference to `res://menu/settings.gd` nor to `uid://b04fekxdgdq0k` remains outside `.godot/`.
- **FR-013**: Names MUST be identical to the GDScript ones: property `config_file`; methods `load_settings`, `save_settings`, `apply_graphics_settings`; action `toggle_fullscreen` handled in `_input`. Checked against the 10 `get_node_as("/root/Settings")` (12 accesses) access sites of the 5 Rust modules.
- **FR-014**: The 5 consumer modules (`bullet.rs`, `flying_forklift.rs`, `level.rs`, `menu.rs`, `main_scene.rs`) and all the other existing modules MUST remain byte-for-byte intact; the commit touches only the new module, `lib.rs` (module registration), `settings.tscn` (new), `project.godot` (1 line), the two deletions and `docs/v2-backlog.md`.
- **FR-015**: The ported code MUST NOT call custom GDScript API (none remains); `propagate_call("set", …)` is base engine API and is allowed because it is what the original does.
- **FR-016**: Each code change MUST be followed by a debug build without new warnings.
- **FR-017**: The port MUST be validated headless: import with extension loading + `main.tscn` (full boot with the autoload in scene), `menu.tscn` and `level.tscn` without new errors beyond the baseline (`CLAUDE.md`: 3 known import errors; intermittent error rule of `main.tscn`); `settings.tscn` MUST NOT be run in isolation.
- **FR-018**: A single `Port …` commit on `main`; the game MUST be playable after it; noticed improvements (SSAO quirk 2 and whatever else surfaces) MUST go to `docs/v2-backlog.md` in the same commit; the only bug fix is the SSAO one (FR-020–FR-024; `docs/upstream-bugs.md` goes to 3 entries).
- **FR-019**: `CLAUDE.md` MAY gain only the operational note about the autoload binding by scene (if the plan deems it useful); no other documentation edit.

**Conservative upstream bug fix — SSAO disabled (Principle I, v1.3.1)**

- **FR-020**: The defect: `settings.gd:88-95` does `if ssao_quality == -1: ssao_enabled = false` and, on the next line, `if ssao_quality == MEDIUM: … else: ssao_enabled = true …` — without `elif`. Result: with "SSAO: Disabled" (−1) chosen in the menu, SSAO is turned off and immediately re-enabled at medium quality with `half_size`; the option has no effect. Unambiguous intent (the first line turns it off; the identical SSIL block at `settings.gd:97-104` uses `elif`) contradicted by the result → **bug**.
- **FR-021**: The fix MUST be minimal: the second test becomes `else if` (equivalent to the SSIL `elif`). Nothing else changes: the branches "medium → high without `half_size`" and "otherwise → medium with `half_size`" stay as they are (quirk 2 preserved), identical parameters, SSIL intact. It is FORBIDDEN to restructure the method, unify SSAO/SSIL or "fix" quirk 2.
- **FR-022**: The fix MUST be isolated and identifiable, with the comment `// upstream bug fix: ...` on the line immediately above the `else if`, inside the port commit (it is the milestone's only commit), whose message mentions the fix.
- **FR-023**: `docs/upstream-bugs.md` MUST receive entry #3 (defect, script/scene, fix applied, commit by subject) in the same commit.
- **FR-024**: Expected result: with `rendering/ssao_quality = -1` in `config_file`, after `apply_graphics_settings` the `environment.ssao_enabled` is **false** (in the original: true); the other values produce the same result as the original.

### Key Entities

- **Settings (autoload, `/root/Settings`)**: exposed property `config_file`; exposed methods `load_settings()`, `save_settings()`, `apply_graphics_settings(window, environment, scene_root)`; state `metalfx_supported`; defaults (15 keys in 2 sections); handles `toggle_fullscreen`. Consumed dynamically by 5 Rust modules (10 `get_node_as("/root/Settings")` (12 accesses)).
- **Settings file (`user://settings.ini`)**: sections `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) and `rendering` (`taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `gi_type`, `gi_quality`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`); created only by `save_settings`; compatible in both directions with the original.
- **Autoload binding**: `menu/settings.tscn` (1 node) + `project.godot:25`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the milestone the project contains **zero** `.gd` files and **zero** `.gd.uid` (outside `addons/`); `project.godot` differs from the previous state in exactly 1 line.
- **SC-002**: The full game — boot → menu (Settings with the 15 rows reflecting and writing each option; Apply applies and persists; Cancel discards; reopening and restarting the game preserve) → loading → level (graphics effects according to each option, including disabled shadows and SSAO/SSIL) → ESC → menu; F11/Alt+Enter toggle fullscreen — is indistinguishable from the original in `../oxide_godot_origins/` in a side-by-side comparison done by the user.
- **SC-003**: A `settings.ini` written by the original is read by the port without change; an Apply without changes in the port rewrites the file identically (same keys, order, types and values).
- **SC-004**: Headless validation (import + `main.tscn` + `menu.tscn` + `level.tscn`) with zero new errors; `main.tscn` goes through boot → menu → automatic host → level with the autoload in Rust.
- **SC-005**: The 5 consumer modules are byte-for-byte equal to the previous commit; the milestone's history has exactly 1 `Port …` commit.
- **SC-006**: Debug build with no warnings at all.
- **SC-007**: No ported line introduces abstraction, refactoring or optimization; the only fix is the SSAO-disabled one (FR-020–FR-024, 4/4 requirements of the clause); `docs/upstream-bugs.md` goes to 3 entries; `docs/v2-backlog.md` gains SSAO quirk 2 (≥ 1 new item).
- **SC-008**: With the user's OK, v1 is **declared complete** (Principle I: no remaining `.gd` script and the game playable end to end). Tag and branch `v2` are separate steps, decided by the user.

## Assumptions

- Phase v1 (constitution v1.3.1); one conservative bug fix (SSAO disabled, FR-020–FR-024); SSAO quirk 2 is preserved and goes to the backlog.
- Visual validation (SC-002, F11, persistence across runs) by the user; automated validation exclusively headless. The reviewer may run a parity harness outside the repository.
- The binding by scene is the faithful reading of Principle II for autoloads (rule declared in Context); the `project.godot` edit is the equivalent of the `type` swap.
- Whether or not to expose the `GIType`/`GIQuality` enums as class constants is the plan's decision (no consumer reads them by name).
- Rust module name (e.g. `settings.rs`) is the plan's decision; the class is `Settings`.
- `metalfx_supported` is false on the validation platform (driver ≠ "metal"); the MetalFX branch is verified by code reading.
- Out of scope: typed access to `Settings` by the consumers (backlog 1); any improvement; multiplayer; fixing the SSAO quirks; tag/branch v2.
