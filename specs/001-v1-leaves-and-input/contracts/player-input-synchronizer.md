# Contrato: `PlayerInputSynchronizer` (port 5)

Superfície pública consumida por código que **permanece em GDScript** e pela cena. Tudo aqui é
visto pelo Godot com nome e tipo idênticos ao `player_input.gd` original. O implementador deve
conferir esta tabela contra `player.gd` e `player.tscn` antes do commit (Princípio II,
"Preservação de nomes de propriedades").

- Classe registrada: `PlayerInputSynchronizer` (nome **obrigatório** — `player.gd:26`
  `@onready var player_input: PlayerInputSynchronizer = $InputSynchronizer`)
- Base: `MultiplayerSynchronizer`
- Node na cena: `player.tscn` → `InputSynchronizer` (l.343)

## Propriedades (todas `#[export]`, com getter e setter)

| Nome | Tipo Godot | Consumidor | Replicada |
|---|---|---|---|
| `aiming` | `bool` | `player.gd:121` | ✅ `player.tscn:51` |
| `shoot_target` | `Vector3` | `player.gd:135` | ✅ `player.tscn:42` |
| `motion` | `Vector2` | `player.gd:87` | ✅ `player.tscn:45` |
| `shooting` | `bool` | `player.gd:133` | ✅ `player.tscn:48` |
| `jumping` | `bool` | `player.gd:107` (lê), `player.gd:114` (escreve `false`) | ❌ |
| `camera_animation` | `AnimationPlayer` | `player.tscn:346` (`node_paths`) | — |
| `crosshair` | `TextureRect` | `player.tscn:347` | — |
| `camera_base` | `Node3D` | `player.tscn:348` | — |
| `camera_rot` | `Node3D` | `player.tscn:349` | — |
| `camera_camera` | `Camera3D` | `player.tscn:350`; `player.gd:211` (`.add_trauma`) | — |
| `color_rect` | `ColorRect` | `player.tscn:351` | — |

## Métodos (`#[func]`)

| Assinatura Godot | Rust | Consumidor |
|---|---|---|
| `get_aim_rotation() -> float` | `fn get_aim_rotation(&self) -> f64` | `player.gd:73` |
| `get_camera_base_quaternion() -> Quaternion` | `fn get_camera_base_quaternion(&self) -> Quaternion` | `player.gd:124` |
| `get_camera_rotation_basis() -> Basis` | `fn get_camera_rotation_basis(&self) -> Basis` | `player.gd:89` |

## RPC

| Nome | Config (= `@rpc("call_local")`) | Rust | Disparado por |
|---|---|---|---|
| `jump` | `authority`, `call_local`, `unreliable`, canal 0 | `#[rpc(authority, call_local, unreliable)] fn jump(&mut self)` | o próprio node, em `process`: `self.base_mut().rpc("jump", &[])` |

## Virtuais implementados

`_ready`, `_process(delta)`, `_input(event)` — via `impl IMultiplayerSynchronizer`.

## API base usada em `player.gd` (não é contrato desta classe, mas confirma que nada mais é preciso)

- `player.gd:41` `$InputSynchronizer.set_multiplayer_authority(value)` — método de `Node`.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'player_input\.\|PlayerInputSynchronizer\|\$InputSynchronizer' player/player.gd
grep -n 'InputSynchronizer' player/player.tscn
```
Cada nome que aparecer deve constar nas tabelas acima.
