# Contract: `EnemyRobot` (port 2)

Surface consumed by `level.gd` (remains in GDScript), by the bullet (Rust, duck typing) and
by the `red_robot.tscn` scene.

- Registered class: `EnemyRobot` (free name — no `class_name`; no script references the
  type; `level.gd:97` types it as `CharacterBody3D`). Checked: there is no `EnemyRobot` in the engine.
- Base: `CharacterBody3D`.
- Node in the scene: root of `enemies/red_robot/red_robot.tscn` (l.10584 before port 1; −1 after).
  Instantiated by `level.gd:97-100` (`EnemyRobot.instantiate()`, `robot.transform = ...`,
  `robot.exploded.connect(_respawn_robot.bind(spawn_point))`, `spawned_nodes.add_child(robot, true)`)
  and replicated by the level's `MultiplayerSpawner`.

## Signal

| Godot signature | Rust | Consumer |
|---|---|---|
| `signal exploded()` | `#[signal] fn exploded();` (main `#[godot_api] impl EnemyRobot` block); emission `self.signals().exploded().emit()` | `level.gd:99` — connection by name; `_respawn_robot` waits 15 s and spawns another robot |

## Methods and RPCs

| Godot signature | Rust | Consumer | Effect |
|---|---|---|---|
| `hit() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn hit(&mut self)` | `Bullet` (`collider.has_method("hit")` → `collider.rpc("hit")`) | damage/death (`red_robot.gd:78-105`) |
| `play_shoot() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn play_shoot(&mut self)` | internal (`rpc("play_shoot")`) | `ShootAnimation.play("shoot")` |
| `shoot_check() -> void` | `#[func] fn shoot_check(&mut self)` | method track of the "shoot" animation, t = 2.25 s (`red_robot.tscn:10296`) | `test_shoot = true` |
| `resume_approach() -> void` | `#[func] fn resume_approach(&mut self)` | method track t = 3 s (`red_robot.tscn:10299`); internal | APPROACH, `aim_preparing = 0.5`, `shoot_countdown = 6` |
| `_on_area_body_entered(body: Node3D)` | `#[func] fn _on_area_body_entered(&mut self, body: Gd<Node3D>)` | `red_robot.tscn:11050` (`PlayerDetectionArea.body_entered`) | `player = body`, APPROACH if `Player` or `name == "Target"` |
| `_on_area_body_exited(body: Node3D)` | `#[func] fn _on_area_body_exited(&mut self, body: Gd<Node3D>)` | `red_robot.tscn:11051` (`body_exited`) | `player = null`, IDLE if `Player` |

Internal methods (private, no `#[func]`, same names): `shoot`, `animate`, `_clip_ray`.
Checked: no `.gd` or `.tscn` references them (`grep -rn 'shoot()\|animate(\|_clip_ray' --include=*.gd --include=*.tscn oxide-godot`
only finds `red_robot.gd`).

## Exported properties

| Name | Godot type | Rust | Default | Replication (`red_robot.tscn`) |
|---|---|---|---|---|
| `test_shoot` | `bool` | `#[export] test_shoot: bool` | false | no |
| `target_position` | `Vector3` | `#[export] target_position: Vector3` | (0,0,0) | l.39 `.:target_position` |
| `health` | `int` | `#[export] health: i32` | 5 | l.33 `.:health` |
| `state` | `int`/enum `State` | `#[export] state: State` | `Idle` (0) | l.36 `.:state` |
| `dead` | `bool` | `#[export] dead: bool` | false | l.42 `.:dead` |
| `aim_preparing` | `float` | `#[export] aim_preparing: f32` | 0.5 | no |

`.:global_transform` (l.30) is base. No export is written on the scene root (l.10584-10587
only has `collision_layer/mask` and `script`).

## What the robot consumes from the Rust classes

`Player::add_camera_shake_trauma` (→ `pub(crate)` in `player.rs`, visibility only, in the robot's
commit); `Part::explode` (`pub(crate)` since port 1); `Blast` only through base API (`Node3D`).

## Implemented virtuals

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## Verification before the commit

```bash
cd oxide-godot
grep -n 'exploded\|EnemyRobot' level/level.gd                                   # l.6, 97, 99 — signal by name
grep -n '"method": &"shoot_check"\|"method": &"resume_approach"' enemies/red_robot/red_robot.tscn   # 2 lines
grep -n 'method="_on_area_body_entered"\|method="_on_area_body_exited"' enemies/red_robot/red_robot.tscn   # 2 lines
grep -n 'properties/[0-9]/path' enemies/red_robot/red_robot.tscn | head -5   # global_transform, health, state, target_position, dead
grep -c 'type="EnemyRobot"' enemies/red_robot/red_robot.tscn                   # 1
grep -c 'ExtResource("1")' enemies/red_robot/red_robot.tscn                  # 0
grep -rn 'has_method("hit")' ../oxide_godot_core/oxide_godot_lib/src/bullet.rs   # 1 — duck typing that calls hit
```
