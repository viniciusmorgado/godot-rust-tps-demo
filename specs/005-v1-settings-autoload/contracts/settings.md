# Contrato: `Settings` (port 5 — autoload, último script da v1)

- Classe registrada: `Settings`. Base: `Node`. Sem colisão (research D2).
- Node: `/root/Settings`, criado pelo autoload `project.godot:25` → `res://menu/settings.tscn`
  (raiz `name="Settings" type="Settings"`). O **nome** do node vem da chave do autoload.
- Módulo: `oxide_godot_core/oxide_godot_lib/src/settings.rs` (`mod settings;`).

## Propriedades expostas (`#[var]`, uso `NONE`)

| Original | Rust | Consumidores (dinâmicos, intocados) |
|---|---|---|
| `var config_file: ConfigFile` | `#[var] config_file: Gd<ConfigFile>` | `get("config_file")` em `bullet.rs:73`, `flying_forklift.rs:20`, `level.rs:49,117,145,174`, `menu.rs:309,480`, `main_scene.rs:30` (8 sítios) — todos `.to::<Gd<ConfigFile>>()` |
| `var metalfx_supported: bool` | `#[var] metalfx_supported: bool` | nenhum (o menu tem o próprio) |

`DEFAULTS` **não** é exposto (campo privado `defaults`; nenhum consumidor).

## Métodos expostos (`#[func]` — alvos de `call` por nome)

| Original | Rust | Quem chama |
|---|---|---|
| `load_settings()` | `#[func] fn load_settings(&mut self)` | `ready` (interno); exposto por fidelidade (era método público do autoload) |
| `save_settings()` | `#[func] fn save_settings(&mut self)` | `menu.rs:611` `settings.call("save_settings", &[])` |
| `apply_graphics_settings(window: Window, environment: Environment, scene_root: Node)` | `#[func] fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)` | `level.rs:44-47` e `menu.rs:208-211, 606-610`: `call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()])` — 3 argumentos |

Nenhum outro `#[func]`. `_input` é virtual (`fn input`), não exposto.

## Constantes (`#[constant]`, i64) — superfície dos enums `GIType`/`GIQuality`

| Original | Rust | Valor |
|---|---|---|
| `GIType.SDFGI` | `GI_TYPE_SDFGI` | 0 |
| `GIType.VOXEL_GI` | `GI_TYPE_VOXEL_GI` | 1 |
| `GIType.LIGHTMAP_GI` | `GI_TYPE_LIGHTMAP_GI` | 2 |
| `GIQuality.DISABLED` | `GI_QUALITY_DISABLED` | 0 |
| `GIQuality.LOW` | `GI_QUALITY_LOW` | 1 |
| `GIQuality.HIGH` | `GI_QUALITY_HIGH` | 2 |

Nenhum consumidor os lê por nome (`level.rs` usa constantes locais — backlog 1).

## Input

Ação `toggle_fullscreen` (`project.godot:172-177`, F11 + Alt+Enter) tratada em `fn input`;
`set_input_as_handled()` no viewport.

## Arquivo

`user://settings.ini` — seções/chaves/tipos/padrões em [data-model.md](../data-model.md).
Compatível nos dois sentidos com o `settings.gd` original (mesmos inteiros/bools/floats, mesma
ordem).

## Verificação antes do commit

```bash
cd oxide-godot
grep -n '^Settings=' project.godot                                          # 25:Settings="*res://menu/settings.tscn"
cat menu/settings.tscn                                                      # 3 linhas; type="Settings"
find . -name '*.gd' -not -path '*/addons/*'; find . -name '*.gd.uid' -not -path '*/addons/*'   # ambos vazios
grep -rn 'b04fekxdgdq0k\|menu/settings.gd' . | grep -v '/.godot/'           # vazio
R=../oxide_godot_core/oxide_godot_lib/src/settings.rs
grep -n '#\[func\]' -A1 $R | grep 'fn '                                     # load_settings, save_settings, apply_graphics_settings (3)
grep -n '#\[var\]' -A1 $R | grep -oE '(metalfx_supported|config_file)'      # os 2
grep -c '#\[constant\]' $R                                                  # 6
grep -n 'upstream bug fix' $R                                               # 1 linha, imediatamente acima do `} else if` do SSAO
grep -n 'else if' $R                                                        # 2 (SSAO e SSIL)
git diff --stat HEAD -- ../oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs   # vazio
```
