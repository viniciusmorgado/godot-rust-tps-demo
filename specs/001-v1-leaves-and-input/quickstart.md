# Quickstart: validação de cada port (Marco A)

Guia de execução/validação — o que rodar, na ordem, e o que esperar. Os detalhes de código estão
em [research.md](research.md); os nomes a conferir, em [contracts/](contracts/). Todos os caminhos
são relativos à raiz do repositório (`oxide-godot/`, onde está este `specs/`).

## Pré-requisitos

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (não há `godot` no PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 já resolvido (não editar `Cargo.toml`).
- Nenhum editor Godot aberto no projeto durante a validação headless (evita hot-reload
  concorrente e lock do `.godot/`). Se estiver aberto, feche antes de rodar o passo 2.

## Baseline (medida em 2026-09-15, commit `6b22de3`, antes de qualquer port)

- `cargo build`: **0 warnings**.
- Import headless: linha 1 = `Initialize godot-rust (API v4.6.stable.official, runtime v4.7.2.stable.official, safeguards strict)`;
  **0 linhas `ERROR`**. Os 3 erros do upstream catalogados no `CLAUDE.md` (`Cannon_Charge already exists`,
  `doorsimple_d.png` ausente, `surfaces.is_empty()`) só aparecem em import limpo (`.godot/` apagado);
  não contam como regressão em nenhum caso.
- Execução headless de `level.tscn`, `player.tscn`, `part_disappear.tscn`, `impact_effect.tscn`:
  **0 linhas `ERROR`/`SCRIPT ERROR`**, exit **124** (timeout — cenas de jogo não encerram sozinhas;
  `part_disappear.tscn` também não: `queue_free` da raiz não encerra a `SceneTree`). Dois
  `WARNING` benignos e pré-existentes: `HDR output requested, but it is not supported by this
  display server` e `[Physics interpolation] Interpolated Camera3D triggered from outside physics
  process` (só nas cenas com Player).

Qualquer linha `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Nonexistent function`, `Invalid
get/set index` ou `panicked` que não esteja nesta baseline é regressão do port.

## Ciclo por script (repetir 5 vezes, na ordem 1 → 5)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
```

Esperado: `Finished 'dev' profile`, **zero warnings novos** (baseline 0 → deve continuar 0).
Contagem rápida: `cargo build 2>&1 | grep -c '^warning'` → `0`.

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
```

Exit 124 (timeout) é o resultado esperado; exit 0 também é aceitável. A validação é a ausência de
linhas novas no `grep`, não o código de saída.

| Port | `<cena>` a rodar |
|---|---|
| 1 `debug.gd` | `level/level.tscn` |
| 2 `part_disappear.gd` | `enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn` |
| 3 `blast.gd` | `enemies/red_robot/laser/impact_effect/impact_effect.tscn` |
| 4 `camera_noise_shake_effect.gd` | `player/player.tscn` **e** `level/level.tscn` |
| 5 `player_input.gd` | `player/player.tscn` **e** `level/level.tscn` |

Para os ports 2 e 3 vale também rodar `level/level.tscn` (o robô instancia as duas cenas em
runtime, mas só sob interação — o headless não exercita; a instanciação isolada acima já prova que
a classe resolve e `ready()` roda).

### 4. Verificações mecânicas do ciclo (Princípio II)

```bash
# o script e o uid sumiram, e nenhuma cena ainda aponta para eles
ls oxide-godot/<caminho>/<script>.gd oxide-godot/<caminho>/<script>.gd.uid   # esperado: No such file
grep -rn '<uid do .gd>' oxide-godot/                                          # esperado: vazio
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # esperado: vazio

# o node trocou de tipo e não tem mais script
grep -n 'type="<ClasseRust>"' oxide-godot/<cena>.tscn                          # esperado: 1 linha
grep -n 'ExtResource("<id do script>")' oxide-godot/<cena>.tscn                # esperado: vazio

# os outros 10 .gd não mudaram
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # esperado: só a deleção do script deste port
```

Uids e ids por port estão na tabela "Edição das cenas" do [plan.md](plan.md).

### 5. Conferência de nomes (ports 4 e 5)

Rodar os comandos da seção "Verificação antes do commit" em
[contracts/camera-noise-shake.md](contracts/camera-noise-shake.md) e
[contracts/player-input-synchronizer.md](contracts/player-input-synchronizer.md); cada nome
encontrado deve existir na classe Rust com o mesmo nome.

### 6. Validação visual (usuário, no editor/jogo)

Abrir o projeto no editor, rodar (F5), entrar no level e conferir o item correspondente da tabela
"Validação visual por script" do [plan.md](plan.md), comparando com `../oxide_godot_origins/`.
Ao abrir a cena editada no editor: o node deve aparecer com o tipo Rust, sem script anexado, e
(port 5) com as 6 referências e a `replication_config` preservadas no inspector.

### 7. Backlog v2

Se alguma melhoria foi percebida durante o port, adicionar uma linha em `docs/v2-backlog.md`
(origem + melhoria + motivação) **antes** do commit. Candidatos já identificados: research.md
§"Backlog v2 candidato".

### 8. Commit

Um commit por script, na `main`, incluindo: módulo Rust novo + `lib.rs`, `.tscn` editada,
`.gd` + `.gd.uid` apagados, `docs/v2-backlog.md` (se houver entrada). Mensagem:

```
Port <script>.gd → <ClasseRust> (<Base>); <cena>.tscn: node <Nome> type="<Base>"→"<ClasseRust>"

- <notas relevantes: decisões de tradução, quirks preservados>
- backlog v2: <itens adicionados, ou "nenhum">
```

## Verificação final do marco (após o 5º commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l          # esperado: 10
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # esperado: 10
git diff --stat 6b22de3 -- 'oxide-godot/**/*.gd'                       # esperado: só 5 deleções
git log --oneline 6b22de3..HEAD                                        # esperado: 5 commits "Port ..."
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 5 módulos
```

E, no jogo: menu → level → mover, olhar, mirar (toggle e hold), pular, atirar, tremor, F3,
impacto do laser, peças sumindo — idênticos ao original (SC-002).
