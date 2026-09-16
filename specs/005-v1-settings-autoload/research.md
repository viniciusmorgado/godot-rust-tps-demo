# Research: Marco E — autoload Settings (v1 raw port)

**Fase**: v1 (constituição v1.3.1). Todas as assinaturas abaixo foram **confirmadas por
compilação** de um rascunho completo (`src/zz_research.rs`, classe `ZzSettings`, 0 erros, 0
warnings) e **exercitadas em headless** com o autoload apontando para a classe de rascunho por
uma cena mínima (§E). Rascunho, cena e a linha do `project.godot` foram revertidos (`git status`
limpo; `cargo build` de volta a 0 warnings no estado de `6f4d9ba`). Referências de linha são das
bindings em `oxide_godot_core/target/debug/build/godot-core-4eba5d49e15a0d7e/out/classes/`.

## D1 — Vínculo do autoload: cena mínima + `project.godot`

- **Decisão**: criar `oxide-godot/menu/settings.tscn` com 3 linhas
  (`[gd_scene format=3]`, vazia, `[node name="Settings" type="Settings"]`) e trocar
  `project.godot:25` de `Settings="*res://menu/settings.gd"` para
  `Settings="*res://menu/settings.tscn"`. Apagar `settings.gd` + `settings.gd.uid`.
- **Racional**: `[autoload]` aceita script ou cena; classe nativa não pode ser autoload direta.
  A cena de 1 node com `type` = classe é a "troca de `type`" do Princípio II para um node que não
  existia em cena. O nome `/root/Settings` vem da chave do autoload (confirmado §E.2:
  `name=Settings path=/root/Settings class=ZzSettings`). O `*` (singleton) é preservado.
- **Sem `uid=` no cabeçalho**: o import headless não gerou `settings.tscn.uid` nem emitiu aviso
  (§E.2) — `.tscn` guardam o uid no próprio cabeçalho e ele é opcional. Se o usuário salvar a
  cena no editor, o Godot pode acrescentar `uid="uid://…"` à l.1 — mudança cosmética futura, fora
  do commit.
- **Alternativas rejeitadas**: `.gd` ponte (`extends Settings`) — proibido, deixaria 1 `.gd`;
  `Engine::register_singleton` na `ExtensionLibrary` — não é node, quebra `get_node("/root/Settings")`
  dos 5 consumidores e o `_input`.

## D2 — Estrutura e nome

- `src/settings.rs`, `mod settings;` em `lib.rs` após `mod main_scene;`. `struct Settings`,
  `#[class(base=Node)]` (**sem** `init` automático — D4). Nenhum conflito de nome no crate.
- Nome conferido: `ls out/classes/ | grep -i settings` → `editor_settings`, `label_settings`,
  `mesh_convex_decomposition_settings`, `open_xr_android_thread_settings_extension`,
  `project_settings` (nenhum `Settings` puro); grep do `CLAUDE.md` nos `.gd`: só o próprio
  `settings.gd`, que é apagado. `ClassDB` registrou `ZzSettings` sem conflito (§E.3).

## D3 — Campos e visibilidade

| Original (`settings.gd`) | Rust | Nota |
|---|---|---|
| `const CONFIG_FILE_PATH = "user://settings.ini"` (l.15) | `const CONFIG_FILE_PATH: &str = "user://settings.ini";` (nível do módulo) | — |
| `var metalfx_supported: bool = RenderingServer.get_current_rendering_driver_name() == "metal"` (l.18) | `#[var] metalfx_supported: bool`, calculado no `init` com `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")` (`rendering_server.rs:5391`) | **`#[var]` imposto pelo compilador**: sem ele, `field metalfx_supported is never read` (o campo só alimenta `scale_filter` no `init`). No original é `var` de topo = propriedade de script; `#[var]` reproduz isso (uso `NONE`). Comparação com `GString::from` (D3 do plan 004) |
| `var DEFAULTS := { video = {…}, rendering = {…} }` (l.20-40) | `defaults: VarDictionary` (campo privado, sem `#[var]`), construído no `init` com `vdict!` aninhado — D5 | Nome `defaults` (campo Rust; o original é `DEFAULTS`). Não exposto: nenhum consumidor lê; se fosse `#[var]` o nome exposto seria `defaults`, diferente do original — melhor não expor |
| `var config_file := ConfigFile.new()` (l.42) | `#[var] config_file: Gd<ConfigFile>` = `ConfigFile::new_gd()` no `init` | `#[var]` (não `#[export]`): os 8 `get("config_file")` dos consumidores resolvem (§E.3 `same_obj=true`); nada aparece no inspector |
| `enum GIType {…}`, `enum GIQuality {…}` (l.3-13) | 6 `#[constant] const …: i64` no bloco `#[godot_api] impl Settings` — D6 | — |

