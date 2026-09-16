# Contract: `Level` (port 2)

- Registered class: `Level` (free name; no collision). Base: `Node3D`.
- Node in the scene: root of `level/level.tscn` (l.40). Instantiated by the `Menu` via
  `replace_main_scene` → `Main.change_scene_to_packed`.

## Signal

| Godot signature | Rust | Consumer |
|---|---|---|
| `signal quit` | `#[signal] fn quit();` (main block) | `main.gd:30-31` — `node.has_signal("quit")` + `node.quit.connect(go_to_main_menu)` (until port 4; afterwards `Main` does the same by name) |

## Methods (all private — none is called by name outside the class)

| Original | Rust | Consumer |
|---|---|---|
| `add_player(id: int, spawn_point: Marker3D = null)` | `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` | `ready` (2 args); `multiplayer.peer_connected` (1 arg) — connected via typed closure `|this, id: i64| this.add_player(id as i32, None)` |
| `del_player(id: int)` | `fn del_player(&mut self, id: i32)` | `multiplayer.peer_disconnected` — typed closure |
| `spawn_robot(spawn_point)` | `fn spawn_robot(&mut self, spawn_point: Gd<Node3D>)` | `ready`; `_respawn_robot` |
| `_respawn_robot(spawn_point)` | `fn _respawn_robot(&mut self, spawn_point: Gd<Node3D>)` | `EnemyRobot.exploded` (`move` closure with the point — the original's `.bind()`) |
| `setup_sdfgi/voxelgi/lightmapgi` | private | `ready` |

Note: the original connects `add_player`/`del_player` as `Callable`s by method reference
(the default parameter covers the 2nd argument). gdext has no default parameter in `#[func]`; the
typed connection via closure preserves the effect (spec FR-008). v2 backlog item 21.

## Properties

None exported/replicated. `lightmap_gi` is internal.

## What the Level consumes from the Rust classes (`pub(crate)` visibility, research D2)

`EnemyRobot::exploded` (signal → `pub(crate) fn exploded();` in `red_robot.rs`);
`Player::set_player_id` (→ `pub(crate)` in `player.rs`). Both only the visibility keyword, in
the port 2 commit.

## Dynamic calls (`Settings` exception)

`get_node("/root/Settings")` → `.call("apply_graphics_settings", [window, environment, self])`,
`.get("config_file")` (`gi_type`, `gi_quality`).

## Verification before the commit

```bash
cd oxide-godot
grep -n 'has_signal(&"quit")\|node.quit.connect' main/main.gd                  # l.30-31 (until port 4)
grep -n 'spawn_path\|_spawnable_scenes' level/level.tscn                       # l.74-75 — MultiplayerSpawner untouched
grep -n 'name="RobotSpawnpoints"\|name="PlayerSpawnpoints"\|name="SpawnedNodes"\|name="WorldEnvironment"\|name="VoxelGI"\|name="ReflectionProbes"' level/level.tscn   # 6 nodes
grep -c 'type="Level"' level/level.tscn                                        # 1
grep -c 'ExtResource("1")' level/level.tscn                                    # 0
git diff HEAD -- ../oxide_godot_core/oxide_godot_lib/src/player.rs ../oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'   # exactly 4 lines (2 pairs)
```
