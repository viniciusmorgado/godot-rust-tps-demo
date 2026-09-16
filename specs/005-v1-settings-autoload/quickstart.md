# Quickstart: validação do port do autoload Settings (Marco E — fim da v1)

Guia de execução/validação — o que rodar, na ordem, e o que esperar. Detalhes de código em
[research.md](research.md); nomes a conferir em [contracts/settings.md](contracts/settings.md).
Caminhos relativos à raiz do repositório.

## Pré-requisitos

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable. `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 com
  `experimental-threads` (não editar `Cargo.toml`).
- **Nenhum editor Godot aberto** durante validação headless (`pgrep -a godot` vazio) — avisar o
  usuário antes; nunca matar o processo dele.
- Árvore limpa na `main` (`git status --short` vazio); HEAD em `6f4d9ba` ou posterior sem `Port `.
- `INI="$HOME/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini"` — caminho
  real de `user://settings.ini` (`config/name` do `project.godot`). **Antes de tudo**:
  `cp "$INI" <scratch>/settings.ini.baseline` (gravado pelo `settings.gd` original; md5 em
  2026-09-16: `99dac170ad99078f46d2428e650516e7`).

## 1. Baseline (medida em 2026-09-16 — research §E.1)

- `cargo build`: 0 warnings. Import: `Initialize godot-rust`, 0 `ERROR` novo (os 3 do `CLAUDE.md`
  não contam).
- `main/main.tscn` exit 124, 0 regressão, 2 WARNINGs (HDR, Physics interpolation);
  `menu/menu.tscn` 124/0/1; `level/level.tscn` 124/0/2. Regra do **erro intermitente** de
  `main.tscn` (`CLAUDE.md`): as 5 linhas do renderizador dummy não contam se sumirem na 2ª
  execução; 3 seguidas = regressão.
- `docs/upstream-bugs.md` 2 entradas; `docs/v2-backlog.md` 24 itens; 1 `.gd` + 1 `.gd.uid`.

Regressão = qualquer `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` ou `shadows a native class` fora da baseline.

## 2. Ciclo do port (uma vez)

1. Conferir o nome: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | sort -u | grep -w Settings` vazio;
   `ls $(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1)/classes/ | grep -ix 'settings.rs'` vazio.
2. Criar `oxide_godot_core/oxide_godot_lib/src/settings.rs` (research D3–D8; rascunho compilado em
   `scratchpad/settings_draft.rs` com `ZzSettings` → renomear para `Settings`); `mod settings;` em
   `lib.rs` após `mod main_scene;`.
3. `cd oxide_godot_core && cargo build 2>&1 | tail -3` → `Finished`; `grep -c '^warning'` = 0.
4. Criar `oxide-godot/menu/settings.tscn` (3 linhas — plan §"Edição do vínculo").
5. `project.godot`: `grep -n '^Settings=' oxide-godot/project.godot` → l.25; `sed -i '25s|settings\.gd"|settings.tscn"|'`;
   `git diff --stat -- oxide-godot/project.godot` = `2 +-`.
6. `git rm oxide-godot/menu/settings.gd oxide-godot/menu/settings.gd.uid`;
   `grep -rn 'b04fekxdgdq0k\|menu/settings.gd' oxide-godot/ | grep -v /.godot/` vazio.
7. Headless (§3). 8. Contrato (§4) + probes (§5). 9. `docs/upstream-bugs.md` #3 e `docs/v2-backlog.md` 25
   (textos em research §"Entrada #3" e §"Backlog v2 candidato"). 10. Commit (§7). 11. Checkpoint do usuário (§6).

## 3. Validação headless (a partir de `oxide-godot/oxide-godot/`)

```bash
/usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log | grep -c 'Initialize godot-rust'   # 1
grep -E 'ERROR|SCRIPT ERROR' /tmp/import.log | grep -vE 'Cannon_Charge already exists|doorsimple_d.png|surfaces.is_empty'   # vazio
ls menu/                                                                     # settings.tscn presente; NENHUM .gd/.uid; nenhum settings.tscn.uid esperado
for s in main/main.tscn menu/menu.tscn level/level.tscn; do
  timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/run.log 2>&1; echo "$s exit=$?"   # 124
  grep -E 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/run.log   # vazio (regra do intermitente em main)
done
md5sum "$INI"                                                                # igual à baseline: o boot não grava o .ini
```

**Nunca** rodar `menu/settings.tscn` isolada (instanciaria o autoload + a cena: dois `Settings`).
`main.tscn` exercita o fluxo inteiro com o autoload em Rust: `Settings.ready` → `load_settings`;
`Main.ready` lê `display_mode`; `Menu.ready` chama `apply_graphics_settings` (3 args) e o host
automático; `Level.ready` chama `apply_graphics_settings` e lê `gi_type`/`gi_quality`;
`FlyingForklift.ready` lê `shadow_mapping`.

## 4. Contrato e mecânicas

Bloco "Verificação antes do commit" de [contracts/settings.md](contracts/settings.md) — todos os
resultados esperados listados lá. Mais:

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l                # 0
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l            # 0
ls oxide_godot_core/oxide_godot_lib/src/ | wc -l                             # 16 (lib.rs + 15 módulos)
git diff --stat HEAD -- oxide-godot/project.godot                            # 1 file, 1 insertion(+), 1 deletion(-)
git diff HEAD -- oxide-godot/project.godot | grep '^[-+]Settings='           # -…settings.gd" / +…settings.tscn"
git status --short | grep -v 'settings\|lib.rs\|project.godot\|upstream-bugs\|v2-backlog'   # vazio (nada fora do listado)
grep -c '^| [0-9]' docs/upstream-bugs.md                                     # 3
grep -c '^| [0-9]' docs/v2-backlog.md                                        # 25
git diff --quiet HEAD -- CLAUDE.md && echo unchanged                         # unchanged
```

