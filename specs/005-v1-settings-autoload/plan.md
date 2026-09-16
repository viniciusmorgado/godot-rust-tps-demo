# Implementation Plan: Marco E — autoload Settings (v1 raw port, último script)

**Branch**: `main` (a v1 vive na `main`; o port é um commit atômico que deixa o jogo jogável) | **Date**: 2026-09-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-v1-settings-autoload/spec.md`

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.1). Tradução direta; nenhuma abstração,
refatoração ou otimização. **Uma** correção conservadora de bug do upstream (SSAO desligado,
spec FR-020–FR-024) dentro do único commit do marco — `docs/upstream-bugs.md` passa a 3 entradas.

## Summary

Portar `menu/settings.gd` (107 l., autoload `Settings`, `extends Node`) para `Settings: Node` em
`oxide_godot_core/oxide_godot_lib/src/settings.rs`, concluindo a v1 com **zero `.gd`** no projeto.
Vínculo especial de autoload: uma classe nativa não pode ser autoload direto, então o equivalente
da "troca de `type`" é uma cena mínima nova `oxide-godot/menu/settings.tscn` (um node,
`name="Settings" type="Settings"`) apontada por `project.godot:25`
(`Settings="*res://menu/settings.gd"` → `Settings="*res://menu/settings.tscn"`) — primeira e
única edição do `project.godot` na v1. Abordagem técnica: `fn init` manual (a ordem dos `var` do
original: `metalfx_supported` → `DEFAULTS` → `config_file`), `DEFAULTS` como `VarDictionary`
aninhado (`vdict!`) iterado em `load_settings` (ordem de inserção preservada → `.ini` idêntico);
`#[var] config_file: Gd<ConfigFile>` para os 10 acessos `get("config_file")` dos 5 consumidores
Rust continuarem válidos **sem edição**; `#[func] load_settings/save_settings/apply_graphics_settings`
(3 argumentos, alvo de `call` por nome); enums `GIType`/`GIQuality` como 6 `#[constant]`; SSAO
com `else if` + comentário `// upstream bug fix`. Todo o mapeamento compilou num rascunho (0
warnings) e foi exercitado em headless com o autoload **em cena** apontando para a classe de
rascunho: `main.tscn`/`menu.tscn`/`level.tscn` sem erro, 15 padrões idênticos (valor **e** tipo)
aos do `settings.gd`, `.ini` regravado byte a byte igual, bug do SSAO reproduzido no original e
corrigido no rascunho — [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext) em
`[workspace.dependencies]` com `features = ["experimental-threads"]` (desde o Marco D). **Nenhuma
mudança de crate neste marco.**

**Primary Dependencies**: gdext 0.5.5 (API prebuilt 4.6 — manter); Godot 4.7.2 stable em
`/usr/bin/godot.x86_64`. `.gdextension` com `reloadable = true`, lib debug em
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings em
`oxide_godot_core/target/debug/build/godot-core-4eba5d49e15a0d7e/out/` (`ls -dt … | head -1`).

**Storage**: `user://settings.ini` via `ConfigFile` — nesta máquina
`~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini` (`config/name` do
`project.godot`). Criado **só** por `save_settings` (Apply); `load_settings` não grava.

**Testing**: `cargo build` (perfil debug, 0 warnings) + Godot headless (import + `main.tscn`,
`menu.tscn`, `level.tscn`; **nunca** `settings.tscn` isolada). Probes `-s` descartáveis fora do
repo (research §E) para SC-003 e FR-024. Validação visual pelo usuário (Settings completo, F11,
persistência entre execuções).

**Target Platform**: Linux x86_64 desktop (driver Vulkan → `metalfx_supported = false`)

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + projeto Godot (`oxide-godot/`)

**Performance Goals**: paridade com o original (não otimizar — Princípio I)

**Constraints**: Princípios I e II da constituição v1.3.1; nomes `config_file`, `load_settings`,
`save_settings`, `apply_graphics_settings` idênticos; os 5 módulos consumidores byte a byte
intactos; um único commit; `.gd` + `.gd.uid` no mesmo commit; `project.godot` muda em exatamente 1
linha; melhorias só em `docs/v2-backlog.md`; correção **única** do SSAO com os 4 requisitos da
cláusula; `CLAUDE.md` intocado (decisão FR-019 abaixo).

**Scale/Scope**: 1 script, 107 linhas de GDScript, 1 cena nova (3 linhas), 1 linha do
`project.godot`, 1 commit, 0 arquivos Rust existentes tocados além de `lib.rs` (+1 linha).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Princípio I — Porte em Três Fases

