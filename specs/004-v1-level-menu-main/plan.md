# Implementation Plan: Marco D — empilhadeira, level, menu e main (v1 raw port)

**Branch**: `main` (a v1 vive na `main`; cada port é um commit atômico que deixa o jogo jogável) | **Date**: 2026-09-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-v1-level-menu-main/spec.md`

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.1). Tradução direta; nenhuma abstração,
refatoração ou otimização. Nenhuma correção de bug prevista — `docs/upstream-bugs.md` permanece
com 2 entradas.

## Summary

Portar `flying_forklift.gd` (21 l.), `level.gd` (127), `menu.gd` (460) e `main.gd` (33) para
quatro classes gdext 0.5.5 — `FlyingForklift: CharacterBody3D` (base = tipo do node, regra do
Princípio II v1.3.1), `Level: Node3D`, `Menu: Node`, `Main: Node` — trocando o `type` da raiz de
`flying_forklift.tscn`, `level.tscn`, `menu.tscn` e `main.tscn` e apagando `.gd` + `.gd.uid` no
mesmo commit, na ordem forklift → level → menu → main. Ao final só `menu/settings.gd` (autoload)
permanece em GDScript, acessado dinamicamente via `/root/Settings` (exceção do Princípio II).
Abordagem técnica: sinais `quit` e `replace_main_scene(PackedScene)` no bloco `#[godot_api]`
principal, conectados **por nome** pelo `main.gd`/`Main` (duck typing do original); `Level`
consome `EnemyRobot` (`signals().exploded()`) e `Player` (`set_player_id`) com acesso tipado —
duas aberturas de visibilidade `pub(crate)`; `add_player`/`del_player` conectados a
`peer_connected`/`peer_disconnected` por closure tipada (gdext não tem parâmetro default);
**feature `experimental-threads` do gdext ativada no port 3** — única forma de obter
`ResourceLoader::load_threaded_request/get_status/get`, que o menu exige para a barra de loading
(decisão do usuário; `Cargo.toml` + linha no `CLAUDE.md`); enums do engine gravados no
`config_file` pelos seus inteiros (`.ord()`); `Main` faz `has_signal` + `connect` por nome e
`call_deferred` por nome, como o original. Todo o mapeamento compilou num rascunho com a feature
ativa (0 warnings) e os sinais/métodos/inteiros foram conferidos em headless — [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext) em
`[workspace.dependencies]`. **Mudança única de configuração** (port 3, decidida pelo usuário):
`godot = { version = "0.5.5", features = ["experimental-threads"] }` — versão inalterada, nenhuma
outra feature (research D1).

