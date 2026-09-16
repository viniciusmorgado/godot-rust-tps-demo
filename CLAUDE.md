# Oxide Godot — guia operacional

Porte do Godot TPS Demo (GDScript) para Rust via godot-rust/gdext. As regras que governam o
projeto estão em `.specify/memory/constitution.md` — este arquivo cobre apenas o *como* operar.

## Layout

```
oxide-godot/                 raiz do repo (git, Spec Kit, este arquivo)
├── oxide-godot/             projeto Godot 4.7 (TPS demo + extension.gdextension)
└── oxide_godot_core/        workspace Cargo; crate `oxide_godot` (cdylib) em oxide_godot_lib/
```

Referência intocada do original em GDScript: `../oxide_godot_origins/` (fora do repo).

## Toolchain

- Godot 4.7.2 stable: `/usr/bin/godot.x86_64` (não existe `godot` no PATH — usar o caminho completo).
- Rust estável (cargo/rustc 1.98). `godot = "0.5.5"` declarado em `[workspace.dependencies]`;
  membros herdam com `{ workspace = true }`. Não fixar feature `api-4-x` nem mudar a versão do crate:
  o template é gerado por ferramenta externa que busca sempre a última estável.
- `Cargo.lock` fica fora do git (decisão do template).

## Ciclo de trabalho

1. Editar Rust em `oxide_godot_core/oxide_godot_lib/src/`.
2. `cd oxide_godot_core && cargo build` — obrigatório após **toda** alteração em Rust; é o build de
   debug que substitui `target/debug/liboxide_godot.so`, referenciado pelo `.gdextension`
   (`reloadable = true`, o editor aberto faz hot-reload).
3. Vincular a classe Rust ao node editando a `.tscn` como texto (equivale ao "Change Type" do editor):
   - na linha `[node name="X" type="Base" ...]`, trocar `type="Base"` por `type="ClasseRust"`;
   - remover a linha `script = ExtResource("N")` desse node;
   - remover o `[ext_resource type="Script" ... path="res://.../x.gd"]` que ficou órfão;
   - apagar o `.gd` e o `.gd.uid` correspondentes.
4. Validar headless (sempre a partir de `oxide-godot/oxide-godot/`):
   - reimportar/verificar extensão: `/usr/bin/godot.x86_64 --headless --import --path .`
     (a linha `Initialize godot-rust (...)` confirma que a lib carregou);
   - rodar uma cena: `/usr/bin/godot.x86_64 --headless --path . caminho/cena.tscn`.
   Erros pré-existentes do demo upstream no import (`Cannon_Charge already exists`,
   `doorsimple_d.png` ausente, `surfaces.is_empty()`) são conhecidos e não são regressão.

## Convenções de porte (v1)

- Uma classe Rust por script `.gd`, com a **mesma base** do script (`CharacterBody3D`,
  `MultiplayerSynchronizer`, `Camera3D`, ...). `class_name Player` → `struct Player`.
- O nome de uma classe registrada NÃO pode coincidir com nenhum identificador de script dos `.gd`
  que ainda existem (`const`, `class_name`, `var` de topo) nem com classe do engine: o GDScript
  rejeita o script inteiro ("The member X shadows a native class"). Conferir antes de nomear:
  `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | sort -u`
  (caso real: `RedRobot` colidia com `const RedRobot` em `level.gd:6` → classe virou `EnemyRobot`).
- Nomes de métodos expostos com `#[func]` iguais aos do GDScript (`_on_door_body_entered`,
  `hit`, `explode`, ...) para as `[connection]` das `.tscn`, `has_method()` e `.rpc()` continuarem válidos.
- `@export var` → `#[export] var`; `@onready var x = $Path` → `#[init(node = "Path")] x: OnReady<Gd<T>>`;
  `signal x` → `#[signal] fn x()`.
- Tradução direta, sem refatoração/abstração (Princípio I da constituição). Melhorias percebidas
  vão para o backlog da v2, nunca para o código da v1.

## Notas de API (gdext 0.5.5)

- `get_tree()` retorna `Gd<SceneTree>` direto (sem `Option`).
- Acesso ao node base: `self.base()` / `self.base_mut()`.