| Regra | Status | Evidência |
|---|---|---|
| Fase declarada em spec/plan/tasks | ✅ | spec.md "Fase: v1 (constituição v1.3.1)"; este plan "Fase: v1" |
| Tradução direta, sem remodelar nodes/cenas | ✅ | Nenhuma cena existente muda; a cena nova tem só o node do autoload; `project.godot` 1 linha |
| Nenhuma abstração/refatoração/otimização | ✅ | Um módulo; `DEFAULTS` continua dicionário aninhado iterado (não vira lista/struct/helper); `apply_graphics_settings` linha a linha com as leituras repetidas do `config_file` (sem variáveis locais "de limpeza" além das exigidas pelo tipo); quirks preservados (research D9): `get_window()` **próprio** para o modo de janela, "média aplica alta" no SSAO, `FIXME` das sombras (não religa), ordem de inserção dos padrões, `metalfx_supported` avaliado na construção |
| Melhorias → `docs/v2-backlog.md` no mesmo commit | ✅ | Item 25 (quirk 2 do SSAO) em research §"Backlog v2 candidato"; nada mais identificado |
| Correção de bugs — 4 requisitos | ✅ | SSAO desligado: (a) spec FR-020–FR-024; (b) `// upstream bug fix: …` na linha imediatamente acima do `else if` em `settings.rs` (research D8); (c) mensagem do commit (quickstart §7); (d) `docs/upstream-bugs.md` entrada #3 no mesmo commit. Defeito **reproduzido** no original em headless (§E.4: `ssao_quality = -1` → `ssao_enabled = true`); a correção é a troca `if` → `else if`, nada mais |

### Princípio II — Ciclo de Porte Verificável (v1.3.1)

| Regra | Status | Evidência |
|---|---|---|
| Uma classe por script, mesma base | ✅ | `Settings: Node` (`extends Node`; o autoload não tem node em cena — a base é a do script) |
| Nome de classe sem colisão | ✅ | `ls out/classes/ \| grep -i settings` → só `editor_settings`, `label_settings`, `project_settings`, `mesh_convex_decomposition_settings`, `open_xr_android_thread_settings_extension`; grep do `CLAUDE.md` nos `.gd`: nenhum identificador `Settings` (o único `.gd` é o próprio, apagado no commit) |
| Vínculo por troca de `type`; nenhum `.gd` ponte | ✅ (adaptado) | Autoload = script **ou** cena; classe nativa não pode ser autoload direto. Equivalente exato: cena nova de 1 node com `type="Settings"` + `project.godot:25`. Declarado na spec (§"Vínculo por tipo de um autoload"); registrado em Complexity Tracking como não-violação. **Confirmado em headless** (§E.2): `/root/Settings` é a classe nativa, `name` vem da chave do autoload |
| Nomes de `#[func]`/propriedades idênticos | ✅ | `config_file` (`#[var]`), `load_settings`, `save_settings`, `apply_graphics_settings(window, environment, scene_root)` — [contracts/settings.md](contracts/settings.md); probe §E.3 (`has_method` ×3, `get("config_file")` devolve o mesmo `ConfigFile`) |
| Propriedades exportadas/replicadas | ✅ | `settings.gd` não exporta nem replica; `config_file` e `metalfx_supported` viram `#[var]` (uso `NONE`, sem inspector) — nomes preservados |
| `cargo build` sem warnings novos | ✅ | Baseline 0; rascunho 0 (após `#[var]` em `metalfx_supported` — research D3) |
| Validação headless (import + cena) | ✅ | quickstart §2–4; baseline medida em `6f4d9ba` (§E.1); `settings.tscn` **não** é rodada isolada (instanciaria dois `Settings`) |
| Commit com script + vínculo na mensagem | ✅ | quickstart §7: cita `settings.tscn`, `project.godot`, a correção do SSAO e o fim da v1 |
| `.gd` + `.gd.uid` apagados no mesmo commit | ✅ | `git rm menu/settings.gd menu/settings.gd.uid`; grep do uid `b04fekxdgdq0k` vazio; staging conferido antes de qualquer commit |
| Ordem de baixo para cima | ✅ | `docs/port-order.md` item 15 — último; todos os consumidores já são Rust |
| Rust não chama API customizada de GDScript | ✅ | Não resta GDScript. `propagate_call_ex("set")` é API base do engine (o original faz o mesmo). Os 5 consumidores continuam com o acesso dinâmico a `/root/Settings` (exceção da constituição l.79; backlog item 1 — fora do escopo, spec) |
| Exceção `Settings` | ✅ (encerra) | A classe nasce em Rust; a exceção deixa de existir para código novo. Os acessos dinâmicos existentes ficam para a v2 |
| Catálogo do `CLAUDE.md` reflete a baseline | ✅ | Inalterado: nenhum erro eliminado nem novo. **FR-019: decisão = não editar o `CLAUDE.md`** — o vínculo de autoload por cena está declarado na spec e no plan; o `CLAUDE.md` descreve o ciclo geral e não terá outro autoload na v1 |

