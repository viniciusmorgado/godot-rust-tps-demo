# Implementation Plan: Marco B — jogador, bala e porta (v1 raw port)

**Branch**: `main` (a v1 vive na `main`; cada port é um commit atômico que deixa o jogo jogável) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-v1-player-bullet-door/spec.md`

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.0). Tradução direta; nenhuma abstração,
refatoração ou otimização. Usa pela primeira vez a cláusula de **correção conservadora de bug do
upstream** — uma única correção, na porta.

## Summary

Portar `player.gd` (211 linhas, `class_name Player`), `bullet.gd` (51) e `door.gd` (12) para três
classes gdext 0.5.5 com a mesma base (`CharacterBody3D`, `CharacterBody3D`, `Area3D`), trocando o
`type` do node raiz em `player.tscn`, `bullet.tscn` e `door.tscn` e apagando `.gd` + `.gd.uid` no
mesmo commit — na ordem player → bullet → door, um commit por script. Os 7 scripts restantes
ficam byte a byte intactos e continuam encontrando `Player`, `player_id`, `hit`,
`add_camera_shake_trauma` pelos nomes originais. Abordagem técnica: o `Player` consome
`PlayerInputSynchronizer` e `CameraNoiseShake` (Marco A) por acesso **tipado**
(`OnReady<Gd<PlayerInputSynchronizer>>`, `bind()`/`bind_mut()`, `cast::<CameraNoiseShake>()`), o
que exige apenas tornar `pub(crate)` os campos/métodos consumidos; `player_id` usa
`#[var(set = set_player_id)]` com setter que funciona fora da árvore; `current_animation` é um
enum derivado (`GodotConvert, Var, Export`, `via = i64`); `motion` é `#[var]` para continuar
replicável por nome; a bala preserva o duck typing `has_method("hit")` + `rpc` e faz o primeiro
uso da exceção `Settings` (`/root/Settings` → `config_file` tipado como `ConfigFile`); a porta
referencia `DoorModel2/AnimationPlayer` com o comentário `// upstream bug fix` no ponto exato e
cria `docs/upstream-bugs.md`. Todas as assinaturas foram confirmadas por compilação de um módulo
de rascunho (0 warnings) e por um probe headless — ver [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), já em
`[workspace.dependencies]` — NÃO alterar versão nem features.

**Primary Dependencies**: gdext 0.5.5 (API prebuilt 4.6 — manter); Godot 4.7.2 stable em
`/usr/bin/godot.x86_64` (não há `godot` no PATH). `.gdextension` com `reloadable = true`, lib
debug em `oxide_godot_core/target/debug/liboxide_godot.so`. Bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` — em 2026-09-15 é o único
diretório `godot-core-*/out`; se houver mais de um, o válido é `ls -dt .../godot-core-*/out | head -1`.

**Storage**: N/A

**Testing**: `cargo build` (perfil debug, 0 warnings) + Godot headless (import + execução de cena).
Sem testes unitários Rust nesta fase (infraestrutura nova — v2). Validação visual pelo usuário.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + projeto Godot
(`oxide-godot/`)

**Performance Goals**: paridade com o original (não otimizar — Princípio I)

**Constraints**: Princípios I e II da constituição v1.3.0; nomes de métodos/propriedades/RPCs
idênticos ao GDScript; um commit por script; `.gd` + `.gd.uid` apagados no mesmo commit;
melhorias só em `docs/v2-backlog.md`; exatamente UMA correção de bug (porta) com os 4 requisitos
da cláusula.

**Scale/Scope**: 3 scripts, 274 linhas de GDScript, 3 cenas editadas (uma vez cada), 3 commits,
+2 arquivos Rust existentes tocados só em visibilidade, 1 documento novo (`docs/upstream-bugs.md`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Princípio I — Porte em Três Fases

| Regra | Status | Evidência |
|---|---|---|
| Fase declarada em spec/plan/tasks | ✅ | spec.md "Fase: v1"; este plan "Fase: v1" |
| Tradução direta, sem remodelar nodes/cenas | ✅ | As 3 cenas só recebem troca de `type` e remoção de `script`/`ext_resource`; `DoorModel2` **não** é renomeado |
| Nenhuma abstração/refatoração/otimização | ✅ | Um módulo por script, sem módulo comum/trait/helper. Quirks preservados (spec Assumptions; research D16): `airborne_time = 100`; `jumping` zerado pelo jogador; `explode` duplo possível; `velocity` não zerada no respawn; `crosshair` nunca usado; `preload` → `load` no ponto de uso |
| Melhorias → `docs/v2-backlog.md` no mesmo commit | ✅ | Candidatos 10–14 em research.md §"Backlog v2 candidato", atribuídos por script; itens 1 e 2 já existentes não são duplicados |
| **Correção de bug: é bug, não melhoria** | ✅ | `door.gd:6` referencia `DoorModel/AnimationPlayer`; a cena tem `DoorModel2` (`door.tscn:13`); resultado: `ERROR: Node not found` e porta que nunca abre — intenção inequívoca contradita pelo resultado. Reproduzido headless (research §E.2: exatamente 1 erro) |
| Correção **conservadora** | ✅ | Só o caminho do node muda (`DoorModel2/AnimationPlayer`); nada renomeado, extraído ou "aproveitado" (FR-031) |
| Requisito (a) — declarada na spec | ✅ | spec.md US3 + FR-030–FR-035 |
| Requisito (b) — isolada com `// upstream bug fix: ...` no ponto exato | ✅ planejado | research D15: comentário na linha acima do `#[init(node = "DoorModel2/AnimationPlayer")]` em `src/door.rs` |
| Requisito (c) — mencionada no commit | ✅ planejado | quickstart.md §8, formato obrigatório do commit do port 3 |
| Requisito (d) — `docs/upstream-bugs.md` | ✅ planejado | Criado no commit do port 3 com cabeçalho + entrada #1 (defeito, script/cena, correção, commit) — data-model.md §"Registro de bugs" |
| Nenhuma outra correção | ✅ | FR-035; todos os demais quirks ficam (research D16) |

