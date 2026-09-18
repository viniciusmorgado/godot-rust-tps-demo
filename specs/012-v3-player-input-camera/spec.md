# Feature Specification: Milestone V3-B — the player: `player`, `player_input`, `camera_noise_shake` as one entity over three nodes

**Feature Branch**: `v3` (work directly, no per-milestone branch; baseline for this milestone is
`abfe35a`, V3-A complete, constitution 1.5.1. Local commits only, never pushed.)

**Created**: 2026-09-18

**Status**: Draft

**Phase**: v3 — ECS layer over the nodes (constitution 1.5.1, Principles I, II and III including
the "ECS shape (v3)" subsection). Both v3 objectives apply, done together in every user story:
(1) v2's two pillars are KEPT — the 36 pure functions/tests of `player/model.rs`,
`player_input/model.rs` and `camera_noise_shake/model.rs` are reused verbatim as the bodies of
gameplay systems; (2) the ECS layer sits OVER the nodes — the player is ONE entity whose three
nodes (`Player`, its `InputSynchronizer` child, its `Camera3D` grandchild) are views, and no node
runs per-frame logic. V3-A proved the core on leaves with no physics, no animation state and no
replication. V3-B is where the architecture meets all three at once: `move_and_slide` and a
raycast mid-tick (the first members of the `EngineQuery` sets), root motion read from an
`AnimationTree`, engine-backed rotation math (`Quaternion::slerp`, `Basis::looking_at`),
`MultiplayerSynchronizer` replication of node properties as a projection of components, per-peer
authority, RPCs with `call_local`, and a `Node::input` callback. Every later milestone
(`bullet`, `part`, `red_robot`, `flying_forklift`) reuses what this one decides.

- **Backlog items closed by this spec**: none.
- **Backlog items explicitly deferred**: #6 (recapture `start_rotation` when other code moves
  the camera — the owner of the camera's rest rotation is a cross-cutting decision, the same
  reason V2-B deferred it) and #29 (occasional FPS drop noticed during V2-C play-testing —
  observe only; this milestone changes the per-frame shape of the player and the checkpoint
  MUST note whether the observation persists, without acting on it). Timing shifts are allowed
  only through the "Measured timing differences" table (constitution 1.5.1, Principle I v3).
- **Residual dynamic access kept after this milestone** (`.rpc("name")`, the one sanctioned
  form): `rpc("land")`/`rpc("jump")` (v2 `player.rs:247`, `:250`), `rpc("shoot")` (`:290`) on
  `Player`; `rpc("jump")` on the input node (`player_input.rs:112`); `rpc("hit")` in
  `hittable.rs:33` (unchanged file). No other dynamic access is introduced. `jump`/`land`/
  `shoot` on `Player` change `call_local` → `call_remote` (FR-004, option (b)).

**Input**: User description: "Milestone V3-B — the player: `player`, `player_input`,
`camera_noise_shake` as ONE entity over three nodes. The player tick on the fixed schedule in
v2's step order with two `EngineQuery` sets; input and camera on the frame schedule; camera
shake as components; the `AnimationTree` ordering question settled by experiment; replication as
a projection of components on the authority; RPC handlers as bridges. Parity with `v2` by a
six-case headless harness on both trees plus a single-player and a two-instance multiplayer
checkpoint."

## Context

Confirmed by reading the v2 code (`abfe35a`) and `player/player.tscn`:

| Module (v2) | Lines | Base | Per-frame logic today | Events / RPCs today |
|---|---|---|---|---|
| `player.rs` + `player/model.rs` | 336 + 348 (16 tests) | `CharacterBody3D` | `physics_process` (`player.rs:97-120`): server → `apply_input` (`:203-335`); else replays the replicated `current_animation` + `motion` into the `AnimationTree` (`:100-119`) | `#[rpc(authority, call_local, unreliable)]` `jump` (`:133-137`), `land` (`:139-143`), `shoot` (`:145-154`), `hit` (`:156-159`), `add_camera_shake_trauma(amount)` (`:161-165`, also called DIRECTLY by `red_robot.rs:452` through `bind_mut()`) |
| `player_input.rs` + `player_input/model.rs` | 192 + 332 (13 tests) | `MultiplayerSynchronizer` (child `InputSynchronizer`) | `process` (`:72-148`): `Input` snapshot, camera rotation, aim state, jump RPC, crosshair raycast, fade; `input` (`:150-156`): mouse motion → `rotate_camera` immediately | `#[rpc(authority, call_local, unreliable)] jump` sets `jumping = true` (`:161-164`) |
| `camera_noise_shake.rs` + `model.rs` | 68 + 116 (7 tests) | `Camera3D` (at `CameraBase/CameraRot/SpringArm3D/Camera3D`, `player.tscn:625`) | `process` (`:44-59`): decay/time/shake/offsets (pure) + three `get_noise_1d` (engine) + `set_rotation` | `add_trauma(amount)` (`:64-67`, `pub(crate)`, called only by `Player::add_camera_shake_trauma`) |

