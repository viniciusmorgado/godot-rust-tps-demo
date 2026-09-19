# Tasks: Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the ECS core

**Input**: Design documents from `/specs/013-v3-bullet-part-robot/` (spec.md, plan.md, research.md
R1–R11, data-model.md, contracts/enemy-entities.md, contracts/zz_ecs_parity.gd,
contracts/zz_r1_robot.gd, contracts/zz_r2_parts.gd, contracts/zz_r3_order.gd, quickstart.md — all
at commit `9d0edec`; backlog #31 is CLOSED by FR-023).

**Branch**: `v3` — local commits only, **never** `git push`. **Baseline**: `9d0edec` (code baseline
`30d1a4d`, V3-B complete; constitution 1.5.2 at `cfd20be`), **179 tests**.

**Tests**: requested by the spec (FR-029, SC-001; Principle III: every gameplay system carries a
`run_system_once` test; the 33 pure tests — `bullet.rs` 3, `part.rs` 5, `red_robot/model.rs` 25 —
are preserved by name). Named test cases below are the MINIMUM; a task that creates pure code
without its named tests is incomplete. Running totals, computed from the names: 179 → **187**
(commit 1: `ecs/apply.rs` +8) → **192** (commit 2: `bullet/system.rs` +5) → **214** (commit 3:
`part/system.rs` +5, `red_robot/system.rs` +16 — the fourteen of data-model.md plus
`robot_that_just_died_is_not_advanced` and `remote_hit_decrements_client_health_and_dies_at_zero`
from the analyze fixes — `ecs/setup.rs` +1) — 187/192/214 (plan.md and data-model.md updated in
the analyze-fix commit).

**Organization**: one task group per commit of research R11's Commit Plan (four commits, in
order). The two user visual checkpoints are 🛑 STOP tasks after commit 3 — confirmed by the user,
not the implementer; both block Phase 5. Every harness task MUST paste the actual `diff` output
in its completion note — never write "identical" without the diff.

