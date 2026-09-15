# Data Model: Marco A — folhas e input do jogador (v1 raw port)

Não há persistência. As "entidades" são o estado em memória de dois nodes cujo estado é lido por
código que permanece em GDScript (`player.gd`) ou replicado pela cena (`player.tscn`). Os outros
três nodes (`DebugLabel`, `PartDisappear`, `Blast`) têm estado puramente interno e efêmero e
não expõem nada — listados no fim por completude.

## 1. PlayerInputSynchronizer (node `InputSynchronizer` em `player.tscn`)

### Estado replicado / lido pelo jogador

| Campo | Tipo Godot | Tipo Rust | Default | Escrito por | Lido por | Replicado (`player.tscn`) |
|---|---|---|---|---|---|---|
| `aiming` | `bool` | `bool` | `false` | `process` (transição de mira) | `player.gd:121` | ✅ `properties/5` |
| `shoot_target` | `Vector3` | `Vector3` | `(0,0,0)` | `process` (enquanto `shooting`) | `player.gd:135` | ✅ `properties/2` |
| `motion` | `Vector2` | `Vector2` | `(0,0)` | `process` (todo frame) | `player.gd:87` | ✅ `properties/3` |
| `shooting` | `bool` | `bool` | `false` | `process` (todo frame) | `player.gd:133` | ✅ `properties/4` |
| `jumping` | `bool` | `bool` | `false` | RPC `jump` (→ `true`) | `player.gd:107` (lê), `:114` (escreve `false`) | ❌ (por RPC, como no original) |

Todos `#[export]` (getter + setter gerados) com o nome exato acima.

### Referências preenchidas pela cena (`node_paths`, `player.tscn:343,346–351`)

| Campo | Tipo Godot | Tipo Rust | NodePath na cena |
|---|---|---|---|
| `camera_animation` | `AnimationPlayer` | `Option<Gd<AnimationPlayer>>` | `../CameraBase/Animation` |
| `crosshair` | `TextureRect` | `Option<Gd<TextureRect>>` | `../Crosshair` |
| `camera_base` | `Node3D` | `Option<Gd<Node3D>>` | `../CameraBase` |
| `camera_rot` | `Node3D` | `Option<Gd<Node3D>>` | `../CameraBase/CameraRot` |
| `camera_camera` | `Camera3D` | `Option<Gd<Camera3D>>` | `../CameraBase/CameraRot/SpringArm3D/Camera3D` (é o `CameraNoiseShake` após o port 4) |
| `color_rect` | `ColorRect` | `Option<Gd<ColorRect>>` | `../ColorRect` |

`player.gd:211` lê `camera_camera` e chama `add_trauma` nele — a referência precisa continuar
exposta com esse nome.

### Estado interno (não exposto)

| Campo | Tipo Rust | Default | Semântica |
|---|---|---|---|
| `toggled_aim` | `bool` | `false` | Mira ligada por toque curto (≤ 0,4 s) |
| `aiming_timer` | `f32` | `0.0` | Segundos acumulados com a mira ativa; zera quando inativa |

### Constantes

`CAMERA_CONTROLLER_ROTATION_SPEED = 3.0`, `CAMERA_MOUSE_ROTATION_SPEED = 0.001`,
`CAMERA_X_ROT_MIN = (-89.9°).to_radians()`, `CAMERA_X_ROT_MAX = (70°).to_radians()`,
`AIM_HOLD_THRESHOLD = 0.4` — todas `f32` (alimentam `Vector2`/rotação).

### Máquina de estados da mira (por frame, em `process`)

```
entrada: just_released(aim), pressed(aim), just_pressed(aim), aiming_timer, toggled_aim
current_aim =
  se just_released(aim) e aiming_timer ≤ 0.4:  true; toggled_aim = true          (toque curto → toggle ON)
  senão:                                        toggled_aim ou pressed(aim);
                                                se just_pressed(aim): toggled_aim = false   (novo toque → toggle OFF)
aiming_timer = current_aim ? aiming_timer + delta : 0
se aiming ≠ current_aim: aiming = current_aim; camera_animation.play(aiming ? "shoot" : "far")
```

### Invariantes

- `camera_rot.rotation.x ∈ [CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX]` após qualquer `rotate_camera`.
- `get_aim_rotation() ∈ [-1, 1]`.
- `color_rect.modulate.a ∈ [0, 1]`: `= min((-17 - y)/15, 1)` se `y < -17`, senão `*= (1 - 4·delta)`.
- Só a autoridade multiplayer processa `process`/`input`; nos demais peers o node fica inerte e
  `color_rect` oculto (decisão tomada uma vez, em `ready`).

## 2. CameraNoiseShake (node `Camera3D` em `player.tscn`)

### Interface

| Método | Assinatura Godot | Chamado por |
|---|---|---|
| `add_trauma` | `add_trauma(amount: float) -> void` | `player.gd:211` (0.35 ao atirar, 0.75 ao ser atingido; 13.0 via `red_robot.gd:133` → `player.gd:210`) |

### Estado interno

| Campo | Tipo Rust | Inicialização | Semântica |
|---|---|---|---|
| `trauma` | `f32` | `0.0` | Intensidade acumulada, `∈ [0, 1.2]` |
| `time` | `f64` | `0.0` | Posição no ruído; `+= delta · 1.0 · 5000` por frame com trauma |
| `start_rotation` | `Vector3` | `rotation` em `ready` | Rotação de repouso; base para o tremor (capturada uma vez — quirk preservado) |
| `noise` | `Gd<FastNoiseLite>` | `new_gd()`; em `ready`: `seed`, `fractal_octaves = 1`, `fractal_lacunarity = 1.0` | Gerador 1D |
| `noise_seed` | `i32` | `randi() as i32` | Semente; `+1`/`+2` para pitch/roll |

### Constantes

`SPEED = 1.0`, `DECAY_RATE = 1.5`, `MAX_YAW = 0.05`, `MAX_PITCH = 0.05`, `MAX_ROLL = 0.1`,
`MAX_TRAUMA = 1.2` (`f32`).

### Transições (por frame, em `process`, só se `trauma > 0`)

```
trauma = max(trauma - 1.5·delta, 0)
time  += delta · 5000
shake  = trauma²
rotation = start_rotation + Vector3(0.05·shake·noise(seed+1, time),   // pitch (x)
                                     0.05·shake·noise(seed,   time),   // yaw   (y)
                                     0.10·shake·noise(seed+2, time))   // roll  (z)
```

`add_trauma(a)`: `trauma = min(trauma + a, 1.2)`.

### Invariantes

- `trauma ∈ [0, 1.2]` sempre.
- No frame em que `trauma` chega a 0, `rotation == start_rotation` (shake = 0) e nos frames
  seguintes a câmera não é tocada.

## 3. Nodes sem estado exposto

| Classe | Estado interno | Ciclo de vida |
|---|---|---|
| `DebugLabel` | nenhum (usa `visible`/`text` do próprio `Label`) | vive com `level.tscn` |
| `PartDisappear` | `mini_blasts: OnReady<Gd<CpuParticles3D>>` | `ready` → +0,2 s emite → +2·lifetime `queue_free` |
| `Blast` | `light_rays`, `animation_player` (`OnReady`), `camera: Option<Gd<Camera3D>>` | `ready` → `animation_finished` → `queue_free` |
