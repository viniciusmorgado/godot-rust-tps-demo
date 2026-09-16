# Data Model: Marco E — autoload Settings (v1 raw port)

Uma classe, um arquivo de configuração. Detalhes de API em [research.md](research.md); nomes
consumidos em [contracts/settings.md](contracts/settings.md).

## Classe `Settings` (base `Node`, autoload em `/root/Settings`)

| Campo | Tipo Rust | Exposição | Origem | Valor inicial |
|---|---|---|---|---|
| `metalfx_supported` | `bool` | `#[var]` (uso `NONE`) | `settings.gd:18` | `RenderingServer` driver atual == `"metal"` (false nesta plataforma) |
| `defaults` | `VarDictionary` (aninhado: seção → {chave → valor}) | privado | `settings.gd:20-40` (`DEFAULTS`) | tabela abaixo |
| `config_file` | `Gd<ConfigFile>` | `#[var]` — **consumido por 8 `get("config_file")`** | `settings.gd:42` | `ConfigFile::new_gd()` |

Constantes `#[constant]` (i64): `GI_TYPE_SDFGI` 0, `GI_TYPE_VOXEL_GI` 1, `GI_TYPE_LIGHTMAP_GI` 2,
`GI_QUALITY_DISABLED` 0, `GI_QUALITY_LOW` 1, `GI_QUALITY_HIGH` 2 (`settings.gd:3-13`).
`CONFIG_FILE_PATH = "user://settings.ini"` (const do módulo).

Ordem de construção (`init`): `metalfx_supported` → `defaults` → `config_file` (ordem dos `var`
do original).

## Arquivo `user://settings.ini` (`ConfigFile`) — seções, chaves, tipos e padrões

Ordem = ordem de inserção dos padrões (o `.ini` gravado segue esta ordem — SC-003).

| Seção | Chave | Tipo | Padrão (valor) | Enum do engine / origem | Quem grava/lê |
|---|---|---|---|---|---|
| `video` | `display_mode` | int | 4 | `Window.Mode.EXCLUSIVE_FULLSCREEN` | menu grava; `Main.ready`, `apply_graphics_settings` leem |
| `video` | `vsync` | int | 1 | `DisplayServer.VSyncMode.ENABLED` | menu; apply |
| `video` | `max_fps` | int | 0 | ilimitado | menu; apply |
| `video` | `resolution_scale` | float | 1.0 | — | menu; apply |
| `video` | `scale_filter` | int | 2 (FSR2) — 4 (`METALFX_TEMPORAL`) se `metalfx_supported` | `Viewport.Scaling3DMode` (5 = Nearest do 4.7, ausente da API 4.6 — ordinal passa direto) | menu; apply |
| `rendering` | `taa` | bool | false | — | menu; apply |
| `rendering` | `msaa` | int | 0 | `Viewport.MSAA.DISABLED` | menu; apply |
| `rendering` | `screen_space_aa` | int | 0 | `Viewport.ScreenSpaceAA.DISABLED` | menu; apply |
| `rendering` | `shadow_mapping` | bool | true | — | menu; apply (`propagate_call`), `Bullet.explode`, `FlyingForklift.ready` |
| `rendering` | `gi_type` | int | 1 | `GI_TYPE_VOXEL_GI` | menu; `Level.ready` |
| `rendering` | `gi_quality` | int | 1 | `GI_QUALITY_LOW` | menu; `Level.setup_*` |
| `rendering` | `ssao_quality` | int | 2 | `RenderingServer.ENV_SSAO_QUALITY_MEDIUM`; −1 = desligado | menu; apply |
| `rendering` | `ssil_quality` | int | −1 | `RenderingServer.ENV_SSIL_QUALITY_*`; −1 = desligado | menu; apply |
| `rendering` | `bloom` | bool | true | → `Environment.glow_enabled` | menu; apply |
| `rendering` | `volumetric_fog` | bool | true | → `Environment.volumetric_fog_enabled` | menu; apply |

Valores e tipos conferidos programaticamente contra `settings.gd` (research §E.3b: 15/15).

## Estados e transições

```
construção (init) ──► entra na árvore (ready) ──► load_settings
                                                    ├─ config_file.load(CONFIG_FILE_PATH)   [Error ignorado; arquivo pode não existir]
                                                    └─ para cada (seção, chave) dos padrões ausente: set_value em memória
                                                       [NÃO grava o arquivo]

menu Apply ──► config_file.set_value(…) ×N (no consumidor) ──► call("apply_graphics_settings", window, env, menu)
           └──────────────────────────────────────────────► call("save_settings") ──► config_file.save(CONFIG_FILE_PATH)  [arquivo criado/atualizado]

level/menu ready ──► call("apply_graphics_settings", window, env, raiz) ──► aplica o estado corrente do config_file
                                                                           (janela própria: modo; window: escala/filtro/TAA/MSAA/AA;
                                                                            DisplayServer: vsync; Engine: max_fps;
                                                                            scene_root: propagate_call("set", ["shadow_enabled", false]) se sombras off;
                                                                            environment: SSAO (corrigido), SSIL, glow, fog)

F11 / Alt+Enter ──► _input ──► janela própria: EXCLUSIVE_FULLSCREEN ⇄ WINDOWED; set_input_as_handled
```

## Tabela de decisão do SSAO (`apply_graphics_settings`)

| `ssao_quality` | Original (`if`/`if`/`else`) | Port (`if`/`else if`/`else`) — FR-020–FR-024 |
|---|---|---|
| −1 | `ssao_enabled` = false **e depois** true + MEDIUM/half_size (bug) | `ssao_enabled` = **false** |
| 2 (MEDIUM) | true + `HIGH`, half_size=false, 0.5, 2, 50, 300 | idem (quirk preservado — backlog 25) |
| outro (3 = HIGH, …) | true + `MEDIUM`, half_size=true, 0.5, 2, 50, 300 | idem |

SSIL (`if`/`elif`/`else` no original — fiel): −1 → false; 2 → true + MEDIUM/false; outro → true + HIGH/true.

## Vínculo do autoload

| Artefato | Conteúdo |
|---|---|
| `oxide-godot/menu/settings.tscn` (nova) | `[gd_scene format=3]` / `` / `[node name="Settings" type="Settings"]` |
| `oxide-godot/project.godot:25` | `Settings="*res://menu/settings.tscn"` |
| `oxide-godot/menu/settings.gd`, `settings.gd.uid` (`uid://b04fekxdgdq0k`) | apagados |
