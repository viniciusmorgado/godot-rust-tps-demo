# Research: Milestone V3-A — ECS core and the three leaf effects

Evidence from the current working tree (`door.rs`, `part_disappear.rs`, `blast.rs`, `part.rs`,
`red_robot.rs`, `lib.rs`, `project.godot`, the three host `.tscn`s), gdext 0.5.5 sources
(`~/.cargo/registry/src/index.crates.io-*/godot-core-0.5.5/`), the generated bindings for the
local Godot 4.7.2 build (`oxide_godot_core/target/debug/build/godot-core-*/out/classes/node.rs`),
`bevy_ecs 0.19.1` sources (`~/.cargo/registry/src/index.crates.io-*/bevy_ecs-0.19.1/`), the
Godot 4.7 engine sources (`scene/main/scene_tree.cpp`, `main/main.cpp`, fetched from the `4.7`
branch on 2026-09-18), and ONE headless experiment run against the local binary (R1 — scratch
scripts, throwaway, reproduced under `contracts/zz_order_probe.gd`).

## R1 — Driver-last process order (EXPERIMENT)

**Decision**: `EcsWorld` sets BOTH `process_priority` and `physics_process_priority` to
`i32::MAX` in its `ready` (`Node::set_process_priority(i32)` and
`Node::set_physics_process_priority(i32)`, generated bindings `node.rs:788` and `:806`). With
that value its `_process` runs after every other node's `_process` AND after the
`AnimationPlayer`'s internal processing (the one that emits `animation_finished`), and its
`_physics_process` runs after every other node's `_physics_process`. No node in the game sets a
priority today (`grep process_priority` over `oxide-godot/**/*.tscn` and `src/**/*.rs`: none),
so `i32::MAX` is unconditionally last; a future node that must run AFTER the driver cannot exist
by design, which is exactly the constitution's intent (one driver, everything else feeds it).

**Experiment** (scratch scene `zz_order.tscn`, autoload `zz_order_autoload.gd` registered
temporarily in `project.godot` and reverted, observer at `i32::MIN`, an `AnimationPlayer` with a
1-frame `blink` animation on autoplay, `SceneTreeTimer`s of 0.0, 1/60, 0.2 and 3.0 s created in
`_ready`, one more created from inside a `timeout` callback, a probe node `queue_free`d from a
`timeout` callback; run `--headless --fixed-fps 60 --quit-after 16`). Observed order, first
frame, at the two priorities:

| Priority of the autoload | Physics step P1 | Process pass F0 |
|---|---|---|
| `0` (default; tree order, autoloads first) | `observer(i32::MIN)` → **`autoload`** → `scene` | `observer(i32::MIN)` → **`autoload`** → `scene _process` → `anim_finished(blink)` → `timer0 timeout` → `probe queue_free` → `timer(1/60) timeout` → `probe tree_exited` |
| `2147483647` (`i32::MAX`) | `observer(i32::MIN)` → `scene` → **`autoload`** | `observer(i32::MIN)` → `scene _process` → `anim_finished(blink)` → **`autoload`** → `timer0 timeout` → `probe queue_free` → `timer(1/60) timeout` → `probe tree_exited` |

Facts the output establishes (all used below):

1. Priority sorts BOTH callbacks; ties keep tree order (the observer at `i32::MIN` is first
   even though it is a child of the scene root; the autoload at `0` precedes the scene because
   autoloads come first under `/root`).
2. The `AnimationPlayer`'s internal processing is ordered by the SAME priority key as
   `_process` (its `animation_finished` came right after the scene root's `_process`, both at
   priority 0, and BEFORE the autoload at `i32::MAX`). So a bridge `#[func]` connected to
   `animation_finished` pushes its event before the driver's frame schedule of the same frame.
3. `SceneTreeTimer`s fire after the WHOLE `_process` pass, whatever the priority (the
   `i32::MAX` autoload printed before every `timeout`). Engine source: `SceneTree::process`
   calls `_process(false)`, then `_flush_ugc()`, `MessageQueue::flush()`, then
   `process_timers(p_time, false)`, then `_flush_delete_queue()` (`scene/main/scene_tree.cpp`,
   4.7). Consequence: an event pushed from a timer callback (the game's puff registration,
   `part.rs:196-205` → `destroy` → `ready`) is consumed by the NEXT iteration's first schedule
   run, which is the fixed schedule of the next physics step — see R5 for why that is still
   frame-aligned with v2.
4. A `queue_free()` issued from a timer callback frees the node in the SAME frame
   (`probe tree_exited` printed in F0): `_flush_delete_queue()` follows `process_timers` in
   `SceneTree::process`. A `queue_free()` issued from the driver's `_process` (SyncOut) is
   therefore also flushed in the same frame.
