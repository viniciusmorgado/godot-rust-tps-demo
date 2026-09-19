# Feature Specification: Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the ECS core

**Feature Branch**: `v3` (work directly, no per-milestone branch; baseline for this milestone is
`30d1a4d`, V3-B complete, constitution 1.5.2 at `cfd20be`. Local commits only, never pushed.)

**Created**: 2026-09-19

**Status**: Draft

**Phase**: v3 — ECS layer over the nodes (constitution 1.5.2, Principles I, II and III including
the "ECS shape (v3)" subsection; `flying_forklift` is excluded by 1.5.2). Both v3 objectives apply
in every user story: (1) v2's pillars are KEPT — the pure functions and tests of `bullet.rs`'s
`mod pure` (3 tests), `part.rs`'s `mod pure` (5 tests) and `red_robot/model.rs` (25 tests) are
reused verbatim as the bodies of gameplay systems; (2) the ECS layer sits OVER the nodes — three
entity kinds over four bridge classes, no node running per-frame logic. After V3-C every gameplay
module of the constitution's list is ECS. Every rule of `CLAUDE.md` "Port conventions (v3)" is
reused: sub-bridges are NOT needed here (each class is a scene root or a self-registering child),
the seven-set fixed tick, the `call_remote` rule (option (b)), `Handles` for user classes, the
`AnimationTree` MANUAL + `advance` rule (a second tree, the robot's), `Tuning<T>`, the harness
with `seed(1)` first and the joypad purge.

- **Backlog items closed by this spec**: none by default. ONE candidate, the user decides at
  plan review: **#31** (reset `aim_preparing`/`shoot_countdown`/`aim_countdown` when the player
  enters the detection area). Default: deferred — the drain sets `Approach` without touching the
  counters, as v2 `red_robot.rs:345-348` does. If closed, FR-023's variant applies and the
  backlog row is marked done in the closing commit.
- **Backlog items explicitly deferred**: #29 (occasional FPS drop — observe at checkpoint (1)
  only), #30 (`door.tscn` never instanced — stays open).
- **Residual dynamic access kept after this milestone** (`.rpc("name")`, the one sanctioned form,
  plus the pre-existing string-keyed engine surfaces): `rpc("explode")` on `Bullet`
  (`bullet.rs:106`, `:126`), `rpc("destroy")` on `Part` (`part.rs:143`), `rpc("play_shoot")` on
  `EnemyRobot` (`red_robot.rs:388`), `rpc("hit")` in `hittable.rs:33`/`:36` (the file gains a
  local dispatch, option (B) below — the by-name RPC stays);
  the `AnimationTree::set/get` parameter paths of `red_robot.rs:285-286`, `:469-488`
  (`parameters/state/transition_request`, `parameters/aiming/blend_amount`,
  `parameters/aim/blend_position`, `parameters/hit{1..3}/request`); the `ShaderMaterial`
  parameter names `emission_cutout` (`part.rs:158`) and `clip` (`red_robot.rs:499`); the `Model`
  node path of `part.rs:110-114`. No other dynamic access is introduced. Attribute changes under
  option (b) (FR-004): `explode` (bullet), `destroy` (part), `play_shoot` (robot) AND `hit`
  (robot) become `call_remote` — the robot's hit is applied locally in the SAME fixed run as the
  bullet's collision (option (B), spec review 2026-09-19); `hit` (player) stays `call_local`.

**Input**: User description: "Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the
ECS core. Three entity kinds over four bridges; the bullet's tick with `move_and_collide` in
`EngineQueryMove`; the part as an engine-simulated body with a phase state machine and a
replicated shader parameter through its setter; the robot's tick with two raycast kinds feeding
the pure state machine, a second `AnimationTree` with root motion settled by a re-run of V3-B's
R1 experiment, RPCs whose local effects the tick applies (option (b) again), the `exploded`
signal kept for v2's `level.rs`, animation method tracks calling `#[func]`s, `hittable.rs`
unchanged. Parity with `v2` by a five-case headless harness on both trees plus a single-player
and a two-instance multiplayer checkpoint."

## Context

Confirmed by reading the v2 code (`30d1a4d`) and the scenes; every line range below was verified
against the files on 2026-09-19.

