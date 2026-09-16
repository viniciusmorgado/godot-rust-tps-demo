# Quickstart: validação de cada port (Marco D)

Guia de execução/validação — o que rodar, na ordem, e o que esperar. Detalhes de código em
[research.md](research.md); nomes a conferir em [contracts/](contracts/). Caminhos relativos à
raiz do repositório (`oxide-godot/`, onde está este `specs/`).

## Pré-requisitos

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (não há `godot` no PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5. **Neste marco `Cargo.toml` muda uma vez** (port 3:
  `features = ["experimental-threads"]`, research D1) — fora disso, não editar.
- **Nenhum editor Godot aberto** no projeto durante validação headless (`pgrep -a godot` vazio).
  Avisar o usuário antes de começar; nunca matar o processo dele.
- Árvore limpa na `main` (`git status --short` vazio) antes de cada port.

## Baseline (medida em 2026-09-16, commit `a866428`, antes de qualquer port deste marco)

- `cargo build`: **0 warnings** (sem a feature).
- Import headless: linha 1 = `Initialize godot-rust (…)`; **0 linhas `ERROR`**. Os 3 erros do
  upstream catalogados no `CLAUDE.md` só aparecem em import limpo e não contam.
- `level/forklift/flying_forklift.tscn`: exit 124, 0 `ERROR`, 1 WARNING (HDR).
- `level/level.tscn`: exit 124, 0 `ERROR`, 2 WARNINGs (HDR, Physics interpolation).
- `menu/menu.tscn`: exit 124, 0 `ERROR`, 1 WARNING (HDR). Em headless o menu **hospeda sozinho**
  (`_on_host_pressed` diferido) e carrega o level em thread; sem o `main`, o sinal
  `replace_main_scene` é emitido e nada o consome.
- `main/main.tscn`: exit 124, 2 WARNINGs. Fluxo completo boot → menu → host automático → level.
  **Erro intermitente do engine** (1 em 6 execuções, a primeira após `--import`): 5 linhas
  `ERROR` do renderizador dummy — `Initializing already initialized RID`, `Parameter "mem" is null.`,
  3× `Parameter "m" is null.` (`rid_owner.h`, `dummy/storage/mesh_storage.h`). Vem da corrida
  entre o carregamento do level em sub-thread e o renderizador dummy; o original intocado tem a
  mesma arquitetura. **Regra**: se essas 5 linhas exatas aparecerem, executar de novo — não são
  regressão se sumirem; qualquer outra linha `ERROR`, ou essas mesmas em 3 execuções seguidas, é
  regressão. Catalogar no `CLAUDE.md` (edição operacional) no commit do port 3 junto com a linha
  da feature.
- `docs/upstream-bugs.md`: 2 entradas (não muda); `docs/v2-backlog.md`: 18 itens.

Qualquer linha `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` ou **`shadows a native class`** que não esteja nesta baseline é regressão.

## Ciclo por script (repetir 4 vezes, na ordem 1 → 4)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
cargo build 2>&1 | grep -c '^warning'      # esperado: 0
```

No port 3, o primeiro build após ativar a feature regenera as bindings do `godot-core` (alguns
minutos na primeira vez; o diretório `target/debug/build/godot-core-*/out` ganha um segundo
hash). Depois disso, `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` é
o diretório válido para consultar assinaturas.

### 2. Import headless (extensão carregou?)

```bash
cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log
grep -n 'Initialize godot-rust' /tmp/import.log        # deve existir (linha 1)
grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log            # esperado: vazio (ou só os 3 do upstream)
```

### 3. Execução headless da(s) cena(s) afetada(s)

```bash
cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . <cena>.tscn 2>&1 | tee /tmp/run.log
grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/run.log   # esperado: vazio
grep -c '^WARNING' /tmp/run.log                                                                                              # só os benignos da baseline
```

| Port | `<cena>` a rodar | O que exercita |
|---|---|---|
| 1 `flying_forklift.gd` | `level/forklift/flying_forklift.tscn` **e** `level/level.tscn` | `ready` da empilhadeira (Settings dinâmico, sorteio do modelo) isolada e nas instâncias do level |
| 2 `level.gd` | `level/level.tscn` **e** `main/main.tscn` | Level isolado: Settings dinâmico, GI, spawn tipado de 4 `EnemyRobot` e do `Player` 1, conexões `peer_*`. `main.tscn` (ainda `main.gd`): menu hospeda → `replace_main_scene` → `main.gd` instancia o `Level` Rust e conecta `quit` por `has_signal` |
| 3 `menu.gd` | `menu/menu.tscn` **e** `main/main.tscn` | Menu isolado em headless: `ready` (85 `OnReady`, 15 `ButtonGroup`), host automático, loading em thread (feature), `DoneTimer`, emissão de `replace_main_scene`. `main.tscn` (ainda `main.gd`): fluxo completo com `Menu` Rust |
| 4 `main.gd` | `main/main.tscn` | Boot Rust → `Menu` → host → `Level`; `has_signal`/`connect` por nome resolvem nas classes Rust |

Exit 124 (timeout) é o esperado; a validação é a ausência de linhas novas no `grep`. Para
`main.tscn`, aplicar a regra do erro intermitente da baseline.

### 4. Verificações mecânicas do ciclo (Princípio II)

```bash
ls oxide-godot/<caminho>/<script>.gd oxide-godot/<caminho>/<script>.gd.uid   # esperado: No such file
grep -rn '<uid do .gd>' oxide-godot/ | grep -v '/.godot/'                     # esperado: vazio
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # esperado: vazio
grep -c 'type="<Classe>"' oxide-godot/<cena>.tscn                             # 1
grep -c 'ExtResource("<id>")' oxide-godot/<cena>.tscn                         # 0  (ids: forklift "3"; level/menu/main "1")
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # só a deleção do script deste port; settings.gd nunca aparece
```

Uids, ids e linhas por port: tabela "Edição das cenas" do [plan.md](plan.md).

### 5. Conferência de nomes (todos os ports)

Rodar a seção "Verificação antes do commit" do contrato do port
([contracts/flying-forklift.md](contracts/flying-forklift.md), [level.md](contracts/level.md),
[menu.md](contracts/menu.md), [main.md](contracts/main.md)).

No port 2, conferir que `player.rs` e `red_robot.rs` só mudaram em visibilidade:

```bash
git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'
# esperado: exatamente 4 linhas — "-    fn set_player_id(" / "+    pub(crate) fn set_player_id(" e "-    fn exploded();" / "+    pub(crate) fn exploded();"
```

No port 3, conferir `Cargo.toml` e `CLAUDE.md`:

```bash
git diff HEAD -- oxide_godot_core/Cargo.toml | grep '^[-+]godot'   # 1 par: features = ["experimental-threads"]
git diff --stat HEAD -- CLAUDE.md                                   # só linhas adicionadas (feature + erro intermitente)
```

### 6. Validação visual (usuário, no editor/jogo)

Abrir o projeto no editor, rodar (F5) e conferir o item do port na tabela "Validação visual por
script" do [plan.md](plan.md), comparando com `../oxide_godot_origins/`. Ao abrir a cena editada:
raiz com o tipo Rust, sem script; `menu.tscn` com as 10 conexões no painel de sinais; `main.tscn`
com o node ainda chamado `main`.

### 7. Backlog v2

Adicionar em `docs/v2-backlog.md` os candidatos de research.md §"Backlog v2 candidato" do script
(port 1: 19; port 2: 20–21; port 3: 22–23; port 4: 24), **antes** do commit. Numeração continua
de 19.

### 8. Commit

Um commit por script, na `main`, incluindo: módulo Rust novo + `lib.rs` (+ port 2: `player.rs`,
`red_robot.rs` só visibilidade; + port 3: `Cargo.toml`, `CLAUDE.md`), `.tscn` editada, `.gd` +
`.gd.uid` apagados, `docs/v2-backlog.md`. Autor: the repository author.

```
Port flying_forklift.gd → FlyingForklift (CharacterBody3D); flying_forklift.tscn: node FlyingForklift type="CharacterBody3D"→"FlyingForklift"

- base CharacterBody3D = tipo do node (script extends Node3D — constituição v1.3.1, Princípio II)
- <notas>; backlog v2: item 19
```

```
Port level.gd → Level (Node3D); level.tscn: node Level type="Node3D"→"Level"

- <notas: sinal quit; EnemyRobot/Player tipados; Settings dinâmico; add_player/del_player por closure>
- player.rs / red_robot.rs: só visibilidade pub(crate) em set_player_id / exploded (acesso tipado, FR-025); nenhuma lógica movida
- backlog v2: itens 20, 21
```

```
Port menu.gd → Menu (Node); menu.tscn: node Menu type="Node"→"Menu"

- <notas: sinal replace_main_scene(PackedScene); 85 OnReady; 9 handlers; loading em thread>
- Cargo.toml: feature experimental-threads do gdext (ResourceLoader::load_threaded_* não é gerado sem ela — godot-codegen special_cases.rs:83-86); versão 0.5.5 inalterada
- CLAUDE.md: linha da feature + catálogo do erro intermitente do renderizador dummy em main.tscn headless
- backlog v2: itens 22, 23
```

```
Port main.gd → Main (Node); main.tscn: node main type="Node"→"Main"

- <notas: has_signal/connect por nome preservados; call_deferred por nome; node continua chamado "main">
- backlog v2: item 24
```

## Verificação final do marco (após o 4º commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*'                  # só oxide-godot/menu/settings.gd
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # 1
git diff --stat a866428 -- 'oxide-godot/**/*.gd'                       # exatamente 4 deleções (flying_forklift, level, menu, main); settings.gd ausente
git log --oneline a866428..HEAD | grep -c '^[0-9a-f]* Port '           # 4
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 14 módulos (… flying_forklift, level, menu, main_scene)
grep -n '^godot' oxide_godot_core/Cargo.toml                           # godot = { version = "0.5.5", features = ["experimental-threads"] }
grep -n 'experimental-threads' CLAUDE.md                               # 1+ linhas
grep -c '^| [0-9]' docs/upstream-bugs.md                               # 2 (inalterado)
grep -c '^| [0-9]' docs/v2-backlog.md                                  # 24
# FR-025: chamadas dinâmicas só as permitidas
grep -nE '\.call\(' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs   # só "apply_graphics_settings" e "save_settings" (level.rs, menu.rs)
grep -nE 'call_deferred\(' oxide_godot_core/oxide_godot_lib/src/{menu,main_scene}.rs                   # só "_on_host_pressed" (menu) e "change_scene_to_packed" (main_scene)
grep -nE '\.get\("' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs   # só "config_file"
grep -nE 'has_signal|from_object_method' oxide_godot_core/oxide_godot_lib/src/main_scene.rs           # "quit" e "replace_main_scene" — duck typing do original
```

E, no jogo (SC-002): boot → menu → Play (barra de loading) → level jogável (jogador, robôs com
respawn 15 s, empilhadeiras com modelos variados, GI conforme a opção) → ESC volta ao menu →
Settings: aplicar, cancelar, reabrir, conferir `user://settings.ini` → Quit encerra — idêntico ao
original.