5. A timer created during the timer pass is first stepped in the NEXT frame (`timer1(created in
   timeout)` fired at F1, not F0) — `process_timers` snapshots `timers.back()` before iterating.
6. `--fixed-fps 60` gives `delta = 0.01666666666667` on every `_process`; the 0.2 s timer fired
   at F12, i.e. on its 13th step (F0..F12), and the GDScript mirror of the engine's subtraction
   printed `0.0` at the 12th step — the engine did not fire there because the residual is a
   positive `4.86e-17` (verified in Python with IEEE doubles: `0.2 - 12 × (1/60)` by iterated
   subtraction = `4.857e-17`, the 13th subtraction goes negative). R5 builds the timer on this.
7. Physics then process within one iteration: `P1` printed before `F0`; `Engine.get_process_frames()`
   during iteration N's physics phase already reads N (`main.cpp:5119` increments
   `_process_frames` at the END of the iteration, after `process()` at `:5062`), so a raw frame
   stamp taken in the physics phase of iteration N+1 reads `N+1`.

The driver CAN be placed after the `AnimationPlayer`'s internal processing (fact 2), so no
consequence for the blast's free frame and nothing to add to Complexity Tracking.

Not established by this probe: the `Area3D.body_entered` emission point. The probe's
`CharacterBody3D`, teleported into the area by assigning `global_position` from
`_physics_process`, raised NO `body_entered` in 13 steps; the harness (R10) must drive the body
the way V2-C's harness did (a `Player` instance moved with `velocity` + `move_and_slide()`, tasks
T031 of `specs/008`), and the engine source settles the phase: `Main::iteration` runs, per
physics step, `PhysicsServer3D::sync()` (`main.cpp:4990`) → `flush_queries()` (`:4991`, this is
where area/body overlap callbacks and therefore `body_entered` are emitted) →
`SceneTree::physics_process()` (`:5001`, the `_physics_process` callbacks) → `end_sync()`,
`step()` (`:5034-5035`). So `body_entered` precedes every `_physics_process` of the same step,
including the driver's fixed schedule at `i32::MAX` — the door event is consumed in the step it
is emitted, as v2's handler ran it.

## R2 — Inbound queue mechanism

**Decision**: `ecs/queue.rs` owns
`thread_local! { static QUEUE: RefCell<Vec<InboundEvent>> = RefCell::new(Vec::new()); }` with
exactly two public functions: `pub fn push(event: InboundEvent)` (a `borrow_mut().push`) and
`pub fn drain() -> Vec<InboundEvent>` (`std::mem::take` on the `borrow_mut()` — swap, never
clone). Bridges call `crate::ecs::queue::push(...)` and never name `EcsWorld`; the driver calls
`drain()` once at the start of each schedule run. `InboundEvent` holds `Gd<T>` handles inside
its `Register` variant, which is legal in a `thread_local!` (no `Send` bound).

**Why this satisfies FR-006 structurally**: the push path borrows the `RefCell` for the duration
of one `Vec::push` and nothing else — it never touches `EcsWorld` (no `bind`/`bind_mut`), never
sees `&World`. A bridge handler invoked re-entrantly while a schedule is running (e.g. `SyncOut`
calls `AnimationPlayer::play` and the engine emits a signal synchronously) still only pushes; the
event is consumed by the next drain. The one re-entrancy that COULD double-borrow the `RefCell`
is a push happening INSIDE `drain()`'s own `borrow_mut()`; `drain()` holds it for a `mem::take`
and returns before any engine call is made, so nothing can be invoked in that window.

**Alternatives rejected**: (a) a `Vec` field on `EcsWorld` filled through
`get_autoload_by_name::<EcsWorld>().bind_mut()` — that is precisely the double borrow the
constitution names: a signal handler that fires while `EcsWorld::process` is executing (the
driver holds `&mut self`) would panic in `bind_mut`; (b) a `static QUEUE: Mutex<Vec<_>>` — no
second thread ever pushes (every engine callback, including `ready` of a node instanced by
`ResourceLoader::load_threaded_*`'s consumer, runs on the main thread; the threaded loader only
loads the `PackedScene` in a worker, `instantiate`/`add_child`/`ready` happen on the main thread
in `menu.rs`), and a `Mutex` would additionally require `InboundEvent: Send`, which
`Gd<T>` (`RawGd.obj: *mut T`, `raw_gd.rs:32`) is not. The workspace's `experimental-threads`
feature (constitution 1.5.1) changes nothing here: it widens gdext's API surface (e.g. the
`load_threaded_*` bindings, `Callable::from_sync_fn`), it does not move any engine callback off
the main thread.

