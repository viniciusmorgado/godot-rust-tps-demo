# Contract: the ECS core as seen by bridges, system authors and reviewers (V3-A)

## 1. Bridge contract (every gameplay scene root class)

A bridge is a `#[derive(GodotClass)]` with the scene node's base. It MAY have:

- `fn ready(&mut self)`: resolve child handles ONCE (`OnReady`/`#[init(node = ...)]`), do
  one-shot engine setup that v2 did at the same moment (e.g. `MiniBlasts.emitting = true`),
  make typed signal connections to its own `#[func]`s — `connect_self` when the bridge itself
  emits the signal, `connect_other(&self.to_gd(), Self::handler)` when a child node emits it
  (`connect_self`'s receiver is the emitter, `typed_signal.rs:265`) — then
  `ecs::queue::push(InboundEvent::Register { id: self.base().instance_id(), handles, initial })`.
- `fn exit_tree(&mut self)`: `ecs::queue::push(InboundEvent::Unregister { id })`.
- `#[func]`/`#[rpc]`/signal handlers: translate the engine event into ONE typed
  `InboundEvent` and push it. Boundary typing (`try_cast`) is allowed here. Nothing else.

A bridge MUST NOT have `process`/`physics_process`, hold an `Entity`, call
`get_autoload_by_name::<EcsWorld>`, `bind`/`bind_mut` anything ECS-related, spawn a
`godot::task`, or free a node.

**Push API** (`ecs/queue.rs`):

```rust
pub fn push(event: InboundEvent);          // never blocks, never borrows EcsWorld or the World
pub fn drain() -> Vec<InboundEvent>;       // driver only
```

**Guarantee**: `push` borrows a module-private `thread_local!` `RefCell` for one `Vec::push`
and returns. It is safe from ANY engine callback, including one fired synchronously by an engine
call the sync layer itself made while a schedule is running; such an event is consumed by the
next schedule run (FIFO across runs).

## 2. System-author contract

**Schedules and sets** — both `Fixed` (run by `EcsWorld::physics_process`) and `Frame` (run by
`EcsWorld::process`) chain `Phase::SyncIn → Phase::Gameplay → Phase::EngineQuery →
Phase::SyncOut`. Before each run the driver drains the queue (registrations applied, gameplay
messages written) — a system never sees an `InboundEvent`.

**Resources present in every run**: `EntityIndex` (`Res`), `NodeHandles` (`NonSend` — sync
systems only), `Messages<DoorBodyEntered>`, `FrameDelta`, `FixedDelta` (the one matching the
running schedule was written this run; the other holds the last value).

**Gameplay systems** (`Phase::Gameplay`): PURE — parameters limited to `Query`, `Res`/`ResMut`
of Send resources, `MessageReader`/`MessageWriter`, `Commands`, `Local`. No `NonSend`, no
`Gd<T>`, no singleton, no engine-backed builtin (`Basis::looking_at`, `Quaternion::slerp*`).
They decide by writing components and inserting markers (`PlayOpen`, `StartEmitting`,
`Remove`); they never call the engine and never free anything. Tested with `World::new()` +
`run_system_once`.

**Sync systems** (`Phase::SyncIn`, `Phase::SyncOut`; live in `ecs.rs`): the only systems that
read `NonSend<NodeHandles>` / `NonSendMut<NodeHandles>`. `SyncIn` reads each engine value once
per run and writes it into components with `set_if_neq`; `SyncOut` writes each component value
once, gated by `Changed<T>` where the write is idempotent, consumes markers (`remove::<Marker>`),
and releases nodes ONLY via `queue_free()` for entities carrying `Remove`, then despawns them and
drops both map entries.

**`EngineQuery`** (empty in V3-A): reserved for systems that need an engine answer mid-tick
(raycasts, `move_and_slide`); each member requires a `docs/v3-tradeoffs.md` row.

**Deltas**: read `Res<FrameDelta>`/`Res<FixedDelta>`; never call `get_process_delta_time`.

## 3. `docs/v3-tradeoffs.md` (created in commit 3 of the plan)

```markdown
# v3 trade-offs — engine touch points

Where the ECS abstraction touches the engine (constitution 1.5.1, Principle I v3 and Principle
III "ECS shape"). One row per touch point, added in the commit that introduces it.

| Module | What the engine owns | Why the ECS cannot hide it | How the sync layer handles it | Location |
|---|---|---|---|---|
```

Rows this milestone adds (final wording at implement time): (a) `part_disappear`, `blast` —
scene-tree timers → tick timers in `Frame`; (b) `blast` — `animation_finished` is emitted by the
`AnimationPlayer`'s own processing → bridge `#[func]` → `BlastAnimationFinished` → `Remove`;
(c) `blast` — the camera's origin lives in the engine and `look_at` is `Basis::looking_at`
(engine-backed) → sync-only entity, `SyncIn` reads once per distinct camera, `SyncOut` writes
when changed; (d) `part`, `level`, `red_robot` — `PackedScene::instantiate` stays in v2 node
code; the spawned node's bridge registers itself from `ready`.

## 4. Harness contract files

- `zz_ecs_parity.gd` — the three-case parity harness with its observer (research R10). Its
  scene `zz_ecs_parity.tscn` is these six lines (Godot rewrites `uid`s on import; fine for a
  scratch file):

  ```text
  [gd_scene load_steps=2 format=3]

  [ext_resource type="Script" path="res://zz_ecs_parity.gd" id="1"]

  [node name="ZzEcsParity" type="Node3D"]
  script = ExtResource("1")
  ```

  `zz_ecs_observer.gd` is the two-line script quoted inside `zz_ecs_parity.gd`'s `_ready`.
- `zz_order_probe.gd` — the R1 experiment (autoload + scene scripts merged into one documented
  file), so the priority decision can be re-verified on any future engine version.

Both are documentation of scratch code: they are copied into the game project only for a
harness run and deleted before every commit; they are never committed under `oxide-godot/`.
