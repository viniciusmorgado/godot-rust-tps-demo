# Data Model: Milestone V3-B — the player as one entity over three nodes

Every type is the exact Rust surface the plan commits to; the v2 line each item reproduces is
cited. The three `model.rs` files are UNTOUCHED: their types are reused (wrapped where a bevy
derive is needed) and their 36 tests preserved by name.

## `Phase` (`ecs/setup.rs`, research R4)

```rust
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase { SyncIn, Gameplay, EngineQueryOrient, GameplayIntegrate, EngineQueryMove, GameplaySettle, EngineQuery, SyncOut }
// build_fixed: (SyncIn, Gameplay, EngineQueryOrient, GameplayIntegrate, EngineQueryMove, GameplaySettle, SyncOut).chain()
// build_frame: (SyncIn, Gameplay, EngineQuery, SyncOut).chain()   — V3-A's, unchanged
```

Tests (setup.rs): the three V3-A tests + `fixed_sets_are_chained_in_order_and_frame_sets_unchanged`
(probe systems added in reverse order; asserts both orders and that a component spawned in
`Gameplay` is visible in `EngineQueryOrient` of the same run).

## Resources (`ecs/markers.rs`)

```rust
#[derive(Resource)] pub struct Tuning<T: Send + Sync + 'static>(pub T);   // build_world inserts
// Tuning(PlayerTuning::default()), Tuning(PlayerInputTuning::default()), Tuning(CameraShakeTuning::default())
```

`player/model.rs:14-34`, `player_input/model.rs:6-40`, `camera_noise_shake/model.rs:6-26` provide
the `Default`s; the constitution's "tuning structs become `Res<XxxTuning>`" without editing them.

## Events (`ecs/event.rs`)

```rust
pub enum InboundEvent {                    // V3-A's four variants +
    JumpPressed { root_id: InstanceId },                        // player_input.rs:161-164
    MouseLook { root_id: InstanceId, screen_relative: Vector2 }, // player_input.rs:150-156
    AddTrauma { root_id: InstanceId, amount: f64 },              // player.rs:161-165 (+ red_robot.rs:452)
    PlayerFx { root_id: InstanceId, fx: PlayerFx },              // player.rs:133-159
}
#[derive(Clone, Copy, Debug, PartialEq)] pub enum PlayerFx { Jump, Land, Shoot }   // no Hit: `hit` pushes AddTrauma { 0.75 }
pub enum Initial { …V3-A…, Player { peer_id: i32, simulates: bool, owns_input: bool, initial_position: Vector3, orientation: Transform3D, start_rotation: Vector3 } }
```

`Initial::Player` fields: `player_id` (`player.rs:75-78`), `is_server()` (`:98`), the input
node's authority check (`player_input.rs:61-62`), `initial_position` (`:88`), the model's global
transform with zero origin (`:90-91`), the camera's `ready` capture (`camera_noise_shake.rs:41`).

## Handles (`ecs.rs`, NonSend)

```rust
Handles::Player {
    root: Gd<Player>, input: Gd<PlayerInputSynchronizer>,   // both user classes: their #[var]/#[export] fields are written via bind_mut() (glue); the guard is dropped before any engine call that can invoke a callback
    anim_tree: Gd<AnimationTree>, model: Gd<Node3D>, shoot_from: Gd<Marker3D>,
    shoot_particle: Gd<CpuParticles3D>, muzzle_particle: Gd<CpuParticles3D>, fire_cooldown: Gd<Timer>,
    snd_jump: Gd<AudioStreamPlayer>, snd_land: Gd<AudioStreamPlayer>, snd_shoot: Gd<AudioStreamPlayer>,
    camera_base: Gd<Node3D>, camera_rot: Gd<Node3D>, camera: Gd<Camera3D>,
    camera_anim: Gd<AnimationPlayer>, crosshair: Gd<TextureRect>, color_rect: Gd<ColorRect>,
    noise: [Gd<FastNoiseLite>; 3], bullet_scene: Gd<PackedScene>, parent_rid: Rid,
}
```

`player.rs:48-73` (eleven handles + scene), `player_input.rs:26-29` (RID), `:44-55` (six refs,
cloned through `player_input.bind()`), `camera_noise_shake.rs:17-22` (three noises, cloned through
`camera.cast::<CameraNoiseShake>().bind()`). `root_valid()` checks `root`.

## Components (`ecs/markers.rs` unless noted)