**Tests** (`queue.rs`, no Godot): `drain_returns_events_in_push_order` (push A, B, C → drain
yields `[A, B, C]`), `drain_empties_the_queue` (second drain is empty), using events that need
no engine handle (`Unregister`, `DoorBodyEntered`, `BlastAnimationFinished` with
`InstanceId::from_i64(n)` — `instance_id.rs` exposes `from_i64`, engine-free).

## R3 — Event delivery inside the World

**Decision**: split by who owns the reaction.

- `Register`/`Unregister` are SYNC-LAYER work (they mutate the two maps and spawn/despawn) and
  are applied directly by the drain step, in FIFO order, before `SyncIn` — no message type.
- `DoorBodyEntered` is a GAMEPLAY event: the drain writes it into
  `Messages<DoorBodyEntered>` (`bevy_ecs::message::Messages`, `src/message/messages.rs:95`,
  `Messages::write` at `:125`), and the door system reads it with `MessageReader<DoorBodyEntered>`
  (`src/message/message_reader.rs:34`, `read()` at `:44`). Naming verified: bevy_ecs 0.19 calls
  them messages (`src/message/`), with `MessageWriter` (`message_writer.rs:62`) for systems that
  emit.
- `BlastAnimationFinished` carries NO decision (unknown id → drop; known id → mark for
  removal), so it is applied directly by the drain step too: `EntityIndex` lookup, then insert
  the `Remove` marker. This keeps the spec's statement that `blast` has no gameplay system
  literally true, and it is a pure function on `&mut World` (testable, R9).

**Exactly-once consumption**: the driver calls `Messages::<DoorBodyEntered>::update()`
(`messages.rs:193` — swaps the two buffers and clears the older one) at the START of every
FIXED schedule run, before the drain. A message written by the drain of run N is read by the
door system of run N (reader cursor advances past it, so a second read in the same or a later
run does not see it again) and is dropped by the `update()` of run N+2 at the latest. A
`DoorBodyEntered` that a FRAME run happened to drain (structurally possible, never in practice:
`body_entered` is a physics-phase signal) sits in the buffer until the next fixed run's door
system reads it — nothing is lost and nothing is read twice. Alternative (b), handing systems a
plain `Vec` resource cleared by the driver, was rejected: it re-implements the cursor by hand and
loses the multi-reader guarantee later milestones will need (hits are read by several systems).

## R4 — Identity maps and handles

**Decision**:

```rust
#[derive(Resource, Default)]
pub struct EntityIndex { by_id: HashMap<InstanceId, Entity> }   // plain resource, Send
pub struct NodeHandles { by_entity: HashMap<Entity, Handles> }  // NonSend resource
pub enum Handles {
    Door  { root: Gd<Area3D>,          anim: Gd<AnimationPlayer> },
    Puff  { root: Gd<CpuParticles3D> },
    Blast { root: Gd<Node3D>, light_rays: Gd<CpuParticles3D>, camera: Option<Gd<Camera3D>> },
}
```

Typed per bridge kind — no `Gd<Node>` plus per-frame `cast`. `InstanceId` derives
`Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash` (`instance_id.rs:20`), so it is a valid
`HashMap` key. `NodeHandles` is inserted with `World::insert_non_send_resource`
(`world/mod.rs:2059`) and read/written by systems through `NonSend`/`NonSendMut`
(`change_detection/params.rs:592`, `:620`); `EntityIndex` with `insert_resource`
(`world/mod.rs:1997`).

- **Validity sweep** (`SyncIn`, both schedules): iterate `NodeHandles.by_entity`, call
  `Handles::root_valid()` — ONE `is_instance_valid()` on the root handle only (`Gd::is_instance_valid`,
  `gd.rs:332` → `InstanceId::lookup_validity`, `instance_id.rs:71-73`, one engine utility call);
  collect dead entities, then despawn each and remove it from BOTH maps (the sweep holds
  `NonSendMut<NodeHandles>`, `ResMut<EntityIndex>` and `Commands`; the map removal is
  immediate, the despawn is a command applied at the end of the run). Children (`anim`,
  `light_rays`) die with their root, so they are never checked separately.
- **Idempotent re-registration**: the drain checks `by_id.contains_key(&id)`; if present, the
  incoming `Register` is dropped (first entity kept, its handles kept). Unit-tested with a fake
  id on a `World` with no engine.
