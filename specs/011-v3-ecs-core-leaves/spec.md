# Feature Specification: Milestone V3-A — ECS core and the three leaf effects

**Feature Branch**: `v3` (work directly, no per-milestone branch — the v3 phase branch was created
from `v2` at `e2932b4`; baseline for this milestone is `85186f6`, constitution 1.5.1. Local
commits only, never pushed.)

**Created**: 2026-09-18

**Status**: Draft

**Phase**: v3 — ECS layer over the nodes (constitution 1.5.1, Principles I, II and III including
the "ECS shape (v3)" subsection). Both v3 objectives apply, done together in every user story:
(1) v2's two pillars are KEPT — idiomatic, type-driven Rust and the thin-glue / pure-core
separation of Principle III; (2) the ECS layer sits OVER the nodes — gameplay logic lives in
systems scheduled by one World and one Schedule, and no gameplay node runs per-frame logic of
its own. This milestone additionally FIXES the infrastructure every later v3 milestone
(`player`/`player_input`/`camera_noise_shake` in V3-B, `bullet`/`part`/`red_robot`/
`flying_forklift` after it) reuses verbatim: the single `EcsWorld` autoload that owns the World
and drives both schedules, the inbound event queue that engine callbacks push to without ever
borrowing the World, the `InstanceId`-based node identity with its two sync-layer maps, the
labeled tick phases `SyncIn → Gameplay → EngineQuery → SyncOut`, the bridge template, the timer
component, and the Godot-free way systems are unit-tested.

- **Backlog items closed by this spec**: none.
- **Backlog items explicitly deferred**: #30 (`door.tscn`'s `collision_mask = 7` so the orphaned
  door would detect the player) — deferred by the user at V2-C checkpoint 2 and left untouched
  here: this milestone changes no scene file's collision properties, and the door keeps being an
  asset never instanced by `level.tscn`. It stays open in `docs/v2-backlog.md`; no separate v3
  backlog is created (constitution 1.5.1, Principle I, v3 "Behavioral parity").
- **Residual dynamic access left in the touched modules after this milestone**: none — none of
  `door.rs`, `part_disappear.rs`, `blast.rs` uses `.rpc`, `.call`, `.get`/`.set` by name,
  `has_method` or a `Callable` by name today (confirmed by reading the three files in full), and
  the v3 rewrite introduces none. The one v2-sanctioned residual (`.rpc("name")`) does not
  occur in this milestone's scope.

**Input**: User description: "Milestone V3-A — ECS core and the three leaf effects (`door`,
`part_disappear`, `blast`). Build the ECS infrastructure every later v3 milestone reuses verbatim
(autoload `EcsWorld` owning the `bevy_ecs::World` and the schedules, inbound event queue, sync
maps keyed by `InstanceId`, labeled tick phases, timer components, Godot-free system tests,
`CLAUDE.md` "Port conventions (v3)", `docs/v3-tradeoffs.md`) and prove it end to end with the
three smallest gameplay modules, chosen because together they exercise the three mechanisms the
constitution mandates: an engine SIGNAL turned into an inbound event answered by a system
(`door`), TIMER components replacing v2's `godot::task` awaits (`part_disappear`), and a
per-frame SyncIn/SyncOut pair with a signal-driven despawn (`blast`). Behavioral parity with `v2`
is mandatory (headless, seeded harness on a `v2` worktree and on `v3`, user visual checkpoints);
frame-level shifts caused by tick timers replacing `SceneTreeTimer`s are allowed only when
measured and listed in this spec."

## Context

The three modules are leaves in `docs/port-order.md`'s graph: no Rust module calls a custom
method or reads a custom field of `Door`, `PartDisappear` or `Blast`. All three enter the game as
scene instances, never through typed calls, so no call site outside the three files changes in
this milestone. Confirmed by reading the v2 code and the scenes:

