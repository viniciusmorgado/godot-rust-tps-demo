# Tasks: Milestone V3-A — ECS core and the three leaf effects

**Input**: Design documents from `/specs/011-v3-ecs-core-leaves/` (spec.md, plan.md, research.md
R1–R11, data-model.md, contracts/ecs-api.md, contracts/zz_ecs_parity.gd,
contracts/zz_order_probe.gd, quickstart.md — all at commit `08bc3bd`).

**Branch**: `v3` — local commits only, **never** `git push`. **Baseline**: `08bc3bd`
(constitution 1.5.1 at `85186f6`, spec `e746e28`, plan `8a7f5d9`, tasks `08bc3bd`), **133 tests**.

**Tests**: requested by the spec (FR-012, FR-018, FR-022; Principle III: pure logic and every
gameplay system MUST be `cargo test`-covered without Godot). Named test cases below are the
MINIMUM data-model.md pins; a task that creates pure code without its named tests is incomplete.

**Organization**: one task group per commit of `plan.md`'s Commit Plan (six commits, in order).
The two user visual checkpoints are 🛑 STOP tasks — confirmed by the user, not the implementer.
Every harness task MUST paste the actual `diff` output in its completion note — never write
"identical" without the diff (constitution parity rule; the V2-D lesson).

## Phase 1: Setup

- [x] T001 Add the `v2` worktree and prepare it for the harness (research.md R10):
  `git worktree add ../oxide-godot-v2 v2`, then `cd ../oxide-godot-v2/oxide_godot_core && cargo
  build`, then `cd ../oxide-godot-v2/oxide-godot && /usr/bin/godot.x86_64 --headless --import
  --path .`. Verification: the worktree builds clean and its import prints
  `Initialize godot-rust (...)`; `git worktree list` shows `../oxide-godot-v2` at `e2932b4`.
- [x] T002 On `v3` at `08bc3bd`, confirm the baseline gate: from `oxide_godot_core/`,
  `cargo build && cargo clippy && cargo test`, then record the SC-007 "before" number:
  `cargo tree --prefix none | sort -u | wc -l`. Verification: zero warnings, **133 tests pass**,
  the count is **22** (plan.md Technical Context) — paste the number in the completion note.

**Checkpoint**: baseline confirmed on both trees; US1 work can start.

---

## Phase 2: User Story 1 — the ECS core (Priority: P1) 🎯 MVP

**Goal**: FR-001…FR-013. **Independent Test**: per spec.md US1 — `cargo test` green for the pure
ECS submodules with no Godot; headless import loads the extension and instantiates the
`EcsWorld` autoload; both schedules run every tick with no entity registered.

### Commit 1 — pure core (`ecs/*.rs`, types only in `ecs.rs`)

- [x] T003 [US1] Pin the dependency (FR-001): in `oxide_godot_core/Cargo.toml` add
  `bevy_ecs = { version = "0.19", default-features = false, features = ["std"] }` to
  `[workspace.dependencies]` (after `godot`); in `oxide_godot_core/oxide_godot_lib/Cargo.toml`
  add `bevy_ecs = { workspace = true }` under `[dependencies]`; in
  `oxide_godot_core/oxide_godot_lib/src/lib.rs` add `mod ecs;` (keep the existing 16 `mod`
  lines). Create `oxide_godot_core/oxide_godot_lib/src/ecs.rs` containing for now only the
  submodule declarations (`pub mod queue; pub mod event; pub mod timer; pub mod index; pub mod
  apply; pub mod setup; pub mod markers;`) and the two glue types from data-model.md "Handles":
  `pub enum Handles { Door { root: Gd<Area3D>, anim: Gd<AnimationPlayer> }, Puff { root:
  Gd<CpuParticles3D> }, Blast { root: Gd<Node3D>, light_rays: Gd<CpuParticles3D>, camera:
  Option<Gd<Camera3D>> } }` with `pub fn root_valid(&self) -> bool` (ONE `is_instance_valid()`
  on the root handle of whichever variant), and `#[derive(Default)] pub struct NodeHandles {
  pub by_entity: HashMap<Entity, Handles> }` (the `NonSend` resource; no `Resource` derive). No
  class yet. Verification: `cargo build` resolves `bevy_ecs 0.19.1` (check `cargo tree | grep
  bevy_ecs`); then record the SC-007 "after" number with `cargo tree --prefix none | sort -u |
  wc -l` and write both numbers (22 → N) into plan.md's Technical Context "Crate-graph delta"
  sentence.
- [x] T004 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/event.rs` (data-model.md
  "Events"): `pub enum InboundEvent { Register { id: InstanceId, handles: Handles, initial:
  Initial }, Unregister { id: InstanceId }, DoorBodyEntered { id: InstanceId, is_player: bool },
  BlastAnimationFinished { id: InstanceId } }`; `pub enum Initial { Door, Puff { lifetime: f32 },
  Blast }` (`#[derive(Clone, Copy, Debug, PartialEq)]`); and the message
  `#[derive(Message, Clone, Copy)] pub struct DoorBodyEntered { pub entity: Entity, pub is_player:
  bool }` (`bevy_ecs::message::Message`, the drain writes it AFTER resolving `id → Entity`).
  `Handles` is imported from `crate::ecs`. Verification: compiles; `InboundEvent` has no
  `Send`/`Sync` bound anywhere (it holds `Gd<T>`).
- [x] T005 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/queue.rs` (research.md R2):
  `thread_local! { static QUEUE: RefCell<Vec<InboundEvent>> = RefCell::new(Vec::new()); }`,
  `pub fn push(event: InboundEvent)` (`QUEUE.with(|q| q.borrow_mut().push(event))`), `pub fn
  drain() -> Vec<InboundEvent>` (`QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))` — swap,
  never clone). Nothing in this file names `EcsWorld` or `World`. Tests in `#[cfg(test)] mod
  tests` using engine-free events (`InboundEvent::Unregister { id: InstanceId::from_i64(n) }`,
  `DoorBodyEntered`, `BlastAnimationFinished`): `drain_returns_events_in_push_order` (push A, B,
  C → drain yields `[A, B, C]`, matched by variant/id), `drain_empties_the_queue` (a second
  `drain()` returns an empty `Vec`). Verification: both tests pass; note that `thread_local!`
  state is per test thread, so each test pushes and drains within itself.
- [x] T006 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/timer.rs` (research.md R5,
  data-model.md "Timer"): `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Timer { time_left:
  f64, expired: bool }`, `pub fn new(seconds: f64) -> Self` (`time_left = seconds, expired =
  false`), `pub fn step(&mut self, dt: f64) -> bool` doing EXACTLY the engine's arithmetic —
  `self.time_left -= dt; if !self.expired && self.time_left <= 0.0 { self.expired = true; true }
  else { false }` (reproduces `SceneTree::process_timers`: `time_left -= p_delta; if (time_left
  <= 0)`; NOT an accumulate-and-compare form). Tests: `expires_on_the_step_that_reaches_zero`
  (`Timer::new(0.2)` stepped with `1.0 / 60.0` returns `false` on steps 1–12 and `true` on step
  13 — the residual after 12 subtractions is `+4.86e-17`, R1 fact 6), `does_not_expire_one_step_before`
  (after 12 steps `step` has returned `false` every time), `fires_exactly_once` (step 14 returns
  `false`), `three_seconds_at_sixty_hz_fires_on_step_181`. Verification: all four pass.
