# Data Model: Milestone D — forklift, level, menu and main

**Phase**: v1 — Raw Port. State of each class exactly as it exists in GDScript; nothing is remodeled.
Signatures and decisions in [research.md](research.md); names consumed by scripts/scenes in
[contracts/](contracts/).

## FlyingForklift (`level/forklift/flying_forklift.gd` → `src/flying_forklift.rs`, base **`CharacterBody3D`**)

Base = the root node's type (`flying_forklift.tscn:36`), not the script's `extends Node3D`
(constitution v1.3.1, Principle II). Instantiated by `level.tscn:8`.

| Field | Rust type | Role |
|---|---|---|
| `spot_light` | `OnReady<Gd<SpotLight3D>>` (`SpotLight3D`) | Shadow turned off in `ready` if `rendering/shadow_mapping` is false |

No state beyond the reference; no external contract. `ready`: `randomize()`, picks
`floor(randf × n)` among the *n* children of child 0 (`FlyingForkliftModel2`) and leaves only it visible.

## Level (`level/level.gd` → `src/level.rs`, base `Node3D`)

### Contract

| Name | Type | Rust | Who uses it |
|---|---|---|---|
| `quit` | signal (0 args) | `#[signal] fn quit();` | `main.gd:30-31` (`has_signal` + connection by name) → `Main` (port 4, same form) |
| `add_player(id, spawn_point = null)` | method | private `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` | `peer_connected` (typed closure); `ready` |
| `del_player(id)` | method | private `fn del_player(&mut self, id: i32)` | `peer_disconnected` (typed closure) |
| `spawn_robot(spawn_point)`, `_respawn_robot(spawn_point)` | methods | private, `Gd<Node3D>` | `ready`; the robot's `exploded` signal (`move` closure); 15 s timer |
| `setup_sdfgi/voxelgi/lightmapgi` | methods | private | `ready` |

### Internal state

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `lightmap_gi` | `Option<Gd<LightmapGi>>` | `None` | Created by `setup_lightmapgi`; `queue_free` (keeping the `Some`) in the other setups |
| `world_environment` | `OnReady<Gd<WorldEnvironment>>` | `WorldEnvironment` | environment for `apply_graphics_settings` and SDFGI |
| `robot_spawn_points` / `player_spawn_points` / `spawned_nodes` | `OnReady<Gd<Node3D>>` | `RobotSpawnpoints` / `PlayerSpawnpoints` / `SpawnedNodes` | 4 robot points; 4 `Marker3D`; parent of the spawns (replicated by the `MultiplayerSpawner`, `level.tscn:73-75`) |

Local constants (`i64`, values from `settings.gd`): `SDFGI = 0`, `VOXEL_GI = 1`; `GI_DISABLED = 0`,
`GI_LOW = 1`, `GI_HIGH = 2`.

### `ready` flow

```
apply_graphics_settings(window, env, self)  [dynamic, Settings]
gi_type: 0 → setup_sdfgi | 1 → setup_voxelgi | * → setup_lightmapgi
server?  ── no ──▶ end
  │ yes
  ▼ spawn_robot × children(RobotSpawnpoints)  (typed EnemyRobot; exploded → _respawn_robot(point) → 15 s → spawn_robot)
  ▼ randomize; shuffle(PlayerSpawnpoints); add_player(1, pop); add_player(id, pop) × peers
  ▼ peer_connected → add_player(id, None); peer_disconnected → del_player(id)
input(quit) → mouse visible; emit quit
```

### Relationships

- **Consumes (typed)**: `EnemyRobot` (`signals().exploded()`), `Player` (`set_player_id`).
- **Consumes (dynamic, exception)**: `Settings` (`apply_graphics_settings`, `config_file`).
- **Is consumed by**: `main.gd`/`Main` (`quit`), `settings.gd` (receives the level as `scene_root`).

## Menu (`menu/menu.gd` → `src/menu.rs`, base `Node`)

### Contract

| Name | Type | Rust | Who uses it |
|---|---|---|---|
| `replace_main_scene(scene)` | signal (1 arg `PackedScene`) | `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);` | `main.gd:32-33` / `Main` |
| 9 handlers | `#[func]` | `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`, `_on_apply_pressed`, `_on_loading_done_timer_timeout` | 10 `[connection]` (`menu.tscn:836-845`); `call_deferred("_on_host_pressed")` in headless |
| `_make_button_group(parent)` | internal method | private | `ready` ×15 |

