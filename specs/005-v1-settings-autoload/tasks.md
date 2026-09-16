# Tasks: Milestone E — Settings autoload (v1 raw port, last script)

**Input**: Design documents from `/specs/005-v1-settings-autoload/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D10, §E), data-model.md, contracts/settings.md, quickstart.md (all approved, commit `619575d`)

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.1). No task may introduce
abstraction, refactoring, optimization, unit tests or infrastructure. If something like that seems
necessary, it becomes an entry in `docs/v2-backlog.md`, not a task. **One conservative upstream bug
fix** is declared in the spec (FR-020–FR-024: SSAO "Disabled" did not turn off — `if` instead
of `elif` at `settings.gd:90`) and goes into the milestone's **single** commit with the 4 requirements of the clause
(spec ✓, comment `// upstream bug fix` at the exact spot, commit message,
`docs/upstream-bugs.md` #3). Any **other** objective defect → STOP and report (requires a spec
before any commit). No crate change (`Cargo.toml` untouched); `CLAUDE.md` untouched
(plan, FR-019).

**Tests**: there are no automated tests in this phase. Validation is the quickstart cycle (build →
headless import → headless scenes → disposable probes outside the repo → mechanical checks →
contract → user visual validation).

**Organization**: a single implementation user story — the `Settings` autoload is one script and
goes in a single `Port …` commit. The spec's US2/US3 (applying the settings; F11) are
behaviors of the same script, verified within this phase and at the user checkpoint; they do not
generate separate commits or code tasks. Afterwards, Polish = final verification of the **whole v1**.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: may run in parallel (different files, no dependency on an incomplete task) —
  practically nonexistent here: build depends on the module, binding depends on the build, validation
  depends on the binding.
- **[Story]**: US1 (spec.md — the only implementation story).
- Paths relative to the repository root (`oxide-godot/` = Godot project;
  `oxide_godot_core/oxide_godot_lib/src/` = Rust crate).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` of each module — only gains `mod settings;`
oxide_godot_core/oxide_godot_lib/src/settings.rs   NEW — struct Settings, base=Node (research D2–D8)
oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs   dynamic consumers — NEVER change (FR-014)
oxide_godot_core/Cargo.toml                        NEVER changes
oxide-godot/menu/settings.tscn                     NEW — 3 lines (autoload binding, plan §"Binding edit")
oxide-godot/project.godot                          l.25 only: Settings="*res://menu/settings.gd" → Settings="*res://menu/settings.tscn"
oxide-godot/menu/settings.gd (+ .uid uid://b04fekxdgdq0k)   DELETE (git rm) — last .gd/.uid of the project
docs/upstream-bugs.md                              entry #3 (text in research §"Entry #3")
docs/v2-backlog.md                                 item 25 (text in research §"Candidate v2 backlog")
CLAUDE.md                                          NEVER changes (FR-019 decided in the plan)
../oxide_godot_origins/                            untouched reference of the original GDScript
~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini   = real user://settings.ini (SC-003)
```

Closed decisions (instruction, not option; details in research.md):

- **Class**: `struct Settings`, `#[class(base=Node)]` **without** automatic `init`; manual `fn init(base: Base<Node>) -> Self`
  computing, in this order, `metalfx_supported` → `defaults` → `config_file` (D4).
