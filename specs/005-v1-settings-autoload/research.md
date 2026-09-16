# Research: Milestone E — Settings autoload (v1 raw port)

**Phase**: v1 (constitution v1.3.1). All the signatures below were **confirmed by
compilation** of a complete draft (`src/zz_research.rs`, class `ZzSettings`, 0 errors, 0
warnings) and **exercised headless** with the autoload pointing to the draft class through
a minimal scene (§E). Draft, scene and the `project.godot` line were reverted (`git status`
clean; `cargo build` back to 0 warnings in the state of `6f4d9ba`). Line references are from the
bindings in `oxide_godot_core/target/debug/build/godot-core-4eba5d49e15a0d7e/out/classes/`.

## D1 — Autoload binding: minimal scene + `project.godot`

- **Decision**: create `oxide-godot/menu/settings.tscn` with 3 lines
  (`[gd_scene format=3]`, empty, `[node name="Settings" type="Settings"]`) and change
  `project.godot:25` from `Settings="*res://menu/settings.gd"` to
  `Settings="*res://menu/settings.tscn"`. Delete `settings.gd` + `settings.gd.uid`.
- **Rationale**: `[autoload]` accepts a script or a scene; a native class cannot be a direct autoload.
  The 1-node scene with `type` = class is the "`type` swap" of Principle II for a node that did not
  exist in a scene. The name `/root/Settings` comes from the autoload key (confirmed §E.2:
  `name=Settings path=/root/Settings class=ZzSettings`). The `*` (singleton) is preserved.
- **No `uid=` in the header**: the headless import neither generated `settings.tscn.uid` nor emitted a warning
  (§E.2) — `.tscn` files keep the uid in their own header and it is optional. If the user saves the
  scene in the editor, Godot may append `uid="uid://…"` to l.1 — a future cosmetic change, outside
  the commit.
- **Rejected alternatives**: bridge `.gd` (`extends Settings`) — forbidden, would leave 1 `.gd`;
  `Engine::register_singleton` in the `ExtensionLibrary` — it is not a node, breaks the 5 consumers'
  `get_node("/root/Settings")` and `_input`.

## D2 — Structure and name

- `src/settings.rs`, `mod settings;` in `lib.rs` after `mod main_scene;`. `struct Settings`,
  `#[class(base=Node)]` (**without** automatic `init` — D4). No name conflict in the crate.
- Name checked: `ls out/classes/ | grep -i settings` → `editor_settings`, `label_settings`,
  `mesh_convex_decomposition_settings`, `open_xr_android_thread_settings_extension`,
  `project_settings` (no bare `Settings`); `CLAUDE.md` grep over the `.gd`s: only
  `settings.gd` itself, which is deleted. `ClassDB` registered `ZzSettings` without conflict (§E.3).

## D3 — Fields and visibility

| Original (`settings.gd`) | Rust | Note |
|---|---|---|
| `const CONFIG_FILE_PATH = "user://settings.ini"` (l.15) | `const CONFIG_FILE_PATH: &str = "user://settings.ini";` (module level) | — |
| `var metalfx_supported: bool = RenderingServer.get_current_rendering_driver_name() == "metal"` (l.18) | `#[var] metalfx_supported: bool`, computed in `init` with `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")` (`rendering_server.rs:5391`) | **`#[var]` imposed by the compiler**: without it, `field metalfx_supported is never read` (the field only feeds `scale_filter` in `init`). In the original it is a top-level `var` = script property; `#[var]` reproduces that (usage `NONE`). Comparison with `GString::from` (D3 of plan 004) |
| `var DEFAULTS := { video = {…}, rendering = {…} }` (l.20-40) | `defaults: VarDictionary` (private field, no `#[var]`), built in `init` with nested `vdict!` — D5 | Name `defaults` (Rust field; the original is `DEFAULTS`). Not exposed: no consumer reads it; if it were `#[var]` the exposed name would be `defaults`, different from the original — better not to expose |
| `var config_file := ConfigFile.new()` (l.42) | `#[var] config_file: Gd<ConfigFile>` = `ConfigFile::new_gd()` in `init` | `#[var]` (not `#[export]`): the consumers' 8 `get("config_file")` resolve (§E.3 `same_obj=true`); nothing appears in the inspector |
| `enum GIType {…}`, `enum GIQuality {…}` (l.3-13) | 6 `#[constant] const …: i64` in the `#[godot_api] impl Settings` block — D6 | — |