### Princípio II — Ciclo de Porte Verificável

| Regra | Status | Evidência |
|---|---|---|
| Uma classe por script, mesma base | ✅ | `Player: CharacterBody3D`, `Bullet: CharacterBody3D`, `Door: Area3D` |
| Vínculo por troca de `type` na `.tscn`; nenhum `.gd` ponte | ✅ | Tabela "Edição das cenas" abaixo, linhas conferidas em 2026-09-15 |
| Nomes de `#[func]`/RPC idênticos | ✅ | `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma`, `explode`, `destroy`, `_on_door_body_entered` — [contracts/](contracts/) |
| Nomes de propriedades exportadas/replicadas idênticos (conferidos na `.tscn`) | ✅ | `player.tscn:17-30` replica `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`, `.:current_animation` → `player_id` (`#[export]`), `current_animation` (`#[export]`), `motion` (`#[var]`) — research D2–D4, probe §E.1; `bullet.tscn:12` `.:global_transform` (base) |
| `cargo build` sem warnings novos | ✅ | quickstart.md §1; baseline 0; rascunho de research compilou com 0 |
| Validação headless (import + cena) | ✅ | quickstart.md §2–3; baseline = os 3 do `CLAUDE.md`; porta: `Node not found` 1 → 0 |
| Commit por port com script + cena na mensagem | ✅ | quickstart.md §8 |
| `.gd` + `.gd.uid` apagados no mesmo commit | ✅ | Tabela "Edição das cenas"; `grep` do uid após remoção |
| Ordem de baixo para cima | ✅ | `docs/port-order.md` itens 6→7→8. Player primeiro: consome só classes já em Rust (`PlayerInputSynchronizer`, `CameraNoiseShake`); instancia a bala por **API base** (`CharacterBody3D`), logo não depende dela. Bullet depois: chama `hit` por duck typing (permitido). Door por último: `is Player` exige `Player` em Rust |
| Rust não chama API customizada de GDScript | ✅ | Únicas chamadas dinâmicas: `has_method("hit")` + `rpc("hit")` na bala (duck typing do original — permitido), `AnimationTree.set("parameters/...")` (API base de `Object`), e a exceção `Settings` |
| Exceção `Settings` | ✅ | Bullet `explode`: `get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>()` (research D14); backlog v2 item 1 já existe |
| Catálogo do `CLAUDE.md` reflete a baseline | ✅ | O `Node not found` da porta **nunca esteve** no catálogo (`CLAUDE.md:39-40` só lista os 3 erros de import) → nada a remover; constatação registrada na mensagem do commit do port 3 (quickstart §8) |

**Visibilidade `pub(crate)` em `player_input.rs`/`camera_noise_shake.rs`**: não é abstração nem
refatoração — nenhuma linha se move, nenhum corpo muda; é o mínimo que Rust exige para o acesso
tipado obrigatório por FR-010/FR-011. Registrado em Complexity Tracking como não-violação
justificada.

**Resultado do gate (pré-Phase 0)**: PASS — nenhuma violação.

## Project Structure

### Documentation (this feature)