## D4 — `init` manual

- **Decisão**: `#[godot_api] impl INode for Settings { fn init(base: Base<Node>) -> Self { … } }`
  computando, **nesta ordem** (a dos `var` do original): `metalfx_supported`, `defaults`
  (usa `metalfx_supported`), `config_file`.
- **Racional**: `#[init(val = …)]` não pode referenciar outro campo; o padrão de `scale_filter`
  depende de `metalfx_supported`. Alternativa (recalcular o driver dentro do `vdict!`) duplicaria a
  leitura do `RenderingServer` que o original faz uma vez.

## D5 — `DEFAULTS` como `VarDictionary` aninhado; `load_settings`

- `vdict! { "video" => &vdict! { … }, "rendering" => &vdict! { … } }` — **imposto pelo
  compilador**: o valor aninhado deve ser passado **por referência** (`&vdict!`), senão
  `E0271: <Dictionary<Variant, Variant> as ToGodot>::Pass == ByValue`. Os valores escalares vão
  por valor: `i64` (`.ord() as i64` dos enums do engine; `0_i64`; `-1_i64`; `Self::GI_TYPE_VOXEL_GI`;
  `Self::GI_QUALITY_LOW`), `f64` (`1.0_f64`), `bool`.
- Valores (conferidos programaticamente contra `settings.gd` — §E.3, 15/15 iguais em valor e
  tipo): `display_mode` = `WindowMode::EXCLUSIVE_FULLSCREEN.ord()` = 4; `vsync` =
  `VSyncMode::ENABLED.ord()` = 1; `max_fps` = 0; `resolution_scale` = 1.0; `scale_filter` =
  `if metalfx_supported { Scaling3DMode::METALFX_TEMPORAL.ord() } else { Scaling3DMode::FSR2.ord() }`
  = 2 aqui; `taa` = false; `msaa` = `Msaa::DISABLED.ord()` = 0; `screen_space_aa` =
  `ScreenSpaceAa::DISABLED.ord()` = 0; `shadow_mapping` = true; `gi_type` = 1; `gi_quality` = 1;
  `ssao_quality` = `EnvironmentSsaoQuality::MEDIUM.ord()` = 2; `ssil_quality` = −1; `bloom` = true;
  `volumetric_fog` = true.
- **Ordem de inserção preservada**: `Dictionary` do Godot é ordenado; `keys_shared()` itera na
  ordem de inserção (§E.3: seções `["video", "rendering"]`, chaves na ordem do original). O
  `.ini` gravado sai na mesma ordem (SC-003 confirmado byte a byte, §E.3).
- `load_settings` (`settings.gd:55-62`): `self.config_file.load(CONFIG_FILE_PATH)`
  (`config_file.rs:214`, `-> Error`, retorno **ignorado** — como o original; a chamada é
  `let _ = …`? **Não**: chamar sem `let _` — `Error` não é `#[must_use]`, compilou sem warning);
  `for section in self.defaults.keys_shared() { let section_defaults = self.defaults.at(&section).to::<VarDictionary>(); for key in section_defaults.keys_shared() { … if !self.config_file.has_section_key(&section_name, &key_name) { self.config_file.set_value(&section_name, &key_name, &section_defaults.at(&key)); } } }`
  (`config_file.rs:164,123`; `section.to::<GString>()`/`key.to::<GString>()` porque `has_section_key`
  pede `AsArg<GString>` e as chaves do dicionário são `Variant`). **Não salva** (§E.3: com o
  `.ini` ausente, `FileAccess.file_exists("user://settings.ini")` = false após o boot).
