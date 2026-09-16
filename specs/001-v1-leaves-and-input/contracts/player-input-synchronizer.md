# Contract: `PlayerInputSynchronizer` (port 5)

Public surface consumed by code that **remains in GDScript** and by the scene. Everything here is
seen by Godot with a name and type identical to the original `player_input.gd`. The implementer must
check this table against `player.gd` and `player.tscn` before the commit (Principle II,
"Preservation of property names").

- Registered class: `PlayerInputSynchronizer` (name is **mandatory** — `player.gd:26`
  `@onready var player_input: PlayerInputSynchronizer = $InputSynchronizer`)
- Base: `MultiplayerSynchronizer`
- Node in the scene: `player.tscn` → `InputSynchronizer` (l.343)

## Properties (all `#[export]`, with getter and setter)

| Name | Godot type | Consumer | Replicated |
|---|---|---|---|
| `aiming` | `bool` | `player.gd:121` | ✅ `player.tscn:51` |
| `shoot_target` | `Vector3` | `player.gd:135` | ✅ `player.tscn:42` |
| `motion` | `Vector2` | `player.gd:87` | ✅ `player.tscn:45` |
| `shooting` | `bool` | `player.gd:133` | ✅ `player.tscn:48` |
| `jumping` | `bool` | `player.gd:107` (reads), `player.gd:114` (writes `false`) | ❌ |
| `camera_animation` | `AnimationPlayer` | `player.tscn:346` (`node_paths`) | — |
| `crosshair` | `TextureRect` | `player.tscn:347` | — |
| `camera_base` | `Node3D` | `player.tscn:348` | — |
| `camera_rot` | `Node3D` | `player.tscn:349` | — |
| `camera_camera` | `Camera3D` | `player.tscn:350`; `player.gd:211` (`.add_trauma`) | — |
| `color_rect` | `ColorRect` | `player.tscn:351` | — |

## Methods (`#[func]`)

| Godot signature | Rust | Consumer |
|---|---|---|
| `get_aim_rotation() -> float` | `fn get_aim_rotation(&self) -> f64` | `player.gd:73` |
| `get_camera_base_quaternion() -> Quaternion` | `fn get_camera_base_quaternion(&self) -> Quaternion` | `player.gd:124` |
| `get_camera_rotation_basis() -> Basis` | `fn get_camera_rotation_basis(&self) -> Basis` | `player.gd:89` |

## RPC

| Name | Config (= `@rpc("call_local")`) | Rust | Triggered by |
|---|---|---|---|
| `jump` | `authority`, `call_local`, `unreliable`, channel 0 | `#[rpc(authority, call_local, unreliable)] fn jump(&mut self)` | the node itself, in `process`: `self.base_mut().rpc("jump", &[])` |

## Implemented virtuals

`_ready`, `_process(delta)`, `_input(event)` — via `impl IMultiplayerSynchronizer`.

## Base API used in `player.gd` (not part of this class's contract, but confirms nothing else is needed)

- `player.gd:41` `$InputSynchronizer.set_multiplayer_authority(value)` — method of `Node`.

## Verification before the commit

```bash
cd oxide-godot
grep -n 'player_input\.\|PlayerInputSynchronizer\|\$InputSynchronizer' player/player.gd
grep -n 'InputSynchronizer' player/player.tscn
```
Every name that appears must be present in the tables above.
