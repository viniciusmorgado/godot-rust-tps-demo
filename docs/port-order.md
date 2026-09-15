# Ordem de porte GDScript → Rust (v1)

Análise do grafo de dependências entre os 15 scripts do TPS demo e a ordem de migração
derivada dele. Insumo para as specs da v1. Regras que governam o porte estão na constituição
(`.specify/memory/constitution.md`); detalhes operacionais no `CLAUDE.md`.

## Grafo de dependências

`A → B` significa "A usa API customizada de B" (método, propriedade, enum ou `is B`).
Chamadas que usam apenas API base do Godot (`look_at`, `add_child`, `global_transform`, ...)
não contam: funcionam igual com o node em GDScript ou em Rust.

```
main ──────────────────┬──→ Settings (config_file)
  │ (has_signal, duck)  │
  ├──→ menu ────────────┤──→ Settings (config_file, apply_graphics_settings, enums GIType/GIQuality — ~100 refs)
  └──→ level ───────────┤──→ Settings (config_file, enums)
         ├──→ red_robot (signal exploded)
         │      ├──→ Player (is Player, add_camera_shake_trauma)
         │      ├──→ part (explode)
         │      │      └──→ part_disappear (só API base)
         │      └──→ blast (só API base)
         ├──→ Player (player_id)
         │      ├──→ PlayerInputSynchronizer (motion/aiming/shooting/jumping/shoot_target,
         │      │      get_aim_rotation, get_camera_rotation_basis, get_camera_base_quaternion, camera_camera)
         │      ├──→ camera_noise_shake_effect (add_trauma)
         │      └──→ bullet (só API base)
         │             ├──→ Settings (config_file)
         │             └──→ red_robot / Player (hit via has_method — duck, dinâmico já no original)
         └──→ debug (nada)
door ──→ Player (is Player)
flying_forklift ──→ Settings (config_file)
```

Folhas (não dependem de ninguém): `debug`, `part_disappear`, `blast`,
`camera_noise_shake_effect`, `player_input`, `part`, `settings`.

## Regra que define a ordem

- **GDScript → Rust** funciona sem esforço: `is ClasseRust`, chamar `#[func]`, ler `#[var]`,
  conectar `#[signal]`, `.rpc()` — tudo dinâmico e nativo.
- **Rust → GDScript** exige chamadas dinâmicas (`.call("x", &[..])`, `.get("y")`, comparar
  `get_script()` no lugar de `is Player`). Funciona, mas é código sem tipo que seria descartado.

Portanto o porte é **de baixo para cima**: dependências antes dos dependentes, de modo que o código
Rust nasça tipado e nunca precise chamar API customizada de GDScript.

### Exceção: o autoload `Settings`

`Settings` é folha, mas é autoload com ~100 referências no `menu.gd` e acesso a enums
(`Settings.GIType.SDFGI`). Portado cedo, `menu.gd`/`level.gd` em GDScript quebram (enum de script
não existe em node nativo). Portado por último, nenhum GDScript depende mais dele.

Custo: consumidores em Rust (forklift, bullet, level, menu, main) acessam `Settings` dinamicamente
(`get_node("/root/Settings").get("config_file")` → a partir daí `Gd<ConfigFile>` tipado; enums
viram constantes inteiras locais). Aceito na v1; **backlog v2: acesso tipado ao Settings**.
Em Rust, `Settings` vira uma cena `menu/settings.tscn` com root do tipo `Settings`, registrada
no autoload do `project.godot`.

## Restrições verificadas nas cenas

- `MultiplayerSynchronizer` replica propriedades dos scripts — precisam existir como
  `#[var]`/`#[export]` com o mesmo nome:
  - `InputSynchronizer`: `aiming`, `motion`, `shooting`, `shoot_target`
  - `Player`: `current_animation`, `player_id`
  - `red_robot`: `target_position`, `health`, `state`, `dead`
  - `part`: `fade_value`
  - `bullet`: só transforms (API base)
- `[connection]` nas cenas (nomes preservados via `#[func]`): `door._on_door_body_entered`,
  `red_robot._on_area_body_entered` / `_on_area_body_exited`, 11 handlers em `menu.tscn`.
- Scripts attached em nodes internos de outra cena (a troca de tipo acontece na cena-mãe):
  `part.gd` em 3 nodes de `red_robot.tscn` (PartShield1, PartShield2, PartHead);
  `player_input.gd` e `camera_noise_shake_effect.gd` em `player.tscn`;
  `debug.gd` em `level.tscn`.

## (1) Scripts que podem vir primeiro

Sem dependência de outro script e chamados por outros apenas via API base ou dinâmica.
Cada um é testável logo após o port, com o resto ainda em GDScript.

