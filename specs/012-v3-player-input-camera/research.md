# Research: Milestone V3-B — the player as one entity over three nodes

Evidence from the working tree at `abfe35a`/`b309e2f` (`player.rs`, `player_input.rs`,
`camera_noise_shake.rs`, their `model.rs`, `player.tscn`, `src/ecs.rs`, `src/ecs/*`), the
Godot 4.7 engine sources fetched from the `4.7` branch on 2026-09-18 (`scene/main/scene_tree.cpp`,
`modules/multiplayer/scene_multiplayer.cpp`, `scene_replication_interface.cpp`,
`multiplayer_synchronizer.cpp`), the generated gdext bindings, `bevy_ecs 0.19.1`, and THREE
headless experiments run on 2026-09-18 (scratch scripts reproduced under `contracts/`).

## R1 — The `AnimationTree` ordering (EXPERIMENT, spec US4 / FR-018)

**Decision: option (B).** `player.tscn:592` changes from `callback_mode_process = 0` to
`callback_mode_process = 2` (MANUAL) on the `AnimationTree`, and the fixed schedule's `SyncOut`
calls `animation_tree.advance(delta)` for EVERY player entity — `Simulates` or not — as the LAST
engine write of the tick for that entity (after the model basis, the respawn reset and the
projection writes; before the RPC calls, which only push events). Option (A) is rejected: a driver
at `i32::MAX` reads the root motion one step early at every step (below).