**Facts from V3-A/V3-B the tasks build on (not re-decided)**: `Handles::X(Box<XHandles>)` boxed
variants matched as `Handles::X(p)`; `pub(crate) mod model` / `pub(crate) mod pure` visibility
where a system file imports a pure type; `type` aliases for `Query` tuples beyond ~7 items
(clippy `type_complexity`, no `#[allow]`); `Tuning<T>` resources; every `bind()`/`bind_mut()`
guard dropped before an engine call that can invoke a callback; `advance(FixedDelta)` LAST in the
entity's `SyncOut`; `Messages::update()` for `RobotHitLocal` next to `DoorBodyEntered`'s at the
start of each FIXED run; the harness: `seed(1)` FIRST, the joypad purge SECOND, scripts set
BEFORE `add_child`, `is_instance_valid(part)` BEFORE reading a part (research R2's lesson).

## Phase 1: Setup

- [x] T001 Verify the `v2` worktree (re-created for research R1/R2): `git worktree list` shows
  `../oxide-godot-v2` at `e2932b4`; else `git worktree add ../oxide-godot-v2 v2`; then `cd
  ../oxide-godot-v2/oxide_godot_core && cargo build` and `cd ../oxide-godot && /usr/bin/godot.x86_64
  --headless --import --path .`. Verification: build clean, import prints `Initialize godot-rust
  (...)`.
- [x] T002 On `v3` at `9d0edec`: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`
  → zero warnings, **179 passed** (paste the `test result:` line); `sed -n 10782p
  oxide-godot/enemies/red_robot/red_robot.tscn` prints `callback_mode_process = 0` (the line T023
  edits); `sed -n 10764p oxide-godot/enemies/red_robot/red_robot.tscn` prints `enabled = false`
  (the laser `RayCast`, research R8 Fact 1 — the constant read T019 keeps); `grep -c '#\[test\]'
  oxide_godot_core/oxide_godot_lib/src/bullet.rs oxide_godot_core/oxide_godot_lib/src/part.rs
  oxide_godot_core/oxide_godot_lib/src/red_robot/model.rs` prints 3, 5, 25 (the 33 tests that must
  survive untouched); `grep -n '^| 31 ' docs/v2-backlog.md` shows the row still open (T024 closes it).

**Checkpoint**: baseline confirmed on both trees; commit 1 can start.

---

## Phase 2: Foundational — commit 1: `ecs` + `hittable` extension, pure

**Goal**: the events, message, components, handles, registration and drain arms every entity of
this milestone needs; `HitKind` in `hittable.rs`. Nothing engine-facing beyond the additive
`hittable.rs` helper. **Independent Test**: `cargo test` green at 187 with V3-A/V3-B's tests
untouched; the game still runs unchanged (no bridge touched).

### Commit 1 — `hittable.rs`, `ecs/event.rs`, `ecs/markers.rs`, `ecs/setup.rs`, `ecs/apply.rs`, `ecs.rs`

- [x] T003 [P] In `oxide_godot_core/oxide_godot_lib/src/hittable.rs` ADD (additive only — no
  existing line changes; `HitTarget`, `resolve` `:16-24`, `rpc_hit` `:30-39` untouched, research R3):
  `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum HitKind { Player(InstanceId),
  Robot(InstanceId) }`; `pub fn kind_of(node: Gd<Node3D>) -> Option<HitKind>` = `resolve(node)`
  mapped to `instance_id()`; `impl HitKind { pub fn rpc_hit(self) { … } pub fn robot_id(self) ->
  Option<InstanceId> }` where `rpc_hit` re-fetches the target with
  `Gd::<Player>::try_from_instance_id(id)` / `Gd::<EnemyRobot>::try_from_instance_id(id)` (godot-core
  `src/obj/gd.rs:255`), wraps it in the existing `HitTarget` and calls the existing `rpc_hit()`
  (the by-name string stays spelled ONCE, `:33`/`:36`); a freed target (`Err`) is a no-op.
  Verification: compiles; `git diff -- hittable.rs | grep '^-' | grep -v '^---' | wc -l` = 0.
- [x] T004 [P] In `oxide_godot_core/oxide_godot_lib/src/ecs/event.rs` add the variants
  `InboundEvent::BulletFx { root_id: InstanceId, fx: BulletFx }`, `BulletDestroy { root_id }`,
  `PartFx { root_id, fx: PartFx }`, `RobotHit { root_id }`, `RobotFx { root_id, fx: RobotFx }`,
  `ShootRequested { root_id }`, `ResumeApproachRequested { root_id }`, `RobotPlayerSeen { root_id,
  player: Option<InstanceId> }`; the enums `#[derive(Clone, Copy, Debug, PartialEq)] pub enum
  BulletFx { Explode }`, `PartFx { Destroy }`, `RobotFx { PlayShoot }`; `Initial::Bullet {
  shadow_mapping: bool, simulates: bool }` (`bullet.rs:143`, `:89`), `Initial::Part { lifetime: f32,
  lifetime_random: f32, disappearing_time: f32, simulates: bool }` (`part.rs:86-94`, `:167`),
  `Initial::Robot { state: State, health: i32, dead: bool, test_shoot: bool, orientation:
  Transform3D, aim_blend: Vector2, simulates: bool }` (`red_robot.rs:35-47`, `:133`;
  `:110-117`; `aim_blend` = the tree's `parameters/aim/blend_position` at `ready`,
  `red_robot.tscn:10785`; `use crate::red_robot::State`); `#[derive(Message, Clone, Copy)] pub
  struct RobotHitLocal { pub robot: InstanceId }` (research R4). Extend `queue.rs`'s test helper
  `key()` with the eight new arms (tags 8–15), as commit 1 of V3-B did. Verification: compiles;
  `Initial` stays `Copy`.
- [x] T005 [P] In `oxide_godot_core/oxide_godot_lib/src/ecs/markers.rs` add every component of
  data-model.md "Components" verbatim, with its derives: bullet — `BulletTag`,
  `BulletStateC(BulletState)` (`use crate::bullet::pure::BulletState`; make `bullet.rs`'s inline
  `mod pure` `pub(crate) mod pure` — the ONE token change in that file this commit; its body and 3
  tests untouched), `BulletBasisZ(Vector3)`, `Collided { hit: Option<HitKind>, collided: bool }`
  (`use crate::hittable::HitKind`), `BulletIntents { active, explode, hit: Option<HitKind>,
  disable_collision }`, `PendingBulletFx(Vec<BulletFx>)`; part — `PartTag`, `PartPhase { Attached,
  Waiting(Timer), Fading { counter: f32 }, Destroyed(Timer) }` (`use super::timer::Timer`),
  `PartLifetimes { lifetime, lifetime_random, disappearing_time }`, `PartIntents { fade:
  Option<f32>, destroy: bool }`, `PendingPartFx(Vec<PartFx>)`; robot — `RobotTag`, `Dead`,
  `RobotState(State)`, `Health(i32)`, `TargetPosition(Vector3)`, `RobotCountersC(RobotCounters)`
  (`crate::red_robot::model::RobotCounters` — make `red_robot.rs`'s `mod model` `pub(crate)`; the
  file `model.rs` untouched), `TrackedPlayer(Option<InstanceId>)`, `AimBlend(Vector2)`,
  `ShootRequested`, `RobotFrame { global_transform, gravity, velocity, player_origin:
  Option<Vector3>, ray_from: Transform3D, ray_mesh: Transform3D, ray_mesh_z: f32, laser_colliding:
  bool, laser_point: Vector3 }`, `ReplayRobot { state, target_position, aim_preparing }`,
  `RobotIntents { idle_branch, raycast: Option<(Vector3, Vector3)>, shoot, clip: Option<f32>,
  play_shoot, anim: Option<AnimDecision>, hit, just_died }` + `AnimDecision { request: &'static
  str, aim: Option<(f32, Vector2)> }`, `RaycastAnswers { sees_player: Option<bool>, shot:
  Option<ShotResult> }` + `ShotResult { max_dist: f32, hit: Option<(Vector3, Option<InstanceId>)> }`,
  `PendingTrauma(Option<(Timer, InstanceId)>)`, `RemovalTimer(Option<Timer>)`,
  `TraumaDue(InstanceId)`, `PendingRobotFx(Vec<RobotFx>)`, `PendingRobotHits(u32)`.
  `Orientation`, `RootMotion`, `Velocity`, `Simulates` are reused from V3-B. Verification:
  compiles; `grep -c 'Gd<' ecs/markers.rs` = 0.
- [x] T006 [P] In `oxide_godot_core/oxide_godot_lib/src/ecs/setup.rs`: `build_world` inserts
  `Messages::<RobotHitLocal>::default()` and `Tuning(RobotTuning::default())`
  (`red_robot/model.rs:15-48`). No set changes (the seven-set fixed chain and the four-set frame
  chain are V3-B's). Verification: `both_schedules_run_on_an_empty_world` and the other V3-B setup
  tests pass unchanged.
- [x] T007 In `oxide_godot_core/oxide_godot_lib/src/ecs/apply.rs` add the eight drain arms
  (data-model.md "Drain arms"; every unknown `root_id` dropped): `BulletFx { fx }` →
  `PendingBulletFx.0.push(fx)` (`bullet.rs:137-146`); `BulletDestroy` → insert `Remove` ONLY if the
  entity has `Simulates` (`:150-153`, v2's server-only guard; `world.get::<Simulates>(e).is_some()`);
  `PartFx { fx }` → `PendingPartFx.0.push(fx)` (`part.rs:194-215`); `RobotHit` →
  `PendingRobotHits.0 += 1` (`red_robot.rs:276-328`, remote peers only — the local path is
  `RobotHitLocal`); `RobotFx { fx }` → `PendingRobotFx.0.push(fx)` (`:330-333`); `ShootRequested`
  → insert the `ShootRequested` marker (`:335-338`); `ResumeApproachRequested` → `RobotState =
  Approach`, counters `(aim_preparing, shoot_countdown) = resume_approach_reset(&tuning.0)`
  (`:267-274`, `model.rs:110-113`; `Res<Tuning<RobotTuning>>` read from the world);
  `RobotPlayerSeen { player }` → `TrackedPlayer = player`; on `Some`: FIRST
  `resume_approach_reset` on the counters (backlog #31 CLOSED, FR-023 — `aim_countdown` untouched),
  THEN `RobotState = Approach`; on `None`: `RobotState = Idle`, counters untouched (`:340-358`).
  Tests (`World::new()` + `EntityIndex` + `Tuning<RobotTuning>` + one entity with the queued
  components, registered under a fake id): `bullet_fx_explode_is_queued`,
  `bullet_destroy_marks_remove_only_on_simulates` (two entities: with/without `Simulates` → only
  the first gets `Remove`), `part_fx_destroy_is_queued`, `robot_hit_is_counted_for_remote_peers`
  (two events → 2), `robot_fx_play_shoot_is_queued`, `shoot_requested_inserts_marker`,
  `robot_player_seen_resets_counters_on_entry` (stale counters `{ aim_preparing: 0.1,
  shoot_countdown: 2.0, aim_countdown: 0.3 }` + `Some(id)` → `Approach`, `aim_preparing ==
  aim_prepare_time`, `shoot_countdown == shoot_wait`, `aim_countdown` still 0.3, `TrackedPlayer ==
  Some(id)`), `robot_player_seen_none_sets_idle_without_reset` (`None` → `Idle`, counters
  unchanged, `TrackedPlayer == None`). The `ResumeApproachRequested` arm is covered by
  `resume_approach_requested_resets_state_and_counters`, asserted as a block INSIDE
  `robot_player_seen_resets_counters_on_entry` (same reset formula, same file) so the file stays at
  eight named tests. Depends on
  T004, T005, T006. Verification: eight tests pass.
- [x] T008 In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add `Handles::Bullet(Box<BulletHandles>)`,
  `Handles::Part(Box<PartHandles>)`, `Handles::Robot(Box<RobotHandles>)` with the field lists of
  data-model.md "Handles" (`BulletHandles { root: Gd<Bullet>, anim: Gd<AnimationPlayer>, collision:
  Gd<CollisionShape3D>, light: Gd<OmniLight3D>, shadow_mapping: bool }` — `shadow_mapping` lives
  here, glue-only; `PartHandles { root: Gd<Part>, synchronizer: Gd<MultiplayerSynchronizer>, col1,
  col2: Gd<CollisionShape3D>, puff_scene: Gd<PackedScene> }`; `RobotHandles { root: Gd<EnemyRobot>,
  anim_tree, shoot_anim, model, ray_from: Gd<BoneAttachment3D>, ray_mesh: Gd<MeshInstance3D>,
  laser_raycast: Gd<RayCast3D>, laser_ember: Gd<CpuParticles3D>, collision_shape, explosion_sound,
  hit_sound: Gd<AudioStreamPlayer3D>, death: Gd<Node3D>, parts: [Gd<Part>; 3], sparks:
  [Gd<CpuParticles3D>; 2], impact_effect_scene: Gd<PackedScene>, rid: Rid, is_dedicated_server:
  bool }`); the three `root_valid()` arms; the three `sync_out_remove` arms (`Handles::Bullet(mut
  p) => p.root.queue_free()`, …); the `apply_register` arms per data-model.md "Registration"
  (`Initial::Bullet` → `BulletTag`, `BulletStateC(Flying { time_alive: 5.0 })` (`bullet.rs:68`),
  `BulletBasisZ(ZERO)`, `Collided::default()`, `BulletIntents::default()`, `PendingBulletFx::default()`;
  `Initial::Part` → `PartTag`, `PartPhase::Attached`, `PartLifetimes`, `PartIntents::default()`,
  `PendingPartFx::default()`; `Initial::Robot` → `RobotTag`, `RobotState`, `Health`, `Dead` iff
  `dead`, `TargetPosition(ZERO)`, `RobotCountersC` with `shoot_countdown = 0.0` iff `test_shoot`
  (`red_robot.rs:49-55`, `:115-117`), `TrackedPlayer(None)`, `Orientation`, `RootMotion(IDENTITY)`,
  `Velocity(ZERO)`, `AimBlend`, `RobotIntents::default()`, `RaycastAnswers::default()`,
  `PendingTrauma::default()`, `RemovalTimer::default()`, `PendingRobotFx::default()`,
  `PendingRobotHits::default()`, `ShootRequested` iff `test_shoot` (`:138-141`); `Simulates` iff
  `initial.simulates` on all three (the field is in the variants, T004, as V3-B's `Initial::Player`);
  and in `EcsWorld::physics_process`
  `self.world.resource_mut::<Messages<RobotHitLocal>>().update()` right after `DoorBodyEntered`'s
  (`ecs.rs`, before the drain). Depends on T004, T005. Verification: compiles; `apply_register` is
  still the only writer of `NodeHandles.by_entity`.
- [x] T009 Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test` — zero
  warnings; **187** = 179 + 8 (apply); paste the `test result:` line. Headless import +
  `main/main.tscn` + `level/level.tscn` clean (nothing engine-facing changed; a sanity run). Commit
  (R11 row 1) with a body (gate line, headless result, deviations): `ecs + hittable: HitKind, enemy
  events/drain arms, RobotHitLocal message, components, Handles/Initial variants (tests)`.

**Checkpoint**: the ECS core carries the enemy's events, message, components, handles and drain;
`hittable.rs` has `HitKind`; no bridge changed yet.

---

## Phase 3: User Story 1 — commit 2: `bullet`

**Goal**: FR-005, FR-006, FR-007, FR-004 (bullet half). **Independent Test**: 192 tests with the 3
`pure` tests untouched; harness case (c) identical on `v2` and `v3`; the game plays as v2
(bullets kill v2 robots — the robot's `hit` is still v2's `call_local` handler, reached by name
through `HitKind::rpc_hit`).

### Commit 2 — `bullet/system.rs`, `bullet/sync.rs`, `bullet.rs`, `ecs.rs`/`ecs/setup.rs` (registration), `docs/v3-tradeoffs.md`

- [x] T010 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/bullet/system.rs` (pure; declare
  `pub(crate) mod system;` in `bullet.rs`; `mod pure` untouched): `pub fn bullet_step(dt:
  Res<FixedDelta>, q: Query<(&mut BulletStateC, &mut BulletIntents), With<Simulates>>)` — per
  entity: `intents = BulletIntents { active: state != Exploded, ..default }` (`bullet.rs:98-100`);
  when active `(state, expired) = pure::step(state, dt as f32)` (`:103-104`), `intents.explode =
  expired` (`:105-107`). `pub fn bullet_settle(q: Query<(&Collided, &mut BulletStateC, &mut
  BulletIntents), With<Simulates>>, w: MessageWriter<RobotHitLocal>)` — when `collided.collided`:
  `intents.hit = collided.hit` (`:117-121`), `intents.disable_collision = true` (`:122`),
  `intents.explode |= matches!(state, Flying { .. })` (backlog #13, `:123-127`), `state = Exploded`
  (`:130`); if `intents.hit.and_then(robot_id)` is `Some(id)` → `w.write(RobotHitLocal { robot:
  id })` (option (B), research R4). Tests (`run_system_once`, `World::new()`, `FixedDelta(1.0/60.0)`,
  `Messages::<RobotHitLocal>::default()`, one entity): `step_keeps_flying_and_counts_down`
  (`time_alive` 5.0 → 5.0 − dt, `active`, no explode), `expiry_sets_explode_intent`
  (`time_alive = 0.01` → `Exploded`, `explode`), `collision_sets_hit_disables_and_explodes`
  (`Collided { hit: Some(Robot(id)), collided: true }` → `hit`, `disable_collision`, `explode`,
  `Exploded`, ONE `RobotHitLocal` in the buffer), `expiry_and_collision_same_step_explode_once`
  (`bullet_step` with `time_alive = 0.01` then `bullet_settle` with a collision → `explode` true
  once, state `Exploded` — the settle sees `Exploded` and does not re-flag; assert by checking the
  intents after both), `exploded_bullet_is_inactive` (`Exploded` → `active = false`, no explode).
  Verification: 5 tests pass; no `godot::classes` import.
- [x] T011 [US1] Create `oxide_godot_core/oxide_godot_lib/src/bullet/sync.rs` (glue; `pub(crate)
  mod sync;`): `fn sync_in_bullet(handles: NonSend<NodeHandles>, q: Query<Entity, (With<BulletTag>,
  With<Simulates>)>, commands: Commands)` → `BulletBasisZ(p.root.get_transform().basis.col_c())`
  (`bullet.rs:112`); `fn move_bullet(dt: Res<FixedDelta>, handles: NonSendMut<NodeHandles>, q:
  Query<(Entity, &BulletBasisZ, &BulletIntents, &mut Collided), With<Simulates>>)`
  (`EngineQueryMove`): when `intents.active`: `col = p.root.move_and_collide(-dt * Bullet::VELOCITY *
  basis_z)` (`:111-113`); `collided.collided = col.is_some()`; `collided.hit =
  col.and_then(get_collider).and_then(try_cast::<Node3D>).and_then(hittable::kind_of)` (`:115-119`,
  research R3); else `Collided::default()`. `fn sync_out_bullet(handles: NonSendMut<NodeHandles>,
  q: Query<(Entity, &BulletIntents), With<Simulates>>)` (`SyncOut`, order-insensitive per the
  spec): if `intents.explode` → `p.anim.play_ex().name("explode").done()` and, when
  `p.shadow_mapping`, `p.light.set_shadow(true)` (`:139-145`); if `let Some(kind) = intents.hit` →
  `kind.rpc_hit()` (`:117-121`; the player's `call_local` handler pushes `AddTrauma`; the robot's
  handler is v2's `call_local` `hit` in THIS commit — flipped to `call_remote` in T021); if
  `intents.disable_collision` → `p.collision.set_disabled(true)` (`:122`); if `intents.explode` →
  `p.root.rpc("explode", &[])` (`call_remote` from T012 on). `fn sync_out_bullet_frame(handles:
  NonSendMut<NodeHandles>, q: Query<(Entity, &mut PendingBulletFx), Without<Simulates>>)` (frame
  `SyncOut`): drain → play `explode` + shadow (`:139-145`). Depends on T010, T008. Verification:
  compiles; `grep -n 'free()' bullet/sync.rs` empty.
- [x] T012 [US1] Rewrite `oxide_godot_core/oxide_godot_lib/src/bullet.rs` as the bridge
  (data-model.md "Bridges"): keep `pub(crate) mod pure` (body untouched), `VELOCITY` (`:83`), the
  three `OnReady` handles (`:71-76`) and the typed `Settings` (`:78-79`); DELETE the `state` field
  (`:68-69`) and `physics_process` (`:95-132`). `ready` (`:88-93`): non-server → `collision_shape.
  set_disabled(true)` (the `set_physics_process(false)` is moot — no callback — and is dropped);
  `let shadow_mapping = self.settings.bind().graphics().shadow_mapping;` (`:143`, read ONCE);
  `queue::push(Register { id, handles: Handles::Bullet(Box::new(BulletHandles { root:
  self.to_gd(), anim, collision, light, shadow_mapping })), initial: Initial::Bullet { .. } })`
  with `simulates = is_server()` (`:89`). `exit_tree` → `Unregister`. `#[rpc(authority,
  call_remote, unreliable)] fn explode(&mut self)` → `queue::push(BulletFx { root_id, fx: Explode })`
  (remote peers only, option (b)); `#[func] fn destroy(&mut self)` → `queue::push(BulletDestroy {
  root_id })` (the drain frees only on `Simulates`, `:150-153`). Depends on T011. Verification:
  `grep -n 'fn physics_process\|fn process' bullet.rs` empty; `grep -c call_remote bullet.rs` = 1;
  `git diff -- bullet.rs | grep '^-' | grep -c 'mod pure'` = 1 and the only `mod pure` change is the
  visibility token (T005).
- [x] T013 [US1] Registration: in `oxide_godot_core/oxide_godot_lib/src/ecs.rs`'s
  `add_engine_systems` (fixed) `bullet::sync::sync_in_bullet.in_set(Phase::SyncIn).after(sweep_dead_nodes)`,
  `move_bullet.in_set(Phase::EngineQueryMove)`, `sync_out_bullet.in_set(Phase::SyncOut).before(sync_out_remove)`;
  (frame) `sync_out_bullet_frame.in_set(Phase::SyncOut).before(sync_out_remove)`. In
  `ecs/setup.rs::build_fixed`: `bullet::system::bullet_step.in_set(Phase::Gameplay)`,
  `bullet_settle.in_set(Phase::GameplaySettle)`. Depends on T011. Verification: setup tests pass
  (the pure systems need `FixedDelta` and `Messages<RobotHitLocal>`, both in `build_world`).
- [x] T014 [US1] Append to `docs/v3-tradeoffs.md` the row: `bullet` — `move_and_collide` and the
  collider resolved into `HitKind` (`move_bullet`, `EngineQueryMove`; the collision answer is
  consumed by `bullet_settle`; `HitKind::rpc_hit` from `SyncOut`). Verification: 15 rows.
- [x] T015 [US1] Gates + headless + harness: `cargo build && cargo clippy && cargo test` — zero
  warnings; **192** = 187 + 5 (bullet/system); the 3 `pure` tests still listed by name; paste the
  `test result:` line. Headless: import (`Initialize godot-rust`), `main/main.tscn` ×2,
  `level/level.tscn` clean vs `CLAUDE.md`'s catalog. Harness (research R10, quickstart §3): write
  `zz_ecs_parity.gd` (from `contracts/zz_ecs_parity.gd`), the two-line `zz_ecs_observer.gd`, the
  three-line `zz_ecs_physics_probe.gd` (both quoted in the harness) and the six-line
  `zz_ecs_parity.tscn` (V3-A contract §4) into BOTH Godot project directories
  (`oxide-godot/oxide-godot/` and `../oxide-godot-v2/oxide-godot/` — NOT the repository roots);
  run case (c) bullet expiry with `--fixed-fps 60 --quit-after 450` under
  `XDG_DATA_HOME=/tmp/xdg-v2` and `/tmp/xdg-v3`; logs at `"$XDG_DATA_HOME/godot/app_userdata/
  Third-Person Shooter Demo/zz_ecs_parity_c.log"`; `diff` the full logs AND the `^RAW` subsets.
  Expected: identical (the explode animation start ≈ step 310, the free frame ≈ 1.5 s later).
  **Paste the actual diff output in the completion note — never write "identical" without the
  diff.** Any difference → the spec's timing-table candidates (T029) with its cause, or a fix
  before committing if it is not a timing shift. Delete `zz_*` (and `.uid`s) from both trees.
  Commit (R11 row 2) with a body (gate, headless, the diff, deviations, and the sentence "the
  game is playable: bullets hit v2 robots through `HitKind::rpc_hit` → the robot's v2 `call_local`
  `hit`; `RobotHitLocal` has no consumer yet and is dropped one run later, research R4 (3)"):
  `bullet: bridge over the ECS core — fixed tick with move_and_collide in EngineQueryMove, HitKind,
  explode call_remote`.

**Checkpoint**: the bullet is an entity; v2 robots still take its hits; the part and the robot
follow in commit 3.

---

## Phase 4: User Story 2 + User Story 3 + User Story 4 — commit 3: `part` + `red_robot`, the `.tscn` edit

**Goal**: FR-008…FR-023 (backlog #31 closed by FR-023), FR-004 (robot and part halves). One
commit: the robot's death explodes the parts directly and `Part::explode` is removed, so the two
cannot be split. **Independent Test**: 214 tests with the 5 + 25 pure tests untouched; harness
(a), (b), (d), (e) identical except (a)'s sanctioned #31 difference; both visual checkpoints.

### Commit 3 — `part/system.rs`, `red_robot/system.rs`, `part/sync.rs`, `red_robot/sync.rs`, `part.rs`, `red_robot.rs`, `ecs.rs`/`ecs/setup.rs`, `red_robot.tscn:10782`, `docs/v3-tradeoffs.md`, `docs/v2-backlog.md`

- [x] T016 [P] [US2] Create `oxide_godot_core/oxide_godot_lib/src/part/system.rs` (pure;
  `pub(crate) mod system;` in `part.rs`; `mod pure` made `pub(crate)`, body untouched): `pub fn
  part_phase_tick(dt: Res<FrameDelta>, q: Query<(Entity, &mut PartPhase, &PartLifetimes, &mut
  PartIntents)>, commands: Commands)` — `intents = default` each frame; `Waiting(timer)`:
  `timer.step(dt)` → `Fading { counter: 0.0 }` (`part.rs:179-191`); `Fading { counter }`:
  `intents.fade = Some(fade_curve(counter, disappearing_time))` (`:139`), `counter += dt as f32`
  (`:141`), `should_destroy(counter, disappearing_time)` → `intents.destroy = true`, phase
  `Destroyed(Timer::new(0.2))` (`:142-144`, `:203-209`); `Destroyed(timer)`: `timer.step(dt)` →
  `commands.entity(e).insert(Remove)` (`:210-214`); `Attached`: nothing. Tests (`run_system_once`,
  `FrameDelta(1.0/60.0)`, `PartLifetimes { 3.0, 3.0, 0.5 }`): `attached_part_does_nothing`,
  `waiting_timer_expiry_starts_fading` (`Waiting(Timer::new(dt))` → one run → `Fading { counter:
  0.0 }`), `fading_writes_fade_curve_each_frame` (`counter = 0.25` → `fade == fade_curve(0.25,
  0.5)`, counter advanced), `fading_destroys_at_t_minus_0_2_once` (`counter = 0.29` → after one run
  `destroy` and `Destroyed(_)`; a second run leaves `destroy = false`),
  `destroyed_timer_expiry_marks_remove` (`Destroyed(Timer::new(dt))` → `Remove` present).
  Verification: 5 tests pass; no `godot::classes`.
- [x] T017 [P] [US3] Create `oxide_godot_core/oxide_godot_lib/src/red_robot/system.rs` (pure;
  `pub(crate) mod system;` in `red_robot.rs`; `model.rs` untouched): `pub fn robot_decide(tuning,
  dt, q: Query<(Entity, &RobotFrame, &RobotState, &RobotCountersC, &TrackedPlayer, &mut
  TargetPosition, &mut RobotIntents, Has<ShootRequested>), (With<Simulates>, Without<Dead>)>,
  commands: Commands)` — `intents = default`; `shoot = has ShootRequested` (marker removed via
  `Commands`, `red_robot.rs:138-141`); `TrackedPlayer == None` → `target = ZERO`, `idle_branch =
  true`, return (`:143-151`); else `target = frame.player_origin` (`:153`); `Approach`: `local =
  gt.basis.transposed() * (target − gt.origin)`, `angle = angle_to_player(local)` (`:164-166`);
  `facing(angle, tol) && shoot_countdown_will_expire(counters, dt)` → `raycast =
  Some((frame.ray_from.origin, target + UP))` (`:177-183`); `Aim | Shooting`: `max_dist = if
  frame.laser_colliding { (frame.ray_from.origin − frame.laser_point).length() } else { 1000.0 }`
  → `clip = Some(max_dist)` (`:199-205`; a constant 1000 with the disabled ray, R8); `Aim &&
  aim_countdown_will_expire` → `raycast = Some((ray_from.origin, target + UP))` (`:216-222`).
  `pub fn robot_step_and_animate(tuning, dt, q: Query<(&RobotFrame, &RaycastAnswers, &mut
  RobotState, &mut RobotCountersC, &TargetPosition, &mut AimBlend, &mut RobotIntents, &RootMotion,
  &mut Orientation, &mut Velocity), (With<Simulates>, Without<Dead>)>)` (a `type` alias) — unless
  `idle_branch`: `inputs = RobotInputs { angle_to_player: Some(angle) if Approach else None,
  sees_player: answers.sees_player }`, `(state, cmds) = model::step(state_at_start, &mut counters,
  dt, &inputs, &tuning)` (`:186-196`, `:225-235`); `cmds`: `RpcPlayShoot` → `play_shoot = true`,
  `ResumeApproach` → nothing further (`step` applied it); THEN the animation decision on the NEW
  state/counters (v2 `animate(delta)` at `:238`): `request = transition_request(state, angle if
  Approach, target == ZERO, &tuning)`; if `target != ZERO`: `blend_amount = aim_blend_amount(
  aim_preparing)`, `(h, v) = cannon_angles(ray_mesh.basis.transposed() * (target + UP −
  ray_mesh.origin))`, `AimBlend = aim_blend_step(AimBlend, h, v, dt, &tuning)` → `anim = Some(
  AnimDecision { request, aim: Some((blend_amount, AimBlend)) })` else `aim: None` (`:456-490`);
  THEN `(Orientation, Velocity) = integrate_root_motion(orientation, root_motion, dt,
  frame.gravity, frame.velocity)` (`:245-251`); on `idle_branch`: `Velocity = idle_velocity(
  frame.gravity, dt)` (`:146`) and the animation decision only (v2 `:145`). `pub fn robot_replay(
  tuning, dt, q: Query<(&ReplayRobot, &RobotFrame, &mut AimBlend, &mut RobotIntents),
  (Without<Simulates>, Without<Dead>)>)` — the same animation decision from the replicated
  `state`/`target_position`/`aim_preparing` (`:134` → `:456-490`). `pub fn robot_hit_apply(r:
  MessageReader<RobotHitLocal>, index: Res<EntityIndex>, q: Query<(&mut Health, &mut RobotIntents,
  Has<Dead>), With<RobotTag>>)` — per message: `index.entity(robot)` → if `Dead` skip (`:278-280`);
  `(health, just_died) = hit_step(health)` (`:289-290`); `intents.hit = true`, `intents.just_died
  |= just_died`. `pub fn robot_timers(dt: Res<FrameDelta>, q: Query<(Entity, &mut PendingTrauma,
  &mut RemovalTimer)>, commands: Commands)` — `PendingTrauma(Some((timer, player)))`: `step(dt)` →
  `TraumaDue(player)` inserted, slot cleared (`:437-453`); `RemovalTimer(Some(timer))`: `step(dt)`
  → `Remove` (`:309-326`). Tests (14, `run_system_once`, `Tuning(RobotTuning::default())`,
  `FixedDelta`/`FrameDelta(1.0/60.0)`): `no_player_branch_uses_idle_velocity_and_zero_target`,
  `approach_requests_raycast_only_when_facing_and_countdown_expiring` (facing + `shoot_countdown =
  0.01` → `raycast`; facing + 2.0 → none; not facing + 0.01 → none),
  `approach_to_aim_when_raycast_sees_player` (`sees_player = Some(true)`, countdown expiring →
  `Aim`, `aim_countdown == aim_time`, `aim_preparing == 0`), `aim_branch_clips_at_1000_when_laser_
  not_colliding`, `aim_to_shooting_emits_play_shoot` (`Aim`, `aim_countdown = 0.01`, `Some(true)`
  → `Shooting`, `play_shoot`), `aim_lost_resumes_approach` (`Some(false)` → `Approach`, counters
  reset), `animation_decided_from_the_post_step_state` (a robot leaving `Approach` this step gets
  `request == "idle"`, not `"walk"`), `aim_blend_steps_from_the_component` (two runs → the second
  starts from the first's `AimBlend`), `integrate_matches_model_twin` (equals a direct
  `integrate_root_motion` call), `shoot_requested_sets_shoot_intent_once` (marker → `shoot` and the
  marker gone; second run no shoot), `replay_builds_animation_from_replicated_fields`,
  `robot_hit_apply_ignores_dead_robot`, `pending_trauma_expiry_flags_trauma_due`,
  `removal_timer_expiry_marks_remove`. Verification: 14 tests pass; no `godot::classes`.
- [x] T018 [US2] Create `oxide_godot_core/oxide_godot_lib/src/part/sync.rs` (glue; `pub(crate)
  mod sync;`): `pub(crate) fn puff_parent(root: &Gd<Part>) -> Gd<Node>` (v2 `part.rs:224-232`
  verbatim on the handle, backlog #15's comment moved with it); `fn sync_out_part(handles:
  NonSendMut<NodeHandles>, q: Query<(Entity, &PartIntents, &mut PendingPartFx, &mut PartPhase,
  Has<Simulates>), With<PartTag>>)` (frame `SyncOut`): on `Simulates`: if `Some(fade) =
  intents.fade` → `{ p.root.bind_mut().set_fade_value(fade); }` (`:140` → `:151-160`, guard
  dropped); if `intents.destroy` → the puff: `puff = p.puff_scene.instantiate_as::<CpuParticles3D>()`,
  `puff_parent(&p.root).add_child(&puff)`, `puff.set_global_position(p.root.get_global_transform().origin)`
  (`:196-200`), then `p.root.rpc("destroy", &[])` (`call_remote`); on non-`Simulates`: drain
  `PendingPartFx` — `Destroy` → the same puff writes and `*phase = Destroyed(Timer::new(0.2))`
  (`:202-214`; the client frees its own node through `Remove`, research R7). Depends on T016,
  T008. Verification: compiles; `grep -n 'free()' part/sync.rs` empty.
- [x] T019 [US3] Create `oxide_godot_core/oxide_godot_lib/src/red_robot/sync.rs` (glue;
  `pub(crate) mod sync;`): `fn sync_in_robot(handles: NonSend<NodeHandles>, q: Query<(Entity,
  &TrackedPlayer, Has<Simulates>), (With<RobotTag>, Without<Dead>)>, commands: Commands)` —
  `RobotFrame { global_transform: p.root.get_global_transform(), gravity: p.root.get_gravity(),
  velocity: p.root.get_velocity(), player_origin: tracked.0.and_then(|id| Gd::<Node3D>::
  try_from_instance_id(id).ok()).map(|n| n.get_global_transform().origin) (research R6's path),
  ray_from: p.ray_from.get_global_transform(), ray_mesh: p.ray_mesh.get_global_transform(),
  ray_mesh_z: p.ray_mesh.get_position().z, laser_colliding: p.laser_raycast.is_colliding(),
  laser_point: p.laser_raycast.get_collision_point() }` (`red_robot.rs:153`, `:164`, `:180`,
  `:199-202`, `:249-250`, `:478`, `:493`; the laser reads are kept although the ray is disabled,
  R8); non-`Simulates`: `ReplayRobot { state, target_position, aim_preparing }` from
  `p.root.bind()` (`:134`). `fn robot_query(handles: NonSendMut<NodeHandles>, q: Query<(Entity,
  &RobotIntents, &TrackedPlayer, &mut RaycastAnswers, &mut RootMotion), (With<Simulates>,
  Without<Dead>)>)` (`EngineQueryOrient`): `answers = default`; `raycast = Some((from, to))` →
  `raycast_to(from, to)` (v2 `:363-382` moved here as a free fn taking `&Gd<EnemyRobot>` and the
  RID: `PhysicsRayQueryParameters3D::create_ex(from, to).collision_mask(0xFFFFFFFF).exclude(&array![
  p.rid]).done().unwrap()`, `get_world_3d().get_direct_space_state().intersect_ray`) →
  `sees_player = Some(hit.collider id == tracked id)` (`:506-511`); `shoot` → the shoot raycast
  along `ray_from.basis.col_b()` for 1000 m (`:399-409`) → `shot = Some(ShotResult { max_dist,
  hit: Some((position, collider id)) | None })`; unless `idle_branch`: `RootMotion = Transform3D::
  new(Basis::from_quaternion(p.anim_tree.get_root_motion_rotation()), p.anim_tree.get_root_motion_
  position())` (`:241-244`). `fn move_robot(handles, q: Query<(Entity, &Velocity),
  (With<Simulates>, Without<Dead>)>)` (`EngineQueryMove`): `set_velocity`, `set_up_direction(UP)`,
  `move_and_slide` (`:147-149`, `:253-255`). `fn sync_out_robot(dt: Res<FixedDelta>, index:
  Res<EntityIndex>, handles: NonSendMut<NodeHandles>, robots: RobotSyncOutQuery, parts:
  Query<(&mut PartPhase, &PartLifetimes), (With<PartTag>, Without<RobotTag>)>, commands:
  Commands)` (a `type` alias for the robot query: `(Entity, &Orientation, &RobotState,
  &TargetPosition, &Health, &RobotCountersC, &RobotIntents, &RaycastAnswers, &TrackedPlayer, &mut
  PendingTrauma, &mut RemovalTimer, Has<Simulates>, Has<Dead>), With<RobotTag>`) — per entity, in
  THIS order (spec scenario 5, research R5/R6): on `Simulates` && !`Dead`: unless `idle_branch`
  `p.root.set_global_basis(orientation.basis)` (`:257-258`);
  `{ let mut g = p.root.bind_mut(); g.state = ..; g.target_position = ..; g.health = ..;
  g.aim_preparing = counters.aim_preparing; }` (guard dropped); if `play_shoot` →
  `p.shoot_anim.play_ex().name("shoot").done()` (`:331-332`) then `p.root.rpc("play_shoot", &[])`
  (`call_remote`); if `Some(shot) = answers.shot` → `clip_ray(&mut p, shot.max_dist)` (the v2
  `_clip_ray` `:492-501` as a free fn: skip when `p.is_dedicated_server`), `p.laser_ember.set_position(
  ember_position(max_dist, ray_mesh_z))`, `set_emission_box_extents(ember_extents(..))`
  (`:415-419`), if `Some((pos, collider))` → `blast = p.impact_effect_scene.instantiate_as::<Node3D>()`,
  `p.root.get_tree().unwrap().get_root().unwrap().add_child(&blast)`, `blast.set_global_position(pos)`
  (`:423-425`), and if `collider == tracked.0` → `PendingTrauma = Some((Timer::new(trauma_delay),
  player_id))` (`:437-453`); if `Some(max_dist) = intents.clip` → `clip_ray` (`:205`); if
  `intents.hit` → `param = format!("parameters/hit{}/request", randi() % 3 + 1)`,
  `p.anim_tree.set(&param, &1.to_variant())`, `p.hit_sound.play()` (`:285-287`, the `randi()` in
  glue, BEFORE any `randf()`); if `intents.just_died` → the death branch (`:293-326`): `{
  p.root.bind_mut().dead = true; }`, `p.anim_tree.set_active(false)`, `p.model.set_visible(false)`,
  `p.death.set_visible(true)`, `p.collision_shape.set_disabled(true)`, both `sparks[i].set_emitting(true)`;
  then for each `parts[i]` in order (`PartShield1`, `PartShield2`, `PartHead`): `part_id =
  parts[i].instance_id()`, `part_entity = index.entity(part_id)`, `lifetimes = parts.get(part_entity)`,
  `angular = random_angular_velocity(randf() as f32, randf() as f32, randf() as f32)`, `wait =
  wait_time(lifetime, lifetime_random, randf() as f32)` (`part.rs:173-175`, twelve draws total),
  `part_handles = handles[part_entity]` (`Handles::Part(q)`): `q.synchronizer.set_visibility_public(true)`,
  `parts[i].set_freeze_enabled(false)`, `q.col1/col2.set_disabled(false)`, `parts[i].set_linear_velocity(3.0
  * UP)`, `parts[i].set_angular_velocity(angular)` (`:164-174`), `*phase = Waiting(Timer::new(wait))`
  (`:175`, `:179-191`) — clone the three `Gd<Part>` handles out of the robot's entry BEFORE the loop
  so the map is free for the parts' lookups (research R5); then `p.explosion_sound.play()`,
  `p.root.signals().exploded().emit()` (`:306-307`), `RemovalTimer = Some(Timer::new(removal_delay))`
  (`:309-326`, server only — `Simulates`), `commands.entity(e).insert(Dead)`; for EVERY !`Dead` robot
  (`Simulates` or not): if `Some(anim) = intents.anim` → `p.anim_tree.set("parameters/state/
  transition_request", request)`, and if `Some((amount, blend))` → `set("parameters/aiming/blend_amount",
  amount)`, `set("parameters/aim/blend_position", blend)` (`:469-488`); LAST: `p.anim_tree.advance(dt.0)`
  (research R1, option B) — SKIPPED when `intents.just_died` fired in this run (spec FR-022 as
  amended): v2 set the tree inactive at death (`:294`), and `AnimationMixer::advance()` does NOT
  check `active` (Godot 4.7 `scene/animation/animation_mixer.cpp:2112-2114` calls
  `_process_animation` unconditionally; `AnimationTree::_blend_pre_process` neither — verified
  against the fetched 4.7 sources on 2026-09-19); whether v2's tree processed that exact step
  depended on tree order (robot before bullet: it did not), so the skip is the simpler,
  harmless choice (the model is hidden and the tree never read again). Test in
  `red_robot/system.rs`: `robot_that_just_died_is_not_advanced` asserting the pure gate
  `should_advance(dead: bool, just_died: bool) -> bool` (`!dead && !just_died`) — the fifteenth
  test of that file. `fn sync_out_robot_frame(handles: NonSendMut<NodeHandles>, q:
  Query<(Entity, Option<&TraumaDue>, &mut PendingRobotFx, &mut PendingRobotHits, Has<Simulates>),
  With<RobotTag>>, commands: Commands)` (frame `SyncOut`): `TraumaDue(player)` → `queue::push(
  AddTrauma { root_id: player, amount: trauma_amount })` and remove the marker (v2 `:452`; the
  player entity's V3-B arm applies it at the next drain); non-`Simulates` (analyze BLOCKER 1:
  `health`/`dead` are spawn-only, `red_robot.tscn:31-33`/`:40-42` — the client keeps its OWN
  count, as v2's per-peer `call_local` handler did, `:278-307`): for each pending hit
  (`PendingRobotHits` → 0), on a robot that is not `Dead`: `hit_step` on the client's `Health`
  (`:289-290`, the pure fn — the query carries `&mut Health`), the reaction `randi()` + parameter
  + `hit_sound.play()` (`:285-287`), and on `just_died`: `{ p.root.bind_mut().dead = true; }`,
  `p.anim_tree.set_active(false)`, model hidden, `Death` visible, collision off, sparks (`:293-300`),
  the parts' `set_visibility_public(true)` + `set_freeze_enabled(false)` (the every-peer half of
  `part.rs:164-166`, no velocities `:167-169`), `p.explosion_sound.play()` and
  `p.root.signals().exploded().emit()` (`:306-307`, every peer in v2; no client listener today),
  and `commands.entity(e).insert(Dead)` — no `DeathVisualsApplied` marker: `Dead` gates it;
  `PendingRobotFx` → `PlayShoot` → `shoot_anim.play("shoot")`. Test (`red_robot/system.rs`, pure
  half): `remote_hit_decrements_client_health_and_dies_at_zero` (five hits on `Health(5)` on a
  non-`Simulates` entity → `Health(0)` and the death intent exactly once; a sixth is ignored) —
  the sixteenth test of that file. Depends on T017, T008. Verification: compiles; `grep -n
  'free()' red_robot/sync.rs` empty; `grep -c 'randf()' red_robot/sync.rs` = 4 (the per-part
  draws) and `grep -c 'randi()' red_robot/sync.rs` = 2 (local + remote reaction).
- [x] T020 [US2] Rewrite `oxide_godot_core/oxide_godot_lib/src/part.rs` as the bridge: keep
  `pub(crate) mod pure` (body untouched), the four `#[export]`s (`:86-97`), the handles
  (`:101-118`, the model-mesh-by-index comment kept), `material`; `ready` = v2 `:123-136` minus
  `set_process(false)` — the material duplication with the upstream bug fix #2 comment (`:129-130`)
  VERBATIM — then `queue::push(Register { id, handles: Handles::Part(Box::new(PartHandles { root:
  self.to_gd(), synchronizer, col1, col2, puff_scene: self.part_disappear_scene.clone() })),
  initial: Initial::Part { lifetime, lifetime_random, disappearing_time, simulates: is_server() } })`;
  `exit_tree` → `Unregister`. `#[func] pub(crate) fn set_fade_value(&mut self, value: f32)` — body
  unchanged (`:151-160`), visibility `pub(crate)` for `sync_out_part`; `#[rpc(authority,
  call_remote, unreliable)] fn destroy(&mut self)` → `queue::push(PartFx { root_id, fx: Destroy })`;
  DELETE `disappearing_counter` (`:99`), `process` (`:138-146`), `explode` (`:162-192`, no caller —
  Complexity Tracking), the private `puff_parent` (`:224-232`, moved to `part/sync.rs`). Depends on
  T018. Verification: `grep -n 'fn process\|fn explode\|godot::task' part.rs` empty; `grep -c
  call_remote part.rs` = 1; the four `#[export]` attributes unchanged; the bug-fix comment present.
- [x] T021 [US3] Rewrite `oxide_godot_core/oxide_godot_lib/src/red_robot.rs` as the bridge: keep
  `State` (`:21-28`), `#[var] test_shoot`, the four replicated `#[export]`s, `#[var] aim_preparing`
  (`:35-50`; all now projection fields, `pub(crate)`), `is_dedicated_server` (`:62-63`), the RID,
  `impact_effect_scene` (`:68-69`), the twelve `OnReady` handles (`:71-105`); DELETE `shoot_countdown`,
  `aim_countdown`, `player`, `orientation`, `RayHit`, `physics_process` (`:128-259`), `raycast_to`,
  `apply_cmds`, `shoot`, `animate`, `_clip_ray`, `hits_player` (`:360-511`, moved to `red_robot/sync.rs`
  and `red_robot/system.rs`). `ready` (`:110-126`): `orientation` with zero origin, `animation_tree.
  set_active(true)`, `rid = get_rid()`, the `dead` handling (`:119-123`), NO `animate(0.0)` (research
  R1), `aim_blend = self.animation_tree.get("parameters/aim/blend_position").to::<Vector2>()`
  (`red_robot.tscn:10785`), then `queue::push(Register { id, handles: Handles::Robot(Box::new(
  RobotHandles { root: self.to_gd(), …, parts: [death_shield1, death_shield2, death_head].map(clone),
  sparks: [..], impact_effect_scene, rid, is_dedicated_server })), initial: Initial::Robot { state,
  health, dead, test_shoot, orientation, aim_blend, simulates: is_server() } })`; `exit_tree` →
  `Unregister`. `#[signal] pub(crate) fn exploded();` kept (emitted by `sync_out_robot`). `#[func]
  fn resume_approach(&mut self)` → `queue::push(ResumeApproachRequested { root_id })` (`:267-274`);
  `#[rpc(authority, call_remote, unreliable)] fn hit(&mut self)` → `queue::push(RobotHit { root_id })`
  (`:276-328`, remote peers only — option (B)); `#[rpc(authority, call_remote, unreliable)] fn
  play_shoot(&mut self)` → `queue::push(RobotFx { root_id, fx: PlayShoot })` (`:330-333`); `#[func]
  fn shoot_check(&mut self)` → `queue::push(ShootRequested { root_id })` (`:335-338`; the
  `test_shoot` field is no longer written); `#[func] fn _on_area_body_entered(&mut self, body:
  Gd<Node3D>)` → `if let Ok(player) = body.try_cast::<Player>() { queue::push(RobotPlayerSeen {
  root_id, player: Some(player.instance_id()) }) }` (`:340-349`); `_on_area_body_exited` → `Some`
  → `player: None` (`:351-357`). Depends on T019. Verification: `grep -n 'fn physics_process\|fn
  process\|fn animate\|fn shoot\b\|_clip_ray\|raycast_to\|godot::task' red_robot.rs` empty; `grep -c
  call_remote red_robot.rs` = 2; `grep -c call_local red_robot.rs` = 0; `hittable.rs`, `level.rs`
  still compile unchanged.
- [x] T022 [US3] Registration + the R4 test: in `ecs.rs`'s `add_engine_systems` (fixed)
  `red_robot::sync::sync_in_robot.in_set(Phase::SyncIn).after(sweep_dead_nodes)`,
  `robot_query.in_set(Phase::EngineQueryOrient)`, `move_robot.in_set(Phase::EngineQueryMove)
  .before(crate::bullet::sync::move_bullet)` (analyze MAJOR 2: in v2 the robots — spawned under
  `SpawnedNodes` at level start, `level.rs:138` — precede the bullets in tree order, so the robot's
  `move_and_slide` ran before the bullet's `move_and_collide` every step and the bullet collided
  with the robot's POST-move body; bevy leaves two systems in one set unordered, so the order is
  pinned; v2's exception — a robot respawned after a bullet already existed — is not reproduced,
  recorded in the tradeoffs row),
  `sync_out_robot.in_set(Phase::SyncOut).before(sync_out_remove)`; (frame)
  `part::sync::sync_out_part.in_set(Phase::SyncOut).before(sync_out_remove)`,
  `red_robot::sync::sync_out_robot_frame.in_set(Phase::SyncOut).before(sync_out_remove)`. In
  `ecs/setup.rs`: `build_fixed` adds `red_robot::system::robot_decide.in_set(Phase::Gameplay)`,
  `robot_step_and_animate.in_set(Phase::GameplayIntegrate)`, `robot_replay.in_set(Phase::GameplayIntegrate)`,
  `robot_hit_apply.in_set(Phase::GameplaySettle).after(bullet::system::bullet_settle)`;
  `build_frame` adds `part::system::part_phase_tick.in_set(Phase::Gameplay)`,
  `red_robot::system::robot_timers.in_set(Phase::Gameplay)`. Test in `ecs/setup.rs`:
  `robot_hit_apply_runs_after_bullet_settle_in_the_same_run` (research R4: `build_world()` +
  `build_fixed()`; a bullet entity `Collided { hit: Some(HitKind::Robot(id)), collided: true }` +
  `BulletStateC(Flying)` + `BulletIntents::default()` + `Simulates`; a robot entity `Health(1)` +
  `RobotIntents::default()` + `RobotTag` registered under `id` in `EntityIndex`;
  `Messages::update()`; ONE `run` → `Health == 0`, `hit && just_died`; then `Collided::default()`,
  `update()`, a second run → `Health` still 0). Depends on T019, T018. Verification: the test and
  V3-A/V3-B's setup tests pass.
- [x] T023 [US4] The `.tscn` edit, then research R1's probe re-run on the REAL build: edit
  `oxide-godot/enemies/red_robot/red_robot.tscn` line 10782 from `callback_mode_process = 0` to
  `callback_mode_process = 2` (the `AnimationTree` → MANUAL; research R1, option B; the
  milestone's ONE scene edit) — the edit MUST precede the run, because with the ECS robot of
  T019–T022 the tick's own `advance` is the only one allowed (a PHYSICS-mode tree would process
  twice). Then copy `contracts/zz_r1_robot.gd` (with its quoted `zz_r1_min.gd`/`zz_r1_max.gd` and
  the tscn) into both Godot project directories and run `ZZ_PZ=8 ZZ_MODE=physics --quit-after 305`
  on BOTH trees (`v2`: PHYSICS tree + v2 tick; `v3`: MANUAL tree + the ECS tick's `advance`; the
  probe's `manual` mode is NOT used on v3 — it would add a second `advance`); `diff` the two logs
  (expected: identical, 601 lines) and record the `S12`/`M12`/`S13` lines and the diff result in
  the commit body. Delete the probe files from both trees. Verification: `git diff
  oxide-godot/enemies/red_robot/red_robot.tscn` shows exactly that one changed line; the diff of
  the two probe logs is empty.
- [x] T024 [US3] Append to `docs/v3-tradeoffs.md` the rows (contracts/enemy-entities.md §5):
  `part` — the `RigidBody3D` simulated and replicated by the engine, the fade through the node's
  setter (`sync_out_part`); the puff instanced from the part's `SyncOut` and the remote handler;
  `red_robot` — the raycasts in `EngineQueryOrient` (`robot_query`); the laser `RayCast3D` read at
  `SyncIn` (disabled in the scene — a constant; research R8's ordering rule); the second
  `AnimationTree` MANUAL + `advance` with R1's `M(n).rm == S(n+1).rm` evidence and `AimBlend`
  replacing the tree `get`; engine RNG draws in glue (the thirteen draws of the death branch, R5);
  the `exploded` signal emitted from glue for v2's `level.rs`; the blast instanced under the tree
  root; the four `call_remote` attributes + the same-run `RobotHitLocal` message (option (B)) and
  `Part::explode` removed. Mark backlog **#31 done** in `docs/v2-backlog.md` citing spec FR-023
  and this commit (the drain resets the counters on detection-area entry). Verification: 23
  rows; `grep -n '^| 31 ' docs/v2-backlog.md` shows the done marker.
- [x] T025 [US3] Gates + headless + harness: `cargo build && cargo clippy && cargo test` — zero
  warnings; **214** = 192 + 5 (part/system) + 16 (red_robot/system) + 1 (setup); the 5 + 25 pure
  tests still listed by name; paste the `test result:` line. Headless: import, `main/main.tscn`
  ×2, `level/level.tscn` clean vs `CLAUDE.md`'s catalog. Harness on BOTH trees as T015 (the four
  scratch files into both Godot project directories): cases (a) `--quit-after 900`, (b) 900, (d)
  200, (e) 305; `diff` full and `^RAW` per case. Expected: (b), (d), (e) identical — (b) compares
  the parts' positions bit-for-bit (research R2), the death step, the three `angular` RAW lines,
  the puff and free frames, the robot's free step; (a) identical up to the robot's FIRST exit from
  the detection area (or entirely, if the player never leaves it — the harness player is static,
  so (a) is EXPECTED identical; the #31 difference shows only on re-entry after `Aim`/`Shooting`).
  How to tell #31 from a bug: before the first `RAW P<n> state=` transition back to `Approach`
  after a `state=2/3`, every line must be identical; after such a re-entry the ONLY allowed
  difference is the step of the next pre-check raycast/transition (v3 fires it `shoot_wait` s after
  re-entry, v2 on the stale countdown) with `aim_preparing` reset to `aim_prepare_time` — record
  it in the timing table (T029) as the sanctioned change. Any other difference (a wrong `state`,
  `target_position`, origin, `aim/blend_position`, a part position, a missing RAW stamp) is a bug
  to fix before committing. **Paste the actual diff output in the completion note — never write
  "identical" without the diff.** Delete `zz_*` (and `.uid`s) from both trees. Commit (R11 row 3)
  with a body (gate, headless, the four diffs, the R1 re-run lines of T023, deviations): `part +
  red_robot: bridges, part phase machine on the frame schedule, robot seven-set tick with same-run
  hit (RobotHitLocal), robot AnimationTree MANUAL + advance (R1 option B)`.
- [x] T026 [US3] 🛑 **STOP 1 — user visual checkpoint (1), single player** (spec SC-005, FR-031).
  Tell the user explicitly: "Checkpoint (1): please test now" — a robot approaches, aims and fires
  the laser (clip, ember, blast); being hit by the laser (camera shake after 0.1 s); killing a
  robot (hit reactions and sound per hit, parts fly and fade, puffs, the robot gone after 10 s and
  respawned by `level`); bullets exploding on walls and on expiry — all as in v2; ask whether
  backlog #29's FPS observation persists and record the answer. Do not proceed until confirmed.
- [x] T027 [US3] 🛑 **STOP 2 — user visual checkpoint (2), multiplayer on one machine**. Tell the
  user explicitly: "Checkpoint (2): please test multiplayer now" — host in one instance, join
  from a second through the demo's menu; the client sees the host's robots move, aim and shoot and
  take hit reactions; parts fly, fade and puff on the client; a client's bullet kills a robot and
  the death replicates (parts, puffs, removal after 10 s); the client's own laser hit shakes its
  camera. Record exactly what the user reports (a remote robot frozen in one pose = the projection
  is not written; parts not moving on the client = the every-peer unfreeze half missing; effects
  missing on the observer = `RobotFx`/`RobotHit`/`PartFx` not reaching the frame appliers). Do not
  start Phase 5 before BOTH checkpoints are confirmed.

**Checkpoint**: User Stories 1–4 complete and independently verified.

---

## Phase 5: Polish — commit 4

- [x] T028 [P] Extend `CLAUDE.md`'s "Port conventions (v3)" with: entity-to-entity communication
  — inside ONE run through `Messages` ordered across sets (`RobotHitLocal`: `bullet_settle` →
  `robot_hit_apply.after(..)` in `GameplaySettle`, `update()` at the start of each fixed run) versus
  across runs through the queue (`AddTrauma` from a timer), and the corollary of option (b): an RPC
  whose local effects must land in the SAME run as their cause is applied through a message, never
  the queue; cross-entity handle and component access from a `SyncOut` system (the robot exploding
  its parts: `EntityIndex` + the parts' `NodeHandles` entries + `Query<&mut PartPhase, Without<RobotTag>>`,
  the robot's part handles cloned before the loop); the engine-RNG-in-glue rule (the thirteen draws
  in v2's order; case (b)'s `angular` RAW lines as the proof); timers that push events on expiry
  (`TraumaDue` → `AddTrauma`); research R8's engine-updated-children rule (a live `RayCast3D` or a
  PHYSICS-mode `AnimationTree` updates after the parent's callback and before the driver: a
  `SyncIn` read at `i32::MAX` is one step newer than v2's — drive the child from the tick or keep a
  one-step buffer; the laser needs neither: `enabled = false`); the harness rules
  `is_instance_valid` BEFORE reading a part and "the Godot project directory, not the repository
  root" for the scratch files; the three new probe scripts as re-verification tools.
  Verification: `grep -c 'RobotHitLocal' CLAUDE.md` ≥ 1.
- [x] T029 [P] Fill `specs/013-v3-bullet-part-robot/spec.md`'s "Measured timing differences" table
  from the five diffs of T015/T025: one row per differing frame with cause — the #31 rows (if case
  (a) showed a re-entry) cite FR-023 as the sanctioned change; the empty cases are recorded below
  the table with the commit hashes where their diffs are pasted; no `(to be measured)` left. Align
  `data-model.md` "Tests by name" with the names that landed (`grep -A1 '#\[test\]'` per file) and
  its signatures with the code where T019/T022 refined them. Verification: `grep -c '(to be
  measured)' spec.md` = 0.
- [x] T030 Run quickstart.md §5's grep list VERBATIM and paste every output (no `fn process`/
  `fn physics_process` in the three bridges; no `godot::task::spawn`/`bind_mut::<EcsWorld>`/
  `get_autoload_by_name::<EcsWorld>` in the three modules; no `godot::classes` in the three
  `system.rs`; `red_robot/model.rs` diff-empty against `30d1a4d`; the two inline `mod pure` diffs
  are the visibility token only; `hittable.rs` additions only — zero removed lines; the excluded
  modules diff-empty by EXPLICIT paths (`player.rs player/ player_input.rs player_input/
  camera_noise_shake.rs camera_noise_shake/ door.rs door/ part_disappear.rs part_disappear/ blast.rs
  level.rs flying_forklift.rs settings.rs settings/ menu.rs menu/ main_scene.rs debug_label.rs`);
  `red_robot.tscn` diff = the one line and no other scene changed; tradeoffs ≥ 23; backlog #29/#30
  open and #31 done). If a grep reveals a violation, STOP and report.
- [x] T031 Final gates + headless (`cargo build && cargo clippy && cargo test` → **214**; import;
  `main.tscn`; `level.tscn`); commit (R11 row 4) with a body (gate line, the grep outputs summary,
  the timing-table rows as written): `CLAUDE.md: entity-to-entity messages vs queue,
  engine-updated children rule, RNG in glue; spec: timing table; tradeoffs complete`.
- [ ] T032 Cleanup and count: `git worktree remove ../oxide-godot-v2 && git worktree prune`;
  `find . -name 'zz_*' -not -path '*/specs/*'` returns nothing; `git status` clean;
  `git log --oneline <tasks-commit>..HEAD` where `<tasks-commit>` is the commit that added this
  file (`git log --format=%h --diff-filter=A -- specs/013-v3-bullet-part-robot/tasks.md`) — state
  the count explicitly: the analyze-fix commit (if `/speckit-analyze` produced one before commit 1)
  plus the four milestone commits plus the two checkpoint-mark commits = seven lines expected
  before this task's own mark (six without an analyze-fix commit), eight after it; `git status -sb`
  shows `## v3` with no upstream.

**Checkpoint**: Milestone V3-C complete — every gameplay module of constitution 1.5.2's ECS list
is ECS.

---

## Dependencies & Execution Order

- Phase 1 → commit 1 → commit 2 → commit 3 → STOP 1 → STOP 2 → commit 4, strictly sequential;
  each commit's gate (build + clippy 0 + tests at the stated count) passes before the next
  starts; the harness runs precede each STOP; both STOPs block Phase 5.
- Commit 1: T003 `[P]` T004 `[P]` T005 `[P]` T006 (disjoint files; T005 imports T003's `HitKind` and
  T004's enums, so build after all three); T007 after T004 + T005 + T006; T008 after T004 + T005;
  T009 last.
- Commit 2: T010 `[P]` with nothing (T011 needs it); T011 after T010 (+ T008's handles); T012 after
  T011; T013 after T011; T014 any time before T015; T015 last.
- Commit 3: T016 `[P]` T017 (disjoint pure files); T018 after T016; T019 after T017; T020 after
  T018; T021 after T019; T022 after T018 + T019 (registration last among the code tasks; its test
  needs `bullet_settle` from commit 2 and `robot_hit_apply`); T023 after T022 (the probe re-run on
  the real build) and before T025; T024 any time before T025; T025 → T026 → T027.
- Commit 4: T028 `[P]` T029; T030 after both; T031 after T030; T032 after T031.

## Parallel Example: commit 3

```text
# Two pure files first:
Task: "T016 part/system.rs — part_phase_tick + 5 tests"
Task: "T017 red_robot/system.rs — robot_decide/robot_step_and_animate/robot_replay/robot_hit_apply/robot_timers + 14 tests"
# then T018 (part/sync.rs) and T019 (red_robot/sync.rs) in parallel, then T020/T021 (bridges),
# T022 (registration + the R4 test), T023 (probe re-run + .tscn), T024 (docs rows + #31),
# T025 (gate + headless + harness + commit), T026/T027 (STOPs).
```

## Implementation Strategy

Sequential by design: Setup → commit 1 (the ECS/hittable extension, pure — every later task
needs the events, components, handles and drain arms) → commit 2 (the bullet: playable on its own,
v2 robots still take its hits) → commit 3 (part + robot: the big one, ONE commit — the robot's
death explodes the parts directly, `Part::explode` is gone, and the same-run hit path needs both
sides) → STOP 1 → STOP 2 → commit 4. The milestone is not shippable partway: after commit 2 the
robot still runs v2's `physics_process` and `call_local` `hit`.

Suggested `/speckit-implement` session split:

- **Session 1** — Phase 1 + commit 1 (T001–T009).
- **Session 2** — commit 2 (T010–T015).
- **Session 3** — commit 3 (T016–T025), ending at 🛑 STOP 1 (T026) and 🛑 STOP 2 (T027). It may be
  worked in two halves — 3a: T016–T022 code + tests + gate; 3b: T023 probe re-run + `.tscn` edit,
  T024, T025 headless and the four harness cases — but it is ONE commit.
- **Session 4** — commit 4 (T028–T032).