## D4 — Manual `init`

- **Decision**: `#[godot_api] impl INode for Settings { fn init(base: Base<Node>) -> Self { … } }`
  computing, **in this order** (that of the `var`s in the original): `metalfx_supported`, `defaults`
  (uses `metalfx_supported`), `config_file`.
- **Rationale**: `#[init(val = …)]` cannot reference another field; the default of `scale_filter`
  depends on `metalfx_supported`. The alternative (recomputing the driver inside the `vdict!`) would duplicate the
  `RenderingServer` read that the original does once.

## D5 — `DEFAULTS` as a nested `VarDictionary`; `load_settings`

- `vdict! { "video" => &vdict! { … }, "rendering" => &vdict! { … } }` — **imposed by the
  compiler**: the nested value must be passed **by reference** (`&vdict!`), otherwise
  `E0271: <Dictionary<Variant, Variant> as ToGodot>::Pass == ByValue`. Scalar values go
  by value: `i64` (`.ord() as i64` of the engine enums; `0_i64`; `-1_i64`; `Self::GI_TYPE_VOXEL_GI`;
  `Self::GI_QUALITY_LOW`), `f64` (`1.0_f64`), `bool`.
- Values (checked programmatically against `settings.gd` — §E.3, 15/15 equal in value and
  type): `display_mode` = `WindowMode::EXCLUSIVE_FULLSCREEN.ord()` = 4; `vsync` =
  `VSyncMode::ENABLED.ord()` = 1; `max_fps` = 0; `resolution_scale` = 1.0; `scale_filter` =
  `if metalfx_supported { Scaling3DMode::METALFX_TEMPORAL.ord() } else { Scaling3DMode::FSR2.ord() }`
  = 2 here; `taa` = false; `msaa` = `Msaa::DISABLED.ord()` = 0; `screen_space_aa` =
  `ScreenSpaceAa::DISABLED.ord()` = 0; `shadow_mapping` = true; `gi_type` = 1; `gi_quality` = 1;
  `ssao_quality` = `EnvironmentSsaoQuality::MEDIUM.ord()` = 2; `ssil_quality` = −1; `bloom` = true;
  `volumetric_fog` = true.
- **Insertion order preserved**: Godot's `Dictionary` is ordered; `keys_shared()` iterates in
  insertion order (§E.3: sections `["video", "rendering"]`, keys in the order of the original). The
  written `.ini` comes out in the same order (SC-003 confirmed byte-for-byte, §E.3).
- `load_settings` (`settings.gd:55-62`): `self.config_file.load(CONFIG_FILE_PATH)`
  (`config_file.rs:214`, `-> Error`, return **ignored** — like the original; is the call
  `let _ = …`? **No**: call without `let _` — `Error` is not `#[must_use]`, compiled without warning);
  `for section in self.defaults.keys_shared() { let section_defaults = self.defaults.at(&section).to::<VarDictionary>(); for key in section_defaults.keys_shared() { … if !self.config_file.has_section_key(&section_name, &key_name) { self.config_file.set_value(&section_name, &key_name, &section_defaults.at(&key)); } } }`
  (`config_file.rs:164,123`; `section.to::<GString>()`/`key.to::<GString>()` because `has_section_key`
  asks for `AsArg<GString>` and the dictionary keys are `Variant`). **Does not save** (§E.3: with the
  `.ini` absent, `FileAccess.file_exists("user://settings.ini")` = false after boot).
