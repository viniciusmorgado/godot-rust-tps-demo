# Contrato: `Menu` (port 3)

Superfície consumida por `menu.tscn` (10 conexões, 85 caminhos de node) e por `main.gd`/`Main`
(sinal `replace_main_scene`). Nomes e caminhos são contrato: copiar do original, nunca traduzir.

- Classe registrada: `Menu` (nome livre; conferido sem colisão no engine e nos `.gd`).
- Base: `Node`. Node na cena: raiz de `menu/menu.tscn` (l.103).

## Sinal

| Assinatura Godot | Rust | Consumidor |
|---|---|---|
| `signal replace_main_scene` — declarado sem parâmetro, **emitido com 1** (`menu.gd:163`) | `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);` — parâmetro declarado (spec Edge Cases) | `main.gd:32-33` — `has_signal` + `connect(replace_main_scene)`, método de 1 argumento |

## Handlers (`#[func]`) — 10 conexões em `menu.tscn:836-845`

| Método | Conectado a | Efeito |
|---|---|---|
| `_on_play_pressed` | `UI/Main/Play.pressed` (l.836); chamado por Host/Connect | Main oculto, Loading visível, `load_threaded_request` do level |
| `_on_play_online_pressed` | `UI/Main/PlayOnline.pressed` (l.837) | Online visível, Main oculto |
| `_on_settings_pressed` | `UI/Main/Settings.pressed` (l.838) | Settings visível; botões refletem `config_file` |
| `_on_quit_pressed` | `UI/Main/Quit.pressed` (l.839) | `get_tree().quit()` |
| `_on_host_pressed` | `UI/Online/Host.pressed` (l.840); `call_deferred` em headless | ENet server → Play |
| `_on_connect_pressed` | `UI/Online/Connect.pressed` (l.841) | ENet client → Play |
| `_on_cancel_pressed` | `UI/Online/Back.pressed` (l.842) **e** `UI/Settings/Actions/Cancel.pressed` (l.844) | Main visível, Settings e Online ocultos |
| `_on_apply_pressed` | `UI/Settings/Actions/Apply.pressed` (l.843) | grava `config_file`, `apply_graphics_settings`, `save_settings` |
| `_on_loading_done_timer_timeout` | `UI/Loading/DoneTimer.timeout` (l.845; 0,5 s one-shot) | `multiplayer_peer = peer`; emite `replace_main_scene` |

Interno (privado, sem `#[func]`): `_make_button_group(common_parent)`.

## Estado

`peer: Gd<MultiplayerPeer>` (inicial `OfflineMultiplayerPeer`), `metalfx_supported: bool`,
`LEVEL_PATH = "res://level/level.tscn"`. Nenhuma propriedade exportada/replicada.

## Referências de cena (85 `@onready`, `menu.gd:11-104`) — `#[init(node = "<caminho>")] OnReady<Gd<T>>`