- `ConfigFile::load` em arquivo ausente: devolve `ERR_FILE_NOT_FOUND` (7) e **não imprime erro**
  (§E.3) — nenhuma linha `ERROR` nova em headless.
- `save_settings` (`l.65-66`): `self.config_file.save(CONFIG_FILE_PATH)` (`config_file.rs:234`),
  retorno ignorado.

## D6 — Enums do script → `#[constant]`

- **Decisão**: no bloco `#[godot_api] impl Settings`: `#[constant] const GI_TYPE_SDFGI: i64 = 0;`
  `GI_TYPE_VOXEL_GI = 1`, `GI_TYPE_LIGHTMAP_GI = 2`, `GI_QUALITY_DISABLED = 0`, `GI_QUALITY_LOW = 1`,
  `GI_QUALITY_HIGH = 2`. Suportado (`godot-macros inherent_impl.rs:627,772`); registrados no
  `ClassDB` (§E.3: `class_has_integer_constant` = true, valores 0/1/2).
- **Racional**: preserva a superfície `Settings.GIType.X` da forma que o gdext permite (enums
  nomeados de script não existem como tal). Os padrões usam `Self::GI_TYPE_VOXEL_GI` /
  `Self::GI_QUALITY_LOW` (o original usa `GIType.VOXEL_GI` / `GIQuality.LOW`). Nenhum consumidor os
  lê por nome (backlog item 1 já prevê constantes locais nos consumidores).
- Alternativa (só inteiros nos padrões) — perderia nomes que o original tem; custo zero de expor.

## D7 — `ready` e `_input`

- `fn ready(&mut self) { self.load_settings(); }` (`l.45-46`).
- `fn input(&mut self, input_event: Gd<InputEvent>)` (`l.49-52`):
  `if input_event.is_action_pressed("toggle_fullscreen")` (`input_event.rs:76`) →
  `let mut window = self.base().get_window().unwrap(); let mode = window.get_mode();`
  (`node.rs:1163` `Option`; `window.rs:381`) →
  `window.set_mode(if !((mode == WindowMode::EXCLUSIVE_FULLSCREEN) || (mode == WindowMode::FULLSCREEN)) { WindowMode::EXCLUSIVE_FULLSCREEN } else { WindowMode::WINDOWED })`
  (`window.rs:372`; `Mode` é `Copy + PartialEq`) → `self.base().get_viewport().unwrap().set_input_as_handled()`
  (`node.rs:1289`; `viewport.rs:741`). `use godot::classes::window::Mode as WindowMode` (alias do
  plan 004/`main_scene.rs`).

## D8 — `apply_graphics_settings(window, environment, scene_root)` linha a linha + correção do SSAO

`#[func] fn apply_graphics_settings(&mut self, mut window: Gd<Window>, mut environment: Gd<Environment>, mut scene_root: Gd<Node>)`
— alvo de `call("apply_graphics_settings", &[window, environment, self])` de `level.rs:44`,
`menu.rs:208,606` (3 `Variant`s → os `Gd<T>` são convertidos pelo gdext; §E.2 rodou os três).
Leituras sempre `self.config_file.get_value(sec, key)` → `Variant` → `.to::<i64>()` /
`.to::<bool>()` / `.to::<f64>()`, repetidas a cada linha como no original (`Settings.config_file`
→ própria propriedade).

