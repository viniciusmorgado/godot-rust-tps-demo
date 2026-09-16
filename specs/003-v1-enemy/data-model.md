# Data Model: Marco C — peça e robô vermelho

**Fase**: v1 — Raw Port. Estado de cada classe tal como existe no GDScript; nada é remodelado.
Assinaturas e decisões em [research.md](research.md); nomes consumidos por outros scripts/cenas
em [contracts/](contracts/).

## Part (`enemies/red_robot/parts/part.gd` → `src/part.rs`, base `RigidBody3D`)

Sem cena própria: 3 instâncias em `enemies/red_robot/red_robot.tscn` (`Death/PartShield1`,
`Death/PartShield2`, `Death/PartHead`), cada uma com filhos `MultiplayerSynchronizer`
(`public_visibility = false`), `Model` (instância do modelo; filho 0 = `MeshInstance3D`), `Col1`,
`Col2` (`CollisionShape3D`, `disabled = true`), e `freeze = true`.

### Estado de contrato (nome é contrato)

| Nome | Tipo Godot | Rust | Registro | Default | Quem usa |
|---|---|---|---|---|---|
| `lifetime` | `float` | `f32` | `#[export]` | 3,0 | inspector (não gravado na cena) |
| `lifetime_random` | `float` | `f32` | `#[export]` | 3,0 | idem |
| `disappearing_time` | `float` | `f32` | `#[export]` | 0,5 | idem |
| `fade_value` | `float` | `f32` | `#[export] #[var(set = set_fade_value)]` | 0,0 | `red_robot.tscn:10419` (`.:fade_value`, replicado); setter aplica `emission_cutout` ao shader |
| `explode()` | método | `#[func] pub(crate)` | — | — | `red_robot.gd:96-98` (por nome, até o port 2); `RedRobot` (tipado, port 2) |
| `destroy()` | RPC `call_local` | `#[rpc(authority, call_local, unreliable)]` | — | — | interno (`rpc("destroy")`) |

### Estado interno

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `_mat` | `Option<Gd<Material>>` | `None` | Cópia própria do material da superfície 0 (com `next_pass` copiado); `None` em servidor dedicado ou antes de `ready` |
| `_disappearing_counter` | `f32` | 0,0 | Contador do fade; `fade_value = (contador / disappearing_time)²` |

### Ciclo de vida

```
[cena]  freeze=true, process off ──ready──▶ material duplicado (não dedicado)
   │
   ▼ explode()  (robô morre)
 sync pública, freeze=false ──não servidor──▶ (fim)
   │ servidor
   ▼ Col1/Col2 on, v=(0,3,0), ω aleatória×10, espera lifetime + lifetime_random·rand (3–6 s)
   ▼ process on: fade 0→1 em 0,5 s (²); em counter ≥ 0,3 s → rpc destroy, process off
   ▼ destroy: puff (PartDisappear, API base) no pai da peça, na posição da peça; 0,2 s → queue_free
```

### Relações

- **Consome**: `part_disappear.tscn` (`PartDisappear`, Marco A) por API base (`CpuParticles3D`).
- **É consumida por**: `red_robot.gd` (`explode()` por nome) → `RedRobot` (tipado, port 2).

## RedRobot (`enemies/red_robot/red_robot.gd` → `src/red_robot.rs`, base `CharacterBody3D`)

### Estado de contrato