## 5. Probes descartáveis (`-s`, fora do repo — scratchpad; research §E.3/E.3b/E.4 têm os scripts)

Rodar de `oxide-godot/oxide-godot/` com `timeout 30 /usr/bin/godot.x86_64 --headless --path . -s <scratch>/probe.gd`:

- **SC-003 (idempotência do `.ini`)**: probe chama `/root/Settings.save_settings()`; depois
  `diff "$INI" <scratch>/settings.ini.baseline` → vazio.
- **FR-024 (SSAO desligado)**: probe seta `config_file.set_value("rendering","ssao_quality",-1)`,
  chama `apply_graphics_settings(root, Environment.new(), Node.new())` e imprime
  `environment.ssao_enabled` → `false`; com MEDIUM/HIGH → `true`. Restaurar o valor no fim (o
  probe não salva).
- **Padrões (com o `.ini` movido de lado e restaurado — conferir md5 depois)**: probe compara
  `config_file` após `load_settings` com `load("res://menu/settings.gd").new().DEFAULTS` — **só
  possível antes do `git rm`** (ou usando a cópia em `../oxide_godot_origins/menu/settings.gd`:
  `load("res://…")` não alcança; copiar temporariamente para o scratch não resolve `res://` —
  então rodar este probe **antes do passo 6 do ciclo**, ou aceitar o resultado já medido em
  research §E.3b, 15/15).
- Os `WARNING … leaked at exit` do probe (objetos criados pelo próprio script) não contam.

## 6. Checkpoint do usuário (validação visual — plan §"Validação visual")

Boot → menu com o modo de janela salvo; Settings reflete o `.ini` atual; Apply aplica e persiste
(reabrir; reiniciar); Cancel descarta; apagar o `.ini` → padrões e o arquivo só volta após Apply;
`Shadow mapping` off → sem sombras; **SSAO Disabled → desligado de fato** (diferença intencional
para o original: upstream-bugs #3); SSAO Medium/High, SSIL 3 opções, bloom, névoa como no
original; F11 e Alt+Enter alternam tela cheia; editor: `settings.tscn` raiz `Settings` sem script,
Autoload apontando para a cena, nenhum `.gd` no FileSystem. Marcos A–D continuam iguais.
Divergência → commit `Fix port settings.gd …`.

## 7. Commit (autor the repository author; `git status`/`git diff --cached` antes; nunca parcial)

Arquivos: `src/settings.rs` (novo), `src/lib.rs`, `oxide-godot/menu/settings.tscn` (nova),
`oxide-godot/project.godot`, deleções de `menu/settings.gd` + `.uid`, `docs/upstream-bugs.md`,
`docs/v2-backlog.md`. Nada mais (nenhum `.rs` existente além de `lib.rs`; `CLAUDE.md` intocado).

Assunto: `Port settings.gd → Settings (Node) autoload via menu/settings.tscn; project.godot: autoload script→cena (upstream bug fix: SSAO Disabled)`

Corpo: vínculo do autoload por cena mínima (classe nativa não pode ser autoload direta; única
edição do `project.godot` na v1); `#[var] config_file`/`metalfx_supported`; `DEFAULTS` como
dicionário aninhado iterado (ordem do `.ini` preservada); 6 `#[constant]` dos enums; `init`
manual; `apply_graphics_settings` linha a linha com `get_window()` próprio e quirk "média aplica
alta" preservados; **correção conservadora**: SSAO `if`→`else if` (settings.gd:90) com comentário
`// upstream bug fix` — `docs/upstream-bugs.md` #3; backlog v2 item 25; **v1: zero `.gd` no
projeto**; consumidores intocados (backlog 1).

## 8. Verificação final da v1 (após o commit, antes do OK final)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*'; find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'   # ambos vazios
git log --oneline 6f4d9ba..HEAD | grep -c '^[0-9a-f]* Port '                 # 1
git diff --stat 6f4d9ba -- '*.rs'                                            # só lib.rs (+1) e settings.rs (novo)
git diff --stat 6f4d9ba -- oxide-godot/project.godot                         # 1 +-  (1 insertion, 1 deletion)
grep -n '^Settings=' oxide-godot/project.godot                               # res://menu/settings.tscn
ls oxide_godot_core/oxide_godot_lib/src/ | wc -l                             # 16
grep -c '^| [0-9]' docs/upstream-bugs.md; grep -c '^| [0-9]' docs/v2-backlog.md   # 3; 25
cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'                 # 0
```

Com o OK do usuário no checkpoint: a v1 é **declarada concluída** (Princípio I). Tag e branch
`v2` são decisões separadas do usuário.