| `settings.gd` | Rust | Bindings |
|---|---|---|
| l.70 `get_window().mode = …display_mode` | `self.base().get_window().unwrap().set_mode(WindowMode::from_ord(… as i32))` — janela **própria**, não o parâmetro (quirk) | `window.rs:372`; `EngineEnum::from_ord` `obj/traits.rs:201` |
| l.71 `DisplayServer.window_set_vsync_mode(…vsync)` | `DisplayServer::singleton().window_set_vsync_mode(VSyncMode::from_ord(… as i32))` | `display_server.rs:2020` |
| l.72 `Engine.max_fps = …max_fps` | `Engine::singleton().set_max_fps(… as i32)` | `engine.rs:88` |
| l.73 `window.scaling_3d_scale = …resolution_scale` | `window.set_scaling_3d_scale(….to::<f64>() as f32)` | `viewport.rs:1128` (herdado por `Window`) |
| l.74 `window.scaling_3d_mode = …scale_filter` | `window.set_scaling_3d_mode(Scaling3DMode::from_ord(… as i32))` — ver **Nearest** abaixo | `viewport.rs:1110` |
| l.76-78 `use_taa`, `msaa_3d`, `screen_space_aa` | `window.set_use_taa(bool)`, `set_msaa_3d(Msaa::from_ord)`, `set_screen_space_aa(ScreenSpaceAa::from_ord)` | `viewport.rs:218,182,200` |
| l.80-86 `if not …shadow_mapping: scene_root.propagate_call("set", ["shadow_enabled", false])` | `if !… { scene_root.propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done(); }` — API base; comentário `FIXME` do upstream copiado | `node.rs:735,2093` (`args: &AnyArray`; `varray!` produz `VarArray` = `AnyArray`) — §E.3: luz filha ficou `shadow_enabled = false` |
| l.88-95 SSAO | `if q == -1 { environment.set_ssao_enabled(false); } else if q == EnvironmentSsaoQuality::MEDIUM.ord() as i64 { set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::HIGH, false, 0.5, 2, 50.0, 300.0); } else { set_ssao_enabled(true); …(EnvironmentSsaoQuality::MEDIUM, true, 0.5, 2, 50.0, 300.0); }` — **`else if` é a correção** (original: `if`); comentário `// upstream bug fix: settings.gd:90 usava `if` em vez de `elif` — "SSAO: Disabled" (-1) era religado pelo `else` abaixo` na linha imediatamente acima do `else if` | `environment.rs:570`; `rendering_server.rs:3473` (`quality, half_size: bool, adaptive_target: f32, blur_passes: i32, fadeout_from: f32, fadeout_to: f32`) |
| l.97-104 SSIL | `if q == -1 { set_ssil_enabled(false) } else if q == EnvironmentSsilQuality::MEDIUM.ord() as i64 { set_ssil_enabled(true); environment_set_ssil_quality(MEDIUM, false, 0.5, 2, 50.0, 300.0) } else { set_ssil_enabled(true); environment_set_ssil_quality(HIGH, true, 0.5, 2, 50.0, 300.0) }` — fiel (o original já é `elif`) | `environment.rs:732`; `rendering_server.rs:3483` |
| l.106-107 `glow_enabled`, `volumetric_fog_enabled` | `environment.set_glow_enabled(bool)`, `environment.set_volumetric_fog_enabled(bool)` | `environment.rs:1038,1508` |

- **Nearest (ordinal 5)**: `Viewport.SCALING_3D_MODE_NEAREST` = 5 existe no Godot 4.7 mas não na
  API 4.6 das bindings (`menu.rs` usa `const SCALING_3D_MODE_NEAREST: i64 = 5`). Em 4.6,
  `Scaling3DMode::try_from_ord` aceita `0..=5` (5 = `MAX`), então **`from_ord(5)` não faz panic**
  e o ordinal 5 chega ao engine, que o interpreta como Nearest (§E.3: `root.scaling_3d_mode = 5`
  após aplicar com `scale_filter = 5`). Nenhum tratamento especial; **não** usar `set("scaling_3d_mode")`
  dinâmico.
- `from_ord` faz panic para ordinais fora do enum (`traits.rs:201-204`). Os valores possíveis vêm
  do menu portado (`.ord()` dos mesmos enums) e dos padrões; um `.ini` editado à mão com lixo
  quebraria também o original (atribuição de inteiro inválido à propriedade). Fidelidade; nada a
  tratar.
- **Bug do SSAO reproduzido no original** (§E.4): com `settings.gd` como autoload,
  `ssao_quality = -1` → `apply_graphics_settings` → `environment.ssao_enabled == true`; SSIL −1 →
  `false`. Com o rascunho: −1 → `false`; MEDIUM → `true`; HIGH → `true` (§E.3).

## D9 — O que NÃO muda (quirks preservados, Princípio I)