**`Player` state and handles** (`player.rs:31-83`): `airborne_time` (starts at 0 — backlog
#10's fix, comment at `:36-37` MUST be kept), `orientation`, `root_motion`, `#[var] motion:
Vector2`, `initial_position`, eleven `OnReady` handles (`InputSynchronizer`, `AnimationTree`,
`PlayerModel`, `ShootFrom`, `ShootParticle`, `MuzzleFlash`, `FireCooldown`, `SoundEffects/Jump`,
`/Land`, `/Shoot`) plus the preloaded `bullet_scene` (backlog #28's comment `:70-71` kept),
`#[export] #[var(set = set_player_id)] player_id: i32` (`:75-78`) and `#[export]
current_animation: Animations` (`:80-82`; `Animations` is `#[godot(via = i64)]`, `:12-19`).
`set_player_id` (`:125-131`) sets the `InputSynchronizer`'s multiplayer authority; `level.rs:177`
calls it through `bind_mut()` BEFORE `add_child` (`:179-182`), i.e. before `ready`.
`ready` (`:87-95`) captures `initial_position` and the `PlayerModel` orientation with a zero
origin, and disables `process` when not the server (the class has no `process`; the call is
moot). `apply_input`'s numbered steps: (1) ONE `player_input.bind_mut()` snapshot into
`InputFrame` and clear `jumping` (`:207-223`); (2) `lerp_motion` (`:226`); (3)
`flatten_camera_axes` (`:229`); (4) `airborne_step` with `is_on_floor()` → jump velocity,
`rpc("land")`, `rpc("jump")` (`:232-251`); (5) the branch (`:254-310`): airborne → `anim_plan` and
NO root-motion reassignment (`:258`); aiming → `slerp` of the orientation quaternion toward the
camera base quaternion (`:261-264`, ENGINE-backed), Strafe plan, root motion read from the
`AnimationTree` (`:269-272`), shoot when `shooting && fire_cooldown.get_time_left() == 0.0`
(`:274-291`: instantiate `bullet_scene`, `add_child_ex` under the PARENT, `set_global_position`
to `ShootFrom`'s origin, `look_at`, `add_collision_exception_with(self)`, `rpc("shoot")`); walking
→ `walk_target` then `Basis::looking_at(target).get_quaternion()` + `slerp` (`:294-301`,
ENGINE-backed), Walk plan, root motion read (`:306-309`); (6) `integrate_root_motion` with
`get_velocity()`/`get_gravity()` (`:313-317`, pure); (7) `set_velocity`, `set_up_direction(UP)`,
`move_and_slide()` (`:320-322`); (8) `PlayerModel.set_global_basis` (`:325-326`); (9)
`should_respawn(y)` → transform origin reset to `initial_position` and zero velocity (`:329-334`,
backlog #11's comment kept). `apply_anim` (`:171-201`) writes `current_animation` and then the
`AnimationTree` parameters in v1's per-variant order (`TRANSITION_REQUEST`, `AIM_ADD_AMOUNT`,
`STRAFE_BLEND`, `WALK_BLEND`, `:21-29`).

**`PlayerInputSynchronizer`** (`player_input.rs:16-56`): `aim_state`, the parent
`CharacterBody3D` and its RID resolved once through `OnReady::from_base_fn` (`:26-29`), the four
replicated `#[export]`s `aiming`, `shoot_target`, `motion`, `shooting` (`:32-39`), the
non-replicated `jumping` (`:41`, backlog #9), six `#[export] OnEditor` node refs saved as
`node_paths` in `player.tscn:339-346` (`camera_animation` → `../CameraBase/Animation`,
`crosshair`, `camera_base`, `camera_rot`, `camera_camera` → `../CameraBase/CameraRot/SpringArm3D/
Camera3D`, `color_rect`). `ready` (`:60-70`): if this peer is the node's multiplayer authority →
`camera_camera.make_current()` + mouse captured; else `set_process(false)`,
`set_process_input(false)`, `color_rect.hide()`. `process` (`:72-148`) in order: the ten `Input`
reads into `InputSnapshot` (`:76-90`; `is_action_just_pressed`/`just_released` are per-FRAME
semantics), `self.motion = snapshot.motion` (`:92`), `scaled_look` + `rotate_camera` (`:94-95`:
`rotate_y`, `orthonormalize`, `clamp_pitch`, `set_rotation`, `:184-191`), `step_aim` → `aiming`
+ camera animation cues `shoot`/`far` (`:97-109`), `rpc("jump")` on `jump_just_pressed`
(`:111-113`), `shooting` + the crosshair raycast (`:115-139`: `project_ray_origin/normal` on the
camera AFTER this frame's rotation, `PhysicsRayQueryParameters3D` with mask `0b11` and the parent
RID excluded, `intersect_ray`, `shoot_target` = hit position or `ray_from + ray_dir * 1000`),
fade (`:144-147`: `alpha_for_height` on the parent's y and the current `modulate.a`). `input`
(`:150-156`): `InputEventMouseMotion` → `scaled_mouse_look(screen_relative, aiming)` →
`rotate_camera` immediately. Three typed getters read the camera nodes (`:171-182`).

**`CameraNoiseShake`** (`camera_noise_shake.rs:9-25`): three `FastNoiseLite` created at init,
`noise_seed = randi() as i32` at init (`:23-24` — NOT `#[export]`, not in `player.tscn`; the
`randi()` CALL ORDER at instantiation matters for seeded parity), `ready` (`:29-42`) seeds the
three noises (`seed`, `seed+1`, `seed+2`, `wrapping_add`), sets octaves 1 / lacunarity 1.0, and
captures `start_rotation`. `process` (`:44-59`): only while `trauma > 0`: `decay`,
`advance_time`, `shake`, three `get_noise_1d(time as f32)`, `offsets`, `set_rotation(start +
offset)`. `add_trauma` (`:64-67`) is pure `model::add_trauma`.

**Replication surface** (`player.tscn`): `ServerSynchronizer` (`:336-337`) with
`SceneReplicationConfig_o4rt5` (`:15-30`): `.:transform`, `.:player_id`, `PlayerModel:transform`,
`.:motion`, `.:current_animation`; `InputSynchronizer` (`:339-340`) with
`SceneReplicationConfig_8yuxf` (`:32-50`): `CameraBase:rotation`, `CameraBase/CameraRot:rotation`,
`InputSynchronizer:shoot_target`, `:motion`, `:shooting`, `:aiming`. Every name and type is a
preserved surface (constitution Principle I v3 "Preserved surfaces"); the Rust fields behind
them become projections of components.

**`AnimationTree`** (`player.tscn:589-595`): `callback_mode_process = 0` (PHYSICS), a CHILD of
`Player`, `root_motion_track` set, driving `PlayerModel/AnimationPlayer` (`:576-577`, itself
`callback_mode_process = 0` but inactive while the tree is active). In v2 the parent's
`physics_process` runs BEFORE the child's internal physics processing in the same pass (tree
order), so each step v2 (a) reads the root motion the tree accumulated during the PREVIOUS
step's processing and (b) writes transition/blend parameters that the tree processes right
after, in THIS step. With the v3 driver at `i32::MAX`, the fixed schedule runs AFTER the tree's
processing of the same step: reads would see this step's root motion and parameter writes would
be processed next step — a two-way shift. This is the central research question (US4, FR-024).
`FireCooldown` (`:672-675`): `wait_time = 0.4`, `one_shot`, `autostart`. `SoundEffects/Jump`,
`/Land`, `/Shoot` (`:663-670`). Node `ready` order is children first: `CameraNoiseShake.ready`
and `PlayerInputSynchronizer.ready` run BEFORE `Player.ready`.

**External consumers, all unchanged**: `level.rs:175-182` (`instantiate_as::<Player>`,
`set_name`, `bind_mut().set_player_id(id)`, `set_transform`, `add_child_ex`); `red_robot.rs:345`
and `:353` (`try_cast::<Player>`), `:452` (`player.clone().bind_mut().add_camera_shake_trauma(
trauma_amount)` inside an async block — `red_robot` stays v2 code, so the method MUST keep
existing on the bridge with its `pub(crate) fn (&mut self, f64)` signature); `hittable.rs:11,
:17, :33` (`Gd<Player>`, `rpc("hit")`); `door.rs:50` (`try_cast::<Player>`); `bullet.rs:118-120`
reaches the player only through `hittable`. None of these files changes.

**V3-A infrastructure reused as is** (`src/ecs.rs`, `src/ecs/*`): `EcsWorld` driver at
`i32::MAX`, `queue::push`, `InboundEvent`/`Initial`, `Handles`/`NodeHandles`, `EntityIndex`,
`Timer`, `Phase` sets, `apply_register`, `sweep_dead_nodes`, `sync_out_remove`. The four rows of
`docs/v3-tradeoffs.md` and `CLAUDE.md`'s "Port conventions (v3)" describe them.

Baseline gates: `cargo build` / `cargo clippy` / `cargo test` clean, **156 tests** (36 of them the
three modules' pure tests, all preserved). Parity baseline: branch `v2` at `e2932b4`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The player tick on the fixed schedule (Priority: P1)

Walking, jumping, landing, aiming, shooting and falling off the map behave exactly as in v2, but
the player is one entity: its `InputSynchronizer` and camera nodes are read into components at
`SyncIn`, the nine steps of v2's `apply_input` run as pure gameplay systems interleaved with two
`EngineQuery` sets (rotation math + animation parameters + root motion + bullet spawn; then
velocity + `move_and_slide`), and `SyncOut` writes the model basis, the respawn reset, the
replicated projection and the RPCs. A player this peer does NOT simulate replays the replicated
animation exactly as v2 did.

**Why this priority**: it is the tick every later gameplay module copies, and the first exercise
of `EngineQuery`, root motion and replication.

**Independent Test**: `cargo test` passes for every new system with the 16 `player/model.rs`
tests preserved; harness cases (a) walk, (b) jump+land, (c) aim+shoot and (f) fall/respawn are
identical on `v2` and `v3` (or every differing frame is in the timing table); visual checkpoint
(1) single player; the tick's replay path is exercised by checkpoint (2) multiplayer.

**Acceptance Scenarios**:

1. **Given** `level.rs:175-182` (instantiate, `set_player_id`, `set_transform`, `add_child`),
   **When** the player enters the tree, **Then** `set_player_id` still performs its one-shot
   engine write (`InputSynchronizer.set_multiplayer_authority`, v2 `player.rs:126-131`) as a
   bridge setter that runs BEFORE `ready` and never touches the ECS; `Player.ready` registers
   ONE entity with ALL handles (its eleven children; the six camera-side nodes reached through
   `player_input`'s exported refs; the `CameraNoiseShake` node and its three `FastNoiseLite`) and
   the initial components: `PeerId(player_id)`, `Simulates` iff `multiplayer.is_server()` (v2
   `:98`), `OwnsInput` iff the input node's `get_multiplayer_authority() == get_unique_id()`
   (v2 `player_input.rs:61-62`), `initial_position` (`:88`), the orientation with zero origin
   (`:90-91`), `airborne_time = 0`, `motion = 0`, `current_animation = Walk`, `AimState::Idle`,
   `Trauma(0)`, `ShakeTime(0)`, `start_rotation` (captured by the camera sub-bridge in its own
   `ready`, which ran first). `exit_tree` unregisters. Rationale for the root registering
   everything: `ready` runs children first, so a sub-bridge cannot register into a root entity
   that does not exist yet.
2. **Given** a `Simulates` entity, **When** the fixed schedule runs, **Then** `SyncIn` reads ONCE
   per value: the input node's `aiming`, `shoot_target`, `motion`, `shooting`, `jumping` (then
   clears `jumping`, v2 `:221`), the three camera reads (`camera_rot` global basis, `camera_base`
   global quaternion, `aim_rotation` from `camera_rot.rotation.x`, v2 `player_input.rs:171-182`),
   `is_on_floor()`, `get_velocity()`, `get_gravity()`, `fire_cooldown.get_time_left()`, the body
   origin y; the pure steps (2), (3), (4) run in `Gameplay` with `lerp_motion`,
   `flatten_camera_axes`, `airborne_step` (`player/model.rs:64-110`, unchanged) producing the
   jump velocity, `land`/`jump` intents and the branch decision.
3. **Given** the branch decision, **When** the first `EngineQuery` set runs, **Then** for the
   aiming branch the orientation quaternion is slerped toward the camera base quaternion
   (`Quaternion::slerp`, engine-backed, v2 `:261-264`); for the walking branch `walk_target`
   (pure, `model.rs:130-133`) is turned into a quaternion by `Basis::looking_at` and slerped
   (v2 `:296-300`); for the airborne branch nothing; THEN the `AnimationTree` parameters are
   written from the `AnimPlan` in v1's per-variant order (v2 `apply_anim`, `:179-200`); THEN
   the root motion is read (`get_root_motion_rotation/position`, v2 `:269-272`/`:306-309`,
   never while airborne, `:258`); THEN, when the shoot decision is `Fire` (`shooting &&
   cooldown_left == 0.0`, `:274`), the bullet is instantiated from the preloaded scene, added
   under the player's parent with `force_readable_name`, positioned at `ShootFrom`'s origin,
   `look_at`-ed toward `shoot_target`, given the collision exception with the player, and the
   `Shoot` RPC intent is set (`:275-291`) — BEFORE `move_and_slide`, because the bullet origin
   is `ShootFrom`'s transform before the move.
4. **Given** the root motion, **When** the second `Gameplay` set runs `integrate_root_motion`
   (`model.rs:137-157`, unchanged) and the second `EngineQuery` set runs `set_velocity`,
   `set_up_direction(UP)`, `move_and_slide()` and reads the post-move origin (v2 `:320-322`,
   `:329`), **Then** the third `Gameplay` set decides `should_respawn` (`model.rs:160-162`) and
   `SyncOut` writes `PlayerModel.set_global_basis` (`:326`), applies the respawn reset (origin =
   `initial_position`, velocity = zero, `:330-333`), writes `motion` and `current_animation` into
   the `Player` node's fields (the replicated projection), and issues `rpc("land")`,
   `rpc("jump")` (in that order, both may fire, `:245-251`) and `rpc("shoot")` (`:290`).
5. **Given** a player entity WITHOUT `Simulates` (a remote player on a client, or every player on
   a client), **When** the fixed schedule runs, **Then** `SyncIn` reads `current_animation` and
   `motion` from the `Player` node (filled by replication) and `aim_rotation` from the camera
   node (whose rotation is replicated), the replay plan is built exactly as v2 `:104-117`
   (`JumpUp`/`JumpDown` as is; `Strafe { aim_rotation, blend (motion.x, -motion.y) }`; `Walk {
   blend (motion.length(), 0) }`), and the `AnimationTree` parameters are written in the same
   per-variant order; no movement, no RPC, no bullet.
6. **Given** the harness's case (a) — `move_forward` held from frame 10 to 100 — **When** both
   trees log per physics step the body origin, velocity, `motion`, `current_animation` and the
   `PlayerModel` global basis, **Then** the logs are identical.

---

### User Story 2 - Input and camera on the frame schedule (Priority: P1)

Controller and mouse look, the aim hold/toggle state, the crosshair raycast, the fall-to-black
fade and the jump press behave exactly as in v2, but the `InputSynchronizer` node has no
`process`/`input`: the `Input` singleton is snapshotted at `SyncIn`, the pure aim/look/fade
functions run in `Gameplay`, the camera rotation and the raycast run in the frame's
`EngineQuery` set (the raycast needs the camera rotated THIS frame, v2 `player_input.rs:94-95`
before `:117-119`), and `SyncOut` writes the cues, the replicated input properties, the fade and
the jump RPC.

**Why this priority**: it feeds US1 and is the only per-FRAME gameplay of the milestone
(`is_action_just_pressed` semantics are per rendered frame, v2 `process`).

**Independent Test**: `cargo test` passes for the input systems with the 13 `player_input/
model.rs` tests preserved; harness cases (c) aim+shoot and (e) mouse look identical on both
trees; checkpoint (1).

**Acceptance Scenarios**:

1. **Given** `PlayerInputSynchronizer.ready` (v2 `:60-70`), **When** it runs, **Then** it keeps
   ONLY the one-shot engine setup: authority → `camera_camera.make_current()` + mouse captured;
   non-authority → `color_rect.hide()` AND `set_process_input(false)` (v2 `:67`) — this call
   is NOT moot: it is the gate of FR-013, the engine-level way to guarantee that the `input`
   callback (and therefore the `MouseLook` event) exists only on the node whose entity owns the
   input; `set_process(false)` (`:66`) does become moot because the class has no `process`. It
   resolves the ROOT player's `InstanceId` once
   (`OnReady::from_base_fn` over `get_parent()`), pushes nothing (the root registers), and keeps
   its `#[export] OnEditor` refs with their names (`node_paths` in `player.tscn:339` unedited).
2. **Given** an `OwnsInput` entity, **When** the frame schedule runs, **Then** `SyncIn` builds
   the `InputSnapshot` from the ten `Input` reads in v2's order (`:76-90`), reads the current
   camera rotations, the parent's global y and `color_rect.modulate.a`; `Gameplay` computes
   `scaled_look` (`model.rs:125-132`), the mouse look from any `MouseLook` event drained this
   frame (`scaled_mouse_look`, `:134-141`), the pitch clamps (`clamp_pitch`, `:143-147`),
   `step_aim` (`:77-123`) with its `CameraCue`, `alpha_for_height` (`:149-156`), and the
   decisions "jump pressed", "shooting"; the frame `EngineQuery` set applies the camera rotation
   (`rotate_y`, `orthonormalize`, `set_rotation` — the same two `rotate_camera` applications in
   v2's order: mouse first, from `input`, then controller, from `process`) and THEN, when
   shooting, the crosshair raycast (`project_ray_origin/normal` at the crosshair center, mask
   `0b11`, the parent RID excluded, `intersect_ray`; `shoot_target` = hit position or `from +
   dir * 1000`, `:117-138`); `SyncOut` plays the `shoot`/`far` camera animation on a cue
   (`:100-109`), writes `aiming`, `shoot_target`, `motion`, `shooting` into the input node
   (the replicated projection, `:92`, `:99`, `:115`, `:135-137`), writes `color_rect`'s modulate
   alpha (`:145-147`) and issues `rpc("jump")` on the input node when the jump was just pressed
   (`:111-113`).
3. **Given** the input node's `#[rpc] jump` (v2 `:161-164`), **When** it is invoked (locally by
   `call_local` in the same frame's `SyncOut`, or from the network), **Then** it only pushes
   `JumpPressed { root_id }`; the next fixed run's drain sets the entity's `jumping` input,
   which the tick consumes and clears — the same one-shot semantics as v2's `jumping` field.
4. **Given** `input(&mut self, event)` (v2 `:150-156`), **When** an `InputEventMouseMotion`
   arrives, **Then** the sub-bridge only pushes `MouseLook { root_id, screen_relative }`; the
   frame schedule of the same iteration drains and applies it (input events are dispatched
   before `_process` — Assumptions), so `CameraBase.rotation.y`/`CameraRot.rotation.x` match v2
   frame by frame (harness case (e)).
5. **Given** a non-`OwnsInput` entity, **When** the frame schedule runs, **Then** no input
   snapshot, no camera write, no raycast, no RPC and no fade write happen for it; `SyncIn` reads
   the four replicated input properties from the node into components uniformly on every peer
   (the server reads a remote player's input from the same node fields the network filled) —
   this is what the fixed tick of US1 consumes.

---

### User Story 3 - Camera shake as components on the player entity (Priority: P2)

Getting hit, shooting and the robot's blast shake the camera exactly as in v2, but the trauma and
time live on the player entity, the pure decay/shake math runs in `Gameplay`, the three noise
samples are an `EngineQuery`, and `SyncOut` writes the camera rotation.

**Why this priority**: small, and the pattern for every "effect state on a parent entity driven
by a grandchild node" case.

**Independent Test**: `cargo test` passes with the 7 `camera_noise_shake/model.rs` tests
preserved plus the shake system's; harness case (d) identical on both trees; checkpoint (1).

**Acceptance Scenarios**:

1. **Given** `CameraNoiseShake` at init and `ready` (v2 `:9-42`), **When** the scene is
   instantiated, **Then** `noise_seed = randi()` is still drawn at init in the SAME call order
   as v2 (seeded parity, harness `seed(1)` first thing), the three noises are seeded and
   configured in `ready` as before, `start_rotation` is captured in `ready`, and the node keeps
   no `process`; the three noise handles and `start_rotation` reach the entity through the
   root's registration (the camera sub-bridge exposes them typed, read once by `Player.ready`).
2. **Given** `Trauma > 0`, **When** the frame schedule runs, **Then** `Gameplay` runs `decay`,
   `advance_time`, `shake` (`model.rs:29-48`, unchanged, in v2's order `:47-49`), the frame
   `EngineQuery` reads the three `get_noise_1d(time as f32)` samples (`:50-54`), `Gameplay`
   computes `offsets` (`:55`), and `SyncOut` writes `camera.set_rotation(start_rotation +
   offset)` (`:56-57`); with `Trauma == 0` nothing runs and nothing is written (v2 `:45`).
3. **Given** `Player::add_camera_shake_trauma(amount)` (kept as `#[rpc(authority, call_local,
   unreliable)] pub(crate) fn (&mut self, f64)` — `red_robot.rs:452` calls it typed, `hittable`
   sends `hit`), **When** it is invoked, **Then** it only pushes `AddTrauma { root_id, amount }`;
   `hit` pushes the same with 0.75 (v2 `:158`) and `shoot`'s local effects include 0.35 (`:153`);
   the drain applies `model::add_trauma` (`:58-63`, clamped at `max_trauma`) to the entity.
   `CameraNoiseShake::add_trauma` (v2 `:64-67`) is deleted: its only caller was the player.
4. **Given** harness case (d) — `player.hit()` at frame 30 — **When** both trees log the camera
   rotation per frame, **Then** the logs are identical (seeded noise, same `start_rotation`).

---

### User Story 4 - The `AnimationTree` ordering decision, implemented and proven (Priority: P2)

The animation the player shows and the root motion that moves it are identical to v2 step by
step, even though the ECS tick now runs after the `AnimationTree`'s own physics processing.

**Why this priority**: it is the one place where the v3 driver's "last in the phase" rule
collides with a child node that both consumes the tick's output and produces the tick's input;
positions diverge every frame if it is wrong.

**Independent Test**: a per-step log of `get_root_motion_position()`, the transition request
and the body origin on `v2` and on `v3` (research experiment, then harness case (a)) is
identical.

**Acceptance Scenarios**:

1. **Given** the three options — (A) keep `callback_mode_process = PHYSICS` and accept the
   one-step shift; (B) set the player's `AnimationTree` to `MANUAL` (`callback_mode_process = 2`
   in `player.tscn:592`) and call `animation_tree.advance(delta)` from the fixed `SyncOut`, after
   the tick, for EVERY player entity on every peer (non-`Simulates` entities animate too); (C)
   anything the research finds — **When** research runs the experiment on both trees, **Then**
   the option that makes the per-step root-motion/origin log identical to v2 is the one
   implemented; (A) is rejected if positions diverge (expected); (B) is the expected answer and
   is the ONE `.tscn` edit of this milestone, allowed by Principle II (an engine property, not
   an exported script property nor a replicated name), listed here with its reason.
2. **Given** option (B), **When** a player is NOT simulated on this peer, **Then** its
   `AnimationTree` is still advanced every fixed run after the replay parameters are written
   (v2: the tree processed after `physics_process` on every peer).
3. **Given** the decision, **When** it lands, **Then** `docs/v3-tradeoffs.md` records it (what
   the engine owns: animation advance and root motion; why the ECS cannot hide it; how the sync
   layer restores v2's order) and `CLAUDE.md`'s v3 section names the rule for later modules
   with an `AnimationTree` (`red_robot`).

---

### Edge Cases

- **`set_player_id` before `ready`** (v2 `level.rs:177`): the setter writes the input node's
  authority through a typed handle and never touches the ECS; registration reads `player_id`
  afterwards, so the entity's `PeerId` and `OwnsInput` are correct even though the value arrived
  before the node was in the tree.
- **`land` and `jump` in the same step** (v2 `player/model.rs:50-52`): both intents fire; the
  RPCs are issued in v2's order (`land` then `jump`, `:246-251`) from `SyncOut`.
- **Local RPC effects on the simulating peer** (option (b), FR-004): `rpc("land")`/`rpc("jump")`/
  `rpc("shoot")` are `call_remote`, so on the `Simulates` entity NO handler runs locally;
  `sync_out_player` applies the local effects itself, inside the physics step, in v2's order:
  `Land` sound, `Jump` sound (step (4)'s position, `:246-251`; the `JumpUp`/`JumpDown` plan
  writes are omitted — v2's step (5) overwrote them in the same step), then after the bullet
  spawn the `Shoot` effects (`:146-154`: both particles restart+emit, `fire_cooldown.start()`,
  `Shoot` sound, trauma 0.35 via `model::add_trauma` on the `Trauma` component). The RPC then
  reaches the remote peers only, whose handlers push `PlayerFx` (non-`Simulates` path, applied
  by their frame run: sounds/particles/cooldown, plus the plan writes that v2's client handlers
  also made). `bind_mut()` on the `Player` bridge (projection writes) is ALWAYS released before
  any engine call that can invoke a callback (`rpc`, `play`, `start`): a `call_local` RPC or a
  synchronous signal would otherwise double-borrow. The harness's RAW stamps of
  `SoundEffects/Jump.playing`, `Land.playing`, `Shoot.playing` and `FireCooldown.time_left`
  verify the same-step timing (cases (b), (c)).
- **`hit` and `add_camera_shake_trauma` (still `call_local`)**: their local handler pushes
  `AddTrauma`, applied by the drain of the NEXT schedule run — the frame run of the same
  iteration when the caller is a physics-phase node (`bullet.rs`'s `rpc("hit")`, `red_robot`'s
  async block resumed in the physics phase), so `shake_decide` of that frame already sees the
  trauma, as v2's `camera.process` did.
- **The `Jump`/`Land` Fx animation write is transient** (spec review, 2026-09-18): in v2 the
  `land` handler's `apply_anim(JumpDown)` (`:141`) runs INLINE in the tick and the tick's own
  plan overwrites it in the SAME step (`:254-310`); on a remote peer the next replay
  (`:100-119`) overwrites it. In v3 the Fx is applied by the FRAME run, AFTER the fixed
  `SyncOut` wrote the projection `current_animation = Walk` into the node, so the node field
  reads `JumpDown` between frame run N and fixed run N+1 (which rewrites `Walk` before anything
  observes it). Invisible to the observer (it logs after that iteration's physics) and to the
  `AnimationTree` (the request is overwritten before the next `advance`/processing); invisible
  to REPLICATION only if Assumption (5) holds — `SceneMultiplayer::poll()` samples node values
  at the top of `SceneTree::process`, BEFORE the frame run. Assumption (5) is therefore
  load-bearing for FR-015: research MUST verify it from the engine source; if the poll instead
  samples after the frame run, the Fx animation write is DROPPED on `Simulates` entities as
  redundant (the tick's plan already set the same or a later state), with the reason recorded
  in `docs/v3-tradeoffs.md`; the sound/particle/cooldown/trauma effects are never dropped.
  Harness case (b) logs `current_animation` per frame on both trees either way.
- **Shoot with a dead cooldown timer**: `fire_cooldown.get_time_left() == 0.0` is read at
  `SyncIn`; the `shoot` handler's `fire_cooldown.start()` (v2 `:151`) is applied by the frame
  schedule's `SyncOut` after the local RPC; the next fixed run reads the new `time_left` — same
  ordering as v2 (the RPC handler ran inside the tick, before the next step's read).
- **Bullet spawn frame**: the bullet is instantiated in the first `EngineQuery` set BEFORE
  `move_and_slide`, so its origin is `ShootFrom`'s pre-move transform exactly as v2 (`:275`
  precedes `:322`); harness case (c) logs the first bullet's global transform on its spawn frame.
- **Respawn**: the reset happens in `SyncOut` after `should_respawn` read the post-move origin
  (`:329`); the next `SyncIn` reads the reset origin. Harness case (f) removes the floor at
  frame 50 and logs origin, `ColorRect.modulate.a` and the respawn frame.
- **A remote player's input on the server**: the server simulates a client's player from the
  input properties the network wrote into that player's `InputSynchronizer`; `SyncIn` reads
  those fields for every entity regardless of `OwnsInput`, so the server's tick sees them.
- **Mouse look while not aiming vs aiming**: `scaled_mouse_look` uses the aim state of the frame
  in which the event is applied (v2 read `self.aiming` at `input` time, `:153`); the harness
  case (e) sends the event at frame 25 with aim idle, so both trees use the same scale.
- **The camera moved by something other than the shake** (backlog #6): `start_rotation` stays
  the `ready`-time capture, as in v2; deferred again.

## Requirements *(mandatory)*

### Functional Requirements

**Entity, bridges, authority (US1, US2, US3)**

- **FR-001**: The player MUST be ONE entity registered by `Player.ready` with all handles
  (eleven children, the six camera-side nodes reached through the input node's exported refs,
  the `CameraNoiseShake` node and its three `FastNoiseLite`) and the initial components of
  scenario 1 (US1); `Player.exit_tree` unregisters. `PlayerInputSynchronizer` and
  `CameraNoiseShake` are SUB-BRIDGES: no `process`/`physics_process`/`input` logic of their own
  beyond pushing events, no registration, `ready` limited to v2's one-shot engine setup
  (`player_input.rs:60-70`, `camera_noise_shake.rs:29-42`), and the root's `InstanceId`
  resolved ONCE through `OnReady::from_base_fn` (`get_parent()` chain or `get_owner()` — plan
  decides; never per event).
- **FR-002**: Two authority markers MUST exist: `Simulates` (`multiplayer.is_server()`, v2
  `player.rs:98`) and `OwnsInput` (the input node's `get_multiplayer_authority() ==
  get_unique_id()`, v2 `player_input.rs:61-62`), set at registration; every gameplay and sync
  system of this milestone filters by them. A single-peer headless run has both; a joined
  client has `OwnsInput` on its own player only and neither on others.
- **FR-003**: `set_player_id` MUST stay a bridge setter (`#[func]`, `#[var(set = …)]`) doing its
  one-shot engine write (v2 `:126-131`) and MUST NOT touch the ECS; registration reads
  `player_id` into `PeerId`.
- **FR-004**: The RPC handlers `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma` on
  `Player` and `jump` on `PlayerInputSynchronizer` MUST keep their exact names and signatures
  (`add_camera_shake_trauma` stays `pub(crate) fn (&mut self, f64)` for `red_robot.rs:452`)
  and MUST only push events: `PlayerFx::{Jump, Land, Shoot}`, `AddTrauma { amount }`,
  `JumpPressed`, each keyed by the root's `InstanceId` (`hit` pushes `AddTrauma { 0.75 }` —
  it never did anything else, v2 `:157-159`). ONE attribute change, approved at plan review
  (2026-09-18, option (b)): `jump`, `land` and `shoot` on `Player` become
  `#[rpc(authority, call_remote, unreliable)]`. Reason: v2 applied their local effects INLINE in
  the physics step through `call_local`; in v3 a locally-invoked handler could only push an
  event consumed by the frame run, which (i) starts `FireCooldown` one frame later than v2
  (the `Timer` node processes in tree order before the driver), (ii) adds the shoot/hit trauma
  after `shake_decide` already ran, and (iii) leaves the transient animation write that R2
  showed replication can sample. With `call_remote`, the `Simulates` entity's fixed `SyncOut`
  applies the local effects itself, in the physics step, in v2's order, and the RPC reaches
  only the remote peers, whose handlers push `PlayerFx` for the non-`Simulates` path. `hit` and
  `add_camera_shake_trauma` keep `call_local` (`hittable.rs` calls `rpc("hit")` and the trauma
  is applied at drain time, so the local path is already same-frame); the input node's `jump`
  keeps `call_local` (its local push is consumed by the next fixed run, as v2's field was).

**The fixed tick (US1)**

- **FR-005**: The fixed schedule MUST run v2's `apply_input` in its step order (scenarios 2-4,
  US1) with these labeled sets, in this order: `SyncIn` → `Gameplay` → `EngineQueryOrient` →
  `GameplayIntegrate` → `EngineQueryMove` → `GameplaySettle` → `SyncOut`. The spec pins the
  labels; the plan pins the API (enum variants or nested sets). V3-A's door system stays in the
  first `Gameplay`; V3-A's `sweep_dead_nodes` stays in `SyncIn` and `sync_out_remove` in
  `SyncOut`. Every engine-touching system is in an `EngineQuery*` set or a sync set; every
  pure step is in a `Gameplay*` set.
- **FR-006**: `SyncIn` MUST read each value ONCE per step (scenario 2, US1), including the
  clear of `jumping` (v2 `:221`) and, for non-`Simulates` entities, `current_animation` and
  `motion` from the `Player` node and `aim_rotation` from the camera (`:104-117`).
- **FR-007**: The pure steps MUST call the existing `player/model.rs` functions unchanged:
  `lerp_motion`, `flatten_camera_axes`, `airborne_step`, `walk_target`, `anim_plan`,
  `integrate_root_motion`, `should_respawn`, with `PlayerTuning::default()` (`model.rs:14-34`);
  the 16 tests stay; each gameplay system gains at least one `run_system_once` test.
- **FR-008**: `EngineQueryOrient` MUST, in this order per entity: slerp the orientation
  (aiming: toward the camera base quaternion, v2 `:261-264`; walking: toward
  `Basis::looking_at(walk_target).get_quaternion()`, `:296-300`; airborne: none), write the
  `AnimationTree` parameters in v1's per-variant order (`:179-200`), read the root motion
  (`:269-272`/`:306-309`; not while airborne, `:258`), and spawn the bullet when the shoot
  decision fired (`:275-291`, including `add_collision_exception_with` and the `Shoot` intent).
- **FR-009**: `EngineQueryMove` MUST perform `set_velocity`, `set_up_direction(Vector3::UP)`,
  `move_and_slide()` and read the post-move origin (`:320-322`, `:329`); `GameplaySettle`
  decides `should_respawn`.
- **FR-010**: `SyncOut` MUST write `PlayerModel.set_global_basis` (`:326`), the respawn reset
  (`:330-333`), the replicated projection `motion` + `current_animation` into the `Player`
  node's fields on `Simulates` entities, and issue `rpc("land")`, `rpc("jump")`, `rpc("shoot")`
  in v2's order; `player_id` is never rewritten after `set_player_id`.
- **FR-011**: The non-`Simulates` replay path MUST reproduce v2 `:100-119` (scenario 5, US1):
  parameters written from the replicated `current_animation` + `motion` in the same per-variant
  order; no movement, RPC or bullet.

**The frame schedule (US2, US3)**

- **FR-012**: The frame schedule MUST keep V3-A's four sets `SyncIn → Gameplay → EngineQuery →
  SyncOut` and place the input and shake work as scenario 2 (US2) and scenario 2 (US3) state:
  `Input` snapshot (ten reads in v2's order, per-frame semantics) at `SyncIn`; `scaled_look`,
  mouse look, `clamp_pitch`, `step_aim`, `alpha_for_height`, decay/time/shake/offsets in
  `Gameplay` (existing functions unchanged, 13 + 7 tests preserved, each system with a
  `run_system_once` test); camera rotation writes THEN the crosshair raycast, and the three
  `get_noise_1d`, in `EngineQuery`; cues, the four replicated input properties, `color_rect`
  modulate, `rpc("jump")` and the shake `set_rotation` in `SyncOut`.
- **FR-013**: Only `OwnsInput` entities MUST snapshot `Input`, rotate the camera, raycast, fade
  and issue `rpc("jump")`; the sub-bridge's `input` callback MUST exist only on the node whose
  entity owns the input, gated exactly as v2 did — `set_process_input(false)` on non-authority
  nodes in `ready` (`:67`), an engine one-shot, not a per-event check. Every entity's `SyncIn`
  MUST read the four replicated input properties from the node into components.
- **FR-014**: The camera rotation MUST be applied in v2's order — the mouse event's
  `rotate_camera` first (v2 `input`), then the controller's (v2 `process` `:95`) — each as
  `rotate_y`, `orthonormalize`, `clamp_pitch`, `set_rotation` (`:184-191`), and BEFORE the
  raycast of the same frame (`:117-119` use the rotated camera). Recorded in
  `docs/v3-tradeoffs.md` (the raycast needs the engine-computed post-rotation camera transform
  through the `SpringArm3D` chain).
- **FR-015**: `JumpPressed`, `MouseLook`, `AddTrauma`, `PlayerFx::*` MUST be applied by the
  drain to the entity found by root id (unknown id dropped). The drain applies trauma
  IMMEDIATELY (`AddTrauma`, and the 0.35 of a remote `Shoot`), so the same run's `shake_decide`
  sees it; the engine effects of `PlayerFx` are queued and applied by the frame schedule's
  `SyncOut` in v2's per-handler order: `Jump` → `JumpUp` plan + `Jump` sound (`:134-137`);
  `Land` → `JumpDown` plan + `Land` sound (`:140-143`); `Shoot` → restart+emit both particles,
  `fire_cooldown.start()`, `Shoot` sound (`:146-153`). On the `Simulates` entity these events
  never occur (FR-004: `call_remote`); its local effects are applied inline by the fixed
  `SyncOut` (Edge Cases). On non-`Simulates` entities the plan writes are kept, as v2's client
  handlers made them (R2).
- **FR-016**: `Trauma`/`ShakeTime` live on the player entity; the shake runs only while
  `Trauma > 0` (v2 `:45`); `add_trauma` clamps at `max_trauma` (`model.rs:58-63`);
  `CameraNoiseShake::add_trauma` is removed (single caller, now an event).
- **FR-017**: `noise_seed` MUST still be drawn with `randi()` at the camera node's init in v2's
  call order (`camera_noise_shake.rs:23-24`), the three noises seeded in `ready` as `:30-36`;
  `start_rotation` captured in `ready` (`:41`) and read into the entity at registration.

**The `AnimationTree` decision (US4)**

- **FR-018**: Research MUST settle the ordering by experiment on both trees (per-step log of
  `get_root_motion_position()`, the transition request, the body origin) and the spec's parity
  criterion decides: the option whose log equals v2's is implemented. If it is (B), the ONE
  scene edit of this milestone is `callback_mode_process = 2` (MANUAL) on `player.tscn:592`'s
  `AnimationTree`, and the fixed `SyncOut` calls `animation_tree.advance(delta)` for EVERY
  player entity after the tick's writes, on every peer; the edit is listed in the plan and in
  `docs/v3-tradeoffs.md`. If (A) is kept, every measured divergence goes to the timing table
  and the user decides at checkpoint (1) whether it is acceptable — the spec's default
  expectation is that it is NOT.

**Preserved surfaces and scope (Principle I v3, Principle II)**

- **FR-019**: These names and types MUST NOT change: the replicated properties `.:transform`,
  `.:player_id`, `PlayerModel:transform`, `.:motion`, `.:current_animation`,
  `CameraBase:rotation`, `CameraBase/CameraRot:rotation`, `InputSynchronizer:shoot_target`,
  `:motion`, `:shooting`, `:aiming` (`player.tscn:15-50`); `set_player_id`; the five `#[rpc]`
  names and signatures; `add_camera_shake_trauma(f64)`; the six `#[export] OnEditor` refs of
  `PlayerInputSynchronizer` (`node_paths`, `player.tscn:339-346`); the `Animations` enum and
  its `via = i64` wire values; the class names `Player`, `PlayerInputSynchronizer`,
  `CameraNoiseShake`. `noise_seed` is not exported today and stays a plain field.
- **FR-020**: `hittable.rs`, `level.rs`, `red_robot.rs`, `door.rs`, `bullet.rs`, `part.rs`,
  `flying_forklift.rs`, `settings`, `menu`, `main_scene`, `debug_label` MUST be untouched;
  `player.tscn` changes only by FR-018's one property (if B).
- **FR-021**: Backlog: none closed; #6 and #29 deferred as the top block states; no v3 backlog
  file.

**Cross-cutting (constitution 1.5.1 compliance)**

- **FR-022**: No bridge (`Player`, `PlayerInputSynchronizer`, `CameraNoiseShake`) MUST have
  `process`/`physics_process`; no engine callback MUST borrow the World; engine access MUST
  appear only in `SyncIn`/`EngineQuery*`/`SyncOut` systems and bridges; components MUST hold no
  `Gd<T>` (all handles in `NodeHandles`); nodes freed only by the sync layer with `queue_free()`
  (no player node is freed by this milestone); `.rpc("name")` the only dynamic access (the five
  call sites of the top block).
- **FR-023**: `docs/v3-tradeoffs.md` MUST gain one row per engine touch point of this milestone,
  in the commit that introduces it: root motion read from the `AnimationTree`; `move_and_slide`
  + post-move origin; the crosshair raycast; the three noise samples; `slerp`/`looking_at`
  (engine-backed math); bullet instancing from the tick; the `AnimationTree` ordering decision;
  replication as a projection (which node fields are written by `SyncOut` and read by `SyncIn`
  on which peers); RPC `call_local` timing (same-iteration argument of the Edge Cases).
  `CLAUDE.md`'s v3 section MUST be extended with the sub-bridge pattern and the two-`EngineQuery`
  tick shape.
- **FR-024**: Gates unchanged: `cargo build`, `cargo clippy` 0 warnings, `cargo test` green with
  the 36 pure tests preserved and at least one `run_system_once` test per new gameplay system
  (count ≥ 156 + the new tests).
- **FR-025**: Parity with `v2` MUST be evidenced by the headless harness on both trees (V3-A's
  recipe: `v2` worktree, split `XDG_DATA_HOME`, `--fixed-fps 60`, observer at `i32::MIN`, the
  real `user://` path, diffs pasted) with `seed(1)` FIRST THING in the harness `_ready`
  (before instantiating `player.tscn`, because `CameraNoiseShake` draws `randi()` at init), a
  floor and one `player.tscn` with `player_id = 1` (single peer → both markers), and these six
  cases: (a) walk — `Input.action_press("move_forward")` from frame 10 to 100 then release; per
  physics step: body origin, velocity, `motion`, `current_animation`, `PlayerModel` global
  basis; (b) jump+land — `action_press("jump")` for one frame at 30; RAW stamps of
  `SoundEffects/Jump.playing` and `Land.playing` transitions, plus `current_animation` logged
  by the observer EVERY frame (the transient-write check of the Edge Cases); (c)
  aim+shoot — `action_press("aim")` from 20, `action_press("shoot")` from 40; per frame
  `InputSynchronizer.shoot_target`, `aiming`, `shooting`, `FireCooldown.time_left`, bullet
  child count under the root, the first bullet's global transform on its spawn frame,
  `AnimationTree` `parameters/aim/add_amount`; (d) camera shake — `player.hit()` at frame 30;
  per frame `Camera3D.rotation`; (e) mouse look — `Input.parse_input_event` with an
  `InputEventMouseMotion` (`screen_relative = (20, -10)`) at frame 25; per frame
  `CameraBase.rotation.y`, `CameraRot.rotation.x`; (f) fall/respawn — floor removed at frame 50;
  per frame origin, `ColorRect.modulate.a`, the respawn frame. Every differing frame goes to
  the "Measured timing differences" table with its cause.
- **FR-026**: User visual checkpoints, both REQUIRED: (1) single player — walk, jump, aim,
  shoot a robot, get hit (shake), fall off the map and respawn, all as in v2, and a note on
  whether backlog #29's FPS observation persists; (2) multiplayer on one machine — host in one
  instance, join from a second through the demo's menu: the remote player's movement, animation
  (`current_animation`/`motion` projection) and aim replicate as in v2, and each client controls
  only its own player. This is the first time the replication projection is exercised; (2) is
  not optional.

### Key Entities

- **Player entity**: one entity per `player.tscn` instance; components `PeerId`, `Simulates`,
  `OwnsInput` (markers), `Motion`, `Orientation`, `RootMotion`, `AirborneTime`,
  `InitialPosition`, `CurrentAnimation`, `AimState`, the input state read from the node
  (`aiming`, `shoot_target`, `motion`, `shooting`, `jumping`), the per-step body/camera
  snapshot, `Trauma`, `ShakeTime`, `StartRotation`, and per-tick intents (`Land`, `Jump`,
  `Shoot`, `Respawn`, `CameraCue`, `JumpPressed`). Exact names and whether intents are markers
  or fields: plan decisions.
- **Handles (`NodeHandles`, `NonSend`)**: the `Player` root and its eleven children; the six
  camera-side nodes (`CameraBase`, `CameraRot`, `Camera3D`, `Animation`, `Crosshair`,
  `ColorRect`); the three `FastNoiseLite`; the input node itself; the bullet `PackedScene`.
- **Inbound events**: `Register`/`Unregister` (root only), `JumpPressed`, `MouseLook`,
  `AddTrauma`, `PlayerFx::{Jump, Land, Shoot, Hit}` — all keyed by the root's `InstanceId`.
- **Sets**: fixed `SyncIn → Gameplay → EngineQueryOrient → GameplayIntegrate → EngineQueryMove →
  GameplaySettle → SyncOut`; frame `SyncIn → Gameplay → EngineQuery → SyncOut` (V3-A's).
- **Replicated projections**: `Player.motion`, `Player.current_animation` (written by `SyncOut`
  on `Simulates`, read by `SyncIn` elsewhere); `InputSynchronizer.{aiming, shoot_target,
  motion, shooting}` (written on `OwnsInput`, read by every entity's `SyncIn`); the camera
  rotations and transforms are replicated by the engine from the node values the sync layer
  writes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build` / `cargo clippy` (0 warnings) / `cargo test` clean; the 36 pure
  tests of the three modules preserved by name; at least one `run_system_once` test per new
  gameplay system (tick steps, replay, input, aim, fade, shake) — total ≥ 156 + 10.
- **SC-002**: `grep -n 'fn process\|fn physics_process' player.rs player_input.rs
  camera_noise_shake.rs` returns nothing; `fn input` exists only as the push-only sub-bridge
  callback; `godot::task::spawn` and `bind_mut()` on `EcsWorld` appear nowhere in the three
  modules.
- **SC-003**: Headless import and `main.tscn`/`level.tscn` show no new errors after every
  commit.
- **SC-004**: Harness cases (a)-(f) produce identical logs on `v2` and `v3`, OR every differing
  frame is listed in the "Measured timing differences" table with its cause — no third outcome.
  The RAW stamps of (b) and (c) show the local RPC effects in the same iteration on both trees.
- **SC-005**: Both visual checkpoints pass, including the two-instance multiplayer session.
- **SC-006**: Review checklist of constitution 1.5.1 passes: no bridge with per-frame
  callbacks; engine access only in sync/query systems and bridges; components without `Gd`;
  `docs/v3-tradeoffs.md` has the nine rows of FR-023; the untouched files of FR-020 diff-empty
  against `abfe35a`; `player.tscn` diff limited to FR-018's property (or empty); backlog #6/#29
  still open.
- **SC-007**: The `AnimationTree` experiment's per-step log (root motion position, transition
  request, origin) is identical on both trees with the implemented option, before the harness
  runs.

## Measured timing differences

Filled during implementation, before the closing commit. An empty table means the six harness
diffs (and the US4 experiment) were empty.

| Case | Signal / timer | `v2` frame | `v3` frame | Delta | Cause |
|---|---|---|---|---|---|
| (to be measured) | | | | | |

Candidates the harness must settle: the local RPC effects (`Jump`/`Land`/`Shoot` sounds,
`FireCooldown.start`) now applied by the frame schedule of the same iteration instead of inline;
the mouse-look event applied by the frame run instead of inside `input`; the camera cue
animations played from `SyncOut`; the respawn reset applied in `SyncOut` after `move_and_slide`
(same step as v2's inline reset).

## Assumptions

- Phase v3 (constitution 1.5.1). Zero behavior deviations from `v2` are sanctioned beyond
  measured frame-level shifts recorded in the table; the one scene edit of FR-018 (if B) changes
  no observable behavior — that is its acceptance criterion.
- **To verify at research time**: (1) `AnimationMixer::advance(f64)` exists in the bindings
  (`animation_mixer.rs:347`) and, with `callback_mode_process = MANUAL` (`ord 2`,
  `animation_mixer.rs:529-531`), the tree advances only when called — the experiment of US4
  settles behavior; (2) `PhysicsDirectSpaceState3D::intersect_ray` from a frame `EngineQuery`
  system inside `_process` is legal (v2 already does it in `process`); (3) `Input.action_press`
  drives `get_action_strength`/`is_action_just_pressed` headless and `Input.parse_input_event`
  reaches `Node::input` headless, with input events dispatched BEFORE `_process` of the same
  iteration (harness case (e) settles the frame); (4) the seven-set fixed chain of FR-005 is
  expressible with `configure_sets(...).chain()` without disturbing V3-A's systems (the door
  system in the first `Gameplay`); (5) `MultiplayerSynchronizer` sends the node values present
  when `SceneMultiplayer::poll()` runs at the top of `SceneTree::process` — i.e. after the fixed
  run's `SyncOut` of the same iteration and BEFORE the frame run, the same relative order as
  v2's `physics_process` writes — LOAD-BEARING for FR-015 (the transient `Jump`/`Land`
  animation write, Edge Cases): research MUST cite `scene/main/scene_tree.cpp` /
  `scene_multiplayer.cpp` for the poll point; (6) `set_player_id`'s engine write before `ready` does not interact with `EcsWorld`
  (it uses a typed child handle, `player.rs:128-130`); (7) `PlayerModel/AnimationPlayer`
  (`player.tscn:576-577`) stays inactive while the `AnimationTree` is active, so only the tree's
  callback mode matters; (8) `get_owner()` of the camera node is the `Player` root (nodes saved
  in `player.tscn` are owned by its root) — the sub-bridge may use it instead of a four-level
  `get_parent()` chain.
- The bullet's own bridge is out of scope (`bullet.rs` stays v2): the player spawns it exactly
  as v2 does, from the first `EngineQuery` set; the bullet registers nothing yet.
- Exact Rust shapes (component and event names, whether intents are markers or fields, whether
  the two `EngineQuery` sets are `Phase` variants or nested sets, whether `PlayerFx` is one
  event enum or four, the drain's application order for several events of one root in one
  frame — FIFO, as V3-A) are plan-time decisions; the spec pins behavior and order.
- The harness reuses V3-A's `zz_ecs_parity.gd` shape (observer, RAW lines, `--case=`), extended
  with the six cases; the three scratch files are never committed.
- Out of scope: `bullet`, `part`, `red_robot`, `flying_forklift` (later milestones); backlog
  #6/#29 fixes; any change to the `Animations` enum, the replication configs or the
  `node_paths`; `EngineQuery` members other than the ones named (orientation math + animation
  parameters + root motion + bullet spawn; velocity + `move_and_slide`; camera rotation +
  raycast; noise samples).