- `ConfigFile::load` on a missing file: returns `ERR_FILE_NOT_FOUND` (7) and **does not print an error**
  (§E.3) — no new `ERROR` line in headless.
- `save_settings` (`l.65-66`): `self.config_file.save(CONFIG_FILE_PATH)` (`config_file.rs:234`),
  return ignored.

## D6 — Script enums → `#[constant]`

- **Decision**: in the `#[godot_api] impl Settings` block: `#[constant] const GI_TYPE_SDFGI: i64 = 0;`
  `GI_TYPE_VOXEL_GI = 1`, `GI_TYPE_LIGHTMAP_GI = 2`, `GI_QUALITY_DISABLED = 0`, `GI_QUALITY_LOW = 1`,
  `GI_QUALITY_HIGH = 2`. Supported (`godot-macros inherent_impl.rs:627,772`); registered in the
  `ClassDB` (§E.3: `class_has_integer_constant` = true, values 0/1/2).
- **Rationale**: preserves the `Settings.GIType.X` surface in the way gdext allows (named
  script enums do not exist as such). The defaults use `Self::GI_TYPE_VOXEL_GI` /
  `Self::GI_QUALITY_LOW` (the original uses `GIType.VOXEL_GI` / `GIQuality.LOW`). No consumer
  reads them by name (backlog item 1 already foresees local constants in the consumers).
- Alternative (only integers in the defaults) — would lose names the original has; zero cost to expose.

## D7 — `ready` and `_input`

- `fn ready(&mut self) { self.load_settings(); }` (`l.45-46`).
- `fn input(&mut self, input_event: Gd<InputEvent>)` (`l.49-52`):
  `if input_event.is_action_pressed("toggle_fullscreen")` (`input_event.rs:76`) →
  `let mut window = self.base().get_window().unwrap(); let mode = window.get_mode();`
  (`node.rs:1163` `Option`; `window.rs:381`) →
  `window.set_mode(if !((mode == WindowMode::EXCLUSIVE_FULLSCREEN) || (mode == WindowMode::FULLSCREEN)) { WindowMode::EXCLUSIVE_FULLSCREEN } else { WindowMode::WINDOWED })`
  (`window.rs:372`; `Mode` is `Copy + PartialEq`) → `self.base().get_viewport().unwrap().set_input_as_handled()`
  (`node.rs:1289`; `viewport.rs:741`). `use godot::classes::window::Mode as WindowMode` (alias from
  plan 004/`main_scene.rs`).

## D8 — `apply_graphics_settings(window, environment, scene_root)` line by line + SSAO fix

`#[func] fn apply_graphics_settings(&mut self, mut window: Gd<Window>, mut environment: Gd<Environment>, mut scene_root: Gd<Node>)`
— target of `call("apply_graphics_settings", &[window, environment, self])` from `level.rs:44`,
`menu.rs:208,606` (3 `Variant`s → the `Gd<T>` are converted by gdext; §E.2 ran all three).
Reads always `self.config_file.get_value(sec, key)` → `Variant` → `.to::<i64>()` /
`.to::<bool>()` / `.to::<f64>()`, repeated on each line as in the original (`Settings.config_file`
→ own property).

