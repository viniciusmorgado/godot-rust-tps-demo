# Data Model: Milestone E — Settings autoload (v1 raw port)

One class, one configuration file. API details in [research.md](research.md); consumed names
in [contracts/settings.md](contracts/settings.md).

## Class `Settings` (base `Node`, autoload at `/root/Settings`)

| Field | Rust type | Exposure | Origin | Initial value |
|---|---|---|---|---|
| `metalfx_supported` | `bool` | `#[var]` (usage `NONE`) | `settings.gd:18` | `RenderingServer` current driver == `"metal"` (false on this platform) |
| `defaults` | `VarDictionary` (nested: section → {key → value}) | private | `settings.gd:20-40` (`DEFAULTS`) | table below |
| `config_file` | `Gd<ConfigFile>` | `#[var]` — **consumed by 8 `get("config_file")`** | `settings.gd:42` | `ConfigFile::new_gd()` |

Constants `#[constant]` (i64): `GI_TYPE_SDFGI` 0, `GI_TYPE_VOXEL_GI` 1, `GI_TYPE_LIGHTMAP_GI` 2,
`GI_QUALITY_DISABLED` 0, `GI_QUALITY_LOW` 1, `GI_QUALITY_HIGH` 2 (`settings.gd:3-13`).
`CONFIG_FILE_PATH = "user://settings.ini"` (module const).

Construction order (`init`): `metalfx_supported` → `defaults` → `config_file` (order of the `var`s
in the original).

## File `user://settings.ini` (`ConfigFile`) — sections, keys, types and defaults

Order = insertion order of the defaults (the written `.ini` follows this order — SC-003).

| Section | Key | Type | Default (value) | Engine enum / origin | Who writes/reads |
|---|---|---|---|---|---|
| `video` | `display_mode` | int | 4 | `Window.Mode.EXCLUSIVE_FULLSCREEN` | menu writes; `Main.ready`, `apply_graphics_settings` read |
| `video` | `vsync` | int | 1 | `DisplayServer.VSyncMode.ENABLED` | menu; apply |
| `video` | `max_fps` | int | 0 | unlimited | menu; apply |
| `video` | `resolution_scale` | float | 1.0 | — | menu; apply |
| `video` | `scale_filter` | int | 2 (FSR2) — 4 (`METALFX_TEMPORAL`) if `metalfx_supported` | `Viewport.Scaling3DMode` (5 = Nearest from 4.7, absent from the 4.6 API — ordinal passes straight through) | menu; apply |
| `rendering` | `taa` | bool | false | — | menu; apply |
| `rendering` | `msaa` | int | 0 | `Viewport.MSAA.DISABLED` | menu; apply |
| `rendering` | `screen_space_aa` | int | 0 | `Viewport.ScreenSpaceAA.DISABLED` | menu; apply |
| `rendering` | `shadow_mapping` | bool | true | — | menu; apply (`propagate_call`), `Bullet.explode`, `FlyingForklift.ready` |
| `rendering` | `gi_type` | int | 1 | `GI_TYPE_VOXEL_GI` | menu; `Level.ready` |
| `rendering` | `gi_quality` | int | 1 | `GI_QUALITY_LOW` | menu; `Level.setup_*` |
| `rendering` | `ssao_quality` | int | 2 | `RenderingServer.ENV_SSAO_QUALITY_MEDIUM`; −1 = disabled | menu; apply |
| `rendering` | `ssil_quality` | int | −1 | `RenderingServer.ENV_SSIL_QUALITY_*`; −1 = disabled | menu; apply |
| `rendering` | `bloom` | bool | true | → `Environment.glow_enabled` | menu; apply |
| `rendering` | `volumetric_fog` | bool | true | → `Environment.volumetric_fog_enabled` | menu; apply |

Values and types checked programmatically against `settings.gd` (research §E.3b: 15/15).

## States and transitions

```
construction (init) ──► enters the tree (ready) ──► load_settings
                                                    ├─ config_file.load(CONFIG_FILE_PATH)   [Error ignored; file may not exist]
                                                    └─ for each (section, key) of the defaults that is missing: set_value in memory
                                                       [does NOT write the file]

menu Apply ──► config_file.set_value(…) ×N (in the consumer) ──► call("apply_graphics_settings", window, env, menu)
           └──────────────────────────────────────────────► call("save_settings") ──► config_file.save(CONFIG_FILE_PATH)  [file created/updated]

level/menu ready ──► call("apply_graphics_settings", window, env, root) ──► applies the current state of config_file
                                                                           (own window: mode; window: scale/filter/TAA/MSAA/AA;
                                                                            DisplayServer: vsync; Engine: max_fps;
                                                                            scene_root: propagate_call("set", ["shadow_enabled", false]) if shadows off;
                                                                            environment: SSAO (fixed), SSIL, glow, fog)

F11 / Alt+Enter ──► _input ──► own window: EXCLUSIVE_FULLSCREEN ⇄ WINDOWED; set_input_as_handled
```

## SSAO decision table (`apply_graphics_settings`)

| `ssao_quality` | Original (`if`/`if`/`else`) | Port (`if`/`else if`/`else`) — FR-020–FR-024 |
|---|---|---|
| −1 | `ssao_enabled` = false **and then** true + MEDIUM/half_size (bug) | `ssao_enabled` = **false** |
| 2 (MEDIUM) | true + `HIGH`, half_size=false, 0.5, 2, 50, 300 | same (quirk preserved — backlog 25) |
| other (3 = HIGH, …) | true + `MEDIUM`, half_size=true, 0.5, 2, 50, 300 | same |

SSIL (`if`/`elif`/`else` in the original — faithful): −1 → false; 2 → true + MEDIUM/false; other → true + HIGH/true.

## Autoload binding

| Artifact | Content |
|---|---|
| `oxide-godot/menu/settings.tscn` (new) | `[gd_scene format=3]` / `` / `[node name="Settings" type="Settings"]` |
| `oxide-godot/project.godot:25` | `Settings="*res://menu/settings.tscn"` |
| `oxide-godot/menu/settings.gd`, `settings.gd.uid` (`uid://b04fekxdgdq0k`) | deleted |
