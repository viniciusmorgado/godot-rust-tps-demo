# Contract: `CameraNoiseShake` (port 4)

Public surface consumed by code that **remains in GDScript**.

- Registered class: `CameraNoiseShake` (free name — the original has no `class_name`; no
  script references the type)
- Base: `Camera3D`
- Node in the scene: `player.tscn` → `CameraBase/CameraRot/SpringArm3D/Camera3D` (l.630). It is the node
  pointed to by `camera_camera` of the `InputSynchronizer` (`player.tscn:350`), and is therefore reached
  by GDScript as `player_input.camera_camera`.

## Methods (`#[func]`)

| Godot signature | Rust | Consumer | Values used |
|---|---|---|---|
| `add_trauma(amount: float) -> void` | `fn add_trauma(&mut self, amount: f64)` | `player.gd:211` (`player_input.camera_camera.add_trauma(amount)`) | 0.35 (`player.gd:201`, shooting), 0.75 (`:206`, hit), 13.0 (`red_robot.gd:133` → `player.gd:210`) |

## Exposed properties

None. `trauma`, `time`, `start_rotation`, `noise`, `noise_seed` are internal; no script
reads them (checked: `grep -rn 'trauma\|noise_seed\|start_rotation' oxide-godot --include=*.gd` only
finds `camera_noise_shake_effect.gd` itself and `add_camera_shake_trauma`/`add_trauma`).

## Implemented virtuals

`_ready`, `_process(delta)` — via `impl ICamera3D`.

## Static typing note

`player_input.camera_camera` is typed `Camera3D` for the GDScript analyzer, which does not know
`add_trauma` on that type → warning `UNSAFE_METHOD_ACCESS`, exactly as today (the original script
also has no `class_name`). Not a regression.

## Verification before the commit

```bash
cd oxide-godot
grep -rn 'add_trauma\|add_camera_shake_trauma' --include=*.gd .
```
Must return only `player.gd:201,206,210,211` and `red_robot.gd:133`.
