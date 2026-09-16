# Contract: `Settings` (port 5 — autoload, last script of v1)

- Registered class: `Settings`. Base: `Node`. No collision (research D2).
- Node: `/root/Settings`, created by the autoload `project.godot:25` → `res://menu/settings.tscn`
  (root `name="Settings" type="Settings"`). The node **name** comes from the autoload key.
- Module: `oxide_godot_core/oxide_godot_lib/src/settings.rs` (`mod settings;`).

## Exposed properties (`#[var]`, usage `NONE`)

| Original | Rust | Consumers (dynamic, untouched) |
|---|---|---|
| `var config_file: ConfigFile` | `#[var] config_file: Gd<ConfigFile>` | `get("config_file")` in `bullet.rs:73`, `flying_forklift.rs:20`, `level.rs:49,117,145,174`, `menu.rs:309,480`, `main_scene.rs:30` (8 sites) — all `.to::<Gd<ConfigFile>>()` |
| `var metalfx_supported: bool` | `#[var] metalfx_supported: bool` | none (the menu has its own) |

`DEFAULTS` is **not** exposed (private field `defaults`; no consumer).

## Exposed methods (`#[func]` — targets of `call` by name)

| Original | Rust | Who calls |
|---|---|---|
| `load_settings()` | `#[func] fn load_settings(&mut self)` | `ready` (internal); exposed for fidelity (it was a public method of the autoload) |
| `save_settings()` | `#[func] fn save_settings(&mut self)` | `menu.rs:611` `settings.call("save_settings", &[])` |
| `apply_graphics_settings(window: Window, environment: Environment, scene_root: Node)` | `#[func] fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)` | `level.rs:44-47` and `menu.rs:208-211, 606-610`: `call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()])` — 3 arguments |

No other `#[func]`. `_input` is virtual (`fn input`), not exposed.

## Constants (`#[constant]`, i64) — surface of the `GIType`/`GIQuality` enums

| Original | Rust | Value |
|---|---|---|
| `GIType.SDFGI` | `GI_TYPE_SDFGI` | 0 |
| `GIType.VOXEL_GI` | `GI_TYPE_VOXEL_GI` | 1 |
| `GIType.LIGHTMAP_GI` | `GI_TYPE_LIGHTMAP_GI` | 2 |
| `GIQuality.DISABLED` | `GI_QUALITY_DISABLED` | 0 |
| `GIQuality.LOW` | `GI_QUALITY_LOW` | 1 |
| `GIQuality.HIGH` | `GI_QUALITY_HIGH` | 2 |

No consumer reads them by name (`level.rs` uses local constants — backlog 1).

## Input

Action `toggle_fullscreen` (`project.godot:172-177`, F11 + Alt+Enter) handled in `fn input`;
`set_input_as_handled()` on the viewport.

## File

`user://settings.ini` — sections/keys/types/defaults in [data-model.md](../data-model.md).
Compatible in both directions with the original `settings.gd` (same integers/bools/floats, same
order).

## Verification before the commit

```bash
cd oxide-godot
grep -n '^Settings=' project.godot                                          # 25:Settings="*res://menu/settings.tscn"
cat menu/settings.tscn                                                      # 3 lines; type="Settings"
find . -name '*.gd' -not -path '*/addons/*'; find . -name '*.gd.uid' -not -path '*/addons/*'   # both empty
grep -rn 'b04fekxdgdq0k\|menu/settings.gd' . | grep -v '/.godot/'           # empty
R=../oxide_godot_core/oxide_godot_lib/src/settings.rs
grep -n '#\[func\]' -A1 $R | grep 'fn '                                     # load_settings, save_settings, apply_graphics_settings (3)
grep -n '#\[var\]' -A1 $R | grep -oE '(metalfx_supported|config_file)'      # the 2
grep -c '#\[constant\]' $R                                                  # 6
grep -n 'upstream bug fix' $R                                               # 1 line, immediately above the SSAO `} else if`
grep -n 'else if' $R                                                        # 2 (SSAO and SSIL)
git diff --stat HEAD -- ../oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs   # empty
```
