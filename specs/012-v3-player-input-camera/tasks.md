# Tasks: Milestone V3-B — the player as one entity over three nodes

**Input**: Design documents from `/specs/012-v3-player-input-camera/` (spec.md, plan.md, research.md
R1–R10, data-model.md, contracts/player-entity.md, contracts/zz_ecs_parity.gd,
contracts/zz_r1_probe.gd, contracts/zz_r3_probe.gd, quickstart.md — all at commit `9010a3a`).

**Branch**: `v3` — local commits only, **never** `git push`. **Baseline**: `9010a3a` (code baseline
`abfe35a`, V3-A complete; constitution 1.5.1 at `85186f6`), **156 tests**.

**Tests**: requested by the spec (FR-007, FR-012, FR-024; Principle III: every gameplay system
carries a `run_system_once` test; the 36 model tests are preserved by name). Named test cases
below are the MINIMUM; a task that creates pure code without its named tests is incomplete.
Running totals are computed from the names: 156 → 163 (commit 1: setup +2, apply +5) → 177
(commit 2: player/system +9, player_input/system +5) → 179 (commit 3: shake +2).

**Organization**: one task group per commit of research R10's Commit Plan (four commits, in
order). The two user visual checkpoints are 🛑 STOP tasks after commit 3 — confirmed by the user,
not the implementer; both block Phase 5. Every harness task MUST paste the actual `diff` output
in its completion note — never write "identical" without the diff.

**Decisions this file follows where the artifacts disagree**: `PlayerFx` is `{ Jump, Land,
Shoot }` — NO `Hit` (data-model.md's drain table, research R7; data-model.md:39 still shows
`Hit` and is corrected in T030); `jump`/`land`/`shoot` are `call_remote` with the local effects
applied inline by the fixed `SyncOut` (option (b), plan Complexity Tracking); `Shoot`'s trauma
is applied at drain time; `Handles::Player.root` is `Gd<Player>` with the guard-dropped-before-
engine-call rule.

## Phase 1: Setup

- [x] T001 Add the `v2` worktree if absent (`git worktree list`; else `git worktree add
  ../oxide-godot-v2 v2`, `cd ../oxide-godot-v2/oxide_godot_core && cargo build`, `cd
  ../oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .`). Verification: the
  worktree is at `e2932b4`, builds clean, import prints `Initialize godot-rust (...)`.
- [x] T002 On `v3` at `9010a3a`: `cd oxide_godot_core && cargo build && cargo clippy && cargo test`
  → zero warnings, **156 passed** (paste the `test result:` line); `sed -n 592p
  oxide-godot/player/player.tscn` prints `callback_mode_process = 0` (the line T017 edits);
  `grep -c '#\[test\]' oxide_godot_core/oxide_godot_lib/src/{player,player_input,camera_noise_shake}/model.rs`
  prints 16, 13, 7 (the 36 tests that must survive untouched).

**Checkpoint**: baseline confirmed on both trees; commit 1 can start.

---

## Phase 2: User Story 1 (core) — commit 1: ecs extension, pure

**Goal**: FR-005, FR-015 (drain), the `Tuning` resources; nothing engine-facing yet. **Independent
Test**: `cargo test` green at 163 with V3-A's tests untouched; the game still runs unchanged
(no bridge touched).

### Commit 1 — `ecs/setup.rs`, `ecs/markers.rs`, `ecs/event.rs`, `ecs/apply.rs`, `ecs.rs`

- [x] T003 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs/markers.rs` add
  `#[derive(Resource)] pub struct Tuning<T: Send + Sync + 'static>(pub T);` and every player
  component of data-model.md "Components" verbatim: `PlayerTag`, `Simulates`, `OwnsInput`,
  `PeerId(i32)`, `Motion(Vector2)`, `Orientation(Transform3D)`, `RootMotion(Transform3D)`,
  `AirborneTime(f32)`, `InitialPosition(Vector3)`, `CurrentAnimation(Animations)` (`use
  crate::player::Animations`), `AimStateC(AimState)` (`crate::player_input::model::AimState` —
  make `player_input::model` `pub(crate)` if it is private; the file itself stays untouched),
  `JumpQueued(bool)`, `PendingMouseLook(Vec<Vector2>)`, `PendingFx(Vec<PlayerFx>)`,
  `InputFrameC(InputFrame)`, `BodyState { on_floor, velocity, gravity, cooldown_left: f64,
  origin_y: f32 }`, `ReplicatedInput { aiming, shoot_target, motion, shooting }`, `ReplayState {
  current_animation, motion, aim_rotation: f64 }`, `InputSnapshotC(InputSnapshot)`, `CameraFrame {
  parent_y, fade_alpha }` (no rotation snapshot — `camera_and_ray` reads the live rotation per
  delta, analyze finding 7), `TickIntents { land, jump, shoot, respawn, jump_velocity_y:
  Option<f32>, orient: Option<OrientTarget>, plan: Option<AnimPlan>, read_root_motion }` +
  `OrientTarget { Camera(Quaternion), Walk(Vector3) }`, `Velocity(Vector3)`, `FrameIntents {
  camera_deltas: Vec<Vector2>, cue: Option<CameraCue>, jump_pressed, shooting, fade_alpha }`,
  `Trauma(f32)`, `ShakeTime(f64)`, `StartRotation(Vector3)`, `ShakePending(Option<(f32, f64)>)`
  (no `ShakeOffset`: the shake is sync-only, analyze finding 2), with the derives data-model.md
  shows (`Default` where listed).
  Verification: compiles; `grep -c 'Gd<' ecs/markers.rs` = 0.
- [x] T004 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs/event.rs` add the variants
  `InboundEvent::JumpPressed { root_id: InstanceId }`, `MouseLook { root_id, screen_relative:
  Vector2 }`, `AddTrauma { root_id, amount: f64 }`, `PlayerFx { root_id, fx: PlayerFx }`;
  `#[derive(Clone, Copy, Debug, PartialEq)] pub enum PlayerFx { Jump, Land, Shoot }` (NO `Hit`:
  `hit` pushes `AddTrauma { amount: 0.75 }`, v2 `player.rs:157-159`); `Initial::Player { peer_id:
  i32, simulates: bool, owns_input: bool, initial_position: Vector3, orientation: Transform3D,
  start_rotation: Vector3 }` (v2 `player.rs:78`, `:98`, `player_input.rs:61-62`, `player.rs:88`,
  `:90-91`, `camera_noise_shake.rs:41`). Verification: compiles; `Initial` stays `Copy`.
