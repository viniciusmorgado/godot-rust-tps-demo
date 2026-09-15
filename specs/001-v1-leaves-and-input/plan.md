# Implementation Plan: Marco A — folhas e input do jogador (v1 raw port)

**Branch**: `main` (a v1 vive na `main`; cada port é um commit atômico que deixa o jogo jogável) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-v1-leaves-and-input/spec.md`

**Fase**: v1 — Raw Port (Princípio I). Tradução direta; nenhuma abstração, refatoração ou otimização.

## Summary

Portar os 5 scripts folha do TPS demo (`debug.gd`, `part_disappear.gd`, `blast.gd`,
`camera_noise_shake_effect.gd`, `player_input.gd`; 242 linhas de GDScript) para 5 classes gdext
0.5.5, uma por script e com a mesma classe base, trocando o `type` do node nas cenas
(`level.tscn`, `part_disappear.tscn`, `impact_effect.tscn`, `player.tscn` ×2) e apagando o `.gd` +
`.gd.uid` no mesmo commit. Os 10 scripts restantes ficam intactos e continuam consumindo a API
portada por nome (`add_trauma`, `PlayerInputSynchronizer.*`). Cada port segue o ciclo do
Princípio II: `cargo build` sem warnings novos → import headless com `Initialize godot-rust` →
execução headless da cena afetada sem erros novos → commit próprio. Abordagem técnica: `await`
vira conexão a sinal tipado (`connect_other` com callable *linked* ao node, invalidada
automaticamente se o node for liberado), `@onready` vira `OnReady` com `#[init(node = ...)]`,
`@export` de node vira `#[export] Option<Gd<T>>`, `@rpc("call_local")` vira
`#[rpc(authority, call_local, unreliable)]` (os defaults do GDScript). Todas as assinaturas foram
conferidas contra o crate e as bindings geradas em `target/debug/build/godot-core-*/out/`
(ver [research.md](research.md)).

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), já em
`[workspace.dependencies]` — NÃO alterar versão nem features.

**Primary Dependencies**: gdext 0.5.5 (API prebuilt padrão 4.6 — confirmado em
`godot-bindings-0.5.5/src/import.rs:72`, manter); Godot 4.7.2 stable em `/usr/bin/godot.x86_64`
(não há `godot` no PATH). `.gdextension` com `reloadable = true`, lib debug em
`oxide_godot_core/target/debug/liboxide_godot.so`.

**Storage**: N/A

**Testing**: `cargo build` (perfil debug) + Godot headless (import + execução de cena). Não há
testes unitários Rust nesta fase (seriam infraestrutura nova — v2). Validação visual pelo usuário.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + projeto Godot
(`oxide-godot/`)

**Performance Goals**: paridade com o original (não otimizar — Princípio I)

**Constraints**: Princípios I e II da constituição v1.2.0; nomes de métodos/propriedades
idênticos ao GDScript; um commit por script; `.gd` + `.gd.uid` apagados no mesmo commit;
melhorias só em `docs/v2-backlog.md`.

**Scale/Scope**: 5 scripts, 242 linhas de GDScript, 4 cenas editadas (uma delas duas vezes),
5 commits.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Princípio I — Porte em Três Fases

| Regra | Status | Evidência |
|---|---|---|
| Fase declarada em spec/plan/tasks | ✅ | spec.md "Fase: v1"; este plan "Fase: v1" |
| Tradução direta, sem remodelar nodes/cenas | ✅ | Estrutura das 4 cenas intocada além da troca de `type` e remoção de `script`/`ext_resource` |
| Nenhuma abstração/refatoração/otimização | ✅ | Um módulo por script, sem módulo comum, sem helpers compartilhados, sem traits (ver Structure Decision). Quirks preservados: texto do overlay recalculado oculto; `start_rotation` capturada uma vez; raycast sem exclusão efetiva (FR-017); `jumping` exportado mas não replicado |
| Melhorias → `docs/v2-backlog.md` no mesmo commit | ✅ | Candidatos já identificados em research.md §"Backlog v2 candidato"; cada um entra no commit do script onde foi percebido |

### Princípio II — Ciclo de Porte Verificável