| Nome | Tipo Godot | Rust | Registro | Default | Quem usa |
|---|---|---|---|---|---|
| `test_shoot` | `bool` | `bool` | `#[export]` | false | `shoot_check()` (method track) → `physics_process` |
| `target_position` | `Vector3` | `Vector3` | `#[export]` | (0,0,0) | `red_robot.tscn:39` (replicado) |
| `health` | `int` | `i32` | `#[export]` | 5 | `red_robot.tscn:33` (replicado) |
| `state` | `int` (enum `State`) | `State` | `#[export]` | `Idle` (0) | `red_robot.tscn:36` (replicado) |
| `dead` | `bool` | `bool` | `#[export]` | false | `red_robot.tscn:42` (replicado) |
| `aim_preparing` | `float` | `f32` | `#[export]` | 0,5 | não replicado (quirk) |
| `exploded` | sinal | `#[signal]` | — | — | `level.gd:99` (`robot.exploded.connect(...)`) |
| `hit()` | RPC | `#[rpc(authority, call_local, unreliable)]` | — | — | `Bullet` (`has_method("hit")` → `rpc("hit")`) |
| `play_shoot()` | RPC | `#[rpc(authority, call_local, unreliable)]` | — | — | interno |
| `shoot_check()`, `resume_approach()` | métodos | `#[func]` | — | — | method tracks da animação "shoot" (`red_robot.tscn:10296-10299`) |
| `_on_area_body_entered(body)`, `_on_area_body_exited(body)` | métodos | `#[func]` | — | — | conexões `red_robot.tscn:11050-11051` |
| `transform` | `Transform3D` | base | — | — | `level.gd:98`; `red_robot.tscn:30` (`.:global_transform`) |

Enum `State` (`#[godot(via = i64)]`): `Idle = 0`, `Approach = 1`, `Aim = 2`, `Shooting = 3`.

### Estado interno

| Campo | Tipo Rust | Inicial | Papel |
|---|---|---|---|
| `shoot_countdown` | `f32` | 6,0 | Tempo de frente para o jogador antes de tentar mirar |
| `aim_countdown` | `f32` | 1,0 | Tempo em AIM antes de confirmar o tiro |
| `player` | `Option<Gd<Node3D>>` | `None` | Corpo detectado (Player ou "Target") |
| `orientation` | `Transform3D` | global com origem zero (em `ready`) | Rotação do robô; recebe root motion; gravada em `global_basis` |

Constantes (`f32`): `PLAYER_AIM_TOLERANCE_DEGREES` = 15° em rad, `SHOOT_WAIT` 6,0, `AIM_TIME` 1,0,
`AIM_PREPARE_TIME` 0,5, `BLEND_AIM_SPEED` 0,05.

### Referências de cena (14 `OnReady`, research D8)

`animation_tree`, `shoot_animation`, `model`, `ray_from`, `ray_mesh`, `laser_raycast`,
`collision_shape`, `explosion_sound`, `hit_sound`, `death`, `death_shield1`/`2`/`death_head`
(**`Gd<Part>`**), `death_detach_spark1`/`2`. `LaserEmber` buscado em `shoot()`.

### Máquina de estados (servidor, `physics_process`)

```
IDLE ──body_entered(Player|"Target")──▶ APPROACH ──body_exited(Player)──▶ IDLE
APPROACH: vira/anda até o jogador; de frente (±15°) decrementa shoot_countdown (6 s);
          < 0 → raycast até o jogador+UP: acerta → AIM (aim_countdown=1, aim_preparing=0)
                                           senão → shoot_countdown = 6
AIM:      laser clipado; aim_preparing ↑ até 0,5; aim_countdown < 0 → raycast:
          acerta → SHOOTING (shoot_countdown=6, rpc play_shoot → anim "shoot")
          senão  → resume_approach() (APPROACH, aim_preparing=0,5, shoot_countdown=6)
SHOOTING: laser clipado; method tracks da animação: shoot_check (2,25 s) → test_shoot →
          shoot() no próximo frame; resume_approach (3 s) → APPROACH
qualquer: hit() ×5 → dead (peças explodem, faíscas, som, sinal exploded; servidor remove em 10 s)
```

`animate(delta)`: `state/transition_request` ∈ {`turn_left`, `turn_right`, `walk`, `idle`};
com alvo: `aiming/blend_amount = clamp(aim_preparing/0,5)`, `aim/blend_position` incrementado
pelos ângulos h/v (graus) do alvo+UP no espaço do `RayMesh`, clampado em [−1, 1].

### Relações

- **Consome (tipado)**: `Player` (`add_camera_shake_trauma(13.0)` após `try_cast`, com atraso de
  0,1 s), `Part` (`explode()` ×3).
- **Consome (API base)**: `impact_effect.tscn` (`Blast`) como `Node3D`.
- **É consumido por**: `level.gd` (`exploded`, `transform`), `Bullet` (`hit` por duck typing).