| Module (v2) | Lines | Base | Per-frame logic today | Async / signal today | Who instances it |
|---|---|---|---|---|---|
| `door.rs` | 83 | `Area3D` | none (`impl IArea3D for Door {}` is empty, `door.rs:66-67`) | `_on_door_body_entered(&mut self, body: Gd<Node3D>)` (`door.rs:71-82`), connected by the scene: `door/door.tscn:35` `[connection signal="body_entered" from="." to="." method="_on_door_body_entered"]` | nobody — `door.tscn` is not instanced by `level.tscn` (backlog #30, open) |
| `part_disappear.rs` | 48 | `CpuParticles3D` | none | `ready` (`part_disappear.rs:14-47`): `mini_blasts.set_emitting(true)` immediately, then one `async` block: `SceneTreeTimer(0.2)` → `set_emitting(true)` on itself → `SceneTreeTimer(lifetime * 2.0)` → `queue_free()`, with `is_instance_valid()` checks after each await | `part.rs`, from the `PackedScene` preloaded at `part.rs:116-117` (`part_disappear.tscn`, root node `PartDisappearPuff` `type="PartDisappear"` at line 42, `lifetime = 1.5` at line 46, child `MiniBlasts` at line 59) |
| `blast.rs` | 49 | `Node3D` | `process` (`blast.rs:41-48`): if the cached camera is still valid, `light_rays.look_at(camera.get_global_transform().origin)` every frame — per-frame logic on a node, forbidden on a v3 bridge | `ready` (`blast.rs:18-39`): caches `get_tree().get_root().get_camera_3d()` into `camera: Option<Gd<Camera3D>>` (`blast.rs:13`), then awaits the child `AnimationPlayer`'s `animation_finished` through a fallible future → `queue_free()` | `red_robot.rs:423-425`: `impact_effect_scene.instantiate_as::<Node3D>()`, added under the root, positioned at the laser hit (`impact_effect.tscn`, root `type="Blast"` at line 169; children `AnimationPlayer` with `autoplay = &"blast"` at lines 171-173 and `LightRays` at line 187) |

**Correction to the milestone brief, recorded so it is not carried forward**: `player/bullet/
bullet.tscn` does NOT use the `Blast` Rust class. Its `Blast` node (`bullet.tscn:514`) is a plain
`type="Node3D"` whose children are driven by the bullet's own animation tracks
(`bullet.tscn:145-205`, `Blast/*:emitting`). The only scene binding the `Blast` class is
`impact_effect.tscn:169`, and its only instancing site is `red_robot.rs:423`. Consequently the
bullet's explosion is unaffected by this milestone, and visual checkpoint (2) below verifies the
`Blast` class through the LASER impact; the bullet explosion is kept in the same checkpoint only
as a regression sanity check.

The door's pure decision already exists in v2 as an inline module: `door.rs:9-50`, `mod pure`
with `enum DoorState { Closed, Open }` and `on_body(state, is_player) -> (DoorState, bool)`,
covered by 3 tests (`closed_door_opens_for_a_player`, `closed_door_ignores_a_non_player_body`,
`open_door_does_not_retrigger_for_a_player`, `door.rs:29-48`). The bridge resolves its
`AnimationPlayer` once at `DoorModel2/AnimationPlayer` (`door.rs:60-63`) under the upstream bug
fix comment (`door.gd` pointed at the non-existent `DoorModel/AnimationPlayer`), which MUST be
kept verbatim. `door.rs:73-75` carries the backlog #14 comment explaining why the boundary
typing is an immediate `try_cast::<Player>()` and not a `Gd<Player>` signal parameter; that
reasoning still holds and the `try_cast` stays in the bridge.

None of the three classes has an `#[export]` or `#[var]` today (confirmed: `door.rs:52-64`,
`part_disappear.rs:4-10`, `blast.rs:4-14` declare only `Base` and `OnReady` fields plus the
door's `state` and the blast's `camera`). `part_disappear.tscn:46`'s `lifetime = 1.5` is the
engine's own `CPUParticles3D.lifetime`, read by v2 through `get_lifetime()`
(`part_disappear.rs:35`), not a script property. No `SceneReplicationConfig` names any of the
three (the door and the blast have none; `part_disappear.tscn` has none). Preserved surfaces are
therefore exactly: the node type names `Door`, `PartDisappear`, `Blast` in the three scenes, and
the method name `_on_door_body_entered` with its `Gd<Node3D>` parameter for `door.tscn:35`.

Registration precedent for the autoload: `project.godot:23-25` `[autoload]
Settings="*res://menu/settings.tscn"`, where `menu/settings.tscn` is a one-node scene whose root
is `[node name="Settings" type="Settings"]` — the gdext class registered as an autoload through a
`.tscn`. `EcsWorld` follows the same shape. `lib.rs` registers 16 modules today
(`lib.rs:3-18`, including `hittable`, a trait module); `ecs` becomes the 17th.

Crate facts verified on 2026-09-18 against the local Cargo registry: `bevy_ecs 0.19.1` is the
current stable line (`bevy_ecs-0.20.0-rc.1` also present locally — an `-rc`, forbidden by the
dependency policy); `edition = "2024"`, `rust-version = "1.95.0"` (local rustc 1.98.1); its
`[features]` block declares `default`, `std`, `bevy_reflect`, `async_executor`,
`multi_threaded` among others, so `default-features = false, features = ["std"]` is a valid
selection that excludes the three the constitution forbids; with that selection the dependency
graph grows from 14 to 58 crates (44 added). `bevy_ecs::Component: Send + Sync + 'static`
(`src/component/mod.rs:511`) while gdext's `Gd<T>` is `!Send` (`RawGd.obj: *mut T`, godot-core
0.5.5 `src/obj/raw_gd.rs:32`), which is why node handles cannot be components. Present in
0.19.1: `NonSend`/`NonSendMut` (`src/change_detection/params.rs:592`, `:620`),
`World::run_schedule` (`src/world/mod.rs:3922`), `World::run_system_cached`
(`src/system/system_registry.rs:813`), `Changed<T>` (`src/query/filter.rs:956`),
`Schedule::configure_sets` (`src/schedule/schedule.rs:247`). Present in gdext 0.5.5:
`godot::tools::get_autoload_by_name` (`godot-core-0.5.5/src/tools/autoload.rs:46`, caches the
resolved node after the first lookup), `Gd::instance_id` (`src/obj/gd.rs:301`),
`Gd::is_instance_valid` (`src/obj/gd.rs:332`), `TypedSignal::connect_self`/`connect_other`
(`src/signal/typed_signal.rs:272`, `:316`).

