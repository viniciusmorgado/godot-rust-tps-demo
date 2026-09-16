# Data Model: Milestone B — player, bullet and door

**Phase**: v1 — Raw Port. State of each class as it exists in GDScript; nothing is remodeled.
Godot types (as GDScript and the scenes see them) and Rust types (gdext 0.5.5) side by side. The
signatures and decisions are in [research.md](research.md); the names consumed by other
scripts/scenes, in [contracts/](contracts/).

## Player (`player/player.gd` → `src/player.rs`, base `CharacterBody3D`)

### Contract state (visible to Godot — name is contract)

| Name | Godot type | Rust type | Registration | Default | Who uses it |
|---|---|---|---|---|---|
| `player_id` | `int` | `i32` | `#[export] #[var(set = set_player_id)]` | `1` | `level.gd:119` (set outside the tree); `player.tscn:20` (replicated on spawn) |
| `current_animation` | `int` (enum `Animations`) | `Animations` | `#[export]` | `Walk` (3) | `player.tscn:29` (replicated per frame); clients animate from it |
| `motion` | `Vector2` | `Vector2` | `#[var]` | `(0, 0)` | `player.tscn:26` (replicated per frame) |

Enum `Animations` (`#[godot(via = i64)]`): `JumpUp = 0`, `JumpDown = 1`, `Strafe = 2`, `Walk = 3`
— same order/values as `enum Animations { JUMP_UP, JUMP_DOWN, STRAFE, WALK }`.

Setter rule (`player_id`): store the value **and** call
`InputSynchronizer.set_multiplayer_authority(value)`; must work with the node outside the tree
(research D2).

### Internal state (private — free names, kept identical to GDScript)

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `airborne_time` | `f32` | `100.0` | Time in the air; `> 0.5` on landing triggers `land`; `> 0.1` ⇒ `on_air`. Quirk: initial 100 ⇒ first landing triggers `land` |
| `orientation` | `Transform3D` | identity → in `ready`, global of `PlayerModel` with zero origin | Model rotation; receives slerp and root motion; origin zeroed and basis orthonormalized every frame |
| `root_motion` | `Transform3D` | identity | `(get_root_motion_rotation, get_root_motion_position)` of the `AnimationTree` in the frame |
| `initial_position` | `Vector3` | `transform.origin` in `ready` | Respawn point (y < −40) |

Constants (`f32`): `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`,
`MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`.

### Scene references (`OnReady<Gd<T>>`, research D5)

`player_input: PlayerInputSynchronizer` (`InputSynchronizer`), `animation_tree: AnimationTree`,
`player_model: Node3D` (`PlayerModel`), `shoot_from: Marker3D`
(`PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom`), `crosshair: TextureRect` (unused,
as in the original), `fire_cooldown: Timer` (`FireCooldown`: 0.4 s, one-shot, autostart),
`sound_effect_jump/land/shoot: AudioStreamPlayer` (`SoundEffects/Jump|Land|Shoot`).
`ShootParticle` and `MuzzleFlash` (`CpuParticles3D`, children of `ShootFrom`) are looked up inside
`shoot()`.

### Relationships

- **Consumes (typed)**: `PlayerInputSynchronizer` — reads `motion`, `aiming`, `shooting`,
  `shoot_target`, `jumping`, `camera_camera`; writes `jumping = false`; calls
  `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`.
  `CameraNoiseShake` — calls `add_trauma(amount)` (via `camera_camera` with `cast`).
- **Produces**: instances of `bullet.tscn` typed as `CharacterBody3D` (base API; never as
  `Bullet`), children of the player's **parent**.
- **Is consumed by**: `red_robot.gd` (`is Player`, `add_camera_shake_trauma(13.0)`), `level.gd`
  (`name`, `player_id`, `transform`), `bullet` (`has_method("hit")` + `rpc("hit")`), `door`
  (`is Player`).

### Animation transitions (`animate`)

| Condition (in `apply_input`) | `current_animation` | `AnimationTree` |
|---|---|---|
| `on_air && velocity.y > 0` | `JumpUp` | `state/transition_request = "jump_up"` |
| `on_air && velocity.y ≤ 0` | `JumpDown` | `state/transition_request = "jump_down"` |
| `!on_air && aiming` | `Strafe` | `"strafe"`; `aim/add_amount = get_aim_rotation()`; `strafe/blend_position = (motion.x, −motion.y)` |
| `!on_air && !aiming` | `Walk` | `aim/add_amount = 0`; `"walk"`; `walk/blend_position = (|motion|, 0)` |
| RPC `jump` | `JumpUp` | + Jump sound |
| RPC `land` | `JumpDown` | + Land sound |

### RPCs (`authority`, `call_local`, `unreliable`)

`jump()`, `land()`, `shoot()`, `hit()`, `add_camera_shake_trauma(amount: float)` — effects in
[contracts/player.md](contracts/player.md).

## Bullet (`player/bullet/bullet.gd` → `src/bullet.rs`, base `CharacterBody3D`)

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `time_alive` | `f32` | `5.0` | Decrements by `delta`; `< 0` ⇒ `hit = true` + RPC `explode` |
| `hit` | `bool` | `false` | When `true`, `physics_process` returns immediately |

Constant: `BULLET_VELOCITY: f32 = 20.0`. References: `animation_player`
(`AnimationPlayer`), `collision_shape` (`CollisionShape3D`), `omni_light` (`OmniLight3D`).

Cycle per physics frame (server): `hit` ⇒ return · `time_alive -= delta` · expires ⇒ explode ·
`move_and_collide(−delta × 20 × basis.z)` · collided ⇒ (`has_method("hit")` ⇒ `rpc("hit")`) ·
disables collision · RPC `explode` · `hit = true`. Preserved quirk: expiring and colliding in the same
frame produces two `explode`.

Contract: RPC `explode()` (plays "explode"; light shadow if `Settings.config_file`
`rendering/shadow_mapping`); method `destroy()` (method track of "explode", `bullet.tscn:104`;
only the server does `queue_free`). Replication: `.:global_transform` (`bullet.tscn:12`).
Single dynamic dependency: `/root/Settings` → `config_file: ConfigFile` (research D14).

Instantiated by: `Player::apply_input` (shot) and `player.tscn:679` (`BulletCache`, invisible,
pre-warming).

## Door (`door/door.gd` → `src/door.rs`, base `Area3D`)

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `open` | `bool` | `false` | Latch: only the first `Player` opens; never closes |

Reference: `animation_player` (`DoorModel2/AnimationPlayer` — **upstream bug fix**;
the original references `DoorModel/AnimationPlayer`, a non-existent node; research D15).

Contract: `_on_door_body_entered(body: Node3D)` (`body_entered` connection in `door.tscn:37`):
`!open && body is Player` ⇒ plays "doorsimple_opening", `open = true`; otherwise nothing.
`door.tscn` is orphan (no scene instantiates it).

## Upstream bug register (`docs/upstream-bugs.md` — created in the port 3 commit)

A table with one row per fix: `#`, defect (error text), script/scene/lines
(`door.gd:6` vs `door.tscn:13`), fix applied (`DoorModel2/AnimationPlayer`; comment
`// upstream bug fix` in `src/door.rs`), commit (port 3 hash, filled in when committing — see
[quickstart.md](quickstart.md) §"Port 3").
