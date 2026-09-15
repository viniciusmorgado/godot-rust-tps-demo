# Data Model: Marco B — jogador, bala e porta

**Fase**: v1 — Raw Port. Estado de cada classe tal como existe no GDScript; nada é remodelado.
Tipos Godot (como o GDScript e as cenas os veem) e tipos Rust (gdext 0.5.5) lado a lado. As
assinaturas e decisões estão em [research.md](research.md); os nomes consumidos por outros
scripts/cenas, em [contracts/](contracts/).

## Player (`player/player.gd` → `src/player.rs`, base `CharacterBody3D`)

### Estado de contrato (visível ao Godot — nome é contrato)

| Nome | Tipo Godot | Tipo Rust | Registro | Default | Quem usa |
|---|---|---|---|---|---|
| `player_id` | `int` | `i32` | `#[export] #[var(set = set_player_id)]` | `1` | `level.gd:119` (set fora da árvore); `player.tscn:20` (replicado no spawn) |
| `current_animation` | `int` (enum `Animations`) | `Animations` | `#[export]` | `Walk` (3) | `player.tscn:29` (replicado por frame); clientes animam a partir dele |
| `motion` | `Vector2` | `Vector2` | `#[var]` | `(0, 0)` | `player.tscn:26` (replicado por frame) |

Enum `Animations` (`#[godot(via = i64)]`): `JumpUp = 0`, `JumpDown = 1`, `Strafe = 2`, `Walk = 3`
— mesma ordem/valores do `enum Animations { JUMP_UP, JUMP_DOWN, STRAFE, WALK }`.

Regra do setter (`player_id`): guardar o valor **e** chamar
`InputSynchronizer.set_multiplayer_authority(value)`; deve funcionar com o node fora da árvore
(research D2).

### Estado interno (privado — nomes livres, mantidos iguais ao GDScript)

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `airborne_time` | `f32` | `100.0` | Tempo no ar; `> 0.5` no pouso dispara `land`; `> 0.1` ⇒ `on_air`. Quirk: 100 inicial ⇒ primeiro pouso dispara `land` |
| `orientation` | `Transform3D` | identidade → em `ready`, global do `PlayerModel` com origem zero | Rotação do modelo; recebe slerp e root motion; origem zerada e base ortonormalizada a cada frame |
| `root_motion` | `Transform3D` | identidade | `(get_root_motion_rotation, get_root_motion_position)` do `AnimationTree` no frame |
| `initial_position` | `Vector3` | `transform.origin` em `ready` | Ponto de respawn (y < −40) |

Constantes (`f32`): `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`,
`MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`.

### Referências de cena (`OnReady<Gd<T>>`, research D5)

`player_input: PlayerInputSynchronizer` (`InputSynchronizer`), `animation_tree: AnimationTree`,
`player_model: Node3D` (`PlayerModel`), `shoot_from: Marker3D`
(`PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom`), `crosshair: TextureRect` (não usado,
como no original), `fire_cooldown: Timer` (`FireCooldown`: 0,4 s, one-shot, autostart),
`sound_effect_jump/land/shoot: AudioStreamPlayer` (`SoundEffects/Jump|Land|Shoot`).
`ShootParticle` e `MuzzleFlash` (`CpuParticles3D`, filhos de `ShootFrom`) são buscados dentro de
`shoot()`.

### Relações

- **Consome (tipado)**: `PlayerInputSynchronizer` — lê `motion`, `aiming`, `shooting`,
  `shoot_target`, `jumping`, `camera_camera`; escreve `jumping = false`; chama
  `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`.
  `CameraNoiseShake` — chama `add_trauma(amount)` (via `camera_camera` com `cast`).
- **Produz**: instâncias de `bullet.tscn` tipadas como `CharacterBody3D` (API base; nunca como
  `Bullet`), filhas do **pai** do jogador.