| Campo | Tipo | Caminho (a partir do `Menu`) |
|---|---|---|
| `world_environment` | `WorldEnvironment` | `WorldEnvironment` |
| `ui` | `Control` | `UI` |
| `main` | `Control` | `UI/Main` |
| `play_button` | `Button` | `UI/Main/Play` |
| `settings_button` | `Button` | `UI/Main/Settings` |
| `quit_button` | `Button` | `UI/Main/Quit` |
| `online` | `Control` | `UI/Online` |
| `online_port` | `SpinBox` | `UI/Online/Port` |
| `online_address` | `LineEdit` | `UI/Online/Address` |
| `settings_menu` | `VBoxContainer` | `UI/Settings` |
| `settings_actions` | `HBoxContainer` | `UI/Settings/Actions` |
| `settings_action_apply` | `Button` | `UI/Settings/Actions/Apply` |
| `settings_action_cancel` | `Button` | `UI/Settings/Actions/Cancel` |
| `display_mode_menu` | `HBoxContainer` | `UI/Settings/DisplayMode` |
| `display_mode_windowed` | `Button` | `UI/Settings/DisplayMode/Windowed` |
| `display_mode_fullscreen` | `Button` | `UI/Settings/DisplayMode/Fullscreen` |
| `display_mode_exclusive_fullscreen` | `Button` | `UI/Settings/DisplayMode/ExclusiveFullscreen` |
| `vsync_menu` | `HBoxContainer` | `UI/Settings/VSync` |
| `vsync_disabled` | `Button` | `UI/Settings/VSync/Disabled` |
| `vsync_enabled` | `Button` | `UI/Settings/VSync/Enabled` |
| `vsync_adaptive` | `Button` | `UI/Settings/VSync/Adaptive` |
| `vsync_mailbox` | `Button` | `UI/Settings/VSync/Mailbox` |
| `max_fps_menu` | `HBoxContainer` | `UI/Settings/MaxFPS` |
| `max_fps_30` | `Button` | `UI/Settings/MaxFPS/30` |
| `max_fps_40` | `Button` | `UI/Settings/MaxFPS/40` |
| `max_fps_60` | `Button` | `UI/Settings/MaxFPS/60` |
| `max_fps_72` | `Button` | `UI/Settings/MaxFPS/72` |
| `max_fps_90` | `Button` | `UI/Settings/MaxFPS/90` |
| `max_fps_120` | `Button` | `UI/Settings/MaxFPS/120` |
| `max_fps_144` | `Button` | `UI/Settings/MaxFPS/144` |
| `max_fps_unlimited` | `Button` | `UI/Settings/MaxFPS/Unlimited` |
| `resolution_scale_menu` | `HBoxContainer` | `UI/Settings/ResolutionScale` |
| `resolution_scale_ultra_performance` | `Button` | `UI/Settings/ResolutionScale/UltraPerformance` |
| `resolution_scale_performance` | `Button` | `UI/Settings/ResolutionScale/Performance` |
| `resolution_scale_balanced` | `Button` | `UI/Settings/ResolutionScale/Balanced` |
| `resolution_scale_quality` | `Button` | `UI/Settings/ResolutionScale/Quality` |
| `resolution_scale_ultra_quality` | `Button` | `UI/Settings/ResolutionScale/UltraQuality` |
| `resolution_scale_native` | `Button` | `UI/Settings/ResolutionScale/Native` |
| `scale_filter_menu` | `HBoxContainer` | `UI/Settings/ScaleFilter` |
| `scale_filter_nearest` | `Button` | `UI/Settings/ScaleFilter/Nearest` |
| `scale_filter_bilinear` | `Button` | `UI/Settings/ScaleFilter/Bilinear` |
| `scale_filter_fsr1` | `Button` | `UI/Settings/ScaleFilter/FSR1` |
| `scale_filter_metalfx_spatial` | `Button` | `UI/Settings/ScaleFilter/MetalFXSpatial` |
| `scale_filter_fsr2` | `Button` | `UI/Settings/ScaleFilter/FSR2` |
| `scale_filter_metalfx_temporal` | `Button` | `UI/Settings/ScaleFilter/MetalFXTemporal` |
| `taa_menu` | `HBoxContainer` | `UI/Settings/TAA` |
| `taa_disabled` | `Button` | `UI/Settings/TAA/Disabled` |
| `taa_enabled` | `Button` | `UI/Settings/TAA/Enabled` |
| `msaa_menu` | `HBoxContainer` | `UI/Settings/MSAA` |
| `msaa_disabled` | `Button` | `UI/Settings/MSAA/Disabled` |
| `msaa_2x` | `Button` | `UI/Settings/MSAA/2X` |
| `msaa_4x` | `Button` | `UI/Settings/MSAA/4X` |
| `msaa_8x` | `Button` | `UI/Settings/MSAA/8X` |
| `screen_space_aa_menu` | `HBoxContainer` | `UI/Settings/ScreenSpaceAA` |
| `screen_space_aa_disabled` | `Button` | `UI/Settings/ScreenSpaceAA/Disabled` |
| `screen_space_aa_fxaa` | `Button` | `UI/Settings/ScreenSpaceAA/FXAA` |
| `screen_space_aa_smaa` | `Button` | `UI/Settings/ScreenSpaceAA/SMAA` |
| `shadow_mapping_menu` | `HBoxContainer` | `UI/Settings/ShadowMapping` |
| `shadow_mapping_disabled` | `Button` | `UI/Settings/ShadowMapping/Disabled` |
| `shadow_mapping_enabled` | `Button` | `UI/Settings/ShadowMapping/Enabled` |
| `gi_type_menu` | `HBoxContainer` | `UI/Settings/GIType` |
| `gi_lightmapgi` | `Button` | `UI/Settings/GIType/LightmapGI` |
| `gi_voxelgi` | `Button` | `UI/Settings/GIType/VoxelGI` |
| `gi_sdfgi` | `Button` | `UI/Settings/GIType/SDFGI` |
| `gi_quality_menu` | `HBoxContainer` | `UI/Settings/GIQuality` |
| `gi_disabled` | `Button` | `UI/Settings/GIQuality/Disabled` |
| `gi_low` | `Button` | `UI/Settings/GIQuality/Low` |
| `gi_high` | `Button` | `UI/Settings/GIQuality/High` |
| `ssao_menu` | `HBoxContainer` | `UI/Settings/SSAO` |
| `ssao_disabled` | `Button` | `UI/Settings/SSAO/Disabled` |
| `ssao_medium` | `Button` | `UI/Settings/SSAO/Medium` |
| `ssao_high` | `Button` | `UI/Settings/SSAO/High` |
| `ssil_menu` | `HBoxContainer` | `UI/Settings/SSIL` |
| `ssil_disabled` | `Button` | `UI/Settings/SSIL/Disabled` |
| `ssil_medium` | `Button` | `UI/Settings/SSIL/Medium` |
| `ssil_high` | `Button` | `UI/Settings/SSIL/High` |
| `bloom_menu` | `HBoxContainer` | `UI/Settings/Bloom` |
| `bloom_disabled` | `Button` | `UI/Settings/Bloom/Disabled` |
| `bloom_enabled` | `Button` | `UI/Settings/Bloom/Enabled` |
| `volumetric_fog_menu` | `HBoxContainer` | `UI/Settings/VolumetricFog` |
| `volumetric_fog_disabled` | `Button` | `UI/Settings/VolumetricFog/Disabled` |
| `volumetric_fog_enabled` | `Button` | `UI/Settings/VolumetricFog/Enabled` |
| `loading` | `HBoxContainer` | `UI/Loading` |
| `loading_progress` | `ProgressBar` | `UI/Loading/Progress` |
| `loading_done_timer` | `Timer` | `UI/Loading/DoneTimer` |
Caminhos derivados mecanicamente do original (`$X` e `pai.get_node(^"Y")` → `X/Y`). Os campos
`ui`, `settings_button`, `quit_button`, `settings_actions`, `settings_action_apply` e os 15
`*_menu` não são lidos fora de `ready`/`_make_button_group` — mantidos por fidelidade.

## Chamadas dinâmicas (exceção `Settings`)

`get_node("/root/Settings")` → `.get("config_file")` (leituras/gravações via `ConfigFile`
tipado), `.call("apply_graphics_settings", [window, environment, self])`, `.call("save_settings")`.
Nenhuma outra chamada dinâmica além de `call_deferred("_on_host_pressed")` no próprio node.

## Verificação antes do commit

```bash
cd oxide-godot
grep -c '^\[connection' menu/menu.tscn                                            # 10
grep -o 'method="[^"]*"' menu/menu.tscn | sort -u                                 # 9 nomes — todos #[func] em menu.rs
grep -n 'replace_main_scene' main/main.gd                                         # l.20, 32, 33 (até o port 4)
grep -c '#\[init(node = ' ../oxide_godot_core/oxide_godot_lib/src/menu.rs         # 85
grep -c 'type="Menu"' menu/menu.tscn                                              # 1
grep -c 'ExtResource("1")' menu/menu.tscn                                         # 0
```