- [x] T007 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/index.rs` (research.md R4,
  data-model.md "Index"): `#[derive(Resource, Default)] pub struct EntityIndex { by_id:
  HashMap<InstanceId, Entity> }`, `pub enum Registration { Registered(Entity),
  AlreadyRegistered(Entity) }`, `pub fn register_if_absent(&mut self, id: InstanceId, spawn:
  impl FnOnce() -> Entity) -> Registration` (`by_id.contains_key(&id)` → `AlreadyRegistered`
  with the stored entity, `spawn` NOT called; else call `spawn`, insert, `Registered`), `pub fn
  unregister(&mut self, id: InstanceId) -> Option<Entity>` (`by_id.remove(&id)`), `pub fn
  entity(&self, id: InstanceId) -> Option<Entity>`, `pub fn remove_entity(&mut self, entity:
  Entity)` (drops every key mapping to it). Tests on a `World::new()` with fake ids
  `InstanceId::from_i64(1..)`: `register_spawns_once` (spawn closure called once, world has one
  entity), `second_register_of_same_id_keeps_first_entity` (returns `AlreadyRegistered(e1)`, the
  closure is not called, world still has one entity), `unregister_unknown_id_is_noop` (returns
  `None`, map unchanged), `unregister_then_late_sweep_is_noop` (unregister → `remove_entity` of
  the same entity → no panic, map empty), `sweep_then_late_unregister_is_noop` (`remove_entity`
  → `unregister` returns `None`). Verification: all five pass.
- [x] T008 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/markers.rs` (data-model.md
  "Components and markers"): `#[derive(Resource, Clone, Copy)] pub struct FrameDelta(pub f64);`,
  `#[derive(Resource, Clone, Copy)] pub struct FixedDelta(pub f64);`, `#[derive(Component)] pub
  struct PlayOpen;`, `#[derive(Component)] pub struct StartEmitting;`, `#[derive(Component)] pub
  struct Remove;`, `#[derive(Component)] pub struct BlastTag;`, `#[derive(Component, Clone, Copy,
  PartialEq)] pub struct LookTarget(pub Vector3);` (`godot::builtin::Vector3` — an FFI-free
  value type allowed in pure code by Principle III). Verification: compiles; no `Gd` anywhere in
  the file.
