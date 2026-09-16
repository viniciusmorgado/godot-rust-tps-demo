# Implementation Plan: Marco C — inimigo: peça e robô vermelho (v1 raw port)

**Branch**: `main` (a v1 vive na `main`; cada port é um commit atômico que deixa o jogo jogável) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-v1-enemy/spec.md`

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.0). Tradução direta; nenhuma abstração,
refatoração ou otimização. Nenhuma correção de bug prevista — `docs/upstream-bugs.md` permanece
com 1 entrada.

## Summary

Portar `part.gd` (57 linhas, `RigidBody3D`, sem cena própria — 3 nodes de `red_robot.tscn`) e
`red_robot.gd` (283 linhas, `CharacterBody3D`, raiz de `red_robot.tscn`) para duas classes gdext
0.5.5, trocando o `type` dos 3 nodes de peça (port 1) e da raiz (port 2) na mesma cena de 11.053
linhas, apagando `.gd` + `.gd.uid` no mesmo commit — na ordem part → red_robot, um commit por
script. Os 5 scripts restantes ficam intactos e continuam encontrando `exploded` (`level.gd:99`)
e `hit` (bala). Abordagem técnica: `fade_value` com `#[var(set = set_fade_value)]` aplicando ao
shader do `next_pass` (setter acionado também pelo `set_indexed` da replicação — confirmado);
duplicação de material por `Gd::duplicate_resource()` (o `duplicate()` gerado está deprecado);
`await` → `create_timer(..).signals().timeout().connect_other(..)`; `#[signal] exploded` no bloco
`#[godot_api]` principal, emitido por `signals().exploded().emit()` e conectado por nome pelo
`level.gd`; enum `State` derivado (`via = i64`); a transformação inversa `Vector3 * Transform3D`
traduzida como `basis.transposed() * (v − origin)` (a `affine_inverse` diverge com escala —
confirmado numericamente); raycasts com exclusão **efetiva** pelo RID do robô; `collider ==
player` por `instance_id()`; acesso tipado a `Part::explode` (`pub(crate)` desde o port 1) e a
`Player::add_camera_shake_trauma` (`pub(crate)` no commit do robô); `Blast` e `PartDisappear`
instanciados por API base. Tudo compilado num rascunho (0 warnings) e exercitado em headless —
[research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), já em
`[workspace.dependencies]` — NÃO alterar versão nem features.

**Primary Dependencies**: gdext 0.5.5 (API prebuilt 4.6 — manter); Godot 4.7.2 stable em
`/usr/bin/godot.x86_64`. `.gdextension` com `reloadable = true`, lib debug em
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (único diretório em
2026-09-15; se houver mais de um, `ls -dt .../godot-core-*/out | head -1`).

**Storage**: N/A

**Testing**: `cargo build` (perfil debug, 0 warnings) + Godot headless (import + `red_robot.tscn`
+ `level.tscn`). Sem testes unitários Rust nesta fase. Validação visual pelo usuário.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + projeto Godot
(`oxide-godot/`)

**Performance Goals**: paridade com o original (não otimizar — Princípio I)

**Constraints**: Princípios I e II da constituição v1.3.0; nomes de métodos/propriedades/RPCs/sinal
idênticos ao GDScript; um commit por script; `.gd` + `.gd.uid` apagados no mesmo commit;
melhorias só em `docs/v2-backlog.md`; nenhuma correção de bug (se surgir defeito objetivo, parar e
declarar em spec antes de qualquer commit).