Modo de janela aplicado a `get_window()` do próprio autoload (não ao parâmetro `window`);
"SSAO média aplica qualidade alta sem `half_size`, e o resto aplica média com `half_size`"
(intenção ambígua — backlog 25); `FIXME` das sombras (com `shadow_mapping` verdadeiro nada é
religado); `metalfx_supported` avaliado uma vez na construção; leituras repetidas de
`config_file.get_value` a cada linha; `load_settings` sem `save`; retornos `Error` de `load`/`save`
ignorados; ordem de inserção dos padrões; a única mudança de comportamento é a correção FR-020–FR-024.

## D10 — Consumidores (intocados) e o encerramento da exceção

Os 10 sítios (`bullet.rs:72`, `flying_forklift.rs:19`, `level.rs:43-49,116,144,173`,
`menu.rs:208,308,479,611`, `main_scene.rs:29`) fazem `get_node_as::<Node>("/root/Settings")` +
`get("config_file")` / `call(…)`. Todos resolveram contra a classe de rascunho em `main.tscn`
(boot → `Main` lê `display_mode` → `Menu` aplica → host → `Level` aplica), `menu.tscn` e
`level.tscn` (§E.2). Ficam como estão (spec FR-014; backlog 1). `CLAUDE.md` não muda (plan,
FR-019).

## §E — Verificação empírica (2026-09-16, headless, sem editor aberto — `pgrep -a godot` vazio)

### E.0 Compilação do rascunho

`ZzSettings` **completo** (init manual, `#[var]` ×2, `vdict!` aninhado, 6 `#[constant]`, `ready`,
`input`, `load_settings`, `save_settings`, `apply_graphics_settings` com a correção): 2 iterações
— (1) `E0271` no `vdict!` aninhado → `&vdict!`; (2) warning `field metalfx_supported is never read`
→ `#[var]`. Final: **0 erros, 0 warnings**. Cópia do rascunho no scratchpad
(`settings_draft.rs`) para o `/speckit-tasks` reproduzir.

### E.1 Baseline (estado `6f4d9ba` = código de `fa27e22`; `0e27c68`/`6f4d9ba` só tocam specs e `.gitignore`)

- `cargo build`: 0 warnings. Import: `Initialize godot-rust` ×1, 0 `ERROR` novo.
- `main.tscn` exit 124 / 0 regressão / 2 WARNINGs (HDR, Physics interpolation); `menu.tscn` 124/0/1;
  `level.tscn` 124/0/2 (medidos no T055 do Marco D em `fa27e22`; nada mudou desde).
- `user://settings.ini` real: `~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini`,
  231 bytes, md5 `99dac170ad99078f46d2428e650516e7`, gravado pelo `settings.gd` original
  (conteúdo atual: `display_mode=3`, `vsync=1`, `max_fps=0`, `resolution_scale=1.0`,
  `scale_filter=2`, `taa=false`, `msaa=3`, `screen_space_aa=0`, `shadow_mapping=true`,
  `gi_type=1`, `gi_quality=2`, `ssao_quality=3`, `ssil_quality=3`, `bloom=true`,
  `volumetric_fog=true`). Cópia em `scratchpad/settings.ini.baseline` para o diff do SC-003.
- `docs/upstream-bugs.md`: 2 entradas; `docs/v2-backlog.md`: 24 itens.

### E.2 Autoload em cena com a classe de rascunho (fluxo completo)

`menu/zz_settings.tscn` (3 linhas, `type="ZzSettings"`) + `project.godot:25` →
`Settings="*res://menu/zz_settings.tscn"`; import: `Initialize godot-rust`, **nenhum `.uid`
gerado para a `.tscn`, nenhum aviso**. `main.tscn` 124/0/2, `menu.tscn` 124/0/1,
`level.tscn` 124/0/2 — iguais à baseline; o `.ini` ficou com o mesmo md5 após os 3 boots
(`load_settings` não grava). Revertido depois: `git checkout -- project.godot`, `rm zz_settings.tscn`.

### E.3 Probe A (`-s`, `.ini` presente) — contrato, constantes, SC-003, FR-024, Nearest, sombras