- **Unknown unregistration**: `by_id.remove(&id)` returns `None` → no-op. Unit-tested.
- **Registration with an invalid root** (node died between push and drain):
  `handles.root_valid()` is false → the event is dropped, no spawn. This check is glue
  (needs the engine) and is exercised by the headless harness, not by `cargo test`.

The pure part of the index (`register_if_absent(&mut self, id, spawn: impl FnOnce() -> Entity)
-> Registered | AlreadyRegistered`, `unregister(&mut self, id) -> Option<Entity>`) lives in
`ecs/index.rs` with the tests; the `Handles` enum is declared in `ecs.rs` (it names `Gd` types)
and only ever built by bridges.

## R5 — Timer component vs `SceneTreeTimer`

**Engine semantics** (`scene/main/scene_tree.cpp`, 4.7, `SceneTree::process_timers`):
`time_left -= p_delta; timer->set_time_left(time_left); if (time_left <= 0) { emit timeout;
erase }`, iterating a snapshot ending at `timers.back()` taken before the loop (timers created
during the pass are not stepped until the next pass — R1 fact 5). Called from
`SceneTree::process` AFTER `_process(false)` and the message-queue flush, BEFORE
`_flush_delete_queue()` (R1 fact 3/4).

**Decision**: `Timer { time_left: f64, expired: bool }`, `Timer::new(seconds: f64)`,
`fn step(&mut self, dt: f64) -> bool` doing exactly the engine's arithmetic —
`self.time_left -= dt; if !self.expired && self.time_left <= 0.0 { self.expired = true; return
true } false`. The SUBTRACTIVE form is chosen over "accumulate `elapsed` and compare `>=
duration`" because the two are not bit-identical under IEEE doubles in general; for the two
values this milestone uses they happen to agree (Python check, `dt = 1/60`: 0.2 s expires on
step 13 under both forms; 3.0 s on step 181 under both), but only the subtractive form
reproduces `SceneTreeTimer` for EVERY duration by construction. FR-011's wording ("first step at
which accumulated elapsed ≥ duration") is satisfied: in exact arithmetic the two statements are
the same, and the constitution's parity rule is about the engine's actual behavior. `expired`
guarantees exactly-once (FR-011 "never again unless reset").

**Delta source**: the driver writes `FrameDelta(f64)` in `process(delta)` and `FixedDelta(f64)`
in `physics_process(delta)` (resources, `world.insert_resource`) before running the schedule;
systems read `Res<FrameDelta>`. No system calls `get_process_delta_time` (node.rs:769).

**Why equivalent for both `part_disappear` waits, and what may still differ** (the spec's
timing table): v2's `SceneTreeTimer` fires in `process_timers` of frame N; the `godot::task`
future is woken through `call_deferred` (`async_runtime.rs:630-660`: "Uses a deferred callable
to poll the associated future" — `callable.call_deferred(&[])` at `:657`), and the next
`MessageQueue::flush()` after `process_timers` is in the next iteration's
`SceneTree::physics_process` (after `_process(true)`), so v2's `set_emitting(true)` /
`queue_free()` EXECUTE in iteration N+1's physics phase. v3's tick timer expires inside the
driver's frame schedule of frame N and `SyncOut` writes in frame N. The observer of R10 logs at
the start of frame N+1's process pass and sees the SAME state on both trees (v2's deferred
write precedes it in N+1's physics phase). Raw frame stamps, however, differ: the v2 write
stamps `N+1` (R1 fact 7), the v3 write stamps `N`. Prediction for the spec's table, to be
CONFIRMED by the harness, not assumed: observer-visible delta 0 for both puff transitions;
raw-stamp delta −1 frame on v3 for `emitting` and for the free; timers' first-step frame
identical (v2's second timer is created in N+1's physics phase and first stepped in N+1's
timer pass; v3's `Emitting` timer is created in N's frame schedule and first stepped in N+1's
frame schedule). For `blast`, v2's future is woken during `_process(false)` and flushed by the
`MessageQueue::flush()` that immediately follows `_process(false)` in the same frame, so raw
stamps match v3 (both `N`). For `door`, both play inside the same physics step (R1, last
paragraph).

## R6 — Consumed flags: `PlayOpen`, `StartEmitting`, `Remove`

**Decision**: marker components, inserted with `Commands` by the system that decides
(`commands.entity(e).insert(PlayOpen)`), consumed by the `SyncOut` system that acts
(`Query<(Entity, ...), With<PlayOpen>>` → act → `commands.entity(e).remove::<PlayOpen>()`,
`EntityCommands::remove` at `system/commands/mod.rs:1726`). `Remove` → `queue_free()` on the
root handle, `by_entity.remove`, `by_id.remove`, `commands.entity(e).despawn()`
(`mod.rs:1906`). A marker cannot persist across ticks because the SAME schedule run that acts
on it removes it, and commands are applied at the end of `Schedule::run` — the next run starts
with the marker gone; a bool field would need a second write-back and could be forgotten.
Three markers, not one enum, because `SyncOut` acts on each with a different engine call and
`With<T>` filters are the ECS-native way to select them.

**`queue_free()` only** (FR-010): `Node::queue_free` (`node.rs:1299`) defers the free to
`_flush_delete_queue()`, which runs after the whole process pass (R1 fact 4) — no engine
callback runs inside the schedule. `Gd::free()` (`gd.rs:895`) frees synchronously, which runs
`exit_tree` → the bridge pushes `Unregister` → fine for the queue (R2), but ALSO destroys a node
whose other handles (`anim`, `light_rays`) other systems of the same run may still touch, and
lets gdext's strict safeguards panic mid-schedule. Forbidden in the sync layer.

## R7 — Blast sync pair

**Decision**:

- `SyncIn` (frame schedule), `sync_in_blast`: `Local<HashMap<InstanceId, Option<Vector3>>>`
  cleared at the start of each run; for each `Handles::Blast` entity with `camera: Some(cam)`:
  key = `cam.instance_id()` (no engine call — the id is stored in the `Gd`); if absent, compute
  `cam.is_instance_valid().then(|| cam.get_global_transform().origin)` ONCE and cache it; if the
  value is `Some(origin)`, write it into the entity's `LookTarget(Vector3)` ONLY when different
  (`Mut::set_if_neq`, `change_detection/traits.rs:215`) or insert it if absent. An invalid
  camera yields no write (v2 `blast.rs:42-44`).
- `SyncOut` (frame schedule), `sync_out_blast`: `Query<(Entity, &LookTarget), Changed<LookTarget>>`
  → `light_rays.look_at(target)` on the handle from `NodeHandles`.

**Why the explicit `!=`**: bevy change detection is write-based — `DerefMut` on `Mut<T>` calls
`set_changed` unconditionally (`change_detection/traits.rs:431-470`), so a plain `*target =
origin` every frame would mark `Changed` every frame and the gate would save nothing. With
`set_if_neq` the gate is real: when the camera did not move, no `look_at` is issued. The result
is behavior-identical because nothing else rotates `LightRays` (its only animated property is
`emitting`, `impact_effect.tscn:29`); on insertion `Added` implies `Changed`, so the first frame
always writes.

**FFI count per blast per frame** (explicit engine calls, both trees under the same gdext
safeguards): v2 `blast.rs:42-47` = `is_instance_valid` + `get_global_transform` + `look_at` =
3. v3 = root validity sweep (1, FR-009) + `look_at` (1, when the target changed — every frame
while the camera moves, as in the game) + `camera.is_instance_valid` and
`get_global_transform` shared by all blasts caching the same camera. One lone blast: 4. Two or
more concurrent blasts (the game spawns one per laser hit, and hits overlap for the blast's
animation length): ≤ 3 each. The 4th call for a lone blast IS the FR-009 sweep the spec itself
mandates; FR-025's literal "at most v2's three" and FR-009 cannot both hold for a lone blast,
and the plan keeps FR-009 (safety of every later engine call in the run depends on it).
Recorded in plan.md Complexity Tracking as a spec-internal tension; resolved at plan review
(2026-09-18) by the one-clause spec edit to FR-025 ("excluding the FR-009 root validity sweep").

## R8 — `EcsWorld` class shape and schedules

**Decision**:

```rust
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct EcsWorld {
    base: Base<Node>,
    #[init(val = setup::build_world())]     world: World,
    #[init(val = setup::build_fixed())]     fixed: Schedule,
    #[init(val = setup::build_frame())]     frame: Schedule,
}
```

`ecs/setup.rs` (pure): `build_world()` inserts `EntityIndex`, `NodeHandles` (non-send),
`Messages<DoorBodyEntered>`, `FrameDelta(0.0)`, `FixedDelta(0.0)`; `build_fixed()`/`build_frame()`
create `Schedule::new(label)` (`schedule/schedule.rs:405`) and `configure_sets(
(Phase::SyncIn, Phase::Gameplay, Phase::EngineQuery, Phase::SyncOut).chain())`
(`configure_sets` at `:247`, `chain` in `schedule/config.rs:471`), with
`#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)] enum Phase`. A unit test builds
both and runs them on the empty World (`Schedule::run(&mut self, world)`, `schedule.rs:560`).
`build_world` is pure (a `World` with resources is engine-free); the systems that touch `Gd`
are added to the schedules by glue in `ecs.rs` (`add_engine_systems(&mut fixed, &mut frame)`),
so `setup.rs` stays testable without Godot.