**Não-violações justificadas** (Complexity Tracking): vínculo do autoload por cena nova +
`project.godot`; `#[var]` em `metalfx_supported`; `fn init` manual.

**Resultado do gate (pré-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/005-v1-settings-autoload/
├── plan.md              # Este arquivo
├── spec.md              # Especificação (commit 0e27c68)
├── research.md          # Phase 0: assinaturas por compilação, probes headless (autoload em cena, defaults, .ini, SSAO), baseline
├── data-model.md        # Phase 1: classe Settings, tabela seção/chave/tipo/default do config_file, fluxos
├── quickstart.md        # Phase 1: baseline, ciclo, probes SC-003/FR-024, verificação final da v1, formato do commit
├── contracts/
│   └── settings.md      # config_file, load_settings, save_settings, apply_graphics_settings(3), 6 constantes; 10 sítios consumidores
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado por este comando)
```

### Source Code (repository root)

```text
oxide_godot_core/
└── oxide_godot_lib/src/
    ├── lib.rs                          # +1 linha `mod settings;` (após `mod main_scene;`)
    ├── settings.rs                     # struct Settings, base=Node  NOVO
    ├── bullet.rs, flying_forklift.rs,  # INTOCADOS (consumidores dinâmicos de /root/Settings)
    ├── level.rs, menu.rs, main_scene.rs
    └── (demais 9 módulos)              # INTOCADOS

oxide-godot/
├── menu/settings.tscn                  # NOVA — 3 linhas: `[gd_scene format=3]`, vazia, `[node name="Settings" type="Settings"]`
├── menu/settings.gd (+ .uid)           # APAGAR (git rm)
└── project.godot                       # l.25: Settings="*res://menu/settings.gd" → Settings="*res://menu/settings.tscn"

docs/upstream-bugs.md                   # entrada #3 (SSAO desligado)
docs/v2-backlog.md                      # item 25 (quirk "média aplica alta")
CLAUDE.md                               # INTOCADO (FR-019: decisão de não editar)
```

**Structure Decision**: um módulo por script (Princípio I); `settings.rs` / `mod settings;` — sem
conflito com nada no crate. A classe chama-se `Settings`; o node em `/root/Settings` mantém o nome
pela chave do autoload. Detalhes em [research.md](research.md) D1–D10.

## Edição do vínculo (conferido em 2026-09-16 — reconferir com `grep -n` antes de editar)

| Arquivo | Ação | Antes | Depois |
|---|---|---|---|
| `oxide-godot/menu/settings.tscn` | **criar** (3 linhas, sem `uid=` no cabeçalho — o import não gera `.uid` para `.tscn` nem reclama; §E.2) | — | `[gd_scene format=3]` / linha vazia / `[node name="Settings" type="Settings"]` |
| `oxide-godot/project.godot` l.25 | 1 linha (`grep -n '^Settings=' project.godot` → 25) | `Settings="*res://menu/settings.gd"` | `Settings="*res://menu/settings.tscn"` |
| `oxide-godot/menu/settings.gd` + `settings.gd.uid` (`uid://b04fekxdgdq0k`) | `git rm` | — | — ; `grep -rn 'b04fekxdgdq0k\|menu/settings.gd' oxide-godot/ \| grep -v /.godot/` vazio |

Nada mais em `project.godot` muda (`git diff --stat -- oxide-godot/project.godot` = `2 +-`); a ação
`toggle_fullscreen` (l.172–177) e `run/main_scene` (l.15) ficam como estão.

## Validação visual (SC-002 — feita pelo usuário no jogo, comparando com `../oxide_godot_origins/`)

