# Tasks: Marco E — autoload Settings (v1 raw port, último script)

**Input**: Design documents from `/specs/005-v1-settings-autoload/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D10, §E), data-model.md, contracts/settings.md, quickstart.md (todos aprovados, commit `619575d`)

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.1). Nenhuma task pode introduzir
abstração, refatoração, otimização, teste unitário ou infraestrutura. Se algo assim parecer
necessário, vira entrada em `docs/v2-backlog.md`, não task. **Uma correção conservadora de bug**
do upstream está declarada na spec (FR-020–FR-024: SSAO "Disabled" não desligava — `if` em vez
de `elif` em `settings.gd:90`) e entra no **único** commit do marco com os 4 requisitos da cláusula
(spec ✓, comentário `// upstream bug fix` no ponto exato, mensagem do commit,
`docs/upstream-bugs.md` #3). Qualquer **outro** defeito objetivo → PARAR e reportar (exige spec
antes de qualquer commit). Nenhuma mudança de crate (`Cargo.toml` intocado); `CLAUDE.md` intocado
(plan, FR-019).

**Tests**: não há testes automatizados nesta fase. A validação é o ciclo do quickstart (build →
import headless → cenas headless → probes descartáveis fora do repo → verificações mecânicas →
contrato → validação visual do usuário).

**Organization**: uma única user story de implementação — o autoload `Settings` é um script só e
vai num único commit `Port …`. As US2/US3 da spec (aplicação das configurações; F11) são
comportamentos do mesmo script, verificados dentro desta phase e no checkpoint do usuário; não
geram commits nem tasks separadas de código. Depois, Polish = verificação final da **v1 inteira**.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência de task incompleta) —
  praticamente inexistente aqui: build depende do módulo, vínculo depende do build, validação
  depende do vínculo.
- **[Story]**: US1 (spec.md — única story de implementação).
- Caminhos relativos à raiz do repositório (`oxide-godot/` = projeto Godot;
  `oxide_godot_core/oxide_godot_lib/src/` = crate Rust).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` de cada módulo — só ganha `mod settings;`
oxide_godot_core/oxide_godot_lib/src/settings.rs   NOVO — struct Settings, base=Node (research D2–D8)
oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs   consumidores dinâmicos — NUNCA mudam (FR-014)
oxide_godot_core/Cargo.toml                        NUNCA muda
oxide-godot/menu/settings.tscn                     NOVA — 3 linhas (vínculo do autoload, plan §"Edição do vínculo")
oxide-godot/project.godot                          l.25 apenas: Settings="*res://menu/settings.gd" → Settings="*res://menu/settings.tscn"
oxide-godot/menu/settings.gd (+ .uid uid://b04fekxdgdq0k)   APAGAR (git rm) — últimos .gd/.uid do projeto
docs/upstream-bugs.md                              entrada #3 (texto em research §"Entrada #3")
docs/v2-backlog.md                                 item 25 (texto em research §"Backlog v2 candidato")
CLAUDE.md                                          NUNCA muda (FR-019 decidido no plan)
../oxide_godot_origins/                            referência intocada do GDScript original
~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini   = user://settings.ini real (SC-003)
```

Decisões fechadas (instrução, não opção; detalhes em research.md):

- **Classe**: `struct Settings`, `#[class(base=Node)]` **sem** `init` automático; `fn init(base: Base<Node>) -> Self`
  manual computando, nesta ordem, `metalfx_supported` → `defaults` → `config_file` (D4).
- **Campos**: `#[var] metalfx_supported: bool` e `#[var] config_file: Gd<ConfigFile>` (**nunca**
  `#[export]`; `#[var]` em `metalfx_supported` é imposto pelo compilador — warning `never read` — e
  reproduz o `var` de topo do original); `defaults: VarDictionary` **privado**, `vdict!` aninhado
  com `&vdict!` nos valores aninhados (E0271 sem o `&`) (D3, D5); `const CONFIG_FILE_PATH: &str`
  no nível do módulo.
- **Constantes**: 6 `#[constant] const …: i64` no **único** bloco `#[godot_api] impl Settings`
  (`GI_TYPE_SDFGI/VOXEL_GI/LIGHTMAP_GI` = 0/1/2, `GI_QUALITY_DISABLED/LOW/HIGH` = 0/1/2); os padrões
  usam `Self::GI_TYPE_VOXEL_GI` e `Self::GI_QUALITY_LOW` (D6).
- **`load_settings`**: `self.config_file.load(CONFIG_FILE_PATH);` (retorno ignorado, sem `let _`);
  loop `keys_shared()` pelos padrões na ordem de inserção com `has_section_key` → `set_value`;
  **não salva**. **`save_settings`**: `self.config_file.save(CONFIG_FILE_PATH);` (D5).