**Driver** (`ecs.rs`, `INode for EcsWorld`):
- `ready`: `set_process_priority(i32::MAX)`, `set_physics_process_priority(i32::MAX)` (R1).
- `physics_process(dt)`: `world.insert_resource(FixedDelta(dt))`; `messages::<DoorBodyEntered>
  ().update()`; `for ev in queue::drain() { apply(&mut world, ev) }`; `fixed.run(&mut world)`.
- `process(dt)`: `world.insert_resource(FrameDelta(dt))`; `for ev in queue::drain() {
  apply(&mut world, ev) }`; `frame.run(&mut world)`.

**Which systems in which schedule** — each placement reproduces the engine phase in which v2
reacted (spec scenario 5, US1):

| Schedule | Set | System | v2 phase it reproduces |
|---|---|---|---|
| fixed | (drain) | `Register`/`Unregister`/`DoorBodyEntered`→message/`BlastAnimationFinished`→`Remove` | — (both schedules drain; whichever runs first after the push) |
| fixed | `SyncIn` | `sweep_dead_nodes` | — (FR-009) |
| fixed | `Gameplay` | `door::system::open_on_player` | `door.rs:72-82` ran inside `body_entered`, a physics-phase signal (R1) |
| fixed | `SyncOut` | `sync_out_door` (`PlayOpen` → `play_ex("doorsimple_opening")`) | same |
| fixed | `SyncOut` | `sync_out_remove` (`Remove` → `queue_free` + despawn) | — (a `Remove` set in a fixed run, e.g. by a future physics event, must not wait a frame) |
| frame | (drain) | same as above | — |
| frame | `SyncIn` | `sweep_dead_nodes`, `sync_in_blast` | `blast.rs:41-48` ran in `process` |
| frame | `Gameplay` | `part_disappear::system::advance` (`Timer`/`DisappearPhase` step) | `SceneTreeTimer`s tick in `SceneTree::process` (R5) |
| frame | `SyncOut` | `sync_out_blast` (`look_at`), `sync_out_puff` (`StartEmitting` → `set_emitting(true)`), `sync_out_remove` | `blast.rs:46`; `part_disappear.rs:34`; `:45` and `blast.rs:37` |