| `settings.gd` | Rust | Bindings |
|---|---|---|
| l.70 `get_window().mode = …display_mode` | `self.base().get_window().unwrap().set_mode(WindowMode::from_ord(… as i32))` — **own** window, not the parameter (quirk) | `window.rs:372`; `EngineEnum::from_ord` `obj/traits.rs:201` |
| l.71 `DisplayServer.window_set_vsync_mode(…vsync)` | `DisplayServer::singleton().window_set_vsync_mode(VSyncMode::from_ord(… as i32))` | `display_server.rs:2020` |
| l.72 `Engine.max_fps = …max_fps` | `Engine::singleton().set_max_fps(… as i32)` | `engine.rs:88` |
| l.73 `window.scaling_3d_scale = …resolution_scale` | `window.set_scaling_3d_scale(….to::<f64>() as f32)` | `viewport.rs:1128` (inherited by `Window`) |
| l.74 `window.scaling_3d_mode = …scale_filter` | `window.set_scaling_3d_mode(Scaling3DMode::from_ord(… as i32))` — see **Nearest** below | `viewport.rs:1110` |
| l.76-78 `use_taa`, `msaa_3d`, `screen_space_aa` | `window.set_use_taa(bool)`, `set_msaa_3d(Msaa::from_ord)`, `set_screen_space_aa(ScreenSpaceAa::from_ord)` | `viewport.rs:218,182,200` |
| l.80-86 `if not …shadow_mapping: scene_root.propagate_call("set", ["shadow_enabled", false])` | `if !… { scene_root.propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done(); }` — base API; upstream `FIXME` comment copied | `node.rs:735,2093` (`args: &AnyArray`; `varray!` produces `VarArray` = `AnyArray`) — §E.3: child light ended up `shadow_enabled = false` |
| l.88-95 SSAO | `if q == -1 { environment.set_ssao_enabled(false); } else if q == EnvironmentSsaoQuality::MEDIUM.ord() as i64 { set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::HIGH, false, 0.5, 2, 50.0, 300.0); } else { set_ssao_enabled(true); …(EnvironmentSsaoQuality::MEDIUM, true, 0.5, 2, 50.0, 300.0); }` — **`else if` is the fix** (original: `if`); comment `// upstream bug fix: settings.gd used `if` instead of `elif` — "SSAO: Disabled" (-1) was re-enabled by the else` on the line immediately above the `else if` | `environment.rs:570`; `rendering_server.rs:3473` (`quality, half_size: bool, adaptive_target: f32, blur_passes: i32, fadeout_from: f32, fadeout_to: f32`) |
| l.97-104 SSIL | `if q == -1 { set_ssil_enabled(false) } else if q == EnvironmentSsilQuality::MEDIUM.ord() as i64 { set_ssil_enabled(true); environment_set_ssil_quality(MEDIUM, false, 0.5, 2, 50.0, 300.0) } else { set_ssil_enabled(true); environment_set_ssil_quality(HIGH, true, 0.5, 2, 50.0, 300.0) }` — faithful (the original is already `elif`) | `environment.rs:732`; `rendering_server.rs:3483` |
| l.106-107 `glow_enabled`, `volumetric_fog_enabled` | `environment.set_glow_enabled(bool)`, `environment.set_volumetric_fog_enabled(bool)` | `environment.rs:1038,1508` |

- **Nearest (ordinal 5)**: `Viewport.SCALING_3D_MODE_NEAREST` = 5 exists in Godot 4.7 but not in the
  bindings' 4.6 API (`menu.rs` uses `const SCALING_3D_MODE_NEAREST: i64 = 5`). In 4.6,
  `Scaling3DMode::try_from_ord` accepts `0..=5` (5 = `MAX`), so **`from_ord(5)` does not panic**
  and ordinal 5 reaches the engine, which interprets it as Nearest (§E.3: `root.scaling_3d_mode = 5`
  after applying with `scale_filter = 5`). No special handling; do **not** use a dynamic
  `set("scaling_3d_mode")`.
- `from_ord` panics for ordinals outside the enum (`traits.rs:201-204`). The possible values come
  from the ported menu (`.ord()` of the same enums) and from the defaults; a hand-edited `.ini` with garbage
  would break the original too (assignment of an invalid integer to the property). Fidelity; nothing to
  handle.
- **SSAO bug reproduced in the original** (§E.4): with `settings.gd` as autoload,
  `ssao_quality = -1` → `apply_graphics_settings` → `environment.ssao_enabled == true`; SSIL −1 →
  `false`. With the draft: −1 → `false`; MEDIUM → `true`; HIGH → `true` (§E.3).

## D9 — What does NOT change (quirks preserved, Principle I)