- **`input`**: `toggle_fullscreen` → janela **própria** para `EXCLUSIVE_FULLSCREEN` se o modo não é
  `FULLSCREEN` nem `EXCLUSIVE_FULLSCREEN`, senão `WINDOWED`; `get_viewport().unwrap().set_input_as_handled()` (D7).
- **`apply_graphics_settings(window, environment, scene_root)`**: tabela D8 linha a linha; modo da
  janela na janela **própria** (`self.base().get_window()` — quirk); `from_ord(x as i32)` em todos
  os enums, inclusive `Scaling3DMode::from_ord(5)` (Nearest do 4.7 — **sem** tratamento especial,
  **sem** `set` dinâmico); `propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done()`
  com o comentário `FIXME` do upstream copiado; SSAO `if / else if / else` com o comentário
  `// upstream bug fix: settings.gd usava if em vez de elif — "SSAO: Disabled" (-1) era religado pelo else`
  na linha imediatamente acima do `} else if`; quirk "média aplica alta" **preservado**; SSIL
  fiel; `set_glow_enabled`/`set_volumetric_fog_enabled`.
- **Vínculo** (D1): cena nova de 3 linhas + 1 linha do `project.godot`; `git rm` do `.gd` + `.uid`.
  `settings.tscn` **nunca** é rodada isolada em headless.
- Um único commit `Port …` na `main` (mensagem em T017); fix após checkpoint = `Fix port …`.
  **Antes de qualquer commit que não seja o do port, conferir que não há deleções de `.gd` em
  staging** (lição do Marco C).

---

## Phase 1: Setup

**Purpose**: registrar a baseline (build, import, 3 cenas, `settings.ini` real) contra a qual o
port é comparado. Nada de infraestrutura (Princípio I).