Baseline gates at the start of this milestone: `cargo build` / `cargo clippy` / `cargo test`
all clean, **133 tests** (run on `34d8eb6` on 2026-09-18: `133 passed; 0 failed`). Behavior
baseline for parity is branch `v2` at `e2932b4` (a separate worktree, built and run
independently, with its own `XDG_DATA_HOME`, as every v2 milestone did against `v1`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The ECS core: one autoload owns the World and drives both schedules (Priority: P1)

A developer adds gameplay to the project by writing components and systems, registering a
scene's root node as an entity from its bridge, and never touching the World from an engine
callback. Everything a later milestone needs — the autoload, the event queue, the identity maps,
the phase sets, the timer component, the test recipe, the operating notes in `CLAUDE.md` and the
`docs/v3-tradeoffs.md` register — exists after this story, exercised by the three stories below.

**Why this priority**: it is the deliverable every later v3 milestone copies; the three leaf
modules exist to prove it, not the other way round.

**Independent Test**: `cargo test` passes for the pure ECS submodules with no Godot binary (FIFO
drain, register/unregister rules, timer stepping); headless import prints
`Initialize godot-rust ...` and the running project instantiates the `EcsWorld` autoload
(verified by a scratch scene that resolves it by name and prints its class); with no gameplay
entity registered, both schedules run every tick without error.

**Acceptance Scenarios**:

1. **Given** `project.godot`'s `[autoload]` block with only `Settings`, **When** the milestone
   adds `EcsWorld="*res://ecs/ecs_world.tscn"` (a one-node scene whose root is
   `type="EcsWorld"`, mirroring `menu/settings.tscn`), **Then** the project imports headless
   with the extension loaded, the autoload node exists at `/root/EcsWorld`, and
   `godot::tools::get_autoload_by_name::<EcsWorld>("EcsWorld")` returns it — the ONLY sanctioned
   way any class reaches it (no `get_node_as("/root/EcsWorld")`, no path strings elsewhere).
2. **Given** the `EcsWorld` node, **When** the engine ticks, **Then** its `physics_process` runs
   the fixed-step schedule and its `process` runs the frame schedule, and no other node in the
   project calls `run_schedule` (grep-verifiable). Each schedule declares the four labeled
   system sets `SyncIn → Gameplay → EngineQuery → SyncOut` in that order; `EngineQuery` has no
   member in this milestone but exists so V3-B adds raycast/`move_and_slide` systems without
   restructuring.
3. **Given** an engine callback on any bridge (signal handler, `ready`, `exit_tree`, a `#[func]`
   invoked by an animation track), **When** it pushes a typed event, **Then** the push never
   borrows `EcsWorld` or the World — by construction: the queue is a separate, module-owned
   store that the push path can reach without a `bind()`/`bind_mut()` on `EcsWorld` and without
   a `&mut World`, so a callback fired while a schedule is running (e.g. a signal emitted by an
   engine call made from `SyncOut`) cannot double-borrow. The exact mechanism (e.g. a
   `thread_local!` queue drained at the start of each schedule run) is a plan decision; the
   structural guarantee is the requirement.
4. **Given** events pushed in the order A, B, C during a frame, **When** the next schedule runs,
   **Then** it drains them FIFO (A, B, C) before `SyncIn`, in a unit test with no Godot.
5. **Given** the driver, **When** its callbacks are ordered relative to the other nodes, **Then**
   `EcsWorld`'s `physics_process` and `process` run AFTER every other node's callback of the
   same phase (process priorities set on the autoload so the tree-order default — autoloads
   first — is overridden), and each schedule run begins by draining the queue. Reason: the
   three modules' events are emitted in different phases of the engine frame — `body_entered`
   is flushed by the physics server before the `_physics_process` callbacks of the same step;
   `animation_finished` is emitted from the `AnimationPlayer`'s own idle processing; a node
   instanced by `red_robot.rs`'s physics step reaches `ready` inside that step — and draining at
   the start of BOTH schedules, with the driver last in each phase, consumes each event in the
   same engine phase in which v2 reacted to it directly (v2 played the door animation inside the
   signal handler, freed the blast inside the signal's future, and ran the blast's `look_at` in
   its own `process`). The harness of each story records the observed frame alignment.
6. **Given** a bridge whose `ready` registers `InstanceId` X with child handles, **When** the
   registration is drained, **Then** the sync layer spawns one entity, records X → Entity in a
   plain resource, and Entity → the handles in a `NonSend` resource; a second registration of
   the same X (the engine re-running `ready`, or a scene re-entering the tree before its
   `exit_tree` unregistration was drained) is idempotent — the first entity is kept, no second
   entity is spawned — and an unregistration of an unknown X is a no-op. Both rules are unit
   tests. Bridges never hold an `Entity`.
7. **Given** a registered entity whose node handle is no longer `is_instance_valid()` at
   `SyncIn` (the node was freed by something other than the sync layer, e.g. its parent scene
   was freed), **When** `SyncIn` runs, **Then** the entity is despawned and both maps drop it,
   without any engine call on the dead handle.
8. **Given** an entity the gameplay layer flagged for removal, **When** `SyncOut` runs, **Then**
   the node is released with `queue_free()` only — never `free()`, which would run `exit_tree`
   (and its unregistration push) while the schedule is in progress — and the entity is despawned
   by the sync layer; no gameplay system frees a node.
9. **Given** a timer component with a duration, **When** it is stepped with the frame delta,
   **Then** it reports expiry exactly once, on the first step at which the countdown
   (`time_left -= dt`, the engine's `SceneTreeTimer` arithmetic) reaches zero or below, and never
   again unless reset — unit-tested at the exact boundary
   (a countdown reaching exactly zero expires), one step before it (does not), and across a
   second step after expiry (does not fire twice).
10. **Given** the milestone's closing commit, **When** `CLAUDE.md` is read, **Then** it has a
    "Port conventions (v3)" section (crate pin `bevy_ecs = { version = "0.19",
    default-features = false, features = ["std"] }` in `[workspace.dependencies]`, autoload
    registration through the one-node `.tscn`, the bridge template — `ready` registers,
    `exit_tree` unregisters, handlers only push — the event-push rule, and the harness recipe
    against a `v2` worktree with a separate `XDG_DATA_HOME`), and `docs/v3-tradeoffs.md` exists
    with the table header `| Module | What the engine owns | Why the ECS cannot hide it | How the
    sync layer handles it | Location |` and at least the four entries this milestone produces
    (scene-tree timers → tick timers; `animation_finished` as an engine-owned event; the camera
    position read from the engine each frame for `blast`; scene instancing staying with
    `part.rs`/`level.rs`/`red_robot.rs`).

---

### User Story 2 - `door`: the signal → event → system → SyncOut exemplar (Priority: P2)

The door behaves exactly as in v2 — it opens once, for the first `Player` body that enters, and
ignores every other body and every later entry — but the decision runs in a gameplay system on a
`DoorState` component, the scene's signal handler only reports what happened, and the animation
is started by `SyncOut`.

**Why this priority**: it is the smallest complete round-trip through all four phases and the
template for every signal-driven behavior later (hits, body entries, RPC-derived events).

**Independent Test**: `cargo test` passes for the door system (the three v2 cases preserved plus
the queue round-trip); the parity harness case (a) logs the same animation-start frame on `v2`
and on `v3`, once, for the `Player` body and never for the non-player body. No in-game visual
checkpoint exists for the door (orphaned scene, backlog #30).

**Acceptance Scenarios**:

1. **Given** `door.tscn:35`'s connection to `_on_door_body_entered`, **When** `Door` becomes a
   bridge, **Then** `_on_door_body_entered(&mut self, body: Gd<Node3D>)` keeps its exact name and
   signature, and its whole body is: compute `is_player = body.try_cast::<Player>().is_ok()`
   (the boundary typing stays in the bridge — it is engine glue, and the backlog #14 comment
   explaining why it is a `try_cast` and not a `Gd<Player>` parameter is kept) and push
   `DoorBodyEntered { id: <this node's InstanceId>, is_player }`. It does not read or write
   `DoorState`, does not touch the `AnimationPlayer`, and does not borrow the World.
2. **Given** the door's `ready`, **When** it runs, **Then** it registers `{ InstanceId,
   DoorState::Closed, the AnimationPlayer handle resolved once at "DoorModel2/AnimationPlayer" }`
   (the upstream bug fix comment kept at the resolution site), and `exit_tree` pushes the
   unregistration. `Door` has no `process`/`physics_process` (it has none today; the empty
   `impl IArea3D` gains only `ready`/`exit_tree`).
3. **Given** a registered door entity in `DoorState::Closed` and a drained
   `DoorBodyEntered { is_player: true }`, **When** the door gameplay system runs (pure: the v2
   `on_body` decision, moved into the ECS module as the system's body), **Then** the component
   becomes `DoorState::Open` and the entity is flagged "play open animation this tick"; **and**
   in `SyncOut` the flagged entity's `AnimationPlayer` plays `doorsimple_opening` exactly once
   (the flag is consumed, not persisted).
4. **Given** the same entity and a `DoorBodyEntered { is_player: false }`, or an already-`Open`
   door and any body, **When** the system runs, **Then** the state is unchanged and nothing is
   flagged — the three v2 tests are preserved as system tests (`World::new()`, one entity, one
   event, run the system, assert the component and the flag), and at least one new test covers
   the event round-trip (event pushed by id → resolved to the entity → consumed).
5. **Given** a `DoorBodyEntered` whose `id` is not registered (the door left the tree between
   the push and the drain), **When** the event is consumed, **Then** it is dropped silently.

---

### User Story 3 - `part_disappear`: the timer-component exemplar (Priority: P2)

A destroyed robot part's puff behaves as in v2 — the mini-blasts burst immediately, the main
puff starts emitting 0.2 s later, and the node is gone `lifetime * 2.0` after that — but the
two sequential waits are one explicit phase enum advanced by a generic timer system in the
frame schedule, with no async task.

**Why this priority**: every v2 `godot::task` await in later modules (`bullet`, `part`,
`red_robot`, `flying_forklift`) becomes this timer component; the phase-enum shape is the
template. Lowest visual risk of the three, but the one that can shift timing by a frame, so it
is where the "measured timing differences" rule is exercised first.

**Independent Test**: `cargo test` passes for the timer and phase system (boundary expiry, no
double fire, phase order); the parity harness case (b) logs per frame the node's `emitting`, the
`MiniBlasts` child's `emitting`, and the frame the node leaves the tree, on `v2` and `v3`; visual
checkpoint (1) — debris puffs appear, emit and vanish as in v2.

**Acceptance Scenarios**:

1. **Given** `PartDisappear`'s `ready`, **When** it runs, **Then** it MAY keep the one-shot engine
   setup v2 performs at that same moment (`MiniBlasts.emitting = true`, `part_disappear.rs:15`
   — not per-frame logic, so allowed on a bridge), then registers `{ InstanceId,
   DisappearPhase::WaitingToEmit with a 0.2 s timer, lifetime read once via get_lifetime() }`
   and the node's own handle for `SyncOut`; `exit_tree` unregisters; the class has no
   `process`/`physics_process` and no `godot::task::spawn`.
2. **Given** the registered entity, **When** the frame schedule steps its timer with the frame
   delta and the 0.2 s expire, **Then** the phase becomes `Emitting` with a fresh
   `lifetime * 2.0` timer, and `SyncOut` sets `emitting = true` on the node exactly once (v2
   `part_disappear.rs:34`).
3. **Given** the `Emitting` phase, **When** its timer expires, **Then** the entity is flagged for
   removal and `SyncOut` calls `queue_free()` on the node (v2 `part_disappear.rs:45`); the
   entity is despawned by the sync layer.
4. **Given** the phase system in a unit test with `World::new()`, **When** stepped with a fixed
   delta, **Then** the phases occur in the order `WaitingToEmit → Emitting → (flagged)`, each
   transition fires exactly once at the expected step count for that delta, and stepping past
   the end never re-fires — no double `emitting` write, no double free flag.
5. **Given** the harness's per-frame log on `v2` (two `SceneTreeTimer`s, ticked by the scene
   tree after the `_process` callbacks) and on `v3` (a timer stepped inside `EcsWorld`'s
   `process`), **When** the two logs are diffed, **Then** they are identical, OR every
   differing frame index is explained by a tick-timer-vs-`SceneTreeTimer` alignment difference
   and recorded in this spec's "Measured timing differences" table with the exact frame delta
   before the closing commit (constitution 1.5.1, Principle I, v3 "Behavioral parity").

---

### User Story 4 - `blast`: per-frame SyncIn/SyncOut with a signal-driven despawn (Priority: P3)

The laser impact effect faces the camera every frame and disappears when its animation ends,
exactly as in v2, but the node has no `process`: the camera position is read by `SyncIn`, the
`look_at` is written by `SyncOut`, and the animation's end reaches the ECS as an event that
marks the entity for removal.

**Why this priority**: it is the only one of the three with per-frame node logic to remove, so
it proves the SyncIn/SyncOut pair; it has NO pure gameplay system, which is worth stating as a
legitimate shape (sync-only entity) so later reviews do not demand an artificial one.

**Independent Test**: headless validation of `level.tscn` shows no new errors; the parity
harness case (c) logs per frame the `LightRays` global basis and the frame the node leaves the
tree, identical on `v2` and `v3`; visual checkpoint (2) — laser impacts face the camera and
vanish when their animation ends.

**Acceptance Scenarios**:

1. **Given** `Blast`'s `ready`, **When** it runs, **Then** it resolves the camera ONCE the way v2
   does (`get_tree().get_root().get_camera_3d()`, `blast.rs:19` — cached at registration, the
   spec's decision: v2 semantics are kept, the handle is NOT re-fetched every frame), connects
   the child `AnimationPlayer`'s `animation_finished` through a typed connection
   (`connect_self`/`connect_other` — no `Callable` by name) to a bridge `#[func]` that only
   pushes `BlastAnimationFinished { id }`, and registers `{ InstanceId, the LightRays handle,
   the camera handle (possibly absent) }`; `exit_tree` unregisters; the class has no
   `process`/`physics_process` and no `godot::task::spawn`.
2. **Given** a registered blast whose cached camera handle is valid, **When** `SyncIn` runs each
   frame, **Then** it reads that camera's global origin once and stores it as the entity's look
   target; if the handle is no longer valid, no target is stored and nothing is written that
   frame (v2 `blast.rs:42-44`). When several blasts cache the same camera instance, the plan
   MAY share one transform read among them — an FFI saving that changes no behavior.
3. **Given** entities with a look target, **When** `SyncOut` runs, **Then** it calls
   `light_rays.look_at(target)` for each — an engine-backed `Basis::looking_at`, glue by the
   1.4.1 rule, which is WHY `blast` has no pure gameplay system (recorded in
   `docs/v3-tradeoffs.md`). The plan MAY skip the write when the target did not change
   (`Changed<T>`), since nothing else rotates `LightRays` (its animation tracks touch only
   `emitting`, `impact_effect.tscn:29`); the harness's per-frame basis log proves the result is
   identical either way. Per blast per frame, the engine calls made are at most v2's three
   (validity check, transform read, `look_at`).
4. **Given** a drained `BlastAnimationFinished { id }`, **When** consumed, **Then** the entity is
   flagged for removal and `SyncOut` calls `queue_free()` on the node (v2 `blast.rs:37`); an
   event whose `id` is unknown is dropped. The node leaves the tree at the same frame as on
   `v2`, or the delta is recorded in the "Measured timing differences" table.
5. **Given** `red_robot.rs:423-425` (instantiates `impact_effect.tscn` as `Node3D`, adds it under
   the root, sets its global position), **When** `Blast` becomes a bridge, **Then** that site is
   untouched — it never referenced the `Blast` type — and `red_robot.rs` stays v2 code.

---

### Edge Cases

- **A signal fires while a schedule is running.** `SyncOut`'s `AnimationPlayer.play()` or
  `queue_free()` may cause the engine to emit signals synchronously (e.g. `animation_finished`
  of a zero-length animation, `tree_exiting`). A bridge handler invoked at that moment pushes to
  the queue and returns; because the queue is not inside the World and the push path takes no
  `bind_mut()` on `EcsWorld`, there is no double borrow. The event is consumed by the NEXT
  schedule run, not the current one (FIFO across runs). Scenario 3 of US1 states the
  structural guarantee; this case is the reason it is structural.
- **`exit_tree` unregistration vs. an already-despawned entity.** When `SyncOut` `queue_free()`s
  a node, the engine runs `exit_tree` at the end of the frame, which pushes an unregistration
  for an `InstanceId` the sync layer already dropped — the no-op rule of scenario 6 (US1)
  absorbs it. When a node is freed by something else first, `SyncIn`'s validity sweep despawns
  it and the later unregistration is again a no-op. Both orders are unit-tested with fake
  events.
- **`ready` without a camera** (`get_camera_3d()` returns none — headless scenes without a
  `Camera3D`, or a blast spawned before a camera is current): the blast registers with no
  camera handle, `SyncIn` stores no target, `SyncOut` writes nothing, and the entity still
  despawns on `animation_finished` — v2's `Option<Gd<Camera3D>>` semantics (`blast.rs:13`,
  `:42`).
- **Frame delta vs. timer boundaries.** At `--fixed-fps 60` with a 0.2 s timer, 12 steps of
  `1/60` accumulate to a value that may sit a rounding error either side of 0.2. The timer
  component uses the engine's `delta` as `f64` and expires on the first step at which
  `time_left -= dt` reaches ≤ 0 (the engine's own arithmetic); the harness records the actual expiry frame on both trees and
  the difference, if any, goes to the "Measured timing differences" table rather than being
  argued away.
- **A door body enters twice in one physics step** (two `body_entered` for the same body, or
  two bodies): two events, FIFO; the first `Player` entry flips the state and flags the play,
  the second is a no-op on an `Open` door — same as v2, where both handler calls ran in
  sequence.
- **Registration event drained after the node died** (a puff instanced and freed within the
  same frame by its parent): the sync layer must not spawn an entity for an invalid handle; the
  registration's handles are validity-checked when drained, and an invalid root handle drops
  the registration.

## Requirements *(mandatory)*

### Functional Requirements

**ECS core (US1)**

- **FR-001**: The crate MUST add `bevy_ecs` as its only new dependency, declared in
  `oxide_godot_core/Cargo.toml`'s `[workspace.dependencies]` as
  `bevy_ecs = { version = "0.19", default-features = false, features = ["std"] }` and inherited
  by the member crate with `{ workspace = true }` (the template's dependency convention). No
  other crate is added; `bevy_reflect`, `async_executor` and `multi_threaded` are not enabled.
  (Scenario 1, US1.)
- **FR-002**: A new module `ecs` (glue `ecs.rs` + pure submodules, the Principle III layout)
  MUST provide the class `EcsWorld` (base `Node`), registered as an autoload named `EcsWorld`
  in `project.godot` through a one-node scene `res://ecs/ecs_world.tscn` whose root is
  `type="EcsWorld"`. It owns the `bevy_ecs::World` and the two schedules. (Scenario 1, US1.)
- **FR-003**: Every access to `EcsWorld` from another class MUST go through
  `godot::tools::get_autoload_by_name::<EcsWorld>("EcsWorld")`; no `/root/EcsWorld` path string
  and no `get_node_as` for it anywhere in the crate. (Scenario 1, US1.)
- **FR-004**: `EcsWorld` MUST be the single driver: its `physics_process` runs the fixed-step
  schedule and its `process` runs the frame schedule; no other node calls `run_schedule`. Its
  process and physics-process priorities MUST place its callbacks after every other node's
  callback of the same phase. (Scenarios 2 and 5, US1.)
- **FR-005**: Both schedules MUST declare the labeled system sets `SyncIn`, `Gameplay`,
  `EngineQuery`, `SyncOut`, chained in that order. `EngineQuery` is declared and empty in this
  milestone. (Scenario 2, US1.)
- **FR-006**: Bridges MUST push typed inbound events (registration, unregistration,
  signal-derived events) to a queue that is NOT inside the World and whose push path takes no
  `bind()`/`bind_mut()` on `EcsWorld` and no `&World`/`&mut World`, so that an engine callback
  can never double-borrow — a structural guarantee, not a discipline. Each schedule run MUST
  begin by draining the queue FIFO, before `SyncIn`. (Scenarios 3, 4 and 5, US1.)
- **FR-007**: Bridges MUST identify themselves by `InstanceId` (`self.base().instance_id()`) and
  MUST NOT hold an `Entity`. The sync layer MUST own an `InstanceId → Entity` map (plain
  resource) and an `Entity → node handles` map (a `NonSend` resource, since `Gd<T>` is `!Send`),
  maintained only on register/unregister/despawn and read only by `SyncIn`/`SyncOut` systems
  and bridges. Components MUST hold no `Gd<T>`. (Scenario 6, US1.)
- **FR-008**: A registration event MUST carry the `InstanceId` plus every child handle the
  bridge resolved once in `ready` (door: its `AnimationPlayer`; part_disappear: its own node
  handle; blast: `LightRays` and the camera if any — NOT its `AnimationPlayer`, whose only use
  is the typed connection made in the bridge, so the sync layer never needs that handle).
  Registration of an already-registered `InstanceId` MUST be idempotent (first entity kept, no
  second spawn); unregistration of an unknown `InstanceId` MUST be a no-op; a registration
  whose root handle is invalid when drained MUST be dropped. (Scenario 6, US1; Edge Cases.)
- **FR-009**: `SyncIn` MUST despawn any entity whose root node handle is no longer
  `is_instance_valid()`, dropping it from both maps without calling the engine on the dead
  handle. (Scenario 7, US1.)
- **FR-010**: Node release MUST happen only in `SyncOut`, only through `queue_free()`, and only
  for entities flagged for removal by a system; `free()` is forbidden in the sync layer; no
  gameplay system calls the engine. (Scenario 8, US1.)
- **FR-011**: A generic timer component MUST exist, stepped by a system in the frame schedule
  with the frame delta (`f64`), counting `time_left` down with the engine's own
  `SceneTreeTimer` arithmetic (`time_left -= dt`, fires when `<= 0`), expiring exactly once on the
  first step that reaches zero and never again unless reset. (Scenario 9, US1.)
- **FR-012**: Pure, Godot-free unit tests MUST cover: FIFO drain order; register/unregister
  idempotence and the unknown-id no-op; the two `exit_tree`-after-despawn orders; timer
  boundary expiry, one-step-before non-expiry, and no double fire; and every gameplay system
  of US2-US4 via `World::new()` + `run_system_once`/`run_system_cached`. (Scenarios 4, 6, 9,
  US1; Edge Cases.)
- **FR-013**: `CLAUDE.md` MUST gain a "Port conventions (v3)" section (crate pin, autoload
  registration, bridge template, event-push rule, harness recipe against a `v2` worktree with a
  separate `XDG_DATA_HOME`), and `docs/v3-tradeoffs.md` MUST be created with the table header of
  scenario 10 (US1) and at least these entries, each added in the commit that introduces the
  touch point: (a) `part_disappear`, `blast` — scene-tree timers replaced by tick timers stepped
  in the frame schedule; (b) `blast` — `animation_finished` is an engine-owned event bridged to
  the queue; (c) `blast` — the camera's position must be read from the engine each frame and
  `look_at` is engine-backed, so the entity is sync-only; (d) `part`, `level`, `red_robot` —
  scene instancing stays with the v2 node code, the spawned node's bridge registers the entity.
  (Scenario 10, US1.)

**`door` (US2)**

- **FR-014**: `Door` MUST become a bridge: `ready` registers `{ InstanceId, DoorState::Closed,
  AnimationPlayer handle }` with the `AnimationPlayer` resolved once at
  `DoorModel2/AnimationPlayer` under the existing upstream bug fix comment (kept verbatim);
  `exit_tree` unregisters; no `process`/`physics_process`. (Scenario 2, US2.)
- **FR-015**: `_on_door_body_entered(&mut self, body: Gd<Node3D>)` MUST keep its exact name and
  signature (`door.tscn:35` unedited) and MUST only compute `is_player` via
  `body.try_cast::<Player>().is_ok()` (backlog #14 comment kept) and push
  `DoorBodyEntered { id, is_player }`. (Scenario 1, US2.)
- **FR-016**: The drain MUST resolve `DoorBodyEntered`'s `InstanceId` to its entity (an unknown
  id is dropped silently) and hand the resolved event to the gameplay layer; a gameplay system
  MUST consume it and apply the v2 `on_body` decision (`door.rs:18-23`, moved into the ECS
  module) to the `DoorState` component, flagging the entity to play the open animation when the
  decision says so; the flag is consumed in the same tick (the marker inserted by the gameplay
  system MUST be visible to `SyncOut` of the same schedule run — analyze M3, 2026-09-18).
  (Scenarios 3, 4, 5, US2.)
- **FR-017**: `SyncOut` MUST play `doorsimple_opening` on the flagged entity's `AnimationPlayer`
  exactly once per flag. (Scenario 3, US2.)
- **FR-018**: The three v2 door tests MUST be preserved as system tests (same three cases), and
  at least one new test MUST cover the event round-trip by id. (Scenario 4, US2.)
- **FR-019**: Backlog #30 MUST remain open and deferred; `door.tscn` MUST NOT change (no
  `collision_mask` edit). (Top block.)

**`part_disappear` (US3)**

- **FR-020**: `PartDisappear` MUST become a bridge: `ready` MAY set `MiniBlasts.emitting = true`
  (one-shot setup, as v2 at `part_disappear.rs:15`), then registers `{ InstanceId,
  DisappearPhase::WaitingToEmit(timer 0.2 s), lifetime read once }` and its own handle;
  `exit_tree` unregisters; no `process`/`physics_process`, no `godot::task::spawn`.
  (Scenario 1, US3.)
- **FR-021**: A `DisappearPhase` enum MUST replace the two sequential awaits:
  `WaitingToEmit(timer)` → on expiry `Emitting(timer of lifetime * 2.0)` with `SyncOut` writing
  `emitting = true` once → on expiry the entity is flagged for removal and `SyncOut`
  `queue_free()`s the node. (Scenarios 2, 3, US3.)
- **FR-022**: Unit tests MUST cover the phase order, exact-boundary expiry for both timers at a
  fixed delta, and the absence of double fires. (Scenario 4, US3.)

**`blast` (US4)**

- **FR-023**: `Blast` MUST become a bridge: `ready` resolves the camera once
  (`get_tree().get_root().get_camera_3d()`, v2 semantics — cached at registration, not
  re-fetched per frame), connects the child `AnimationPlayer`'s `animation_finished` through a
  typed connection to a `#[func]` that only pushes `BlastAnimationFinished { id }`, and
  registers `{ InstanceId, LightRays handle, camera handle (optional) }`; `exit_tree`
  unregisters; no `process`/`physics_process`, no `godot::task::spawn`. (Scenario 1, US4.)
- **FR-024**: `SyncIn` MUST, per frame and per blast whose cached camera is valid, read the
  camera's global origin once and store it as the entity's look target; an invalid camera
  yields no target and no write. (Scenario 2, US4.)
- **FR-025**: `SyncOut` MUST call `light_rays.look_at(target)` for each entity with a target
  (engine-backed, glue by the 1.4.1 rule); `Changed<T>` MAY gate the write; per blast per frame
  the engine calls MUST be at most v2's three, EXCLUDING the FR-009 root validity sweep (one
  `is_instance_valid` per registered entity per run, mandated by the spec itself and shared by
  every entity kind — plan review, 2026-09-18). `blast` has NO pure gameplay system, and
  `docs/v3-tradeoffs.md` MUST say why. (Scenario 3, US4.)
- **FR-026**: `BlastAnimationFinished` MUST flag the entity for removal (unknown id dropped),
  and `SyncOut` MUST `queue_free()` the node. (Scenario 4, US4.)
- **FR-027**: `red_robot.rs:423-425`, `part.rs`, `bullet.rs` MUST stay untouched — no call site
  changes type, since the three scenes are instanced through `PackedScene`, never through typed
  calls. (Scenario 5, US4; Context.)

**Cross-cutting (constitution 1.5.1 compliance)**

- **FR-028**: No gameplay bridge (`Door`, `PartDisappear`, `Blast`) MUST have `process` or
  `physics_process`; no engine callback MUST borrow the World; engine access MUST appear only in
  `SyncIn`/`EngineQuery`/`SyncOut` systems and bridges; `settings`, `menu`, `main_scene`,
  `level`, `debug_label` MUST be untouched; `.rpc` is the only sanctioned dynamic access and is
  not used by the three modules. Each of these is a grep-and-review item of SC-006.
- **FR-029**: Parity with `v2` MUST be evidenced per user story: headless import (extension
  loaded, `EcsWorld` instantiated), headless `level.tscn`/`main.tscn` with no new errors, the
  three harness cases (a)-(c) run on a `v2` worktree (`git worktree add ../oxide-godot-v2 v2`,
  own `cargo build`, `--headless --import` first) and on `v3`, with separate `XDG_DATA_HOME`,
  `--fixed-fps 60`, seeded RNG wherever a `randf` is on the path, the actual output files
  diffed (never reported as identical without the diff), and the user visual checkpoints (1)
  and (2). Scratch scenes are named `zz_*` and never committed. Each case MUST reproduce the
  engine phase in which the game itself creates the event, because scenario 5 (US1)'s frame
  alignment depends on it: (a) the `Player` body and the non-player body enter the area from a
  scratch node's `_physics_process` (as physics bodies move in the game); (b) the puff is
  instanced from a `SceneTreeTimer.timeout` callback — the game's own path, `part.rs:180-205`
  (the async wait on `create_timer` whose resumption calls `destroy` → `instantiate` → `ready`),
  i.e. AFTER every `_process` callback of that frame, never from a `_ready`; (c) the blast is
  instanced from a scratch node's `_physics_process` (as `red_robot.rs:423-425` does inside the
  robot's physics step) AND the scene's `Camera3D` moves every frame from a `_process`, so the
  per-frame `LightRays` basis log distinguishes a `look_at` executed before the camera moved
  from one executed after. In every case the observer is a separate scratch node that logs at
  the start of its `_process` with the lowest process priority (`i32::MIN`), i.e. the state at
  the end of the previous frame, same script on both trees.
- **FR-030**: Any frame-level difference between the `v2` and `v3` harness logs MUST be recorded
  in this spec's "Measured timing differences" table (case, first differing frame, delta,
  cause) before the closing commit; an unrecorded difference is a parity failure, not an
  accepted one.