Window mode applied to the autoload's own `get_window()` (not to the `window` parameter);
"SSAO medium applies high quality without `half_size`, and the rest applies medium with `half_size`"
(ambiguous intent — backlog 25); shadows `FIXME` (with `shadow_mapping` true nothing is
re-enabled); `metalfx_supported` evaluated once at construction; repeated reads of
`config_file.get_value` on each line; `load_settings` without `save`; `Error` returns of `load`/`save`
ignored; insertion order of the defaults; the only behavior change is the FR-020–FR-024 fix.

## D10 — Consumers (untouched) and the closing of the exception

The 10 sites (`bullet.rs:72`, `flying_forklift.rs:19`, `level.rs:43-49,116,144,173`,
`menu.rs:208,308,479,611`, `main_scene.rs:29`) do `get_node_as::<Node>("/root/Settings")` +
`get("config_file")` / `call(…)`. All resolved against the draft class in `main.tscn`
(boot → `Main` reads `display_mode` → `Menu` applies → host → `Level` applies), `menu.tscn` and
`level.tscn` (§E.2). They stay as they are (spec FR-014; backlog 1). `CLAUDE.md` does not change (plan,
FR-019).

## §E — Empirical verification (2026-09-16, headless, no editor open — `pgrep -a godot` empty)

### E.0 Draft compilation

`ZzSettings` **complete** (manual init, `#[var]` ×2, nested `vdict!`, 6 `#[constant]`, `ready`,
`input`, `load_settings`, `save_settings`, `apply_graphics_settings` with the fix): 2 iterations
— (1) `E0271` in the nested `vdict!` → `&vdict!`; (2) warning `field metalfx_supported is never read`
→ `#[var]`. Final: **0 errors, 0 warnings**. Copy of the draft in the scratchpad
(`settings_draft.rs`) for `/speckit-tasks` to reproduce.

### E.1 Baseline (state `6f4d9ba` = code of `fa27e22`; `0e27c68`/`6f4d9ba` only touch specs and `.gitignore`)

- `cargo build`: 0 warnings. Import: `Initialize godot-rust` ×1, 0 new `ERROR`.
- `main.tscn` exit 124 / 0 regressions / 2 WARNINGs (HDR, Physics interpolation); `menu.tscn` 124/0/1;
  `level.tscn` 124/0/2 (measured in T055 of Milestone D at `fa27e22`; nothing changed since).
- Real `user://settings.ini`: `~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini`,
  231 bytes, md5 `99dac170ad99078f46d2428e650516e7`, written by the original `settings.gd`
  (current content: `display_mode=3`, `vsync=1`, `max_fps=0`, `resolution_scale=1.0`,
  `scale_filter=2`, `taa=false`, `msaa=3`, `screen_space_aa=0`, `shadow_mapping=true`,
  `gi_type=1`, `gi_quality=2`, `ssao_quality=3`, `ssil_quality=3`, `bloom=true`,
  `volumetric_fog=true`). Copy in `scratchpad/settings.ini.baseline` for the SC-003 diff.
- `docs/upstream-bugs.md`: 2 entries; `docs/v2-backlog.md`: 24 items.

### E.2 Autoload in scene with the draft class (full flow)

`menu/zz_settings.tscn` (3 lines, `type="ZzSettings"`) + `project.godot:25` →
`Settings="*res://menu/zz_settings.tscn"`; import: `Initialize godot-rust`, **no `.uid`
generated for the `.tscn`, no warning**. `main.tscn` 124/0/2, `menu.tscn` 124/0/1,
`level.tscn` 124/0/2 — equal to the baseline; the `.ini` kept the same md5 after the 3 boots
(`load_settings` does not write). Reverted afterwards: `git checkout -- project.godot`, `rm zz_settings.tscn`.

### E.3 Probe A (`-s`, `.ini` present) — contract, constants, SC-003, FR-024, Nearest, shadows