- [ ] T001 Confirmar pré-condições: nenhum editor Godot aberto (`pgrep -a godot` vazio — se houver, avisar o usuário e aguardar, nunca matar); `git status --short` vazio na `main`; anotar `git rev-parse --short HEAD` (esperado `619575d` ou posterior sem commits "Port …" — este hash é o commit-base do quickstart §8/T020); `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = só `oxide-godot/menu/settings.gd`; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'` = só `oxide-godot/menu/settings.gd.uid`; `grep -n '^Settings=' oxide-godot/project.godot` = `25:Settings="*res://menu/settings.gd"`; `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = { version = "0.5.5", features = ["experimental-threads"] }` (não muda neste marco)
- [ ] T002 Registrar baseline do build: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → esperado `0`; anotar em `specs/005-v1-settings-autoload/quickstart.md` §1 se diferir
- [ ] T003 Registrar baseline do import: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirmar `grep -c 'Initialize godot-rust' /tmp/import.log` = 1 e `grep -E 'ERROR|SCRIPT ERROR' /tmp/import.log | grep -vE 'Cannon_Charge already exists|doorsimple_d.png|surfaces.is_empty'` vazio
- [ ] T004 [P] Conferir nome e bindings: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | awk '{print $NF}' | sort -u | grep -wx Settings` vazio (o único `.gd` é o próprio `settings.gd`, que não declara `Settings`); `B=$(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1); ls $B/classes/ | grep -ix 'settings.rs'` vazio (só `project_settings.rs`, `editor_settings.rs`, `label_settings.rs`, …); anotar `$B` (esperado `…/godot-core-4eba5d49e15a0d7e/out`). Contagens: `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2; `grep -c '^| [0-9]' docs/v2-backlog.md` = 24
- [ ] T005 Copiar o `settings.ini` real para o scratch como baseline do SC-003: `INI="$HOME/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini"`; se **não** existir, rodar `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . main/main.tscn` uma vez — **atenção**: o `settings.gd` original **não** cria o arquivo no boot (só em Apply); se continuar ausente, anotar "sem baseline de `.ini`" e o SC-003 será verificado só pela comparação dos padrões (T013) e pelo checkpoint do usuário; se existir: `cp "$INI" <scratchpad>/settings.ini.baseline; md5sum "$INI"` (em 2026-09-16: `99dac170ad99078f46d2428e650516e7`, 231 bytes); anotar o caminho do scratchpad
- [ ] T006 Medir a baseline das 3 cenas (quickstart §1): `cd oxide-godot && for s in main/main.tscn menu/menu.tscn level/level.tscn; do timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/base_$(basename $s .tscn).log 2>&1; echo "$s $?"; done`; esperado exit 124 em todas, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/base_*.log` vazio (regra do erro intermitente de `main.tscn`: as 5 linhas do renderizador dummy não contam se sumirem numa 2ª execução), WARNINGs esperados só `HDR output…` e `[Physics interpolation]…` (main 2, menu 1, level 2); `md5sum "$INI"` igual ao de T005 (o boot não grava o `.ini`)

**Checkpoint**: baseline conhecida — 0 warnings, extensão carrega, 0 `ERROR` no import e nas 3 cenas, `.ini` copiado.

---

## Phase 2: Foundational

**Não existe neste marco.** Um único script; a única edição compartilhada é `mod settings;` em
`lib.rs`, dentro da story. Nenhum módulo comum, helper, trait ou constante compartilhada pode ser
criado (Princípio I). Os 5 consumidores continuam com o acesso dinâmico (backlog 1 — fora do escopo).

---

## Phase 3: User Story 1 — Autoload Settings em Rust (Priority: P1) 🎯 MVP = marco inteiro

**Goal**: `menu/settings.gd` (107 l., autoload `Settings`, `extends Node`) → `Settings: Node` em
`src/settings.rs`; vínculo por cena mínima `menu/settings.tscn` + `project.godot:25`; `.gd` + `.uid`
apagados → **zero `.gd` no projeto**. Inclui os comportamentos das US2 (aplicação das
configurações, com a correção do SSAO) e US3 (F11) da spec.

**Independent Test**: `main.tscn` headless (boot completo com o autoload em Rust: `Settings.ready`
→ `Main` lê `display_mode` → `Menu` aplica → host automático → `Level` aplica) sem erro novo;
probes: `.ini` regravado byte a byte igual, 15 padrões iguais ao original, SSAO −1 → desligado; no
jogo, Settings do menu refletem/persistem, F11 alterna, SSAO Disabled desliga de fato.

- [ ] T007 [US1] Criar `oxide_godot_core/oxide_godot_lib/src/settings.rs` — parte 1 (usos, constante, struct, `init` manual, constantes): `use godot::classes::display_server::VSyncMode; use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality}; use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa}; use godot::classes::window::Mode as WindowMode; use godot::classes::{ConfigFile, DisplayServer, Engine, Environment, INode, InputEvent, Node, RenderingServer, Window}; use godot::prelude::*;`; `const CONFIG_FILE_PATH: &str = "user://settings.ini";`; `#[derive(GodotClass)] #[class(base=Node)] pub struct Settings { base: Base<Node>, /* MetalFX is only supported when using the Metal rendering driver. */ #[var] metalfx_supported: bool, defaults: VarDictionary, #[var] config_file: Gd<ConfigFile>, }`; `#[godot_api] impl INode for Settings { fn init(base: Base<Node>) -> Self { let metalfx_supported = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"); let defaults = vdict! { "video" => &vdict! { "display_mode" => WindowMode::EXCLUSIVE_FULLSCREEN.ord() as i64, "vsync" => VSyncMode::ENABLED.ord() as i64, "max_fps" => 0_i64, "resolution_scale" => 1.0_f64, "scale_filter" => if metalfx_supported { Scaling3DMode::METALFX_TEMPORAL.ord() as i64 } else { Scaling3DMode::FSR2.ord() as i64 }, }, "rendering" => &vdict! { "taa" => false, "msaa" => Msaa::DISABLED.ord() as i64, "screen_space_aa" => ScreenSpaceAa::DISABLED.ord() as i64, "shadow_mapping" => true, "gi_type" => Self::GI_TYPE_VOXEL_GI, "gi_quality" => Self::GI_QUALITY_LOW, "ssao_quality" => EnvironmentSsaoQuality::MEDIUM.ord() as i64, "ssil_quality" => -1_i64, /* Disabled */ "bloom" => true, "volumetric_fog" => true, }, }; Self { base, metalfx_supported, defaults, config_file: ConfigFile::new_gd() } } /* ready e input em T008 */ }`; e o bloco `#[godot_api] impl Settings { #[constant] const GI_TYPE_SDFGI: i64 = 0; #[constant] const GI_TYPE_VOXEL_GI: i64 = 1; #[constant] const GI_TYPE_LIGHTMAP_GI: i64 = 2; #[constant] const GI_QUALITY_DISABLED: i64 = 0; #[constant] const GI_QUALITY_LOW: i64 = 1; #[constant] const GI_QUALITY_HIGH: i64 = 2; /* #[func]s em T008 */ }`. Ordem das chaves **exatamente** a de `settings.gd:20-40` (é a ordem do `.ini`). Rascunho compilado idêntico (classe `ZzSettings`) em `<scratchpad>/settings_draft.rs` (research §E.0) (research D2–D6)
- [ ] T008 [US1] Completar `oxide_godot_core/oxide_godot_lib/src/settings.rs` — parte 2 (virtuais + 3 `#[func]`): no `impl INode`: `fn ready(&mut self) { self.load_settings(); }` e `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("toggle_fullscreen") { let mut window = self.base().get_window().unwrap(); let mode = window.get_mode(); window.set_mode(if !((mode == WindowMode::EXCLUSIVE_FULLSCREEN) || (mode == WindowMode::FULLSCREEN)) { WindowMode::EXCLUSIVE_FULLSCREEN } else { WindowMode::WINDOWED }); self.base().get_viewport().unwrap().set_input_as_handled(); } }`. No bloco `#[godot_api] impl Settings` (o mesmo das constantes — **único**): `#[func] fn load_settings(&mut self) { self.config_file.load(CONFIG_FILE_PATH); /* comentário do original settings.gd:57-58 */ for section in self.defaults.keys_shared() { let section_defaults = self.defaults.at(&section).to::<VarDictionary>(); for key in section_defaults.keys_shared() { let section_name = section.to::<GString>(); let key_name = key.to::<GString>(); if !self.config_file.has_section_key(&section_name, &key_name) { self.config_file.set_value(&section_name, &key_name, &section_defaults.at(&key)); } } } }`; `#[func] fn save_settings(&mut self) { self.config_file.save(CONFIG_FILE_PATH); }`; `#[func] fn apply_graphics_settings(&mut self, mut window: Gd<Window>, mut environment: Gd<Environment>, mut scene_root: Gd<Node>) { self.base().get_window().unwrap().set_mode(WindowMode::from_ord(self.config_file.get_value("video", "display_mode").to::<i64>() as i32)); DisplayServer::singleton().window_set_vsync_mode(VSyncMode::from_ord(self.config_file.get_value("video", "vsync").to::<i64>() as i32)); Engine::singleton().set_max_fps(self.config_file.get_value("video", "max_fps").to::<i64>() as i32); window.set_scaling_3d_scale(self.config_file.get_value("video", "resolution_scale").to::<f64>() as f32); window.set_scaling_3d_mode(Scaling3DMode::from_ord(self.config_file.get_value("video", "scale_filter").to::<i64>() as i32)); window.set_use_taa(self.config_file.get_value("rendering", "taa").to::<bool>()); window.set_msaa_3d(Msaa::from_ord(self.config_file.get_value("rendering", "msaa").to::<i64>() as i32)); window.set_screen_space_aa(ScreenSpaceAa::from_ord(self.config_file.get_value("rendering", "screen_space_aa").to::<i64>() as i32)); if !self.config_file.get_value("rendering", "shadow_mapping").to::<bool>() { /* copiar as 5 linhas de comentário de settings.gd:81-85 (Disable shadows … FIXME …) */ scene_root.propagate_call_ex("set").args(&varray!["shadow_enabled", false]).done(); } if self.config_file.get_value("rendering", "ssao_quality").to::<i64>() == -1 { environment.set_ssao_enabled(false); [LINHA PRÓPRIA DE COMENTÁRIO: `// upstream bug fix: settings.gd usava if em vez de elif — "SSAO: Disabled" (-1) era religado pelo else`] } else if self.config_file.get_value("rendering", "ssao_quality").to::<i64>() == EnvironmentSsaoQuality::MEDIUM.ord() as i64 { environment.set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::HIGH, false, 0.5, 2, 50.0, 300.0); } else { environment.set_ssao_enabled(true); RenderingServer::singleton().environment_set_ssao_quality(EnvironmentSsaoQuality::MEDIUM, true, 0.5, 2, 50.0, 300.0); } if self.config_file.get_value("rendering", "ssil_quality").to::<i64>() == -1 { environment.set_ssil_enabled(false); } else if self.config_file.get_value("rendering", "ssil_quality").to::<i64>() == EnvironmentSsilQuality::MEDIUM.ord() as i64 { environment.set_ssil_enabled(true); RenderingServer::singleton().environment_set_ssil_quality(EnvironmentSsilQuality::MEDIUM, false, 0.5, 2, 50.0, 300.0); } else { environment.set_ssil_enabled(true); RenderingServer::singleton().environment_set_ssil_quality(EnvironmentSsilQuality::HIGH, true, 0.5, 2, 50.0, 300.0); } environment.set_glow_enabled(self.config_file.get_value("rendering", "bloom").to::<bool>()); environment.set_volumetric_fog_enabled(self.config_file.get_value("rendering", "volumetric_fog").to::<bool>()); }`. O comentário `// upstream bug fix: …` fica **sozinho numa linha, imediatamente acima da linha `} else if`** do SSAO (única linha com esse texto no arquivo). **Não** trocar o quirk "MEDIUM → HIGH sem half_size / senão MEDIUM com half_size"; **não** tocar no SSIL; **não** tratar `from_ord(5)` (research D8)
- [ ] T009 [US1] Adicionar `mod settings;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod main_scene;`); nenhuma outra linha muda
- [ ] T010 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. Se aparecer `field … is never read` ou `E0271` no `vdict!`, conferir T007 (`#[var]` em `metalfx_supported`; `&vdict!` nos aninhados) — não inventar outra solução
- [ ] T011 [US1] Vínculo do autoload (plan §"Edição do vínculo"): (a) criar `oxide-godot/menu/settings.tscn` com **exatamente** 3 linhas — `printf '[gd_scene format=3]\n\n[node name="Settings" type="Settings"]\n' > oxide-godot/menu/settings.tscn` — e conferir `cat -A` (sem `uid=`, sem `script`, sem `ext_resource`); (b) `grep -n '^Settings=' oxide-godot/project.godot` → `25:Settings="*res://menu/settings.gd"`; `sed -i '25s|^Settings="\*res://menu/settings\.gd"$|Settings="*res://menu/settings.tscn"|' oxide-godot/project.godot`; `grep -n '^Settings=' oxide-godot/project.godot` → `25:Settings="*res://menu/settings.tscn"`; `git diff --stat -- oxide-godot/project.godot` = `1 file changed, 1 insertion(+), 1 deletion(-)`; `git diff -- oxide-godot/project.godot | grep -c '^[-+][^-+]'` = 2 (nenhuma outra linha — `run/main_scene` l.15 e `toggle_fullscreen` l.172–177 intocados)
- [ ] T012 [US1] Apagar `oxide-godot/menu/settings.gd` e `oxide-godot/menu/settings.gd.uid` (`git rm`); verificar `grep -rn 'uid://b04fekxdgdq0k' oxide-godot/ | grep -v '/.godot/'` vazio, `grep -rn 'menu/settings.gd' oxide-godot/ --include='*.tscn' --include='*.godot' --include='*.gd' --include='*.cfg'` vazio; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 0; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 0; `git status --short` = exatamente `D menu/settings.gd`, `D menu/settings.gd.uid`, `M project.godot`, `M lib.rs`, `?? settings.rs`, `?? menu/settings.tscn`
- [ ] T013 [US1] Validação headless (sem editor aberto): import `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log` → `Initialize godot-rust` ×1, sem `ERROR` novo; `ls oxide-godot/menu/` → `settings.tscn` presente, **nenhum** `settings.tscn.uid` gerado (se aparecer, anotar e incluir no commit — research D1 diz que não aparece); `timeout 20 /usr/bin/godot.x86_64 --headless --path . main/main.tscn 2>&1 | tee /tmp/run_main.log` → exit 124, grep de regressão (`ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`) vazio — **rodar 2 vezes** (regra do erro intermitente); `menu/menu.tscn` → 124, grep vazio; `level/level.tscn` → 124, grep vazio; WARNINGs iguais à baseline (T006); `md5sum "$INI"` igual a T005 (o boot com o autoload em Rust não grava o `.ini`). **NUNCA** rodar `menu/settings.tscn` isolada. Fluxo exercitado em `main.tscn`: `Settings.ready` → `load_settings`; `Main.ready` `get("config_file")` → `display_mode`; `Menu.ready` `call("apply_graphics_settings", …)` + host automático; `Level.ready` `call("apply_graphics_settings", …)` + `gi_type`/`gi_quality`; `FlyingForklift.ready` `shadow_mapping`
- [ ] T014 [US1] Probes descartáveis (`-s`, scripts no scratchpad — **nunca** dentro do repo; modelos em research §E.3/E.3b, adaptados de `ZzSettings` para `Settings`), rodados de `oxide-godot/oxide-godot/` com `timeout 30 /usr/bin/godot.x86_64 --headless --path . -s <scratchpad>/probe_x.gd`: **(A, `.ini` presente)** `/root/Settings.get_class()` = `Settings`, `name` = `Settings`, `get("config_file")` é `ConfigFile` e é o mesmo objeto em duas leituras, `get("metalfx_supported")` = false, `has_method` de `load_settings`/`save_settings`/`apply_graphics_settings` = true, `ClassDB.class_has_integer_constant("Settings", c)` = true para as 6 constantes com valores 0/1/2/0/1/2, `config_file.get_sections()` = `["video", "rendering"]` e chaves na ordem de `settings.gd`; `save_settings()` → **`diff "$INI" <scratchpad>/settings.ini.baseline` vazio** (SC-003; se T005 não tinha baseline, pular esta linha); FR-024: `config_file.set_value("rendering","ssao_quality",-1)` + `apply_graphics_settings(root, Environment.new(), Node.new())` → `ssao_enabled == false`; MEDIUM → true; HIGH → true; `ssil_quality = -1` → `ssil_enabled == false`; `scale_filter = 5` → `root.scaling_3d_mode == 5` sem panic; `shadow_mapping = false` + `DirectionalLight3D` filha do `scene_root` → `shadow_enabled == false`; **restaurar** todos os valores alterados no fim (o probe não salva depois disso). **(B, `.ini` movido de lado — `mv "$INI" "$INI.aside"` antes, `mv` de volta depois, md5 conferido)**: após o boot, `config_file` tem 15 chaves com os valores/tipos `4 int, 1 int, 0 int, 1.0 float, 2 int, false, 0 int, 0 int, true, 1 int, 1 int, 2 int, -1 int, true, true` (tabela de data-model.md; o `settings.gd` já foi apagado — comparar com a tabela, não com `load("res://menu/settings.gd")`), e `FileAccess.file_exists("user://settings.ini")` = false. Os `WARNING … leaked at exit` gerados pelos objetos do próprio probe não contam
- [ ] T015 [US1] Verificações mecânicas e contrato (`contracts/settings.md` §"Verificação antes do commit"): `R=oxide_godot_core/oxide_godot_lib/src/settings.rs`; `grep -n '#\[func\]' -A1 $R | grep 'fn '` = `load_settings`, `save_settings`, `apply_graphics_settings` (3, nenhum outro); `grep -n '#\[var\]' -A1 $R | grep -oE '(metalfx_supported|config_file)'` = os 2; `grep -c '#\[export\]' $R` = 0; `grep -c '#\[constant\]' $R` = 6; `grep -c 'upstream bug fix' $R` = 1 e a linha seguinte é `} else if …ssao_quality…`; `grep -c 'else if' $R` = 2 (SSAO e SSIL); `grep -c 'get_window()' $R` = 2 (`input` ×1, `apply_graphics_settings` ×1 — modo na janela própria); `grep -n 'fn init' $R` = 1 (manual); `grep -c 'vdict!' $R` = 3; `grep -c 'CONFIG_FILE_PATH' $R` = 3 (const + load + save); `grep -n 'name="Settings" type="Settings"' oxide-godot/menu/settings.tscn` = l.3; `wc -l < oxide-godot/menu/settings.tscn` = 3; `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/{bullet,flying_forklift,level,menu,main_scene}.rs` **vazio** (FR-014; os 12 sítios consumidores intocados); `git diff --quiet HEAD -- CLAUDE.md oxide_godot_core/Cargo.toml && echo unchanged`; `ls oxide_godot_core/oxide_godot_lib/src/ | wc -l` = 16
- [ ] T016 [US1] Registrar em `docs/upstream-bugs.md` a entrada **#3** (texto de research §"Entrada #3 de docs/upstream-bugs.md": defeito `settings.gd:88-95` `if` sem `elif` → "SSAO: Disabled" religado; reproduzido em headless no original; script/cena + `specs/005-v1-settings-autoload` FR-020–FR-024; correção `else if` em `settings.rs` `apply_graphics_settings` com comentário `// upstream bug fix`, ramos/parâmetros/SSIL intocados; coluna Commit = o **assunto** do commit de T017) e em `docs/v2-backlog.md` a linha **25** (research §"Backlog v2 candidato": `menu/settings.gd` (port 5) — rever o mapeamento SSAO "Medium → HIGH sem half_size / High → MEDIUM com half_size"; intenção ambígua, preservado). `grep -c '^| [0-9]' docs/upstream-bugs.md` = 3; `grep -c '^| [0-9]' docs/v2-backlog.md` = 25; não duplicar itens 1–24 nem entradas 1–2
- [ ] T017 [US1] Commit único do port na `main` (autor the repository author; `git add` explícito de `oxide_godot_core/oxide_godot_lib/src/settings.rs`, `oxide_godot_core/oxide_godot_lib/src/lib.rs`, `oxide-godot/menu/settings.tscn`, `oxide-godot/project.godot`, `docs/upstream-bugs.md`, `docs/v2-backlog.md` — as 2 deleções já estão em staging pelo `git rm`; conferir `git status --short` e `git diff --cached --stat` = 8 arquivos, nada mais). Assunto: `Port settings.gd → Settings (Node, autoload); menu/settings.tscn nova; project.godot: Settings="*res://menu/settings.tscn"`. Corpo (linhas `- `): `- vínculo por autoload: cena mínima + project.godot (única edição), regra declarada na spec` (classe nativa não pode ser autoload direta; nome do node vem da chave do autoload); `- #[var] config_file / metalfx_supported; DEFAULTS como dicionário aninhado iterado (ordem do .ini preservada); 6 #[constant] dos enums; init manual`; `- apply_graphics_settings linha a linha: modo de janela na janela própria e quirk "média aplica alta" preservados`; `- upstream bug fix: SSAO "Disabled" (-1) era religado pelo else (settings.gd:90 usava if em vez de elif) — agora if/else if/else; comentário // upstream bug fix no ponto exato`; `- docs/upstream-bugs.md: entrada #3.`; `- backlog v2: item 25`; `- settings.gd e settings.gd.uid removidos: ZERO .gd no projeto — v1 sem GDScript`; `- consumidores (bullet, flying_forklift, level, menu, main_scene) intocados (backlog 1)`. Anotar o hash
- [ ] T018 [US1] **Checkpoint do usuário (validação visual, plan §"Validação visual")** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: boot → menu com o modo de janela salvo; Settings reflete o `settings.ini` atual (gravado pelo `settings.gd` original — lido sem mudança); alterar opções → Apply aplica e persiste (reabrir Settings; **fechar e reabrir o jogo**); Cancel descarta; apagar o `.ini` → padrões e o arquivo só volta após um Apply; level: `Shadow mapping` off → nenhuma luz projeta sombra; **SSAO: Disabled → SSAO realmente desligado** (no original ficava ligado — é o único comportamento diferente, declarado: upstream-bugs #3); SSAO Medium/High e SSIL Disabled/Medium/High como no original; **F11 e Alt+Enter** alternam tela cheia exclusiva ↔ janela no menu e no level; jogo inteiro menu → Play → level → ESC → menu sem nenhum `.gd`. No editor: `menu/settings.tscn` raiz `Settings` do tipo `Settings` sem script; Project Settings → Autoload: `Settings` → `res://menu/settings.tscn`, singleton ligado; nenhum `.gd` no FileSystem. Divergência → commit `Fix port settings.gd …`. Só seguir para o Polish com o OK explícito

**Checkpoint**: 0 `.gd` no projeto; autoload em Rust; jogo jogável de ponta a ponta em Rust.

---

## Phase 4: Polish — verificação final da v1

**Purpose**: verificação mecânica da v1 inteira (quickstart §8) e validação visual completa
(SC-002); com o OK, a v1 é declarada concluída (SC-008). **Tag e branch `v2` não fazem parte
destas tasks** — decisões do usuário.

- [ ] T019 Verificação mecânica do marco e da v1 (quickstart §8): `find oxide-godot -name '*.gd' -not -path '*/addons/*'` vazio e `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'` vazio (SC-001); `git log --oneline 619575d..HEAD | grep -c '^[0-9a-f]* Port '` = 1 (`Fix port …` não contam); `git diff --stat 619575d -- '*.rs'` = só `lib.rs` (+1) e `settings.rs` (novo); `git diff --stat 619575d -- oxide-godot/project.godot` = `1 insertion(+), 1 deletion(-)` e `grep -n '^Settings=' oxide-godot/project.godot` = `res://menu/settings.tscn`; `git diff --stat 619575d -- oxide-godot/ | grep -v 'settings\|project.godot'` vazio (nenhuma outra cena/arquivo do projeto Godot mudou); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 15 módulos (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot flying_forklift level menu main_scene settings`); `git diff --quiet 619575d -- CLAUDE.md oxide_godot_core/Cargo.toml && echo unchanged`; `grep -c '^| [0-9]' docs/upstream-bugs.md` = 3; `grep -c '^| [0-9]' docs/v2-backlog.md` = 25; FR-015: `grep -nE '\.call\(|call_deferred\(|\.get\("' oxide_godot_core/oxide_godot_lib/src/settings.rs` **vazio** (o `Settings` não faz chamada dinâmica alguma; `propagate_call_ex` não casa com o padrão e é API base); `grep -rn '/root/Settings' oxide_godot_core/oxide_godot_lib/src/ | wc -l` = 10 (os consumidores, inalterados)
- [ ] T020 Validação headless final da v1 (sem editor aberto): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; import headless com `Initialize godot-rust` e sem `ERROR` novo; `main/main.tscn` (×2, regra do intermitente), `menu/menu.tscn`, `level/level.tscn`, `level/forklift/flying_forklift.tscn`, `player/player.tscn`, `enemies/red_robot/red_robot.tscn` headless com grep de regressão vazio; `md5sum "$INI"` inalterado pelos boots
- [ ] T021 **Validação visual completa do usuário (SC-002 — v1 inteira)**: sessão lado a lado com `../oxide_godot_origins/`: boot → menu (Play, Play Online → Host, Settings com as 15 linhas refletindo e gravando cada opção — Apply, Cancel, reabrir, reiniciar o jogo, conferir `user://settings.ini` —, Quit) → loading com barra → level jogável (jogador, tiro, tremor de câmera, robôs com peças e respawn 15 s, porta, empilhadeiras variadas, GI/sombras/SSAO/SSIL/bloom/névoa conforme as opções) → ESC → menu → Play de novo; F11/Alt+Enter em qualquer tela. Única diferença esperada e declarada: SSAO Disabled desliga o SSAO (upstream-bugs #3). Editor: as 5 cenas raiz Rust sem script + `settings.tscn`; Autoload em cena; nenhum `.gd`. **Com o OK do usuário: marcar T001–T021 `[x]`, commitar só `tasks.md` (`Tasks 005: marco E concluído — v1 sem GDScript`) e declarar a v1 concluída (SC-008; Princípio I: nenhum script `.gd` restante e o jogo jogável de ponta a ponta)**. Tag e branch `v2`: fora destas tasks

---

## Dependencies & Execution Order

### Ordem obrigatória

```
Phase 1 (Setup: T001–T006)          — baseline + cópia do settings.ini real
  → Phase 3 US1 (T007–T018)  → commit único (Settings; project.godot 1 linha; ZERO .gd)
  → Phase 4 Polish (T019–T021) → v1 concluída
```

- **Phase 2 (Foundational)**: não existe.
- **Ordem interna da US1** (dependências estritas): módulo parte 1 (T007) → parte 2 (T008, mesmo
  arquivo) → `mod` (T009) → build (T010) → vínculo (T011: cena nova + `project.godot`) → deleções
  (T012) → headless (T013) → probes (T014) → mecânicas/contrato (T015) → docs (T016) → commit
  (T017) → checkpoint (T018). O vínculo só é editado depois do build porque `type="Settings"`
  precisa existir na lib para o autoload resolver no import. As deleções (T012) vêm depois do
  vínculo (T011) para o projeto nunca apontar para um `.gd` inexistente.
- **Checkpoints do usuário** (T018, T021) são bloqueantes.

### Parallel Opportunities

- Setup: T004 é [P] em relação a T002/T003/T005/T006 (só lê diretórios e faz greps).
- Dentro da US1 nenhuma task é [P]: T007→T008 escrevem o mesmo arquivo; tudo o mais consome o
  passo anterior. T016 (docs) poderia vir antes, mas edita arquivos que entram no mesmo commit —
  manter sequencial.

### Parallel Example

```bash
# Único par realmente independente (Phase 1):
Task: "T002 cargo build → contar warnings"
Task: "T004 grep de colisão de nomes; ls das bindings; contagens de docs"
```

---

## Implementation Strategy

### MVP = marco inteiro (User Story 1)

1. Phase 1: Setup (T001–T006) — baseline e `settings.ini.baseline`.
2. Phase 3: US1 (T007–T018) — `settings.gd` portado, autoload em cena, commit, OK do usuário.
3. **PARAR E VALIDAR**: primeiro (e único) autoload da v1 e a primeira edição do `project.godot`;
   qualquer problema com o vínculo por cena ou com os 12 acessos dinâmicos aparece em T013.

### Incremental Delivery

Um só incremento: após o commit de T017 o projeto tem **zero** `.gd` e o jogo é jogável; a Phase 4
só verifica e declara.

### Se algo falhar no meio

Não commitar parcial. Ou o port inteiro (módulo + `lib.rs` + cena nova + `project.godot` +
deleções + docs) entra no commit, ou nada:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/settings.rs oxide-godot/menu/settings.tscn`
volta à árvore limpa de `619575d`, que é jogável (com o `settings.gd`). Se a implementação exigir
algo não previsto nas tasks (outro arquivo, outra linha do `project.godot`, outra correção), parar
e reportar.

---

## Notes

- Nenhuma task cria helper, trait, módulo comum, teste ou log novo. `DEFAULTS` continua dicionário
  aninhado iterado; `apply_graphics_settings` mantém as leituras repetidas do `config_file`.
- **Uma** correção de bug (SSAO `if` → `else if`), com os 4 requisitos. Qualquer outro defeito →
  parar e reportar; em dúvida, é melhoria (backlog).
- `settings.tscn` é criada como texto (3 linhas, sem `uid=`); o import não gera `.uid` para ela
  (research D1). Se o usuário salvar a cena no editor e o Godot acrescentar `uid="…"` ao cabeçalho,
  é mudança cosmética futura — fora deste marco.
- Nomes `config_file`, `metalfx_supported`, `load_settings`, `save_settings`,
  `apply_graphics_settings` e as 6 constantes são contrato (FR-013) — copiar, nunca "traduzir".
- O grep de regressão headless é `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`;
  WARNINGs da baseline (`HDR output`, `Physics interpolation`) não contam; em `main.tscn`, as 5
  linhas do renderizador dummy não contam se sumirem na 2ª execução (3 seguidas = regressão);
  `WARNING … leaked at exit` dos probes `-s` são do próprio probe.
- **Nunca** rodar `menu/settings.tscn` isolada em headless (autoload + cena = dois `Settings`).
- Nenhum editor Godot aberto durante validações headless — avisar o usuário antes; nunca matar o
  processo dele.
- O `settings.ini` real do usuário é tocado só por `save_settings` no probe A (regrava idêntico —
  SC-003) e movido/restaurado no probe B; conferir o md5 após cada probe.
- O commit (T017) vem **antes** do checkpoint visual; divergência encontrada pelo usuário é
  corrigida em commit `Fix port …` (nunca `Port …`, para `grep -c '^[0-9a-f]* Port '` continuar = 1).
- **Staging**: antes de qualquer commit que não seja o do port, `git status` não pode ter deleções
  de `.gd` em staging (lição do Marco C).
- Arquivos que **nunca** mudam neste marco: `.gdextension`, `Cargo.toml`, `CLAUDE.md`, os 14
  módulos Rust existentes, todas as `.tscn` existentes, e todas as linhas do `project.godot`
  exceto a 25.
