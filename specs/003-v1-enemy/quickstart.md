# Quickstart: validação de cada port (Marco C)

Guia de execução/validação — o que rodar, na ordem, e o que esperar. Detalhes de código em
[research.md](research.md); nomes a conferir em [contracts/](contracts/). Caminhos relativos à
raiz do repositório (`oxide-godot/`, onde está este `specs/`).

## Pré-requisitos

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (não há `godot` no PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 já resolvido (não editar `Cargo.toml`).
- **Nenhum editor Godot aberto** no projeto durante validação headless (`pgrep -a godot` vazio).
  Avisar o usuário antes de começar; nunca matar o processo dele.
- Árvore limpa na `main` (`git status --short` vazio) antes de cada port.

## Baseline (medida em 2026-09-15, commit `4bb8f7f`, antes de qualquer port deste marco)

- `cargo build`: **0 warnings**.
- Import headless: linha 1 = `Initialize godot-rust (API v4.6.stable.official, runtime v4.7.2.stable.official, safeguards strict)`;
  **0 linhas `ERROR`** em `.godot/` já importado. Os 3 erros do upstream catalogados no
  `CLAUDE.md` só aparecem em import limpo e não contam como regressão.
- `enemies/red_robot/red_robot.tscn`: exit **124**, **0 linhas `ERROR`/`SCRIPT ERROR`**, 1 `WARNING`
  benigno (`HDR output requested…`). O robô isolado fica IDLE (sem jogador) e cai com a
  gravidade; as 3 peças rodam `ready` (duplicação de materiais).
- `level/level.tscn`: exit 124, 0 `ERROR`, 2 `WARNING` benignos (`HDR output…`,
  `[Physics interpolation] Interpolated Camera3D…`).
- `docs/upstream-bugs.md`: 1 entrada na baseline; passa a 2 na Phase 3b (correção conservadora da peça, FR-028–FR-032); `docs/v2-backlog.md`: 14 itens.

Qualquer linha `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` que não esteja nesta baseline é regressão do port.

## Ciclo por script (repetir 2 vezes, na ordem 1 → 2)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
cargo build 2>&1 | grep -c '^warning'      # esperado: 0 (atenção: Resource::duplicate() está deprecado — usar duplicate_resource(), research D3)
```

### 2. Import headless (extensão carregou?)

```bash
cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log
grep -n 'Initialize godot-rust' /tmp/import.log        # deve existir (linha 1)
grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log            # esperado: vazio (ou só os 3 do upstream)
```

### 3. Execução headless da(s) cena(s) afetada(s)

```bash
cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . <cena>.tscn 2>&1 | tee /tmp/run.log
grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log   # esperado: vazio
grep -c '^WARNING' /tmp/run.log                                                                         # esperado: só os benignos da baseline
```

| Port | `<cena>` a rodar | O que exercita |
|---|---|---|
| 1 `part.gd` | `enemies/red_robot/red_robot.tscn` **e** `level/level.tscn` | As 3 peças instanciam a classe Rust: `ready` (`set_process(false)`, `get_node("Model").get_child(0)`, duplicação do material e do `next_pass`). `explode()` não é exercitado sem morte (o robô ainda é GDScript; `red_robot.gd:96-98` resolve o nome no editor/jogo) |
| 2 `red_robot.gd` | `enemies/red_robot/red_robot.tscn` **e** `level/level.tscn` | Robô isolado: `ready` (14 `OnReady`, `AnimationTree` ativo, `animate(0)`), `physics_process` sem jogador (IDLE, gravidade, `move_and_slide`). Level: `level.gd` instancia, atribui `transform`, conecta `exploded` por nome, `add_child`; robôs spawnam e ficam IDLE (o Player headless nasce parado e, se a área de detecção o alcançar, APPROACH/AIM rodam também) |

Exit 124 (timeout) é o esperado; a validação é a ausência de linhas novas no `grep`.

### 4. Verificações mecânicas do ciclo (Princípio II)

```bash
# o script e o uid sumiram, e nenhuma cena ainda aponta para eles
ls oxide-godot/<caminho>/<script>.gd oxide-godot/<caminho>/<script>.gd.uid   # esperado: No such file
grep -rn '<uid do .gd>' oxide-godot/ | grep -v '/.godot/'                     # esperado: vazio (o cache em .godot/ é regenerado pelo import)
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # esperado: vazio

# o(s) node(s) trocou(aram) de tipo e não tem(êm) mais script
grep -c 'type="Part"' oxide-godot/enemies/red_robot/red_robot.tscn            # port 1: 3
grep -c 'ExtResource("24")' oxide-godot/enemies/red_robot/red_robot.tscn      # port 1: 0
grep -c 'type="RedRobot"' oxide-godot/enemies/red_robot/red_robot.tscn        # port 2: 1
grep -c 'ExtResource("1")' oxide-godot/enemies/red_robot/red_robot.tscn       # port 2: 0

# os 5 .gd fora do marco não mudaram
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # esperado: só a deleção do script deste port
```

Uids, ids e linhas por port: tabela "Edição das cenas" do [plan.md](plan.md). `red_robot.tscn` tem
11.053 linhas: a edição é por `sed` em linhas conferidas com `grep -n` imediatamente antes.

### 5. Conferência de nomes (todos os ports)

Rodar a seção "Verificação antes do commit" do contrato do port
([contracts/part.md](contracts/part.md), [contracts/red-robot.md](contracts/red-robot.md)); cada
nome encontrado deve existir na classe Rust com o mesmo nome.

No port 2, conferir também que `player.rs` só mudou em visibilidade:

```bash
git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs | grep '^[-+]' | grep -v '^[-+][-+]'
# esperado: exatamente 2 linhas — "-    fn add_camera_shake_trauma(" e "+    pub(crate) fn add_camera_shake_trauma("
```

E que `part.rs` **não** mudou no port 2 (`git diff --stat HEAD -- .../part.rs` vazio — o
`pub(crate)` de `explode` já entrou no port 1).

### 6. Validação visual (usuário, no editor/jogo)

Abrir o projeto no editor, rodar (F5), entrar no level e conferir o item do port na tabela
"Validação visual por script" do [plan.md](plan.md), comparando com `../oxide_godot_origins/`.
Ao abrir `red_robot.tscn` no editor: (port 1) os 3 nodes `PartShield1/2`, `PartHead` com tipo
`Part`, sem script, `freeze` marcado e `MultiplayerSynchronizer` filho com `public_visibility`
desmarcado; (port 2) raiz `RedRobot`, sem script, `MultiplayerSynchronizer` com a
`replication_config`, conexões da `PlayerDetectionArea` no painel de sinais.

### 7. Backlog v2

Adicionar em `docs/v2-backlog.md` os candidatos de research.md §"Backlog v2 candidato"
atribuídos a **este** script (port 1: item 15; port 2: itens 16–18), **antes** do commit.
Numeração continua de 15.

### 8. Commit

Um commit por script, na `main`, incluindo: módulo Rust novo + `lib.rs` (+ no port 2,
`player.rs` só visibilidade), `red_robot.tscn` editada, `.gd` + `.gd.uid` apagados,
`docs/v2-backlog.md`. Autor: the repository author. Mensagem:

```
Port part.gd → Part (RigidBody3D); red_robot.tscn: nodes Death/PartShield1, Death/PartShield2, Death/PartHead type="RigidBody3D"→"Part"

- <notas: decisões de tradução, quirks preservados>
- backlog v2: item 15
```

```
Port red_robot.gd → RedRobot (CharacterBody3D); red_robot.tscn: node RedRobot type="CharacterBody3D"→"RedRobot"

- <notas>
- player.rs: só visibilidade pub(crate) em add_camera_shake_trauma (acesso tipado do robô, FR-018); nenhuma lógica movida
- backlog v2: itens 16, 17, 18
```

`docs/upstream-bugs.md` **não** muda (nenhuma correção prevista). Se um defeito objetivo for
encontrado, parar e reportar — a cláusula exige declaração na spec antes do commit.

## Verificação final do marco (após o 2º commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l          # esperado: 5
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # esperado: 5
git diff --stat 4bb8f7f -- 'oxide-godot/**/*.gd'                       # esperado: só 2 deleções (part.gd, red_robot.gd)
git log --oneline 4bb8f7f..HEAD | grep -c '^[0-9a-f]* Port '           # esperado: 2
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 10 módulos (… player, bullet, door, part, red_robot)
grep -c '^| [12] ' docs/upstream-bugs.md                               # 2 (porta + peça)
git diff --stat 4bb8f7f -- CLAUDE.md docs/upstream-bugs.md             # esperado: vazio
grep -c '^| [0-9]' docs/v2-backlog.md                                  # 18
# FR-018: nenhuma API customizada de GDScript
grep -nE '\.call\(|\.call_deferred\(|get_script' oxide_godot_core/oxide_godot_lib/src/part.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs   # vazio
grep -nE '\.get\("' oxide_godot_core/oxide_godot_lib/src/red_robot.rs   # só "parameters/aim/blend_position", "position", "collider"
grep -nE '\.set\("' oxide_godot_core/oxide_godot_lib/src/red_robot.rs   # só "parameters/…" (+ o set(&param, …) do hit)
```

E, no jogo (SC-002): robôs patrulham/viram, miram com laser clipado, atiram (impacto + tremor
13,0 ao acertar), reagem a tiros (animação + som), morrem no 5º tiro (peças voam, faíscas, som,
fade + puff entre 3 e 6,5 s), respawn 15 s depois — idênticos ao original.
