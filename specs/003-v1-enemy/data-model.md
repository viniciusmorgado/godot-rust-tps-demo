# Data Model: Milestone C — part and red robot

**Phase**: v1 — Raw Port. State of each class as it exists in the GDScript; nothing is remodeled.
Signatures and decisions in [research.md](research.md); names consumed by other scripts/scenes
in [contracts/](contracts/).

## Part (`enemies/red_robot/parts/part.gd` → `src/part.rs`, base `RigidBody3D`)

No scene of its own: 3 instances in `enemies/red_robot/red_robot.tscn` (`Death/PartShield1`,
`Death/PartShield2`, `Death/PartHead`), each with children `MultiplayerSynchronizer`
(`public_visibility = false`), `Model` (instance of the model; child 0 = `MeshInstance3D`), `Col1`,
`Col2` (`CollisionShape3D`, `disabled = true`), and `freeze = true`.

### Contract state (name is contract)

| Name | Godot type | Rust | Registration | Default | Who uses it |
|---|---|---|---|---|---|
| `lifetime` | `float` | `f32` | `#[export]` | 3.0 | inspector (not written in the scene) |
| `lifetime_random` | `float` | `f32` | `#[export]` | 3.0 | same |
| `disappearing_time` | `float` | `f32` | `#[export]` | 0.5 | same |
| `fade_value` | `float` | `f32` | `#[export] #[var(set = set_fade_value)]` | 0.0 | `red_robot.tscn:10419` (`.:fade_value`, replicated); setter applies `emission_cutout` to the shader |
| `explode()` | method | `#[func] pub(crate)` | — | — | `red_robot.gd:96-98` (by name, until port 2); `EnemyRobot` (typed, port 2) |
| `destroy()` | RPC `call_local` | `#[rpc(authority, call_local, unreliable)]` | — | — | internal (`rpc("destroy")`) |

### Internal state

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `_mat` | `Option<Gd<Material>>` | `None` | Own copy of the surface 0 material (with `next_pass` copied); `None` on a dedicated server or before `ready` |
| `_disappearing_counter` | `f32` | 0.0 | Fade counter; `fade_value = (counter / disappearing_time)²` |

### Life cycle

```
[scene]  freeze=true, process off ──ready──▶ material duplicated (not dedicated)
   │
   ▼ explode()  (robot dies)
 public sync, freeze=false ──not server──▶ (end)
   │ server
   ▼ Col1/Col2 on, v=(0,3,0), random ω×10, waits lifetime + lifetime_random·rand (3–6 s)
   ▼ process on: fade 0→1 in 0.5 s (²); at counter ≥ 0.3 s → rpc destroy, process off
   ▼ destroy: puff (PartDisappear, base API) on the part's parent, at the part's position; 0.2 s → queue_free
```

### Relations

- **Consumes**: `part_disappear.tscn` (`PartDisappear`, Milestone A) through base API (`CpuParticles3D`).
- **Is consumed by**: `red_robot.gd` (`explode()` by name) → `EnemyRobot` (typed, port 2).

## EnemyRobot (`enemies/red_robot/red_robot.gd` → `src/red_robot.rs`, base `CharacterBody3D`)

### Contract state