- [x] T009 [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/apply.rs` (research.md R3):
  `pub fn apply_non_register(world: &mut World, event: InboundEvent)` handling three applied
  variants plus a guarded `Register` arm — `Unregister { id }`: `EntityIndex::unregister(id)`; on `Some(e)` remove `e` from
  `NodeHandles.by_entity` (via `world.get_non_send_resource_mut::<NodeHandles>()` — the
  `Option`-returning accessor, so the step is skipped when the resource is absent and tests need
  not insert it; the non-`get_` form panics on a missing resource) and `world.despawn(e)`; `DoorBodyEntered { id,
  is_player }`: `EntityIndex::entity(id)` → `Some(e)` → `world.resource_mut::<Messages<
  DoorBodyEntered>>().write(DoorBodyEntered { entity: e, is_player })`, `None` → drop silently;
  `BlastAnimationFinished { id }`: `entity(id)` → `Some(e)` → `world.entity_mut(e).insert(Remove)`,
  `None` → drop. `Register` is NOT handled here: an explicit arm
  `InboundEvent::Register { .. } => debug_assert!(false, "Register is routed to apply_register by
  the driver")` (a no-op in release, a loud failure in tests and debug builds).
  Depends on T004, T007, T008. Tests (`World::new()` + `EntityIndex` + `Messages` inserted by
  hand): `blast_animation_finished_marks_remove` (registered fake id → entity has `Remove`),
  `blast_animation_finished_for_unknown_id_is_dropped` (no entity gains `Remove`, no panic),
  `unregister_despawns_and_drops_index_entry` (extra: world entity count 0, `entity(id)` is
  `None`). The door round-trip tests (`event_for_registered_id_reaches_its_entity`,
  `event_for_unknown_id_is_dropped`) live in `door/system.rs` (T017), NOT here, because they
  assert on the door system's outcome. Verification: three tests pass.
- [x] T010 [US1] Create `oxide_godot_core/oxide_godot_lib/src/ecs/setup.rs` (research.md R8,
  data-model.md "Schedules and sets"): `#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq,
  Hash)] pub enum Phase { SyncIn, Gameplay, EngineQuery, SyncOut }`; `#[derive(ScheduleLabel,
  Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct Fixed;` and `pub struct Frame;`; `pub fn
  build_world() -> World` (inserts `EntityIndex::default()`, `NodeHandles::default()` via
  `insert_non_send_resource`, `Messages::<DoorBodyEntered>::default()`, `FrameDelta(0.0)`,
  `FixedDelta(0.0)`); `pub fn build_fixed() -> Schedule` and `pub fn build_frame() -> Schedule`
  (`Schedule::new(Fixed)`/`Schedule::new(Frame)` then `configure_sets((Phase::SyncIn,
  Phase::Gameplay, Phase::EngineQuery, Phase::SyncOut).chain())`; keep bevy's
  `ScheduleBuildSettings::auto_insert_apply_deferred` at its default `true` and say so in a
  comment — it is what makes a marker inserted through `Commands` in `Gameplay` visible to
  `SyncOut` of the SAME run (research.md R6; `schedule/schedule.rs:1605`, `:1629`); no systems
  added yet — the gameplay systems join in T018/T024 and the engine systems in
  `ecs.rs::add_engine_systems`, T012). Tests: `both_schedules_run_on_an_empty_world`
  (`build_fixed().run(&mut build_world())` and the frame twin, no panic),
  `phase_sets_are_chained_in_order` (one probe system per set pushing its `Phase` into a
  `Vec<Phase>` resource, run, assert `[SyncIn, Gameplay, EngineQuery, SyncOut]`), and
  `marker_inserted_in_gameplay_is_visible_in_sync_out_of_the_same_run` (a `Gameplay` probe does
  `commands.spawn(ProbeMarker)`, a `SyncOut` probe counts `Query<(), With<ProbeMarker>>` into a
  resource; after ONE run the count is 1). Verification: all three pass.
- [x] T011 [US1] Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`.
  Verification: zero warnings; test count = 133 + 2 (queue) + 4 (timer) + 5 (index) + 3 (apply)
  + 3 (setup) = **150**, paste the `test result:` line. Commit (plan.md R11 row 1):
  `ecs: pure core — queue, timer, index, apply, setup (tests); bevy_ecs 0.19 pinned`.

### Commit 2 — `EcsWorld` autoload + driver + sync systems

- [x] T012 [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add the class and the driver
  (research.md R8): `#[derive(GodotClass)] #[class(base=Node)] pub struct EcsWorld { base:
  Base<Node>, world: World, fixed: Schedule, frame: Schedule }` — NO `#[class(init)]`: the two
  schedules must be built and then passed together to `add_engine_systems`, which two independent
  `#[init(val = ...)]` expressions cannot do, so the constructor is written by hand in the trait
  impl: `fn init(base: Base<Node>) -> Self { let mut fixed = setup::build_fixed(); let mut frame
  = setup::build_frame(); add_engine_systems(&mut fixed, &mut frame); Self { base, world:
  setup::build_world(), fixed, frame } }` (plan review, 2026-09-18);
  `#[godot_api] impl INode for EcsWorld`: `init` as above; `ready` → `self.base_mut().set_process_priority(
  i32::MAX); self.base_mut().set_physics_process_priority(i32::MAX);` (R1 — driver last in both
  phases); `physics_process(&mut self, delta: f64)` → `self.world.insert_resource(FixedDelta(
  delta))`; `self.world.resource_mut::<Messages<DoorBodyEntered>>().update()`; `for ev in
  queue::drain() { match ev { InboundEvent::Register { id, handles, initial } =>
  apply_register(&mut self.world, id, handles, initial), other => apply::apply_non_register(&mut
  self.world, other) } }`; `self.fixed.run(&mut self.world)`; `process(&mut self, delta: f64)` →
  same with `FrameDelta`, no `Messages::update`, `self.frame.run(...)`. `#[godot_api] impl
  EcsWorld {}` stays empty (no `#[func]` needed). Verification: `run_schedule`/`.run(&mut` appear
  only in this file (SC-002).
- [x] T013 [US1] In the same `ecs.rs` add `fn apply_register(world: &mut World, id: InstanceId,
  handles: Handles, initial: Initial)` (research.md R4, FR-008): if `!handles.root_valid()` →
  return (registration dropped, Edge Cases); `match world.resource_mut::<EntityIndex>()
  .register_if_absent(id, || world.spawn_empty().id())` — on `AlreadyRegistered(_)` return
  WITHOUT touching handles (idempotent, first entity kept). The closure needs `&mut World` while
  the index is borrowed from the same `World`: use `world.resource_scope::<EntityIndex, _>(|world,
  mut index| index.register_if_absent(id, || world.spawn_empty().id()))` — `resource_scope`
  temporarily removes the resource and hands both borrows to the closure; do NOT "spawn first and
  despawn on `AlreadyRegistered`" (a wasted spawn per doubled `ready`). On `Registered(e)`: insert the `Initial`-derived components —
  `Initial::Door` → `DoorState::Closed` (T016's type; until T016 lands, insert nothing for `Door`
  and leave a `// filled in commit 3` comment), `Initial::Puff { lifetime }` →
  `DisappearPhase::start()` + `Lifetime(lifetime)` (T023; same placeholder rule),
  `Initial::Blast` → `BlastTag`; then `world.non_send_resource_mut::<NodeHandles>().by_entity
  .insert(e, handles)`. Verification: compiles; the function is the ONLY place that inserts into
  `NodeHandles.by_entity`.
- [x] T014 [US1] In the same `ecs.rs` add the two engine systems shared by both schedules
  (research.md R4/R6): `fn sweep_dead_nodes(mut handles: NonSendMut<NodeHandles>, mut index:
  ResMut<EntityIndex>, mut commands: Commands)` — collect every `(entity, handles)` whose
  `root_valid()` is `false`, remove each from `by_entity` and `index.remove_entity(e)`, then
  `commands.entity(e).despawn()`; NO other engine call on a dead handle (FR-009).
  `fn sync_out_remove(query: Query<Entity, With<Remove>>, mut handles: NonSendMut<NodeHandles>,
  mut index: ResMut<EntityIndex>, mut commands: Commands)` — for each entity: take its `Handles`
  out of `by_entity`, call `queue_free()` on the ROOT handle (`Node::queue_free`, never
  `Gd::free`, FR-010), `index.remove_entity(e)`, `commands.entity(e).despawn()`. Add `pub fn
  add_engine_systems(fixed: &mut Schedule, frame: &mut Schedule)` registering
  `sweep_dead_nodes.in_set(Phase::SyncIn)` and `sync_out_remove.in_set(Phase::SyncOut)` in BOTH
  schedules (`sync_out_door`, `sync_out_puff`, `sync_in_blast`, `sync_out_blast` are added by
  T018, T024, T030). Verification: `grep -n 'free()' ecs.rs` shows only `queue_free()`.
- [x] T015 [US1] Register the autoload (FR-002): create `oxide-godot/oxide-godot/ecs/ecs_world.tscn`
  with exactly `[gd_scene format=3]` + blank line + `[node name="EcsWorld" type="EcsWorld"]`
  (mirror of `menu/settings.tscn`); in `oxide-godot/oxide-godot/project.godot` add
  `EcsWorld="*res://ecs/ecs_world.tscn"` on the line after `Settings="*res://menu/settings.tscn"`
  in `[autoload]`. Gates: `cargo build && cargo clippy && cargo test` (150 tests still). Headless
  (from `oxide-godot/oxide-godot/`): `/usr/bin/godot.x86_64 --headless --import --path .`
  (expect `Initialize godot-rust (...)`, no new error beyond `CLAUDE.md`'s catalog); a scratch
  `zz_autoload_check.tscn` (root `Node` with an inline script:
  `func _ready(): print(get_node("/root/EcsWorld").get_class()); get_tree().quit()`) run with
  `--headless --path . zz_autoload_check.tscn` prints `EcsWorld`; `--headless --path . main.tscn`
  and `level.tscn` show no new errors. Re-run research.md R1's probe on THIS build
  (`contracts/zz_order_probe.gd` split into its three files, autoload `ZzOrder` added
  temporarily to `project.godot`, `ZZ_PRIO=2147483647 ... --fixed-fps 60 --quit-after 16
  zz_order.tscn`) and confirm the order `scene _process → anim_finished → autoload _process →
  timer timeouts`. Delete `zz_autoload_check.*`, `zz_order*` and their `.uid` files and revert
  the `ZzOrder` autoload line before committing; `git status` must show only `ecs.rs`,
  `ecs/ecs_world.tscn`, `project.godot`. Commit (R11 row 2) with the probe's order output pasted
  into the message body: `ecs: EcsWorld autoload + driver (priorities i32::MAX) + sync systems;
  ecs_world.tscn; project.godot autoload`.

**Checkpoint**: User Story 1 complete — the core runs headless with no entity; every later
story only adds bridges and systems.

---

## Phase 3: User Story 2 — `door`: signal → message → system → SyncOut (Priority: P2)

**Goal**: FR-014…FR-019. **Independent Test**: per spec.md US2 — the three v2 door tests
preserved as system tests plus the round-trip; harness case (a) logs the same animation-start
frame on both trees, once, for the `Player` body and never for the non-player body.

### Commit 3 — `door/system.rs` + `door.rs` bridge + `sync_out_door` + `docs/v3-tradeoffs.md`

- [ ] T016 [P] [US2] Create `oxide_godot_core/oxide_godot_lib/src/door/system.rs` and declare
  `mod system;` in `door.rs` (data-model.md "Components and markers"): move `#[derive(Component,
  Clone, Copy, Debug, PartialEq)] pub enum DoorState { Closed, Open }` and `pub fn on_body(state:
  DoorState, is_player: bool) -> (DoorState, bool)` VERBATIM from v2 `door.rs:10-23` (same `match
  (state, is_player)` body, same doc comment citing v1 `door.rs:26-29`); add the gameplay system
  `pub fn open_on_player(mut reader: MessageReader<DoorBodyEntered>, mut doors: Query<&mut
  DoorState>, mut commands: Commands)` — for each message, `doors.get_mut(msg.entity)` (`Err` →
  skip: the entity was despawned after the drain), `let (next, play) = on_body(*state,
  msg.is_player); *state = next; if play { commands.entity(msg.entity).insert(PlayOpen) }`. No
  `Gd`, no `NonSend`. Verification: compiles; `door/system.rs` imports nothing from
  `godot::classes`.
- [ ] T017 [US2] Add `#[cfg(test)] mod tests` to `door/system.rs` (FR-018; recipe research.md
  R9: `use bevy_ecs::system::RunSystemOnce;`, `World::new()`, insert `EntityIndex` and
  `Messages::<DoorBodyEntered>::default()`, spawn one entity with `DoorState`, write one message,
  `world.run_system_once(open_on_player).unwrap()`, assert `world.get::<DoorState>(e)` and
  `world.get::<PlayOpen>(e).is_some()`): `closed_door_opens_for_a_player` (`Closed` + `is_player:
  true` → `Open`, `PlayOpen` present), `closed_door_ignores_a_non_player_body` (`Closed` + `false`
  → `Closed`, no `PlayOpen`), `open_door_does_not_retrigger_for_a_player` (`Open` + `true` →
  `Open`, no `PlayOpen`) — the three v2 cases of `door.rs:29-48`; plus the round-trip through
  the drain: `event_for_registered_id_reaches_its_entity` (register a fake id in `EntityIndex`
  for the spawned entity, call `apply::apply_non_register(&mut world, InboundEvent::DoorBodyEntered
  { id, is_player: true })`, run the system, assert `Open` + `PlayOpen`) and
  `event_for_unknown_id_is_dropped` (unregistered id → no message written, door stays `Closed`).
  Depends on T016. Verification: five tests pass.
- [ ] T018 [US2] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add `fn sync_out_door(query:
  Query<Entity, With<PlayOpen>>, mut handles: NonSendMut<NodeHandles>, mut commands: Commands)`
  — for each entity, `if let Some(Handles::Door { anim, .. }) = handles.by_entity.get_mut(&e) {
  anim.play_ex().name("doorsimple_opening").done(); }` (v2 `door.rs:80`, exactly once per flag),
  then `commands.entity(e).remove::<PlayOpen>()` (FR-017); register it in `add_engine_systems`
  as `sync_out_door.in_set(Phase::SyncOut)` in the FIXED schedule only, ordered before
  `sync_out_remove` (`.before(sync_out_remove)` — a door is never removed in this milestone, but
  the order is the general rule: act, then release). In `ecs/setup.rs::build_fixed` add
  `crate::door::system::open_on_player.in_set(Phase::Gameplay)`. In `apply_register` replace the
  `Initial::Door` placeholder with `world.entity_mut(e).insert(DoorState::Closed)`. Depends on
  T016. Verification: `phase_sets_are_chained_in_order` and the T017 tests still pass.
- [ ] T019 [US2] Rewrite `oxide_godot_core/oxide_godot_lib/src/door.rs` as a bridge (data-model.md
  "Bridges", FR-014/FR-015): delete `mod pure`, the `state` field and the `play_ex` call; keep
  `#[derive(GodotClass)] #[class(init, base=Area3D)] pub struct Door { base: Base<Area3D>,
  #[init(node = "DoorModel2/AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>> }`
  with the two-line upstream bug fix comment above the field kept VERBATIM (`door.rs:60-61`);
  `#[godot_api] impl IArea3D for Door { fn ready(&mut self) { let id = self.base().instance_id();
  crate::ecs::queue::push(InboundEvent::Register { id, handles: Handles::Door { root:
  self.to_gd().upcast::<Area3D>(), anim: self.animation_player.clone() }, initial: Initial::Door }); } fn
  exit_tree(&mut self) { crate::ecs::queue::push(InboundEvent::Unregister { id:
  self.base().instance_id() }); } }` — NO `process`/`physics_process`; `#[godot_api] impl Door {
  #[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>) { /* backlog #14 comment kept
  verbatim from door.rs:73-75 */ let is_player = body.try_cast::<Player>().is_ok();
  crate::ecs::queue::push(InboundEvent::DoorBodyEntered { id: self.base().instance_id(),
  is_player }); } }` — same name and signature as today (`door.tscn:35` unedited, FR-019: no
  `door.tscn` change at all). Verification: `grep -n 'fn process\|fn physics_process\|play_ex\|
  DoorState' door.rs` returns nothing; `grep -c 'upstream bug fix' door.rs` = 1;
  `grep -c 'Backlog #14' door.rs` = 1.
- [ ] T020 [US2] Create `docs/v3-tradeoffs.md` with the header block from
  `contracts/ecs-api.md` §3 (title, the two-sentence intro citing constitution 1.5.1, the
  five-column table header) and row (d): `part`, `level`, `red_robot` — the engine owns scene
  instancing (`PackedScene::instantiate` + `add_child`), the ECS cannot hide it because the node
  must exist before it can be viewed, the sync layer handles it by having the spawned node's
  bridge push `Register` from `ready` (locations: `part.rs:116-117`/`:196-205`,
  `red_robot.rs:423-425`, `level.rs`'s spawner). Verification: file has the header + 1 row.
- [ ] T021 [US2] Gates + headless: `cargo build && cargo clippy && cargo test` (150 − 3 + 5 = **152** — the three v2 door tests leave `door.rs`, five land in `door/system.rs`
  tests; the three v2 door tests are gone from `door.rs` and present in `door/system.rs` — none
  lost); `--headless --import`, `main.tscn`, `level.tscn` clean. Harness case (a) (research.md
  R10, quickstart.md §3): write `oxide-godot/oxide-godot/zz_ecs_parity.gd` from
  `contracts/zz_ecs_parity.gd`, `zz_ecs_observer.gd` (the 2-line script quoted in the contract's
  `_ready`), and `zz_ecs_parity.tscn` — exactly the six lines in contracts/ecs-api.md §4
  (`[gd_scene load_steps=2 format=3]`, the `ext_resource` Script line, the `Node3D` root with
  `script = ExtResource("1")`); copy the same three
  files into `../oxide-godot-v2/oxide-godot/`; run `--headless --path . --fixed-fps 60
  --quit-after 400 zz_ecs_parity.tscn -- --case=a` on both trees with `XDG_DATA_HOME=/tmp/xdg-v2`
  and `/tmp/xdg-v3`; `diff` the two `zz_ecs_parity_a.log` files AND the `grep '^RAW'` subsets
  (the exact commands in R10; the `user://` directory is
  `$XDG_DATA_HOME/godot/app_userdata/Third-Person Shooter Demo/` — `project.godot:13`'s `config/name`,
  quote the path). The contract already drives the `Player` with `velocity` + `move_and_slide()`
  at `collision_layer = 1`. If `body_entered` still never fires on EITHER tree (R1's teleported
  probe body never did), escalate in this order and log each answer: (1) `door.monitoring` and
  `door.get_overlapping_bodies()` printed per physics frame from the harness — is the body ever
  overlapping?; (2) if never, the body is not reaching the area: log its `global_position` per
  frame and fix the harness geometry (floor height, door position, speed); (3) if overlapping but
  no signal, check `door.tscn`'s `collision_mask` against the scratch body's layer on BOTH trees.
  All harness-only; never a `door.tscn` edit. **Paste the actual diff output in the completion note — never write
  "identical" without the diff.** Delete the `zz_*` files (and `.uid`s) from both trees.
  Commit (R11 row 3): `door: bridge + open_on_player system (v2 on_body preserved, 3 tests +
  round-trip); docs/v3-tradeoffs.md created (entry d)`.

**Checkpoint**: User Story 2 complete and verified by the harness (no visual checkpoint —
orphaned scene, backlog #30 stays open).

---

## Phase 4: User Story 3 — `part_disappear`: the timer-component exemplar (Priority: P2)

**Goal**: FR-020…FR-022. **Independent Test**: per spec.md US3 — timer/phase tests green;
harness case (b) per-frame log identical on both trees (observer lines), RAW stamps recorded;
visual checkpoint (1).

### Commit 4 — `part_disappear/system.rs` + bridge + `sync_out_puff`

- [ ] T022 [P] [US3] Create `oxide_godot_core/oxide_godot_lib/src/part_disappear/system.rs` and
  declare `mod system;` in `part_disappear.rs` (data-model.md "Components and markers"):
  `#[derive(Component, Clone, Copy, Debug, PartialEq)] pub enum DisappearPhase {
  WaitingToEmit(Timer), Emitting(Timer) }`, `#[derive(Component, Clone, Copy)] pub struct
  Lifetime(pub f32);`, `#[derive(Debug, PartialEq)] pub enum Transition { None, StartEmitting,
  Finished }`, `impl DisappearPhase { pub const EMIT_DELAY: f64 = 0.2; /* v2
  part_disappear.rs:26 */ pub const LIFETIME_FACTOR: f32 = 2.0; /* v2 part_disappear.rs:37 */
  pub fn start() -> Self { WaitingToEmit(Timer::new(Self::EMIT_DELAY)) } pub fn step(&mut self,
  dt: f64, lifetime: f32) -> Transition }` — `step`: `WaitingToEmit(t)` → if `t.step(dt)` then
  `*self = Emitting(Timer::new((lifetime * Self::LIFETIME_FACTOR) as f64))` (f32 multiply THEN
  widen, as v2 passes the `f32` product to `create_timer(f64)`) and return `StartEmitting`, else
  `None`; `Emitting(t)` → if `t.step(dt)` then `Finished` else `None`. System `pub fn
  advance(dt: Res<FrameDelta>, mut puffs: Query<(Entity, &mut DisappearPhase, &Lifetime)>, mut
  commands: Commands)` — per entity `match phase.step(dt.0, lifetime.0) { StartEmitting =>
  insert StartEmitting, Finished => insert Remove, None => {} }`. Verification: compiles; no
  `godot::classes` import.
- [ ] T023 [US3] Add `#[cfg(test)] mod tests` to `part_disappear/system.rs` (FR-022), driving
  `advance` with `run_system_once` on a `World` holding `FrameDelta(1.0 / 60.0)` and one entity
  `(DisappearPhase::start(), Lifetime(1.5))` (the scene's `lifetime = 1.5`,
  `part_disappear.tscn:46`): `phases_occur_in_order` (`WaitingToEmit` → `Emitting` → `Remove`
  present, in that order, never skipping), `waiting_expires_on_step_13_at_sixty_hz_and_starts_emitting_once`
  (`StartEmitting` absent after 12 runs, present after the 13th, and — after removing the marker
  by hand as `sync_out_puff` would — absent again after run 14), `emitting_expires_after_lifetime_times_two_and_finishes_once`
  (from `Emitting(Timer::new((1.5_f32 * 2.0) as f64))`, `Remove` appears on the step the
  subtractive timer reaches ≤ 0 for 3.0 s at 1/60 — step 181 per data-model.md — and not
  before), `no_double_fire_past_the_end` (10 more runs after `Finished`: no second
  `StartEmitting`, `Remove` count stays one). Depends on T022. Verification: four tests pass.
- [ ] T024 [US3] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add `fn sync_out_puff(query:
  Query<Entity, With<StartEmitting>>, mut handles: NonSendMut<NodeHandles>, mut commands:
  Commands)` — `if let Some(Handles::Puff { root }) = handles.by_entity.get_mut(&e) {
  root.set_emitting(true); }` (v2 `part_disappear.rs:34`, once), then
  `commands.entity(e).remove::<StartEmitting>()`; register it in `add_engine_systems` as
  `sync_out_puff.in_set(Phase::SyncOut).before(sync_out_remove)` in the FRAME schedule. In
  `ecs/setup.rs::build_frame` add `crate::part_disappear::system::advance.in_set(Phase::Gameplay)`.
  In `apply_register` replace the `Initial::Puff` placeholder with `world.entity_mut(e).insert((
  DisappearPhase::start(), Lifetime(lifetime)))`. Depends on T022. Verification: compiles; the
  literal `0.2` appears nowhere in `ecs.rs` or `part_disappear.rs` (only as
  `DisappearPhase::EMIT_DELAY` in `system.rs`).
- [ ] T025 [US3] Rewrite `oxide_godot_core/oxide_godot_lib/src/part_disappear.rs` as a bridge
  (FR-020): keep `#[class(init, base=CpuParticles3D)] pub struct PartDisappear { base:
  Base<CpuParticles3D>, #[init(node = "MiniBlasts")] mini_blasts: OnReady<Gd<CpuParticles3D>> }`;
  `impl ICpuParticles3D`: `ready` → `self.mini_blasts.set_emitting(true);` (v2 `:15`, one-shot,
  kept) then `push(InboundEvent::Register { id: self.base().instance_id(), handles:
  Handles::Puff { root: self.to_gd().upcast::<CpuParticles3D>() }, initial: Initial::Puff { lifetime:
  self.base().get_lifetime() } })`; `exit_tree` → `push(Unregister { id })`. Delete the
  `godot::task::spawn` block and its comment (`part_disappear.rs:17-46`); no
  `process`/`physics_process`. Verification: `grep -n 'task::spawn\|create_timer\|fn process'
  part_disappear.rs` returns nothing.
- [ ] T026 [US3] Append row (a) to `docs/v3-tradeoffs.md`: `part_disappear`, `blast` — the
  engine owns frame time (`_process` delta) and its `SceneTreeTimer`s; the ECS cannot hide time
  itself; the sync layer feeds `FrameDelta` from `EcsWorld::process` and the `Timer` component
  reproduces `SceneTree::process_timers`' subtractive arithmetic (research.md R5); location
  `ecs/timer.rs`, `part_disappear/system.rs`. Verification: 2 rows.
- [ ] T027 [US3] Gates + headless: `cargo build && cargo clippy && cargo test` (152 + 4 =
  **156**); `--headless --import`, `main.tscn`, `level.tscn` clean. Harness case (b) on both
  trees exactly as T021 did for (a) (`--case=b`; the puff is instanced from a
  `SceneTreeTimer.timeout` callback — FR-029 (b)); diff the observer lines AND the RAW lines.
  Expected per research.md R5: observer lines identical; RAW `puff_tree_exited` (and the
  `emitting` transition, if a RAW line is added for it) stamped one frame LATER on v2 — record
  WHATEVER is measured for the spec's timing table (T033). **Paste the actual diff output in the
  completion note — never write "identical" without the diff.** Delete the `zz_*` files from
  both trees. Commit (R11 row 4): `part_disappear: bridge + DisappearPhase/Timer system (tests:
  boundaries, order, no double fire); tradeoffs entry (a)`.
- [ ] T028 [US3] 🛑 **STOP — user visual checkpoint (1)** (spec.md SC-005). Ask the user to run
  the game (`cargo build` done, editor or exported run), shoot a robot until it explodes, and
  confirm: the debris puffs appear, their mini-blasts burst immediately, the main puff starts
  emitting shortly after, and every puff vanishes as in v2. Do not proceed to Phase 5 until
  confirmed.

**Checkpoint**: User Stories 1, 2 and 3 complete and independently verified.

---

## Phase 5: User Story 4 — `blast`: per-frame SyncIn/SyncOut with signal-driven despawn (Priority: P3)

**Goal**: FR-023…FR-027. **Independent Test**: per spec.md US4 — headless `level.tscn` clean;
harness case (c) per-frame `LightRays` basis and in-tree log identical on both trees; visual
checkpoint (2).

### Commit 5 — `blast.rs` bridge + `sync_in_blast`/`sync_out_blast`

- [ ] T029 [P] [US4] Rewrite `oxide_godot_core/oxide_godot_lib/src/blast.rs` as a bridge
  (data-model.md "Bridges" row, FR-023): keep `#[class(init, base=Node3D)] pub struct Blast {
  base: Base<Node3D>, #[init(node = "LightRays")] light_rays: OnReady<Gd<CpuParticles3D>>,
  #[init(node = "AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>> }` and DELETE
  the `camera` field, `process` (`blast.rs:41-48`) and the `godot::task::spawn` block
  (`:21-38`); `impl INode3D`: `ready` → `let camera = self.base().get_tree().get_root().unwrap()
  .get_camera_3d();` (v2 `:19`, cached once — `Option<Gd<Camera3D>>`, absent in headless scenes
  without a camera); `self.animation_player.signals().animation_finished().connect_other(&self
  .to_gd(), Self::_on_animation_finished);` — `connect_other`, NOT `connect_self` (in gdext 0.5.5
  `connect_self`'s receiver is the EMITTER, `typed_signal.rs:265-268`; the parent receiving a
  child's signal is `connect_other`, `:299`); then `push(Register { id, handles: Handles::Blast {
  root: self.to_gd().upcast::<Node3D>(), light_rays: self.light_rays.clone(), camera }, initial: Initial::Blast })`
  (`self.to_gd()` is `Gd<Blast>`; the variant wants the engine type — `Gd::upcast`, `gd.rs:425`; same in T019/T025);
  `exit_tree` → `push(Unregister { id })`. `#[godot_api] impl Blast { #[func] fn
  _on_animation_finished(&mut self, _name: StringName) { push(InboundEvent::BlastAnimationFinished
  { id: self.base().instance_id() }); } }` (the `StringName` parameter matches the signal's
  signature and is unused). Verification: `grep -n 'fn process\|task::spawn\|look_at\|camera:'
  blast.rs` returns nothing; `impact_effect.tscn` unedited.
- [ ] T030 [US4] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add the blast sync pair
  (research.md R7, FR-024/FR-025): `fn sync_in_blast(blasts: Query<Entity, With<BlastTag>>,
  handles: NonSend<NodeHandles>, mut targets: Query<&mut LookTarget>, mut cache:
  Local<HashMap<InstanceId, Option<Vector3>>>, mut commands: Commands)` — `cache.clear()` at the
  start of each run; for each blast with `Handles::Blast { camera: Some(cam), .. }`: `let key =
  cam.instance_id();` (no engine call); `let origin = *cache.entry(key).or_insert_with(||
  cam.is_instance_valid().then(|| cam.get_global_transform().origin));` (ONE validity check + ONE
  transform read per distinct camera per frame); on `Some(o)`: if the entity has `LookTarget`,
  `target.set_if_neq(LookTarget(o))` (explicit inequality — bevy's `DerefMut` marks `Changed`
  on every write, `change_detection/traits.rs:431-470`), else `commands.entity(e)
  .insert(LookTarget(o))`; on `None` (invalid camera): nothing (v2 `blast.rs:42-44`).
  `fn sync_out_blast(query: Query<(Entity, &LookTarget), Changed<LookTarget>>, mut handles:
  NonSendMut<NodeHandles>)` — `if let Some(Handles::Blast { light_rays, .. }) = handles
  .by_entity.get_mut(&e) { light_rays.look_at(target.0); }` (v2 `blast.rs:46`; engine-backed
  `Basis::looking_at` — glue by the 1.4.1 rule). Register in `add_engine_systems`:
  `sync_in_blast.in_set(Phase::SyncIn).after(sweep_dead_nodes)` and `sync_out_blast.in_set(
  Phase::SyncOut).before(sync_out_remove)`, FRAME schedule only. In `apply_register`,
  `Initial::Blast` already inserts `BlastTag` (T013). Verification: per lone blast per frame the
  explicit engine calls are `is_instance_valid` (camera) + `get_global_transform` + `look_at`
  (when changed) = v2's three, plus the FR-009 root sweep (excluded by FR-025 as amended) — count
  them by reading the two systems and write the count in the completion note.
- [ ] T031 [US4] Append rows (b) and (c) to `docs/v3-tradeoffs.md`: (b) `blast` — the engine owns
  animation playback and emits `animation_finished` from the `AnimationPlayer`'s own idle
  processing; the ECS cannot hide it because the animation's end is engine time, not tick time;
  the bridge's `#[func]` (typed `connect_other`) pushes `BlastAnimationFinished`, the drain marks
  `Remove`, `sync_out_remove` `queue_free()`s (locations `blast.rs`, `ecs/apply.rs`); (c) `blast`
  — the engine owns the camera's transform and `look_at` is `Basis::looking_at` (engine-backed,
  1.4.1 rule); the entity is sync-only (no gameplay system); `sync_in_blast` reads once per
  distinct camera per frame into `LookTarget` with `set_if_neq`, `sync_out_blast` writes on
  `Changed`; measured explicit engine calls per blast per frame: 3 (v2's count) + the FR-009
  sweep for a lone blast, ≤ 3 + sweep amortized from two concurrent blasts — matching FR-025
  as amended (locations `ecs.rs::sync_in_blast`/`sync_out_blast`). Verification: 4 rows.
- [ ] T032 [US4] Gates + headless: `cargo build && cargo clippy && cargo test` (**156**, no new
  pure tests in this commit — the blast's drain test landed in T009); `--headless --import`,
  `main.tscn`, `level.tscn` clean (a laser hit is not scriptable headless; the harness covers
  it). Harness case (c) on both trees exactly as T021 did (`--case=c`; the blast is instanced
  from `_physics_process` at physics frame 5 and the camera moves every frame from `_process` —
  FR-029 (c)); diff the observer lines (per-frame `rays_basis` at full precision and
  `blast_in_tree`) AND the RAW lines (expected per research.md R5: both identical). **Paste the
  actual diff output in the completion note — never write "identical" without the diff.**
  Delete the `zz_*` files from both trees. Commit (R11 row 5): `blast: bridge + SyncIn/SyncOut
  pair (look_at gated by real change), animation_finished → Remove; tradeoffs entries (b), (c)`.
- [ ] T033 [US4] 🛑 **STOP — user visual checkpoint (2)** (spec.md SC-005). Ask the user to run
  the game, let a robot's laser hit walls and the player while moving the camera, and confirm:
  every laser impact blast faces the camera as it moves and disappears when its animation ends;
  a bullet explosion (unaffected by this milestone — `bullet.tscn:514` is a plain `Node3D`) still
  looks as in v2. Do not proceed to Phase 6 until confirmed.

**Checkpoint**: All four user stories complete and independently verified.

---

## Phase 6: Polish

- [ ] T034 [P] Add the section "Port conventions (v3)" to `CLAUDE.md` after "Port conventions
  (v2)" (FR-013, scenario 10 of US1): the crate pin `bevy_ecs = { version = "0.19",
  default-features = false, features = ["std"] }` in `[workspace.dependencies]` and why the
  three excluded features stay off (constitution 1.5.1); autoload registration through
  `ecs/ecs_world.tscn` + the `[autoload]` line; the bridge template (`ready` registers with the
  handles resolved once, `exit_tree` unregisters, handlers only push; `connect_other` for a
  child's signal received by the parent); the push-never-borrows rule (`ecs::queue::push` from
  any engine callback, never `bind_mut` on `EcsWorld`); the driver's priorities `i32::MAX` and
  why (research.md R1: last in both phases, after the `AnimationPlayer`'s internal processing);
  `queue_free()` only, never `free()`, in the sync layer; the harness recipe against the `v2`
  worktree with split `XDG_DATA_HOME` and an `i32::MIN` observer (R10). Also add ONE sentence to
  the v2 section's "Await-style sequences" bullet: "In v3 gameplay modules these awaits become
  timer components stepped by the frame schedule (see Port conventions (v3)); v2 modules keep the
  async pattern." Verification: `grep -c 'Port conventions (v3)' CLAUDE.md` = 1.
- [ ] T035 [P] Fill the "Measured timing differences" table in
  `specs/011-v3-ecs-core-leaves/spec.md` (FR-030) from the three RAW diffs pasted in T021, T027,
  T032: one row per differing RAW event (case, signal/timer, `v2` frame, `v3` frame, delta,
  cause — e.g. "v2's `godot::task` future resumes via `call_deferred`, flushed in the next
  iteration's physics phase; v3's `SyncOut` writes in the frame the timer expired"); replace the
  `(to be measured)` row. If all three RAW diffs were empty, replace it with a single row
  `| none | — | — | — | 0 | three empty diffs on 2026-XX-XX (outputs quoted in tasks T021/T027/T032) |`.
  Verification: no `(to be measured)` left in spec.md.
- [ ] T036 Confirm `docs/v3-tradeoffs.md` has the header + rows (a), (b), (c), (d) with the final
  wording (`grep -c '^| ' docs/v3-tradeoffs.md` ≥ 5) and that `docs/v2-backlog.md` row #30 still
  reads `open — deferred` (unchanged; no v3 backlog file exists: `ls docs/ | grep v3-backlog`
  returns nothing).
- [ ] T037 Run quickstart.md §5's SC-006 grep list verbatim and paste every output: no
  `fn process`/`fn physics_process` in `door.rs`/`part_disappear.rs`/`blast.rs`; `run_schedule`/
  `.run(&mut` only in `ecs.rs`; no `godot::task::spawn` in the three modules; no
  `get_autoload_by_name`/`/root/EcsWorld` in Rust (or only typed uses); `git diff 08bc3bd --
  oxide_godot_core/oxide_godot_lib/src/{settings,menu,main_scene,level,debug_label,part,bullet,
  red_robot}* | wc -l` = 0; `grep -n 'Gd<' ecs/markers.rs door/system.rs part_disappear/system.rs`
  returns nothing (components and gameplay systems hold no engine handle). Report the residual
  dynamic-access list for the three modules: none.
- [ ] T038 Final gates + headless recipe one more time (`cargo build && cargo clippy && cargo
  test`; `--headless --import`; `--headless main.tscn`/`level.tscn`). Report the final test count
  (SC-001: expect **156** = 133 + 23; the spec's floor is ≥ 148). Commit (R11 row 6):
  `CLAUDE.md: Port conventions (v3); spec: measured timing differences filled;
  docs/v3-tradeoffs.md complete`.
- [ ] T039 Remove the harness worktree and verify cleanliness: `git worktree remove
  ../oxide-godot-v2 && git worktree prune`; `find . ../oxide-godot-v2 -name 'zz_*' 2>/dev/null`
  returns nothing; `git status` clean on `v3`; `git log --oneline 08bc3bd..HEAD` shows exactly
  the six milestone commits; nothing pushed (`git status -sb` shows no upstream ahead/behind
  line that implies a push).

**Checkpoint**: Milestone V3-A complete.

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (US1, commits 1–2) → Phase 3 (US2, commit 3) → Phase 4 (US3, commit 4, STOP 1)
  → Phase 5 (US4, commit 5, STOP 2) → Phase 6 (commit 6), strictly sequential — commit N's gate
  (build + clippy 0 warnings + tests) must pass before commit N+1 starts; each 🛑 STOP task
  blocks the next phase until the user confirms; the harness run precedes each STOP.
- Within commit 1: T003 (crate pin, `ecs.rs` types, `lib.rs`) first; then T004 (`event.rs`) —
  needed by T005 (`queue.rs`) and T009 (`apply.rs`); T005, T006 (`timer.rs`), T007 (`index.rs`),
  T008 (`markers.rs`) are `[P]` (disjoint files, no dependency among them beyond T003/T004);
  T009 and T010 after T004 + T007 + T008 (T010's `build_world` inserts `Messages<DoorBodyEntered>`,
  `EntityIndex`, the deltas — so it is NOT `[P]`); T011 last.
- Within commit 2: T012 → T013 → T014 (same file, in order) → T015 (scene + autoload + headless
  + probe re-run + commit).
- Within commit 3: T016 `[P]` with T020 (`docs/v3-tradeoffs.md`); T017 after T016; T018 after
  T016; T019 after T018 (the bridge needs `Handles::Door` consumed somewhere to be meaningful,
  and `apply_register`'s `DoorState` insertion); T021 last.
- Within commit 4: T022 `[P]` with T026; T023 and T024 after T022; T025 after T024; T027 →
  T028 (STOP).
- Within commit 5: T029 `[P]` with T031; T030 after T029 (it consumes `Handles::Blast`'s
  `light_rays`/`camera`); T032 → T033 (STOP).
- Within commit 6: T034 `[P]` with T035; T036, T037 after them; T038 (commit) then T039.

## Parallel Example: Commit 1

```text
# After T003 and T004 land, five pure files have no dependency on each other:
Task: "T005 ecs/queue.rs — push/drain + 2 tests"
Task: "T006 ecs/timer.rs — Timer::step (subtractive) + 4 tests"
Task: "T007 ecs/index.rs — EntityIndex + 5 tests"
Task: "T008 ecs/markers.rs — resources + markers"
# then T009 (apply.rs) and T010 (setup.rs, 3 tests) — both need event/index/markers — then T011 (gate + commit).
```

## Implementation Strategy

Sequential by design (single implementer; the stories are three small leaves on one shared
core): Setup → US1 (the core — commits 1–2, the MVP in the sense that every later v3 milestone
copies it) → US2 (door, commit 3, no STOP) → US3 (commit 4) → **STOP 1** → US4 (commit 5) →
**STOP 2** → Polish (commit 6). The milestone is not shippable partway: US1 alone proves the
core only headless, and the constitution's parity rule needs the three harness cases plus both
visual checkpoints.

Suggested `/speckit-implement` session split:

- **Session 1** — Phase 1 + commits 1–2 (T001–T015), including the R1 probe re-run on the real
  `EcsWorld` build.
- **Session 2** — commits 3–4 (T016–T027), ending at 🛑 STOP 1 (T028).
- **Session 3** — commit 5 (T029–T032), ending at 🛑 STOP 2 (T033).
- **Session 4** — commit 6 (T034–T039).