| Regra | Status | Evidência |
|---|---|---|
| Uma classe por script, mesma base | ✅ | `DebugLabel: Label`, `PartDisappear: CpuParticles3D`, `Blast: Node3D`, `CameraNoiseShake: Camera3D`, `PlayerInputSynchronizer: MultiplayerSynchronizer` |
| Vínculo por troca de `type` na `.tscn`; nenhum `.gd` ponte | ✅ | Tabela "Edição das cenas" abaixo, linhas conferidas em 2026-09-15 |
| Nomes de `#[func]` idênticos | ✅ | `add_trauma`, `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis`, `jump` — [contracts/](contracts/) |
| Nomes de propriedades exportadas/replicadas idênticos (conferidos na `.tscn`) | ✅ | `player.tscn` linhas 42–53 replicam `InputSynchronizer:shoot_target/motion/shooting/aiming`; linhas 343–351 `node_paths` `camera_animation/crosshair/camera_base/camera_rot/camera_camera/color_rect` — todos existem no contrato com o mesmo nome |
| `cargo build` sem warnings novos antes de validar/commitar | ✅ | quickstart.md passo 1; baseline atual: 0 warnings |
| Validação headless (import + cena) | ✅ | quickstart.md passos 2–3; baseline de erros = os 3 do `CLAUDE.md` |
| Commit por port com script + cena na mensagem | ✅ | Formato em quickstart.md §"Commit" |
| `.gd` + `.gd.uid` apagados no mesmo commit | ✅ | Tabela "Edição das cenas"; verificação `grep` do uid após remoção |
| Ordem de baixo para cima | ✅ | Os 5 são folhas (`docs/port-order.md`); nenhum depende de outro. O item 4 é chamado pelo 5? Não: é chamado por `player.gd` (GDScript) via `player_input.camera_camera.add_trauma()` — chamada GDScript→Rust, permitida |
| Rust não chama API customizada de GDScript | ✅ | Nenhuma das 5 classes chama `.call()`/`.get()` em script: `get_parent()` usa só `global_transform`/`get_world_3d()` (API base de Node3D) |
| Exceção `Settings` | N/A | Nenhum dos 5 scripts usa `Settings` |

**Resultado do gate (pré-Phase 0)**: PASS — nenhuma violação, Complexity Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/001-v1-leaves-and-input/
├── plan.md              # Este arquivo
├── spec.md              # Especificação (já validada)
├── research.md          # Phase 0: assinaturas gdext 0.5.5 confirmadas + decisões
├── data-model.md        # Phase 1: estado do PlayerInputSynchronizer e trauma da câmera
├── quickstart.md        # Phase 1: comandos de validação na ordem exata
├── contracts/
│   ├── player-input-synchronizer.md   # superfície consumida por player.gd / player.tscn
│   └── camera-noise-shake.md          # superfície consumida por player.gd (via camera_camera)
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado por este comando)
```

### Source Code (repository root)

```text
oxide_godot_core/                       # workspace Cargo (inalterado)
└── oxide_godot_lib/
    ├── Cargo.toml                      # inalterado (godot = { workspace = true })
    └── src/
        ├── lib.rs                      # ExtensionLibrary + `mod` de cada módulo (só isso)
        ├── debug_label.rs              # struct DebugLabel,              base=Label                  (port 1)
        ├── part_disappear.rs           # struct PartDisappear,           base=CpuParticles3D         (port 2)
        ├── blast.rs                    # struct Blast,                   base=Node3D                 (port 3)
        ├── camera_noise_shake.rs       # struct CameraNoiseShake,        base=Camera3D               (port 4)
        └── player_input.rs             # struct PlayerInputSynchronizer, base=MultiplayerSynchronizer (port 5)

oxide-godot/                            # projeto Godot
├── level/level.tscn                    # port 1: node Debug → type="DebugLabel"
├── level/debug.gd (+ .uid)             # port 1: APAGAR
├── enemies/red_robot/parts/part_disappear_effect/
│   ├── part_disappear.tscn             # port 2: raiz → type="PartDisappear"
│   └── part_disappear.gd (+ .uid)      # port 2: APAGAR
├── enemies/red_robot/laser/impact_effect/
│   ├── impact_effect.tscn              # port 3: raiz → type="Blast"
│   └── blast.gd (+ .uid)               # port 3: APAGAR
└── player/
    ├── player.tscn                     # port 4: Camera3D → type="CameraNoiseShake"; port 5: InputSynchronizer → type="PlayerInputSynchronizer"
    ├── camera_noise_shake_effect.gd (+ .uid)  # port 4: APAGAR
    └── player_input.gd (+ .uid)        # port 5: APAGAR

