# Data Model: Milestone V3-A — ECS core and the three leaf effects

Every type below is the exact Rust surface the plan commits to; the "v2 line" column names the
v2 code each item reproduces. Pure types carry their tests (FR-012, FR-018, FR-022) by name.
Engine-handle types (`Handles`, `NodeHandles`) are glue (`ecs.rs`); everything else is pure.

## Events (`ecs/event.rs`)

```rust
pub enum InboundEvent {
    Register { id: InstanceId, handles: Handles, initial: Initial },
    Unregister { id: InstanceId },
    DoorBodyEntered { id: InstanceId, is_player: bool },
    BlastAnimationFinished { id: InstanceId },
}
pub enum Initial {                    // the pure half of a registration (Send, testable)
    Door,                             // → DoorState::Closed
    Puff { lifetime: f32 },           // → DisappearPhase::start() (= WaitingToEmit(Timer::new(EMIT_DELAY))), Lifetime(lifetime)
    Blast,                            // → no component beyond the marker `BlastTag`
}
#[derive(Message, Clone, Copy)] pub struct DoorBodyEntered { pub entity: Entity, pub is_player: bool }
```

| Item | v2 line reproduced |
|---|---|
| `DoorBodyEntered { id, is_player }` pushed by the bridge; `is_player = body.try_cast::<Player>().is_ok()` | `door.rs:76` |
| `Initial::Puff { lifetime }` = `get_lifetime()` read once in `ready` (f32) | `part_disappear.rs:35` |
| `BlastAnimationFinished` pushed by the `#[func]` connected to `animation_finished` | `blast.rs:29-33` |

The `Message` form (`DoorBodyEntered { entity, is_player }`) is what the drain writes AFTER
resolving `id → Entity` through `EntityIndex` (unknown id → dropped, spec US2 scenario 5), so the
door system never sees an `InstanceId`.

## Queue (`ecs/queue.rs`)

`thread_local! QUEUE: RefCell<Vec<InboundEvent>>`; `pub fn push(InboundEvent)`;
`pub fn drain() -> Vec<InboundEvent>` (`mem::take`).

Tests: `drain_returns_events_in_push_order`, `drain_empties_the_queue`.

## Handles (`ecs.rs`, glue, NonSend)

```rust
pub enum Handles {
    Door  { root: Gd<Area3D>,          anim: Gd<AnimationPlayer> },          // door.rs:55, :62-63
    Puff  { root: Gd<CpuParticles3D> },                                      // part_disappear.rs:7
    Blast { root: Gd<Node3D>, light_rays: Gd<CpuParticles3D>, camera: Option<Gd<Camera3D>> }, // blast.rs:7, :9, :13
}
impl Handles { pub fn root_valid(&self) -> bool /* one is_instance_valid on root */ }
#[derive(Default)] pub struct NodeHandles { pub by_entity: HashMap<Entity, Handles> }  // NonSend
```

`Handles::Blast` carries NO `AnimationPlayer` (FR-008 as amended in `e746e28`): the typed
connection is made in the bridge and never needed by the sync layer.

## Index (`ecs/index.rs`, pure)

```rust
#[derive(Resource, Default)] pub struct EntityIndex { by_id: HashMap<InstanceId, Entity> }
pub enum Registration { Registered(Entity), AlreadyRegistered(Entity) }
impl EntityIndex {
    pub fn register_if_absent(&mut self, id: InstanceId, spawn: impl FnOnce() -> Entity) -> Registration;
    pub fn unregister(&mut self, id: InstanceId) -> Option<Entity>;   // None = unknown id, no-op
    pub fn entity(&self, id: InstanceId) -> Option<Entity>;
    pub fn remove_entity(&mut self, entity: Entity);                   // used by the sweep / Remove
}
```

Tests: `register_spawns_once`, `second_register_of_same_id_keeps_first_entity`,
`unregister_unknown_id_is_noop`, `unregister_then_late_sweep_is_noop`,
`sweep_then_late_unregister_is_noop` (the two `exit_tree`-after-despawn orders, spec Edge Cases,
exercised on `EntityIndex` + a `World` with fake ids `InstanceId::from_i64(n)`).

