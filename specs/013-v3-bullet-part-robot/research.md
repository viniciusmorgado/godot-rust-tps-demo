# Research: Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the ECS core

Evidence from the working tree at `30d1a4d` (`bullet.rs`, `part.rs`, `red_robot.rs`,
`red_robot/model.rs`, `hittable.rs`, `red_robot.tscn`, `bullet.tscn`, `level.tscn`, `src/ecs.rs`,
`src/ecs/*`, `player/sync.rs`), the spec at `e5ca11a` (option (B): the robot's hit in the SAME
fixed run as the bullet's collision), the generated gdext 0.5.5 bindings, `bevy_ecs 0.19.1`, the
Godot 4.7 sources (`modules/multiplayer/scene_replication_interface.cpp`, fetched 2026-09-19),
and FOUR headless experiments run on 2026-09-19 (scratch scripts reproduced under `contracts/`).
Every decision below ends in a citation or an experiment line.

## R1 — The robot's `AnimationTree` (EXPERIMENT, spec US4 / FR-022)

**Decision: option (B) holds for the robot.** `red_robot.tscn:10782` changes from
`callback_mode_process = 0` to `2` (MANUAL) and `sync_out_robot` calls `anim_tree.advance(delta)`
for EVERY non-`Dead` robot on every peer as the LAST engine write of the fixed run for that entity,
after the parameter writes. Option (A) is rejected for the same reason as the player's: the
driver at `i32::MAX` reads each step's root motion one step early.

**Experiment** (`contracts/zz_r1_robot.gd`; twin of specs/012 `zz_r1_probe.gd`): a floor, one
`red_robot.tscn` at `(0, 0.05, 0)`, `seed(1)` first, the joypad purge, `--fixed-fps 60
--quit-after 305`; a `player.tscn` (`player_id = 1`) is added by the `i32::MIN` probe at physics
step 10 at `(0, 0.05, ZZ_PZ)` inside the detection area (`PlayerDetectionArea` is a sphere of
radius 20, `red_robot.tscn:10210-10211`, layer/mask 2). `ProbeMin` (priority `i32::MIN`, the
point v2's tick reads) logs `S<n> rm rmr origin basis_z state node target aim`; `ProbeMax`
(`i32::MAX`, the driver's point) logs `M<n> rm rmr origin` and, with `ZZ_MODE=manual`, calls
`advance(delta)` before logging. Runs: (i) `v2` PHYSICS, (ii) `v3` PHYSICS, (iii) `v3` MANUAL +
advance — on both trees the robot code is v2's, which is what makes the probes comparable.

Facing: the robot's front is +Z (`red_robot/model.rs:90-95`, `angle_to_player` uses
`x.atan2(z)`). With `ZZ_PZ=-8` (behind) the robot only turns (`node=turn_left` for 289 steps, `rm`
stays zero — no translation, useless as a root-motion test); with `ZZ_PZ=8` (in front) it walks:

```
S11 rm=(0.0, 0.0, 0.0)      origin=(0.0, 0.0255, 0.0)      state=1 node=idle target=(0.0, 0.0, 0.0)
M11 rm=(0.0, 0.0, 0.0)      origin=(0.0, 0.020056, 0.0)
S12 rm=(0.0, 0.0, 0.0)      origin=(0.0, 0.020056, 0.0)    state=1 node=walk target=(0.0, 0.05, 8.0)
M12 rm=(0.0, 0.0, 0.000016) origin=(0.0, 0.011889, 0.0)
S13 rm=(0.0, 0.0, 0.000016) origin=(0.0, 0.011889, 0.0)    state=1 node=walk
M13 rm=(0.0, 0.0, 0.000032) origin=(0.0, 0.001425, 0.000016)
…
S200 rm=(0.0, 0.0, 0.014289) origin=(0.0, 0.001235, 2.213145) state=1 node=walk
M200 rm=(0.0, 0.0, 0.014937) origin=(0.0, 0.002439, 2.227435)
S201 rm=(0.0, 0.0, 0.014937) origin=(0.0, 0.002439, 2.227435)
M300 rm=(0.0, 0.0, 0.00444)  origin=(0.0, 0.002436, 3.914878)
```

Results (601 lines per run, `ZZ_PZ=8`; the same three verdicts with `ZZ_PZ=-8`):

- `diff (i) (ii)`: **no differences** — the `v3` tree behaves as `v2` for the robot today.
- `diff (i) (iii)`: **no differences except the `MODE` header line** — advancing a MANUAL tree
  right after the robot's tick reproduces PHYSICS mode step by step, including the first steps
  (`S1 rm=0`, `M1 rm=0`: nothing processes before the first tick in either mode).
- Option-A read-point shift, every step: `M(n).rm == S(n+1).rm` (M12 0.000016 = S13, M13
  0.000032 = S14, M200 0.014937 = S201). An option-A tick would integrate each step's root
  motion one step early; the origin would diverge from step 13 on (3.91 m walked by S300).

Consequences: the tree no longer processes itself, so `advance` MUST run every fixed run for
every registered robot that is not `Dead` (v2 set the tree inactive at death, `red_robot.rs:294`,
and at `ready` when `dead`, `:122` — a `Dead` entity is skipped, never advanced); `advance(delta)`
uses `FixedDelta`; the parameter writes of the same run precede it; the root motion read by the
NEXT run's `EngineQueryOrient` is the one this run's `advance` produced. `ready`'s `animate(0.0)`
(`:125`) needs no replacement: the first fixed run writes the parameters before the first
`advance`, and the experiment shows the tree's first processing happens after tick 1 in both
modes (`S1`/`M1` identical). `rmr` stayed `(0, 0, 0, 1)` while walking straight (no root
rotation); the turn branch was exercised with `ZZ_PZ=-8` and matched too.

## R2 — `RigidBody3D` determinism of the parts and the 13-draw order (EXPERIMENT, Assumptions (4), (8))

**Decision: case (b) compares the parts' POSITIONS, velocities, `fade_value` and phase frames
bit-for-bit.** The engine's rigid-body simulation is reproducible run-to-run and across the two
trees for this scene, and the seeded draw order is proven by identical angular velocities.

**Experiment** (`contracts/zz_r2_parts.gd`): a floor, one `red_robot.tscn`, `seed(1)` first, the
joypad purge; the `i32::MIN` probe calls `robot.hit()` FIVE times at physics step 30 (`hit` is a
callable `#[rpc]` method, `call_local` in v2 — `health = 5`, `red_robot.rs:41-42`) and logs per
step, for each part in scene order, `global_position`, `linear_velocity`, `angular_velocity`,
`fade_value` (`--quit-after 340`). Runs: `v2` twice, `v3` once (v2 part/robot code on both).

```
RAW S30 killed health=0 dead=true
RAW S30 part0 angular=(-1.435281, 8.067381, -9.53594)  wait_started
RAW S30 part1 angular=(8.159636, -1.621307, -9.841543) wait_started
RAW S30 part2 angular=(3.475226, 4.696961, -8.445145)  wait_started
S30 | p0 pos=(-1.37822, 1.611749, 1.09121) lv=(0.0, 3.0, 0.0) av=(-1.435281, 8.067381, -9.53594) fade=0.000000
S31 | p0 pos=(-1.385496, 1.663503, 1.096162) lv=(0.0, 2.831666, 0.0) av=(-1.425712, 8.013598, -9.472367)
…
S241 | p2 … fade=0.001111      ← PartHead's fade starts (wait = 3.0 + 3.0·r ≈ 3.52 s after S30)
S258 | p2 … fade=0.321111      ← the last write before should_destroy (v2 process disabled)
S269 (last logged line: the probe read a freed part at S270 — PartHead freed 0.2 s after destroy)
```

- `diff v2_run1 v2_run2`: **no differences** (273 lines) — the solver is deterministic under
  `--fixed-fps 60` with the same scene order.
- `diff v2_run1 v3_run1`: **no differences** — the `v3` tree's physics equals `v2`'s.
- The explode-step angular velocities are identical on both trees: the thirteen draws (`randi()`
  for the reaction at `red_robot.rs:285`, then per part `randf()` ×3 for
  `random_angular_velocity` and ×1 for `wait_time`, `part.rs:173-175`, in the order
  `PartShield1`, `PartShield2`, `PartHead`, `red_robot.rs:302-304`) reproduce for `seed(1)`. v3
  performs exactly these thirteen draws, in this order, in `sync_out_robot`'s death branch (R5).

Harness rule learned: a probe MUST test `is_instance_valid(part)` BEFORE reading a part's
properties (the R2 probe read `global_position` first and stopped logging at S269 with
`Invalid access … previously freed`); the free frame itself is the RAW stamp.

## R3 — `move_and_collide` and `hittable::resolve` from `EngineQueryMove`; `HitKind` (spec FR-006, Assumption (3))

**Facts** (gdext 0.5.5 generated bindings, `target/debug/build/godot-core-*/out/classes/`):
`PhysicsBody3D::move_and_collide(&mut self, motion: Vector3) -> Option<Gd<KinematicCollision3D>>`
(`physics_body_3d.rs:37`); `KinematicCollision3D::get_collider(&self) -> Option<Gd<Object>>`
(`kinematic_collision_3d.rs:259`) — v2 already does `col.get_collider().and_then(|c|
c.try_cast::<Node3D>().ok())` (`bullet.rs:115-116`) and `hittable::resolve(collider)` two
`try_cast`s (`hittable.rs:16-24`), all inside its `physics_process`; the same calls from a system
running inside the driver's `physics_process` of the same step see the same physics-server state
(the driver runs after every node's callback, CLAUDE.md "Autoload"). `Gd::<T>::try_from_instance_id(
InstanceId) -> Result<Gd<T>, ConvertError>` (godot-core `src/obj/gd.rs:255`) re-fetches a handle
by id; it fails (`Err`) for a freed object, which cannot happen within one run (nodes are freed
only by `sync_out_remove`, at the end).

**Decision — `hittable.rs`, additive only (FR-025)**:

```rust
/// The `Send` projection of a `HitTarget`: which class and which node, as ids — a component may
/// carry it (no `Gd`). Resolved once, in the bullet's `EngineQueryMove`, right after
/// `move_and_collide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind { Player(InstanceId), Robot(InstanceId) }
/// `resolve` + `instance_id()`.
pub fn kind_of(node: Gd<Node3D>) -> Option<HitKind>;
impl HitKind {
    /// Re-fetches the target by id and sends the by-name RPC (the ONE spelling stays in
    /// `HitTarget::rpc_hit`): `Player` → `#[rpc(call_local)]` runs its handler locally too
    /// (pushes `AddTrauma`); `EnemyRobot` → `call_remote` from commit 3 on, so nothing runs
    /// locally — the local path is `Messages<RobotHitLocal>` (R4). A freed target is a no-op.
    pub fn rpc_hit(self);
    pub fn robot_id(self) -> Option<InstanceId>;
}
```

`resolve`/`rpc_hit`/`HitTarget` keep their signatures. In commit 2 the robot's `hit` is still v2's
`call_local` handler, so `HitKind::Robot(id).rpc_hit()` runs it locally exactly as v2 did — the
game stays playable with v2 robots (R11).

## R4 — Same-run hit application (spec option (B), FR-019; EXPERIMENT in a probe crate)

**Decision**: `Messages<RobotHitLocal { robot: InstanceId }>`, written by `bullet_settle`
(`GameplaySettle`, `MessageWriter`) and read by `robot_hit_apply` (`GameplaySettle`,
`.after(bullet_settle)`, `MessageReader`) in the SAME run; `robot_hit_apply` resolves the id
through `EntityIndex`, applies v2's `dead` guard and `hit_step` (`red_robot.rs:278-290`, pure)
to the robot's `Health`, and sets `RobotIntents { hit: true, just_died }`, which
`sync_out_robot` (same run) consumes — reaction `randi()`, hit sound, and on `just_died` the
death branch (R5). Update rule: `Messages::<RobotHitLocal>::update()` once at the start of each
FIXED run, before the drain, next to `DoorBodyEntered` (`ecs.rs:97-101`).

**Probe crate** (`bevy_ecs 0.19.1`, same pin; `scratchpad/r4probe`, output 2026-09-19):

```
R4 same-run: hp=0 hit=true just_died=true part2=Waiting(3.5)
R4 next run: hp=0 (unchanged => message consumed exactly once)
R4 buffer after one update: len=1
R4 buffer after two updates: len=0
```

(1) two systems in one chained set, added in REVERSE order with only `.after` between them: the
writer's message is read by the reader in the same `schedule.run` — the robot's `Health` went
1 → 0 and `just_died` was set in that run; (2) the next run re-read nothing (exactly-once); (3)
a message nobody read survives ONE `update()` and is dropped by the second — so a `RobotHitLocal`
written in commit 2 (no consumer yet) is dropped harmlessly one run later, and a message written
and read within one run never survives. **Unit test** (commit 3, `ecs/setup.rs`):
`robot_hit_apply_runs_after_bullet_settle_in_the_same_run` — `build_world()` + `build_fixed()`
with the real `bullet_settle` and `robot_hit_apply` registered; one bullet entity with
`Collided { hit: Some(HitKind::Robot(id)) }` and one robot entity `Health(1)` registered under
`id` in `EntityIndex`; `Messages::update()`; ONE `schedule.run`; assert `Health == 0`,
`RobotIntents.hit && just_died`, and after a second run with `Collided { hit: None }` the health
is still 0 (consumed once). A second test in `red_robot/system.rs`, `robot_hit_apply_ignores_
dead_robot`: a `Dead` robot keeps its `Health` and gets no `hit` intent (`:278-280`).

## R5 — Cross-entity part explosion from the robot's `SyncOut` (FR-009/FR-014; probe crate)

**Decision**: `sync_out_robot`'s death branch holds, besides the robot's own query,
`ResMut<EntityIndex>` (read-only use), `NonSendMut<NodeHandles>` (ONE map: the robot's entry AND
the parts' entries) and `Query<&mut PartPhase, Without<RobotTag>>` — the probe crate compiled and
ran a system with `ResMut<Index> + NonSendMut<Handles> + Query<(Entity, &RobotIntents)> +
Query<&mut PartPhase>` (disjoint component types, no access conflict — `part2=Waiting(3.5)` above
was set through it). Borrow shape inside the branch, per part in scene order: `let part_id =
handles[robot].parts[i].instance_id()` (the `Gd<Part>` handle, `red_robot.rs:96-101`); the draws
`randf()` ×3 → `random_angular_velocity`, `randf()` ×1 → `wait_time(lifetime, lifetime_random,
r)` with the part's exported lifetimes read from the part's `PartLifetimes` component (or from
`p.root.bind()`); the node writes through the ROBOT's `Gd<Part>` handle (`Deref` to
`RigidBody3D`): `set_freeze_enabled(false)` (`rigid_body_3d.rs:717`), `set_linear_velocity(3·UP)`
(`:276`), `set_angular_velocity` (`:294`); `col1/col2.set_disabled(false)` and
`synchronizer.set_visibility_public(true)` (`multiplayer_synchronizer.rs:281`) through the PART's
own `NodeHandles` entry (`EntityIndex.entity(part_id)` → `Handles::Part(p)` → `p.synchronizer`,
`p.col1`, `p.col2`); the phase through `parts.get_mut(part_entity)` → `PartPhase::Waiting(
Timer::new(wait))`. Order per part = v2's `explode` (`part.rs:164-175`): visibility public,
unfreeze, [server:] collisions on, linear velocity, angular velocity, wait. The thirteen draws
happen in glue before anything else of the death branch touches RNG: `randi() % 3 + 1` for the
reaction (`red_robot.rs:285`, drawn once per live hit, before `hit_step`'s outcome is used) then
the twelve `randf()` (`part.rs:173-175` ×3). Both maps are borrowed mutably once for the whole
system (`NonSendMut<NodeHandles>` is exclusive anyway); the robot's handles are taken out of the
map with `remove`/re-`insert` around the per-part writes if the borrow checker needs it — the
plan pins: `let robot_handles = handles.by_entity.get(&robot).…` cloned `Gd`s (cheap ref-counted
clones of the three part handles) BEFORE the loop, so the map is free for the parts' lookups.
`Part::explode` (`part.rs:162-192`) has no caller after this and is REMOVED (Complexity Tracking).

## R6 — Robot systems and sets (FR-015–FR-021)

| Set | System | Reads / writes (shape) | v2 lines |
|---|---|---|---|
| `SyncIn` | `sync_in_robot` (glue, `red_robot/sync.rs`) | every robot: `RobotFrame { global_transform, gravity, velocity, ray_from_transform, ray_mesh_transform, ray_mesh_z, laser_colliding, laser_point, player_origin: Option<Vector3> }` — the tracked player's origin via `Gd::<Node3D>::try_from_instance_id(id)` (decision, Assumption (6): one FFI lookup + one `get_global_transform`, no `EntityIndex`/`NodeHandles` borrow in `SyncIn`, and it works for a player that is not an ECS entity — none exists after V3-B, but the lookup is the one v2 made, `:153`); non-`Simulates`: `ReplayRobot { state, target_position, aim_preparing }` from `root.bind()` | `:153`, `:164`, `:180-181`, `:199-202`, `:249-250`, `:478`, `:493`; `:134` |
| `Gameplay` | `robot_decide` (pure, `red_robot/system.rs`) | `Simulates`, not `Dead`: the no-player branch (`target = ZERO`, `idle_velocity`, `idle_branch = true`); else `target = player_origin`; `state_at_start`; `Approach`: local angle → `facing && shoot_countdown_will_expire` → `raycast = Some((ray_from.origin, target + UP))`; `Aim | Shooting`: `max_dist` from the laser snapshot (a constant 1000: R8) → `clip = Some(max_dist)`; `Aim && aim_countdown_will_expire` → `raycast`; `ShootRequested` marker → `shoot = true` (removed) | `:138-151`, `:160-184`, `:197-223` |
| `EngineQueryOrient` | `robot_query` (glue) | `raycast` → `raycast_to` (`:363-382`) → `sees_player = collider id == tracked id`; `shoot` → the shoot raycast along `ray_from.basis.col_b()` for 1000 m → `ShotResult { max_dist, hit: Option<(position, Option<InstanceId>)> }`; root motion → `RootMotion` (previous `advance`'s value, R1) | `:177-183`, `:216-222`, `:399-409`, `:241-244` |
| `GameplayIntegrate` | `robot_step_and_animate` (pure) | `model::step(state_at_start, counters, dt, inputs { angle, sees_player }, tuning)` → `RobotState`, counters, `cmds` (`RpcPlayShoot` → `play_shoot` intent; `ResumeApproach` → already applied by `step`); THEN the animation decision on the NEW state and counters: `transition_request`, `aim_blend_amount`, `cannon_angles` from the `RayMesh` snapshot, `aim_blend_step` on `AimBlend` → `AnimDecision`; THEN `integrate_root_motion` (or `idle_velocity` on the idle branch) → `Orientation`, `Velocity`; non-`Simulates`: the replay `AnimDecision` from `ReplayRobot` (`animate` on the replicated fields) | `:186-196`, `:225-235`, `:238` → `:456-490`, `:245-251`, `:146`; `:134` |
| `EngineQueryMove` | `move_robot` (glue) | `set_velocity`, `set_up_direction(UP)`, `move_and_slide` (also on the idle branch) | `:147-149`, `:253-255` |
| `GameplaySettle` | `robot_hit_apply` (pure, `.after(bullet_settle)`) | R4 | `:278-290` |
| `SyncOut` | `sync_out_robot` (glue) | per `Simulates` robot: `set_global_basis` (not on the idle branch, `:150`); projection `state`/`target_position`/`health`/`dead`/`aim_preparing` via `p.root.bind_mut()` (dropped); `play_shoot` → `shoot_anim.play("shoot")` inline + `rpc("play_shoot")` (`call_remote`); shoot effects: `_clip_ray(max_dist)` (skipped on dedicated servers), ember position/extents, blast under the tree root at the hit, `PendingTrauma(Timer(0.1))` when the hit collider is the tracked player; the Aim/Shooting `_clip_ray` (order-insensitive: nothing reads the shader parameter mid-tick — a `SyncOut` write); hit: reaction `randi()` + parameter + sound; death: R5 + explosion sound + `exploded` emitted + `RemovalTimer(10 s)` on the server; every non-`Dead` robot: the tree parameter writes (`transition_request`, `aiming/blend_amount`, `aim/blend_position`) then `advance(FixedDelta)` LAST | `:257-258`, `:331-332`, `:411-425`, `:437-453`, `:205`, `:285-287`, `:293-326`, `:469-488` |

Frame schedule: `robot_timers` (pure, `Gameplay`): steps `PendingTrauma` → `TraumaDue` marker on
expiry; `RemovalTimer` → `Remove` on expiry (V3-A's pattern: `Remove` inserted from a pure
system, released by `sync_out_remove`). `sync_out_robot_frame` (glue, `SyncOut`): `TraumaDue` →
`queue::push(AddTrauma { root_id: tracked player, amount: 13.0 })` (V3-B's arm applies it to the
player entity at the next run's drain — v2's `add_camera_shake_trauma` does the same push since
V3-B), and the remote `RobotFx` application on non-`Simulates` robots: `Hit` → reaction (own
`randi()`, as v2's remote `call_local` handler drew its own), sound, and if the replicated `dead`
is now true the death visuals (tree inactive, model hidden, `Death` visible, collision off,
sparks; the parts' visibility public + unfreeze — the every-peer half of `part.rs:164-166`,
without velocities: `:167-169`); `PlayShoot` → `shoot_anim.play("shoot")`. The `ShootAnimation`'s
method tracks (`red_robot.tscn:10283-10297`: `shoot_check`, `resume_approach`) fire during that
`AnimationPlayer`'s idle processing, before the driver in tree order, so their pushes are drained
by the same pass's frame run: `ShootRequested` marker (consumed by the next fixed run's
`robot_decide` — v2's `test_shoot` flag was also consumed by the next physics step, `:138-141`);
`ResumeApproachRequested` applied at drain time (`Approach` + `resume_approach_reset`,
`:268-273`, visible to the next physics step as v2's inline write was).

`AimBlend`: initialized once at registration from `anim_tree.get("parameters/aim/blend_position")`
(the scene value `red_robot.tscn:10785`), stepped in `robot_step_and_animate`, written by
`sync_out_robot`; v2 read it back from the tree every step (`:482-485`) — the tree returns what
the tick wrote (a Vector2 parameter is not modified by processing), so the component is
behavior-identical and saves one `get` per step (recorded in `docs/v3-tradeoffs.md`).

`Dead`: a marker inserted at registration when the scene says `dead` and by the death branch;
the fixed tick's queries are `Without<Dead>`; the projection writes `dead = true` once, in the
death branch (`:293`).

## R7 — Bullet and part systems (FR-005–FR-013)

Bullet, fixed schedule, `Simulates` (the server): `sync_in_bullet` (`SyncIn`: `basis.col_c()`
into `BulletBasisZ`, `bullet.rs:112`) → `bullet_step` (`Gameplay`, pure: `active = state !=
Exploded` at run start (`:98-100`); when active `pure::step` → `BulletStateC`, `intents.explode =
expired` (`:103-107`)) → `move_bullet` (`EngineQueryMove`, glue: when active `move_and_collide(
-dt · VELOCITY · basis_z)` (`:111-113`) → `Collided { hit: kind_of(collider) }` (`:115-116`,
R3)) → `bullet_settle` (`GameplaySettle`, pure: when collided: `intents.hit = collided.hit`,
`disable_collision = true`, `explode |= state is still Flying` (backlog #13, `:123-127`), `state =
Exploded` (`:130`); for `HitKind::Robot(id)`: `RobotHitLocal { robot: id }` written) →
`sync_out_bullet` (`SyncOut`, glue, order-insensitive per the spec: `explode` local effects once —
`anim.play("explode")`, `light.set_shadow(true)` when the registration's `shadow_mapping`
(`:139-145`); `hit.rpc_hit()` (R3); `collision.set_disabled(true)` (`:122`); `rpc("explode")`
`call_remote` once). `pure::step` and its 3 tests stay inside `bullet.rs` as `pub(crate) mod
pure` (one visibility token; no file move). Non-`Simulates` bullets: the remote `explode`
handler pushes `BulletFx::Explode`, applied by the frame `SyncOut` (`sync_out_bullet_frame`:
play + shadow). `destroy` (`bullet.tscn:92-104`, method track at 1.5 s, every peer) pushes
`BulletDestroy { id }`; the drain inserts `Remove` only when the entity has `Simulates`
(`:150-152`); the client's replica is a spawner-spawned node (the player's tick instantiates
`bullet.tscn` under `SpawnedNodes`, which `level.tscn:71-73`'s `MultiplayerSpawner` watches with
three spawnable scenes — the player `uid://cs1k22tdf04k4`, the robot `uid://byi6b08jpb2iw`, the
bullet `uid://jphgr3qep5`) and is despawned by replication when the server frees its node —
`scene_replication_interface.cpp`: `on_despawn_receive` (`:773`) frees only nodes present in
`recv_nodes`, i.e. spawned through `on_spawn_receive` with a `net_id` assigned by
`_make_spawn_packet` (`:678`) (Assumption (5) confirmed).

Part, frame schedule: `part_phase_tick` (`Gameplay`, pure): `Waiting(timer)` → `step(dt)` →
`Fading { counter: 0 }` (`part.rs:179-191`); `Fading { counter }` → `PartFade(Some(fade_curve(
counter, disappearing_time)))` (`:139`), `counter += dt` (`:141`), `should_destroy` (`:142`) →
`destroy` intent + `Destroyed(Timer(0.2))`; `Destroyed(timer)` → `Remove` on expiry (`:202-214`).
`sync_out_part` (`SyncOut`, glue, `Simulates`): `p.root.bind_mut().set_fade_value(fade)`
(`:140` → `:151-160`; the setter becomes `pub(crate)`), then on the destroy intent the puff
instanced under `puff_parent(&p.root)` (a free function in `part/sync.rs` mirroring `:224-232`)
at the part's origin (`:196-200`) and `rpc("destroy")` (`call_remote`). Non-`Simulates` parts: the
remote `destroy` handler pushes `PartFx::Destroy`; the frame `SyncOut` instances the puff and
sets the entity's `Destroyed(Timer(0.2))`, so the client frees its own node through `Remove` as
v2's `call_local` handler did (`:202-214`) — the parts are scene children of the spawned robot,
NOT spawner-spawned, so no despawn packet ever frees them (R7's citation above: only
`recv_nodes` are despawned). On the client the `Waiting`/`Fading` phases never run (v2 returned
before the timer off-server, `:167-169`); `fade_value` arrives by replication and the engine
invokes the same setter (a projection-inbound engine write inside the bridge). `puff_parent`'s
backlog #15 comment moves with the function.

## R8 — The laser `RayCast3D` and the engine-update ordering (EXPERIMENTS, Assumption (2))

**Fact 1**: the laser `RayCast` node is `enabled = false` (`red_robot.tscn:10762-10766`,
`target_position = (0, 0, -1000)`, mask 3). A disabled `RayCast3D` never updates itself, so v2's
`laser_raycast.is_colliding()` (`red_robot.rs:200`) is always `false` and `max_dist` is always
`1000.0` (`:199-204`): `_clip_ray(1000.0)` every Aim/Shooting step. Verified by the R1 scene with
`ZZ_PROBE=zz_r3` (`contracts/zz_r1_robot.gd`'s R3 probes): `colliding=false` at every step from
S300 to S520 through `state=2` (Aim, from S371) and `state=3` (Shooting, from S432). Decision:
`sync_in_robot` reads `is_colliding()`/`get_collision_point()` exactly as v2 did (two reads per
Aim/Shooting step, kept for parity and for a future scene that enables the ray) and `robot_decide`
derives `max_dist` from the snapshot — placement is irrelevant for a disabled ray.

**Fact 2 — the general rule** (`contracts/zz_r3_order.gd`): a live `RayCast3D` CHILD updates
AFTER its parent's priority-0 `_physics_process` and BEFORE the `i32::MAX` driver:

```
S5 holder.x=0.05 (set now) ray_point=(0.0, 0.0, 0.0)
H5 (parent, priority 0) sees ray_point.x=0.00 colliding=true while holder.x=0.05
M5 (MAX) sees ray_point.x=0.05
S6 holder.x=0.06 (set now) ray_point=(0.05, 0.0, 0.0)
H6 (parent, priority 0) sees ray_point.x=0.05 colliding=true while holder.x=0.06
M6 (MAX) sees ray_point.x=0.06
```

The parent reads the PREVIOUS step's raycast result; the driver reads THIS step's — the same
one-step relation R1 found for the tree. Rule for `CLAUDE.md` (commit 4): when a tick reads an
engine-updated child (`RayCast3D` internal physics update, `AnimationTree` in PHYSICS mode), a
`SyncIn` read at `i32::MAX` is one step NEWER than v2's read inside the parent's callback; to
reproduce v2, either drive the child from the tick (the tree: MANUAL + `advance`) or keep a
one-step buffer (`prev`/`curr`) in the snapshot. The laser needs neither (Fact 1).

## R9 — Engine-call budget (a record, not a goal)

Read from the code, explicit calls per entity per step. Bullet (flying, no hit): v2 =
`get_transform` + `move_and_collide` = 2 (`bullet.rs:112-113`); v3 = the same 2 + the root sweep
= 3; on the hit step both add `get_collider` + 2 `try_cast` + `rpc("hit")` + `set_disabled` +
`rpc("explode")` (v3's `rpc_hit` re-fetches the target: +1 `try_from_instance_id`). Part (fading,
per frame): v2 = 1 shader `set` (+ `rpc("destroy")` once); v3 = the same + the sweep = 2. Robot
(Approach, walking, no raycast step): v2 = `get_global_transform` ×2 (`:164`, `:461`) + player
`get_global_transform` (`:153`) + tree `set` ×3 + tree `get` ×1 (`:469-488`) + `ray_mesh`
`get_global_transform` (`:478`) + root motion ×2 (`:242-243`) + `get_gravity` + `get_velocity`
(`:249-250`) + `set_velocity` + `set_up_direction` + `move_and_slide` (`:253-255`) +
`set_global_basis` (`:258`) = 16; v3 = 16 − the tree `get` (AimBlend, R6) + `advance` +
`try_from_instance_id` (the tracked player, R6) + the sweep = 18; raycast steps add 3 on both
(`raycast_to`: `create_ex`, `get_world_3d`, `intersect_ray`); Aim/Shooting steps add
`is_colliding` + `get_collision_point` + the shader `set` on both.

## R10 — Parity harness (FR-030)

`contracts/zz_ecs_parity.gd` (V3-B's shape and lessons: observer at `i32::MIN`, physics probe at
`process_physics_priority = i32::MIN`, scripts set BEFORE `add_child`, positions set BEFORE
`add_child`, `seed(1)` FIRST, the joypad purge SECOND, RAW lines, `--case=`), five cases, the
floor at y = −0.5, the robot at `(0, 0.05, 0)` facing +Z:

| Case | Drive | Per-step / per-frame log | RAW | `--quit-after` |
|---|---|---|---|---|
| (a) robot state machine | `player.tscn` (`player_id = 1`, no input) added at step 10 at `(0, 0.05, 8)` | per step: `state`, `target_position`, `aim_preparing`, origin, tree `parameters/state/current_state`, `aim/blend_position` | `ShootAnimation.is_playing()` transitions (`play_shoot`), the blast spawn (tree-root child count), state transitions | 900 (shoot_wait 6 s + aim 1 s + shoot) |
| (b) robot shot to death | a `bullet.tscn` instanced every 30 frames from frame 30 at `(0, 1.2, 6)` with `look_at(robot origin + UP·1.2)` (the player's spawn path: `instantiate`, `add_child` under the harness root, `set_global_position`, `look_at`); no player node | per step: `health`, `dead`, per part (validity FIRST) `global_position`, `linear_velocity`, `angular_velocity`, `fade_value` | each `health` change, the death step, the three parts' `angular_velocity` on the death step, each puff spawn (harness-root child count), each part's free step, the robot's free step (10 s) | 900 |
| (c) bullet expiry | one `bullet.tscn` at frame 10 at `(0, 1, 0)` looking +X (into empty space) | per step: origin, `AnimationPlayer.current_animation`, collision shape `disabled` | the `explode` animation start, the free step | 450 (5 s + 1.5 s) |
| (d) laser at the player | the player at `(0, 0.05, 8)` from step 1; `robot.test_shoot = true` set BEFORE `add_child` (→ `shoot_countdown = 0`, then `test_shoot` fires `shoot()` on the first physics step, `:138-141`) | per frame: the `clip` shader parameter (`ray_mesh.get_surface_override_material(0).get_shader_parameter("clip")`), the ember position, the player's `Camera3D.rotation` | the blast spawn, the first frame the camera rotation changes (trauma arrival) | 200 |
| (e) root motion | R1's scene (`ZZ_PZ=8`), R1's log format | per step: `rm`, `origin`, `state`, `node` | — | 305 |

R2's verdict makes (b) compare part positions bit-for-bit. Commands as V3-B (the `v2` worktree
at `e2932b4`, split `XDG_DATA_HOME`, `--fixed-fps 60`, logs at the real `user://` path, full and
`^RAW` diffs pasted). Timing candidates already known: none expected — option (B) puts the hit,
the death and the parts' velocities in the same fixed run as v2; the frame-run timers reproduce
`SceneTreeTimer` arithmetic (V3-A R5); the trauma push lands one drain later than v2's typed
call but v2's V3-B path already pushes.

## R11 — Commit plan

| # | Commit | Files | Gate + validation |
|---|---|---|---|
| 1 | `ecs + hittable: HitKind, enemy events/drain arms, RobotHitLocal message, components, Handles/Initial variants (tests)` | `hittable.rs` (+`HitKind`, `kind_of`, `HitKind::rpc_hit`), `ecs/event.rs`, `ecs/markers.rs`, `ecs/apply.rs`, `ecs/setup.rs` (`Messages<RobotHitLocal>`, `Tuning(RobotTuning)`), `ecs.rs` (`BulletHandles`/`PartHandles`/`RobotHandles`, `apply_register` arms, `sync_out_remove` arms, the `update()` call) | gates (179 + 8 drain-arm tests = 187); headless sanity |
| 2 | `bullet: bridge over the ECS core — fixed tick with move_and_collide in EngineQueryMove, HitKind, explode call_remote` | `bullet.rs`, `bullet/system.rs`, `bullet/sync.rs`, `ecs.rs`/`ecs/setup.rs` (registration), `docs/v3-tradeoffs.md` (row: `move_and_collide` + collider resolution) | gates (+5 = 192); headless; harness (c) both trees. Playable: bullets hit v2 robots through `HitKind::rpc_hit` → the robot's v2 `call_local` `hit` runs locally as today; `RobotHitLocal` has no consumer yet and is dropped one run later (R4 (3)) |
| 3 | `part + red_robot: bridges, part phase machine on the frame schedule, robot seven-set tick with same-run hit (RobotHitLocal), robot AnimationTree MANUAL + advance (R1 option B)` | `part.rs`, `part/system.rs`, `part/sync.rs`, `red_robot.rs`, `red_robot/system.rs`, `red_robot/sync.rs`, `ecs.rs`/`ecs/setup.rs` (registration, `.after(bullet_settle)`), `red_robot.tscn:10782`, `docs/v3-tradeoffs.md` (rows: part, puff, raycasts, laser, second tree + AimBlend, RNG in glue, `exploded` from glue, blast) — the two cannot be split: the robot's death explodes the parts directly and `Part::explode` is removed | gates (+20 = 212); headless; harness (a), (b), (d), (e) both trees; **STOP 1** single player; **STOP 2** multiplayer |
| 4 | `CLAUDE.md: entity-to-entity messages vs queue, engine-updated children rule, RNG in glue; spec: timing table; tradeoffs complete` | `CLAUDE.md`, `spec.md`, `docs/v3-tradeoffs.md`, `docs/v2-backlog.md` (only if #31 closed) | docs-only |

Two STOPs after commit 3, checkpoint-mark commits as in V3-B; the `v2` worktree stays until the
last task (re-created on 2026-09-19 for these experiments).