`EngineQuery` is configured in both schedules with no members. `sweep_dead_nodes` and
`sync_out_remove` are the same system functions added to both schedules.

**Autoload**: `res://ecs/ecs_world.tscn` = `[gd_scene format=3]` + `[node name="EcsWorld"
type="EcsWorld"]` (mirrors `menu/settings.tscn`); `project.godot` `[autoload]` gains
`EcsWorld="*res://ecs/ecs_world.tscn"` AFTER `Settings`. Tree order among autoloads is
irrelevant once the priorities are `i32::MAX` (R1 fact 1), and `Settings` is not read by
`EcsWorld`, so the relative order carries no dependency. Nothing in this milestone needs
`get_autoload_by_name::<EcsWorld>` (bridges push to the queue, not to the node); FR-003 is
satisfied vacuously and remains the rule for any later typed access (the harness's scratch
scene resolves it from GDScript by `/root/EcsWorld` only to print its class — scratch code).

## R9 — Module layout under Principle III

**Decision** (mirrors v2's `x.rs` glue + `x/model.rs` pure):

```text
src/ecs.rs                    glue: EcsWorld class + driver; Handles enum; NodeHandles;
                              apply_register (needs handles); sweep_dead_nodes; sync_out_door,
                              sync_out_puff, sync_out_blast, sync_in_blast, sync_out_remove;
                              add_engine_systems
src/ecs/queue.rs              pure: InboundEvent-agnostic thread_local queue, push/drain + tests
src/ecs/event.rs              InboundEvent enum (its Register variant names Handles → declared
                              here but only constructed by bridges); DoorBodyEntered,
                              BlastAnimationFinished (Message)
src/ecs/timer.rs              pure: Timer + tests
src/ecs/index.rs              pure: EntityIndex (register_if_absent/unregister) + tests
src/ecs/apply.rs              pure: apply_non_register(world, event) for Unregister /
                              DoorBodyEntered / BlastAnimationFinished + tests
src/ecs/setup.rs              pure: Phase set, build_world/build_fixed/build_frame + tests
src/ecs/markers.rs            PlayOpen, StartEmitting, Remove; FrameDelta, FixedDelta
src/door.rs                   bridge (ready/exit_tree/_on_door_body_entered)
src/door/system.rs            pure: DoorState, on_body (v2 door.rs:18-23 verbatim),
                              open_on_player system + tests (3 preserved + round-trip)
src/part_disappear.rs         bridge
src/part_disappear/system.rs  pure: DisappearPhase, Lifetime, Transition, advance system + tests
src/blast.rs                  bridge (no system module — sync-only entity, R3/R7)
```

Gameplay systems live BESIDE their bridges (not under `ecs/systems/`) because that is where v2
put each module's pure model, and a reviewer checking "this module's logic is pure" opens one
directory. `ecs/apply.rs` needs `&mut World` — pure in the constitution's sense (no `Gd`, no
engine), and its `Register` counterpart stays in `ecs.rs` because it stores `Gd` handles.