**Primary Dependencies**: gdext 0.5.5 (API prebuilt 4.6 — manter); Godot 4.7.2 stable em
`/usr/bin/godot.x86_64`. `.gdextension` com `reloadable = true`, lib debug em
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-*/out/` — **dois** diretórios após a feature
(`aea5c50e7fda9d57` sem, `4eba5d49e15a0d7e` com); `ls -dt … | head -1` devolve o válido.

**Storage**: N/A (o `config_file` é do `settings.gd`, fora do escopo).

**Testing**: `cargo build` (perfil debug, 0 warnings) + Godot headless (import + `flying_forklift`,
`level`, `menu`, `main`). Sem testes unitários. Validação visual pelo usuário, incluindo o menu de
configurações completo.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + projeto Godot (`oxide-godot/`)

**Performance Goals**: paridade com o original (não otimizar — Princípio I)

**Constraints**: Princípios I e II da constituição v1.3.1; nomes de sinais/métodos/handlers e os
85 caminhos de node do menu idênticos ao GDScript; um commit por script; `.gd` + `.gd.uid` no
mesmo commit; melhorias só em `docs/v2-backlog.md`; nenhuma correção de bug (defeito objetivo →
parar e declarar em spec); `settings.gd` intocado.

**Scale/Scope**: 4 scripts, 641 linhas de GDScript, 4 cenas editadas (uma vez cada), 4 commits,
2 arquivos Rust existentes tocados só em visibilidade, `Cargo.toml` (1 linha) e `CLAUDE.md`
(linhas operacionais) tocados uma vez.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Princípio I — Porte em Três Fases

| Regra | Status | Evidência |
|---|---|---|
| Fase declarada em spec/plan/tasks | ✅ | spec.md "Fase: v1 (constituição v1.3.1)"; este plan "Fase: v1" |
| Tradução direta, sem remodelar nodes/cenas | ✅ | As 4 cenas só recebem troca de `type` e remoção de `script`/`ext_resource`; o node `main` não é renomeado |
| Nenhuma abstração/refatoração/otimização | ✅ | Um módulo por script, sem helper compartilhado; as 15 chamadas de `_make_button_group` e as ~30 cadeias `if/elif` do menu ficam como no original (research D8–D9); quirks preservados (D11): `randomize()` ×3, `add_child(player)` sem nome legível, `has_signal`/`connect`/`call_deferred` por nome, `lightmap_gi` mantido após `queue_free` |
| Melhorias → `docs/v2-backlog.md` no mesmo commit | ✅ | Candidatos 19–24 em research.md §"Backlog v2 candidato", atribuídos por script |
| Correção de bugs | N/A | Nenhum defeito objetivo conhecido; `docs/upstream-bugs.md` fica com 2 entradas. O parâmetro do sinal `replace_main_scene` (declarado sem, emitido com 1) é exigência da emissão tipada, documentado na spec — não é correção |

### Princípio II — Ciclo de Porte Verificável (v1.3.1)

| Regra | Status | Evidência |
|---|---|---|
| Uma classe por script, mesma base **efetiva do node** | ✅ | `FlyingForklift: CharacterBody3D` (`extends Node3D` é ancestral do node `flying_forklift.tscn:36` — regra esclarecida na v1.3.1; declarado em spec US1); `Level: Node3D`, `Menu: Node`, `Main: Node` |
| Nome de classe sem colisão (engine + identificadores dos `.gd` remanescentes) | ✅ | grep do `CLAUDE.md` em `a866428`: `FlyingForklift`, `Level`, `Menu`, `Main` ausentes; `menu.gd` tem `var main` (minúsculo; e é portado antes de `Main`) |
| Vínculo por troca de `type` na `.tscn`; nenhum `.gd` ponte | ✅ | Tabela "Edição das cenas" abaixo |
| Nomes de `#[func]`/sinais idênticos | ✅ | `quit`, `replace_main_scene`, 9 handlers (10 conexões), `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` — [contracts/](contracts/); probe §E.1 |
| Propriedades exportadas/replicadas | N/A | Nenhum dos 4 scripts exporta ou replica propriedades (`level.tscn` `MultiplayerSpawner` replica **cenas**, API base) |
| `cargo build` sem warnings novos | ✅ | Baseline 0; rascunho com a feature compilou com 0 |
| Validação headless (import + cena) | ✅ | quickstart.md §2–3; baseline medida em `a866428` (§E.2), incluindo o **erro intermitente do renderizador dummy** em `main.tscn` headless (regra de re-execução; a catalogar no `CLAUDE.md`) |
| Commit por port com script + cena na mensagem | ✅ | quickstart.md §8 (o do port 3 cita a feature e o `CLAUDE.md`) |
| `.gd` + `.gd.uid` apagados no mesmo commit | ✅ | Tabela "Edição das cenas"; `grep` do uid após remoção; **staging conferido antes de qualquer commit intermediário** (lição do Marco C) |
| Ordem de baixo para cima | ✅ | `docs/port-order.md` 11→14: forklift (folha), level (consome `EnemyRobot`/`Player`, emite `quit`), menu (emite `replace_main_scene`), main (consome os dois sinais por nome) |
| Rust não chama API customizada de GDScript | ✅ | Únicas chamadas dinâmicas: exceção `Settings` (`config_file` get/set, `call("apply_graphics_settings")`, `call("save_settings")`), `has_signal`/`connect` por nome no `Main` e `call_deferred` por nome (duck typing/chamadas do original), `AnimationTree`-like nenhuma. `EnemyRobot`/`Player` tipados. Verificação final por grep no quickstart |
| Exceção `Settings` | ✅ | Usada pelos 4 scripts; `settings.gd` é o Marco E; backlog item 1 |
| Catálogo do `CLAUDE.md` reflete a baseline | ✅ | Nenhum erro eliminado; **acrescenta-se** o erro intermitente de `main.tscn` headless (edição operacional, port 3) |

