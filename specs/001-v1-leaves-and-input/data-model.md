# Data Model: Milestone A — leaves and player input (v1 raw port)

There is no persistence. The "entities" are the in-memory state of two nodes whose state is read by
code that remains in GDScript (`player.gd`) or replicated by the scene (`player.tscn`). The other
three nodes (`DebugLabel`, `PartDisappear`, `Blast`) have purely internal and ephemeral state and
expose nothing — listed at the end for completeness.

## 1. PlayerInputSynchronizer (node `InputSynchronizer` in `player.tscn`)

### State replicated / read by the player

| Field | Godot type | Rust type | Default | Written by | Read by | Replicated (`player.tscn`) |
|---|---|---|---|---|---|---|
| `aiming` | `bool` | `bool` | `false` | `process` (aim transition) | `player.gd:121` | ✅ `properties/5` |
| `shoot_target` | `Vector3` | `Vector3` | `(0,0,0)` | `process` (while `shooting`) | `player.gd:135` | ✅ `properties/2` |
| `motion` | `Vector2` | `Vector2` | `(0,0)` | `process` (every frame) | `player.gd:87` | ✅ `properties/3` |
| `shooting` | `bool` | `bool` | `false` | `process` (every frame) | `player.gd:133` | ✅ `properties/4` |
| `jumping` | `bool` | `bool` | `false` | RPC `jump` (→ `true`) | `player.gd:107` (reads), `:114` (writes `false`) | ❌ (via RPC, as in the original) |

All `#[export]` (generated getter + setter) with the exact name above.

### References filled in by the scene (`node_paths`, `player.tscn:343,346–351`)

| Field | Godot type | Rust type | NodePath in the scene |
|---|---|---|---|
| `camera_animation` | `AnimationPlayer` | `Option<Gd<AnimationPlayer>>` | `../CameraBase/Animation` |
| `crosshair` | `TextureRect` | `Option<Gd<TextureRect>>` | `../Crosshair` |
| `camera_base` | `Node3D` | `Option<Gd<Node3D>>` | `../CameraBase` |
| `camera_rot` | `Node3D` | `Option<Gd<Node3D>>` | `../CameraBase/CameraRot` |
| `camera_camera` | `Camera3D` | `Option<Gd<Camera3D>>` | `../CameraBase/CameraRot/SpringArm3D/Camera3D` (it is the `CameraNoiseShake` after port 4) |
| `color_rect` | `ColorRect` | `Option<Gd<ColorRect>>` | `../ColorRect` |

`player.gd:211` reads `camera_camera` and calls `add_trauma` on it — the reference must remain
exposed under that name.

### Internal state (not exposed)

| Field | Rust type | Default | Semantics |
|---|---|---|---|
| `toggled_aim` | `bool` | `false` | Aim turned on by a short tap (≤ 0.4 s) |
| `aiming_timer` | `f32` | `0.0` | Seconds accumulated with aim active; resets to zero when inactive |

### Constants

`CAMERA_CONTROLLER_ROTATION_SPEED = 3.0`, `CAMERA_MOUSE_ROTATION_SPEED = 0.001`,
`CAMERA_X_ROT_MIN = (-89.9°).to_radians()`, `CAMERA_X_ROT_MAX = (70°).to_radians()`,
`AIM_HOLD_THRESHOLD = 0.4` — all `f32` (they feed `Vector2`/rotation).

### Aim state machine (per frame, in `process`)

```
input: just_released(aim), pressed(aim), just_pressed(aim), aiming_timer, toggled_aim
current_aim =
  if just_released(aim) and aiming_timer ≤ 0.4:  true; toggled_aim = true          (short tap → toggle ON)
  else:                                          toggled_aim or pressed(aim);
                                                if just_pressed(aim): toggled_aim = false   (new tap → toggle OFF)
aiming_timer = current_aim ? aiming_timer + delta : 0
if aiming ≠ current_aim: aiming = current_aim; camera_animation.play(aiming ? "shoot" : "far")
```

### Invariants

- `camera_rot.rotation.x ∈ [CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX]` after any `rotate_camera`.
- `get_aim_rotation() ∈ [-1, 1]`.
- `color_rect.modulate.a ∈ [0, 1]`: `= min((-17 - y)/15, 1)` if `y < -17`, otherwise `*= (1 - 4·delta)`.
- Only the multiplayer authority runs `process`/`input`; on the other peers the node stays inert and
  `color_rect` hidden (decision taken once, in `ready`).

## 2. CameraNoiseShake (node `Camera3D` in `player.tscn`)

### Interface

| Method | Godot signature | Called by |
|---|---|---|
| `add_trauma` | `add_trauma(amount: float) -> void` | `player.gd:211` (0.35 when shooting, 0.75 when hit; 13.0 via `red_robot.gd:133` → `player.gd:210`) |

### Internal state

| Field | Rust type | Initialization | Semantics |
|---|---|---|---|
| `trauma` | `f32` | `0.0` | Accumulated intensity, `∈ [0, 1.2]` |
| `time` | `f64` | `0.0` | Position in the noise; `+= delta · 1.0 · 5000` per frame with trauma |
| `start_rotation` | `Vector3` | `rotation` in `ready` | Rest rotation; base for the shake (captured once — quirk preserved) |
| `noise` | `Gd<FastNoiseLite>` | `new_gd()`; in `ready`: `seed`, `fractal_octaves = 1`, `fractal_lacunarity = 1.0` | 1D generator |
| `noise_seed` | `i32` | `randi() as i32` | Seed; `+1`/`+2` for pitch/roll |

### Constants

`SPEED = 1.0`, `DECAY_RATE = 1.5`, `MAX_YAW = 0.05`, `MAX_PITCH = 0.05`, `MAX_ROLL = 0.1`,
`MAX_TRAUMA = 1.2` (`f32`).

### Transitions (per frame, in `process`, only if `trauma > 0`)

```
trauma = max(trauma - 1.5·delta, 0)
time  += delta · 5000
shake  = trauma²
rotation = start_rotation + Vector3(0.05·shake·noise(seed+1, time),   // pitch (x)
                                     0.05·shake·noise(seed,   time),   // yaw   (y)
                                     0.10·shake·noise(seed+2, time))   // roll  (z)
```

`add_trauma(a)`: `trauma = min(trauma + a, 1.2)`.

### Invariants

- `trauma ∈ [0, 1.2]` always.
- On the frame in which `trauma` reaches 0, `rotation == start_rotation` (shake = 0) and on the following
  frames the camera is not touched.

## 3. Nodes with no exposed state

| Class | Internal state | Lifecycle |
|---|---|---|
| `DebugLabel` | none (uses the `Label`'s own `visible`/`text`) | lives with `level.tscn` |
| `PartDisappear` | `mini_blasts: OnReady<Gd<CpuParticles3D>>` | `ready` → +0.2 s emits → +2·lifetime `queue_free` |
| `Blast` | `light_rays`, `animation_player` (`OnReady`), `camera: Option<Gd<Camera3D>>` | `ready` → `animation_finished` → `queue_free` |