| O que conferir |
|---|
| Boot → menu com o modo de janela salvo; Settings mostra as 15 linhas com os valores do `settings.ini` atual (o arquivo gravado pelo `settings.gd` original é lido sem mudança) |
| Alterar opções → Apply: efeito imediato (modo de janela, vsync, fps, escala/filtro, TAA/MSAA/AA, bloom, névoa) e persistência (reabrir Settings; reiniciar o jogo); Cancel descarta |
| Apagar `user://settings.ini` → boot com os padrões (tela cheia exclusiva, vsync, FSR2, VoxelGI baixa, SSAO média, SSIL off, sombras/bloom/névoa on); o arquivo só reaparece após um Apply |
| Level: `Shadow mapping` off → nenhuma luz projeta sombra; **SSAO: Disabled → SSAO realmente desligado** (no original fica ligado — a correção); SSAO Medium/High e SSIL Disabled/Medium/High como no original |
| F11 e Alt+Enter alternam tela cheia exclusiva ↔ janela em qualquer tela (menu e level) |
| Editor: `menu/settings.tscn` com a raiz `Settings` do tipo `Settings`, sem script; Project Settings → Autoload: `Settings` → `res://menu/settings.tscn`, singleton ligado; nenhum `.gd` no FileSystem |

## Complexity Tracking

> Preenchido para registrar **não-violações justificadas** (o gate não tem violações).

| Item | Por que é necessário | Alternativa mais simples rejeitada porque |
|---|---|---|
| Vínculo do autoload por **cena nova** (`menu/settings.tscn`, 1 node) + edição de **1 linha** do `project.godot` | O Godot só aceita script ou cena como autoload; uma classe nativa registrada por GDExtension não pode ser autoload direta. A cena de um node com `type="Settings"` é literalmente a "troca de `type`" do Princípio II para um node que não existia em cena alguma; o `project.godot` é onde o vínculo do autoload vive. É a primeira e única edição do `project.godot` na v1, declarada na spec | Manter um `settings.gd` ponte (`extends Settings`) — proibido pelo Princípio II (nenhum `.gd` ponte) e deixaria 1 `.gd`; registrar o singleton via `Engine.register_singleton` no `ExtensionLibrary` — muda a arquitetura (deixa de ser node em `/root/Settings`, quebra `get_node("/root/Settings")` dos 5 consumidores e o `_input`) |
| `#[var]` em `metalfx_supported` (além de `config_file`) | O campo só é lido no `init` (padrão de `scale_filter`); sem `#[var]` o compilador emite `field is never read` (warning — proibido). No original é um `var` de topo = propriedade de script; `#[var]` (uso `NONE`) reproduz isso sem inspector | Deixar o valor como variável local do `init` — o campo do original desapareceria; `#[allow(dead_code)]` — supressão de warning em vez de fidelidade |
| `fn init(base)` manual em vez de `#[class(init)]` + `#[init(val = …)]` | Os padrões dependem de `metalfx_supported` (`scale_filter`), e `#[init(val)]` não pode referenciar outro campo. O `init` manual reproduz a ordem dos `var` do original (`metalfx_supported` → `DEFAULTS` → `config_file`) | Recalcular o nome do driver dentro do `vdict!` — duplicaria a leitura do `RenderingServer`, que o original faz uma vez |

## Constitution Check — re-avaliação pós-design (Phase 1)

Re-avaliado após research.md, data-model.md, contracts/ e quickstart.md:

- Nenhum artefato introduz módulo comum, trait ou helper; `DEFAULTS` permanece dicionário
  aninhado iterado; `apply_graphics_settings` mantém as leituras repetidas e a ordem do original;
  SSIL intacto; quirk "média aplica alta" intacto (backlog 25). ✅
- A correção do SSAO é a menor possível (`if` → `else if`), isolada pelo comentário
  `// upstream bug fix`, declarada na spec, com entrada #3 preparada para `docs/upstream-bugs.md`
  e citada na mensagem do commit (quickstart §7). O defeito foi **reproduzido** no original
  (§E.4) e o rascunho devolve `ssao_enabled = false` para −1. ✅
- Contrato reproduz os nomes conferidos nos 10 sítios dos consumidores (`get("config_file")` ×8,
  `call("apply_graphics_settings", 3 args)` ×3, `call("save_settings")` ×1) — todos resolveram
  em headless contra a classe de rascunho (`main.tscn` completo). ✅
- Os 15 padrões batem em valor **e** tipo com `settings.gd` (§E.3, comparação programática), a
  ordem seção/chave é a mesma e o `.ini` regravado é byte a byte idêntico (SC-003). ✅
- `Scaling3DMode::from_ord(5)` (Nearest do 4.7, ausente da API 4.6) não faz panic — a API 4.6
  aceita 0..=5 (5 = `MAX`) e transmite o ordinal ao engine (§E.3). ✅
- `CLAUDE.md` intocado (FR-019 decidido); `project.godot` 1 linha; nenhuma cena existente
  editada. ✅

**Resultado do gate (pós-Phase 1)**: PASS.
