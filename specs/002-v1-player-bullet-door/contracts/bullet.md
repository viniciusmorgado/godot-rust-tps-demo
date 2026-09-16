# Contract: `Bullet` (port 2)

Surface consumed by the `bullet.tscn` scene and by the `Player` (base API only).

- Registered class: `Bullet` (free name — no `class_name` in the original; no script
  references the type). Verified: no `Bullet` class exists in the engine (`out/classes/`).
- Base: `CharacterBody3D`.
- Node in the scene: root of `player/bullet/bullet.tscn` (l.481). Instantiated by
  `Player::apply_input` (`load` + `instantiate_as::<CharacterBody3D>`) and by `player.tscn:679`
  (`BulletCache`, `visible = false`).

## Methods and RPCs

| Godot signature | Rust | Consumer | Effect |
|---|---|---|---|
| `explode() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn explode(&mut self)` | internal (`rpc("explode")` on expiry / collision) | `AnimationPlayer.play("explode")`; if `Settings.config_file.get_value("rendering", "shadow_mapping")` → `OmniLight3D.shadow_enabled = true` |
| `destroy() -> void` | `#[func] fn destroy(&mut self)` | **method track** of the "explode" animation: `bullet.tscn:93-105` (`tracks/1/type = "method"`, `"method": &"destroy"`, t = 1.5 s) | not server → return; server → `queue_free()` |

Dynamic call that the bullet **makes** (duck typing from the original, preserved — FR-025):
`collider.has_method("hit")` → `collider.rpc("hit", &[])` on the collided `Node3D`
(`Player.hit` or `red_robot.gd` `hit`).

## Properties

None registered. `time_alive` and `hit` are internal (no script reads them — verified:
`grep -rn 'time_alive' --include=*.gd oxide-godot` → only `bullet.gd`).

Replication (`bullet.tscn:12-14`): `.:global_transform` (spawn + per frame) — base property of
`Node3D`, nothing to do.

## What the Player uses from the bullet (base API of `CharacterBody3D`/`Node3D`/`Node`)

`set_global_position`, `look_at`, `add_collision_exception_with`, `add_child` (on the parent) — no
method of the `Bullet` class; that is why the Player types the instance as `CharacterBody3D` and port 1
does not depend on port 2.

## Implemented virtuals

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## Verification before the commit

```bash
cd oxide-godot
grep -n '"method": &"destroy"\|tracks/1/type' player/bullet/bullet.tscn   # l.93 and l.104
grep -n 'properties/0/path' player/bullet/bullet.tscn                     # l.12: .:global_transform
grep -rn 'explode\|destroy' --include=*.gd .                              # only bullet.gd (until the port); empty afterwards
```