- [x] T005 [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs` add `Handles::Player { root:
  Gd<Player>, input: Gd<PlayerInputSynchronizer>, anim_tree: Gd<AnimationTree>, model: Gd<Node3D>,
  shoot_from: Gd<Marker3D>, shoot_particle: Gd<CpuParticles3D>, muzzle_particle:
  Gd<CpuParticles3D>, fire_cooldown: Gd<Timer>, snd_jump, snd_land, snd_shoot:
  Gd<AudioStreamPlayer>, camera_base: Gd<Node3D>, camera_rot: Gd<Node3D>, camera: Gd<Camera3D>,
  camera_anim: Gd<AnimationPlayer>, crosshair: Gd<TextureRect>, color_rect: Gd<ColorRect>, noise:
  [Gd<FastNoiseLite>; 3], bullet_scene: Gd<PackedScene>, parent_rid: Rid }` (data-model.md
  "Handles"), the `root_valid()` arm on `root`, a `Handles::Player { mut root, .. } =>
  root.queue_free()` arm in `sync_out_remove`'s EXHAUSTIVE match (`ecs.rs:191-195` has no
  wildcard — without the arm commit 1 does not compile; no player is ever removed by this
  milestone, analyze finding 9), and the `apply_register` arm `Initial::Player {
  .. }` inserting `PlayerTag`, `PeerId(peer_id)`, `Motion(Vector2::ZERO)` (v2 `player.rs:44`, zero
  at init), `Orientation(orientation)`, `RootMotion(Transform3D::IDENTITY)`, `AirborneTime(0.0)`
  (`:38-39`, backlog #10 — keep the comment), `InitialPosition(initial_position)`,
  `CurrentAnimation(Animations::Walk)` (`:81`), `AimStateC(AimState::Idle)` (`player_input.rs:21-22`),
  `Trauma(0.0)`, `ShakeTime(0.0)`, `StartRotation(start_rotation)`, `JumpQueued::default()`,
  `PendingMouseLook::default()`, `PendingFx::default()`, `TickIntents::default()`,
  `FrameIntents::default()`, `ShakePending::default()`,
  `ReplicatedInput { aiming: false, shoot_target: Vector3::ZERO, motion: Vector2::ZERO, shooting:
  false }`, `Velocity(Vector3::ZERO)`, and the markers `Simulates` iff `simulates`, `OwnsInput`
  iff `owns_input`. Depends on T003, T004. Verification: compiles; `apply_register` is still the
  only writer of `NodeHandles.by_entity`.
- [x] T006 [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs/setup.rs`: `Phase` gains
  `EngineQueryOrient`, `GameplayIntegrate`, `EngineQueryMove`, `GameplaySettle` (research R4,
  option (a)); `build_fixed` chains `(SyncIn, Gameplay, EngineQueryOrient, GameplayIntegrate,
  EngineQueryMove, GameplaySettle, SyncOut)`; `build_frame` unchanged (`SyncIn, Gameplay,
  EngineQuery, SyncOut`); keep the `auto_insert_apply_deferred` comment; `build_world` inserts
  `Tuning(PlayerTuning::default())`, `Tuning(PlayerInputTuning::default())`,
  `Tuning(CameraShakeTuning::default())` (make the three `model` modules `pub(crate)` where
  needed; the model files stay untouched). Tests (V3-A's three untouched):
  `fixed_sets_are_chained_in_order_and_frame_sets_unchanged` (one probe per set in BOTH schedules,
  added in reverse order; asserts `[SyncIn, Gameplay, EngineQueryOrient, GameplayIntegrate,
  EngineQueryMove, GameplaySettle, SyncOut]` and `[SyncIn, Gameplay, EngineQuery, SyncOut]`) and
  `marker_inserted_in_gameplay_is_visible_in_engine_query_orient_of_the_same_run` (a `Gameplay`
  probe spawns a marker via `Commands`, an `EngineQueryOrient` probe counts 1). Depends on T003.
  Verification: both pass; `phase_sets_are_chained_in_order` still passes.
- [x] T007 [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs/apply.rs` add the four drain arms
  (data-model.md "Drain arms"; every unknown `root_id` dropped): `JumpPressed` → `JumpQueued.0 =
  true` (v2 `player_input.rs:163`); `MouseLook` → `PendingMouseLook.0.push(screen_relative)`
  (`:151-154`); `AddTrauma` → `Trauma.0 = camera_noise_shake::model::add_trauma(trauma.0, amount
  as f32, &world.resource::<Tuning<CameraShakeTuning>>().0)` (`player.rs:163-164`,
  `camera_noise_shake.rs:64-67`, clamped at `max_trauma`); `PlayerFx { fx }` →
  `PendingFx.0.push(fx)` and, for `PlayerFx::Shoot`, ALSO the trauma write with `0.35` at drain
  time (`player.rs:153`; research R7: so the same run's `shake_decide` sees it). Tests
  (`World::new()` + `EntityIndex` + `Tuning<CameraShakeTuning>` + one entity with the queued
  components and `Trauma(0.0)`, registered under a fake id): `jump_pressed_sets_jump_queued`,
  `mouse_look_is_queued_in_order` (two events → `[a, b]`), `add_trauma_clamps_at_max` (0.75 twice
  → `min(1.5, max_trauma = 1.2)` = 1.2), `player_fx_is_queued_and_shoot_adds_trauma_at_drain`
  (`[Jump, Shoot]` queued in order; `Trauma == 0.35` after the drain),
  `player_events_for_unknown_root_are_dropped` (all four events with an unregistered id change
  nothing, no panic). Depends on T003, T004. Verification: five tests pass.
- [x] T008 [US1] Gates: `cd oxide_godot_core && cargo build && cargo clippy && cargo test` —
  zero warnings; **163** = 156 + 2 (setup) + 5 (apply); paste the `test result:` line. Headless
  import + `main/main.tscn` + `level/level.tscn` clean (nothing engine-facing changed; a sanity
  run). Commit (R10 row 1): `ecs: seven fixed-schedule sets (Phase extension), Tuning<T>
  resources, drain arms for JumpPressed/MouseLook/AddTrauma/PlayerFx (tests)`.

**Checkpoint**: the ECS core carries the player's sets, resources, components, events and drain;
no bridge changed yet.

---

## Phase 3: User Story 1 + User Story 2 + User Story 4 — commit 2: `player` + `player_input`, the `.tscn` edit

**Goal**: FR-001…FR-011, FR-012…FR-014, FR-018 (option B), FR-019. **Independent Test**: 177
tests green with the 16 + 13 model tests untouched; harness cases (a), (b), (c), (e), (f)
identical on `v2` and `v3`; the game plays as v2 in single player (shake still on v2's path until
commit 3).

### Commit 2 — `player/system.rs`, `player/sync.rs`, `player.rs`, `player_input/system.rs`, `player_input/sync.rs`, `player_input.rs`, `camera_noise_shake.rs` (accessors only), `ecs.rs`, `player.tscn:592`, `docs/v3-tradeoffs.md`

- [x] T009 [P] [US1] Create `oxide_godot_core/oxide_godot_lib/src/player/system.rs` (pure; declare
  `pub(crate) mod system;` in `player.rs`; `player/model.rs` untouched): `pub fn tick_decide(tuning:
  Res<Tuning<PlayerTuning>>, dt: Res<FixedDelta>, q: Query<(&InputFrameC, &BodyState, &Orientation,
  &mut Motion, &mut AirborneTime, &mut TickIntents), With<Simulates>>)` — per entity, in v2's
  order: `Motion = lerp_motion(motion, frame.motion, dt, &tuning)` (`player.rs:226`); `(camera_x,
  camera_z) = flatten_camera_axes(frame.camera_rotation_basis)` (`:229`); `airborne_step(airborne,
  dt, body.on_floor, frame.jumping, &tuning)` → `AirborneTime`, `intents.jump_velocity_y`,
  `intents.land`, `intents.jump` (`:232-251`); the branch (`:254-310`): `on_air` → `plan =
  anim_plan(true, intents.jump_velocity_y.unwrap_or(body.velocity.y), false, motion, 0.0)` —
  the POST-jump velocity: v2 wrote the jump velocity at `:240-244` and re-read it at `:255`
  before `anim_plan`, so the jump step plans `JumpUp` (analyze BLOCKER 1) — `orient = None`, `read_root_motion =
  false` (`:255-258`); aiming → `orient = Some(OrientTarget::Camera(frame.camera_base_quaternion))`
  (`:262`), `plan = anim_plan(false, 0.0, true, motion, frame.aim_rotation)` (`:266`),
  `read_root_motion = true`, `shoot = frame.shooting && body.cooldown_left == 0.0` (`:274`); else
  → `orient = walk_target(camera_x, camera_z, motion).map(OrientTarget::Walk)` (`:294-295`),
  `plan = anim_plan(false, 0.0, false, motion, 0.0)` (`:303`), `read_root_motion = true`;
  `respawn = false` at this point. `pub fn tick_integrate(dt: Res<FixedDelta>, q: Query<(&BodyState,
  &RootMotion, &mut Orientation, &mut Velocity, &TickIntents), With<Simulates>>)`: velocity in =
  `body.velocity` with `y` replaced by `jump_velocity_y` when `Some` (`:240-244` happened before
  `:313` in v2), `(orientation, velocity) = integrate_root_motion(orientation, root_motion, dt,
  body.gravity, velocity_in)` (`:313-317`). `pub fn tick_settle(tuning, q: Query<(&BodyState, &mut
  TickIntents), With<Simulates>>)`: `intents.respawn = should_respawn(body.origin_y, &tuning)`
  (`:329`). `pub fn replay_plan(state: &ReplayState) -> AnimPlan` reproducing `:104-117` (`JumpUp`,
  `JumpDown`, `Strafe { aim_rotation, blend (motion.x, -motion.y) }`, `Walk { blend
  (motion.length(), 0) }`). Tests (`run_system_once`, `World::new()`, `Tuning(PlayerTuning::default())`,
  `FixedDelta(1.0/60.0)`, one entity): `walk_decides_walk_plan_and_walk_target` (grounded, motion
  forward, not aiming → `plan = Walk`, `orient = Some(Walk(_))`, `read_root_motion`),
  `aim_decides_strafe_plan_and_camera_orient` (`aiming` → `Strafe`, `orient = Some(Camera(q))`),
  `airborne_decides_jump_plan_and_skips_orient_and_root_motion` (`on_floor = false`, airborne
  time past the threshold → `JumpUp`/`JumpDown` by velocity sign, `orient = None`,
  `read_root_motion = false`), `jump_step_plans_jump_up_from_the_post_jump_velocity` (`on_floor
  = true`, `jumping = true`, `body.velocity.y = 0` → `jump_velocity_y = Some(jump_speed)` and
  `plan = JumpUp`, not `JumpDown`), `land_and_jump_intents_can_both_fire` (the model's same-frame case,
  `player/model.rs:50-52`), `shoot_fires_only_when_aiming_and_cooldown_zero` (`shooting` with
  `cooldown_left = 0.1` → no shoot; `0.0` → shoot), `integrate_updates_orientation_and_velocity`
  (equals a direct `integrate_root_motion` call on the same inputs),
  `settle_flags_respawn_below_threshold` (`origin_y = -41` → true; `-39` → false),
  `replay_builds_v2_plans_for_all_four_animations`. Verification: 9 tests pass; no
  `godot::classes` import in the file.
- [x] T010 [P] [US2] Create `oxide_godot_core/oxide_godot_lib/src/player_input/system.rs` (pure;
  `pub(crate) mod system;` in `player_input.rs`; `player_input/model.rs` untouched): `pub fn
  input_decide(tuning: Res<Tuning<PlayerInputTuning>>, dt: Res<FrameDelta>, q:
  Query<(&InputSnapshotC, &CameraFrame, &mut PendingMouseLook, &mut AimStateC, &mut FrameIntents,
  &mut ReplicatedInput), With<OwnsInput>>)` — `camera_deltas` = for each queued mouse motion in
  order `scaled_mouse_look(rel, aim_state.is_aiming(), &tuning)` (v2 `player_input.rs:153`, using
  the aim state BEFORE this frame's `step_aim`, as v2's `input` ran before `process`), THEN
  `scaled_look(snapshot.camera_move, aiming, dt, &tuning)` (`:94`); clear `PendingMouseLook`;
  `replicated.motion = snapshot.motion` (`:92`); `(next, cue) = step_aim(aim_state, &snapshot, dt,
  &tuning)` → `AimStateC`, `replicated.aiming = next.is_aiming()`, `intents.cue = cue` (`:97-99`);
  `intents.jump_pressed = snapshot.jump_just_pressed` (`:111`); `intents.shooting =
  snapshot.shoot_pressed`, `replicated.shooting = …` (`:115`); `intents.fade_alpha =
  alpha_for_height(camera.parent_y, camera.fade_alpha, dt, &tuning)` (`:146`). `clamp_pitch` is
  applied per delta by `camera_and_ray` (it needs the live rotation between deltas); the pure
  system emits the deltas in order (no `pitch_after` helper: `camera_and_ray` clamps on the live
  `get_rotation()` per delta exactly as v2 `:188-190`, analyze finding 7). Tests:
  `controller_look_is_scaled` (delta = `scaled_look` of the snapshot, aiming and not),
  `mouse_look_is_applied_before_controller_look` (queued
  mouse + controller → `camera_deltas[0]` is the mouse delta),
  `aim_hold_and_toggle_reach_v2_cues` (hold past 0.4 s then release → `Far`; tap → `Toggled`,
  no cue), `fade_alpha_follows_height` (`parent_y = -32` → 1.0; above −17 → decays),
  `jump_just_pressed_sets_intent_for_one_frame`. Verification: 5 tests pass; no `godot::classes`.
- [x] T011 [US1] Create `oxide_godot_core/oxide_godot_lib/src/player/sync.rs` (glue; `pub(crate)
  mod sync;` in `player.rs`) with: `fn sync_in_player(input_tuning: Res<Tuning<PlayerInputTuning>>,
  handles: NonSend<NodeHandles>, q: Query<(Entity, &mut JumpQueued, Has<Simulates>),
  With<PlayerTag>>, commands: Commands)` — for
  each `Handles::Player`: `InputFrameC` from `input.bind()`'s `aiming`/`shoot_target`/`motion`/
  `shooting` (`:211-216`) + `jumping = jump_queued.0` then `jump_queued.0 = false` (`:215`, `:221`)
  + the three camera reads `camera_rot.get_global_transform().basis`,
  `camera_base.get_global_transform().basis.get_quaternion()`, `aim_rotation(camera_rot.get_rotation().x,
  &tuning)` (`player_input.rs:171-182`); `BodyState { on_floor: root.is_on_floor(), velocity:
  root.get_velocity(), gravity: root.get_gravity(), cooldown_left: fire_cooldown.get_time_left(),
  origin_y: 0.0 }` (`player.rs:235`, `:241`, `:314`, `:274`; `origin_y` is written ONLY by
  `move_body` post-move — v2 read the origin once, at `:329`; no `ReplicatedInput` on the fixed
  side: the `InputFrameC` read IS the projection's consumer, analyze findings 13/14); for
  non-`Simulates`: `ReplayState { current_animation:
  root.bind().current_animation, motion: root.bind().motion, aim_rotation }` (`:104-116`). All
  `bind()` guards dropped before the next engine call. `fn orient_and_anim(...)`
  (`EngineQueryOrient`): `Simulates` — `orient` `Camera(q_to)` → `Orientation.basis =
  Basis::from_quaternion(q_from.slerp(q_to, dt * tuning.rotation_interpolate_speed))` (`:261-264`);
  `Walk(target)` → `q_to = Basis::looking_at(target).get_quaternion()` then the same slerp
  (`:296-300`); NO `AnimationTree` parameter write here (moved to `sync_out_player`, analyze
  finding 3 — the root motion read below returns the previous `advance`'s value regardless, R1);
  when `read_root_motion`: `RootMotion = Transform3D::new(Basis::from_quaternion(anim_tree.get_root_motion_rotation()),
  anim_tree.get_root_motion_position())` (`:269-272`/`:306-309`); when `intents.shoot`: the bullet
  spawn verbatim (`:275-291`: `shoot_from.get_global_transform().origin`, `bullet_scene.instantiate_as::<CharacterBody3D>()`,
  `root.get_parent().unwrap().add_child_ex(&bullet).force_readable_name(true).done()`,
  `set_global_position`, `look_at(origin + dir)`, `add_collision_exception_with(&root)`);
  `orient_and_anim` runs on `Simulates` only. Define here `pub(crate) fn apply_anim(anim_tree:
  &mut Gd<AnimationTree>, plan: AnimPlan)` — v2 `apply_anim`'s parameter writes (`:179-200`) in
  v1's per-variant order, with the EIGHT constants moved from `player.rs:21-29`: the four
  parameter paths (`TRANSITION_REQUEST`, `AIM_ADD_AMOUNT`, `STRAFE_BLEND`, `WALK_BLEND`) and the
  four transition names (`jump_up`, `jump_down`, `strafe`, `walk`); called by `sync_out_player`
  and, in commit 3, by `apply_player_fx` (analyze finding 10). `fn
  move_body(...)` (`EngineQueryMove`): `root.set_velocity(velocity.0)`,
  `root.set_up_direction(Vector3::UP)`, `root.move_and_slide()`, `body.origin_y =
  root.get_transform().origin.y` (`:320-322`, `:329`). `fn sync_out_player(dt: Res<FixedDelta>,
  handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Orientation, &InitialPosition, &Motion,
  &CurrentAnimation, &TickIntents, Has<Simulates>), With<PlayerTag>>)` — per entity: `model.set_global_basis(orientation.basis)`
  (`:326`); on `Simulates`: if `intents.respawn` { transform origin = initial_position;
  `set_velocity(ZERO)` } (`:330-333`, backlog #11 comment); `{ let mut g = root.bind_mut(); g.motion
  = motion.0; g.current_animation = current_animation.0; }` — guard DROPPED here; the local
  effects inline in v2's order (option (b)): if `intents.land` → `snd_land.play()` (`:142`); if
  `intents.jump` → `snd_jump.play()` (`:136`); if `intents.shoot` → `shoot_particle.restart()` +
  `set_emitting(true)`, `muzzle_particle` same, `fire_cooldown.start()`, `snd_shoot.play()`
  (`:147-152`), and the trauma 0.35 — IN THIS COMMIT through the v2 path
  `camera.clone().cast::<CameraNoiseShake>().bind_mut().add_trauma(0.35)` (`:153`; `.clone()`
  because `Gd::cast` consumes an owned `Gd` and `camera` is borrowed from the handles map —
  analyze finding 12; commit 3 replaces it with the `Trauma` component write); then
  `root.rpc("land")`, `root.rpc("jump")`, `root.rpc("shoot")` when flagged, in that order
  (`:246-251`, `:290`; now `call_remote`, T014); then the `AnimationTree` parameter writes:
  `apply_anim(&mut anim_tree, plan)` with `plan = intents.plan` on `Simulates` or
  `replay_plan(&replay_state)` on non-`Simulates` (`:179-200`, `:118`; analyze finding 3 —
  behavior-identical to v2, where the tree consumed them only when it processed after the
  tick); LAST for EVERY `PlayerTag` entity: `anim_tree.advance(dt.0)` (R1, option B). Depends on T009,
  T005. Verification: compiles; `grep -n 'free()' player/sync.rs` empty; every `bind_mut()` scope
  ends before the next engine call.
- [x] T012 [US2] Create `oxide_godot_core/oxide_godot_lib/src/player_input/sync.rs` (glue;
  `pub(crate) mod sync;`): `fn sync_in_input(handles: NonSend<NodeHandles>, q: Query<(Entity,
  Has<OwnsInput>), With<PlayerTag>>, commands: Commands)` — on `OwnsInput`: the ten `Input`
  reads into `InputSnapshotC` in v2's order (`player_input.rs:76-90`), `CameraFrame { parent_y:
  root.get_global_transform().origin.y, fade_alpha: color_rect.get_modulate().a }` (`:144-145`);
  NOTHING for other players (the fixed `sync_in_player` reads the projection, analyze finding
  14). `fn camera_and_ray(tuning: Res<Tuning<PlayerInputTuning>>, handles: NonSendMut<NodeHandles>,
  q: Query<(Entity, &FrameIntents, &mut ReplicatedInput), With<OwnsInput>>)` (`EngineQuery`): for each delta in `camera_deltas`: `camera_base.rotate_y(-mv.x)`,
  `camera_base.orthonormalize()`, `rot = camera_rot.get_rotation(); rot.x = clamp_pitch(rot.x,
  mv.y, &tuning); camera_rot.set_rotation(rot)` (`:184-191`); THEN if `intents.shooting`: the
  crosshair raycast verbatim (`:117-138`: `crosshair.get_position() + get_size() * 0.5`,
  `camera.project_ray_origin/normal`, `PhysicsRayQueryParameters3D::create_ex(from, from + dir *
  1000.0).collision_mask(0b11).exclude(&array![parent_rid]).done()`,
  `root.get_world_3d().unwrap().get_direct_space_state().unwrap().intersect_ray(&params)`;
  `replicated.shoot_target` = hit `position` or `from + dir * 1000.0`). `fn sync_out_input(handles,
  q: Query<(Entity, &FrameIntents, &ReplicatedInput), With<OwnsInput>>)` (`SyncOut`): cue →
  `camera_anim.play_ex().name("shoot"|"far").done()` (`:100-109`); `{ let mut g = input.bind_mut();
  g.aiming = ..; g.shoot_target = ..; g.motion = ..; g.shooting = ..; }` guard dropped;
  `color_rect.set_modulate` with `a = intents.fade_alpha` (`:145-147`); if `intents.jump_pressed`
  → `input.rpc("jump", &[])` (`:111-113`). Depends on T010, T005. Verification: compiles; the
  raycast happens after the rotation writes in the same system.
- [x] T013 [P] [US1] In `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs` add ONLY two
  `pub(crate)` accessors on the (still v2) class: `fn noises(&self) -> [Gd<FastNoiseLite>; 3]`
  (clones of `noise_yaw`, `noise_pitch`, `noise_roll`, `:17-22`) and `fn start_rotation(&self) ->
  Vector3` (`:14`, captured in `ready` `:41`) — read by `Player.ready` (T014); the class's
  `process`/`add_trauma` stay for this commit (R10: commit 2 keeps the v2 trauma path).
  Verification: `git diff` of the file shows only the two accessors.
- [x] T014 [US1] Rewrite `oxide_godot_core/oxide_godot_lib/src/player.rs` as the root bridge
  (data-model.md "Bridges"): keep `Animations` (`:12-19`), the eleven `OnReady` handles (`:48-68`),
  `bullet_scene` (`:72-73`, backlog #28 comment kept), `#[export] #[var(set = set_player_id)]
  player_id` + `set_player_id` (`:75-78`, `:125-131`, untouched), `#[export] current_animation`
  (`:80-82`) and `#[var] motion` (`:43-44`) as projection fields; DELETE `airborne_time`,
  `orientation`, `root_motion`, `initial_position`, `physics_process`, `apply_input`, `apply_anim`
  and the eight `AnimationTree` constants (`:21-29`, moved to `player/sync.rs`); move backlog #10's comment to
  `apply_register`'s `AirborneTime(0.0)` (T005) and #11's to `sync_out_player`'s respawn (T011).
  `ready`: `let mp = self.base().get_multiplayer().unwrap(); let simulates = mp.is_server();`
  (`:98`); `let owns_input = self.player_input.get_multiplayer_authority() ==
  mp.get_unique_id();` (`player_input.rs:61-62`); `initial_position = self.base().get_transform().origin`
  (`:88`); `orientation = self.player_model.get_global_transform()` with `origin = ZERO`
  (`:90-91`); `{ let pi = self.player_input.bind(); camera_base/camera_rot/camera/camera_anim/
  crosshair/color_rect = pi.camera_base.clone() … ; parent_rid = *pi.parent_rid; }`; `let cam =
  camera.clone().cast::<CameraNoiseShake>(); let (noise, start_rotation) = { let c = cam.bind();
  (c.noises(), c.start_rotation()) };` then `queue::push(InboundEvent::Register { id:
  self.base().instance_id(), handles: Handles::Player { root: self.to_gd(), input:
  self.player_input.clone(), … }, initial: Initial::Player { … } })`. `exit_tree` → `Unregister`.
  RPCs: `#[rpc(authority, call_remote, unreliable)] fn jump/land/shoot` push
  `PlayerFx::{Jump, Land, Shoot}` (they now run on REMOTE peers only; option (b)); `#[rpc(authority,
  call_local, unreliable)] fn hit` and `pub(crate) fn add_camera_shake_trauma(&mut self, amount:
  f64)` — IN THIS COMMIT keep v2's typed call `self.player_input.bind().camera_camera.clone().cast::<CameraNoiseShake>().bind_mut().add_trauma(amount)`
  (`:163-164`; `hit` calls it with 0.75) so the shake keeps working on v2's path; commit 3 (T023)
  turns both into `AddTrauma` pushes. Depends on T011, T013. Verification: `grep -n 'fn
  physics_process\|fn process\|apply_input\|apply_anim' player.rs` empty; `grep -c 'call_remote'
  player.rs` = 3; `grep -c 'call_local' player.rs` = 2; `level.rs`, `red_robot.rs`, `hittable.rs`
  still compile unchanged.
- [x] T015 [US2] Rewrite `oxide_godot_core/oxide_godot_lib/src/player_input.rs` as the input
  sub-bridge: keep the four `#[export] pub(crate)` fields (`:32-39`), the six `#[export]
  OnEditor` refs with their names (`:44-55`; `node_paths` in `player.tscn:339` unedited), `parent`
  and `parent_rid` (`:26-29`, `parent_rid` made `pub(crate)`); ADD `#[init(val =
  OnReady::from_base_fn(|b| b.get_owner().unwrap().instance_id()))] root_id: OnReady<InstanceId>`
  (R1's `OWNER` check: the owner is the `Player`); DELETE `aim_state`, `jumping`, `process`, the
  three getters (`:171-182`) and `rotate_camera` (`:184-191`, now in `player_input/sync.rs`).
  `ready` = v2 `:60-70` with `set_process(false)` dropped (no `process`) and
  `set_process_input(false)` KEPT on non-authority nodes (FR-013's gate); `fn input(&mut self,
  event: Gd<InputEvent>)`: on `InputEventMouseMotion` → `queue::push(InboundEvent::MouseLook {
  root_id: *self.root_id, screen_relative: mm.get_screen_relative() })` (`:150-156`); `#[rpc(authority,
  call_local, unreliable)] fn jump(&mut self)` → `queue::push(InboundEvent::JumpPressed { root_id:
  *self.root_id })` (`:161-164`). Depends on T012. Verification: `grep -n 'fn process\|jumping\|
  get_aim_rotation\|rotate_camera' player_input.rs` empty; the four `#[export]` fields and the six
  refs unchanged (`git diff` shows no attribute line removed).
- [x] T016 [US1] In `oxide_godot_core/oxide_godot_lib/src/ecs.rs`'s `add_engine_systems` register:
  fixed — `player::sync::sync_in_player.in_set(Phase::SyncIn).after(sweep_dead_nodes)`,
  `orient_and_anim.in_set(Phase::EngineQueryOrient)`, `move_body.in_set(Phase::EngineQueryMove)`,
  `sync_out_player.in_set(Phase::SyncOut).before(sync_out_remove)`; frame —
  `player_input::sync::sync_in_input.in_set(Phase::SyncIn).after(sweep_dead_nodes)`,
  `camera_and_ray.in_set(Phase::EngineQuery)`, `sync_out_input.in_set(Phase::SyncOut).before(sync_out_remove)`.
  In `ecs/setup.rs`: `build_fixed` adds `player::system::tick_decide.in_set(Phase::Gameplay)`,
  `tick_integrate.in_set(Phase::GameplayIntegrate)`, `tick_settle.in_set(Phase::GameplaySettle)`;
  `build_frame` adds `player_input::system::input_decide.in_set(Phase::Gameplay)`. Depends on
  T011, T012. Verification: `setup.rs` tests still pass (the pure systems compile into the
  schedules built by the tests — they need the `Tuning`/delta resources `build_world` inserts).
- [x] T017 [US4] Edit `oxide-godot/player/player.tscn` line 592 from `callback_mode_process = 0` to
  `callback_mode_process = 2` (the `AnimationTree` → MANUAL; research R1, option B; the
  milestone's ONE scene edit). Verification: `git diff oxide-godot/player/player.tscn` shows
  exactly that one changed line (the `AnimationPlayer` at `:577` stays `0`).
- [x] T018 [US1] Append to `docs/v3-tradeoffs.md` the rows (contracts/player-entity.md §5): root
  motion read from the `AnimationTree` (`orient_and_anim`); `move_and_slide` + post-move origin
  (`move_body`); crosshair raycast after the in-set camera rotation (`camera_and_ray`);
  `slerp`/`looking_at` engine-backed math (`orient_and_anim`); bullet instancing from the tick
  (`orient_and_anim`); the `AnimationTree` MANUAL + `advance` decision (`sync_out_player`,
  `player.tscn:592`, with R1's `M(n).rm == S(n+1).rm` evidence); replication as projection (the
  §2 table); RPC timing — `jump`/`land`/`shoot` `call_remote`, local effects inline in the fixed
  `SyncOut` (option (b), R2). Verification: 5 + 8 = 13 rows.
- [x] T019 [US1] Gates + headless + harness: `cargo build && cargo clippy && cargo test` — zero
  warnings; **177** = 163 + 9 (player/system) + 5 (player_input/system); the 16 + 13 model tests
  still listed by name; paste the `test result:` line. Headless: import (`Initialize godot-rust`),
  `main/main.tscn` ×2, `level/level.tscn` clean vs `CLAUDE.md`'s catalog. Harness (research R9,
  quickstart §3): write `zz_ecs_parity.gd` (from `contracts/zz_ecs_parity.gd`),
  `zz_ecs_observer.gd` (two lines), `zz_ecs_physics_probe.gd` (three literal lines, quoted in the
  contract) and `zz_ecs_parity.tscn` (six lines, V3-A contract §4) into BOTH trees; run cases
  (a) walk, (b) jump+land, (c) aim+shoot, (e) mouse look, (f) fall/respawn with `--fixed-fps 60
  --quit-after 400` under `XDG_DATA_HOME=/tmp/xdg-v2` and `/tmp/xdg-v3`; logs at
  `"$XDG_DATA_HOME/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_<case>.log"`;
  `diff` the full logs AND the `^RAW` subsets per case. Expected (R1–R3): all identical; the RAW
  `Jump.playing`/`Land.playing`/`Shoot.playing` stamps of (b)/(c) on the same frames (option (b):
  inline in the physics step on both trees). **Paste the actual diff output in the completion
  note — never write "identical" without the diff.** Any difference → the spec's timing-table
  candidates (T030) with its cause, or a fix before committing if it is not a timing shift.
  Delete `zz_*` (and `.uid`s) from both trees. Commit (R10 row 2): `player + player_input:
  bridges, Player entity (Handles::Player), fixed tick (7 sets) and frame input systems;
  AnimationTree MANUAL + advance (R1 option B)`.

**Checkpoint**: the player tick and input run on the ECS; single player plays as v2 (shake on
the v2 path); multiplayer remote effects arrive with commit 3.

---

## Phase 4: User Story 3 — commit 3: camera shake, the trauma path, remote `PlayerFx`

**Goal**: FR-015 (`apply_player_fx`), FR-016, FR-017, and the last v2 per-frame callback gone.
**Independent Test**: 179 tests with the 7 model tests untouched; harness (d) identical, (b) and
(c) re-run identical; both visual checkpoints.

### Commit 3 — `camera_noise_shake/system.rs`, `camera_noise_shake/sync.rs`, `camera_noise_shake.rs`, `player/sync.rs` (`apply_player_fx`, trauma component), `player.rs` (handlers push only), `ecs.rs`, `docs/v3-tradeoffs.md`

- [ ] T020 [P] [US3] Create `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake/system.rs`
  (pure; `pub(crate) mod system;`; `model.rs` untouched): `pub fn shake_decide(tuning:
  Res<Tuning<CameraShakeTuning>>, dt: Res<FrameDelta>, q: Query<(&mut Trauma, &mut ShakeTime, &mut
  ShakePending), With<PlayerTag>>)` — if `trauma.0 > 0.0` (v2 `camera_noise_shake.rs:45`):
  `trauma.0 = decay(trauma.0, dt as f32, &tuning)`, `time.0 = advance_time(time.0, dt, &tuning)`,
  `pending.0 = Some((shake(trauma.0), time.0))` (`:47-49`, that order); else `pending.0 = None`.
  Tests: `shake_runs_only_while_trauma_positive` (`Trauma(0)` → `None`; `Trauma(0.5)` → `Some`),
  `decay_and_time_advance_in_v2_order` (values equal a direct `decay`/`advance_time`/`shake`
  chain). Verification: 2 tests pass; no `godot::classes`.
- [ ] T021 [US3] Create `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake/sync.rs` (glue;
  `pub(crate) mod sync;`): ONE system `fn sync_out_shake(tuning: Res<Tuning<CameraShakeTuning>>,
  handles: NonSendMut<NodeHandles>, q: Query<(Entity, &StartRotation, &ShakePending)>)`
  (`SyncOut`): on `Some((shake, time))` → `samples = [noise[0].get_noise_1d(time as f32),
  noise[1]…, noise[2]…]` (`:50-54`), `offset = offsets(shake, samples, &tuning)` (`:55`, pure,
  called from glue), `camera.set_rotation(start.0 + offset)` (`:56-57`); else nothing. The shake
  has NO `EngineQuery` member (analyze finding 2: the samples answer no mid-tick decision; a
  sync-only pair like V3-A's blast). In `player/sync.rs` add `fn apply_player_fx(handles:
  NonSendMut<NodeHandles>, q: Query<(Entity, &mut PendingFx), Without<Simulates>>)` (`SyncOut`,
  frame; remote peers only, option (b)): drain `PendingFx` in order — `Jump` → `apply_anim(&mut
  anim_tree, AnimPlan::JumpUp)` + `{ root.bind_mut().current_animation = Animations::JumpUp }`
  (the NODE field, as v2's handler wrote `self.current_animation`, `:172-177`; guard dropped
  before the sound — analyze finding 19) + `snd_jump.play()` (`:134-137`); `Land` → the same with
  `JumpDown` + `snd_land.play()` (`:140-143`); `Shoot` → both particles restart +
  `set_emitting(true)`, `fire_cooldown.start()`, `snd_shoot.play()` (`:147-152`; its trauma was
  applied at drain time). Depends on T020, T005. Verification: compiles; `grep -n 'EngineQuery'
  camera_noise_shake/sync.rs` empty.
- [ ] T022 [US3] Rewrite `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs` as the
  camera sub-bridge: keep the three `FastNoiseLite` fields and `#[init(val = (randi() as i32))]
  noise_seed` (`:17-24`, same init call order), `start_rotation`, `ready` (`:29-42` verbatim), the
  two accessors of T013; ADD `root_id: OnReady<InstanceId>` via `get_owner()`; DELETE `trauma`,
  `time`, `process` (`:44-59`), `add_trauma` (`:64-67`). Verification: `grep -n 'fn process\|
  add_trauma\|trauma' camera_noise_shake.rs` empty (except the doc comment).
- [ ] T023 [US3] In `oxide_godot_core/oxide_godot_lib/src/player.rs`: `hit` → `queue::push(
  InboundEvent::AddTrauma { root_id: self.base().instance_id(), amount: 0.75 })` (`:157-159`);
  `add_camera_shake_trauma(&mut self, amount: f64)` → `queue::push(InboundEvent::AddTrauma { …,
  amount })` (`:161-165`; `red_robot.rs:452` unchanged); the v2 typed call removed. In
  `player/sync.rs`'s `sync_out_player`, replace the commit-2 `add_trauma(0.35)` call with the
  component write `trauma.0 = add_trauma(trauma.0, 0.35, &tuning)` — add `&mut Trauma` and
  `Res<Tuning<CameraShakeTuning>>` to its parameters (this commit owns the component path).
  Depends on T021, T022. Verification: `grep -n 'CameraNoiseShake\|add_trauma' player.rs
  player/sync.rs` shows only the `cast::<CameraNoiseShake>()` in `ready` and the model call.
- [ ] T024 [US3] In `ecs.rs`'s `add_engine_systems` register (frame): `camera_noise_shake::sync::
  sync_out_shake.in_set(Phase::SyncOut).before(sync_out_remove)`,
  `player::sync::apply_player_fx.in_set(Phase::SyncOut).before(sync_out_remove)`; in
  `ecs/setup.rs::build_frame`: `camera_noise_shake::system::shake_decide.in_set(Phase::Gameplay)`.
  Depends on T021. Verification: setup tests pass.
- [ ] T025 [US3] Append to `docs/v3-tradeoffs.md` the row: noise samples (`sync_out_shake`, a
  sync-only pair — the engine owns `FastNoiseLite`; three reads per frame while `Trauma > 0`, R8).
  Verification: 14 rows.
- [ ] T026 [US3] Gates + headless + harness: `cargo build && cargo clippy && cargo test` — zero
  warnings; **179** = 177 + 2 (shake); paste the line. Headless import, `main.tscn`, `level.tscn`
  clean. Harness on both trees as T019: case (d) camera shake (`player.hit()` at frame 30; per
  frame `Camera3D.rotation`), and RE-RUN (b) and (c) (the trauma path changed: `Shoot` trauma at
  drain time); `diff` full and `^RAW`. **Paste the actual diff output in the completion note —
  never write "identical" without the diff.** Delete `zz_*` from both trees. Commit (R10 row 3):
  `camera_noise_shake: sub-bridge, shake systems on the player entity, AddTrauma/PlayerFx trauma
  path`.
- [ ] T027 [US3] 🛑 **STOP 1 — user visual checkpoint (1), single player** (spec SC-005). Tell the
  user explicitly: "Checkpoint (1): please test now" — run the game, walk, jump, aim, shoot a
  robot, get hit (shake), fall off the map and respawn, all as in v2; ask whether backlog #29's
  FPS observation persists and record the answer. Do not proceed until confirmed.
- [ ] T028 [US3] 🛑 **STOP 2 — user visual checkpoint (2), multiplayer on one machine**. Tell the
  user explicitly: "Checkpoint (2): please test multiplayer now" — host in one instance, join
  from a second through the demo's menu; the remote player's movement, animation
  (`current_animation`/`motion` projection) and aim replicate as in v2; each client controls only
  its own player; the remote `jump`/`land`/`shoot` effects (sounds, particles) play on the
  client (`apply_player_fx`). Do not start Phase 5 before BOTH checkpoints are confirmed.

**Checkpoint**: User Stories 1–4 complete and independently verified.

---

## Phase 5: Polish — commit 4

- [ ] T029 [P] Extend `CLAUDE.md`'s "Port conventions (v3)" with: the sub-bridge pattern (the
  root registers everything in its `ready`, children ready first; sub-bridges keep only v2's
  one-shot setup and push events keyed by `get_owner().instance_id()` resolved once); the
  seven-set fixed tick with two `EngineQuery` sets and why it respects the constitution; the
  `call_remote` rule (RPCs whose local effects the tick applies inline on `Simulates`; handlers
  run on remote peers only); the `bind_mut()` guard dropped before any engine call that can
  invoke a callback (`Handles::Player.root: Gd<Player>`); the `AnimationTree` MANUAL + `advance`
  rule for later modules with a tree (`red_robot`); the layout `x/system.rs` (pure) +
  `x/sync.rs` (glue); the three probe scripts as re-verification tools. Verification: `grep -c
  'call_remote' CLAUDE.md` ≥ 1.
- [ ] T030 [P] Fill `specs/012-v3-player-input-camera/spec.md`'s "Measured timing differences"
  table from the six diffs of T019/T026 (one row per differing RAW event with cause; or a single
  "no differences" row quoting the commit hashes where the empty diffs are recorded); align
  `data-model.md` with the code (the "Tests by name" list already mirrors T006/T007/T009/T010/
  T020 after the analyze-fix commit — re-check names actually landed); confirm
  `docs/v3-tradeoffs.md` has the 9 new rows (14 total). Verification: no `(to be measured)` in
  spec.md; `grep -n 'PlayerFx::Hit' specs/012-v3-player-input-camera/*.md` empty.
- [ ] T031 Run quickstart.md §5's grep list VERBATIM and paste every output (no `process`/
  `physics_process` in the three bridges; `fn input` only in `player_input.rs`; no
  `godot::task::spawn`/`bind_mut::<EcsWorld>`/`get_autoload_by_name::<EcsWorld>` in the three
  modules; no `godot::classes` in the three `system.rs`; the three `model.rs` diff-empty against
  `abfe35a`; the excluded modules diff-empty; `player.tscn` diff = the one line; tradeoffs ≥ 14;
  backlog #6/#29 open). If a grep reveals a violation, STOP and report.
- [ ] T032 Final gates + headless (`cargo build && cargo clippy && cargo test` → **179**; import;
  `main.tscn`; `level.tscn`); commit (R10 row 4): `CLAUDE.md: v3 sub-bridge pattern +
  two-EngineQuery tick; spec: measured timing differences filled; tradeoffs complete`.
- [ ] T033 Cleanup and count: `git worktree remove ../oxide-godot-v2 && git worktree prune`;
  `find . -name 'zz_*' -not -path '*/specs/*'` returns nothing; `git status` clean;
  `git log --oneline <tasks-commit>..HEAD` where `<tasks-commit>` is the commit that added this
  file (`git log --format=%h --diff-filter=A -- specs/012-v3-player-input-camera/tasks.md`) —
  state the count explicitly: the analyze-fix commit (made before commit 1) plus the four
  milestone commits plus the checkpoint-mark commits (two, after T027 and T028) = seven lines
  expected; `git status -sb` shows `## v3` with no upstream.

**Checkpoint**: Milestone V3-B complete.

---

## Dependencies & Execution Order

- Phase 1 → commit 1 → commit 2 → commit 3 → STOP 1 → STOP 2 → commit 4, strictly sequential;
  each commit's gate (build + clippy 0 + tests at the stated count) passes before the next
  starts; the harness runs precede each STOP; both STOPs block Phase 5.
- Commit 1: T003 `[P]` T004 (disjoint files); T005 after T003 + T004; T006 after T003 (the
  `Tuning` resources); T007 after T003 + T004; T008 last.
- Commit 2: T009 `[P]` T010 `[P]` T013 (disjoint, no intra-commit dependency); T011 after T009
  (+ T005's `Handles::Player`); T012 after T010; T014 after T011 + T013 (the bridge needs the
  systems' helper and the camera accessors); T015 after T012; T016 after T011 + T012 (registration
  last among the code tasks); T017 and T018 any time before T019; T019 last.
- Commit 3: T020 `[P]` with nothing else (T021 needs it); T021 after T020; T022 after T021
  (accessors kept, `process` removed only once the systems exist); T023 after T021 + T022; T024
  after T021; T025 any time; T026 → T027 → T028.
- Commit 4: T029 `[P]` T030; T031 after both; T032 after T031; T033 after T032.

## Parallel Example: commit 2

```text
# Three pure/independent files first:
Task: "T009 player/system.rs — tick_decide/tick_integrate/tick_settle/replay_plan + 8 tests"
Task: "T010 player_input/system.rs — input_decide + 5 tests"
Task: "T013 camera_noise_shake.rs — the two pub(crate) accessors"
# then T011 (player/sync.rs) and T012 (player_input/sync.rs) in parallel, then T014/T015 (bridges),
# T016 (registration), T017 (.tscn), T018 (tradeoffs), T019 (gate + harness + commit).
```

## Implementation Strategy

Sequential by design: Setup → commit 1 (the ECS extension, pure — the MVP in the sense that every
later gameplay module needs the seven sets and the `Tuning` resources) → commit 2 (the player and
its input: the tick, the frame run, the `.tscn` edit — the big one) → commit 3 (shake, the trauma
path, remote effects) → STOP 1 → STOP 2 → commit 4. The milestone is not shippable partway:
commit 2 leaves the shake on v2's path and remote `PlayerFx` unconsumed until commit 3.

Suggested `/speckit-implement` session split:

- **Session 1** — Phase 1 + commit 1 (T001–T008).
- **Session 2** — commit 2 (T009–T019). It may be worked in two halves — 2a: T009–T016 code +
  tests + gate; 2b: T017 `.tscn` edit + T018 + T019 headless and the five harness cases — but it
  is ONE commit.
- **Session 3** — commit 3 (T020–T026), ending at 🛑 STOP 1 (T027) and 🛑 STOP 2 (T028).
- **Session 4** — commit 4 (T029–T033).