```text
specs/002-v1-player-bullet-door/
├── plan.md              # Este arquivo
├── spec.md              # Especificação (já validada, commit 108584e)
├── research.md          # Phase 0: assinaturas gdext 0.5.5 confirmadas por compilação + probe headless
├── data-model.md        # Phase 1: estado de Player (contrato + interno), Bullet, Door, registro de bugs
├── quickstart.md        # Phase 1: comandos de validação na ordem, baseline da porta, formato dos commits
├── contracts/
│   ├── player.md        # nome de classe, player_id/current_animation/motion, 5 RPCs, consumidores
│   ├── bullet.md        # explode, destroy (method track), global_transform
│   └── door.md          # _on_door_body_entered, connection, referência corrigida
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado por este comando)
```

### Source Code (repository root)

```text
oxide_godot_core/                       # workspace Cargo (inalterado)
└── oxide_godot_lib/
    ├── Cargo.toml                      # inalterado
    └── src/
        ├── lib.rs                      # ExtensionLibrary + `mod` de cada módulo (+3 linhas `mod`)
        ├── debug_label.rs              # Marco A (inalterado)
        ├── part_disappear.rs           # Marco A (inalterado)
        ├── blast.rs                    # Marco A (inalterado)
        ├── camera_noise_shake.rs       # Marco A — port 1: `add_trauma` → pub(crate) (só visibilidade)
        ├── player_input.rs             # Marco A — port 1: 6 campos + 3 métodos → pub(crate) (só visibilidade)
        ├── player.rs                   # struct Player, base=CharacterBody3D  (port 1)  NOVO
        ├── bullet.rs                   # struct Bullet, base=CharacterBody3D  (port 2)  NOVO
        └── door.rs                     # struct Door,   base=Area3D           (port 3)  NOVO

oxide-godot/                            # projeto Godot
├── player/
│   ├── player.tscn                     # port 1: raiz Player → type="Player"
│   ├── player.gd (+ .uid)              # port 1: APAGAR
│   └── bullet/
│       ├── bullet.tscn                 # port 2: raiz Bullet → type="Bullet"
│       └── bullet.gd (+ .uid)          # port 2: APAGAR
└── door/
    ├── door.tscn                       # port 3: raiz Door → type="Door" (DoorModel2 intocado)
    └── door.gd (+ .uid)                # port 3: APAGAR

docs/
├── v2-backlog.md                       # itens 10–14, um commit por origem
└── upstream-bugs.md                    # NOVO no port 3: cabeçalho + entrada #1 (porta)
```

**Structure Decision**: um módulo Rust por script GDScript, sem módulo compartilhado e sem
helpers comuns (Princípio I). `lib.rs` só ganha três `mod`. O nome `Player` é obrigatório
(`is Player` em `red_robot.gd:131,275,281` e `door.gd:10`); `Bullet` e `Door` são livres e não
colidem com classes do engine (conferido nas bindings). Os módulos do Marco A consumidos pelo
Player mudam **apenas** a palavra de visibilidade nos itens listados em research.md D1, no commit
do port 1.

Detalhes de cada classe (campos, virtuais, RPCs, assinaturas) estão em [research.md](research.md)
§"Mapa por script" e D1–D15; não são repetidos aqui.

## Edição das cenas (linhas conferidas em 2026-09-15 — reconferir com `grep -n` antes de editar)

| Port | Cena | Node (linha) | `type` antes → depois | Remover | Manter | Apagar |
|---|---|---|---|---|---|---|
| 1 | `player/player.tscn` | raiz `Player` (l.333) | `CharacterBody3D` → `Player` | l.336 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://ctlx3bqglonsx" path="res://player/player.gd" id="1"]` | `collision_layer`/`collision_mask` (l.334–335); `ServerSynchronizer` (l.338–339, `replication_config`); `InputSynchronizer` (l.341–348); `BulletCache` (l.679–683) e `[editable path="BulletCache"]` | `player/player.gd`, `player/player.gd.uid` |
| 2 | `player/bullet/bullet.tscn` | raiz `Bullet` (l.481) | `CharacterBody3D` → `Bullet` | l.485 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://iybteh2g0be4" path="res://player/bullet/bullet.gd" id="1"]` | `transform`/`collision_layer`/`collision_mask` (l.482–484); `MultiplayerSynchronizer` (l.487–488); method track `destroy` (l.93–105) | `player/bullet/bullet.gd`, `.uid` |
| 3 | `door/door.tscn` | raiz `Door` (l.10) | `Area3D` → `Door` | l.11 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://7v3r683kok5s" path="res://door/door.gd" id="1"]` | `DoorModel2` (l.13) — **NÃO renomear**; `AnimationPlayer` (l.25); `[connection ...]` (l.37); `[editable path="DoorModel2"]` (l.39) | `door/door.gd`, `door/door.gd.uid` |