```rust
// identity / authority (registration)
#[derive(Component)] pub struct PlayerTag;
#[derive(Component)] pub struct Simulates;                       // player.rs:98
#[derive(Component)] pub struct OwnsInput;                       // player_input.rs:61-62
#[derive(Component, Clone, Copy)] pub struct PeerId(pub i32);    // player.rs:78
// persistent tick state (player/system.rs)
#[derive(Component, Clone, Copy)] pub struct Motion(pub Vector2);                // player.rs:44
#[derive(Component, Clone, Copy)] pub struct Orientation(pub Transform3D);       // :41
#[derive(Component, Clone, Copy)] pub struct RootMotion(pub Transform3D);        // :42
#[derive(Component, Clone, Copy)] pub struct AirborneTime(pub f32);              // :38-39 (starts 0, backlog #10)
#[derive(Component, Clone, Copy)] pub struct InitialPosition(pub Vector3);       // :46
#[derive(Component, Clone, Copy, PartialEq)] pub struct CurrentAnimation(pub Animations); // :82
#[derive(Component, Clone, Copy)] pub struct AimStateC(pub AimState);            // player_input.rs:22
// queued by the drain (ecs/apply.rs), consumed by one system
#[derive(Component, Default)] pub struct JumpQueued(pub bool);                   // replaces player_input.rs:41 `jumping`
#[derive(Component, Default)] pub struct PendingMouseLook(pub Vec<Vector2>);
#[derive(Component, Default)] pub struct PendingFx(pub Vec<PlayerFx>);
// per-run snapshots (written by SyncIn)
#[derive(Component, Clone, Copy)] pub struct InputFrameC(pub InputFrame);        // player/model.rs:38-48
#[derive(Component, Clone, Copy)] pub struct BodyState { pub on_floor: bool, pub velocity: Vector3, pub gravity: Vector3, pub cooldown_left: f64, pub origin_y: f32 }
#[derive(Component, Clone, Copy)] pub struct ReplicatedInput { pub aiming: bool, pub shoot_target: Vector3, pub motion: Vector2, pub shooting: bool } // player_input.rs:32-39 — frame run, OwnsInput only (the values this peer projects); the fixed run reads the node directly into InputFrameC
#[derive(Component, Clone, Copy)] pub struct ReplayState { pub current_animation: Animations, pub motion: Vector2, pub aim_rotation: f64 } // player.rs:104-117
#[derive(Component, Clone, Copy)] pub struct InputSnapshotC(pub InputSnapshot);  // player_input/model.rs:64-72
#[derive(Component, Clone, Copy)] pub struct CameraFrame { pub parent_y: f32, pub fade_alpha: f32 } // player_input.rs:144-146; the live rotations are read per delta by camera_and_ray
// per-run outputs (research R6)
#[derive(Component, Clone, Copy, Default)] pub struct TickIntents { pub land: bool, pub jump: bool, pub shoot: bool, pub respawn: bool, pub jump_velocity_y: Option<f32>, pub orient: Option<OrientTarget>, pub plan: Option<AnimPlan>, pub read_root_motion: bool }
#[derive(Clone, Copy, Debug, PartialEq)] pub enum OrientTarget { Camera(Quaternion), Walk(Vector3) }   // player.rs:262 / :294
#[derive(Component, Clone, Copy)] pub struct Velocity(pub Vector3);              // integrate output, :316
#[derive(Component, Clone, Default)] pub struct FrameIntents { pub camera_deltas: Vec<Vector2>, pub cue: Option<CameraCue>, pub jump_pressed: bool, pub shooting: bool, pub fade_alpha: f32 }
// shake (camera_noise_shake/system.rs)
#[derive(Component, Clone, Copy)] pub struct Trauma(pub f32);                    // camera_noise_shake.rs:15
#[derive(Component, Clone, Copy)] pub struct ShakeTime(pub f64);                 // :16
#[derive(Component, Clone, Copy)] pub struct StartRotation(pub Vector3);         // :14, :41
#[derive(Component, Clone, Copy, Default)] pub struct ShakePending(pub Option<(f32, f64)>); // (shake, time) from shake_decide; consumed by sync_out_shake (no ShakeOffset, no EngineQuery member)
```

## Systems (signatures; research R7 for the sets and the v2 lines)