| # | Script | Base | Linhas | Como testar | O que introduz |
|---|---|---|---|---|---|
| 1 | `level/debug.gd` | `Label` | 15 | rodar level, F3 alterna overlay de FPS | `process`, `Input`, `Engine`/`OS` |
| 2 | `enemies/red_robot/parts/part_disappear_effect/part_disappear.gd` | `CPUParticles3D` | 9 | matar robô, partes somem com puff | `await create_timer` → timer + signal |
| 3 | `enemies/red_robot/laser/impact_effect/blast.gd` | `Node3D` | 15 | robô atira, impacto anima e some | `await animation_finished`, `get_camera_3d` |
| 4 | `player/camera_noise_shake_effect.gd` | `Camera3D` | 61 | atirar, câmera treme | `FastNoiseLite`, `#[func] add_trauma` chamado do GDScript |
| 5 | `player/player_input.gd` | `MultiplayerSynchronizer` | 142 | mover câmera, mirar (toggle/hold), pular | `#[export]` replicados, `#[rpc(call_local)]`, `_input`, raycast, `OnReady` via `node_paths` |

## (2) Ordem completa de migração

| Ordem | Script | Base | Cena onde troca o tipo | Depende de (já em Rust) | Observações |
|---|---|---|---|---|---|
| 1 | `level/debug.gd` | Label | `level.tscn` | — | |
| 2 | `part_disappear.gd` | CPUParticles3D | `part_disappear.tscn` | — | |
| 3 | `blast.gd` | Node3D | `impact_effect.tscn` | — | |
| 4 | `camera_noise_shake_effect.gd` | Camera3D | `player.tscn` (Camera3D) | — | |
| 5 | `player_input.gd` | MultiplayerSynchronizer | `player.tscn` (InputSynchronizer) | — | `class_name PlayerInputSynchronizer`; Player em GDScript chama dinamicamente |
| 6 | `player/player.gd` | CharacterBody3D | `player.tscn` (root) | 5, 4 | `class_name Player`; `player_id` com setter; `#[rpc]` jump/land/shoot/hit; instancia bullet (base) |
| 7 | `player/bullet/bullet.gd` | CharacterBody3D | `bullet.tscn` | Settings (dinâmico) | `has_method("hit")` + `rpc("hit")` dinâmico, como o original |
| 8 | `door/door.gd` | Area3D | `door.tscn` | 6 | `try_cast::<Player>()` |
| 9 | `enemies/red_robot/parts/part.gd` | RigidBody3D | `red_robot.tscn` (3 nodes) | 2 | `fade_value` setter mexe em shader; red_robot GDScript chama `explode()` dinamicamente |
| 10 | `enemies/red_robot/red_robot.gd` | CharacterBody3D | `red_robot.tscn` (root) | 6, 9, 3 | maior script (283 l); state machine, `#[signal] exploded`, `#[rpc] hit/play_shoot`, `await` → timers |
| 11 | `level/forklift/flying_forklift.gd` | Node3D | `flying_forklift.tscn` | Settings (dinâmico) | trivial; cabe em qualquer ponto após 1 |
| 12 | `level/level.gd` | Node3D | `level.tscn` (root) | 10, 6, Settings (dinâmico) | spawn tipado `Gd<RedRobot>`/`Gd<Player>`; `#[signal] quit`; `_input`; sinais de multiplayer peer |
| 13 | `menu/menu.gd` | Node | `menu.tscn` | Settings (dinâmico) | 460 l, mecânico: ~50 `OnReady` + 11 `#[func]` handlers; `#[signal] replace_main_scene`; `load_threaded` |
| 14 | `main/main.gd` | Node | `main.tscn` | 13, 12 (signals) | `has_signal` duck → `try_cast` nos dois tipos Rust |
| 15 | `menu/settings.gd` | Node (autoload) | nova `menu/settings.tscn` + `project.godot` | — | último: nenhum GDScript resta; enums viram `#[constant]` |

Após o 15 não resta nenhum `.gd` — critério de conclusão da v1.

## Marcos sugeridos (agrupamento para specs)

- **A** — 1 a 5: folhas e input.
- **B** — 6 a 8: player completo (player, bullet, door).
- **C** — 9 a 10: inimigo (part, red_robot).
- **D** — 11 a 15: level, menu, main, settings — jogo inteiro em Rust.

## Backlog v2 identificado nesta análise

Registrado em `docs/v2-backlog.md` (itens 1–3).

- Acesso tipado ao autoload `Settings` (substituir `get_node("/root/Settings").get(...)`).
- Substituir `has_method("hit")` + rpc dinâmico do `bullet` por um trait/enum de alvo.
- Substituir `has_signal` duck typing do `main` por um gestor de cenas.
