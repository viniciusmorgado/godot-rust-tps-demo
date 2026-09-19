# Contract: the enemy entities (V3-C) — extends `specs/012-v3-player-input-camera/contracts/player-entity.md`

The V3-A and V3-B contracts (bridge rules, push API, system-author rules, sub-bridges, the
`bind_mut` guard rule, the `AnimationTree` rule, tradeoffs header) apply unchanged; this file adds
what the enemy introduces.

## 1. Three bridges, three entity kinds

- **`Bullet`**: registers in `ready` with `BulletHandles`; unregisters in `exit_tree`. Remote-only
  handler `explode` (`call_remote`) pushes `BulletFx::Explode`; the method-track `destroy` pushes
  `BulletDestroy` on every peer (the drain frees only on `Simulates`).
- **`Part`**: registers ITSELF in `ready` (a scene child of the robot with its own class and
  synchronizer — NOT a sub-bridge: it outlives the robot's death sequence, replicates and is freed
  independently); `set_fade_value` stays the projection setter (`pub(crate)`, called by
  `sync_out_part` on the server and by replication on clients); `destroy` (`call_remote`) pushes
  `PartFx::Destroy`; `explode` is removed.
- **`EnemyRobot`**: registers in `ready` with `RobotHandles`, which include the three `Gd<Part>`
  (for the cross-entity explosion, §3) and `AimBlend`'s initial value read once from the tree;
  `hit` and `play_shoot` are `call_remote` (remote-only handlers push `RobotHit`/`RobotFx`); the
  method-track `#[func]`s `shoot_check`/`resume_approach` and the area signals push.

## 2. The same-run hit path (option (B))

```
fixed run N (the bullet's collision step)
  SyncIn            sync_in_bullet, sync_in_robot
  Gameplay          bullet_step (pure::step), robot_decide
  EngineQueryOrient robot_query (raycasts, root motion)
  GameplayIntegrate robot_step_and_animate
  EngineQueryMove   move_bullet: move_and_collide → Collided { hit: kind_of(collider) }   ← HitKind::Robot(id)
                    move_robot
  GameplaySettle    bullet_settle:    explode intents; MessageWriter<RobotHitLocal>{ robot: id }
                    robot_hit_apply:  .after(bullet_settle) — MessageReader; EntityIndex → robot; dead guard; hit_step → Health, RobotIntents{hit, just_died}
  SyncOut           sync_out_bullet:  explode effects, HitKind::rpc_hit (network: call_remote for the robot), collision off, rpc("explode")
                    sync_out_robot:   reaction randi() + param + sound; on just_died the death branch (§3); tree params; advance LAST
```

`Messages<RobotHitLocal>` is `update()`d once at the start of each fixed run, before the drain; a
message written and read in one run never survives; one written with no reader (commit 2) is
dropped one run later. Remote peers get `rpc("hit")` and apply the visual half in their frame run.

## 3. The cross-entity part explosion (from `sync_out_robot`'s death branch)

Per part in scene order (`PartShield1`, `PartShield2`, `PartHead`), after the reaction
`randi()`: `randf()` ×3 → `random_angular_velocity`, `randf()` ×1 → `wait_time(lifetime,
lifetime_random, r)`; node writes through the robot's `Gd<Part>` handle (`set_visibility_public`
via the part's own `NodeHandles` entry, `set_freeze_enabled(false)`, `col1/col2` enabled,
`set_linear_velocity(3·UP)`, `set_angular_velocity`) and the part ENTITY's phase
`PartPhase::Waiting(Timer(wait))` through `EntityIndex` + `Query<&mut PartPhase, Without<RobotTag>>`.
Thirteen draws, v2's order, glue only. The parts never draw.

## 4. Projections per peer role

| Node field | Written by | Read by |
|---|---|---|
| `EnemyRobot.state`, `.target_position`, `.health`, `.dead`, `.aim_preparing` (`#[var]`) | fixed `sync_out_robot`, `Simulates` only (`dead` once, in the death branch) | fixed `sync_in_robot` on non-`Simulates` (replay: `state`, `target_position`, `aim_preparing`) |
| `EnemyRobot.test_shoot` (`#[var]`) | nothing at runtime (registration reads it) | registration |
| `Part.fade_value` | frame `sync_out_part` through `set_fade_value`, `Simulates` only | the engine's replication invokes the same setter on clients (shader write inside the bridge) |
| `Part.position/rotation/linear_velocity/angular_velocity`, `Bullet.global_transform`, `EnemyRobot.global_transform` | the engine (physics, `move_and_slide`, `move_and_collide`) and `sync_out_robot`'s `set_global_basis` | engine replication |

## 5. `docs/v3-tradeoffs.md` rows this milestone adds

`move_and_collide` + collider resolution into `HitKind` (`move_bullet`); the `RigidBody3D` part
simulated and replicated by the engine, the fade through the node's setter (`sync_out_part`);
the puff instanced from the part's `SyncOut` (server) and the remote handler (clients); the
robot's raycasts (pre-check/aim and shoot) in `EngineQueryOrient` (`robot_query`); the laser
`RayCast3D` read at `SyncIn` (a disabled ray — constant; the general ordering rule of R8); the
robot's `AnimationTree` MANUAL + `advance` (second tree, R1) and `AimBlend` replacing the tree
`get`; engine RNG draws in glue for seeded parity (the death branch, R5); the `exploded` signal
emitted from glue for v2's `level.rs`; the blast instanced under the tree root from `SyncOut`;
the four `call_remote` attributes and the same-run message (option (B)); `Part::explode` removed.

## 6. Harness contract files

- `zz_ecs_parity.gd` — the five-case harness (research R10) with its observer and physics probe;
  scene `zz_ecs_parity.tscn` as in V3-A's contract §4; `zz_ecs_observer.gd` and
  `zz_ecs_physics_probe.gd` as quoted inside.
- `zz_r1_robot.gd` — the robot `AnimationTree` experiment (`ZZ_MODE`, `ZZ_PZ`, `ZZ_PROBE`) with
  its four probe scripts quoted (R1 and R8's laser probes).
- `zz_r2_parts.gd` — the parts determinism / RNG-order experiment with its probe quoted.
- `zz_r3_order.gd` — the engine-updated-child ordering experiment with its three scripts quoted.

All scratch: copied into the game project for a run, deleted before every commit.