`class=ZzSettings name=Settings path=/root/Settings`; `config_file class=ConfigFile same_obj=true`;
`metalfx_supported=false`; `has_method` `load_settings`/`save_settings`/`apply_graphics_settings`
= true; 6 constants with `class_has_integer_constant` = true and values 0/1/2/0/1/2; sections
`["video", "rendering"]` and keys in the order of the original; `ConfigFile.load` of a nonexistent file →
`7` (`ERR_FILE_NOT_FOUND`) without an `ERROR` line; **`save_settings` rewrote the `.ini` byte-for-byte
equal to the baseline** (`diff` empty — SC-003); `ssao_quality` −1 → `ssao_enabled=false`
(**FR-024**), MEDIUM → true, HIGH → true; `scale_filter = 5` → `root.scaling_3d_mode = 5` without
panic; `shadow_mapping=false` → `DirectionalLight3D` child of the `scene_root` with
`shadow_enabled=false` (`propagate_call`). The `WARNING … ObjectDB instances were leaked` /
`RID allocations … leaked` at the end belong to the probe itself (Environment/Node created and not freed),
not to the port.

### E.3b Probe B (`-s`, `.ini` **absent** — moved and restored; final md5 equal)

Programmatic comparison of the draft's 15 defaults (via `config_file` after `load_settings`)
with `load("res://menu/settings.gd").new().DEFAULTS`: **15/15 equal in value and `typeof`**
(`int`/`float`/`bool`); order of sections and keys identical; `FileAccess.file_exists("user://settings.ini")`
= **false** after boot (the original does not write in `load_settings` either).

### E.4 Probe C (`-s`, autoload = **original** `settings.gd`) — bug reproduction

`class=Node script=res://menu/settings.gd`; `ssao_quality = -1` → `apply_graphics_settings` →
**`ssao_enabled = true`** (bug); `ssil_quality = -1` → `ssil_enabled = false` (correct). Basis
for entry #3 of `docs/upstream-bugs.md`.

## Candidate v2 backlog (record in `docs/v2-backlog.md` in the port commit)

| # | Origin | Improvement | Motivation |
|---|---|---|---|
| 25 | `menu/settings.gd` (port 5) | Review the SSAO mapping: "Medium" applies `ENV_SSAO_QUALITY_HIGH` without `half_size` and "High"/others apply `MEDIUM` with `half_size` (SSIL does the opposite, consistent with the names) | Ambiguous intent in the upstream (swap of names or of constants); v1 preserves it for fidelity — only the `if`→`elif` of "Disabled" was fixed (upstream-bugs #3) |

## Entry #3 of `docs/upstream-bugs.md` (text to record in the commit; Commit column = commit subject)

| # | Defect | Script / scene | Fix applied | Commit |
|---|---|---|---|---|
| 3 | "SSAO: Disabled" did not turn SSAO off — `settings.gd:88-95` did `if ssao_quality == -1: ssao_enabled = false` followed by `if ssao_quality == MEDIUM: … else: ssao_enabled = true …` (without `elif`), so the `else` re-enabled SSAO at medium quality with `half_size` for any value ≠ MEDIUM, including −1. Reproduced headless in the original (`ssao_enabled == true` after applying with −1); the identical SSIL block (`settings.gd:97-104`) uses `elif` and works | `menu/settings.gd:88-95`; declared in `specs/005-v1-settings-autoload` FR-020–FR-024 | `oxide_godot_core/oxide_godot_lib/src/settings.rs` `apply_graphics_settings`: the second test becomes `else if`; comment `// upstream bug fix` on the line above. Branches "medium → high without `half_size`" and "otherwise → medium with `half_size`", parameters and SSIL untouched (naming quirk stays in backlog 25). Headless: −1 → `ssao_enabled = false`; MEDIUM/HIGH → `true` | `Port settings.gd → Settings (Node, autoload); menu/settings.tscn nova; project.godot: Settings="*res://menu/settings.tscn"` |