**Não-violações justificadas** (Complexity Tracking): feature `experimental-threads`;
`pub(crate)` em `Player::set_player_id` e `EnemyRobot::exploded`; `add_player`/`del_player` por
closure em vez de `#[func]` com default.

**Resultado do gate (pré-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/004-v1-level-menu-main/
├── plan.md              # Este arquivo
├── spec.md              # Especificação (commit dee6af2, atualizada para v1.3.1 em a866428)
├── research.md          # Phase 0: feature, assinaturas por compilação, probes, baseline (inclui o erro intermitente)
├── data-model.md        # Phase 1: 4 classes, fluxos, tabela de opções do config_file
├── quickstart.md        # Phase 1: ciclo, baseline, regra do erro intermitente, formato dos 4 commits
├── contracts/
│   ├── flying-forklift.md
│   ├── level.md         # quit; add_player/del_player por closure; visibilidades pub(crate)
│   ├── menu.md          # replace_main_scene(PackedScene); 9 handlers/10 conexões; tabela dos 85 OnReady
│   └── main.md          # go_to_main_menu/replace_main_scene/change_scene_to_packed
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado por este comando)
```

### Source Code (repository root)

```text
oxide_godot_core/
├── Cargo.toml                          # port 3: godot = { version = "0.5.5", features = ["experimental-threads"] }
└── oxide_godot_lib/src/
    ├── lib.rs                          # +4 linhas `mod`
    ├── player.rs                       # port 2: `set_player_id` → pub(crate) (só visibilidade)
    ├── red_robot.rs                    # port 2: `#[signal] fn exploded()` → pub(crate) (só visibilidade)
    ├── flying_forklift.rs              # struct FlyingForklift, base=CharacterBody3D (port 1)  NOVO
    ├── level.rs                        # struct Level,          base=Node3D          (port 2)  NOVO
    ├── menu.rs                         # struct Menu,           base=Node            (port 3)  NOVO
    └── main_scene.rs                   # struct Main,           base=Node            (port 4)  NOVO  (`mod main_scene;`)

oxide-godot/
├── level/forklift/flying_forklift.tscn (+ .gd/.uid APAGAR)   # port 1
├── level/level.tscn                    (+ .gd/.uid APAGAR)   # port 2
├── menu/menu.tscn                      (+ .gd/.uid APAGAR)   # port 3
├── main/main.tscn                      (+ .gd/.uid APAGAR)   # port 4
└── menu/settings.gd (+ .uid)           # INTOCADO — Marco E

