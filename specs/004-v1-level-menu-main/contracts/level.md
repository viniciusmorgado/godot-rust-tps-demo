# Contrato: `Level` (port 2)

- Classe registrada: `Level` (nome livre; sem colisão). Base: `Node3D`.
- Node na cena: raiz de `level/level.tscn` (l.40). Instanciado pelo `Menu` via
  `replace_main_scene` → `Main.change_scene_to_packed`.

## Sinal

| Assinatura Godot | Rust | Consumidor |
|---|---|---|
| `signal quit` | `#[signal] fn quit();` (bloco principal) | `main.gd:30-31` — `node.has_signal("quit")` + `node.quit.connect(go_to_main_menu)` (até o port 4; depois `Main` faz o mesmo por nome) |

## Métodos (todos privados — nenhum é chamado por nome fora da classe)

| Original | Rust | Consumidor |
|---|---|---|
| `add_player(id: int, spawn_point: Marker3D = null)` | `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` | `ready` (2 args); `multiplayer.peer_connected` (1 arg) — conectado por closure tipada `|this, id: i64| this.add_player(id as i32, None)` |
| `del_player(id: int)` | `fn del_player(&mut self, id: i32)` | `multiplayer.peer_disconnected` — closure tipada |
| `spawn_robot(spawn_point)` | `fn spawn_robot(&mut self, spawn_point: Gd<Node3D>)` | `ready`; `_respawn_robot` |
| `_respawn_robot(spawn_point)` | `fn _respawn_robot(&mut self, spawn_point: Gd<Node3D>)` | `EnemyRobot.exploded` (closure `move` com o ponto — o `.bind()` do original) |
| `setup_sdfgi/voxelgi/lightmapgi` | privados | `ready` |

Nota: o original conecta `add_player`/`del_player` como `Callable`s por referência de método
(parâmetro default cobre o 2º argumento). gdext não tem parâmetro default em `#[func]`; a
conexão tipada por closure preserva o efeito (spec FR-008). Backlog v2 item 21.

## Propriedades

Nenhuma exportada/replicada. `lightmap_gi` é interno.

## O que o Level consome das classes Rust (visibilidade `pub(crate)`, research D2)

`EnemyRobot::exploded` (sinal → `pub(crate) fn exploded();` em `red_robot.rs`);
`Player::set_player_id` (→ `pub(crate)` em `player.rs`). Ambos só a palavra de visibilidade, no
commit do port 2.

## Chamadas dinâmicas (exceção `Settings`)

`get_node("/root/Settings")` → `.call("apply_graphics_settings", [window, environment, self])`,
`.get("config_file")` (`gi_type`, `gi_quality`).

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'has_signal(&"quit")\|node.quit.connect' main/main.gd                  # l.30-31 (até o port 4)
grep -n 'spawn_path\|_spawnable_scenes' level/level.tscn                       # l.74-75 — MultiplayerSpawner intocado
grep -n 'name="RobotSpawnpoints"\|name="PlayerSpawnpoints"\|name="SpawnedNodes"\|name="WorldEnvironment"\|name="VoxelGI"\|name="ReflectionProbes"' level/level.tscn   # 6 nodes
grep -c 'type="Level"' level/level.tscn                                        # 1
grep -c 'ExtResource("1")' level/level.tscn                                    # 0
git diff HEAD -- ../oxide_godot_core/oxide_godot_lib/src/player.rs ../oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'   # exatamente 4 linhas (2 pares)
```