```rust
// player/system.rs (pure)
pub fn tick_decide(tuning: Res<Tuning<PlayerTuning>>, dt: Res<FixedDelta>, q: Query<(&InputFrameC, &BodyState, &Orientation, &mut Motion, &mut AirborneTime, &mut TickIntents), With<Simulates>>);
pub fn tick_integrate(dt: Res<FixedDelta>, q: Query<(&BodyState, &RootMotion, &mut Orientation, &mut Velocity, &TickIntents), With<Simulates>>);
pub fn tick_settle(tuning: Res<Tuning<PlayerTuning>>, q: Query<(&BodyState, &mut TickIntents), With<Simulates>>);
pub fn replay_plan(state: &ReplayState) -> AnimPlan;                                   // player.rs:104-117
// player/sync.rs (glue)
fn sync_in_player(input_tuning: Res<Tuning<PlayerInputTuning>>, handles: NonSend<NodeHandles>, q: Query<(Entity, &mut JumpQueued, Has<Simulates>), With<PlayerTag>>, commands: Commands);
fn orient_and_anim(tuning: Res<Tuning<PlayerTuning>>, dt: Res<FixedDelta>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &mut Orientation, &mut RootMotion, &TickIntents, &InputFrameC), With<Simulates>>);   // slerp/looking_at, root-motion read, bullet spawn — NO parameter writes (moved to sync_out_player)
pub(crate) fn apply_anim(anim_tree: &mut Gd<AnimationTree>, plan: AnimPlan);   // v2 player.rs:179-200: the four parameter paths + four transition names (eight consts) live here; called by sync_out_player and apply_player_fx
fn move_body(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Velocity, &mut BodyState), With<Simulates>>);
fn sync_out_player(dt: Res<FixedDelta>, shake_tuning: Res<Tuning<CameraShakeTuning>>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Orientation, &InitialPosition, &Motion, &CurrentAnimation, &TickIntents, &mut Trauma, Has<Simulates>, Option<&ReplayState>), With<PlayerTag>>);   // basis, respawn, projection (bind_mut dropped), local effects on Simulates (+0.35 trauma), rpcs (call_remote), apply_anim(plan | replay plan), advance
fn apply_player_fx(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &mut PendingFx), Without<Simulates>>);   // remote peers only: Jump/Land → apply_anim + node field current_animation via root.bind_mut() (dropped) + sound; Shoot → particles, fire_cooldown.start(), sound
// player_input/system.rs (pure)
pub fn input_decide(tuning: Res<Tuning<PlayerInputTuning>>, dt: Res<FrameDelta>, q: Query<(&InputSnapshotC, &CameraFrame, &mut PendingMouseLook, &mut AimStateC, &mut FrameIntents, &mut ReplicatedInput), With<OwnsInput>>);
// player_input/sync.rs (glue)
fn sync_in_input(handles: NonSend<NodeHandles>, q: Query<(Entity, Has<OwnsInput>), With<PlayerTag>>, commands: Commands);
fn camera_and_ray(tuning: Res<Tuning<PlayerInputTuning>>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &FrameIntents, &mut ReplicatedInput), With<OwnsInput>>);   // clamp_pitch per delta on the live rotation, then the raycast
fn sync_out_input(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &FrameIntents, &ReplicatedInput), With<OwnsInput>>);
// camera_noise_shake/system.rs (pure)
pub fn shake_decide(tuning: Res<Tuning<CameraShakeTuning>>, dt: Res<FrameDelta>, q: Query<(&mut Trauma, &mut ShakeTime, &mut ShakePending), With<PlayerTag>>);
// camera_noise_shake/sync.rs (glue)
fn sync_out_shake(tuning: Res<Tuning<CameraShakeTuning>>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &StartRotation, &ShakePending)>);   // samples + offsets + set_rotation in ONE SyncOut system (no EngineQuery member)
```

`sync_out_player` ends with `anim_tree.advance(dt.0)` for every `PlayerTag` entity (R1, option
B), after the model basis (`player.rs:326`), the respawn reset (`:330-333`), the projection writes
(`root.bind_mut()`, guard dropped), the LOCAL effects on `Simulates` in v2's order (option (b),
FR-004: `Land` sound, `Jump` sound, `Shoot` effects `:146-154` with `Trauma += 0.35`), the
`AnimationTree` parameter writes (`apply_anim`, analyze finding 3) and the
three `rpc` calls — `call_remote` — in v2's order (`:246-251`, `:290`).

## Drain arms (`ecs/apply.rs`, pure)

