# Contract: `Player` (port 1)

Public surface consumed by code that **remains in GDScript** and by the scenes. Everything here is
a contract name: copy from the original, never translate.

- Registered class: **`Player`** — MANDATORY name (`class_name Player`): `red_robot.gd:131,275,281`
  and `door.gd:10` do `body is Player`.
- Base: `CharacterBody3D`.
- Node in the scene: root of `player/player.tscn` (l.333). Instantiated by `level.gd:117`
  (`PlayerScene.instantiate()`, typed `CharacterBody3D`) and added to `spawned_nodes`
  (`level.gd:121`) — the level's `MultiplayerSpawner` replicates the spawn.

## Properties (name + Godot type)

| Name | Godot type | Rust | Consumer | Note |
|---|---|---|---|---|
| `player_id` | `int` (default 1) | `#[export] #[var(set = set_player_id)] player_id: i32` | `level.gd:119` (`player.player_id = id`, **before** `add_child`); `player.tscn:20` (`.:player_id`, spawn) | Setter calls `set_multiplayer_authority(value)` on `InputSynchronizer` |
| `current_animation` | `int` / enum `Animations` (default `WALK` = 3) | `#[export] current_animation: Animations` | `player.tscn:29` (`.:current_animation`, per frame) | Values 0..3 in the same order as the original |
| `motion` | `Vector2` | `#[var] motion: Vector2` | `player.tscn:26` (`.:motion`, per frame) | Not exported, only registered (`#[var]`) |
| `transform` | `Transform3D` | base API | `level.gd:120`; `player.tscn:17` (`.:transform`) | Base — nothing to do |
| `name` | `StringName` | base API | `level.gd:118` (`player.name = str(id)`) | Base — nothing to do |
| `PlayerModel:transform` | `Transform3D` | child, base API | `player.tscn:23` | Base — nothing to do |

## Methods and RPCs (`#[rpc(authority, call_local, unreliable)]`)

| Godot signature | Rust | Consumer | Effect |
|---|---|---|---|
| `jump() -> void` | `fn jump(&mut self)` | internal (`apply_input`, via `rpc("jump")`) | `animate(JUMP_UP)`; plays `SoundEffects/Jump` |
| `land() -> void` | `fn land(&mut self)` | internal (`rpc("land")`) | `animate(JUMP_DOWN)`; plays `SoundEffects/Land` |
| `shoot() -> void` | `fn shoot(&mut self)` | internal (`rpc("shoot")`) | `restart()` + `emitting` on `ShootParticle` and `MuzzleFlash`; `FireCooldown.start()`; plays `SoundEffects/Shoot`; `add_camera_shake_trauma(0.35)` |
| `hit() -> void` | `fn hit(&mut self)` | `bullet.gd:31-32` (`collider.has_method(&"hit")` → `collider.hit.rpc()`); after port 2, `Bullet` in Rust via `has_method`/`rpc` (duck typing) | `add_camera_shake_trauma(0.75)` |
| `add_camera_shake_trauma(amount: float) -> void` | `fn add_camera_shake_trauma(&mut self, amount: f64)` | `red_robot.gd:133` (`player.add_camera_shake_trauma(13.0)`) | `player_input.camera_camera` (cast `CameraNoiseShake`) `.add_trauma(amount)` |
| `set_player_id(value: int)` | `#[func] fn set_player_id(&mut self, value: i32)` | nobody (it is the registered setter of `player_id`) | Exposed as required by gdext |

Internal methods (private, no `#[func]`, same names as GDScript): `animate(anim, _delta)`,
`apply_input(delta)`. Verified: no `.gd` calls `animate`/`apply_input` **of the Player**
(`grep -rn 'animate(\|apply_input(' --include=*.gd oxide-godot` → `player.gd` and the
`animate(delta)` of `red_robot.gd:69,136,176,185,241` itself, which is the robot's local function).

## Implemented virtuals

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## What the Player consumes from the Rust classes (`pub(crate)` visibility, research D1)

`PlayerInputSynchronizer`: fields `motion`, `aiming`, `shooting`, `shoot_target`, `jumping`
(read and write), `camera_camera`; methods `get_aim_rotation`, `get_camera_base_quaternion`,
`get_camera_rotation_basis`. `CameraNoiseShake`: `add_trauma`.

## Static typing note

`level.gd:117` types `player` as `CharacterBody3D` and accesses `player_id` dynamically
(`UNSAFE_PROPERTY_ACCESS` already today); `red_robot.gd:133` calls `add_camera_shake_trauma` on a
`Player` resolved via `is Player` — with the native class registered, the analyzer resolves the
`Player` type and the method. No new warning expected.

## Verification before the commit

```bash
cd oxide-godot
grep -rn 'is Player\|player_id\|add_camera_shake_trauma\|\.hit\.rpc\|has_method(&"hit")' --include=*.gd .
```
Must return only: `red_robot.gd:131,133,275,281`, `level.gd:119`, `bullet.gd:31,32` (until
port 2), `door.gd:10` (until port 3). Each name found exists in `src/player.rs` with the same
name.

```bash
grep -n 'properties/[0-9]/path' player/player.tscn | sed -n 1,5p
```
Must list `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`,
`.:current_animation` — the three from the script exist in the Rust class.