Após remover o `ext_resource` da l.3, as linhas seguintes deslocam −1 (a raiz do `player.tscn`
passa a l.332, etc.). Depois de cada remoção: `grep -rn "<uid>" oxide-godot/` deve retornar vazio
(pré-conferido: cada uid é referenciado por exatamente uma cena). Nos 3 ports o id do script é
`"1"`, e nenhum outro `ExtResource("1")` existe nessas cenas além da linha `script`.

Observação: o Godot grava `type="<nome da classe registrada>"`; ao abrir a cena no editor depois
do port, o editor pode reescrever a `.tscn` (ordem de propriedades, `unique_id`) — diffs assim
são aceitáveis desde que o `type` e as propriedades permaneçam.

## Validação visual por script (SC-002/SC-009 — feita pelo usuário no editor/jogo)

| Port | O que conferir no jogo (comparar com `../oxide_godot_origins/`) |
|---|---|
| 1 | Entrar no level: spawn com **som de pouso** imediato (quirk `airborne_time = 100`); andar em todas as direções com a orientação do modelo seguindo a câmera; correr/parar com blend suave; pular (som Jump, animação subindo/descendo) e pousar após > 0,5 s no ar (som Land); mirar (strafe lateral, mira acompanha o pitch da câmera) e atirar (partículas do cano, som, tremor leve, cooldown 0,4 s); ser atingido pelo laser do robô (tremor forte 13,0); cair do mapa → reaparece na posição inicial. A bala neste ponto ainda é GDScript |
| 2 | Atirar: bala azul visível sai do cano, voa reto, explode ao acertar parede/chão (animação + luz) e ao acertar o robô (robô reage ao `hit`); bala perdida explode sozinha após 5 s; a bala some após a explosão; com `Shadow mapping` ligado nas configurações, a luz da explosão projeta sombra; `BulletCache` invisível no spawn |
| 3 | Nada visível no jogo (asset órfão). Abrir `door/door.tscn` no editor: raiz do tipo `Door`, sem script, `DoorModel2` intacto, sem erro no Output. Opcional (SC-009): cena de teste **não commitada** com `door.tscn` + `player.tscn`; ao andar até a porta, a animação `doorsimple_opening` toca uma vez |

## Complexity Tracking

> Preenchido para registrar uma **não-violação justificada** (o gate não tem violações).

| Item | Por que é necessário | Alternativa mais simples rejeitada porque |
|---|---|---|
| `pub(crate)` em 6 campos + 3 métodos de `player_input.rs` e em `add_trauma` de `camera_noise_shake.rs` (commit do port 1) | FR-010/FR-011 exigem acesso **tipado** do `Player` a `PlayerInputSynchronizer` e `CameraNoiseShake`; em Rust, itens sem `pub` são privados ao módulo. Só a palavra de visibilidade muda — nenhuma lógica movida, extraída ou reescrita (Princípio I intacto) | Acesso via `Variant` (`.get("motion")`, `.call("add_trauma")`) — viola FR-011 e a regra do Princípio II contra acesso dinâmico entre classes já em Rust; mover as classes para um módulo só — refatoração desnecessária de código já commitado |

## Constitution Check — re-avaliação pós-design (Phase 1)

Re-avaliado após research.md, data-model.md, contracts/ e quickstart.md:

- Nenhum artefato introduz módulo comum, trait, helper ou enum além do `Animations` que o
  original já tem (Princípio I). ✅
- O contrato de `Player` reproduz o nome de classe, as 3 propriedades (`player_id`,
  `current_animation`, `motion`) e os 5 RPCs, conferidos contra `player.gd`, `player.tscn:17-30`,
  `red_robot.gd:131,133,275,281`, `level.gd:118-119`, `bullet.gd:31-32`, `door.gd:10`. ✅
- Os contratos de `Bullet` (`explode`, `destroy` — `bullet.tscn:104`; `.:global_transform`) e
  `Door` (`_on_door_body_entered` — `door.tscn:37`) reproduzem os nomes das cenas. ✅
- A correção do bug está confinada a uma linha de `src/door.rs` (+ comentário), ao commit do port
  3 e a `docs/upstream-bugs.md`; `door.tscn` só troca `type`/remove script (4/4 requisitos). ✅
- Todas as assinaturas do mapeamento compilaram com 0 warnings e o probe headless confirmou
  setter fora da árvore, `#[var] motion` por nome e enum com default 3 (research §E). ✅
- Decisões que se afastam do texto do input do comando, todas em favor da fidelidade ou por
  exigência do gdext: `col_c()`/`col_a()` para `basis.z`/`basis.x` (D9); intermediário
  `sound_effects` omitido em favor dos caminhos completos (D5); `try_cast` direto no `body`
  sem `clone()` (D15); `0.to_variant()` inteiro em `aim/add_amount` (D8). ✅

**Resultado do gate (pós-Phase 1)**: PASS.
