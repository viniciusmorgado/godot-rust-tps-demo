# Contrato: `Bullet` (port 2)

Superfície consumida pela cena `bullet.tscn` e pelo `Player` (só API base).

- Classe registrada: `Bullet` (nome livre — sem `class_name` no original; nenhum script
  referencia o tipo). Conferido: não existe classe `Bullet` no engine (`out/classes/`).
- Base: `CharacterBody3D`.
- Node na cena: raiz de `player/bullet/bullet.tscn` (l.481). Instanciada por
  `Player::apply_input` (`load` + `instantiate_as::<CharacterBody3D>`) e por `player.tscn:679`
  (`BulletCache`, `visible = false`).

## Métodos e RPCs

| Assinatura Godot | Rust | Consumidor | Efeito |
|---|---|---|---|
| `explode() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn explode(&mut self)` | interno (`rpc("explode")` ao expirar / colidir) | `AnimationPlayer.play("explode")`; se `Settings.config_file.get_value("rendering", "shadow_mapping")` → `OmniLight3D.shadow_enabled = true` |
| `destroy() -> void` | `#[func] fn destroy(&mut self)` | **method track** da animação "explode": `bullet.tscn:93-105` (`tracks/1/type = "method"`, `"method": &"destroy"`, t = 1,5 s) | não servidor → return; servidor → `queue_free()` |

Chamada dinâmica que a bala **faz** (duck typing do original, preservado — FR-025):
`collider.has_method("hit")` → `collider.rpc("hit", &[])` sobre o `Node3D` colidido
(`Player.hit` ou `red_robot.gd` `hit`).

## Propriedades

Nenhuma registrada. `time_alive` e `hit` são internos (nenhum script os lê — conferido:
`grep -rn 'time_alive' --include=*.gd oxide-godot` → só `bullet.gd`).

Replicação (`bullet.tscn:12-14`): `.:global_transform` (spawn + por frame) — propriedade base de
`Node3D`, nada a fazer.

## O que o Player usa da bala (API base de `CharacterBody3D`/`Node3D`/`Node`)

`set_global_position`, `look_at`, `add_collision_exception_with`, `add_child` (no pai) — nenhum
método da classe `Bullet`; por isso o Player tipa a instância como `CharacterBody3D` e o port 1
não depende do port 2.

## Virtuais implementados

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n '"method": &"destroy"\|tracks/1/type' player/bullet/bullet.tscn   # l.93 e l.104
grep -n 'properties/0/path' player/bullet/bullet.tscn                     # l.12: .:global_transform
grep -rn 'explode\|destroy' --include=*.gd .                              # só bullet.gd (até o port); vazio depois
```