| Event | Effect on the entity resolved by `root_id` (unknown id dropped) | v2 line |
|---|---|---|
| `JumpPressed` | `JumpQueued.0 = true` | `player_input.rs:163` |
| `MouseLook` | `PendingMouseLook.0.push(screen_relative)` | `:151-154` |
| `AddTrauma` | `Trauma.0 = model::add_trauma(trauma, amount as f32, &tuning)` | `player.rs:163-164`, `camera_noise_shake.rs:64-67` |
| `PlayerFx` | `PendingFx.0.push(fx)`; for `Shoot` also `Trauma.0 = model::add_trauma(trauma, 0.35, &tuning)` at drain time (same-run shake) | `player.rs:133-154` |
| (`hit`) | pushes `AddTrauma { 0.75 }` — no `PlayerFx::Hit` variant | `player.rs:157-159` |

## Bridges

| Class | `ready` | `exit_tree` | Handlers / callbacks | Preserved |
|---|---|---|---|---|
| `Player` (`CharacterBody3D`) | builds `Handles::Player` (its eleven `OnReady` + `bullet_scene`; the six camera refs via `player_input.bind()`; noises + `start_rotation` via `camera_camera.cast::<CameraNoiseShake>().bind()`; `parent_rid` via `player_input.bind().parent_rid`), computes `Initial::Player`, pushes `Register` | `Unregister` | `set_player_id` (unchanged, `:126-131`); `#[rpc] jump/land/shoot/hit` → `PlayerFx`; `#[rpc] pub(crate) add_camera_shake_trauma(f64)` → `AddTrauma` | all `#[export]`/`#[var]`/`#[rpc]` names and types; `Animations` |
| `PlayerInputSynchronizer` (`MultiplayerSynchronizer`) | v2 `:60-70` verbatim (authority → `make_current` + captured mouse; else `set_process_input(false)` + `color_rect.hide()`; `set_process(false)` dropped — no `process`); `root_id: OnReady<InstanceId>` via `get_owner()` | nothing | `input(event)` → `MouseLook`; `#[rpc] jump` → `JumpPressed` | the four `#[export]` fields (now written only by `sync_out_input` through `bind_mut()`), the six `#[export] OnEditor` refs; `jumping` and the three getters removed |
| `CameraNoiseShake` (`Camera3D`) | v2 `:29-42` verbatim (seeding, octaves/lacunarity, `start_rotation`); `noise_seed = randi()` at init unchanged | nothing | none (`add_trauma` removed) | class name; `noise_seed` plain field |

## Tests by name

Preserved (36): `player/model.rs` 16, `player_input/model.rs` 13, `camera_noise_shake/model.rs` 7.

New (the authoritative names are tasks.md's — T006, T007, T009, T010, T020; this list mirrors
them, analyze finding 21):

- `ecs/setup.rs` (2): `fixed_sets_are_chained_in_order_and_frame_sets_unchanged`,
  `marker_inserted_in_gameplay_is_visible_in_engine_query_orient_of_the_same_run`.
- `ecs/apply.rs` (5): `jump_pressed_sets_jump_queued`, `mouse_look_is_queued_in_order`,
  `add_trauma_clamps_at_max`, `player_fx_is_queued_and_shoot_adds_trauma_at_drain`,
  `player_events_for_unknown_root_are_dropped`.
- `player/system.rs` (9): `walk_decides_walk_plan_and_walk_target`,
  `aim_decides_strafe_plan_and_camera_orient`,
  `airborne_decides_jump_plan_and_skips_orient_and_root_motion`,
  `jump_step_plans_jump_up_from_the_post_jump_velocity` (analyze finding 1),
  `land_and_jump_intents_can_both_fire`, `shoot_fires_only_when_aiming_and_cooldown_zero`,
  `integrate_updates_orientation_and_velocity`, `settle_flags_respawn_below_threshold`,
  `replay_builds_v2_plans_for_all_four_animations`.
- `player_input/system.rs` (5): `controller_look_is_scaled`,
  `mouse_look_is_applied_before_controller_look`, `aim_hold_and_toggle_reach_v2_cues`,
  `fade_alpha_follows_height`, `jump_just_pressed_sets_intent_for_one_frame`.
- `camera_noise_shake/system.rs` (2): `shake_runs_only_while_trauma_positive`,
  `decay_and_time_advance_in_v2_order`.

Total: 156 + 23 = 179 (163 after commit 1, 177 after commit 2, 179 after commit 3).