| Module (v2) | Lines | Base | Per-frame logic today | Events / RPCs / `#[func]`s today |
|---|---|---|---|---|
| `bullet.rs` (inline `mod pure`, 3 tests) | 155 | `CharacterBody3D` | `physics_process` (server only, `:95-132`) | `#[rpc(authority, call_local, unreliable)] explode` (`:137-146`); `#[func] destroy` (`:148-154`, called by `bullet.tscn`'s method track at 1.5 s, `:92-104`) |
| `part.rs` (inline `mod pure`, 5 tests) | 233 | `RigidBody3D` | `process` (`:138-146`, enabled by `explode`'s timer) | `#[func] set_fade_value` (`:151-160`); `#[func] pub(crate) explode` (`:162-192`, called TYPED by the robot, `red_robot.rs:302-304`); `#[rpc(authority, call_local, unreliable)] destroy` (`:194-215`) |
| `red_robot.rs` + `red_robot/model.rs` (25 tests) | 511 + 569 | `CharacterBody3D` | `physics_process` (`:128-259`) | `#[signal] exploded` (`:264-265`); `#[func] resume_approach` (`:267-274`); `#[rpc(authority, call_local, unreliable)] hit` (`:276-328`); `#[rpc(authority, call_local, unreliable)] play_shoot` (`:330-333`); `#[func] shoot_check` (`:335-338`); `#[func] _on_area_body_entered/exited` (`:340-358`) |
| `hittable.rs` | 40 | — | none | `HitTarget::{Player, Robot}`, `resolve` (`:16-24`), `rpc_hit` (`:30-39`) — gains a `Send` projection `HitKind::{Player(InstanceId), Robot(InstanceId)}` and a local-dispatch path (option (B)); `rpc_hit` by name stays for the network |

**`Bullet`** (`bullet.rs:63-84`): `BulletState { Flying { time_alive }, Exploded }` (`:14-18`,
`pure::step` `:22-34`: decrement, `< 0.0` → `Exploded` + "explode now"), `VELOCITY = 20.0`
(`:83`), handles `AnimationPlayer`, `CollisionShape3D`, `OmniLight3D` (`:71-76`) and the typed
`Settings` autoload (`:78-79`). `ready` (`:88-93`): non-server → `set_physics_process(false)` +
collision shape disabled. `physics_process` (`:95-132`): `Exploded` → return (`:98-100`);
`pure::step` → `rpc("explode")` on expiry (`:103-107`); ALWAYS `move_and_collide(-dt · VELOCITY ·
basis.col_c())` (`:111-113`); on a collision: `hittable::resolve(collider)` → `rpc_hit()`
(`:114-121`), collision shape disabled (`:122`), `rpc("explode")` only if still `Flying` (backlog
#13, `:123-127`), then `state = Exploded` (`:130`). `explode` (`:137-146`): plays `explode`,
enables the light's shadow when `graphics().shadow_mapping`. `destroy` (`:148-154`): returns
early off-server, else `queue_free`. The bullet is spawned by `player/sync.rs::orient_and_anim`
(V3-B) under the player's parent with a collision exception; `bullet.tscn` replicates
`.:global_transform` (`:10-13`) — the client's node is a spawner-spawned replica.

**`Part`** (`part.rs:79-119`; three instances inside `red_robot.tscn` — `Death/PartShield1`
`:10830`, `Death/PartShield2` `:10881`, `Death/PartHead` `:10931` — each with its own
`MultiplayerSynchronizer` replicating `fade_value`, `position`, `rotation`, `linear_velocity`,
`angular_velocity`, `SceneReplicationConfig_hqtbc` `:10416-10431`): `#[export] lifetime = 3.0`,
`lifetime_random = 3.0`, `disappearing_time = 0.5` (`:86-94`), `#[export] #[var(set =
set_fade_value)] fade_value` (`:95-97`; the setter `:151-160` writes the duplicated material's
`next_pass` shader parameter `emission_cutout` — invoked by replication on clients),
`disappearing_counter` (`:99`), handles `MultiplayerSynchronizer`, `Col1`, `Col2`, the model
mesh by INDEX (`:101-115`), the preloaded `part_disappear.tscn` (`:116-118`). Pure `fade_curve`,
`should_destroy`, `random_angular_velocity`, `wait_time` (`:16-34`, 5 tests). `ready`
(`:123-136`): `set_process(false)`; material duplication unless `dedicated_server` — the
upstream bug fix #2 comment (`:129-130`) MUST stay. `process` (`:138-146`): `fade_curve` →
`set_fade_value`, counter += dt, `should_destroy` → `rpc("destroy")` + `set_process(false)`.
`explode` (`:162-192`): synchronizer visibility public, unfreeze (every peer); server only:
collisions on, `linear_velocity = 3·UP`, `random_angular_velocity(randf, randf, randf)`,
`wait_time(lifetime, lifetime_random, randf)` — FOUR `randf()` calls in this order (seeded
parity) — then `SceneTreeTimer(wait)` → `set_process(true)` (`:179-191`). `destroy`
(`:194-215`): instantiate the puff under `puff_parent()` (`:224-232`: the robot's parent, else
the robot, else the immediate parent — backlog #15) at the part's origin, then
`SceneTreeTimer(0.2)` → `queue_free` — on EVERY peer (`call_local`; the parts are scene children
of the robot, not spawner-spawned, so no replication despawn frees them on clients). The puff is
V3-A's bridge (`part_disappear.rs:28-30`, registers itself).

**`EnemyRobot`** (`red_robot.rs:30-106`): `#[var] test_shoot` (`:35-36`), replicated `#[export]`s
`target_position`, `health = 5`, `state: State { Idle, Approach, Aim, Shooting }` (`:21-28`,
`via = i64`), `dead` (`:38-47`; `SceneReplicationConfig_h6xi0` `red_robot.tscn:27-42` also
replicates `.:global_transform`), `#[var] aim_preparing` (`:48-50`, the aim blend's source, NOT
replicated), counters `shoot_countdown`/`aim_countdown` (`:52-55`), `player: Option<Gd<Player>>`
(`:58`), `orientation` (`:59`), `is_dedicated_server` read once (`:62-63`), `rid` (`:65-66`),
preloaded `impact_effect_scene` (`:68-69`), twelve `OnReady` handles (`:71-105`: `AnimationTree`,
`ShootAnimation`, `RedRobotModel`, `RayFrom: BoneAttachment3D`, `RayMesh`, `RayCast: RayCast3D`,
`LaserEmber`, `CollisionShape3D`, explosion/hit sounds, `Death`, the three `Part`s, two sparks).
`ready` (`:110-126`): orientation with zero origin, `animation_tree.set_active(true)`, RID,
`test_shoot` → `shoot_countdown = 0`, `dead` → model hidden/collision off/tree inactive,
`animate(0.0)`. `physics_process` (`:128-259`): `dead` → return (`:129-131`); non-server →
`animate(delta)` only (`:133-136`); `test_shoot` → `shoot()` + clear (`:138-141`); no player →
`target_position = ZERO`, `animate`, `idle_velocity`, `set_velocity`/`set_up_direction`/
`move_and_slide`, return — NO root motion, NO `set_global_basis` (`:143-151`); else
`target_position = player origin` (`:153`); `state_at_frame_start` captured once (`:160`);
`Approach` (`:162-196`): local angle (`:164-166`), `facing && shoot_countdown_will_expire` →
raycast pre-check from `RayFrom`'s origin to the player's origin + UP (`:177-184`, `hits_player`
`:506-511`) → `model::step` → counters + `apply_cmds` (`:186-196`); `Aim | Shooting`
(`:197-236`): `laser_raycast.is_colliding()` → `max_dist` → `_clip_ray` (`:199-205`, the `clip`
shader parameter unless dedicated server, `:492-501`), `Aim && aim_countdown_will_expire` →
raycast to `target_position + UP` (`:216-223`) → `step` → cmds (`RpcPlayShoot` → `rpc("play_
shoot")`, `ResumeApproach` → `resume_approach()`, `:384-395`); then `animate(delta)` (`:238`,
`:456-490`: `transition_request` from `model::transition_request`; when `target_position !=
ZERO`: `aiming/blend_amount` from `aim_preparing`, cannon angles from `RayMesh`'s global
transform, `aim/blend_position` READ from the tree with `get` (`:482-485`), `aim_blend_step`,
written back); root motion read (`:241-244`) → `integrate_root_motion` (`:245-251`, the model's
twin of the player's) → `set_velocity`/`set_up_direction`/`move_and_slide` (`:253-255`) →
`set_global_basis` (`:257-258`). `resume_approach` (`:267-274`): state `Approach`, counters from
`resume_approach_reset` — called by the shoot animation's method track (`red_robot.tscn:10297`)
AND by `Cmd::ResumeApproach`. `hit` (`:276-328`): `dead` guard; reaction
`parameters/hit{randi() % 3 + 1}/request = 1` (ONE `randi()`, seeded), hit sound; `hit_step` →
health (`:289-290`); on death (`:292-327`): `dead = true`, tree inactive, model hidden, `Death`
visible, collision off, both sparks emitting, the three parts' `explode()` TYPED (`:302-304`),
explosion sound, `exploded` emitted (`:307`), server → `SceneTreeTimer(removal_delay = 10)` →
`queue_free` (`:309-326`). `play_shoot` (`:330-333`): plays `shoot` on `ShootAnimation`, whose
method track calls `shoot_check` (`red_robot.tscn:10294`) and later `resume_approach`
(`:10297`). `shoot_check` (`:335-338`): `test_shoot = true`. `_on_area_body_entered/exited`
(`:340-358`; `PlayerDetectionArea` connections `red_robot.tscn:11044-11045`): `try_cast::<Player>`
→ `player = Some` + `Approach` / `None` + `Idle`. `shoot` (`:397-454`): raycast along `RayFrom`'s
`basis.col_b()` for 1000 m, `max_dist`, `_clip_ray`, ember position/extents (pure `:415-419`),
blast (`impact_effect.tscn`) instanced under the tree ROOT at the hit (`:423-425`), if the
collider is the tracked player → `SceneTreeTimer(trauma_delay = 0.1)` →
`player.bind_mut().add_camera_shake_trauma(13.0)` (`:437-453`; in V3-B that method pushes
`AddTrauma`). `RobotTuning` (8 fields, `model.rs:15-25`), `RayHit { position, collider:
Option<Gd<Object>> }` (glue, `:16-19`). `model::step` (`model.rs:116-191`), `hit_step`
(`:194-197`), `transition_request` (`:200`), `aim_blend_amount` (`:220`), `cannon_angles`
(`:226`), `aim_blend_step` (`:234`), `ember_position`/`ember_extents` (`:244-256`),
`integrate_root_motion` (`:259-280`), `idle_velocity` (`:282-284`).

**`red_robot.tscn:10779-10786`**: the robot's `AnimationTree` has `callback_mode_process = 0`
(`:10782`), `root_motion_track = Armature/Skeleton3D:MASTER`, a CHILD of the robot processed
AFTER the robot's `physics_process` in v2 and BEFORE the v3 driver — the V3-B situation exactly
(specs/012 research R1). The scene also seeds `parameters/aim/blend_position = (0.0048, 0.1266)`
and `parameters/aiming/blend_amount = 1.0` (`:10785-10786`). `bullet.tscn` and the parts have no
tree.

**External consumers, all unchanged**: `level.rs:130-140` (`instantiate_as::<EnemyRobot>`,
`set_transform`, the typed `exploded` connection to `_respawn_robot`, `add_child_ex`);
`player/sync.rs::orient_and_anim` (V3-B: `instantiate_as::<CharacterBody3D>` of `bullet.tscn`,
`add_child_ex` under the player's parent, `look_at`, `add_collision_exception_with`);
`hittable.rs` (`rpc_hit` by name on either class); `part.rs:117` keeps the puff preload.

**V3-A/V3-B infrastructure reused as is** (`src/ecs.rs`, `src/ecs/*`): the driver, the queue,
`Handles`/`PlayerHandles`, `EntityIndex`, `Timer`, the seven-set fixed `Phase` chain and the
four-set frame chain, `apply_register`, `sweep_dead_nodes`, `sync_out_remove`, `Tuning<T>`, the
`AddTrauma` drain arm (the robot's laser trauma reaches the player entity through it).

Baseline gates: `cargo build` / `cargo clippy` / `cargo test` clean, **179 tests** (33 of them the
three modules' pure tests, all preserved). Parity baseline: branch `v2` at `e2932b4`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The bullet as an entity (Priority: P1)

A bullet flies, expires after 5 s or hits something, explodes once, and is freed by its
animation's method track exactly as in v2, but it is an entity: the server's fixed tick steps its
lifetime in `Gameplay`, moves it with `move_and_collide` in `EngineQueryMove`, decides the hit in
`GameplaySettle`, and `SyncOut` applies the local explode effects, issues `rpc_hit` and the
`explode` RPC (remote peers only), while `destroy` becomes an event the sync layer turns into a
`Remove`.

**Why this priority**: the smallest of the three, and the one that exercises `move_and_collide`
plus the cross-class `rpc_hit` dispatch that US3 depends on.

**Independent Test**: `cargo test` passes for the bullet systems with the 3 `pure` tests
preserved; harness case (c) bullet expiry identical on `v2` and `v3`; case (b)'s bullets hit the
robot on the same steps.

**Acceptance Scenarios**:

1. **Given** `bullet.tscn` instanced by the player's tick (V3-B, unchanged), **When** `Bullet.ready`
   runs, **Then** it keeps v2's one-shot setup — non-server → collision shape disabled (`:91`;
   `set_physics_process(false)` becomes moot: the class has no `physics_process`) — and registers
   ONE entity with its handles (`AnimationPlayer`, `CollisionShape3D`, `OmniLight3D`, the
   `Settings` autoload's `shadow_mapping` read ONCE into the registration) and the initial
   components: `BulletState::Flying { time_alive: 5.0 }` (`:68`), `Simulates` iff
   `multiplayer.is_server()` (`:89`). `exit_tree` unregisters.
2. **Given** a `Simulates` bullet, **When** the fixed schedule runs, **Then** `SyncIn` reads the
   body's basis (`:112`); `Gameplay` runs `pure::step` (`:103`) → `BulletIntents { expired }`
   and returns early for an `Exploded` bullet (`:98-100`); `EngineQueryMove` calls
   `move_and_collide(-dt · VELOCITY · basis.col_c())` (`:111-113`) and records the collision —
   the collider's `InstanceId` when it is a `Node3D` (`:115-116`) — into a `Collided` snapshot;
   `GameplaySettle` decides: `hit_target = collided`, `disable_collision = collided`, `explode =
   expired || (collided && state was still Flying)` (backlog #13, `:123-127`), `state =
   Exploded` when collided (`:130`).
3. **Given** the decisions, **When** the rest of the run proceeds, **Then**: `EngineQueryMove`
   resolved the collider right after `move_and_collide` (`hittable::resolve`, `:117-119`, an
   engine `try_cast`) into a `Send` `HitKind::{Player(id), Robot(id)}` carried on the entity;
   `GameplaySettle` (bullet, pure) decides the explode intents (`:123-131`, backlog #13) and,
   for `HitKind::Robot(id)`, writes `Messages<RobotHitLocal { robot: id }>` — consumed by the
   robot's `hit_apply` system in the SAME `GameplaySettle` set, ordered after the bullet's
   (option (B)); `SyncOut` then applies, order-insensitively (the three writes touch different
   nodes; v2 interleaved them around `move_and_collide`, `:105-127`): the `explode` local
   effects once per fired intent (inline, option (b): play `explode`, shadow on when
   `shadow_mapping`, `:139-145`), the network `rpc_hit()` for the target (`call_remote` for the
   robot; for the player `hit` is `call_local` and its handler pushes `AddTrauma` — same
   iteration as v2's inline `add_camera_shake_trauma`), the collision shape disabled (`:122`),
   and `rpc("explode")` — `call_remote` — reaching remote peers only.
4. **Given** the remote `explode` handler (client bullet, non-`Simulates`), **When** it is invoked
   from the network, **Then** it only pushes `BulletFx::Explode { id }`; the client's frame
   `SyncOut` plays `explode` and sets the shadow (`:139-145`) exactly as v2's `call_local` handler
   did on every peer.
5. **Given** the animation's method track calling `destroy` at 1.5 s (`bullet.tscn:92-104`,
   every peer), **When** it fires, **Then** the bridge only pushes `BulletDestroy { id }`; the drain
   inserts `Remove` on a `Simulates` entity only (v2's server-only guard `:150-152`) and
   `sync_out_remove` frees the node; the client's replica is freed by the spawner's despawn as in
   v2 (Assumption (5)).
6. **Given** harness case (c) — one bullet spawned by the harness at frame 10 flying into empty
   space — **When** both trees log per step the origin and the state, and RAW the `explode` and
   `destroy` frames, **Then** the logs are identical.

---

### User Story 2 - The part as an engine-simulated entity (Priority: P1)

A robot's three parts fly off, fade and puff away exactly as in v2, but each part is an entity
with a phase: `Attached` until the robot dies, `Waiting` on a tick timer, `Fading` while the pure
fade curve runs, `Destroyed` when the puff is spawned and the node freed 0.2 s later; the body is
still simulated and replicated by the engine, and the fade reaches the shader and the wire through
the node's own setter.

**Why this priority**: the first engine-simulated body (no tick integrates it), the first
replicated shader parameter, the first entity whose lifecycle is driven by ANOTHER entity's event
(the robot's death), and four seeded RNG draws in glue.

**Independent Test**: `cargo test` passes for the part systems with the 5 `pure` tests preserved;
harness case (b)'s parts fade, puff and free on the same frames on both trees.

**Acceptance Scenarios**:

1. **Given** `red_robot.tscn` instanced (three `Part` children under `Death`), **When** each
   `Part.ready` runs (children before the robot), **Then** it keeps v2's one-shot setup
   (`set_process(false)` becomes moot; the material duplication with the upstream bug fix #2
   comment `:125-135` stays verbatim) and registers ITS OWN entity — a part is not a sub-bridge
   of the robot: it outlives the robot's death sequence, replicates independently, and is freed
   independently — with handles (`MultiplayerSynchronizer`, `Col1`, `Col2`, the puff scene, the
   node's own `Gd<Part>` for the setter) and the initial components `PartPhase::Attached`, the
   three exported lifetimes, `Simulates` iff `is_server()`. `exit_tree` unregisters.
2. **Given** the robot's death (US3), **When** the robot's `SyncOut` death branch runs in the
   SAME fixed run as the bullet's collision (option (B)), **Then** it applies v2's `explode`
   writes to each part NODE through the robot's `Gd<Part>` handles, in order (`:164-175`):
   synchronizer visibility public, unfreeze (every peer); on `Simulates`: collisions on,
   `linear_velocity = 3·UP`, `angular_velocity` from the draws, and sets the part ENTITY's phase
   to `Waiting(Timer(wait))` through `EntityIndex` (cross-entity component write from a
   `SyncOut` system — allowed glue) — so the parts' velocities take effect in the same physics
   step as v2, not one step later; the four `randf()` draws per part that produced
   `angular_velocity` and `wait` happened in the ROBOT's death branch in v2's call order (FR-014),
   not here. On non-`Simulates` the phase stays `Attached` (v2 returned at `:167-169`).
3. **Given** `Waiting`, **When** the frame schedule steps the `Timer` (V3-A's subtractive
   arithmetic, replacing `SceneTreeTimer(wait)` `:179-191`) to expiry, **Then** the phase becomes
   `Fading { counter: 0 }`; each frame in `Fading`, `Gameplay` computes `fade_curve(counter,
   disappearing_time)` (`:139`), advances the counter (`:141`) and decides `should_destroy`
   (`:142`); `SyncOut` writes the fade THROUGH the node's setter (`set_fade_value`, `:140`, so the
   replicated field and the shader parameter stay one path) and, when destroying, applies
   `destroy`'s local effects inline (option (b), `:196-200`: the puff instanced under
   `puff_parent()` at the part's origin) and starts `Destroyed(Timer(0.2))`; `rpc("destroy")`
   reaches remote peers only.
4. **Given** the remote `destroy` handler (client part, non-`Simulates`), **When** invoked from the
   network, **Then** it pushes `PartFx::Destroy { id }`; the client's frame `SyncOut` instances
   the puff at the part's origin and starts its own `Destroyed(Timer(0.2))` — v2's handler ran on
   every peer (`call_local`), and the client's part node is NOT freed by replication (it is a
   scene child, not spawner-spawned), so both peers free it through `Remove` when the 0.2 s timer
   expires (`:202-214`).
5. **Given** a client's part receiving `fade_value` by replication, **When** the engine sets the
   property, **Then** the setter writes the shader parameter (`:151-160`) — a projection-inbound
   engine write inside the bridge, allowed as glue, touching no component.
6. **Given** harness case (b), **When** both trees log per frame each part's `fade_value` and phase
   transitions (RAW: explode frame, fade start, puff spawn, free), **Then** the logs are identical
   (the parts' positions are logged but compared only if Assumption (4) holds).

---

### User Story 3 - The robot tick on the fixed schedule (Priority: P1)

The robot idles, approaches, aims, shoots its laser, reacts to hits and dies exactly as in v2, but
it is one entity: the pure state machine runs in `Gameplay`, the two raycast kinds are answered in
`EngineQueryOrient`, root motion is integrated and the body moved as the player's is, and `SyncOut`
projects the replicated fields, applies the RPCs' local effects and advances the tree.

**Why this priority**: the largest module and the second `AnimationTree`; every later review of
"an enemy with a tree" copies it.

**Independent Test**: `cargo test` passes for the robot systems with the 25 `model.rs` tests
preserved; harness cases (a), (b), (d), (e) identical (or every differing frame in the timing
table); both checkpoints.

**Acceptance Scenarios**:

1. **Given** `level.rs` instancing the robot, **When** `EnemyRobot.ready` runs, **Then** it keeps
   v2's one-shot setup (`animation_tree.set_active(true)`, `:113`; `dead` → hide/disable/tree
   inactive, `:119-123`) and registers ONE entity with the twelve handles (`:71-105`), the three
   parts as `Gd<Part>` handles (for their ids), `impact_effect_scene`, the RID and
   `is_dedicated_server` (read once, `:62-66`), and the initial components: `Orientation` with
   zero origin (`:111-112`), `RobotState(state)`, `Health`, `Dead`, `TargetPosition`,
   `RobotCounters { aim_preparing, shoot_countdown (0 when test_shoot, :115-117), aim_countdown }`
   (`:49-55`), `TrackedPlayer(None)`, `AimBlend` initialized ONCE from the scene's
   `parameters/aim/blend_position` (`red_robot.tscn:10785`; replaces v2's per-step `get`
   `:482-485`, one engine call fewer per step, behavior-identical — recorded), `Simulates` iff
   `is_server()`. v2's `animate(0.0)` at `ready` (`:125`) is subsumed by the first fixed run's
   parameter writes, which precede the tree's first `advance` (verified by the US4 experiment).
2. **Given** a `Simulates`, non-`Dead` robot, **When** the fixed schedule runs, **Then** `SyncIn`
   reads once per value: the body's global transform, gravity and velocity (`:164`, `:249-250`),
   the tracked player's origin when `TrackedPlayer` is `Some` (`:153`, `:181`), `RayFrom`'s and
   `RayMesh`'s global transforms (`:180`, `:478`), `laser_raycast.is_colliding()` and its
   collision point (`:200-202`, engine-updated at the previous physics step — Assumption (2)),
   and `ShootRequested` (from `shoot_check`) into the frame snapshot.
3. **Given** the snapshot, **When** `Gameplay` runs, **Then** it reproduces `:138-236` and
   `:456-490` in order without an engine call: the no-player branch (`target_position = ZERO`,
   `idle_velocity`, `:143-151`); else `target_position` = the player's origin; the angle and
   `facing`/`*_will_expire` gates decide `NeedsRaycast { from: RayFrom origin, to: target + UP }`
   (`:177-183`, `:216-222`); `max_dist` from the laser raycast snapshot (`:199-204`) and the
   `_clip_ray` intent; `test_shoot` → the shoot intent (`:138-141`) — all from `model.rs`,
   unchanged. NOT here: the animation decision — v2 calls `animate(delta)` AFTER `step` and
   `apply_cmds` (`:238`), with the NEW state and counters (a robot leaving `Approach` this step
   animates `idle` this step), so it belongs to the post-`step` pure step (scenario 4).
4. **Given** the intents, **When** `EngineQueryOrient` runs, **Then** it performs the pre-check/aim
   raycast when flagged (`raycast_to`, `:363-382`, mask `0xFFFFFFFF`, the robot's RID excluded)
   and records `sees_player` = the collider's id equals the tracked player's id (`:506-511`);
   performs the shoot raycast along `RayFrom`'s `basis.col_b()` when the shoot intent fired
   (`:399-409`) into `ShotResult { max_dist, hit: Option<(position, collider id)> }`; and reads
   the root motion (`:241-244`) — the value the previous `advance` produced, independent of this
   step's parameter writes (specs/012 research R1), so one query set suffices. Then a second
   `Gameplay` step (`GameplayIntegrate`'s pure half or a dedicated system — the plan pins the
   set) runs `model::step` with `sees_player` (`:186-196`, `:225-235`) → new state, counters,
   `Cmd`s; THEN the animation decision on the updated state (`transition_request`,
   `aim_blend_amount`, `cannon_angles`, `aim_blend_step` on `AimBlend`, v2 `animate` `:456-490`
   called at `:238`); THEN `integrate_root_motion` (`:245-251`); `EngineQueryMove` sets velocity/up direction
   and `move_and_slide`s (`:253-255`).
5. **Given** the tick's outputs, **When** `SyncOut` runs, **Then** in this order: `set_global_basis`
   (`:257-258`, not on the no-player branch `:150`); the projection of `state`,
   `target_position`, `health`, `dead`, `aim_preparing` into the node's fields through
   `bind_mut()` (guard dropped); `Cmd::RpcPlayShoot` → the local `shoot_animation.play("shoot")`
   inline (option (b), `:331-332`) then `rpc("play_shoot")` (`call_remote`); `Cmd::ResumeApproach`
   → already applied to the components by `step`; the shoot effects (`_clip_ray(max_dist)`, ember
   position/extents, the blast instanced under the tree root at the hit, `:411-425`; when the
   collider is the tracked player → `PendingTrauma(Timer(0.1))`, `:437-453`); the `_clip_ray`
   of the Aim/Shooting branch (`:205`, order-insensitive: nothing reads the shader parameter
   mid-tick, so it is a `SyncOut` write, not an `EngineQuery` one); the `AnimationTree` parameter
   writes (`transition_request`, `aiming/blend_amount`, `aim/blend_position`, `:469-488`);
   LAST, for EVERY non-`Dead` robot on every peer: `advance(FixedDelta)` (US4).
6. **Given** `PendingTrauma` expiring in a later frame run, **When** its `Timer` fires, **Then**
   `SyncOut` pushes `AddTrauma { root_id: the tracked player's id, amount: 13.0 }` to the queue,
   which the next run's drain applies to the PLAYER entity (V3-B's arm) — the same path v2's
   `add_camera_shake_trauma(13.0)` takes in V3-B, one schedule run after the timer instead of
   inside the timer's callback.
7. **Given** a non-`Simulates` robot (client), **When** the fixed schedule runs, **Then** `SyncIn`
   reads the replicated `state`, `target_position` and the local `aim_preparing` from the node
   (`:134`), `Gameplay` builds the replay animation decision as `animate` did (`:456-490`), and
   `SyncOut` writes the parameters and `advance`s; no movement, no raycast, no RPC.
8. **Given** the area signals (`:340-358`, physics-phase, emitted before `_physics_process`),
   **When** a `Player` body enters or exits, **Then** the handler pushes `RobotPlayerSeen { id,
   player: Some(instance_id) | None }` and the drain of the same step's fixed run sets
   `TrackedPlayer` and the state (`Approach`/`Idle`) — the same step as v2's inline write.
   Backlog #31 deferred: the counters are untouched (FR-023).
9. **Given** harness case (a) — the robot alone, a static `Player` body placed inside the
   detection area at frame 10 — **When** both trees log per step `state`, `target_position`,
   `aim_preparing`, the origin, the transition request and `aim/blend_position`, RAW the
   `play_shoot`, `shoot_check` and blast-spawn frames, **Then** the logs are identical.

---

### User Story 4 - The robot's `AnimationTree` ordering, proven again (Priority: P2)

The robot walks by root motion exactly as in v2 step by step, with the V3-B rule (MANUAL +
`advance` last in the robot's `SyncOut`) applied only after the R1-style experiment shows it holds
for this tree too.

**Why this priority**: the rule was proven on the player's tree; the robot's tree has a different
root-motion track and blend tree, and the constitution's parity criterion forbids assuming.

**Independent Test**: a per-step log of `get_root_motion_position()`, the transition request and
the origin on `v2` and on `v3` (research experiment, then harness case (e)) is identical.

**Acceptance Scenarios**:

1. **Given** the three options of specs/012 US4, **When** research re-runs `zz_r1_probe.gd`'s
   shape on `red_robot.tscn` (a floor, one robot with a static `Player` body 8 m away inside the
   area so it walks; probes at `i32::MIN` and `i32::MAX`; runs (i) `v2` PHYSICS, (ii) `v3`
   PHYSICS, (iii) `v3` MANUAL + `advance` after the tick), **Then** the option whose per-step log
   equals v2's is implemented; (B) is expected and is the ONE `.tscn` edit of this milestone
   (`red_robot.tscn:10782` `callback_mode_process` 0 → 2).
2. **Given** option (B), **When** a robot is non-`Simulates` or `Dead`, **Then** its tree is
   advanced every fixed run while alive on every peer (the replay parameters precede it) and NOT
   advanced once `Dead` (v2 set the tree inactive at death, `:294`).
3. **Given** the decision, **When** it lands, **Then** `docs/v3-tradeoffs.md` records it and
   `CLAUDE.md`'s v3 section notes that the rule held for a second tree.

---

### Edge Cases

- **Bullet expiry and collision in the same step** (backlog #13, `bullet.rs:123-127`): both
  intents fire; `explode`'s local effects and the RPC are issued ONCE — the settle decision
  suppresses the collision-driven explode when expiry already fired that step.
- **A bullet hitting a dead robot**: the local dispatch still reaches `hit_apply`, which runs
  v2's `dead` guard (`:278-280`) — no reaction, no sound, no health change; the network
  `rpc("hit")` still goes out (v2 sent it too).
- **`rpc_hit` from `SyncOut` is synchronous locally only for the player**: `Player::hit` is
  `call_local` and its handler pushes `AddTrauma` (push-never-borrows). The robot's `hit` is
  `call_remote` (option (B)), so nothing runs locally from the RPC; the local path is the
  message of the bullet's `GameplaySettle`. The bullet's collider is carried as a `Send`
  `HitKind` resolved in `EngineQueryMove` (same run, right after `move_and_collide`); a collider
  freed within the run is impossible (nodes are freed only by `sync_out_remove`, at the end).
- **The robot's `hit` timing — option (B), user-approved 2026-09-19**: v2 applied the whole
  reaction and death sequence inside the bullet's physics step (`rpc_hit` → `hit`
  synchronously, `:117-121` → `:276-328`). Routing it through the queue would apply the death in
  the frame run and the parts' velocities in the NEXT fixed run — one physics step late, every
  part position off by one step for the whole fall (rejected). Instead, in the SAME fixed run:
  the bullet's `GameplaySettle` writes `RobotHitLocal { robot }`; the robot's `hit_apply`
  (`GameplaySettle`, ordered after the bullet's settle) runs the `dead` guard and `hit_step`
  (`:278-290`, pure) and flags the reaction/death intents; the robot's `SyncOut` (same run)
  draws `randi() % 3 + 1` (glue, `:285`), writes the reaction parameter, plays the hit sound
  and, on death, applies v2's sequence (`:293-326`): `dead` projection, tree inactive, model
  hidden, `Death` visible, collision off, sparks, the three parts exploded DIRECTLY through the
  robot's `Gd<Part>` handles with the twelve `randf()` draws in v2's per-part order (FR-014;
  the part entities' phases set through `EntityIndex`), explosion sound, the `exploded` signal
  emitted from glue, and on the server `RemovalTimer(10 s)` → `Remove`. Physics effects,
  replication samples and the observer all see the same step as v2. Remote peers receive
  `rpc("hit")` (`call_remote`); their handler pushes `RobotHit { id }` and their frame run
  applies the visual half (reaction — with their own `randi()`, as v2's remote handler drew
  its own — sound, death visuals, parts' visibility/unfreeze without velocities, `:167-169`).
- **`exploded` emitted from glue**: `level.rs`'s typed connection runs `_respawn_robot`
  synchronously inside the robot's `SyncOut`; it only spawns a `SceneTreeTimer` task (v2 code,
  unchanged) — no World access.
- **The parts' RNG order** (FR-014): v2 drew `randi()` (reaction) then, per part in scene order
  (`PartShield1`, `PartShield2`, `PartHead`, `:302-304`), `randf()` ×3 (angular) and ×1 (wait),
  all inside `hit`. v3 draws the `randi()` and the twelve `randf()` in the robot's `SyncOut`
  death branch of the SAME fixed run, in the same order (reaction first, then per part) — the
  part entities never draw. No other RNG consumer runs in between (the camera's `randi()` is at
  instantiation only), so the sequence matches v2's for `seed(1)`. Case (b) logs each part's
  `angular_velocity` on its explode frame.
- **The part's `Waiting` timer on the client**: never started (v2's `explode` returned before the
  timer off-server, `:167-169`); the client's part fades by replication of `fade_value` and is
  destroyed by the remote `destroy` handler's `PartFx::Destroy`.
- **`resume_approach` from the method track vs `Cmd::ResumeApproach`**: the method track's
  `#[func]` pushes `ResumeApproachRequested { id }`, applied by the next fixed run's drain
  (`Approach` + `resume_approach_reset`, `:268-273`); `Cmd::ResumeApproach` from `step` is
  already applied to the components by `step` itself (`model.rs:170-176`) — nothing further.
- **`shoot_check` from the method track**: pushes `ShootRequested { id }`; the next fixed run's
  `Gameplay` consumes it as v2's `test_shoot` flag (`:138-141`) — the `#[var] test_shoot` field
  stays as the registration input (`ready`, `:115-117`) and is no longer written at runtime.
- **The `ShootAnimation` player and the robot's tree**: `ShootAnimation` is a separate
  `AnimationPlayer` (idle processing, `red_robot.tscn:10814`), not the MANUAL tree; its method
  tracks fire during its own processing, before the driver in tree order.
- **A robot dying while `PendingTrauma` is pending**: the timer keeps running on the entity
  (v2's task checked `this.is_instance_valid()` on the robot only after the await); the push
  happens when it expires, as v2 did while the robot node lived; if the robot was removed, the
  entity is gone and the timer with it (v2: the task returned early).
- **`Dead` robots at `ready`** (scene-set `dead = true`): registered `Dead`; the tick skips them,
  the tree is never advanced (v2: inactive at ready, `:122`).

## Requirements *(mandatory)*

### Functional Requirements

**Entities, bridges, authority (US1, US2, US3)**

- **FR-001**: Three entity kinds MUST exist, registered by their own bridge's `ready` and
  unregistered in `exit_tree`: `Bullet` (one entity per `bullet.tscn`), `Part` (one entity per
  part node — three per robot; a part is NOT a sub-bridge of the robot), `EnemyRobot` (one
  entity per robot; `Death`, the sparks, the sounds, the ray nodes, the shoot `AnimationPlayer`
  and the three `Gd<Part>` are handles). Sub-bridges are not used in this milestone.
- **FR-002**: `Simulates` MUST be set at registration from `multiplayer.is_server()` on all
  three (`bullet.rs:89`, `part.rs:167`, `red_robot.rs:133`); every gameplay and sync system of
  this milestone filters by it or by `Without<Simulates>` for the remote-only paths.
- **FR-003**: Each bridge's `ready` MUST keep only v2's one-shot engine setup: bullet `:88-93`
  (collision shape off on non-server), part `:123-136` (material duplication with the upstream
  bug fix #2 comment kept verbatim), robot `:110-125` (tree active, RID, `dead` handling; the
  `animate(0.0)` is subsumed by the first fixed run). No bridge MUST have `process` or
  `physics_process`.
- **FR-004**: The RPC/`#[func]` handlers MUST keep their exact names and signatures and MUST only
  push events keyed by the node's own `InstanceId`: bullet `explode` → `BulletFx::Explode`,
  `destroy` → `BulletDestroy`; part `destroy` → `PartFx::Destroy`, `explode` (kept `#[func]
  pub(crate)`, no caller after this milestone — see FR-014; the plan decides whether it stays as
  a thin wrapper or is removed); robot `hit` → `RobotHit { id }` (REMOTE peers only under option
  (B); the reaction index is drawn by whoever applies it — `SyncOut` locally, the remote's frame
  run on clients — as v2's per-peer handler drew its own), `play_shoot` →
  `RobotFx::PlayShoot`, `shoot_check` → `ShootRequested`, `resume_approach` →
  `ResumeApproachRequested`, `_on_area_body_entered/exited` → `RobotPlayerSeen { player:
  Option<InstanceId> }` after the `try_cast::<Player>` at the boundary (`:345`, `:353`).
  Attribute changes under option (b), same rationale as specs/012 FR-004: `explode` (bullet),
  `destroy` (part), `play_shoot` (robot) AND `hit` (robot) become `#[rpc(authority, call_remote,
  unreliable)]` — their local effects are applied by the simulating peer inside the fixed run
  (the robot's hit through the bullet's message, option (B)); `hit` (player) keeps `call_local`
  (its local path is the `AddTrauma` event, same iteration). `hittable.rs` gains `HitKind`
  (`Send` ids) and keeps `rpc_hit` for the network. `set_fade_value` keeps `#[func]` and its
  setter role (FR-011).

**The bullet tick (US1)**

- **FR-005**: The bullet's tick MUST run on the FIXED schedule on `Simulates` entities in v2's
  order: `SyncIn` (basis) → `Gameplay` (`pure::step`, `:98-107`, the `Exploded` early return) →
  `EngineQueryMove` (`move_and_collide`, `:111-113`, the collider's `InstanceId`) →
  `GameplaySettle` (the hit/explode decisions of `:114-131`, backlog #13's suppression) →
  `SyncOut` (scenario 3). The bullet uses no `EngineQueryOrient`. `pure::step` and its 3 tests
  stay in `bullet.rs`'s `mod pure` untouched.
- **FR-006**: `EngineQueryMove` MUST resolve the collider (`hittable::resolve`, `:117-119`)
  into `HitKind` right after `move_and_collide`; `GameplaySettle` MUST decide the explode
  intents (`:123-131`) and write `RobotHitLocal` for a robot target (option (B)); `SyncOut` MUST
  apply, order-insensitively: the explode local effects (play `explode`; shadow when the
  registration's `shadow_mapping` is true, `:139-145`), the network `rpc_hit()` (`:117-121`;
  `call_local` for the player, `call_remote` for the robot), collision shape disabled (`:122`),
  and `rpc("explode")` (`call_remote`) once per explode intent; the remote handler's
  `BulletFx::Explode` is applied by the frame `SyncOut` on non-`Simulates` entities.
- **FR-007**: `destroy` MUST become `BulletDestroy`; the drain MUST insert `Remove` only on a
  `Simulates` entity (`:150-152`); the node is freed only by `sync_out_remove`.

**The part (US2)**

- **FR-008**: `PartPhase { Attached, Waiting(Timer), Fading { counter }, Destroyed(Timer) }`
  MUST be the part entity's state; `Attached → Waiting` when the robot's death branch explodes it
  (`Simulates`, through `EntityIndex`; on remote peers the phase stays `Attached` — v2's client
  parts never fade on their own, `:167-169`), `Waiting →
  Fading` on the timer (`:179-191`, V3-A's `Timer` arithmetic), `Fading → Destroyed` on
  `should_destroy` (`:142-144`), `Destroyed → Remove` on its 0.2 s timer (`:202-214`). The
  timers step on the FRAME schedule (v2's `SceneTreeTimer`s and `process` were frame-driven).
- **FR-009**: The robot's `SyncOut` death branch (FR-014) MUST apply v2's `explode` writes to
  each part node in order (`:164-175`): visibility public and unfreeze on every peer; on
  `Simulates` collisions on, `linear_velocity = 3·UP`, the drawn `angular_velocity`, and the
  part entity's `Waiting(Timer(wait))` — all in the same fixed run as the hit (option (B)). No
  `PartExplode` event exists.
- **FR-010**: In `Fading`, `Gameplay` MUST call `fade_curve` and `should_destroy` unchanged
  (`:139-142`) and advance the counter by the frame delta; the 5 `pure` tests stay.
- **FR-011**: `SyncOut` MUST write the fade through the node's setter (`root.bind_mut().
  set_fade_value(fade)`, `:140`, `:151-160`) on `Simulates` — the replicated field and the shader
  parameter are one path; on clients the engine's replication invokes the same setter (a
  projection-inbound write inside the bridge, touching no component).
- **FR-012**: `destroy`'s local effects (the puff instanced under `puff_parent()` at the part's
  origin, `:196-200`, `:224-232`) MUST be applied inline by the simulating peer's `SyncOut` and
  by the remote handler's `PartFx::Destroy` on clients; both then run `Destroyed(Timer(0.2))` →
  `Remove` — the client frees its own node as v2's `call_local` handler did (Edge Cases).
- **FR-013**: The puff is V3-A's bridge and registers itself; the part's preload of
  `part_disappear.tscn` (`:117`) stays with the part.

**The robot (US3, US4)**

- **FR-014**: The robot's death branch in `SyncOut` MUST, in v2's order (`:293-326`): project
  `dead`, set the tree inactive, hide the model, show `Death`, disable the collision, start both
  sparks, then for each part in scene order (`PartShield1`, `PartShield2`, `PartHead`) draw
  `randf()` ×3 → `random_angular_velocity` and `randf()` ×1 → `wait_time(lifetime,
  lifetime_random, r)` (the part's exported lifetimes, read from the part's handle) and apply
  FR-009's writes to that part directly (node through the `Gd<Part>` handle, entity phase through
  `EntityIndex`); play the explosion sound; emit `exploded`; on the server start
  `RemovalTimer(removal_delay)` → `Remove`. The reaction `randi()` precedes the twelve `randf()`;
  all thirteen draws happen in glue in this order, so the seeded sequence matches v2 (Edge
  Cases). The branch runs in the SAME fixed run as the bullet's collision (option (B)).
- **FR-015**: The robot's tick MUST run on the FIXED schedule on `Simulates`, non-`Dead` entities
  with the seven sets: `SyncIn` (scenario 2) → `Gameplay` (scenario 3, pure) → `EngineQueryOrient`
  (scenario 4: the pre-check/aim raycast, the shoot raycast, the root-motion read) →
  `GameplayIntegrate` (`model::step` with the raycast answer, THEN the animation decision on the
  updated state — v2 `:238` after `:187`/`:226`, THEN `integrate_root_motion` or
  `idle_velocity`; also the `hit_apply`-driven `Dead` transition is visible here, see FR-019) →
  `EngineQueryMove` (`set_velocity`, `set_up_direction(UP)`,
  `move_and_slide`, `:253-255`; also on the no-player branch, `:146-149`) → `GameplaySettle`
  (the robot's `hit_apply`, option (B), FR-019 — ordered after the bullet's settle) → `SyncOut`
  (scenario 5, plus the hit/death branch of FR-014/FR-019). Every `model.rs` function is called
  unchanged; the 25 tests stay.
- **FR-016**: `AimBlend(Vector2)` MUST be a component initialized once at registration from the
  scene's `parameters/aim/blend_position` (`red_robot.tscn:10785`) and stepped by
  `aim_blend_step` in `Gameplay`; `SyncOut` writes it to the tree. This replaces v2's per-step
  `get` (`:482-485`) — behavior-identical, one engine call fewer, recorded in
  `docs/v3-tradeoffs.md`.
- **FR-017**: `TrackedPlayer(Option<InstanceId>)` MUST be the robot's player reference (no `Gd`
  in components); `SyncIn` reads the player's origin through the handle re-fetched from the id
  (or through the player entity's handles — the plan decides, Assumption (6)); `sees_player` and
  the laser's "hit the player" test compare instance ids (`:506-511`, `:428-432`).
- **FR-018**: The shoot sequence (`:397-454`) MUST split as: `ShootRequested` (from
  `shoot_check`, or `test_shoot` at registration) → `Gameplay` shoot intent → `EngineQueryOrient`
  raycast → `SyncOut`: `_clip_ray(max_dist)` (skipped on dedicated servers, `:494`), ember
  position/extents (pure `ember_position`/`ember_extents`), the blast instanced under the tree
  root at the hit position, and `PendingTrauma(Timer(trauma_delay))` when the collider is the
  tracked player; on expiry (frame schedule) `SyncOut` pushes `AddTrauma { root_id: player,
  amount: trauma_amount }` (scenario 6).
- **FR-019**: The local hit MUST be applied as the Edge Cases state (option (B)): the bullet's
  `GameplaySettle` writes `Messages<RobotHitLocal { robot: InstanceId }>`; the robot's
  `hit_apply` system, in `GameplaySettle` ordered after the bullet's settle, resolves the id
  through `EntityIndex`, runs the `dead` guard and `hit_step` (`:278-290`, pure) and flags the
  reaction/death intents; the robot's `SyncOut` of the same run draws the reaction `randi()`,
  writes the parameter and plays the hit sound on every hit of a live robot, then FR-014 on
  death. The remote `RobotHit { id }` (from the `call_remote` RPC) is applied by the frame run on
  non-`Simulates` robots: reaction (own `randi()`), sound, death visuals, parts' visibility/
  unfreeze.
- **FR-020**: `RobotPlayerSeen` MUST set `TrackedPlayer` and the state (`Approach` on `Some`,
  `Idle` on `None`, `:345-356`) at drain time — the same physics step as v2 (the area signal is
  emitted before `_physics_process`, specs/011 research R1). `ResumeApproachRequested` MUST apply
  `Approach` + `resume_approach_reset` at drain time (`:268-273`).
- **FR-021**: The non-`Simulates` replay path MUST reproduce `:133-136`: `SyncIn` reads the
  replicated `state`/`target_position` and the node's `aim_preparing`; `Gameplay` builds the
  animation decision as `animate` does; `SyncOut` writes the parameters and `advance`s; no
  movement, raycast or RPC.
- **FR-022**: The `AnimationTree` ordering MUST be settled by re-running the R1 experiment on the
  robot (US4 scenario 1) before the `.tscn` edit; if (B), `red_robot.tscn:10782` becomes
  `callback_mode_process = 2` and `SyncOut` calls `advance(FixedDelta)` LAST for every
  non-`Dead` robot on every peer, after the parameter writes; the edit is the milestone's one
  scene change and is listed in `docs/v3-tradeoffs.md`.
- **FR-023**: Backlog #31 variant: by default the drain does NOT reset the counters on `Approach`
  entry (v2 behavior). If the user closes #31 at plan review, the drain applies
  `resume_approach_reset` on `RobotPlayerSeen { Some }` and the backlog row is marked done in
  the closing commit; the harness case (a) then records the difference in the timing table as
  the sanctioned behavior change.

**Preserved surfaces and scope (Principle I v3, Principle II)**

- **FR-024**: These names and types MUST NOT change: the replicated properties `.:global_transform`,
  `.:health`, `.:state`, `.:target_position`, `.:dead` (`red_robot.tscn:27-42`), `.:fade_value`,
  `.:position`, `.:rotation`, `.:linear_velocity`, `.:angular_velocity` (`:10416-10431`),
  `.:global_transform` (`bullet.tscn:10-13`); `#[var] test_shoot`, `#[var] aim_preparing`; the
  `#[export]`s `lifetime`, `lifetime_random`, `disappearing_time`, `fade_value`; `#[signal]
  exploded`; the `#[func]` names `resume_approach`, `shoot_check`, `_on_area_body_entered`,
  `_on_area_body_exited`, `destroy` (bullet), `explode` (part, `pub(crate)`), `set_fade_value`;
  the `#[rpc]` names `hit`, `play_shoot`, `explode` (bullet), `destroy` (part) with their
  signatures; `Bullet::VELOCITY`; the `State` enum and its `via = i64` wire values; the class
  names `Bullet`, `Part`, `EnemyRobot`; `HitTarget`/`resolve`/`rpc_hit` (additive: `HitKind`).
- **FR-025**: `level.rs`, `player/sync.rs`, `player.rs`, `player_input*`,
  `camera_noise_shake*`, `door*`, `part_disappear*`, `blast.rs`, `flying_forklift.rs`,
  `settings*`, `menu*`, `main_scene.rs`, `debug_label.rs` MUST be untouched; the three pure
  cores (`bullet.rs`'s `mod pure`, `part.rs`'s `mod pure`, `red_robot/model.rs`) MUST be
  byte-identical except that an inline `mod pure` MAY move to `x/model.rs` verbatim if the plan
  needs it importable (the plan decides; the tests stay by name). `hittable.rs` changes ONLY
  additively (`HitKind`, a local-dispatch helper); `resolve`/`rpc_hit` keep their signatures.
  `red_robot.tscn` changes only
  by FR-022's property; no other scene changes.
- **FR-026**: Backlog: #31 per FR-023; #29 observed at checkpoint (1); #30 stays open; no v3
  backlog file.

**Cross-cutting (constitution 1.5.2 compliance)**

- **FR-027**: No bridge (`Bullet`, `Part`, `EnemyRobot`) MUST have `process`/`physics_process`; no
  engine callback MUST borrow the World (the handlers push; the player's `call_local` `hit`
  invoked from the bullet's `SyncOut` only pushes); engine access MUST appear only in `SyncIn`/`EngineQuery*`/`SyncOut`
  systems and bridges; components MUST hold no `Gd<T>` (`TrackedPlayer` and the bullet's
  collider are `InstanceId`s); nodes freed only through `Remove` and `sync_out_remove` with
  `queue_free()` (bullet on the server, parts on every peer, robots on the server after 10 s);
  residual dynamic access limited to the top block's list.
- **FR-028**: `docs/v3-tradeoffs.md` MUST gain one row per engine touch point, in the introducing
  commit: `move_and_collide` + collider resolution (bullet); the `RigidBody3D` simulated and
  replicated by the engine, with the shader parameter written through the node's setter (part);
  the puff instanced from the part's `SyncOut`; the robot's raycasts (pre-check/aim and shoot)
  in `EngineQueryOrient`; the laser `RayCast3D` read at `SyncIn`; the robot's `AnimationTree`
  MANUAL + `advance` (second tree, US4's evidence) and `AimBlend` replacing the tree `get`;
  engine RNG draws in glue for seeded parity (the robot's death branch); the `exploded` signal
  emitted from glue for v2's `level.rs`; the blast instanced under the tree root. `CLAUDE.md`'s
  v3 section MUST be extended with what is new: entity-to-entity communication inside one run
  through `Messages` ordered across sets (`RobotHitLocal`) versus through the queue across runs
  (`AddTrauma` from a timer), cross-entity handle and component access from a `SyncOut` system
  (the robot exploding its parts), the engine-RNG-in-glue rule, timers that push events on
  expiry, and the corollary of option (b): an RPC whose local effects must land in the SAME run
  as their cause is applied through a message, never through the queue.
- **FR-029**: Gates unchanged: `cargo build`, `cargo clippy` 0 warnings, `cargo test` green with
  the 33 pure tests preserved and at least one `run_system_once` test per new gameplay system
  (count ≥ 179 + the new tests).
- **FR-030**: Parity with `v2` MUST be evidenced by the headless harness on both trees (V3-B's
  recipe: `v2` worktree at `e2932b4`, split `XDG_DATA_HOME`, `--fixed-fps 60`, `seed(1)` FIRST,
  the joypad-binding purge, observer at `i32::MIN`, the physics probe, RAW lines, diffs pasted)
  with these five cases: (a) robot alone — a static `Player` body placed inside the detection
  area at frame 10, `--quit-after 900`; per step `state`, `target_position`, `aim_preparing`,
  origin, the transition request, `aim/blend_position`; RAW `play_shoot`/`shoot_check`/blast
  spawn frames; (b) robot shot to death — bullets instanced by the harness from `bullet.tscn`
  every 30 frames at a fixed point aimed at the robot (the harness reproduces the player's spawn
  path: `instantiate_as`, `add_child`, `set_global_position`, `look_at`), `--quit-after 900`;
  per step `health`, `dead`, each part's `fade_value`, phase-derived flags and
  `angular_velocity` on its explode frame, positions (compared per Assumption (4)); RAW hit
  frames, death frame, puff spawn frames, part free frames, robot removal frame; (c) bullet
  expiry alone — one bullet at frame 10 into empty space; per step origin and state; RAW
  `explode` frame (animation playing) and `destroy`/free frame; (d) laser hitting the player —
  the static `Player` body in front of the robot, `test_shoot = true` set before `add_child`;
  RAW blast spawn frame, the trauma arrival frame (the player's camera rotation changing), the
  `clip` shader parameter per frame when readable (`get_shader_parameter("clip")`); (e) root
  motion — the robot walking toward a player placed 8 m away; per step origin and root-motion
  position (the US4 experiment's log format). Every differing frame goes to the "Measured timing
  differences" table with its cause.
- **FR-031**: User visual checkpoints, both REQUIRED: (1) single player — a robot approaches,
  aims, fires the laser with clip/ember/blast, the player is hit (shake after 0.1 s), the player
  kills a robot (parts fly, fade, puff, the robot disappears after 10 s and respawns), bullets
  explode on walls and on expiry, all as in v2; note backlog #29. (2) multiplayer on one machine —
  the client sees the host's robots move, aim and shoot; parts fly and fade on the client; a
  client's bullet kills a robot and the death replicates (parts, puffs, removal).

### Key Entities

- **Bullet entity**: `BulletState` (the pure enum), `BulletIntents { expired, explode, hit:
  Option<InstanceId>, disable_collision }`, `Collided { collider: Option<InstanceId> }`,
  `PendingBulletFx` (remote path), `Simulates`; handles `AnimationPlayer`, `CollisionShape3D`,
  `OmniLight3D`; registration carries `shadow_mapping`.
- **Part entity**: `PartPhase` (FR-008), `PartLifetimes { lifetime, lifetime_random,
  disappearing_time }`, `FadeValue` (the projection), `PendingPartFx`, `Simulates`; handles
  `Gd<Part>` (the setter), `MultiplayerSynchronizer`, `Col1`, `Col2`, the puff scene.
- **Robot entity**: `RobotState(State)`, `Health`, `Dead`, `TargetPosition`, `RobotCounters`,
  `AimPreparing` (in the counters), `TrackedPlayer(Option<InstanceId>)`, `Orientation`,
  `RootMotion`, `AimBlend`, `RobotIntents` (raycast requests, shoot, cmds), `RaycastAnswers`,
  `ShootRequested`, `PendingTrauma(Timer)`, `RemovalTimer(Timer)`, `PendingRobotFx`, `Simulates`;
  handles the twelve nodes, the three `Gd<Part>`, the impact scene, the RID, `is_dedicated_server`.
- **Inbound events**: `Register`/`Unregister` (each bridge), `BulletFx::Explode`, `BulletDestroy`,
  `PartFx::Destroy`, `RobotHit { id }` (remote peers only),
  `RobotFx::PlayShoot`, `ShootRequested`, `ResumeApproachRequested`, `RobotPlayerSeen { id,
  player }`, and V3-B's `AddTrauma` (pushed by the robot's trauma timer).
- **Sets**: the seven-set fixed chain (bullet: `Gameplay`, `EngineQueryMove`, `GameplaySettle`,
  `SyncOut`; robot: all seven; part: `SyncOut` only) and the four-set frame chain (parts' timers
  and fade; the robot's trauma/removal timers; remote Fx).
- **Replicated projections**: robot `state`, `target_position`, `health`, `dead` (written by the
  robot's `SyncOut` on `Simulates`, read by `SyncIn` elsewhere); part `fade_value` (written
  through the setter on `Simulates`; the engine writes the body's transform/velocities); bullet
  `global_transform` (engine).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build` / `cargo clippy` (0 warnings) / `cargo test` clean; the 33 pure tests
  of the three modules preserved by name; at least one `run_system_once` test per new gameplay
  system (bullet step/settle, part fade/phase, robot decide/step/replay) — total ≥ 179 + 10.
- **SC-002**: `grep -n 'fn process\|fn physics_process' bullet.rs part.rs red_robot.rs` returns
  nothing; `godot::task::spawn` and `bind_mut()` on `EcsWorld` appear nowhere in the three
  modules; the three modules' new `system.rs` files import nothing from `godot::classes`.
- **SC-003**: Headless import and `main.tscn`/`level.tscn` show no new errors after every
  commit (the catalogued intermittent bursts excluded).
- **SC-004**: Harness cases (a)–(e) produce identical logs on `v2` and `v3`, OR every differing
  frame is listed in the "Measured timing differences" table with its cause — no third outcome.
  The RAW stamps of (b) show each hit, the death, the three explode frames and the puffs.
- **SC-005**: Both visual checkpoints pass, including the two-instance multiplayer session.
- **SC-006**: The constitution 1.5.2 review checklist passes: no bridge with per-frame callbacks;
  engine access only in sync/query systems and bridges; components without `Gd`;
  `docs/v3-tradeoffs.md` has FR-028's rows; FR-025's untouched files diff-empty against
  `30d1a4d`; `red_robot.tscn` diff limited to FR-022's property; backlog #29/#30 open and #31
  per FR-023.
- **SC-007**: The robot `AnimationTree` experiment's per-step log (root motion position,
  transition request, origin) is identical on both trees with the implemented option, before the
  harness runs.

## Measured timing differences

Filled during implementation, before the closing commit. An empty table means the five harness
diffs (and the US4 experiment) were empty.

| Case | Signal / timer | `v2` frame | `v3` frame | Delta | Cause |
|---|---|---|---|---|---|
| (to be measured) | | | | | |

Candidates the harness must settle: the robot's `hit` applied in the same fixed run through
`RobotHitLocal` (option (B): expected identical, including the parts' first moving step); the part's `Waiting`/`Destroyed` timers as tick timers
instead of `SceneTreeTimer`s (V3-A's arithmetic: same step expected); the trauma pushed one
schedule run after the 0.1 s timer; `resume_approach` from the method track applied at the next
fixed run's drain; the blast/puff instanced from `SyncOut` at the end of the phase instead of
inside the callback (same iteration).

## Assumptions

- Phase v3 (constitution 1.5.2). Zero behavior deviations from `v2` are sanctioned beyond
  measured frame-level shifts recorded in the table, the `call_remote` attributes of FR-004
  (same-step local effects, as in V3-B), the one scene edit of FR-022 (no observable behavior
  change is its acceptance criterion) and, only if the user closes it, backlog #31.
- **To verify at research time**: (1) the robot `AnimationTree` experiment (US4 — the R1 twin,
  not assumed); (2) `RayCast3D::is_colliding()`/`get_collision_point()` reflect the previous
  physics step's update (engine-updated on the physics tick) so `SyncIn` is the right read point
  and v2's read inside `physics_process` saw the same values; (3) `move_and_collide` from an
  `EngineQuery` system inside the driver's `physics_process` behaves as v2's call inside the
  bullet's own `physics_process` (same physics step, same server-side state); (4) `RigidBody3D`
  determinism across the two trees at `--fixed-fps 60` with `seed(1)` — if the parts'
  trajectories diverge for engine reasons (different physics-server insertion order, not the
  tick), the harness compares `fade_value`, phase frames and the explode-frame
  `angular_velocity`, not positions, and says so; (5) the client's bullet replica is freed by
  the spawner's despawn when the server frees its node (v2's `destroy` returns early off-server)
  — cite `scene_replication_interface.cpp`; (6) reading the tracked player's origin:
  `Gd::<Node3D>::try_from_instance_id` per step versus a lookup through `EntityIndex` +
  `NodeHandles` — the plan picks one and records the FFI count; (7) `Node::get_tree().get_root()`
  from a `SyncOut` system for the blast (v2 `:424`) and `add_child` mid-schedule (the blast is
  V3-A's bridge and registers itself through the queue — push-never-borrows); (8) the parts'
  `explode` handler order and the twelve `randf()` draws reproduce v2's sequence for `seed(1)`
  (case (b)'s `angular_velocity` log is the proof).
- Exact Rust shapes (component and event names, whether intents are markers or fields, whether
  the robot's second pure step lives in `GameplayIntegrate` or a new system in that set, the
  exact `Messages<RobotHitLocal>` update rule, the tracked-player read path)
  are plan-time decisions; the spec pins behavior and order.
- Out of scope: `flying_forklift` (constitution 1.5.2), `level`, `menu`, `main_scene`,
  `settings`, `debug_label`; backlog fixes other than the #31 decision; changes to
  `player/sync.rs` (`hittable.rs` changes only additively, FR-025); any change to the replication configs, the `State` enum or the scenes
  beyond FR-022.
