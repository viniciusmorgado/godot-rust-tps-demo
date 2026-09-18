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
#[derive(Clone, Copy, Debug, PartialEq)] pub enum PlayerFx { Jump, Land, Shoot, Hit }
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
#[derive(Component, Clone, Copy)] pub struct ReplicatedInput { pub aiming: bool, pub shoot_target: Vector3, pub motion: Vector2, pub shooting: bool } // player_input.rs:32-39
#[derive(Component, Clone, Copy)] pub struct ReplayState { pub current_animation: Animations, pub motion: Vector2, pub aim_rotation: f64 } // player.rs:104-117
#[derive(Component, Clone, Copy)] pub struct InputSnapshotC(pub InputSnapshot);  // player_input/model.rs:64-72
#[derive(Component, Clone, Copy)] pub struct CameraFrame { pub rot_x: f32, pub parent_y: f32, pub fade_alpha: f32 } // player_input.rs:144-146, :188
// per-run outputs (research R6)
#[derive(Component, Clone, Copy, Default)] pub struct TickIntents { pub land: bool, pub jump: bool, pub shoot: bool, pub respawn: bool, pub jump_velocity_y: Option<f32>, pub orient: Option<OrientTarget>, pub plan: Option<AnimPlan>, pub read_root_motion: bool }
#[derive(Clone, Copy, Debug, PartialEq)] pub enum OrientTarget { Camera(Quaternion), Walk(Vector3) }   // player.rs:262 / :294
#[derive(Component, Clone, Copy)] pub struct Velocity(pub Vector3);              // integrate output, :316
#[derive(Component, Clone, Default)] pub struct FrameIntents { pub camera_deltas: Vec<Vector2>, pub cue: Option<CameraCue>, pub jump_pressed: bool, pub shooting: bool, pub fade_alpha: f32 }
// shake (camera_noise_shake/system.rs)
#[derive(Component, Clone, Copy)] pub struct Trauma(pub f32);                    // camera_noise_shake.rs:15
#[derive(Component, Clone, Copy)] pub struct ShakeTime(pub f64);                 // :16
#[derive(Component, Clone, Copy)] pub struct StartRotation(pub Vector3);         // :14, :41
#[derive(Component, Clone, Copy, Default)] pub struct ShakePending(pub Option<(f32, f64)>); // (shake, time) from shake_decide
#[derive(Component, Clone, Copy, Default)] pub struct ShakeOffset(pub Option<Vector3>);     // from shake_sample
```

## Systems (signatures; research R7 for the sets and the v2 lines)

```rust
// player/system.rs (pure)
pub fn tick_decide(tuning: Res<Tuning<PlayerTuning>>, dt: Res<FixedDelta>, q: Query<(&InputFrameC, &BodyState, &Orientation, &mut Motion, &mut AirborneTime, &mut TickIntents), With<Simulates>>);
pub fn tick_integrate(dt: Res<FixedDelta>, q: Query<(&BodyState, &RootMotion, &mut Orientation, &mut Velocity, &TickIntents), With<Simulates>>);
pub fn tick_settle(tuning: Res<Tuning<PlayerTuning>>, q: Query<(&BodyState, &mut TickIntents), With<Simulates>>);
pub fn replay_plan(state: &ReplayState) -> AnimPlan;                                   // player.rs:104-117
// player/sync.rs (glue)
fn sync_in_player(handles: NonSend<NodeHandles>, q: Query<(Entity, &mut JumpQueued, Has<Simulates>), With<PlayerTag>>, commands: Commands);
fn orient_and_anim(tuning, dt, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &mut Orientation, &mut RootMotion, &mut CurrentAnimation, &TickIntents, &InputFrameC, Has<Simulates>, Option<&ReplayState>), With<PlayerTag>>);
fn move_body(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Velocity, &mut BodyState), With<Simulates>>);
fn sync_out_player(dt: Res<FixedDelta>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Orientation, &InitialPosition, &Motion, &CurrentAnimation, &TickIntents, Has<Simulates>), With<PlayerTag>>);
fn apply_player_fx(tuning: Res<Tuning<CameraShakeTuning>>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &mut PendingFx, &mut Trauma, &mut CurrentAnimation, Has<Simulates>)>);
// player_input/system.rs (pure)
pub fn input_decide(tuning: Res<Tuning<PlayerInputTuning>>, dt: Res<FrameDelta>, q: Query<(&InputSnapshotC, &CameraFrame, &mut PendingMouseLook, &mut AimStateC, &mut FrameIntents, &mut ReplicatedInput), With<OwnsInput>>);
// player_input/sync.rs (glue)
fn sync_in_input(handles: NonSend<NodeHandles>, q: Query<(Entity, Has<OwnsInput>), With<PlayerTag>>, commands: Commands);
fn camera_and_ray(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &FrameIntents, &mut ReplicatedInput), With<OwnsInput>>);
fn sync_out_input(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &FrameIntents, &ReplicatedInput), With<OwnsInput>>);
// camera_noise_shake/system.rs (pure)
pub fn shake_decide(tuning: Res<Tuning<CameraShakeTuning>>, dt: Res<FrameDelta>, q: Query<(&mut Trauma, &mut ShakeTime, &mut ShakePending), With<PlayerTag>>);
// camera_noise_shake/sync.rs (glue)
fn shake_sample(tuning, handles: NonSend<NodeHandles>, q: Query<(Entity, &ShakePending, &mut ShakeOffset)>);
fn sync_out_shake(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &StartRotation, &ShakeOffset)>);
```

`sync_out_player` ends with `anim_tree.advance(dt.0)` for every `PlayerTag` entity (R1, option
B), after the model basis (`player.rs:326`), the respawn reset (`:330-333`), the projection writes
(`root.bind_mut()`, guard dropped), the LOCAL effects on `Simulates` in v2's order (option (b),
FR-004: `Land` sound, `Jump` sound, `Shoot` effects `:146-154` with `Trauma += 0.35`) and the
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

New:

- `ecs/setup.rs`: `fixed_sets_are_chained_in_order_and_frame_sets_unchanged`.
- `ecs/apply.rs`: `jump_pressed_sets_jump_queued`, `mouse_look_is_queued_on_the_entity_in_order`,
  `add_trauma_applies_model_clamp`, `player_fx_is_queued_in_order`.
- `player/system.rs` (tick, ≥ 6): `tick_decide_walking_sets_walk_plan_and_walk_target`,
  `tick_decide_aiming_sets_strafe_plan_and_camera_target`,
  `tick_decide_airborne_sets_jump_plan_and_skips_root_motion_read`,
  `tick_decide_jump_sets_velocity_and_land_jump_intents_in_v2_order`,
  `tick_decide_fires_only_when_shooting_with_zero_cooldown`,
  `tick_integrate_matches_model_integrate_root_motion`,
  `tick_settle_flags_respawn_below_threshold_only`, `replay_plan_maps_current_animation_as_v2`.
- `player_input/system.rs` (≥ 3): `input_decide_orders_mouse_deltas_before_controller_delta`,
  `input_decide_aim_state_and_cue_match_step_aim`, `input_decide_sets_jump_pressed_and_shooting`,
  `input_decide_fade_alpha_matches_model`.
- `camera_noise_shake/system.rs` (≥ 2): `shake_decide_is_idle_without_trauma`,
  `shake_decide_decay_time_and_shake_match_model`.

Total ≥ 156 + 19.