## Timer (`ecs/timer.rs`, pure)

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timer { time_left: f64, expired: bool }
impl Timer {
    pub fn new(seconds: f64) -> Self;
    /// Engine arithmetic (SceneTree::process_timers): time_left -= dt; fires once when <= 0.
    pub fn step(&mut self, dt: f64) -> bool;
}
```

| Item | v2 line reproduced |
|---|---|
| `Timer::new(DisappearPhase::EMIT_DELAY)` (`EMIT_DELAY: f64 = 0.2`) | `part_disappear.rs:26` `create_timer(0.2)` |
| `Timer::new((lifetime * DisappearPhase::LIFETIME_FACTOR) as f64)` (`LIFETIME_FACTOR: f32 = 2.0`) — f32 multiply, then widen, as v2 passes an `f32` product to a `f64` parameter | `part_disappear.rs:35-37` |

Tests: `expires_on_the_step_that_reaches_zero` (0.2 at `dt = 1/60` fires on step 13 and not on
step 12 — the value R1 fact 6 observed on the engine), `does_not_expire_one_step_before`,
`fires_exactly_once` (a further step returns `false`), `three_seconds_at_sixty_hz_fires_on_step_181`.

## Components and markers (`ecs/markers.rs`, `door/system.rs`, `part_disappear/system.rs`)

```rust
#[derive(Resource, Clone, Copy)] pub struct FrameDelta(pub f64);   // written by EcsWorld::process
#[derive(Resource, Clone, Copy)] pub struct FixedDelta(pub f64);   // written by EcsWorld::physics_process
#[derive(Component)] pub struct PlayOpen;        // consumed by sync_out_door
#[derive(Component)] pub struct StartEmitting;   // consumed by sync_out_puff
#[derive(Component)] pub struct Remove;          // consumed by sync_out_remove
#[derive(Component)] pub struct BlastTag;        // selects blast entities in sync_in/out_blast
#[derive(Component, Clone, Copy, PartialEq)] pub struct LookTarget(pub Vector3);  // written by sync_in_blast (set_if_neq)

// door/system.rs
#[derive(Component, Clone, Copy, Debug, PartialEq)] pub enum DoorState { Closed, Open }
pub fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool);   // v2 door.rs:18-23, verbatim
pub fn open_on_player(reader: MessageReader<DoorBodyEntered>, doors: Query<&mut DoorState>, commands: Commands);