| Name | Godot type | Rust | Registration | Default | Who uses it |
|---|---|---|---|---|---|
| `test_shoot` | `bool` | `bool` | `#[export]` | false | `shoot_check()` (method track) → `physics_process` |
| `target_position` | `Vector3` | `Vector3` | `#[export]` | (0,0,0) | `red_robot.tscn:39` (replicated) |
| `health` | `int` | `i32` | `#[export]` | 5 | `red_robot.tscn:33` (replicated) |
| `state` | `int` (enum `State`) | `State` | `#[export]` | `Idle` (0) | `red_robot.tscn:36` (replicated) |
| `dead` | `bool` | `bool` | `#[export]` | false | `red_robot.tscn:42` (replicated) |
| `aim_preparing` | `float` | `f32` | `#[export]` | 0.5 | not replicated (quirk) |
| `exploded` | signal | `#[signal]` | — | — | `level.gd:99` (`robot.exploded.connect(...)`) |
| `hit()` | RPC | `#[rpc(authority, call_local, unreliable)]` | — | — | `Bullet` (`has_method("hit")` → `rpc("hit")`) |
| `play_shoot()` | RPC | `#[rpc(authority, call_local, unreliable)]` | — | — | internal |
| `shoot_check()`, `resume_approach()` | methods | `#[func]` | — | — | method tracks of the "shoot" animation (`red_robot.tscn:10296-10299`) |
| `_on_area_body_entered(body)`, `_on_area_body_exited(body)` | methods | `#[func]` | — | — | connections `red_robot.tscn:11050-11051` |
| `transform` | `Transform3D` | base | — | — | `level.gd:98`; `red_robot.tscn:30` (`.:global_transform`) |

Enum `State` (`#[godot(via = i64)]`): `Idle = 0`, `Approach = 1`, `Aim = 2`, `Shooting = 3`.

### Internal state

| Field | Rust type | Initial | Role |
|---|---|---|---|
| `shoot_countdown` | `f32` | 6.0 | Time facing the player before trying to aim |
| `aim_countdown` | `f32` | 1.0 | Time in AIM before confirming the shot |
| `player` | `Option<Gd<Node3D>>` | `None` | Detected body (Player or "Target") |
| `orientation` | `Transform3D` | global with zero origin (in `ready`) | Robot rotation; receives root motion; written to `global_basis` |

Constants (`f32`): `PLAYER_AIM_TOLERANCE_DEGREES` = 15° in rad, `SHOOT_WAIT` 6.0, `AIM_TIME` 1.0,
`AIM_PREPARE_TIME` 0.5, `BLEND_AIM_SPEED` 0.05.

### Scene references (15 `OnReady`, research D8)

`animation_tree`, `shoot_animation`, `model`, `ray_from`, `ray_mesh`, `laser_raycast`,
`collision_shape`, `explosion_sound`, `hit_sound`, `death`, `death_shield1`/`2`/`death_head`
(**`Gd<Part>`**), `death_detach_spark1`/`2`. `LaserEmber` looked up in `shoot()`.

### State machine (server, `physics_process`)

```
IDLE ──body_entered(Player|"Target")──▶ APPROACH ──body_exited(Player)──▶ IDLE
APPROACH: turns/walks toward the player; facing (±15°) decrements shoot_countdown (6 s);
          < 0 → raycast to the player+UP: hits → AIM (aim_countdown=1, aim_preparing=0)
                                           otherwise → shoot_countdown = 6
AIM:      laser clipped; aim_preparing ↑ up to 0.5; aim_countdown < 0 → raycast:
          hits → SHOOTING (shoot_countdown=6, rpc play_shoot → anim "shoot")
          otherwise → resume_approach() (APPROACH, aim_preparing=0.5, shoot_countdown=6)
SHOOTING: laser clipped; method tracks of the animation: shoot_check (2.25 s) → test_shoot →
          shoot() on the next frame; resume_approach (3 s) → APPROACH
any:      hit() ×5 → dead (parts explode, sparks, sound, exploded signal; server removes in 10 s)
```

`animate(delta)`: `state/transition_request` ∈ {`turn_left`, `turn_right`, `walk`, `idle`};
with a target: `aiming/blend_amount = clamp(aim_preparing/0.5)`, `aim/blend_position` incremented
by the h/v angles (degrees) of the target+UP in the `RayMesh` space, clamped to [−1, 1].

### Relations

- **Consumes (typed)**: `Player` (`add_camera_shake_trauma(13.0)` after `try_cast`, with a delay of
  0.1 s), `Part` (`explode()` ×3).
- **Consumes (base API)**: `impact_effect.tscn` (`Blast`) as `Node3D`.
- **Is consumed by**: `level.gd` (`exploded`, `transform`), `Bullet` (`hit` by duck typing).