CLAUDE.md                               # port 3: linha da feature (Toolchain) + catálogo do erro intermitente de main.tscn headless
docs/v2-backlog.md                      # itens 19 (p1), 20–21 (p2), 22–23 (p3), 24 (p4)
```

**Structure Decision**: um módulo por script (Princípio I). `main_scene.rs` em vez de `main.rs`
para não sugerir um binário; a classe chama-se `Main` e o node continua `main`. Os 85 `OnReady`
do menu ficam todos na struct, com caminhos completos (`UI/Settings/MSAA/4X` etc.) —
tabela em [contracts/menu.md](contracts/menu.md). Detalhes de cada classe em [research.md](research.md)
D4–D10.

## Edição das cenas (linhas conferidas em 2026-09-16 — reconferir com `grep -n` antes de editar)

| Port | Cena | Node (linha) | `type` antes → depois | Remover | Manter | Apagar |
|---|---|---|---|---|---|---|
| 1 | `level/forklift/flying_forklift.tscn` (127 l.) | raiz `FlyingForklift` (l.36) | `CharacterBody3D` → `FlyingForklift` | l.37 `script = ExtResource("3")`; l.5 `[ext_resource type="Script" uid="uid://dcqnfagy55nrx" path="res://level/forklift/flying_forklift.gd" id="3"]` | `FlyingForkliftModel2` (l.39) e seus `visible = false` (l.42, 45); `Collider` (l.47); `SpotLight3D` (l.114) | `flying_forklift.gd`, `.uid` |
| 2 | `level/level.tscn` (262 l.) | raiz `Level` (l.40) | `Node3D` → `Level` | l.41 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://ccxbls23ev7u3" path="res://level/level.gd" id="1"]` | `SpawnedNodes` (l.43), `RobotSpawnpoints` (l.45), `PlayerSpawnpoints` (l.59), `MultiplayerSpawner` (l.73–75), `WorldEnvironment` (l.85), `VoxelGI` (l.88), `ReflectionProbes` (l.94), instâncias de forklift | `level.gd`, `.uid` |
| 3 | `menu/menu.tscn` (845 l.) | raiz `Menu` (l.103) | `Node` → `Menu` | l.104 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://4pwshyfo5i0d" path="res://menu/menu.gd" id="1"]` | toda a árvore `UI/…` (85 caminhos), `WorldEnvironment`, `DoneTimer` (l.832–834), as **10** `[connection]` (l.836–845) | `menu.gd`, `.uid` |
| 4 | `main/main.tscn` (6 l.) | raiz `main` (l.5) | `Node` → `Main` | l.6 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://chrcwbh6kvb7i" path="res://main/main.gd" id="1"]` | `name="main"` (não renomear) | `main.gd`, `.uid` |

Cada cena é editada uma única vez; após remover o `ext_resource` as linhas seguintes deslocam −1
(a raiz do forklift → 35; do level → 39; do menu → 102; do main → 4). Conferido: `ExtResource("3")`
ocorre 1 vez em `flying_forklift.tscn` e `ExtResource("1")` 1 vez em cada uma das outras três
(nenhum outro recurso usa esses ids). Depois de cada remoção: `grep -rn "<uid>" oxide-godot/ | grep -v /.godot/`
vazio.

Também no port 3: `oxide_godot_core/Cargo.toml` l.7 `godot = "0.5.5"` →
`godot = { version = "0.5.5", features = ["experimental-threads"] }`; `CLAUDE.md` §Toolchain: uma
linha "feature `experimental-threads` habilitada desde o Marco D — sem ela o gdext não gera
`ResourceLoader::load_threaded_*` (godot-codegen `special_cases.rs:83-86`), exigidos pelo menu"; e
§Ciclo de trabalho: o erro intermitente de `main.tscn` headless (research §E.2) no catálogo de
erros pré-existentes.

## Validação visual por script (SC-002 — feita pelo usuário no editor/jogo)

| Port | O que conferir no jogo (comparar com `../oxide_godot_origins/`) |
|---|---|
| 1 | Entrar no level: as empilhadeiras flutuantes aparecem com modelos/cores variados entre si (sortear de novo a cada Play); com `Shadow mapping` desligado em Settings, o farol da empilhadeira não projeta sombra; o jogador e os robôs continuam colidindo com elas. No editor: raiz `FlyingForklift` (tipo derivado de `CharacterBody3D`), `Collider` intacto |
| 2 | Play → level carrega e aplica as configurações; trocar `GI type`/`GI quality` em Settings e reentrar: SDFGI / VoxelGI / lightmap (com `ReflectionProbes`) refletem a opção; 4 robôs nascem, respawnam 15 s após morrer; jogador nasce num ponto aleatório com `player_id` 1 (som de pouso do quirk); **ESC** libera o mouse e volta ao menu. No editor: raiz `Level`, `MultiplayerSpawner` intacto |
| 3 | Menu: Play → barra de loading progride até 100 % e o level abre em ~0,5 s; Settings → cada linha mostra pressionado o valor atual; mudar opções e Apply → aplica na hora e persiste (`user://settings.ini`; reabrir Settings mostra os novos valores; reiniciar o jogo mantém); Cancel/Back descartam; Play Online mostra Host/Connect (Host inicia o jogo como servidor); Quit fecha; botões MetalFX ocultos (Linux); teclado navega com foco em Play/Cancel como no original |
| 4 | Boot direto no menu (modo de janela salvo aplicado); Play → level; ESC → menu (peer offline recriado); Play de novo → level de novo (cena anterior liberada, sem duplicar); tudo indistinguível do original. No editor: `main.tscn` com node `main` do tipo `Main` |

