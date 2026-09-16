# Data Model: Marco D — empilhadeira, level, menu e main

**Fase**: v1 — Raw Port. Estado de cada classe tal como existe no GDScript; nada é remodelado.
Assinaturas e decisões em [research.md](research.md); nomes consumidos por scripts/cenas em
[contracts/](contracts/).

## FlyingForklift (`level/forklift/flying_forklift.gd` → `src/flying_forklift.rs`, base **`CharacterBody3D`**)

Base = tipo do node raiz (`flying_forklift.tscn:36`), não o `extends Node3D` do script
(constituição v1.3.1, Princípio II). Instanciada por `level.tscn:8`.

| Campo | Tipo Rust | Papel |
|---|---|---|
| `spot_light` | `OnReady<Gd<SpotLight3D>>` (`SpotLight3D`) | Sombra desligada em `ready` se `rendering/shadow_mapping` for falso |

Sem estado além da referência; sem contrato externo. `ready`: `randomize()`, sorteia
`floor(randf × n)` entre os *n* filhos do filho 0 (`FlyingForkliftModel2`) e deixa só ele visível.

## Level (`level/level.gd` → `src/level.rs`, base `Node3D`)

### Contrato

| Nome | Tipo | Rust | Quem usa |
|---|---|---|---|
| `quit` | sinal (0 args) | `#[signal] fn quit();` | `main.gd:30-31` (`has_signal` + conexão por nome) → `Main` (port 4, mesma forma) |
| `add_player(id, spawn_point = null)` | método | privado `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` | `peer_connected` (closure tipada); `ready` |
| `del_player(id)` | método | privado `fn del_player(&mut self, id: i32)` | `peer_disconnected` (closure tipada) |
| `spawn_robot(spawn_point)`, `_respawn_robot(spawn_point)` | métodos | privados, `Gd<Node3D>` | `ready`; sinal `exploded` do robô (closure `move`); timer 15 s |
| `setup_sdfgi/voxelgi/lightmapgi` | métodos | privados | `ready` |

### Estado interno

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `lightmap_gi` | `Option<Gd<LightmapGi>>` | `None` | Criado por `setup_lightmapgi`; `queue_free` (mantendo o `Some`) nos outros setups |
| `world_environment` | `OnReady<Gd<WorldEnvironment>>` | `WorldEnvironment` | ambiente para `apply_graphics_settings` e SDFGI |
| `robot_spawn_points` / `player_spawn_points` / `spawned_nodes` | `OnReady<Gd<Node3D>>` | `RobotSpawnpoints` / `PlayerSpawnpoints` / `SpawnedNodes` | 4 pontos de robô; 4 `Marker3D`; pai dos spawns (replicado pelo `MultiplayerSpawner`, `level.tscn:73-75`) |

Constantes locais (`i64`, valores do `settings.gd`): `SDFGI = 0`, `VOXEL_GI = 1`; `GI_DISABLED = 0`,
`GI_LOW = 1`, `GI_HIGH = 2`.

### Fluxo de `ready`

```
apply_graphics_settings(window, env, self)  [dinâmico, Settings]
gi_type: 0 → setup_sdfgi | 1 → setup_voxelgi | * → setup_lightmapgi
servidor?  ── não ──▶ fim
  │ sim
  ▼ spawn_robot × filhos(RobotSpawnpoints)  (EnemyRobot tipado; exploded → _respawn_robot(ponto) → 15 s → spawn_robot)
  ▼ randomize; shuffle(PlayerSpawnpoints); add_player(1, pop); add_player(id, pop) × peers
  ▼ peer_connected → add_player(id, None); peer_disconnected → del_player(id)
input(quit) → mouse visível; emit quit
```

### Relações

- **Consome (tipado)**: `EnemyRobot` (`signals().exploded()`), `Player` (`set_player_id`).
- **Consome (dinâmico, exceção)**: `Settings` (`apply_graphics_settings`, `config_file`).
- **É consumido por**: `main.gd`/`Main` (`quit`), `settings.gd` (recebe o level como `scene_root`).

## Menu (`menu/menu.gd` → `src/menu.rs`, base `Node`)

### Contrato

| Nome | Tipo | Rust | Quem usa |
|---|---|---|---|
| `replace_main_scene(scene)` | sinal (1 arg `PackedScene`) | `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);` | `main.gd:32-33` / `Main` |
| 9 handlers | `#[func]` | `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`, `_on_apply_pressed`, `_on_loading_done_timer_timeout` | 10 `[connection]` (`menu.tscn:836-845`); `call_deferred("_on_host_pressed")` em headless |
| `_make_button_group(parent)` | método interno | privado | `ready` ×15 |