**Experiment** (`contracts/zz_r1_probe.gd`): a floor, `player.tscn` with `player_id = 1`,
`seed(1)` first, `--fixed-fps 60 --quit-after 135`, `move_forward` pressed from frame 10 to 100.
Two probes in `_physics_process`: `ProbeMin` at physics priority `i32::MIN` (runs BEFORE
`Player::physics_process`, i.e. at the point where v2's tick reads) logs `S<step> rm=<root motion
position> origin vel anim blend`; `ProbeMax` at `i32::MAX` (runs AFTER the player's tick AND after
the tree's own PHYSICS processing — the point where an option-A driver would read) logs
`M<step> rm origin`. `ZZ_MODE=manual` sets the tree to MANUAL and has `ProbeMax` call
`advance(delta)` right after the player's tick (option B's placement). Runs: (i) the `v2`
worktree in PHYSICS mode; (ii) `v3` in PHYSICS mode; (iii) `v3` in MANUAL+advance. On both trees
the player code is v2's (the `v3` tree has not touched `player.rs` yet), which is what makes the
probes comparable.

Results (263 log lines per run):

- `diff (i) (ii)`: **no differences** — the `v3` build behaves as `v2` for the player today.
- `diff (i) (iii)`: **no differences except the `MODE` header line** — advancing a MANUAL tree
  right after the tick reproduces PHYSICS mode exactly, step by step, including the first steps
  (`S1 rm=0 origin=(0,0.05,0)`, `M1 rm=0`, `S2 rm=0`: the tree's first processing/advance
  happens after tick 1 in both modes; nothing processes before the first tick).
- Option-A read-point shift, every step: `M(n).rm == S(n+1).rm` (e.g. step 12: the tick reads
  `S12 rm=0`, a MAX driver would read `M12 rm=(0,0,0.004791)` which is `S13`'s value; step 13:
  `S13 0.004791` vs `M13 0.010979 = S14`; …). An option-A tick would therefore integrate each
  step's root motion one step earlier than v2 and the origin log would diverge from step 13 on
  (root motion accumulates while walking: `origin.z` 0.004791 at S14, 3.51 at S60, 7.05 at
  S101). Rejected by the spec's parity criterion.

Consequences pinned by this decision: the tree no longer processes itself, so `advance` MUST
run every fixed run for every registered player (a non-`Simulates` player animates through the
replay path + advance); `advance(delta)` uses the fixed delta (`FixedDelta`); the transition
request and blend parameters written by the tick in `EngineQueryOrient` are consumed by that
same run's `advance`, exactly as v2's child processing consumed them; the root motion read by
the NEXT run's `EngineQueryOrient` is the one this run's `advance` produced. `get_owner()` of the
camera and input nodes is the `Player` root (logged `OWNER camera.owner==player: true ;
input.owner==player: true` on both trees) — spec Assumption (8) confirmed.

## R2 — Where replication samples node values (spec Assumption (5), FR-015)

**Facts** (Godot 4.7 sources): `SceneTree::process` (`scene_tree.cpp:688`) calls
`multiplayer->poll()` (`:707`) BEFORE `_process(false)` (`:719`) and `process_timers` (`:729`);
`SceneMultiplayer::poll()` (`scene_multiplayer.cpp:67`) polls the peer, reads packets, then the
replication interface's `on_network_process()` (`scene_replication_interface.cpp:129`) runs
`_send_sync` (`:151`, `:802`) for every peer: it calls `MultiplayerSynchronizer::get_state(props,
node, …)` (`:829`) — the node property values AT THAT MOMENT — gated by
`update_outbound_sync_time` (`multiplayer_synchronizer.cpp:124-135`): with `replication_interval`
= 0 (the default; `player.tscn` sets no interval on either synchronizer, `:336-340`) it sends on
every poll, i.e. once per RENDERED frame, never per physics step. The synchronizer's own
`NOTIFICATION_INTERNAL_PROCESS`/`PHYSICS_PROCESS` (`:306-311`) only updates visibility filters.
`player_id` uses `replication_mode = 0` (NEVER) with `spawn = true` (`player.tscn:19-21`): sent
at spawn only.

**Consequence**: the sample point is after ALL physics steps of the iteration (so after the fixed
run's `SyncOut`) and BEFORE the frame run. A value written by the FRAME run of iteration N is
visible to the poll of iteration N+1 only if iteration N+1 has NO physics step before its poll
— which happens whenever rendering runs faster than 60 Hz (v2's default settings have VSync
OFF; at 144 fps ~58% of iterations carry no physics step).

**Decision (superseded at plan review by option (b), see R7 and FR-004)**: originally, the
`PlayerFx::Jump`/`Land` handlers' `JumpUp`/`JumpDown` plan writes were to be DROPPED on
`Simulates` entities with only the sounds applied. With `jump`/`land`/`shoot` now `call_remote`,
no handler runs on the `Simulates` peer at all — its effects are applied inline by the fixed
`SyncOut` — so the transient write cannot occur there; the analysis below still governs the
non-`Simulates` path and is why the local path had to move out of the frame run. Reasoning:
in v2 both writes happened INSIDE the physics step (`rpc(...)` with `call_local` at `player.rs:247`/
`:250`, before step (5) at `:254-310`), and step (5)'s `apply_anim` always overwrote
`current_animation` and the tree's `transition_request` before the tree processed and before any
poll — the transient never reached the tree (last write wins: the transition node reads the
request when the tree processes) nor the network. In v3 the same write would land in the FRAME
run, after the fixed run's projection write and after the tree's `advance`, so (a) it could be
sampled by the next poll in a physics-less iteration (a client would receive `JumpDown` for one
packet and its replay would show it), and (b) it would leave a stale `transition_request` for the
next `advance` — overwritten by the next tick in practice, but not by construction. Dropping the
write reproduces v2's observable state at every sample point; the harness's case (b) logs
`current_animation` every frame to prove it. On NON-`Simulates` entities the plan write is kept
(v2's client-side handler behaved the same way; the client's replay write of the next fixed run
precedes its `advance`, so the outcome is v2's). `Shoot`'s effects (particles, cooldown start,
sound, trauma 0.35) and `hit`'s trauma (`AddTrauma { 0.75 }`, no `PlayerFx::Hit` variant) are not
animation writes and are applied on every peer.

## R3 — Headless input driving (EXPERIMENT, spec Assumption (3))

**Experiment** (`contracts/zz_r3_probe.gd`): a root node at priority 0 calls
`Input.action_press("move_forward")` in `_process` at frame 10 and `action_release` at 12, and
`Input.parse_input_event(InputEventMouseMotion{screen_relative=(20,-10)})` at frame 25; a child
`Reader` at priority `i32::MAX` logs the action state per frame; a child `Listener` logs
`_input` and `_process` with a global sequence counter. Output:

```
F10 seq=31 action_press(move_forward)
F10 seq=33 reader(MAX) strength=1.0 just_pressed=true pressed=true
F11 seq=36 reader(MAX) strength=1.0 just_pressed=false pressed=true
F12 seq=37 action_release(move_forward)
F12 seq=39 reader(MAX) strength=0.0 just_pressed=false pressed=false
F25 seq=76 parse_input_event(mouse 20,-10)
F25 seq=77 listener._process
F26 seq=79 listener._input(mouse relative=(20.0, -10.0))
F26 seq=80 root._process
```

**Decisions**: (1) a press made in a node's `_process` is visible with `just_pressed = true` to
every later node of the SAME frame and `just_pressed` is true for exactly one frame — so a
harness driver at priority 0 (the scene root) pressing at frame N is seen by v2's input node
(a descendant, later in tree order) and by the v3 driver (at `i32::MAX`) in the same frame N;
(2) `parse_input_event` from `_process` is buffered and delivered at the START of the next
iteration, and `_input` runs BEFORE `_process` there — so a mouse event injected at frame 25
reaches `_input` at frame 26 on both trees, v2 rotates the camera inside that `_input`, and v3's
`MouseLook` push at frame 26 is drained by iteration 26's first schedule run and applied by its
frame run: the observer at frame 27 sees the same rotation on both trees. Harness frame
conventions (cases (a)–(f)) are written with these two facts.

## R4 — One `Phase` enum, two chains (EXPERIMENT in a probe crate)

**Decision**: option (a). `Phase` becomes `{ SyncIn, Gameplay, EngineQueryOrient,
GameplayIntegrate, EngineQueryMove, GameplaySettle, EngineQuery, SyncOut }`; `build_fixed`
chains the seven `SyncIn → Gameplay → EngineQueryOrient → GameplayIntegrate → EngineQueryMove →
GameplaySettle → SyncOut`; `build_frame` keeps V3-A's `SyncIn → Gameplay → EngineQuery → SyncOut`.
A probe crate with `bevy_ecs 0.19.1` (same pin) proved: (1) `configure_sets(...).chain()` accepts
a chain that omits variants; (2) systems added in reverse order run as `[sync_in,
gameplay(door stays here), orient, integrate, move, settle, sync_out]` in the fixed schedule and
`[sync_in, gameplay, engine_query, sync_out]` in the frame schedule; (3) a component spawned via
`Commands` in `Gameplay` is visible to a query in `EngineQueryOrient` of the SAME run (bevy's
`auto_insert_apply_deferred`, V3-A R6) — printed `true`. `setup.rs`'s existing three tests stay
and gain `fixed_sets_are_chained_in_order_and_frame_sets_unchanged`. V3-A's systems keep their
sets: `open_on_player` in `Gameplay`, `sweep_dead_nodes` in `SyncIn`, `sync_out_*` in `SyncOut`.
Why this still respects the constitution's `SyncIn → gameplay → EngineQuery → SyncOut` rule: the
rule names "`EngineQuery` sets" (plural) "only where a decision needs an engine answer mid-tick";
the player's tick needs two such answers (the orientation slerp + root motion before integrating;
`move_and_slide`'s result before deciding the respawn), each followed by a pure step, and every
engine-touching system is still confined to an `EngineQuery*` or sync set.

## R5 — Entity shape and registration

**Decision**:

```rust
Handles::Player {
    root: Gd<Player>,                        // the user class: projection writes (`motion`, `current_animation`) need bind_mut(); Deref reaches CharacterBody3D/Node for the engine calls
    input: Gd<PlayerInputSynchronizer>,      // user class: its replicated fields are read/written via bind()/bind_mut() (glue)
    anim_tree: Gd<AnimationTree>, model: Gd<Node3D>, shoot_from: Gd<Marker3D>,
    shoot_particle: Gd<CpuParticles3D>, muzzle_particle: Gd<CpuParticles3D>,
    fire_cooldown: Gd<Timer>, snd_jump: Gd<AudioStreamPlayer>, snd_land: Gd<AudioStreamPlayer>, snd_shoot: Gd<AudioStreamPlayer>,
    camera_base: Gd<Node3D>, camera_rot: Gd<Node3D>, camera: Gd<Camera3D>,   // camera upcast: only set_rotation is needed after ready
    camera_anim: Gd<AnimationPlayer>, crosshair: Gd<TextureRect>, color_rect: Gd<ColorRect>,
    noise: [Gd<FastNoiseLite>; 3], bullet_scene: Gd<PackedScene>, parent_rid: Rid,
}
Initial::Player { peer_id: i32, simulates: bool, owns_input: bool, initial_position: Vector3, orientation: Transform3D, start_rotation: Vector3 }
```

`Player.ready` builds it: its own eleven `OnReady` handles (`player.rs:48-68`) and `bullet_scene`
(`:72-73`); the six camera-side handles through `self.player_input.bind()` (`camera_base`,
`camera_rot`, `camera_camera`, `camera_animation`, `crosshair`, `color_rect` — `OnEditor` values
resolved because the child's `ready` ran first, and the parent RID the child resolved,
`player_input.rs:26-29`); the three `FastNoiseLite` and `start_rotation` through
`camera_camera.cast::<CameraNoiseShake>().bind()` (seeded and captured in the camera's `ready`,
which also ran first). `simulates = get_multiplayer().is_server()` (v2 `player.rs:98`);
`owns_input = input.get_multiplayer_authority() == get_multiplayer().get_unique_id()`
(v2 `player_input.rs:61-62`, read on the input node). Sub-bridges resolve the root id ONCE:
`#[init(val = OnReady::from_base_fn(|b| b.get_owner().unwrap().instance_id()))] root_id:
OnReady<InstanceId>` — `get_owner()` is the `Player` for both nodes (R1's `OWNER` line), so no
parent chain. The input node keeps `parent`/`parent_rid` (`:26-29`) for the RID only.

## R6 — Component model of the tick

**Decision** (persistent, per entity): `Motion(Vector2)`, `Orientation(Transform3D)`,
`RootMotion(Transform3D)`, `AirborneTime(f32)`, `InitialPosition(Vector3)`,
`CurrentAnimation(Animations)`, `AimState` (the model's enum, `Component` via a newtype
`AimStateC(AimState)` so `player_input/model.rs` stays untouched), `Trauma(f32)`,
`ShakeTime(f64)`, `StartRotation(Vector3)`, `PeerId(i32)`, markers `Simulates`, `OwnsInput`,
`PlayerTag`; queued by the drain: `JumpQueued(bool)`, `PendingMouseLook(Vec<Vector2>)`,
`PendingFx(Vec<PlayerFx>)`. Per-run snapshots written by `SyncIn`: `InputFrameC(InputFrame)`
(the model's struct wrapped — `player/model.rs:38-48`), `BodyState { on_floor, velocity,
gravity, cooldown_left, origin_y }` (`origin_y` is written ONLY by `move_body`, post-move —
`SyncIn` does not read the origin, v2 read it once at `:329`), `ReplicatedInput { aiming, shoot_target, motion, shooting }`
(frame run, `OwnsInput` only: the values this peer will project into the input node; the fixed
run reads the node directly into `InputFrameC` for every entity — analyze finding 14), `ReplayState { current_animation, motion,
aim_rotation }` (non-`Simulates`), `InputSnapshotC(InputSnapshot)` + `CameraFrame { parent_y, fade_alpha }` (frame, `OwnsInput`; the live camera rotations are read
per delta by `camera_and_ray`, as v2's `rotate_camera` did, so they are not snapshotted). Per-run outputs: ONE `TickIntents`
struct (`land`, `jump`, `shoot`, `respawn: bool`, `jump_velocity_y: Option<f32>`, `orient:
Option<OrientTarget>` with `OrientTarget::Camera(Quaternion) | Walk(Vector3)`, `plan: AnimPlan`,
`root_motion_read: bool`) written by `tick_decide` and read by THREE later systems
(`orient_and_anim`, `move_body`, `sync_out_player`) — a struct, per V3-A R6's rule (markers for a
single `SyncOut` consumer, a struct when several systems read the same intent set);
`FrameIntents` (`camera_deltas: Vec<Vector2>` in application order, `cue: Option<CameraCue>`,
`jump_pressed`, `shooting`, `fade_alpha`) for the frame run; `ShakePending(Option<(f32, f64)>)`
for the shake (consumed by `sync_out_shake`; no `ShakeOffset`). `Remove`-like one-shots stay markers (none new). Tuning: `#[derive(Resource)] pub struct
Tuning<T>(pub T)` in `ecs/markers.rs`, inserted by `build_world` as
`Tuning(PlayerTuning::default())`, `Tuning(PlayerInputTuning::default())`,
`Tuning(CameraShakeTuning::default())` — the constitution's "tuning structs become
`Res<XxxTuning>`" without editing the model files (they keep their `Default`).

## R7 — The systems

Fixed schedule (per player entity; `Simulates` filter where stated):

| Set | System | Parameters (shape) | v2 lines |
|---|---|---|---|
| `SyncIn` | `sync_in_player` (glue, `player/sync.rs`) | `NonSend<NodeHandles>`, `Query<(Entity, &JumpQueued, …), With<PlayerTag>>`, `Commands` → writes `InputFrameC` (from `input.bind()` fields — the projection's consumer for EVERY entity — + the three camera reads, `aim_rotation` via `Res<Tuning<PlayerInputTuning>>`), `BodyState` (no origin read: `origin_y` is written post-move by `move_body`); clears `JumpQueued`; for non-`Simulates` writes `ReplayState` | `:207-223`, `:235`, `:241`, `:274`, `:313-314`; `:104-117` |
| `Gameplay` | `tick_decide` (pure, `player/system.rs`) | `Res<Tuning<PlayerTuning>>`, `Res<FixedDelta>`, `Query<(&InputFrameC, &BodyState, &mut Motion, &mut AirborneTime, &mut TickIntents, &Orientation), With<Simulates>>` — steps (2), (3), (4), the branch decision, `anim_plan`, `walk_target`, the shoot decision | `:226-258`, `:266`, `:274`, `:294-303` |
| `EngineQueryOrient` | `orient_and_anim` (glue) | slerp / `looking_at` + slerp into `Orientation`; root motion read into `RootMotion` (skipped when airborne; the value is the previous `advance`'s, independent of this step's parameter writes — R1); bullet spawn when `intents.shoot` (a write, but it MUST precede `move_and_slide`). The `AnimationTree` parameter writes moved to `sync_out_player` (analyze finding 3) | `:261-264`, `:296-300`, `:269-272`/`:306-309`, `:275-291` |
| `GameplayIntegrate` | `tick_integrate` (pure) | `integrate_root_motion` → new `Orientation`, `Velocity(Vector3)` | `:313-317` |
| `EngineQueryMove` | `move_body` (glue) | `set_velocity`, `set_up_direction(UP)`, `move_and_slide`, read post-move origin into `BodyState.origin_y` | `:320-322`, `:329` |
| `GameplaySettle` | `tick_settle` (pure) | `should_respawn` → `intents.respawn` | `:329` |
| `SyncOut` | `sync_out_player` (glue) | `model.set_global_basis`; respawn reset; projection writes into the `Player` node (`motion`, `current_animation`) through `root.bind_mut()` (`root: Gd<Player>`), the guard DROPPED before any further engine call; then, on `Simulates`, the LOCAL effects inline in v2's order (option (b), FR-004): `Land` sound, `Jump` sound, then the `Shoot` effects (particles restart+emit, `fire_cooldown.start()`, sound, `Trauma += 0.35`); then `rpc("land")`, `rpc("jump")`, `rpc("shoot")` — now `call_remote`, reaching remote peers only; then the `AnimationTree` parameter writes (`apply_anim(&mut anim_tree, plan)` — the `Simulates` plan or the non-`Simulates` replay plan, v1's per-variant order, `:179-200`; moved here from `EngineQueryOrient`, analyze finding 3); LAST: `anim_tree.advance(FixedDelta)` for every player | `:326`, `:330-333`, `:134-154`, `:246-251`, `:290`, `:179-200`; R1 |

Frame schedule:

| Set | System | v2 lines |
|---|---|---|
| `SyncIn` | `sync_in_input` (glue, `player_input/sync.rs`): on `OwnsInput` only: the ten `Input` reads → `InputSnapshotC`; parent y, `modulate.a` → `CameraFrame` (no rotation snapshot: `camera_and_ray` reads the live rotation per delta) | `:76-90`, `:144-146` |
| `Gameplay` | `input_decide` (pure, `player_input/system.rs`): drains `PendingMouseLook` into `scaled_mouse_look` deltas FIRST, then `scaled_look` for the controller; `clamp_pitch` per delta in order; `step_aim` + cue; `alpha_for_height`; `jump_pressed`, `shooting` → `FrameIntents`, updates `AimStateC` | `:92-99`, `:111`, `:115`, `:146`, `:153` |
| `Gameplay` | `shake_decide` (pure, `camera_noise_shake/system.rs`): while `Trauma > 0`: `decay`, `advance_time`, `shake` → `ShakePending { shake, time }`; else `None` | `:45-49` |
| `EngineQuery` | `camera_and_ray` (glue): applies each camera delta as `rotate_y`/`orthonormalize`/`set_rotation` in order, THEN the crosshair raycast when shooting → `ReplicatedInput.shoot_target` | `:184-191`, `:117-138` |
| `SyncOut` | `sync_out_input` (glue): camera cue play; `input.bind_mut()` writes of `aiming`, `shoot_target`, `motion`, `shooting`; `color_rect.set_modulate`; `input.rpc("jump")` when jump pressed | `:100-109`, `:92/:99/:115/:135-137`, `:147`, `:111-113` |
| `SyncOut` | `apply_player_fx` (glue): drains `PendingFx` in order — only ever non-empty on non-`Simulates` entities (remote peers; option (b)) — `Jump`: `JumpUp` plan write + sound; `Land`: `JumpDown` plan write + sound; `Shoot`: both particles restart+emit, `fire_cooldown.start()`, sound (its trauma was already applied at drain time) | `:134-154` |
| `SyncOut` | `sync_out_shake` (glue): when `ShakePending` is `Some((shake, time))`: three `get_noise_1d(time as f32)` → `offsets(shake, samples, &tuning)` (pure, called from glue) → `camera.set_rotation(start_rotation + offset)`. NO `EngineQuery` member for the shake (analyze, 2026-09-18): the samples answer no mid-tick decision, so a sync-only pair is the honest shape (V3-A blast precedent); `ShakeOffset` is dropped | `:50-57` |

Drain (`ecs/apply.rs`, pure): `JumpPressed { root_id }` → `JumpQueued = true`; `MouseLook {
root_id, screen_relative }` → push into `PendingMouseLook`; `AddTrauma { root_id, amount }` →
`Trauma = model::add_trauma(trauma, amount as f32, &tuning)` (this is also what `hit` pushes:
`AddTrauma { 0.75 }`); `PlayerFx { root_id, fx }` → push into `PendingFx`, and for `Shoot` ALSO
`Trauma += 0.35` at drain time so the same run's `shake_decide` sees it (plan review: applying
it in `SyncOut` would start the shake one frame late). No `Messages` for these (single consumer each, keyed to the entity, consumed and
cleared by the consuming system — exactly-once by construction, no per-schedule `update()` rule);
the door keeps its `Messages`. `jumping` is DROPPED from the input node (backlog #9 already removed
its export; nothing reads it once `JumpQueued` exists); `CameraNoiseShake::add_trauma` is deleted.
The three `pub(crate)` getters of `player_input.rs` (`:171-182`) are deleted too: `sync_in_player`
reads the camera nodes through the handles.

## R8 — Engine-call budget (a record, not a goal)

Per player per physics step, walking (explicit calls read from the code): v2 = 3 camera reads +
`is_on_floor` + `get_velocity` + `get_gravity` + `looking_at` + `slerp` + 3 tree `set` + 2 root
motion reads + `set_velocity` + `set_up_direction` + `move_and_slide` + `set_global_basis` +
`get_transform` = 18 (`player.rs:217-329`); v3 = the same 18 + `fire_cooldown.get_time_left`
(read every step at `SyncIn`, v2 read it only while aiming+shooting: +1) + `advance` (+1, replacing
the tree's own processing) + the FR-009 root sweep (+1) = 21. Per frame, `OwnsInput` idle: v2 = 10
`Input` reads + 4 camera rotation calls + `get_global_transform` + `get_modulate`/`set_modulate` =
17 (`player_input.rs:76-147`); shooting adds 2 crosshair reads + 2 projections + 3 for the ray
(+7); v3 = the same, plus reading the four replicated fields (a `bind()`, no FFI) and writing
them (`bind_mut()`, no FFI): 17/24 + the sweep. Shake while `Trauma > 0`: 3 + 1 on both.

## R9 — Parity harness

`contracts/zz_ecs_parity.gd` (V3-A's shape: observer at `i32::MIN`, fixed line formats, RAW
lines, `--case=`) extended with cases (a)–(f) exactly as FR-025 states; `seed(1)` is the FIRST
statement of `_ready` (before `load(...).instantiate()` of `player.tscn`, which draws `randi()`
in `CameraNoiseShake`'s init); the scene: a `StaticBody3D` floor (removable for (f)), one
`player.tscn` at `(0, 0.05, 0)` with `player_id = 1` (single peer: `is_server()` true, authority
1 → both markers), no `level.tscn` (`Settings` autoload is present in both trees; `bullet.tscn`
is instanced by the player's own code in (c) and its transform is read by the harness on the
frame the child count under the harness root increases). Drives: `Input.action_press/release`
from the harness root's `_process` (R3), `Input.parse_input_event` at frame 25 (case (e), R3),
`player.hit()` at frame 30 (case (d); an `#[rpc]` is a callable method), `floor.queue_free()` at
frame 50 (case (f)). Logs per FR-025; RAW lines: `Jump.playing`/`Land.playing`/`Shoot.playing`
transitions, `FireCooldown.time_left` on the shoot frame, the bullet's spawn frame and global
transform, the respawn frame. Commands as V3-A R10 (`../oxide-godot-v2` worktree, split
`XDG_DATA_HOME`, the real `user://` path, diffs pasted).

## R10 — Commit plan

Option (B) is required for parity of case (a) (R1), so the `.tscn` edit and `advance` land in the
player commit. The game stays playable after every commit: commit 2 keeps the v2 trauma path
(the `Player` handlers still call `camera.bind_mut().add_trauma`) until commit 3 replaces it.

| # | Commit | Files | Gate + validation |
|---|---|---|---|
| 1 | `ecs: seven fixed-schedule sets (Phase extension), Tuning<T> resources, drain arms for JumpPressed/MouseLook/AddTrauma/PlayerFx (tests)` | `ecs/setup.rs`, `ecs/markers.rs`, `ecs/event.rs`, `ecs/apply.rs`, `ecs.rs` (Handles::Player, Initial::Player; a `Player` arm in `sync_out_remove`'s exhaustive match) | gates (156 + 2 setup + 5 apply = 163 tests) |
| 2 | `player + player_input: bridges, Player entity (Handles::Player), fixed tick (7 sets) and frame input systems; AnimationTree MANUAL + advance (R1 option B)` | `player.rs`, `player/system.rs`, `player/sync.rs`, `player_input.rs`, `player_input/system.rs`, `player_input/sync.rs`, `player.tscn:592`, `ecs.rs` (registration of the systems), `docs/v3-tradeoffs.md` (rows: root motion, move_and_slide, raycast, slerp/looking_at, bullet spawn, AnimationTree ordering, replication projection, RPC call_local timing) | gates (+ ≥ 11 tests); headless; harness (a), (b), (c), (e), (f) on both trees |
| 3 | `camera_noise_shake: sub-bridge, shake systems on the player entity, AddTrauma/PlayerFx trauma path` | `camera_noise_shake.rs`, `camera_noise_shake/system.rs`, `camera_noise_shake/sync.rs`, `player.rs` (handlers now push only), `ecs.rs`, `docs/v3-tradeoffs.md` (row: noise samples) | gates (+ ≥ 2 tests); headless; harness (d) + rerun (b)/(c) (trauma path changed); **STOP 1 — checkpoint (1) single player**; **STOP 2 — checkpoint (2) multiplayer, two instances** |
| 4 | `CLAUDE.md: v3 sub-bridge pattern + two-EngineQuery tick; spec: measured timing differences filled; tradeoffs complete` | `CLAUDE.md`, `spec.md`, `docs/v3-tradeoffs.md` | docs-only |

Two STOPs after commit 3, in sequence (single player first; multiplayer second), both before the
docs commit; checkpoint-mark commits as in V3-A.
