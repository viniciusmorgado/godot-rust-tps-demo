# Contrato: `RedRobot` (port 2)

Superfície consumida por `level.gd` (permanece em GDScript), pela bala (Rust, duck typing) e
pela cena `red_robot.tscn`.

- Classe registrada: `RedRobot` (nome livre — sem `class_name`; nenhum script referencia o
  tipo; `level.gd:97` tipa como `CharacterBody3D`). Conferido: não existe `RedRobot` no engine.
- Base: `CharacterBody3D`.
- Node na cena: raiz de `enemies/red_robot/red_robot.tscn` (l.10584 antes do port 1; −1 depois).
  Instanciado por `level.gd:97-100` (`RedRobot.instantiate()`, `robot.transform = ...`,
  `robot.exploded.connect(_respawn_robot.bind(spawn_point))`, `spawned_nodes.add_child(robot, true)`)
  e replicado pelo `MultiplayerSpawner` do level.

## Sinal

| Assinatura Godot | Rust | Consumidor |
|---|---|---|
| `signal exploded()` | `#[signal] fn exploded();` (bloco `#[godot_api] impl RedRobot` principal); emissão `self.signals().exploded().emit()` | `level.gd:99` — conexão por nome; `_respawn_robot` espera 15 s e spawna outro robô |

## Métodos e RPCs

| Assinatura Godot | Rust | Consumidor | Efeito |
|---|---|---|---|
| `hit() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn hit(&mut self)` | `Bullet` (`collider.has_method("hit")` → `collider.rpc("hit")`) | dano/morte (`red_robot.gd:78-105`) |
| `play_shoot() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn play_shoot(&mut self)` | interno (`rpc("play_shoot")`) | `ShootAnimation.play("shoot")` |
| `shoot_check() -> void` | `#[func] fn shoot_check(&mut self)` | method track da animação "shoot", t = 2,25 s (`red_robot.tscn:10296`) | `test_shoot = true` |
| `resume_approach() -> void` | `#[func] fn resume_approach(&mut self)` | method track t = 3 s (`red_robot.tscn:10299`); interno | APPROACH, `aim_preparing = 0,5`, `shoot_countdown = 6` |
| `_on_area_body_entered(body: Node3D)` | `#[func] fn _on_area_body_entered(&mut self, body: Gd<Node3D>)` | `red_robot.tscn:11050` (`PlayerDetectionArea.body_entered`) | `player = body`, APPROACH se `Player` ou `name == "Target"` |
| `_on_area_body_exited(body: Node3D)` | `#[func] fn _on_area_body_exited(&mut self, body: Gd<Node3D>)` | `red_robot.tscn:11051` (`body_exited`) | `player = null`, IDLE se `Player` |

Métodos internos (privados, sem `#[func]`, mesmos nomes): `shoot`, `animate`, `_clip_ray`.
Conferido: nenhum `.gd` ou `.tscn` os referencia (`grep -rn 'shoot()\|animate(\|_clip_ray' --include=*.gd --include=*.tscn oxide-godot`
só encontra `red_robot.gd`).

## Propriedades exportadas

| Nome | Tipo Godot | Rust | Default | Replicação (`red_robot.tscn`) |
|---|---|---|---|---|
| `test_shoot` | `bool` | `#[export] test_shoot: bool` | false | não |
| `target_position` | `Vector3` | `#[export] target_position: Vector3` | (0,0,0) | l.39 `.:target_position` |
| `health` | `int` | `#[export] health: i32` | 5 | l.33 `.:health` |
| `state` | `int`/enum `State` | `#[export] state: State` | `Idle` (0) | l.36 `.:state` |
| `dead` | `bool` | `#[export] dead: bool` | false | l.42 `.:dead` |
| `aim_preparing` | `float` | `#[export] aim_preparing: f32` | 0,5 | não |

`.:global_transform` (l.30) é base. Nenhum export está gravado na raiz da cena (l.10584-10587
só tem `collision_layer/mask` e `script`).

## O que o robô consome das classes Rust

`Player::add_camera_shake_trauma` (→ `pub(crate)` em `player.rs`, só visibilidade, no commit do
robô); `Part::explode` (`pub(crate)` desde o port 1); `Blast` só por API base (`Node3D`).

## Virtuais implementados

`_ready`, `_physics_process(delta)` — via `impl ICharacterBody3D`.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'exploded\|RedRobot' level/level.gd                                   # l.6, 97, 99 — sinal por nome
grep -n '"method": &"shoot_check"\|"method": &"resume_approach"' enemies/red_robot/red_robot.tscn   # 2 linhas
grep -n 'method="_on_area_body_entered"\|method="_on_area_body_exited"' enemies/red_robot/red_robot.tscn   # 2 linhas
grep -n 'properties/[0-9]/path' enemies/red_robot/red_robot.tscn | head -5   # global_transform, health, state, target_position, dead
grep -c 'type="RedRobot"' enemies/red_robot/red_robot.tscn                   # 1
grep -c 'ExtResource("1")' enemies/red_robot/red_robot.tscn                  # 0
grep -rn 'has_method("hit")' ../oxide_godot_core/oxide_godot_lib/src/bullet.rs   # 1 — duck typing que chama hit
```