// part_disappear/system.rs
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum DisappearPhase { WaitingToEmit(Timer), Emitting(Timer) }
#[derive(Component, Clone, Copy)] pub struct Lifetime(pub f32);
#[derive(Debug, PartialEq)] pub enum Transition { None, StartEmitting, Finished }
impl DisappearPhase {
    /// Tuning of this type (constitution Principle III: tuning constants belong to the type).
    pub const EMIT_DELAY: f64 = 0.2;        // v2 part_disappear.rs:26 — create_timer(0.2)
    pub const LIFETIME_FACTOR: f32 = 2.0;   // v2 part_disappear.rs:37 — lifetime * 2.0
    pub fn start() -> Self;                 // WaitingToEmit(Timer::new(Self::EMIT_DELAY)); the registration
                                            // path (Initial::Puff) calls this and never sees the literal
    pub fn step(&mut self, dt: f64, lifetime: f32) -> Transition;
}
pub fn advance(dt: Res<FrameDelta>, puffs: Query<(Entity, &mut DisappearPhase, &Lifetime)>, commands: Commands);
```

| Item | v2 line reproduced |
|---|---|
| `DoorState`, `on_body` | `door.rs:10-23` (moved, unchanged) |
| `PlayOpen` → `play_ex().name("doorsimple_opening")` in `sync_out_door` | `door.rs:79-81` |
| `WaitingToEmit` expiry → `StartEmitting` → `set_emitting(true)` in `sync_out_puff`; phase becomes `Emitting(Timer::new((lifetime*2.0) as f64))` | `part_disappear.rs:30-41` |
| `Emitting` expiry → `Transition::Finished` → `Remove` → `queue_free()` | `part_disappear.rs:45` |
| `LookTarget` from `camera.get_global_transform().origin`, `look_at` in `sync_out_blast` | `blast.rs:45-46` |
| `BlastAnimationFinished` → `Remove` → `queue_free()` | `blast.rs:37` |

State transitions of `DisappearPhase`: `WaitingToEmit(t)` —`t.step(dt)` fires→
`Emitting(Timer::new((lifetime * Self::LIFETIME_FACTOR) as f64))` + `Transition::StartEmitting`;
`Emitting(t)` —fires→ unchanged phase + `Transition::Finished` (once; the `Remove` marker makes
the entity leave before a second step could matter, and `Timer::expired` guards anyway).

Tests — door (`door/system.rs`, FR-018): `closed_door_opens_for_a_player`,
`closed_door_ignores_a_non_player_body`, `open_door_does_not_retrigger_for_a_player` (the three
v2 cases, now `World::new()` + one entity + one message + `run_system_once(open_on_player)`,
asserting `DoorState` and the presence/absence of `PlayOpen`), plus
`event_for_registered_id_reaches_its_entity` (round-trip through `EntityIndex` and
`apply::apply_non_register`) and `event_for_unknown_id_is_dropped`.

Tests — puff (`part_disappear/system.rs`, FR-022): `phases_occur_in_order`,
`waiting_expires_on_step_13_at_sixty_hz_and_starts_emitting_once`,
`emitting_expires_after_lifetime_times_two_and_finishes_once`, `no_double_fire_past_the_end`
(stepping 10 extra steps yields `Transition::None` and no second `StartEmitting`/`Remove`).

Tests — blast (`ecs/apply.rs`, FR-012/FR-026): `blast_animation_finished_marks_remove`,
`blast_animation_finished_for_unknown_id_is_dropped`.

Tests — setup (`ecs/setup.rs`): `both_schedules_run_on_an_empty_world`,
`phase_sets_are_chained_in_order` (a probe system per set writes to a `Vec` resource; the order
observed is `SyncIn, Gameplay, EngineQuery, SyncOut`),
`marker_inserted_in_gameplay_is_visible_in_sync_out_of_the_same_run` (pins bevy's
`auto_insert_apply_deferred` default, research R6 / analyze M3).

## Schedules and sets (`ecs/setup.rs`)

```rust
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase { SyncIn, Gameplay, EngineQuery, SyncOut }
#[derive(ScheduleLabel, ...)] pub struct Fixed;  #[derive(ScheduleLabel, ...)] pub struct Frame;
pub fn build_world() -> World;            // EntityIndex, NodeHandles (non-send), Messages<DoorBodyEntered>, FrameDelta, FixedDelta
pub fn build_fixed() -> Schedule;         // sets chained; pure systems: open_on_player (Gameplay)
pub fn build_frame() -> Schedule;         // sets chained; pure systems: advance (Gameplay)
```

Engine-touching systems are added by `ecs.rs::add_engine_systems(fixed, frame)` after
construction (research R8/R9), so `setup.rs` is testable without Godot.

## Bridges (glue; the exact surface each scene sees)

| Class (base) | `ready` pushes | `exit_tree` pushes | Handlers | Preserved surface |
|---|---|---|---|---|
| `Door` (`Area3D`) | `Register { id, Handles::Door { root: self.to_gd().upcast::<Area3D>(), anim }, Initial::Door }` — `anim` from `#[init(node = "DoorModel2/AnimationPlayer")]` under the upstream bug fix comment (`door.rs:60-63`) | `Unregister { id }` | `#[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>)` → `try_cast::<Player>()` (backlog #14 comment kept, `door.rs:73-75`) → push `DoorBodyEntered` | `door.tscn:35` connection; type name `Door` |
| `PartDisappear` (`CpuParticles3D`) | `mini_blasts.set_emitting(true)` (v2 `:15`, one-shot); `Register { id, Handles::Puff { root: self.to_gd().upcast::<CpuParticles3D>() }, Initial::Puff { lifetime: self.base().get_lifetime() } }` | `Unregister { id }` | none | type name `PartDisappear` (`part_disappear.tscn:42`) |
| `Blast` (`Node3D`) | camera = `get_tree().get_root().get_camera_3d()` (v2 `:19`); `animation_player.signals().animation_finished().connect_other(&self.to_gd(), Self::_on_animation_finished)` — `connect_other`, NOT `connect_self`: in gdext 0.5.5 `connect_self` hands the receiver `&mut C` where `C` is the signal's EMITTER (`typed_signal.rs:265-268`, here the `AnimationPlayer`); receiving a child's signal on the parent is `connect_other` (`:299`), which captures a strong `Gd<Blast>` inside the child's connection — harmless for a `Node` (manual memory: `queue_free` frees it regardless, and the child dies with it); `Register { id, Handles::Blast { root: self.to_gd().upcast::<Node3D>(), light_rays, camera }, Initial::Blast }` | `Unregister { id }` | `#[func] fn _on_animation_finished(&mut self, _name: StringName)` → push `BlastAnimationFinished` (the `StringName` parameter is required by the signal's signature; it is not used) | type name `Blast` (`impact_effect.tscn:169`) |

`id` is always `self.base().instance_id()` (`Gd::instance_id`, `gd.rs:301`, through
`WithBaseField::base`, `traits.rs:420`). `self.to_gd()` returns `Gd<Self>` (the bridge class), so
every root handle is upcast to the engine type the `Handles` variant declares (`Gd::upcast`,
`gd.rs:425`) — analyze B1, 2026-09-18. None of the three has `process`/`physics_process`, an
`Entity` field, or any access to `EcsWorld`.