### Estado interno

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `peer` | `Gd<MultiplayerPeer>` | `OfflineMultiplayerPeer` | Substituído por `ENetMultiplayerPeer` em Host/Connect; entregue ao `MultiplayerAPI` no fim do loading |
| `metalfx_supported` | `bool` | driver == "metal" | Oculta botões MetalFX; fallback do filtro |
| 85 referências de UI | `OnReady<Gd<T>>` | caminhos de `menu.gd:11-104` | [contracts/menu.md](contracts/menu.md) |

Constante `LEVEL_PATH = "res://level/level.tscn"`.

### Fluxo

```
ready: apply_graphics_settings; headless → call_deferred(_on_host_pressed); foco Play; MetalFX ocultos; 15 ButtonGroups
Play ──▶ Main oculto, Loading visível, load_threaded_request(level, sub-threads)
process (Loading visível): IN_PROGRESS → barra; LOADED → 100, process off, DoneTimer(0,5 s); erro → print, Main
DoneTimer ──▶ multiplayer_peer = peer; emit replace_main_scene(load_threaded_get(level))
Settings ──▶ botões refletem config_file; Apply grava + apply_graphics_settings + save_settings; Cancel/Back voltam
Play Online ──▶ Host: ENet server(porta) → Play | Connect: ENet client(endereço, porta) → Play
Quit ──▶ get_tree().quit()
```

### Opções gravadas (`user://settings.ini`, via `Settings.config_file`)

| Seção/chave | Tipo | Valores (inteiros do engine / `settings.gd`) |
|---|---|---|
| `video/display_mode` | int | `Window.MODE_WINDOWED` 0, `FULLSCREEN` 3, `EXCLUSIVE_FULLSCREEN` 4 (leitura trata `MAXIMIZED` 2 como Windowed) |
| `video/vsync` | int | `DisplayServer.VSYNC_DISABLED` 0, `ENABLED` 1, `ADAPTIVE` 2, `MAILBOX` 3 |
| `video/max_fps` | int | 30, 40, 60, 72, 90, 120, 144, 0 (ilimitado) |
| `video/resolution_scale` | float | 1/3, 1/2, 1/1,7, 1/1,5, 1/1,3, 1,0 (leitura por `is_equal_approx`) |
| `video/scale_filter` | int | `Viewport.SCALING_3D_MODE_NEAREST` 0, `BILINEAR` 1, `FSR` 2*, `FSR2` 2*, `METALFX_SPATIAL`, `METALFX_TEMPORAL` (*valores conforme o engine; usar `.ord()`) |
| `rendering/gi_type` | int | 2 LightmapGI, 1 VoxelGI, 0 SDFGI |
| `rendering/gi_quality` | int | 0 Disabled, 1 Low, 2 High |
| `rendering/taa`, `shadow_mapping`, `bloom`, `volumetric_fog` | bool | botão "Enabled" pressionado |
| `rendering/msaa` | int | `Viewport.MSAA_DISABLED` 0, `2X` 1, `4X` 2, `8X` 3 |
| `rendering/screen_space_aa` | int | `DISABLED` 0, `FXAA` 1, `SMAA` 2 |
| `rendering/ssao_quality`, `ssil_quality` | int | −1 desligado, `ENV_SS*_QUALITY_MEDIUM`, `HIGH` |

(Os inteiros exatos vêm de `.ord()` dos enums gdext — research D9 §E.1 confirma os que
importam; o `settings.gd` lê os mesmos.)

## Main (`main/main.gd` → `src/main_scene.rs`, base `Node`)

| Nome | Rust | Quem usa |
|---|---|---|
| `go_to_main_menu()` | `#[func]` | `ready`; `connect("quit", …)` por nome |
| `replace_main_scene(resource)` | `#[func]` | `connect("replace_main_scene", …)` por nome |
| `change_scene_to_packed(resource)` | `#[func]` | `call_deferred` **por nome** em `replace_main_scene`; `go_to_main_menu` |

Sem estado próprio. Fluxo: `ready` → `server_relay = false`, headless → 60 fps, `randomize`,
modo da janela do `config_file`, `go_to_main_menu` → carrega `menu.tscn`, fecha o peer, peer
offline, `change_scene_to_packed(menu)` → instancia, remove/libera filhos, adiciona, conecta
`quit`/`replace_main_scene` se existirem (`has_signal`).