- **Fields**: `#[var] metalfx_supported: bool` and `#[var] config_file: Gd<ConfigFile>` (**never**
  `#[export]`; `#[var]` on `metalfx_supported` is imposed by the compiler — `never read` warning — and
  reproduces the original's top-level `var`); `defaults: VarDictionary` **private**, nested `vdict!`
  with `&vdict!` on the nested values (E0271 without the `&`) (D3, D5); `const CONFIG_FILE_PATH: &str`
  at module level.
- **Constants**: 6 `#[constant] const …: i64` in the **single** `#[godot_api] impl Settings` block
  (`GI_TYPE_SDFGI/VOXEL_GI/LIGHTMAP_GI` = 0/1/2, `GI_QUALITY_DISABLED/LOW/HIGH` = 0/1/2); the defaults
  use `Self::GI_TYPE_VOXEL_GI` and `Self::GI_QUALITY_LOW` (D6).
- **`load_settings`**: `self.config_file.load(CONFIG_FILE_PATH);` (return ignored, no `let _`);
  `keys_shared()` loop over the defaults in insertion order with `has_section_key` → `set_value`;
  **does not save**. **`save_settings`**: `self.config_file.save(CONFIG_FILE_PATH);` (D5).
- **`input`**: `toggle_fullscreen` → **own** window to `EXCLUSIVE_FULLSCREEN` if the mode is neither
  `FULLSCREEN` nor `EXCLUSIVE_FULLSCREEN`, else `WINDOWED`; `get_viewport().unwrap().set_input_as_handled()` (D7).
- **`apply_graphics_settings(window, environment, scene_root)`**: table D8 line by line; window
  mode on the **own** window (`self.base().get_window()` — quirk); `from_ord(x as i32)` on all
  enums, including `Scaling3DMode::from_ord(5)` (Nearest from 4.7 — **no** special handling,
  **no** dynamic `set`); `propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done()`
  with the upstream `FIXME` comment copied; SSAO `if / else if / else` with the comment
  `// upstream bug fix: settings.gd used if instead of elif — "SSAO: Disabled" (-1) was re-enabled by the else`
  on the line immediately above the `} else if`; "medium applies high" quirk **preserved**; SSIL
  faithful; `set_glow_enabled`/`set_volumetric_fog_enabled`.
- **Binding** (D1): new 3-line scene + 1 line of `project.godot`; `git rm` of the `.gd` + `.uid`.
  `settings.tscn` is **never** run in isolation in headless.
- A single `Port …` commit on `main` (message in T017); fix after the checkpoint = `Fix port …`.
  **Before any commit other than the port one, check that there are no `.gd` deletions in
  staging** (lesson from Milestone C).

---

## Phase 1: Setup

**Purpose**: record the baseline (build, import, 3 scenes, real `settings.ini`) against which the
port is compared. No infrastructure (Principle I).

- [x] T001 Confirm preconditions: no Godot editor open (`pgrep -a godot` empty — if there is one, warn the user and wait, never kill); `git status --short` empty on `main`; note `git rev-parse --short HEAD` (expected `619575d` or later without "Port …" commits — this hash is the base commit of quickstart §8/T020); `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = only `oxide-godot/menu/settings.gd`; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'` = only `oxide-godot/menu/settings.gd.uid`; `grep -n '^Settings=' oxide-godot/project.godot` = `25:Settings="*res://menu/settings.gd"`; `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = { version = "0.5.5", features = ["experimental-threads"] }` (does not change in this milestone)
- [x] T002 Record the build baseline: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → expected `0`; note in `specs/005-v1-settings-autoload/quickstart.md` §1 if it differs
- [x] T003 Record the import baseline: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirm `grep -c 'Initialize godot-rust' /tmp/import.log` = 1 and `grep -E 'ERROR|SCRIPT ERROR' /tmp/import.log | grep -vE 'Cannon_Charge already exists|doorsimple_d.png|surfaces.is_empty'` empty
- [x] T004 [P] Check name and bindings: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | awk '{print $NF}' | sort -u | grep -wx Settings` empty (the only `.gd` is `settings.gd` itself, which does not declare `Settings`); `B=$(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1); ls $B/classes/ | grep -ix 'settings.rs'` empty (only `project_settings.rs`, `editor_settings.rs`, `label_settings.rs`, …); note `$B` (expected `…/godot-core-4eba5d49e15a0d7e/out`). Counts: `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2; `grep -c '^| [0-9]' docs/v2-backlog.md` = 24
- [x] T005 Copy the real `settings.ini` to the scratch as the SC-003 baseline: `INI="$HOME/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini"`; if it does **not** exist (the original `settings.gd` does not create it at boot, only on Apply — running the game headless does not help), note "no `.ini` baseline" and SC-003 will be verified only by the comparison of the defaults (T013) and by the user checkpoint; if it exists: `cp "$INI" <scratchpad>/settings.ini.baseline; md5sum "$INI"` (on 2026-09-16: `99dac170ad99078f46d2428e650516e7`, 231 bytes); note the scratchpad path
- [x] T006 Measure the baseline of the 3 scenes (quickstart §1): `cd oxide-godot && for s in main/main.tscn menu/menu.tscn level/level.tscn; do timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/base_$(basename $s .tscn).log 2>&1; echo "$s $?"; done`; expected exit 124 on all, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/base_*.log` empty (intermittent error rule of `main.tscn`: the 5 lines of the dummy renderer do not count if they disappear on a 2nd run), expected WARNINGs only `HDR output…` and `[Physics interpolation]…` (main 2, menu 1, level 2); `md5sum "$INI"` equal to that of T005 (boot does not write the `.ini`)

**Checkpoint**: baseline known — 0 warnings, extension loads, 0 `ERROR` on import and in the 3 scenes, `.ini` copied.

---

## Phase 2: Foundational

**Does not exist in this milestone.** A single script; the only shared edit is `mod settings;` in
`lib.rs`, inside the story. No common module, helper, trait or shared constant may be
created (Principle I). The 5 consumers keep the dynamic access (backlog 1 — out of scope).

---

## Phase 3: User Story 1 — Settings autoload in Rust (Priority: P1) 🎯 MVP = whole milestone

**Goal**: `menu/settings.gd` (107 l., autoload `Settings`, `extends Node`) → `Settings: Node` in
`src/settings.rs`; binding by minimal scene `menu/settings.tscn` + `project.godot:25`; `.gd` + `.uid`
deleted → **zero `.gd` in the project**. Includes the behaviors of the spec's US2 (applying the
settings, with the SSAO fix) and US3 (F11).

**Independent Test**: `main.tscn` headless (full boot with the autoload in Rust: `Settings.ready`
→ `Main` reads `display_mode` → `Menu` applies → automatic host → `Level` applies) without new error;
probes: `.ini` rewritten byte-for-byte identical, 15 defaults equal to the original, SSAO −1 → disabled; in the
game, menu Settings reflect/persist, F11 toggles, SSAO Disabled actually turns off.

- [x] T007 [US1] Create `oxide_godot_core/oxide_godot_lib/src/settings.rs` — part 1 (uses, constant, struct, manual `init`, constants): `use godot::classes::display_server::VSyncMode; use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality}; use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa}; use godot::classes::window::Mode as WindowMode; use godot::classes::{ConfigFile, DisplayServer, Engine, Environment, INode, InputEvent, Node, RenderingServer, Window}; use godot::prelude::*;`; `const CONFIG_FILE_PATH: &str = "user://settings.ini";`; `#[derive(GodotClass)] #[class(base=Node)] pub struct Settings { base: Base<Node>, /* MetalFX is only supported when using the Metal rendering driver. */ #[var] metalfx_supported: bool, defaults: VarDictionary, #[var] config_file: Gd<ConfigFile>, }`; `#[godot_api] impl INode for Settings { fn init(base: Base<Node>) -> Self { let metalfx_supported = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"); let defaults = vdict! { "video" => &vdict! { "display_mode" => WindowMode::EXCLUSIVE_FULLSCREEN.ord() as i64, "vsync" => VSyncMode::ENABLED.ord() as i64, "max_fps" => 0_i64, "resolution_scale" => 1.0_f64, "scale_filter" => if metalfx_supported { Scaling3DMode::METALFX_TEMPORAL.ord() as i64 } else { Scaling3DMode::FSR2.ord() as i64 }, }, "rendering" => &vdict! { "taa" => false, "msaa" => Msaa::DISABLED.ord() as i64, "screen_space_aa" => ScreenSpaceAa::DISABLED.ord() as i64, "shadow_mapping" => true, "gi_type" => Self::GI_TYPE_VOXEL_GI, "gi_quality" => Self::GI_QUALITY_LOW, "ssao_quality" => EnvironmentSsaoQuality::MEDIUM.ord() as i64, "ssil_quality" => -1_i64, /* Disabled */ "bloom" => true, "volumetric_fog" => true, }, }; Self { base, metalfx_supported, defaults, config_file: ConfigFile::new_gd() } } /* ready and input in T008 */ }`; and the block `#[godot_api] impl Settings { #[constant] const GI_TYPE_SDFGI: i64 = 0; #[constant] const GI_TYPE_VOXEL_GI: i64 = 1; #[constant] const GI_TYPE_LIGHTMAP_GI: i64 = 2; #[constant] const GI_QUALITY_DISABLED: i64 = 0; #[constant] const GI_QUALITY_LOW: i64 = 1; #[constant] const GI_QUALITY_HIGH: i64 = 2; /* #[func]s in T008 */ }`. Key order **exactly** that of `settings.gd:20-40` (it is the order of the `.ini`). Identical compiled draft (class `ZzSettings`) in `<scratchpad>/settings_draft.rs` (research §E.0) (research D2–D6)
- [x] T008 [US1] Complete `oxide_godot_core/oxide_godot_lib/src/settings.rs` — part 2 (virtuals + 3 `#[func]`): in the `impl INode`: `fn ready(&mut self) { self.load_settings(); }` and `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("toggle_fullscreen") { let mut window = self.base().get_window().unwrap(); let mode = window.get_mode(); window.set_mode(if !((mode == WindowMode::EXCLUSIVE_FULLSCREEN) || (mode == WindowMode::FULLSCREEN)) { WindowMode::EXCLUSIVE_FULLSCREEN } else { WindowMode::WINDOWED }); self.base().get_viewport().unwrap().set_input_as_handled(); } }`. In the `#[godot_api] impl Settings` block (the same one as the constants — **single**): `#[func] fn load_settings(&mut self) { self.config_file.load(CONFIG_FILE_PATH); /* comment from the original settings.gd:57-58 */ for section in self.defaults.keys_shared() { let section_defaults = self.defaults.at(&section).to::<VarDictionary>(); for key in section_defaults.keys_shared() { let section_name = section.to::<GString>(); let key_name = key.to::<GString>(); if !self.config_file.has_section_key(&section_name, &key_name) { self.config_file.set_value(&section_name, &key_name, &section_defaults.at(&key)); } } } }`; `#[func] fn save_settings(&mut self) { self.config_file.save(CONFIG_FILE_PATH); }`; `#[func] fn apply_graphics_settings(&mut self, mut window: Gd<Window>, mut environment: Gd<Environment>, mut scene_root: Gd<Node>) { self.base().get_window().unwrap().set_mode(WindowMode::from_ord(self.config_file.get_value("video", "display_mode").to::<i64>() as i32)); DisplayServer::singleton().window_set_vsync_mode(VSyncMode::from_ord(self.config_file.get_value("video", "vsync").to::<i64>() as i32)); Engine::singleton().set_max_fps(self.config_file.get_value("video", "max_fps").to::<i64>() as i32); window.set_scaling_3d_scale(self.config_file.get_value("video", "resolution_scale").to::<f64>() as f32); window.set_scaling_3d_mode(Scaling3DMode::from_ord(self.config_file.get_value("video", "scale_filter").to::<i64>() as i32)); window.set_use_taa(self.config_file.get_value("rendering", "taa").to::<bool>()); window.set_msaa_3d(Msaa::from_ord(self.config_file.get_value("rendering", "msaa").to::<i64>() as i32)); window.set_screen_space_aa(ScreenSpaceAa::from_ord(self.config_file.get_value("rendering", "screen_space_aa").to::<i64>() as i32)); if !self.config_file.get_value("rendering", "shadow_mapping").to::<bool>() { /* copy the 5 comment lines from settings.gd:81-85 (Disable shadows … FIXME …) */ scene_root.propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done(); } if self.config_file.get_value("rendering", "ssao_quality").to::<i64>() == -1 { environment.set_ssao_enabled(false); [OWN COMMENT LINE: `// upstream bug fix: settings.gd used if instead of elif — "SSAO: Disabled" (-1) was re-enabled by the else`] } else if self.config_file.get_value("rendering", "ssao_quality").to::<i64>() == EnvironmentSsaoQuality::MEDIUM.ord() as i64 { environment.set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::HIGH, false, 0.5, 2, 50.0, 300.0); } else { environment.set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::MEDIUM, true, 0.5, 2, 50.0, 300.0); } if self.config_file.get_value("rendering", "ssil_quality").to::<i64>() == -1 { environment.set_ssil_enabled(false); } else if self.config_file.get_value("rendering", "ssil_quality").to::<i64>() == EnvironmentSsilQuality::MEDIUM.ord() as i64 { environment.set_ssil_enabled(true); RenderingServer::singleton().environment_set_ssil_quality(EnvironmentSsilQuality::MEDIUM, false, 0.5, 2, 50.0, 300.0); } else { environment.set_ssil_enabled(true); RenderingServer::singleton().environment_set_ssil_quality(EnvironmentSsilQuality::HIGH, true, 0.5, 2, 50.0, 300.0); } environment.set_glow_enabled(self.config_file.get_value("rendering", "bloom").to::<bool>()); environment.set_volumetric_fog_enabled(self.config_file.get_value("rendering", "volumetric_fog").to::<bool>()); }`. The comment `// upstream bug fix: …` stays **alone on a line, immediately above the `} else if` line** of the SSAO (the only line with that text in the file). Do **not** swap the quirk "MEDIUM → HIGH without half_size / otherwise MEDIUM with half_size"; do **not** touch the SSIL; do **not** handle `from_ord(5)` (research D8)
- [x] T009 [US1] Add `mod settings;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod main_scene;`); no other line changes
- [x] T010 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. If `field … is never read` or `E0271` in the `vdict!` appears, check T007 (`#[var]` on `metalfx_supported`; `&vdict!` on the nested ones) — do not invent another solution
- [x] T011 [US1] Autoload binding (plan §"Binding edit"): (a) create `oxide-godot/menu/settings.tscn` with **exactly** 3 lines — `printf '[gd_scene format=3]\n\n[node name="Settings" type="Settings"]\n' > oxide-godot/menu/settings.tscn` — and check `cat -A` (no `uid=`, no `script`, no `ext_resource`); (b) `grep -n '^Settings=' oxide-godot/project.godot` → `25:Settings="*res://menu/settings.gd"`; `sed -i '25s|^Settings="\*res://menu/settings\.gd"$|Settings="*res://menu/settings.tscn"|' oxide-godot/project.godot`; `grep -n '^Settings=' oxide-godot/project.godot` → `25:Settings="*res://menu/settings.tscn"`; `git diff --stat -- oxide-godot/project.godot` = `1 file changed, 1 insertion(+), 1 deletion(-)`; `git diff -- oxide-godot/project.godot | grep -c '^[-+][^-+]'` = 2 (no other line — `run/main_scene` l.15 and `toggle_fullscreen` l.172–177 untouched)
- [x] T012 [US1] Delete `oxide-godot/menu/settings.gd` and `oxide-godot/menu/settings.gd.uid` (`git rm`); verify `grep -rn 'uid://b04fekxdgdq0k' oxide-godot/ | grep -v '/.godot/'` empty, `grep -rn 'menu/settings.gd' oxide-godot/ --include='*.tscn' --include='*.godot' --include='*.gd' --include='*.cfg'` empty; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 0; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 0; `git status --short` = exactly `D menu/settings.gd`, `D menu/settings.gd.uid`, `M project.godot`, `M lib.rs`, `?? settings.rs`, `?? menu/settings.tscn`
- [x] T013 [US1] Headless validation (no editor open): import `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log` → `Initialize godot-rust` ×1, no new `ERROR`; `ls oxide-godot/menu/` → `settings.tscn` present, **no** `settings.tscn.uid` generated (if it appears, note it and include it in the commit — research D1 says it does not appear); `timeout 20 /usr/bin/godot.x86_64 --headless --path . main/main.tscn 2>&1 | tee /tmp/run_main.log` → exit 124, regression grep (`ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`) empty — **run 2 times** (intermittent error rule); `menu/menu.tscn` → 124, grep empty; `level/level.tscn` → 124, grep empty; WARNINGs equal to the baseline (T006); `md5sum "$INI"` equal to T005 (boot with the autoload in Rust does not write the `.ini`). **NEVER** run `menu/settings.tscn` in isolation. Flow exercised in `main.tscn`: `Settings.ready` → `load_settings`; `Main.ready` `get("config_file")` → `display_mode`; `Menu.ready` `call("apply_graphics_settings", …)` + automatic host; `Level.ready` `call("apply_graphics_settings", …)` + `gi_type`/`gi_quality`; `FlyingForklift.ready` `shadow_mapping`
- [x] T014 [US1] Disposable probes (`-s`, scripts in the scratchpad — **never** inside the repo; templates in research §E.3/E.3b, adapted from `ZzSettings` to `Settings`), run from `oxide-godot/oxide-godot/` with `timeout 30 /usr/bin/godot.x86_64 --headless --path . -s <scratchpad>/probe_x.gd`: **(A, `.ini` present)** `/root/Settings.get_class()` = `Settings`, `name` = `Settings`, `get("config_file")` is a `ConfigFile` and is the same object in two reads, `get("metalfx_supported")` = false, `has_method` of `load_settings`/`save_settings`/`apply_graphics_settings` = true, `ClassDB.class_has_integer_constant("Settings", c)` = true for the 6 constants with values 0/1/2/0/1/2, `config_file.get_sections()` = `["video", "rendering"]` and keys in the order of `settings.gd`; `save_settings()` → **`diff "$INI" <scratchpad>/settings.ini.baseline` empty** (SC-003; if T005 had no baseline, skip this line); FR-024: `config_file.set_value("rendering","ssao_quality",-1)` + `apply_graphics_settings(root, Environment.new(), Node.new())` → `ssao_enabled == false`; MEDIUM → true; HIGH → true; `ssil_quality = -1` → `ssil_enabled == false`; `scale_filter = 5` → `root.scaling_3d_mode == 5` without panic; `shadow_mapping = false` + `DirectionalLight3D` child of the `scene_root` → `shadow_enabled == false`; **restore** all changed values at the end (the probe does not save after that). **(B, `.ini` moved aside — `mv "$INI" "$INI.aside"` before, `mv` back afterwards, md5 checked)**: after boot, `config_file` has 15 keys with the values/types `4 int, 1 int, 0 int, 1.0 float, 2 int, false, 0 int, 0 int, true, 1 int, 1 int, 2 int, -1 int, true, true` (table of data-model.md; `settings.gd` has already been deleted — compare with the table, not with `load("res://menu/settings.gd")`), and `FileAccess.file_exists("user://settings.ini")` = false. The `WARNING … leaked at exit` generated by the probe's own objects do not count Additionally (spec US1 scen. 4 and US2 scen. 5): (C) partial `.ini` in the scratch containing only `[video]\ndisplay_mode=3` → after `load_settings`, `display_mode` == 3 and ALL the other 14 keys receive the default; (D) in T014-A, also `ssil_quality = 2` and `= 3` → `environment.ssil_enabled == true` (and `-1` → false).
- [x] T015 [US1] Mechanical checks and contract (`contracts/settings.md` §"Verification before the commit"): `R=oxide_godot_core/oxide_godot_lib/src/settings.rs`; `grep -n '#\[func\]' -A1 $R | grep 'fn '` = `load_settings`, `save_settings`, `apply_graphics_settings` (3, no other); `grep -n '#\[var\]' -A1 $R | grep -oE '(metalfx_supported|config_file)'` = the 2; `grep -c '#\[export\]' $R` = 0; `grep -c '#\[constant\]' $R` = 6; `grep -c 'upstream bug fix' $R` = 1 and the next line is `} else if …ssao_quality…`; `grep -c 'else if' $R` = 2 (SSAO and SSIL); `grep -c 'get_window()' $R` = 2 (`input` ×1, `apply_graphics_settings` ×1 — mode on the own window); `grep -n 'fn init' $R` = 1 (manual); `grep -c 'vdict!' $R` = 3; `grep -c 'CONFIG_FILE_PATH' $R` = 3 (const + load + save); `grep -n 'name="Settings" type="Settings"' oxide-godot/menu/settings.tscn` = l.3; `wc -l < oxide-godot/menu/settings.tscn` = 3; `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs` **empty** (FR-014; the 12 consumer sites untouched); `git diff --quiet HEAD -- CLAUDE.md oxide_godot_core/Cargo.toml && echo unchanged`; `ls oxide_godot_core/oxide_godot_lib/src/ | wc -l` = 16
- [x] T016 [US1] Record in `docs/upstream-bugs.md` entry **#3** (text from research §"Entry #3 of docs/upstream-bugs.md": defect `settings.gd:88-95` `if` without `elif` → "SSAO: Disabled" re-enabled; reproduced headless in the original; script/scene + `specs/005-v1-settings-autoload` FR-020–FR-024; fix `else if` in `settings.rs` `apply_graphics_settings` with comment `// upstream bug fix`, branches/parameters/SSIL untouched; Commit column = the **subject** of the T017 commit) and in `docs/v2-backlog.md` row **25** (research §"Candidate v2 backlog": `menu/settings.gd` (port 5) — review the SSAO mapping "Medium → HIGH without half_size / High → MEDIUM with half_size"; ambiguous intent, preserved). `grep -c '^| [0-9]' docs/upstream-bugs.md` = 3; `grep -c '^| [0-9]' docs/v2-backlog.md` = 25; do not duplicate items 1–24 nor entries 1–2
- [x] T017 [US1] Single port commit on `main` (explicit `git add` of `oxide_godot_core/oxide_godot_lib/src/settings.rs`, `oxide_godot_core/oxide_godot_lib/src/lib.rs`, `oxide-godot/menu/settings.tscn`, `oxide-godot/project.godot`, `docs/upstream-bugs.md`, `docs/v2-backlog.md` — the 2 deletions are already in staging from the `git rm`; check `git status --short` and `git diff --cached --stat` = 8 files, nothing else). Subject: `Port settings.gd → Settings (Node, autoload); menu/settings.tscn nova; project.godot: Settings="*res://menu/settings.tscn"`. Body (`- ` lines): `- vínculo por autoload: cena mínima + project.godot (única edição), regra declarada na spec` (a native class cannot be a direct autoload; the node name comes from the autoload key); `- #[var] config_file / metalfx_supported; DEFAULTS como dicionário aninhado iterado (ordem do .ini preservada); 6 #[constant] dos enums; init manual`; `- apply_graphics_settings linha a linha: modo de janela na janela própria e quirk "média aplica alta" preservados`; `- upstream bug fix: SSAO "Disabled" (-1) era religado pelo else (settings.gd:90 usava if em vez de elif) — agora if/else if/else; comentário // upstream bug fix no ponto exato`; `- docs/upstream-bugs.md: entrada #3.`; `- backlog v2: item 25`; `- settings.gd e settings.gd.uid removidos: ZERO .gd no projeto — v1 sem GDScript`; `- consumidores (bullet, flying_forklift, level, menu, main_scene) intocados (backlog 1)`. Note the hash
- [x] T018 [US1] **User checkpoint (visual validation, plan §"Visual validation")** — done by the user in the game, comparing with `../oxide_godot_origins/`: boot → menu with the saved window mode; Settings reflects the current `settings.ini` (written by the original `settings.gd` — read without change); change options → Apply applies and persists (reopen Settings; **close and reopen the game**); Cancel discards; delete the `.ini` → defaults and the file only comes back after an Apply; level: `Shadow mapping` off → no light casts a shadow; **SSAO: Disabled → SSAO really disabled** (in the original it stayed on — it is the only different behavior, declared: upstream-bugs #3); SSAO Medium/High and SSIL Disabled/Medium/High as in the original; **F11 and Alt+Enter** toggle exclusive fullscreen ↔ windowed in the menu and in the level; whole game menu → Play → level → ESC → menu without any `.gd`. In the editor: `menu/settings.tscn` root `Settings` of type `Settings` without script; Project Settings → Autoload: `Settings` → `res://menu/settings.tscn`, singleton enabled; no `.gd` in the FileSystem. Divergence → commit `Fix port settings.gd …`. Only proceed to Polish with the explicit OK

**Checkpoint**: 0 `.gd` in the project; autoload in Rust; game playable end to end in Rust.

---

## Phase 4: Polish — final v1 verification

**Purpose**: mechanical verification of the whole v1 (quickstart §8) and full visual validation
(SC-002); with the OK, v1 is declared complete (SC-008). **Tag and branch `v2` are not part of
these tasks** — user decisions.

- [x] T019 Mechanical verification of the milestone and of v1 (quickstart §8): `find oxide-godot -name '*.gd' -not -path '*/addons/*'` empty and `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'` empty (SC-001); `git log --oneline 619575d..HEAD | grep -c '^[0-9a-f]* Port '` = 1 (`Fix port …` do not count); `git diff --stat 619575d -- '*.rs'` = only `lib.rs` (+1) and `settings.rs` (new); `git diff --stat 619575d -- oxide-godot/project.godot` = `1 insertion(+), 1 deletion(-)` and `grep -n '^Settings=' oxide-godot/project.godot` = `res://menu/settings.tscn`; `git diff --stat 619575d -- oxide-godot/ | grep -v 'settings\|project.godot'` empty (no other scene/file of the Godot project changed); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 15 modules (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot flying_forklift level menu main_scene settings`); `git diff --quiet 619575d -- CLAUDE.md oxide_godot_core/Cargo.toml && echo unchanged`; `grep -c '^| [0-9]' docs/upstream-bugs.md` = 3; `grep -c '^| [0-9]' docs/v2-backlog.md` = 25; FR-015: `grep -nE '\.call\(|call_deferred\(|\.get\("' oxide_godot_core/oxide_godot_lib/src/settings.rs` **empty** (`Settings` makes no dynamic call at all; `propagate_call_ex` does not match the pattern and is base API); `grep -rn '/root/Settings' oxide_godot_core/oxide_godot_lib/src/ | wc -l` = 10 (the consumers, unchanged)
- [x] T020 Final headless validation of v1 (no editor open): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; headless import with `Initialize godot-rust` and no new `ERROR`; `main/main.tscn` (×2, intermittent rule), `menu/menu.tscn`, `level/level.tscn`, `level/forklift/flying_forklift.tscn`, `player/player.tscn`, `enemies/red_robot/red_robot.tscn` headless with empty regression grep; `md5sum "$INI"` unchanged by the boots
- [x] T021 **Full user visual validation (SC-002 — whole v1)**: side-by-side session with `../oxide_godot_origins/`: boot → menu (Play, Play Online → Host, Settings with the 15 rows reflecting and writing each option — Apply, Cancel, reopen, restart the game, check `user://settings.ini` —, Quit) → loading with bar → playable level (player, shooting, camera shake, robots with parts and 15 s respawn, door, assorted forklifts, GI/shadows/SSAO/SSIL/bloom/fog according to the options) → ESC → menu → Play again; F11/Alt+Enter on any screen. The only expected and declared difference: SSAO Disabled turns SSAO off (upstream-bugs #3). Editor: the 5 Rust root scenes without script + `settings.tscn`; Autoload in scene; no `.gd`. **With the user's OK: mark T001–T021 `[x]`, commit only `tasks.md` (`Tasks 005: Milestone E complete — v1 without GDScript`) and declare v1 complete (SC-008; Principle I: no remaining `.gd` script and the game playable end to end)**. Tag and branch `v2`: outside these tasks

---

## Dependencies & Execution Order

### Mandatory order

```
Phase 1 (Setup: T001–T006)          — baseline + copy of the real settings.ini
  → Phase 3 US1 (T007–T018)  → single commit (Settings; project.godot 1 line; ZERO .gd)
  → Phase 4 Polish (T019–T021) → v1 complete
```

- **Phase 2 (Foundational)**: does not exist.
- **Internal order of US1** (strict dependencies): module part 1 (T007) → part 2 (T008, same
  file) → `mod` (T009) → build (T010) → binding (T011: new scene + `project.godot`) → deletions
  (T012) → headless (T013) → probes (T014) → mechanics/contract (T015) → docs (T016) → commit
  (T017) → checkpoint (T018). The binding is only edited after the build because `type="Settings"`
  needs to exist in the lib for the autoload to resolve at import. The deletions (T012) come after the
  binding (T011) so the project never points to a nonexistent `.gd`.
- **User checkpoints** (T018, T021) are blocking.

### Parallel Opportunities

- Setup: T004 is [P] with respect to T002/T003/T005/T006 (it only reads directories and does greps).
- Inside US1 no task is [P]: T007→T008 write the same file; everything else consumes the
  previous step. T016 (docs) could come earlier, but it edits files that go into the same commit —
  keep it sequential.

### Parallel Example

```bash
# The only truly independent pair (Phase 1):
Task: "T002 cargo build → count warnings"
Task: "T004 name collision grep; ls of the bindings; docs counts"
```

---

## Implementation Strategy

### MVP = whole milestone (User Story 1)

1. Phase 1: Setup (T001–T006) — baseline and `settings.ini.baseline`.
2. Phase 3: US1 (T007–T018) — `settings.gd` ported, autoload in scene, commit, user's OK.
3. **STOP AND VALIDATE**: first (and only) autoload of v1 and the first edit of `project.godot`;
   any problem with the binding by scene or with the 12 dynamic accesses shows up in T013.

### Incremental Delivery

A single increment: after the T017 commit the project has **zero** `.gd` and the game is playable; Phase 4
only verifies and declares.

### If something fails midway

Do not commit partially. Either the whole port (module + `lib.rs` + new scene + `project.godot` +
deletions + docs) goes into the commit, or nothing:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/settings.rs oxide-godot/menu/settings.tscn`
returns to the clean tree of `619575d`, which is playable (with `settings.gd`). If the implementation requires
something not foreseen in the tasks (another file, another line of `project.godot`, another fix), stop
and report.

---

## Notes

- No task creates a helper, trait, common module, test or new log. `DEFAULTS` remains an iterated
  nested dictionary; `apply_graphics_settings` keeps the repeated reads of `config_file`.
- **One** bug fix (SSAO `if` → `else if`), with the 4 requirements. Any other defect →
  stop and report; when in doubt, it is an improvement (backlog).
- `settings.tscn` is created as text (3 lines, no `uid=`); the import does not generate a `.uid` for it
  (research D1). If the user saves the scene in the editor and Godot appends `uid="…"` to the header,
  it is a future cosmetic change — outside this milestone.
- The names `config_file`, `metalfx_supported`, `load_settings`, `save_settings`,
  `apply_graphics_settings` and the 6 constants are contract (FR-013) — copy, never "translate".
- The headless regression grep is `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`;
  baseline WARNINGs (`HDR output`, `Physics interpolation`) do not count; in `main.tscn`, the 5
  lines of the dummy renderer do not count if they disappear on the 2nd run (3 in a row = regression);
  `WARNING … leaked at exit` from the `-s` probes belong to the probe itself.
- **Never** run `menu/settings.tscn` in isolation in headless (autoload + scene = two `Settings`).
- No Godot editor open during headless validations — warn the user beforehand; never kill their
  process.
- The user's real `settings.ini` is touched only by `save_settings` in probe A (rewrites identical —
  SC-003) and moved/restored in probe B; check the md5 after each probe.
- The commit (T017) comes **before** the visual checkpoint; a divergence found by the user is
  fixed in a `Fix port …` commit (never `Port …`, so that `grep -c '^[0-9a-f]* Port '` stays = 1).
- **Staging**: before any commit other than the port one, `git status` must not have `.gd`
  deletions in staging (lesson from Milestone C).
- Files that **never** change in this milestone: `.gdextension`, `Cargo.toml`, `CLAUDE.md`, the 14
  existing Rust modules, all the existing `.tscn`, and all the lines of `project.godot`
  except 25.