### Internal state

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `peer` | `Gd<MultiplayerPeer>` | `OfflineMultiplayerPeer` | Replaced by `ENetMultiplayerPeer` on Host/Connect; handed to the `MultiplayerAPI` at the end of loading |
| `metalfx_supported` | `bool` | driver == "metal" | Hides MetalFX buttons; filter fallback |
| 85 UI references | `OnReady<Gd<T>>` | paths from `menu.gd:11-104` | [contracts/menu.md](contracts/menu.md) |

Constant `LEVEL_PATH = "res://level/level.tscn"`.

### Flow

```
ready: apply_graphics_settings; headless → call_deferred(_on_host_pressed); focus Play; MetalFX hidden; 15 ButtonGroups
Play ──▶ Main hidden, Loading visible, load_threaded_request(level, sub-threads)
process (Loading visible): IN_PROGRESS → bar; LOADED → 100, process off, DoneTimer(0.5 s); error → print, Main
DoneTimer ──▶ multiplayer_peer = peer; emit replace_main_scene(load_threaded_get(level))
Settings ──▶ buttons reflect config_file; Apply writes + apply_graphics_settings + save_settings; Cancel/Back go back
Play Online ──▶ Host: ENet server(port) → Play | Connect: ENet client(address, port) → Play
Quit ──▶ get_tree().quit()
```

### Saved options (`user://settings.ini`, via `Settings.config_file`)

| Section/key | Type | Values (engine integers / `settings.gd`) |
|---|---|---|
| `video/display_mode` | int | `Window.MODE_WINDOWED` 0, `FULLSCREEN` 3, `EXCLUSIVE_FULLSCREEN` 4 (reading treats `MAXIMIZED` 2 as Windowed) |
| `video/vsync` | int | `DisplayServer.VSYNC_DISABLED` 0, `ENABLED` 1, `ADAPTIVE` 2, `MAILBOX` 3 |
| `video/max_fps` | int | 30, 40, 60, 72, 90, 120, 144, 0 (unlimited) |
| `video/resolution_scale` | float | 1/3, 1/2, 1/1.7, 1/1.5, 1/1.3, 1.0 (read via `is_equal_approx`) |
| `video/scale_filter` | int | `BILINEAR` 0, `FSR` 1, `FSR2` 2, `METALFX_SPATIAL` 3, `METALFX_TEMPORAL` 4 (via `.ord()`), `NEAREST` 5 (local constant — absent from the prebuilt 4.6 API; Godot 4.7.2) |
| `rendering/gi_type` | int | 2 LightmapGI, 1 VoxelGI, 0 SDFGI |
| `rendering/gi_quality` | int | 0 Disabled, 1 Low, 2 High |
| `rendering/taa`, `shadow_mapping`, `bloom`, `volumetric_fog` | bool | "Enabled" button pressed |
| `rendering/msaa` | int | `Viewport.MSAA_DISABLED` 0, `2X` 1, `4X` 2, `8X` 3 |
| `rendering/screen_space_aa` | int | `DISABLED` 0, `FXAA` 1, `SMAA` 2 |
| `rendering/ssao_quality`, `ssil_quality` | int | −1 off, `ENV_SS*_QUALITY_MEDIUM`, `HIGH` |

(The exact integers come from `.ord()` of the gdext enums — research D9 §E.1 confirms the ones that
matter; `settings.gd` reads the same ones.)

## Main (`main/main.gd` → `src/main_scene.rs`, base `Node`)

| Name | Rust | Who uses it |
|---|---|---|
| `go_to_main_menu()` | `#[func]` | `ready`; `connect("quit", …)` by name |
| `replace_main_scene(resource)` | `#[func]` | `connect("replace_main_scene", …)` by name |
| `change_scene_to_packed(resource)` | `#[func]` | `call_deferred` **by name** in `replace_main_scene`; `go_to_main_menu` |

No state of its own. Flow: `ready` → `server_relay = false`, headless → 60 fps, `randomize`,
window mode from `config_file`, `go_to_main_menu` → loads `menu.tscn`, closes the peer, offline
peer, `change_scene_to_packed(menu)` → instantiates, removes/frees children, adds, connects
`quit`/`replace_main_scene` if they exist (`has_signal`).