- **É consumido por**: `red_robot.gd` (`is Player`, `add_camera_shake_trauma(13.0)`), `level.gd`
  (`name`, `player_id`, `transform`), `bullet` (`has_method("hit")` + `rpc("hit")`), `door`
  (`is Player`).

### Transições de animação (`animate`)

| Condição (em `apply_input`) | `current_animation` | `AnimationTree` |
|---|---|---|
| `on_air && velocity.y > 0` | `JumpUp` | `state/transition_request = "jump_up"` |
| `on_air && velocity.y ≤ 0` | `JumpDown` | `state/transition_request = "jump_down"` |
| `!on_air && aiming` | `Strafe` | `"strafe"`; `aim/add_amount = get_aim_rotation()`; `strafe/blend_position = (motion.x, −motion.y)` |
| `!on_air && !aiming` | `Walk` | `aim/add_amount = 0`; `"walk"`; `walk/blend_position = (|motion|, 0)` |
| RPC `jump` | `JumpUp` | + som Jump |
| RPC `land` | `JumpDown` | + som Land |

### RPCs (`authority`, `call_local`, `unreliable`)

`jump()`, `land()`, `shoot()`, `hit()`, `add_camera_shake_trauma(amount: float)` — efeitos em
[contracts/player.md](contracts/player.md).

## Bullet (`player/bullet/bullet.gd` → `src/bullet.rs`, base `CharacterBody3D`)

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `time_alive` | `f32` | `5.0` | Decrementa por `delta`; `< 0` ⇒ `hit = true` + RPC `explode` |
| `hit` | `bool` | `false` | Quando `true`, `physics_process` retorna imediatamente |

Constante: `BULLET_VELOCITY: f32 = 20.0`. Referências: `animation_player`
(`AnimationPlayer`), `collision_shape` (`CollisionShape3D`), `omni_light` (`OmniLight3D`).

Ciclo por frame de física (servidor): `hit` ⇒ return · `time_alive -= delta` · expira ⇒ explode ·
`move_and_collide(−delta × 20 × basis.z)` · colidiu ⇒ (`has_method("hit")` ⇒ `rpc("hit")`) ·
desabilita colisão · RPC `explode` · `hit = true`. Quirk preservado: expirar e colidir no mesmo
frame gera dois `explode`.

Contrato: RPC `explode()` (toca "explode"; sombra da luz se `Settings.config_file`
`rendering/shadow_mapping`); método `destroy()` (method track de "explode", `bullet.tscn:104`;
só o servidor faz `queue_free`). Replicação: `.:global_transform` (`bullet.tscn:12`).
Dependência dinâmica única: `/root/Settings` → `config_file: ConfigFile` (research D14).

Instanciada por: `Player::apply_input` (tiro) e `player.tscn:679` (`BulletCache`, invisível,
pré-aquecimento).

## Door (`door/door.gd` → `src/door.rs`, base `Area3D`)

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `open` | `bool` | `false` | Trava: só o primeiro `Player` abre; nunca fecha |

Referência: `animation_player` (`DoorModel2/AnimationPlayer` — **correção do bug do upstream**;
o original referencia `DoorModel/AnimationPlayer`, node inexistente; research D15).

Contrato: `_on_door_body_entered(body: Node3D)` (conexão `body_entered` em `door.tscn:37`):
`!open && body is Player` ⇒ toca "doorsimple_opening", `open = true`; senão nada.
`door.tscn` é órfã (nenhuma cena a instancia).

## Registro de bugs do upstream (`docs/upstream-bugs.md` — criado no commit do port 3)

Uma tabela com uma linha por correção: `#`, defeito (texto do erro), script/cena/linhas
(`door.gd:6` vs `door.tscn:13`), correção aplicada (`DoorModel2/AnimationPlayer`; comentário
`// upstream bug fix` em `src/door.rs`), commit (hash do port 3, preenchido ao commitar — ver
[quickstart.md](quickstart.md) §"Port 3").