**Unit-test recipe** (every system test): `use bevy_ecs::system::RunSystemOnce;` (trait at
`system/system.rs:353`, `run_system_once` at `:355`); `let mut world = World::new();` insert
resources by hand (`EntityIndex`, `Messages<DoorBodyEntered>`, `FrameDelta`), spawn the entity
with its components, write a message with `world.resource_mut::<Messages<_>>().write(..)`, then
`world.run_system_once(system).unwrap()` and assert on components / markers with
`world.get::<T>(entity)`. No `NonSend` resource is needed by any gameplay system, so tests never
touch `Handles`.

## R10 — Parity harness

**Design** (`contracts/zz_ecs_parity.gd`, one script, `--case=a|b|c`, same file on both trees;
scratch scene `zz_ecs_parity.tscn` = a `Node3D` root with that script; NEVER committed):

- **Observer**: a child `Node` created by the script with `process_priority = -2147483648`
  (`i32::MIN` — first in every process pass, R1 fact 1), logging at the start of its `_process`
  the state left by the previous frame, in FIXED-format lines written to
  `user://zz_ecs_parity_<case>.log` (a `FileAccess` opened once, flushed at quit). Each line:
  `F<frame> <key>=<value>` with `<frame>` = `Engine.get_process_frames()`.
- **RAW lines** (informational, diffed separately): signal/timer stamps as they happen,
  prefixed `RAW F<frame> <event>` — the lines R5 predicts to differ by one frame for the puff.
- **(a) door**: instance `door/door.tscn` under the root; instance `player/player.tscn`
  (`Player` class, needed for `try_cast::<Player>`) at 10 m away on a floor `StaticBody3D`;
  from the harness's `_physics_process`, set `player.velocity` toward the door and call
  `player.move_and_slide()`; observer logs `door_anim=<current_animation>` each frame; RAW line
  on the first frame `current_animation == "doorsimple_opening"`. Then a second door instance
  and a plain `CharacterBody3D` (non-player) driven the same way; observer must never log
  `doorsimple_opening` for it. Backlog #30's mask issue is bypassed by setting the scratch
  player's `collision_layer = 1` in the harness (a harness-only property on the scratch body,
  not a `door.tscn` edit), so the area detects it on both trees identically.
- **(b) part_disappear**: from a `get_tree().create_timer(0.5).timeout` callback (the game's
  path: a `SceneTreeTimer` resumption — `part.rs:196-205`; on v2 that resumption is itself a
  deferred call, and the harness reproduces the TIMER phase, which is what R1/R5 reason about),
  instance `part_disappear.tscn`, add it under the root; observer logs per frame
  `puff_emitting=<bool> mini_emitting=<bool> puff_in_tree=<bool>`; RAW lines on `tree_exited`.
- **(c) blast**: a `Camera3D` made current, moved every frame from the harness's `_process`
  (`position.x += 0.1`) so `look_at` before/after the move is distinguishable; from the
  harness's `_physics_process` at physics frame 5 (as `red_robot.rs:423-425` does inside the
  robot's physics step), instance `impact_effect.tscn`, add under the root, set global
  position; observer logs per frame `rays_basis=<LightRays.global_transform.basis>` (the
  `Basis` printed with `%s`, 6 decimals via `snapped` is NOT used — full precision is what the
  diff must compare) and `blast_in_tree=<bool>`; RAW line on `tree_exited`.
- Seeding: `seed(1)` at `_ready` on both trees (nothing on these three paths calls `randf`,
  but the puff's `lifetime_randomness` is engine-side particle randomness that does not affect
  the logged properties; the seed is there for the rule, not for a known effect).

**Commands** (from the repo root; the `v2` worktree once per milestone):