### Key Entities

- **`EcsWorld`**: the autoload node that owns the World and the fixed and frame schedules and
  runs them from its own `physics_process`/`process`, last in each phase; the only node that
  calls `run_schedule`; resolved by name through gdext's typed autoload lookup.
- **Inbound event queue**: a FIFO of typed events (`Register { id, handles, initial components
  }`, `Unregister { id }`, `DoorBodyEntered { id, is_player }`, `BlastAnimationFinished { id }`)
  outside the World, pushed by bridges from engine callbacks without borrowing anything, drained
  at the start of every schedule run.
- **Identity maps**: `InstanceId → Entity` (plain resource) and `Entity → handles` (`NonSend`
  resource holding the `Gd<T>`s a bridge resolved once); the only places engine handles live.
- **Bridge**: the Rust class of a gameplay scene root (`Door`, `PartDisappear`, `Blast`) —
  `ready` registers, `exit_tree` unregisters, handlers push events; no per-frame callbacks, no
  `Entity`, no World access.
- **Timer component**: `time_left` counted down with the engine's subtractive arithmetic,
  stepped by the frame schedule, expiring exactly once.
- **`DoorState`** (`Closed`/`Open`) with a consumed "play open animation" flag;
  **`DisappearPhase`** (`WaitingToEmit(timer)` / `Emitting(timer)`) with a "remove" flag;
  **look target** (`Vector3`) written by `SyncIn`, read by `SyncOut`; **removal flag** consumed
  by `SyncOut`.