**Scale/Scope**: 2 scripts, 340 linhas de GDScript, 1 cena editada duas vezes (4 nodes), 2
commits, +1 arquivo Rust existente tocado só em visibilidade (`player.rs`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Princípio I — Porte em Três Fases

| Regra | Status | Evidência |
|---|---|---|
| Fase declarada em spec/plan/tasks | ✅ | spec.md "Fase: v1"; este plan "Fase: v1" |
| Tradução direta, sem remodelar nodes/cenas | ✅ | `red_robot.tscn` só recebe troca de `type` em 4 nodes e remoção de `script`/`ext_resource`; nenhum node renomeado ou movido |
| Nenhuma abstração/refatoração/otimização | ✅ | Um módulo por script; os 3 raycasts repetidos do original ficam **inline** (research D12 — helper descartado por ser extração); quirks preservados (research D15): `body.name == "Target"`, `pass # Kill.`, `player: Node3D`, `await` 10 s no `hit`, puff no pai da peça, exports não replicados |
| Melhorias → `docs/v2-backlog.md` no mesmo commit | ✅ | Candidatos 15–18 em research.md §"Backlog v2 candidato", atribuídos por script |
| Correção de bugs | N/A | Nenhum defeito objetivo conhecido; `docs/upstream-bugs.md` fica com 1 entrada. Se um surgir: parar, declarar na spec (requisito a), e só então comentário/commit/registro (b–d) |

### Princípio II — Ciclo de Porte Verificável

| Regra | Status | Evidência |
|---|---|---|
| Uma classe por script, mesma base | ✅ | `Part: RigidBody3D`, `EnemyRobot: CharacterBody3D` |
| Vínculo por troca de `type` na `.tscn`; nenhum `.gd` ponte | ✅ | Tabela "Edição das cenas" abaixo (port 1: 3 nodes + `ext_resource id="24"`; port 2: raiz + `id="1"`) |
| Nomes de `#[func]`/RPC/sinal idênticos | ✅ | `explode`, `destroy`; `exploded`, `hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered`, `_on_area_body_exited` — [contracts/](contracts/); probe §E.2 confirma `has_method`/`has_signal` |
| Nomes de propriedades exportadas/replicadas idênticos (conferidos na `.tscn`) | ✅ | Peça: `red_robot.tscn:10419` `.:fade_value` → `#[export] #[var(set)]` (setter acionado por `set_indexed`, §E.1). Robô: l.33/36/39/42 `health`/`state`/`target_position`/`dead` → `#[export]`; `aim_preparing`/`test_shoot` exportados e não replicados, como no original |
| `cargo build` sem warnings novos | ✅ | Baseline 0; rascunho compilou com 0 (após trocar `duplicate()` deprecado por `duplicate_resource()`) |
| Validação headless (import + cena) | ✅ | quickstart.md §2–3; baseline medida em `4bb8f7f` (§E.3) |
| Commit por port com script + cena na mensagem | ✅ | quickstart.md §8 |
| `.gd` + `.gd.uid` apagados no mesmo commit | ✅ | Tabela "Edição das cenas"; `grep` do uid após remoção |
| Ordem de baixo para cima | ✅ | `docs/port-order.md` itens 9 → 10. Peça primeiro: consome só `PartDisappear` (Marco A) por API base. Robô depois: consome `Part` (tipado), `Player` (tipado), `Blast` (API base) — todos já em Rust |
| Rust não chama API customizada de GDScript | ✅ | O robô não consome nenhum script GDScript remanescente; únicas chamadas dinâmicas são `AnimationTree.set/get("parameters/…")`, `col.get("position"/"collider")` (Dictionary do raycast) — API base. Nenhum `.call(` (verificação final no quickstart) |
| Exceção `Settings` | N/A | Não usado por nenhum dos dois scripts |
| Catálogo do `CLAUDE.md` reflete a baseline | ✅ | Nenhum erro eliminado; `CLAUDE.md` intocado (verificação final) |

**Visibilidade `pub(crate)`**: `Part::explode` (`#[func] pub(crate)`, port 1) e
`Player::add_camera_shake_trauma` (port 2, só a palavra de visibilidade em `player.rs`) —
não-violação justificada, precedente do Marco B; ver Complexity Tracking.

**Resultado do gate (pré-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/003-v1-enemy/
├── plan.md              # Este arquivo
├── spec.md              # Especificação (commit 4bb8f7f)
├── research.md          # Phase 0: assinaturas confirmadas por compilação + probes headless + baseline
├── data-model.md        # Phase 1: Part e EnemyRobot (contrato, interno, máquina de estados)
├── quickstart.md        # Phase 1: comandos de validação, baseline, formato dos commits
├── contracts/
│   ├── part.md          # explode, destroy, 4 exports; consumidor red_robot.gd:96-98; replicação l.10419
│   └── red-robot.md     # exploded, hit, play_shoot, shoot_check, resume_approach, _on_area_body_*, 6 exports
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado por este comando)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── lib.rs                      # +2 linhas `mod`
├── debug_label.rs, part_disappear.rs, blast.rs, camera_noise_shake.rs, player_input.rs   # inalterados
├── player.rs                   # port 2: `add_camera_shake_trauma` → pub(crate) (só visibilidade)
├── bullet.rs, door.rs          # inalterados
├── part.rs                     # struct Part,     base=RigidBody3D      (port 1)  NOVO
└── red_robot.rs                # struct EnemyRobot, base=CharacterBody3D (port 2)  NOVO

oxide-godot/enemies/red_robot/
├── red_robot.tscn              # port 1: Death/PartShield1|2, Death/PartHead → type="Part"; port 2: raiz → type="EnemyRobot"
├── red_robot.gd (+ .uid)       # port 2: APAGAR
└── parts/part.gd (+ .uid)      # port 1: APAGAR

docs/v2-backlog.md              # itens 15 (port 1), 16–18 (port 2)
```

**Structure Decision**: um módulo Rust por script, sem módulo compartilhado (Princípio I).
`Part` e `EnemyRobot` são nomes livres (sem `class_name`) — `RedRobot` foi descartado em T024 por colidir com `const RedRobot` em `level.gd:6` ("shadows a native class"; ver research D1), conferidos sem colisão nas bindings.
`Part::explode` nasce `#[func] pub(crate)` no port 1 para que o commit do robô não toque em
`part.rs`. Detalhes de cada classe em [research.md](research.md) §"Mapa por script".

## Edição das cenas (`enemies/red_robot/red_robot.tscn`, 11.053 linhas; linhas conferidas em 2026-09-15 — reconferir com `grep -n` antes de editar)

| Port | Node (linha) | `type` antes → depois | Remover | Manter | Apagar |
|---|---|---|---|---|---|
| 1 | `Death/PartShield1` (l.10833) | `RigidBody3D` → `Part` | l.10841 `script = ExtResource("24")` | l.10834–10840: `transform`, `collision_layer = 3`, `collision_mask = 3`, `mass = 2000.0`, `physics_material_override`, `freeze = true`, `angular_damp = 0.3`; filhos l.10843+ (`MultiplayerSynchronizer` com `replication_config` + `public_visibility = false`, `Model`, `Col1`, `Col2`) | — |
| 1 | `Death/PartShield2` (l.10885) | `RigidBody3D` → `Part` | l.10892 `script = ExtResource("24")` | idem (l.10886–10891; filhos l.10894+) | — |
| 1 | `Death/PartHead` (l.10936) | `RigidBody3D` → `Part` | l.10944 `script = ExtResource("24")` | idem (l.10937–10943; filhos l.10946+) | — |
| 1 | — | — | l.26 `[ext_resource type="Script" uid="uid://c3vo80hyj6w6c" path="res://enemies/red_robot/parts/part.gd" id="24"]` | todos os outros `ext_resource` | `parts/part.gd`, `parts/part.gd.uid` |
| 2 | raiz `RedRobot` (l.10584 → **10583** após o port 1: só −1 do `ext_resource` l.26 — as 3 linhas `script` removidas ficam abaixo da raiz e não a deslocam) | `CharacterBody3D` → `EnemyRobot` | `script = ExtResource("1")` (l.10587 → 10583); l.3 `[ext_resource type="Script" uid="uid://bf14mo0lrrvjl" path="res://enemies/red_robot/red_robot.gd" id="1"]` | `collision_layer/mask = 3`; `MultiplayerSynchronizer` (`replication_config`); `AnimationTree`; `ShootAnimation` (method tracks); `PlayerDetectionArea`; as 2 `[connection …]` (fim do arquivo) | `red_robot.gd`, `red_robot.gd.uid` |

Após o port 1, **todas** as linhas ≥ 26 deslocam −1 e as ≥ 10841 deslocam até −4; após o port 2,
as ≥ 3 deslocam −1 de novo. Editar por `sed` só depois de `grep -n` na mesma sessão. Conferido:
`ExtResource("24")` ocorre exatamente 3 vezes e `ExtResource("1")` exatamente 1 vez na cena
(nenhum outro recurso usa esses ids). Depois de cada remoção: `grep -rn "<uid>" oxide-godot/ | grep -v /.godot/`
deve retornar vazio.

## Validação visual por script (SC-002 — feita pelo usuário no editor/jogo)

| Port | O que conferir no jogo (comparar com `../oxide_godot_origins/`) |
|---|---|
| 1 | Matar um robô (5 tiros): os dois escudos e a cabeça se soltam para cima com rotação aleatória, caem e quicam com física, ficam de 3 a 6 s no chão, somem num fade (~0,3 s, com o brilho de `emission_cutout`) e terminam com o puff (Marco A). Cada peça some no seu próprio tempo, sem afetar as outras. No editor: os 3 nodes com tipo `Part`, sem script, `freeze` marcado |
| 2 | Robô parado ao longe (IDLE, animação idle); ao aproximar, vira (`turn_left/right`) e anda (`walk`) até ficar de frente; ~6 s de frente → mira (laser vermelho aparece e é clipado no cenário/jogador, animação de mira acompanha o jogador); ~1 s → atira (animação "shoot", faíscas do laser, impacto no ponto, **tremor forte** se acertar); volta a aproximar. Cada tiro recebido: animação de dano (uma de três) + som; 5º tiro: morte como no port 1 + faíscas + som de explosão; 10 s depois o robô some; 15 s após a morte, outro robô nasce no mesmo ponto (level). Sair da área de detecção → robô volta a IDLE |

## Complexity Tracking

> Preenchido para registrar **não-violações justificadas** (o gate não tem violações).

| Item | Por que é necessário | Alternativa mais simples rejeitada porque |
|---|---|---|
| `Part::explode` como `#[func] pub(crate)` (port 1) | `#[func]` porque `red_robot.gd` chama por nome até o port 2 (Princípio II, nomes preservados); `pub(crate)` porque FR-018 exige acesso tipado do robô no port 2 — declarado já no port 1 para que o commit do robô não edite `part.rs` | Só `#[func]` e `bind_mut().call("explode")` no port 2 — acesso dinâmico entre classes Rust (proibido) |
| `Player::add_camera_shake_trauma` → `pub(crate)` em `player.rs` (commit do robô) | FR-018: `player.bind_mut().add_camera_shake_trauma(13.0)` tipado após `try_cast::<Player>`; só a palavra de visibilidade muda (precedente Marco B) | `player.call("add_camera_shake_trauma", …)` ou `rpc` — dinâmico; o original chama diretamente |

## Constitution Check — re-avaliação pós-design (Phase 1)

Re-avaliado após research.md, data-model.md, contracts/ e quickstart.md:

- Nenhum artefato introduz módulo comum, trait ou helper: os raycasts repetidos ficam inline
  (research D12 descartou o helper como refatoração). ✅
- Os contratos reproduzem todos os nomes conferidos na cena e nos consumidores: peça
  (`explode` — `red_robot.gd:96-98`; `fade_value` — `red_robot.tscn:10419`), robô (`exploded` —
  `level.gd:99`; `hit` — `bullet.rs`; method tracks l.10296-10299; conexões l.11050-11051;
  replicação l.30-42). ✅
- Todas as assinaturas compilaram com 0 warnings; probes confirmaram setter por `set_indexed`,
  duplicação de material/`next_pass`, sinal por nome, exports/defaults, e a identidade
  `v * t = transposed * (v − origin)`. ✅
- Decisões que se afastam do texto do input do comando, impostas pelo compilador ou pela
  fidelidade: `duplicate_resource()` em vez de `duplicate()` (deprecado, warning); `create_timer`
  sem `unwrap` (retorna `Gd` direto); `VarDictionary` para o resultado do raycast; `StringName::from("Target")`
  na comparação de nome; raycasts inline em vez de helpers. ✅
- Nenhum bug corrigido; `docs/upstream-bugs.md` e `CLAUDE.md` intocados. ✅

**Resultado do gate (pós-Phase 1)**: PASS.