```sh
git worktree add ../oxide-godot-v2 v2
( cd ../oxide-godot-v2/oxide_godot_core && cargo build )
( cd ../oxide-godot-v2/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . )
cp specs/011-v3-ecs-core-leaves/contracts/zz_ecs_parity.gd oxide-godot/
cp specs/011-v3-ecs-core-leaves/contracts/zz_ecs_parity.gd ../oxide-godot-v2/oxide-godot/
# zz_ecs_parity.tscn (one Node3D root with the script) copied the same way
for c in a b c; do
  ( cd ../oxide-godot-v2/oxide-godot && XDG_DATA_HOME=/tmp/xdg-v2 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=$c )
  ( cd oxide-godot                    && XDG_DATA_HOME=/tmp/xdg-v3 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=$c )
  diff /tmp/xdg-v2/godot/app_userdata/oxide-godot/zz_ecs_parity_$c.log /tmp/xdg-v3/godot/app_userdata/oxide-godot/zz_ecs_parity_$c.log && echo "case $c: IDENTICAL"
  diff <(grep '^RAW' /tmp/xdg-v2/godot/app_userdata/oxide-godot/zz_ecs_parity_$c.log) <(grep '^RAW' /tmp/xdg-v3/godot/app_userdata/oxide-godot/zz_ecs_parity_$c.log) || echo "case $c: RAW stamps differ (record in the timing table)"
done
```

The observer lines are the parity evidence (SC-004); the RAW diff feeds the spec's timing table
(FR-030). The `user://` path under a split `XDG_DATA_HOME` is
`$XDG_DATA_HOME/godot/app_userdata/<project name>/` (the project name is `oxide-godot` in both
trees — the `config/name` line of `project.godot`; confirm at harness time and adjust the
`diff` paths if it differs). Scratch files are removed from both trees after the diff.

## R11 — Commit plan

| # | Commit | Files | Gate + validation |
|---|---|---|---|
| 1 | `ecs: pure core — queue, timer, index, apply, setup (tests)` | `Cargo.toml` (workspace pin `bevy_ecs = { version = "0.19", default-features = false, features = ["std"] }`), `oxide_godot_lib/Cargo.toml` (`bevy_ecs = { workspace = true }`), `src/lib.rs` (`mod ecs;`), `src/ecs/{queue,event,timer,index,apply,setup,markers}.rs`, `src/ecs.rs` (types only, no class yet) | `cargo build && cargo clippy && cargo test` (new tests pass); **this commit measures SC-007**: `cargo tree --prefix none \| sort -u \| wc -l` before (22 on `85186f6`, measured 2026-09-18 — the spec's "14" came from a probe crate, see SC-007) and after, the delta recorded in plan.md's Technical Context |
| 2 | `ecs: EcsWorld autoload + driver (priorities i32::MAX) + sync systems; ecs_world.tscn; project.godot autoload` | `src/ecs.rs`, `oxide-godot/ecs/ecs_world.tscn`, `oxide-godot/project.godot` | gates + headless import (`Initialize godot-rust`) + a scratch scene printing `/root/EcsWorld`'s class + headless `main.tscn`/`level.tscn` clean; R1's experiment output pasted into the commit message body |
| 3 | `door: bridge + open_on_player system (v2 on_body preserved, 3 tests + round-trip)` | `src/door.rs`, `src/door/system.rs`, `docs/v3-tradeoffs.md` (created, header + entry d) | gates + headless; harness case (a) on both trees; no visual checkpoint (orphaned scene) |
| 4 | `part_disappear: bridge + DisappearPhase/Timer system (tests: boundaries, order, no double fire)` | `src/part_disappear.rs`, `src/part_disappear/system.rs`, `docs/v3-tradeoffs.md` (entry a) | gates + headless; harness case (b); **STOP — visual checkpoint (1)** |
| 5 | `blast: bridge + SyncIn/SyncOut pair (look_at gated by real change), animation_finished → Remove` | `src/blast.rs`, `src/ecs.rs` (blast sync systems), `docs/v3-tradeoffs.md` (entries b, c) | gates + headless; harness case (c); **STOP — visual checkpoint (2)** |
| 6 | `CLAUDE.md: Port conventions (v3); spec: measured timing differences table filled; docs/v3-tradeoffs.md complete` | `CLAUDE.md`, `specs/011-.../spec.md`, `docs/v3-tradeoffs.md` | docs-only; the timing table is filled from the three harness diffs of commits 3-5 (an empty table is written explicitly as "no differences measured" with the diff outputs quoted) |

The harness runs after commits 3, 4 and 5; scratch files (`zz_*`) are never committed and are
deleted from both trees before each commit.
