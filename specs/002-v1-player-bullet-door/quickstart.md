# Quickstart: validação de cada port (Marco B)

Guia de execução/validação — o que rodar, na ordem, e o que esperar. Detalhes de código em
[research.md](research.md); nomes a conferir em [contracts/](contracts/). Caminhos relativos à
raiz do repositório (`oxide-godot/`, onde está este `specs/`).

## Pré-requisitos

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (não há `godot` no PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 já resolvido (não editar `Cargo.toml`).
- **Nenhum editor Godot aberto** no projeto durante validação headless (`pgrep -a godot` vazio).
  Avisar o usuário antes de começar; nunca matar o processo dele.
- Árvore limpa na `main` (`git status --short` vazio) antes de cada port.

## Baseline (medida em 2026-09-15, commit `108584e`, antes de qualquer port deste marco)

- `cargo build`: **0 warnings**.
- Import headless: linha 1 = `Initialize godot-rust (API v4.6.stable.official, runtime v4.7.2.stable.official, safeguards strict)`;
  **0 linhas `ERROR`** em `.godot/` já importado. Os 3 erros do upstream catalogados no
  `CLAUDE.md` (`Cannon_Charge already exists`, `doorsimple_d.png` ausente, `surfaces.is_empty()`)
  só aparecem em import limpo e não contam como regressão.
- Execução headless de `level.tscn`, `player.tscn`, `player/bullet/bullet.tscn`: **0 linhas
  `ERROR`/`SCRIPT ERROR`**, exit **124**. `WARNING` benignos e pré-existentes: `HDR output
  requested, but it is not supported by this display server` (todas as cenas) e `[Physics
  interpolation] Interpolated Camera3D triggered from outside physics process` (cenas com Player).
- **`door/door.tscn`: exatamente 1 `ERROR`** — `ERROR: Node not found: "DoorModel/AnimationPlayer"
  (relative to "/root/Door").` (`grep -c 'Node not found' /tmp/run.log` → `1`), exit 124, 1
  WARNING (HDR). **Este erro DEVE desaparecer após o port 3** (FR-027, SC-003). Ele não consta do
  catálogo do `CLAUDE.md` — nada a remover de lá.

Qualquer linha `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` que não esteja nesta baseline é regressão do port.

## Ciclo por script (repetir 3 vezes, na ordem 1 → 2 → 3)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
cargo build 2>&1 | grep -c '^warning'      # esperado: 0
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

Exit 124 (timeout) é o esperado; exit 0 também é aceitável. A validação é a ausência de linhas
novas no `grep`.

| Port | `<cena>` a rodar | O que exercita |
|---|---|---|
| 1 `player.gd` | `player/player.tscn` **e** `level/level.tscn` | `ready` do Player (OnReady ×10, `orientation`), `physics_process` no servidor (`apply_input`, WALK, primeiro pouso → `land` RPC + som), `BulletCache` (bala ainda GDScript). No level: `level.gd` spawna o Player (`name`, `player_id` fora da árvore → setter), `red_robot.gd` resolve `is Player`/`add_camera_shake_trauma` pelo nome — qualquer nome errado aparece aqui |
| 2 `bullet.gd` | `player/bullet/bullet.tscn`, `player/player.tscn` **e** `level/level.tscn` | Bala isolada: `ready` (servidor), voo, **expira aos 5 s → `explode` → `Settings` lido → `destroy` aos 1,5 s da animação** (tudo dentro dos 20 s). `player.tscn`: `BulletCache` instancia a classe Rust. Level: integração completa |
| 3 `door.gd` | `door/door.tscn` (+ `level/level.tscn` por segurança) | Instanciação sem `Node not found`; conexão `_on_door_body_entered` resolve |

**Port 3 — verificação adicional obrigatória**:

```bash
grep -c 'Node not found' /tmp/run.log          # esperado: 0 (baseline: 1)
```

### 4. Verificações mecânicas do ciclo (Princípio II)

```bash
# o script e o uid sumiram, e nenhuma cena ainda aponta para eles
ls oxide-godot/<caminho>/<script>.gd oxide-godot/<caminho>/<script>.gd.uid   # esperado: No such file
grep -rn '<uid do .gd>' oxide-godot/                                          # esperado: vazio
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # esperado: vazio

# o node trocou de tipo e não tem mais script
grep -n 'type="<ClasseRust>"' oxide-godot/<cena>.tscn                          # esperado: 1 linha
grep -n 'ExtResource("1")' oxide-godot/<cena>.tscn                             # esperado: vazio (nos 3 ports o id do script é "1")

# os 7 .gd fora do marco não mudaram
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # esperado: só a deleção do script deste port
```

Uids, ids e linhas por port: tabela "Edição das cenas" do [plan.md](plan.md).

### 5. Conferência de nomes (todos os ports)

Rodar a seção "Verificação antes do commit" do contrato do port
([contracts/player.md](contracts/player.md), [bullet.md](contracts/bullet.md),
[door.md](contracts/door.md)); cada nome encontrado deve existir na classe Rust com o mesmo nome.

No port 1, conferir também que `player_input.rs` e `camera_noise_shake.rs` só mudaram em
visibilidade:

```bash
git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player_input.rs oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs | grep '^[-+]' | grep -v '^[-+][-+]' | grep -v 'pub(crate)'
# esperado: apenas as linhas "-" originais correspondentes (10 pares -/+ ao todo: 6 campos + 3 métodos + add_trauma); nenhuma outra linha
```

### 6. Validação visual (usuário, no editor/jogo)

Abrir o projeto no editor, rodar (F5), entrar no level e conferir o item do port na tabela
"Validação visual por script" do [plan.md](plan.md), comparando com `../oxide_godot_origins/`.
Ao abrir a cena editada: o node raiz deve aparecer com o tipo Rust, sem script anexado;
`player.tscn` deve manter `ServerSynchronizer`/`InputSynchronizer`/`BulletCache` intactos.

### 7. Backlog v2

Adicionar em `docs/v2-backlog.md` os candidatos de research.md §"Backlog v2 candidato"
atribuídos a **este** script (port 1: itens 10–12; port 2: item 13; port 3: item 14), **antes** do
commit. Itens 1 (Settings) e 2 (`Hittable`) já existem — não duplicar. Numeração continua de 10.

### 8. Commit

Um commit por script, na `main`, incluindo: módulo Rust novo + `lib.rs` (+ no port 1, as duas
alterações de visibilidade), `.tscn` editada, `.gd` + `.gd.uid` apagados, `docs/v2-backlog.md`
(se houver entrada) e, **no port 3, `docs/upstream-bugs.md` novo**. Autor:
the repository author. Mensagem:

```
Port <script>.gd → <ClasseRust> (<Base>); <cena>.tscn: node <Nome> type="<Base>"→"<ClasseRust>"

- <notas: decisões de tradução, quirks preservados>
- backlog v2: <itens adicionados, ou "nenhum">
```

Port 1 acrescenta a nota: `- player_input.rs / camera_noise_shake.rs: só visibilidade pub(crate)
nos campos/métodos consumidos pelo Player (acesso tipado, FR-010/FR-011); nenhuma lógica movida`.

**Port 3 — formato obrigatório (requisito (c) da cláusula de bugs)**:

```
Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"

- upstream bug fix: door.gd referenciava "DoorModel/AnimationPlayer" (node inexistente); o port
  referencia "DoorModel2/AnimationPlayer" — a porta passa a abrir e o "ERROR: Node not found"
  desaparece (baseline 1 → 0). Correção mínima; node da cena não renomeado; lógica de open intacta.
- docs/upstream-bugs.md criado com a entrada #1 (commit: este).
- CLAUDE.md: catálogo inalterado — o erro da porta nunca constou dele (só os 3 erros de import).
- backlog v2: item 14
```

Coluna "commit" de `docs/upstream-bugs.md`: o hash só existe depois do commit, então a entrada
identifica o commit pelo **assunto** (`Port door.gd → Door (Area3D); ...`) — suficiente para
`git log --grep`. Se o hash literal for desejado, `git commit --amend` logo após o commit do port
3, **antes** de qualquer outro commit (continua um commit por script).

## Verificação final do marco (após o 3º commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l          # esperado: 7
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # esperado: 7
git diff --stat 108584e -- 'oxide-godot/**/*.gd'                       # esperado: só 3 deleções (player.gd, bullet.gd, door.gd)
git log --oneline 108584e..HEAD | grep -c '^[0-9a-f]* Port '           # esperado: 3
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 8 módulos (debug_label, part_disappear, blast, camera_noise_shake, player_input, player, bullet, door)
test -f docs/upstream-bugs.md && grep -c '^| 1 ' docs/upstream-bugs.md # 1 entrada
git diff --stat 108584e -- CLAUDE.md                                   # esperado: vazio
grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/      # esperado: 1 linha (door.rs)
```

E, no jogo (SC-002): menu → level → mover, pular (som), pousar (som), mirar, atirar com bala
visível que explode e acerta robôs, tremor de câmera, respawn ao cair abaixo de −40 — idênticos
ao original. Porta (SC-009): em cena de teste isolada, **não commitada** (p.ex. no scratchpad ou
uma `.tscn` temporária em `oxide-godot/` removida antes do commit), `door.tscn` + um `Player`
entrando na área → animação toca uma vez; `door.tscn` headless sem erro.