`class=ZzSettings name=Settings path=/root/Settings`; `config_file class=ConfigFile same_obj=true`;
`metalfx_supported=false`; `has_method` `load_settings`/`save_settings`/`apply_graphics_settings`
= true; 6 constantes com `class_has_integer_constant` = true e valores 0/1/2/0/1/2; seções
`["video", "rendering"]` e chaves na ordem do original; `ConfigFile.load` de arquivo inexistente →
`7` (`ERR_FILE_NOT_FOUND`) sem linha `ERROR`; **`save_settings` regravou o `.ini` byte a byte
igual à baseline** (`diff` vazio — SC-003); `ssao_quality` −1 → `ssao_enabled=false`
(**FR-024**), MEDIUM → true, HIGH → true; `scale_filter = 5` → `root.scaling_3d_mode = 5` sem
panic; `shadow_mapping=false` → `DirectionalLight3D` filha do `scene_root` com
`shadow_enabled=false` (`propagate_call`). Os `WARNING … ObjectDB instances were leaked` /
`RID allocations … leaked` no fim são do próprio probe (Environment/Node criados e não liberados),
não do port.

### E.3b Probe B (`-s`, `.ini` **ausente** — movido e restaurado; md5 final igual)

Comparação programática dos 15 padrões do rascunho (via `config_file` após `load_settings`)
com `load("res://menu/settings.gd").new().DEFAULTS`: **15/15 iguais em valor e `typeof`**
(`int`/`float`/`bool`); ordem das seções e das chaves idêntica; `FileAccess.file_exists("user://settings.ini")`
= **false** após o boot (o original também não grava em `load_settings`).

### E.4 Probe C (`-s`, autoload = `settings.gd` **original**) — reprodução do bug

`class=Node script=res://menu/settings.gd`; `ssao_quality = -1` → `apply_graphics_settings` →
**`ssao_enabled = true`** (bug); `ssil_quality = -1` → `ssil_enabled = false` (correto). Base
para a entrada #3 de `docs/upstream-bugs.md`.

## Backlog v2 candidato (registrar em `docs/v2-backlog.md` no commit do port)

| # | Origem | Melhoria | Motivação |
|---|---|---|---|
| 25 | `menu/settings.gd` (port 5) | Rever o mapeamento SSAO: "Medium" aplica `ENV_SSAO_QUALITY_HIGH` sem `half_size` e "High"/outros aplicam `MEDIUM` com `half_size` (o SSIL faz o inverso, coerente com os nomes) | Intenção ambígua no upstream (troca de nomes ou de constantes); a v1 preserva por fidelidade — só o `if`→`elif` do "Disabled" foi corrigido (upstream-bugs #3) |

## Entrada #3 de `docs/upstream-bugs.md` (texto a gravar no commit; coluna Commit = assunto do commit)

| # | Defeito | Script / cena | Correção aplicada | Commit |
|---|---|---|---|---|
| 3 | "SSAO: Disabled" não desligava o SSAO — `settings.gd:88-95` fazia `if ssao_quality == -1: ssao_enabled = false` seguido de `if ssao_quality == MEDIUM: … else: ssao_enabled = true …` (sem `elif`), então o `else` religava o SSAO em qualidade média com `half_size` para qualquer valor ≠ MEDIUM, inclusive −1. Reproduzido em headless no original (`ssao_enabled == true` após aplicar com −1); o bloco SSIL idêntico (`settings.gd:97-104`) usa `elif` e funciona | `menu/settings.gd:88-95`; declarado em `specs/005-v1-settings-autoload` FR-020–FR-024 | `oxide_godot_core/oxide_godot_lib/src/settings.rs` `apply_graphics_settings`: o segundo teste vira `else if`; comentário `// upstream bug fix` na linha acima. Ramos "média → alta sem `half_size`" e "senão → média com `half_size`", parâmetros e SSIL intocados (quirk de nomes fica no backlog 25). Headless: −1 → `ssao_enabled = false`; MEDIUM/HIGH → `true` | `Port settings.gd → Settings (Node) autoload via menu/settings.tscn; project.godot: autoload script→cena (upstream bug fix: SSAO Disabled)` |