docs/v2-backlog.md                      # entradas novas por commit, quando percebidas
```

**Structure Decision**: um módulo Rust por script GDScript, nome do arquivo = nome do script
(exceto sufixos redundantes), sem módulo compartilhado e sem helpers comuns — qualquer utilitário
comum seria abstração (Princípio I). `lib.rs` contém apenas a `ExtensionLibrary` e as declarações
`mod`. O nome de classe `PlayerInputSynchronizer` é obrigatório: `player.gd:26` declara
`@onready var player_input: PlayerInputSynchronizer = $InputSynchronizer` e, sem o `.gd`, o
analisador do GDScript resolve esse tipo na classe nativa registrada. Os outros 4 nomes são livres
(sem `class_name` no original); a escolha acima evita colidir com classes do engine
(`Label`, `Blast` não existe no engine — conferido na lista de bindings).

Detalhes de cada classe (campos, virtuais, sinais) estão em [research.md](research.md) §"Mapa por
script"; não são repetidos aqui.

## Edição das cenas (linhas conferidas em 2026-09-15 — reconferir com `grep -n` antes de editar)

| Port | Cena | Node (linha) | `type` antes → depois | Remover | Apagar |
|---|---|---|---|---|---|
| 1 | `level/level.tscn` | `Debug` (l.148) | `Label` → `DebugLabel` | l.156 `script = ExtResource("9")`; l.7 `[ext_resource type="Script" uid="uid://6ec6m14rhsxi" ... id="9"]` | `level/debug.gd`, `level/debug.gd.uid` |
| 2 | `.../part_disappear.tscn` | raiz `PartDisappearPuff` (l.43) | `CPUParticles3D` → `PartDisappear` | l.59 `script = ExtResource("3")`; l.5 ext_resource `uid://dxd6xoeg627y6` id="3" | `part_disappear.gd`, `.uid` |
| 3 | `.../impact_effect.tscn` | raiz `Blast` (l.170) | `Node3D` → `Blast` | l.171 `script = ExtResource("5")`; l.7 ext_resource `uid://bk20efkdq4v3m` id="5" | `blast.gd`, `.uid` |
| 4 | `player/player.tscn` | `Camera3D` (l.630, parent `CameraBase/CameraRot/SpringArm3D`) | `Camera3D` → `CameraNoiseShake` | l.633 `script = ExtResource("8")`; l.11 ext_resource `uid://byrvr71jmaisi` id="8" | `camera_noise_shake_effect.gd`, `.uid` |
| 5 | `player/player.tscn` | `InputSynchronizer` (l.343) | `MultiplayerSynchronizer` → `PlayerInputSynchronizer` | l.345 `script = ExtResource("2_g11dy")`; l.5 ext_resource `uid://m1xn31x0lssx` id="2_g11dy". **MANTER** `node_paths=PackedStringArray(...)` na l.343, `replication_config` (l.344) e as 6 linhas `xxx = NodePath(...)` (l.346–351) | `player_input.gd`, `.uid` |

Após o port 4, as linhas do port 5 deslocam (−1 pela remoção do ext_resource id="8" na l.11):
reconferir. Depois de cada remoção: `grep -rn "<uid>" oxide-godot/` deve retornar vazio
(pré-conferido: cada uid é referenciado por exatamente uma cena).

Observação sobre o `type` na `.tscn`: o Godot grava `type="<nome da classe registrada>"`; para
classes GDExtension isso é o nome do struct Rust. Ao abrir a cena no editor depois do port, o editor
pode reescrever a `.tscn` (ordem de propriedades, `unique_id`) — diffs assim são aceitáveis desde
que o `type` e as propriedades permaneçam.

## Validação visual por script (SC-002 — feita pelo usuário no editor/jogo)

| Port | O que conferir no jogo (comparar com `../oxide_godot_origins/`) |
|---|---|
| 1 | Entrar no level; F3 alterna o overlay; linhas `FPS: 60.0` (com `.0`), `VSync: Enabled/Disabled`, `Memory: xx.xx MiB`, `Online: No` (sem linha de ID em single-player); valores mudam a cada frame |
| 2 | Matar um robô: cada peça, ao sumir, dispara mini-blasts na hora e o puff de fumaça 0,2 s depois; o efeito some sozinho (~3,2 s) |
| 3 | Deixar o robô atirar: o impacto anima, os raios de luz ficam voltados para a câmera enquanto se move ao redor, e o efeito some ao fim da animação |
| 4 | Atirar (tremor leve), ser atingido (médio), ser acertado pelo robô (forte, saturado); a câmera volta exatamente à posição de repouso; intensidade/duração iguais ao original |
| 5 | Mover (WASD/analógico), olhar (mouse e analógico; mais lento ao mirar), pitch trava em −89,9°/70°, mira por toque curto (fica) e por hold (solta), animações de câmera shoot/far, pular, atirar acerta o ponto sob o crosshair, cair do mapa escurece a tela (preta em −32) e volta com fade-out |

## Complexity Tracking

> Vazio — o Constitution Check não tem violações a justificar.

## Constitution Check — re-avaliação pós-design (Phase 1)

Re-avaliado após research.md, data-model.md, contracts/ e quickstart.md:

- Nenhum artefato introduz módulo comum, trait, enum de estado ou helper — cada classe é
  autocontida (Princípio I). ✅
- O contrato de `PlayerInputSynchronizer` reproduz os 11 nomes de propriedade e 4 de método do
  original, conferidos contra `player.gd` (linhas 26, 73, 87, 89, 107, 114, 121, 124, 133, 135,
  211) e `player.tscn` (linhas 42–53, 343–351). ✅
- O contrato de `CameraNoiseShake` reproduz `add_trauma(amount: float)`, chamado por
  `player.gd:211` e, por tabela, por `red_robot.gd:133`. ✅
- A única decisão que se afasta do texto do input do comando (transfer mode do RPC: `unreliable`
  em vez de `reliable`) foi tomada **em favor** da fidelidade ao original — ver research.md D5. ✅
- Melhorias percebidas durante o research foram listadas como candidatas ao backlog, não
  aplicadas. ✅

**Resultado do gate (pós-Phase 1)**: PASS.
