# Contrato: `Player` (port 1)

Superfície pública consumida por código que **permanece em GDScript** e pelas cenas. Tudo aqui é
nome de contrato: copiar do original, nunca traduzir.

- Classe registrada: **`Player`** — nome OBRIGATÓRIO (`class_name Player`): `red_robot.gd:131,275,281`
  e `door.gd:10` fazem `body is Player`.
- Base: `CharacterBody3D`.
- Node na cena: raiz de `player/player.tscn` (l.333). Instanciado por `level.gd:117`
  (`PlayerScene.instantiate()`, tipado `CharacterBody3D`) e adicionado a `spawned_nodes`
  (`level.gd:121`) — o `MultiplayerSpawner` do level replica o spawn.

## Propriedades (nome + tipo Godot)

| Nome | Tipo Godot | Rust | Consumidor | Observação |
|---|---|---|---|---|
| `player_id` | `int` (default 1) | `#[export] #[var(set = set_player_id)] player_id: i32` | `level.gd:119` (`player.player_id = id`, **antes** de `add_child`); `player.tscn:20` (`.:player_id`, spawn) | Setter chama `set_multiplayer_authority(value)` em `InputSynchronizer` |
| `current_animation` | `int` / enum `Animations` (default `WALK` = 3) | `#[export] current_animation: Animations` | `player.tscn:29` (`.:current_animation`, por frame) | Valores 0..3 na mesma ordem do original |
| `motion` | `Vector2` | `#[var] motion: Vector2` | `player.tscn:26` (`.:motion`, por frame) | Não exportado, só registrado (`#[var]`) |
| `transform` | `Transform3D` | API base | `level.gd:120`; `player.tscn:17` (`.:transform`) | Base — nada a fazer |
| `name` | `StringName` | API base | `level.gd:118` (`player.name = str(id)`) | Base — nada a fazer |
| `PlayerModel:transform` | `Transform3D` | filho, API base | `player.tscn:23` | Base — nada a fazer |

## Métodos e RPCs (`#[rpc(authority, call_local, unreliable)]`)

| Assinatura Godot | Rust | Consumidor | Efeito |
|---|---|---|---|
| `jump() -> void` | `fn jump(&mut self)` | interno (`apply_input`, via `rpc("jump")`) | `animate(JUMP_UP)`; toca `SoundEffects/Jump` |
| `land() -> void` | `fn land(&mut self)` | interno (`rpc("land")`) | `animate(JUMP_DOWN)`; toca `SoundEffects/Land` |
| `shoot() -> void` | `fn shoot(&mut self)` | interno (`rpc("shoot")`) | `restart()` + `emitting` em `ShootParticle` e `MuzzleFlash`; `FireCooldown.start()`; toca `SoundEffects/Shoot`; `add_camera_shake_trauma(0.35)` |
| `hit() -> void` | `fn hit(&mut self)` | `bullet.gd:31-32` (`collider.has_method(&"hit")` → `collider.hit.rpc()`); depois do port 2, `Bullet` em Rust por `has_method`/`rpc` (duck typing) | `add_camera_shake_trauma(0.75)` |
| `add_camera_shake_trauma(amount: float) -> void` | `fn add_camera_shake_trauma(&mut self, amount: f64)` | `red_robot.gd:133` (`player.add_camera_shake_trauma(13.0)`) | `player_input.camera_camera` (cast `CameraNoiseShake`) `.add_trauma(amount)` |
| `set_player_id(value: int)` | `#[func] fn set_player_id(&mut self, value: i32)` | ninguém (é o setter registrado de `player_id`) | Exposto por exigência do gdext |

Métodos internos (privados, sem `#[func]`, nomes iguais ao GDScript): `animate(anim, _delta)`,
`apply_input(delta)`. Conferido: nenhum `.gd` chama `animate`/`apply_input` **do Player**
(`grep -rn 'animate(\|apply_input(' --include=*.gd oxide-godot` → `player.gd` e o `animate(delta)`
próprio de `red_robot.gd:69,136,176,185,241`, que é função local do robô).

## Virtuais implementados

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## O que o Player consome das classes Rust (visibilidade `pub(crate)`, research D1)

`PlayerInputSynchronizer`: campos `motion`, `aiming`, `shooting`, `shoot_target`, `jumping`
(leitura e escrita), `camera_camera`; métodos `get_aim_rotation`, `get_camera_base_quaternion`,
`get_camera_rotation_basis`. `CameraNoiseShake`: `add_trauma`.

## Nota de tipagem estática

`level.gd:117` tipa `player` como `CharacterBody3D` e acessa `player_id` dinamicamente
(`UNSAFE_PROPERTY_ACCESS` já hoje); `red_robot.gd:133` chama `add_camera_shake_trauma` num
`Player` resolvido por `is Player` — com a classe nativa registrada, o analisador resolve o tipo
`Player` e o método. Nenhum warning novo esperado.

## Verificação antes do commit

```bash
cd oxide-godot
grep -rn 'is Player\|player_id\|add_camera_shake_trauma\|\.hit\.rpc\|has_method(&"hit")' --include=*.gd .
```
Deve retornar apenas: `red_robot.gd:131,133,275,281`, `level.gd:119`, `bullet.gd:31,32` (até o
port 2), `door.gd:10` (até o port 3). Cada nome encontrado existe em `src/player.rs` com o mesmo
nome.

```bash
grep -n 'properties/[0-9]/path' player/player.tscn | sed -n 1,5p
```
Deve listar `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`,
`.:current_animation` — os três do script existem na classe Rust.