## Complexity Tracking

> Preenchido para registrar **não-violações justificadas** (o gate não tem violações).

| Item | Por que é necessário | Alternativa mais simples rejeitada porque |
|---|---|---|
| Feature `experimental-threads` em `Cargo.toml` (port 3) — mudança de configuração do crate, decidida pelo usuário | `menu.gd:130,158,163` usa `ResourceLoader.load_threaded_request/get_status/get`; o codegen do gdext os omite sem a feature (`special_cases.rs:83-86`). É a única forma de reproduzir o loading em thread com API tipada | `ResourceLoader::singleton().call("load_threaded_request", …)` — chamada dinâmica ao engine, sem tipo (contra o espírito do Princípio II); carregar de forma síncrona — mudaria o comportamento (sem barra de progresso) |
| `pub(crate)` em `Player::set_player_id` e em `EnemyRobot::exploded` (`#[signal]`) — commit do port 2 | FR-025: `player.bind_mut().set_player_id(id)` e `robot.signals().exploded().connect_other(..)` tipados; o acessor do sinal herda a visibilidade do `fn` (erro E0624 no rascunho). Só a palavra de visibilidade muda | `player.set("player_id", …)` / `robot.connect("exploded", …)` — dinâmico entre classes Rust |
| `add_player`/`del_player` privados, conectados a `peer_connected`/`peer_disconnected` por closure tipada | gdext não tem parâmetro default em `#[func]`; o original conecta o método de 2 parâmetros (1 default) a um sinal de 1 argumento. A closure `|this, id| this.add_player(id as i32, None)` reproduz o efeito; nenhum script chama os métodos por nome (spec FR-008) | Dois `#[func]` (`add_player` e `add_player_at`) — nome novo não existe no original; expor `add_player(id, spawn_point)` com 2 args — a conexão por nome falharia (aridade) |

## Constitution Check — re-avaliação pós-design (Phase 1)

Re-avaliado após research.md, data-model.md, contracts/ e quickstart.md:

- Nenhum artefato introduz módulo comum, trait ou helper; o menu mantém as 15 chamadas e as
  cadeias `if/elif` do original. ✅
- Contratos reproduzem os nomes conferidos: `quit` (`main.gd:30-31`), `replace_main_scene`
  com 1 parâmetro (`main.gd:32-33`, `menu.gd:163`), 9 handlers/10 conexões (`menu.tscn:836-845`),
  3 métodos do `Main` (um deles alvo de `call_deferred` por nome), 85 caminhos do menu derivados
  mecanicamente de `menu.gd:11-104`. ✅
- Todo o mapeamento compilou com a feature ativa e 0 warnings; probes confirmaram base do
  forklift, sinais (aridade e tipo do argumento) conectáveis por nome a partir de GDScript,
  métodos expostos/privados e os inteiros dos enums do engine. ✅
- Baseline medida inclui um erro **intermitente do engine** em `main.tscn` headless (research
  §E.2) — a regra de re-execução está no quickstart e o catálogo do `CLAUDE.md` é atualizado no
  port 3 (Princípio II, erros pré-existentes catalogados). ✅
- Decisões que se afastam do texto do input do comando, impostas pelo compilador ou pela
  fidelidade: `GString::from("…")` nas comparações de string (D3); `pub(crate)` também em
  `EnemyRobot::exploded` (D2); `EnvironmentSdfgiRayCount::COUNT_96/32` (sem `RAY_`) (D5);
  `lightmap_gi` mantido `Some` após `queue_free` (D5); `set_name(&id.to_string())` (D6);
  `progress.at(0)` de um `VarArray` (D8); `ResourceLoader::singleton().load(..)` no `Main`
  como o original, não `load()` global (D10). ✅

**Resultado do gate (pós-Phase 1)**: PASS.