- **`docs/v3-tradeoffs.md`**: the register of engine touch points (module, what the engine
  owns, why the ECS cannot hide it, how the sync layer handles it, location), the v3 counterpart
  of `docs/api-gaps.md`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build` / `cargo clippy` (0 warnings) / `cargo test` clean, with at least 15
  new unit tests (queue FIFO, register/unregister rules, the two exit-order cases, timer
  boundaries and no-double-fire, door system ×4, disappear phases ×3, blast removal ×1) and none
  removed — total at or above 148 (133 baseline + 15).
- **SC-002**: `grep -rn 'fn process\|fn physics_process'` over `door.rs`, `part_disappear.rs`,
  `blast.rs` returns nothing; over the whole crate, `Schedule::run`/`run_schedule` appear only in
  `ecs.rs`;
  `godot::task::spawn` no longer appears in the three modules.
- **SC-003**: Headless import prints `Initialize godot-rust ...` and a scratch scene resolving
  `EcsWorld` by name prints its class; headless `level.tscn` and `main.tscn` show zero new
  errors relative to the `CLAUDE.md` baseline after every commit.
- **SC-004**: Harness cases (a) door, (b) part_disappear, (c) blast produce identical output
  files on the `v2` worktree and on `v3`, OR every differing frame is listed in the "Measured
  timing differences" table with its delta and cause — no third outcome.
- **SC-005**: User visual checkpoints pass: (1) a robot shot until it explodes shows debris
  puffs that appear, emit and vanish as in v2; (2) laser impact blasts face the camera and
  disappear when their animation ends (the bullet explosion, unaffected, still looks as in v2).
- **SC-006**: Review checklist of constitution 1.5.1 passes in full: no bridge with per-frame
  callbacks; no engine callback borrowing the World; engine access confined to sync systems and
  bridges; `docs/v3-tradeoffs.md` has the four entries; `settings`/`menu`/`main_scene`/`level`/
  `debug_label` diff-empty against `85186f6`; `part.rs`/`bullet.rs`/`red_robot.rs` diff-empty;
  backlog #30 still open.
- **SC-007**: `Cargo.toml` shows exactly one new dependency line; the real dependency-graph
  growth is measured with `cargo tree` on the crate itself and recorded in the plan (the 14 → 58
  figure in Context came from a probe crate holding `bevy_ecs` alone; `godot` already brings some
  of those crates, so the true delta is at most 44).

## Measured timing differences

Filled during implementation (FR-030), before the closing commit. An empty table at closing
means the three harness diffs were empty; a non-empty table is the only sanctioned form of a
frame-level deviation from `v2` in this milestone.

| Case | Signal / timer | `v2` frame | `v3` frame | Delta | Cause |
|---|---|---|---|---|---|
| (b) `part_disappear` | `puff_tree_exited` RAW stamp (the `lifetime * 2.0` free) | F225 | F224 | −1 frame on `v3` | v2's `godot::task` future resumes via `call_deferred`, flushed in the NEXT iteration's physics phase, so its `queue_free()` runs one iteration after the timer fired; v3's `SyncOut` `queue_free()`s in the frame the tick timer expired. The observer lines (state at frame start) are identical on both trees — `puff_in_tree` false from F225 on both — so the difference is not observable in-game. Measured in commit `932f651`. |

Cases (a) `door` (commit `ac09b56`) and (c) `blast` (commit `3c6deb9`) produced EMPTY diffs, full
and RAW: the door plays at F61 on both trees, the blast leaves the tree at F63 on both with the
`LightRays` basis identical every frame. The puff's 0.2 s `emitting` transition is not stamped by
a RAW line; the observer saw `puff_emitting=true` from F44 on both trees. The harness outputs are
quoted in the three commit messages.

Candidates the harness must settle explicitly, whatever the outcome: the `part_disappear` 0.2 s
expiry frame and the `lifetime * 2.0` free frame (tick timer stepped in `EcsWorld.process` vs a
`SceneTreeTimer` processed by the scene tree after the `_process` callbacks); the `blast` free
frame (`animation_finished` consumed by the frame schedule of the same frame vs v2's immediate
`queue_free` in the signal's future); the `door` animation-start frame (`body_entered` consumed
by the fixed schedule of the same physics step vs v2's immediate `play`).

## Assumptions

- Phase v3 (constitution 1.5.1). Zero behavior deviations from `v2` are sanctioned by this
  spec beyond measured frame-level timing shifts recorded in the table above; no backlog item
  is closed.
- **To verify at research time (listed by the milestone brief)**: (1) `get_autoload_by_name`
  works for a class instantiated from an autoload `.tscn` whose root node type is the gdext
  class — the `Settings` precedent already does exactly this, and `autoload.rs:46-51` resolves
  `/root/{name}` then casts, so the expectation is yes; (2) autoload `_process`/
  `_physics_process` run before the current scene's nodes in tree order, and
  `set_process_priority`/`set_physics_process_priority` on the autoload place its callbacks
  AFTER every scene node's callback of the same phase, including the `AnimationPlayer`'s
  internal processing — this is the premise of FR-004 and scenario 5 (US1), so research MUST
  prove it with a scratch scene that prints the callback order (autoload at the chosen
  priority, a scene node at default priority, an `AnimationPlayer` emitting `animation_finished`)
  rather than assume it; (3) where a
  `SceneTreeTimer`'s `timeout` fires within the engine frame relative to the `_process`
  callbacks and the delete-queue flush, versus a tick timer stepped inside `EcsWorld.process`
  — the harness settles the resulting frame alignment; (4) `bevy_ecs 0.19` with the pinned
  features and no `bevy_reflect` compiles in this edition-2024 crate on rustc 1.98 — its own
  `Cargo.toml` declares `edition = "2024"`, `rust-version = "1.95.0"`, so the expectation is
  yes; a build is the check.
- **Constitution wording corrected by PATCH 1.5.1 (`85186f6`)**: the 1.5.0 dependency policy
  justified excluding `multi_threaded` with "gdext is built without `experimental-threads`",
  but the workspace `Cargo.toml` enables that feature since v1 milestone D (for
  `ResourceLoader::load_threaded_*`). The obligation is unchanged — `bevy_ecs` is built without
  `multi_threaded` because the Godot API is main-thread only and `Gd<T>` stays `!Send` under any
  gdext feature; 1.5.1 fixed the stated reason.
- **Correction to the brief** (Context): `bullet.tscn` does not bind the `Blast` class; only
  `impact_effect.tscn` does. Visual checkpoint (2) is about the laser impact.
- The queue is drained at the start of BOTH schedule runs (fixed and frame), not only the fixed
  one, and the driver runs last in each phase — the spec's decision (scenario 5, US1) so that
  each of the three modules' events is consumed in the same engine phase where v2 reacted to it.
  If the harness shows this still leaves a frame shift for some case, the shift is recorded, not
  hidden by moving the drain.
- Double registration is idempotent (first entity kept) rather than rejected — the spec's
  decision (scenario 6, US1): `ready`/`exit_tree` pairs are engine-driven, and an ECS that
  panics on a doubled engine callback would take the game down for a scene-tree quirk.
- The camera handle for `blast` is cached once at registration and validity-checked each frame
  (v2 semantics), not re-fetched per frame — the spec's decision (scenario 1, US4).
- Exact Rust shapes — the queue's storage (`thread_local!` or otherwise), the component and
  event type names, whether flags are marker components or fields, whether `Changed<T>` gates
  the blast write, and the process-priority values — are plan-time decisions; this spec pins
  the behavior and the constitution's structural rules.
- The harness reuses the v2 milestones' worktree + `XDG_DATA_HOME`-split pattern (V2-A through
  V2-E); the three scratch scenes and their logging scripts are new work for this milestone.
- Out of scope: `player`, `player_input`, `camera_noise_shake`, `bullet`, `part`, `red_robot`,
  `flying_forklift` (later v3 milestones); multiplayer authority components (no replicated
  property among the three modules); any member of the `EngineQuery` set (declared empty);
  backlog #30 (deferred, see top block); any change to the three host `.tscn` files beyond what
  a name-preserving rewrite requires (none is expected).
