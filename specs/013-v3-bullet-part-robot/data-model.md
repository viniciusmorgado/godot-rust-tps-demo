# Data Model: Milestone V3-C — the enemy over the ECS core

Every type is the exact Rust surface the plan commits to; the v2 line each item reproduces is
cited. The three pure cores are UNTOUCHED (`bullet.rs`'s `mod pure` and `part.rs`'s `mod pure`
gain only the `pub(crate)` visibility token; `red_robot/model.rs` is byte-identical); their 33
tests are preserved by name.

## `hittable.rs` (additive, research R3)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind { Player(InstanceId), Robot(InstanceId) }      // the Send projection of HitTarget
pub fn kind_of(node: Gd<Node3D>) -> Option<HitKind>;           // resolve (:16-24) + instance_id()
impl HitKind {
    pub fn rpc_hit(self);            // try_from_instance_id → HitTarget::rpc_hit (:30-39); freed target = no-op
    pub fn robot_id(self) -> Option<InstanceId>;
}
```

## Resources (`ecs/markers.rs`, `ecs/setup.rs`)

`Tuning(RobotTuning::default())` inserted by `build_world` (`red_robot/model.rs:15-48`);
`Messages::<RobotHitLocal>::default()` inserted by `build_world`, `update()`d once at the start
of every FIXED run before the drain (`ecs.rs`, next to `DoorBodyEntered`).

## Events and message (`ecs/event.rs`)

```rust
pub enum InboundEvent {                    // V3-A + V3-B variants +
    BulletFx { root_id: InstanceId, fx: BulletFx },        // bullet.rs:137-146 (remote peers)
    BulletDestroy { root_id: InstanceId },                 // bullet.rs:148-154 (method track, every peer)
    PartFx { root_id: InstanceId, fx: PartFx },            // part.rs:194-215 (remote peers)
    RobotHit { root_id: InstanceId },                      // red_robot.rs:276-328 (remote peers, option (B))
    RobotFx { root_id: InstanceId, fx: RobotFx },          // red_robot.rs:330-333 (remote peers)
    ShootRequested { root_id: InstanceId },                // red_robot.rs:335-338 (method track)
    ResumeApproachRequested { root_id: InstanceId },       // red_robot.rs:267-274 (method track)
    RobotPlayerSeen { root_id: InstanceId, player: Option<InstanceId> },   // red_robot.rs:340-358
}
#[derive(Clone, Copy, Debug, PartialEq)] pub enum BulletFx { Explode }
#[derive(Clone, Copy, Debug, PartialEq)] pub enum PartFx { Destroy }
#[derive(Clone, Copy, Debug, PartialEq)] pub enum RobotFx { PlayShoot }
pub enum Initial { …,
    Bullet { shadow_mapping: bool },                                          // bullet.rs:143 (Settings read once)
    Part { lifetime: f32, lifetime_random: f32, disappearing_time: f32 },     // part.rs:86-94
    Robot { state: State, health: i32, dead: bool, test_shoot: bool, orientation: Transform3D, aim_blend: Vector2 }, // red_robot.rs:35-47, :110-117; aim_blend = the tree's parameters/aim/blend_position at ready (red_robot.tscn:10785)
}
/// The same-run hit (research R4): written by bullet_settle, read by robot_hit_apply.
#[derive(Message, Clone, Copy)] pub struct RobotHitLocal { pub robot: InstanceId }
```

`Initial` stays `Copy` (`State` is `Copy`).

## Handles (`ecs.rs`, NonSend; boxed like `PlayerHandles`)

```rust
Handles::Bullet(Box<BulletHandles>)  // root: Gd<Bullet>, anim: Gd<AnimationPlayer>, collision: Gd<CollisionShape3D>, light: Gd<OmniLight3D>   (bullet.rs:71-76)
Handles::Part(Box<PartHandles>)      // root: Gd<Part>, synchronizer: Gd<MultiplayerSynchronizer>, col1: Gd<CollisionShape3D>, col2: Gd<CollisionShape3D>, puff_scene: Gd<PackedScene>   (part.rs:101-118)
Handles::Robot(Box<RobotHandles>)    // root: Gd<EnemyRobot>, anim_tree: Gd<AnimationTree>, shoot_anim: Gd<AnimationPlayer>, model: Gd<Node3D>, ray_from: Gd<BoneAttachment3D>, ray_mesh: Gd<MeshInstance3D>, laser_raycast: Gd<RayCast3D>, laser_ember: Gd<CpuParticles3D>, collision_shape: Gd<CollisionShape3D>, explosion_sound: Gd<AudioStreamPlayer3D>, hit_sound: Gd<AudioStreamPlayer3D>, death: Gd<Node3D>, parts: [Gd<Part>; 3], sparks: [Gd<CpuParticles3D>; 2], impact_effect_scene: Gd<PackedScene>, rid: Rid, is_dedicated_server: bool   (red_robot.rs:62-105)
```

`root_valid()` checks `root`; `sync_out_remove` gains the three `queue_free` arms. `root` is the
USER class on all three (projection fields through `bind_mut()`: the bullet has none but its
`Gd<Bullet>` derefs to `CharacterBody3D` for `move_and_collide`; the part's `set_fade_value`;
the robot's five replicated fields) — the guard-dropped-before-engine-call rule applies.

## Components (`ecs/markers.rs` unless noted)

```rust
// bullet (bullet/system.rs)
#[derive(Component)] pub struct BulletTag;
#[derive(Component, Clone, Copy, PartialEq)] pub struct BulletStateC(pub BulletState);   // bullet.rs:14-18, :68
#[derive(Component, Clone, Copy)] pub struct BulletBasisZ(pub Vector3);                  // :112 (SyncIn snapshot)
#[derive(Component, Clone, Copy, Default)] pub struct Collided { pub hit: Option<HitKind>, pub collided: bool }  // :114-116 (EngineQueryMove)
#[derive(Component, Clone, Copy, Default)] pub struct BulletIntents { pub active: bool, pub explode: bool, pub hit: Option<HitKind>, pub disable_collision: bool }  // :98-131
#[derive(Component, Default)] pub struct PendingBulletFx(pub Vec<BulletFx>);
// part (part/system.rs)
#[derive(Component)] pub struct PartTag;
#[derive(Component, Debug, PartialEq)] pub enum PartPhase { Attached, Waiting(Timer), Fading { counter: f32 }, Destroyed(Timer) }  // part.rs:162-215
#[derive(Component, Clone, Copy)] pub struct PartLifetimes { pub lifetime: f32, pub lifetime_random: f32, pub disappearing_time: f32 }  // :86-94
#[derive(Component, Clone, Copy, Default)] pub struct PartIntents { pub fade: Option<f32>, pub destroy: bool }   // :139-144
#[derive(Component, Default)] pub struct PendingPartFx(pub Vec<PartFx>);
// robot (red_robot/system.rs)
#[derive(Component)] pub struct RobotTag;
#[derive(Component)] pub struct Dead;                                                     // red_robot.rs:47 (marker: Without<Dead> tick)
#[derive(Component, Clone, Copy, PartialEq)] pub struct RobotState(pub State);            // :45
#[derive(Component, Clone, Copy)] pub struct Health(pub i32);                             // :42
#[derive(Component, Clone, Copy)] pub struct TargetPosition(pub Vector3);                 // :39
#[derive(Component, Clone, Copy)] pub struct RobotCountersC(pub RobotCounters);           // :49-55 (aim_preparing inside)
#[derive(Component, Clone, Copy)] pub struct TrackedPlayer(pub Option<InstanceId>);      // :58 (no Gd)
#[derive(Component, Clone, Copy)] pub struct AimBlend(pub Vector2);                       // :482-485 → component (FR-016)
#[derive(Component)] pub struct ShootRequested;                                           // :35-36, :335-338 (marker from the drain)
#[derive(Component, Clone, Copy)] pub struct RobotFrame { pub global_transform: Transform3D, pub gravity: Vector3, pub velocity: Vector3, pub player_origin: Option<Vector3>, pub ray_from: Transform3D, pub ray_mesh: Transform3D, pub ray_mesh_z: f32, pub laser_colliding: bool, pub laser_point: Vector3 }  // SyncIn snapshot
#[derive(Component, Clone, Copy)] pub struct ReplayRobot { pub state: State, pub target_position: Vector3, pub aim_preparing: f32 }  // :134 (non-Simulates)
#[derive(Component, Clone, Default)] pub struct RobotIntents { pub idle_branch: bool, pub raycast: Option<(Vector3, Vector3)>, pub shoot: bool, pub clip: Option<f32>, pub play_shoot: bool, pub anim: Option<AnimDecision>, pub hit: bool, pub just_died: bool }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct AnimDecision { pub request: &'static str, pub aim: Option<(f32, Vector2)> }   // :469-488 (blend_amount, blend_position)
#[derive(Component, Clone, Copy, Default)] pub struct RaycastAnswers { pub sees_player: Option<bool>, pub shot: Option<ShotResult> }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct ShotResult { pub max_dist: f32, pub hit: Option<(Vector3, Option<InstanceId>)> }   // :405-409, :428-432
#[derive(Component, Default)] pub struct PendingTrauma(pub Option<(Timer, InstanceId)>);   // :437-453 (frame schedule)
#[derive(Component, Default)] pub struct RemovalTimer(pub Option<Timer>);                // :309-326 (frame schedule)
#[derive(Component)] pub struct TraumaDue(pub InstanceId);                               // robot_timers → sync_out_robot_frame
#[derive(Component, Default)] pub struct PendingRobotFx(pub Vec<RobotFx>);
#[derive(Component, Default)] pub struct PendingRobotHits(pub u32);                      // remote RobotHit count (frame run)
// reused from V3-B: Orientation, RootMotion, Velocity, Simulates
```

`Timer` is V3-A's (`ecs/timer.rs`: `Timer::new(seconds)`, `step(dt) -> bool` once).

## Registration (`apply_register` arms)

- `Initial::Bullet { shadow_mapping }` → `BulletTag`, `BulletStateC(Flying { time_alive: 5.0 })`
  (`bullet.rs:68`), `BulletBasisZ(Vector3::ZERO)`, `Collided::default()`, `BulletIntents::default()`,
  `PendingBulletFx::default()`, `Simulates` iff server; `shadow_mapping` is kept in `BulletHandles`
  (glue-only value) — the plan stores it there, not as a component.
- `Initial::Part { … }` → `PartTag`, `PartPhase::Attached`, `PartLifetimes`, `PartIntents::default()`,
  `PendingPartFx::default()`, `Simulates` iff server.
- `Initial::Robot { state, health, dead, test_shoot, orientation, aim_blend }` → `RobotTag`,
  `RobotState(state)`, `Health(health)`, `Dead` iff `dead` (`:119-123`), `TargetPosition(ZERO)`,
  `RobotCountersC(RobotCounters { aim_preparing: aim_prepare_time, shoot_countdown: if test_shoot
  { 0.0 } else { shoot_wait }, aim_countdown: aim_time })` (`:49-55`, `:115-117`),
  `TrackedPlayer(None)`, `Orientation(orientation)`, `RootMotion(IDENTITY)`, `Velocity(ZERO)`,
  `AimBlend(aim_blend)`, `RobotIntents::default()`, `RaycastAnswers::default()`,
  `PendingTrauma::default()`, `RemovalTimer::default()`, `PendingRobotFx::default()`,
  `PendingRobotHits::default()`, `ShootRequested` iff `test_shoot` (v2 `:138-141` fires on the
  first step), `Simulates` iff server.

## Drain arms (`ecs/apply.rs`, pure; unknown id dropped)

| Event | Effect | v2 line |
|---|---|---|
| `BulletFx { Explode }` | `PendingBulletFx.0.push(fx)` | `bullet.rs:137-146` |
| `BulletDestroy` | `Remove` inserted iff the entity has `Simulates` | `:150-153` |
| `PartFx { Destroy }` | `PendingPartFx.0.push(fx)` | `part.rs:194-215` |
| `RobotHit` | `PendingRobotHits.0 += 1` (remote peers only; the local path is `RobotHitLocal`) | `red_robot.rs:276-328` |
| `RobotFx { PlayShoot }` | `PendingRobotFx.0.push(fx)` | `:330-333` |
| `ShootRequested` | insert the `ShootRequested` marker | `:335-338` |
| `ResumeApproachRequested` | `RobotState = Approach`; counters `aim_preparing`, `shoot_countdown` = `resume_approach_reset` | `:267-274` |
| `RobotPlayerSeen { player }` | `TrackedPlayer = player`; `RobotState = Approach` on `Some`, `Idle` on `None`; counters untouched (backlog #31 deferred, FR-023) | `:340-358` |

## Systems (signatures; research R6/R7 for the sets and the v2 lines)

```rust
// bullet/system.rs (pure)
pub fn bullet_step(dt: Res<FixedDelta>, q: Query<(&mut BulletStateC, &mut BulletIntents), With<Simulates>>);              // :98-107
pub fn bullet_settle(q: Query<(&Collided, &mut BulletStateC, &mut BulletIntents), With<Simulates>>, w: MessageWriter<RobotHitLocal>);  // :114-131
// bullet/sync.rs (glue)
fn sync_in_bullet(handles: NonSend<NodeHandles>, q: Query<Entity, (With<BulletTag>, With<Simulates>)>, commands: Commands);          // :112
fn move_bullet(dt: Res<FixedDelta>, handles: NonSendMut<NodeHandles>, q: Query<(Entity, &BulletBasisZ, &BulletIntents, &mut Collided), With<Simulates>>);  // :111-116
fn sync_out_bullet(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &BulletIntents), With<Simulates>>);                          // :105-107, :117-127, :137-146
fn sync_out_bullet_frame(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &mut PendingBulletFx), Without<Simulates>>);           // :137-146 (remote)
// part/system.rs (pure)
pub fn part_phase_tick(dt: Res<FrameDelta>, q: Query<(Entity, &mut PartPhase, &PartLifetimes, &mut PartIntents)>, commands: Commands);   // :138-146, :179-191, :202-214
// part/sync.rs (glue)
fn sync_out_part(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &PartIntents, &mut PendingPartFx, &mut PartPhase, Has<Simulates>), With<PartTag>>);  // :140, :151-160, :196-200
pub(crate) fn puff_parent(root: &Gd<Part>) -> Gd<Node>;                                                                              // :224-232
// red_robot/system.rs (pure)
pub fn robot_decide(tuning: Res<Tuning<RobotTuning>>, dt: Res<FixedDelta>, q: Query<(Entity, &RobotFrame, &RobotState, &RobotCountersC, &TrackedPlayer, &mut TargetPosition, &mut RobotIntents, Has<ShootRequested>), (With<Simulates>, Without<Dead>)>, commands: Commands);  // :138-151, :160-184, :197-223
pub fn robot_step_and_animate(tuning, dt, q: Query<(&RobotFrame, &RaycastAnswers, &mut RobotState, &mut RobotCountersC, &TargetPosition, &mut AimBlend, &mut RobotIntents, &RootMotion, &mut Orientation, &mut Velocity), (With<Simulates>, Without<Dead>)>);  // :186-196, :225-235, :238 → :456-490, :245-251, :146
pub fn robot_replay(tuning, dt, q: Query<(&ReplayRobot, &RobotFrame, &mut AimBlend, &mut RobotIntents), (Without<Simulates>, Without<Dead>)>);  // :134 → :456-490
pub fn robot_hit_apply(r: MessageReader<RobotHitLocal>, index: Res<EntityIndex>, q: Query<(&mut Health, &mut RobotIntents, Has<Dead>), With<RobotTag>>);   // :278-290 (R4)
pub fn robot_timers(dt: Res<FrameDelta>, q: Query<(Entity, &mut PendingTrauma, &mut RemovalTimer)>, commands: Commands);              // :437-453, :309-326
// red_robot/sync.rs (glue)
fn sync_in_robot(handles: NonSend<NodeHandles>, q: Query<(Entity, &TrackedPlayer, Has<Simulates>), (With<RobotTag>, Without<Dead>)>, commands: Commands);
fn robot_query(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &RobotIntents, &TrackedPlayer, &mut RaycastAnswers, &mut RootMotion), (With<Simulates>, Without<Dead>)>);   // :363-382, :399-409, :241-244
fn move_robot(handles: NonSendMut<NodeHandles>, q: Query<(Entity, &Velocity), (With<Simulates>, Without<Dead>)>);                    // :147-149, :253-255
fn sync_out_robot(dt: Res<FixedDelta>, index: Res<EntityIndex>, handles: NonSendMut<NodeHandles>, robots: RobotSyncOutQuery, parts: Query<(&mut PartPhase, &PartLifetimes), (With<PartTag>, Without<RobotTag>)>, commands: Commands);   // R5/R6
fn sync_out_robot_frame(handles: NonSendMut<NodeHandles>, q: Query<(Entity, Option<&TraumaDue>, &mut PendingRobotFx, &mut PendingRobotHits, Has<Simulates>), With<RobotTag>>, commands: Commands);
```

`sync_out_robot` order per entity (R6): `Simulates` && !`Dead`: `set_global_basis` unless
`idle_branch`; projection (`state`, `target_position`, `health`, `aim_preparing`) via
`p.root.bind_mut()` (dropped); `play_shoot` local + `rpc("play_shoot")`; shoot effects (clip,
ember, blast, `PendingTrauma`); the Aim/Shooting clip; `hit` → reaction `randi()` + parameter +
sound; `just_died` → the death branch (R5: `dead` projection, tree inactive, model hidden, `Death`
visible, collision off, sparks, the thirteen draws and the three parts, explosion sound,
`exploded` emit, `RemovalTimer(10 s)` on the server, `Dead` inserted via `Commands`); every
!`Dead` robot: the tree parameter writes from `intents.anim` then `advance(dt.0)` LAST.

## Bridges

| Class | `ready` | `exit_tree` | Handlers / callbacks | Preserved |
|---|---|---|---|---|
| `Bullet` (`CharacterBody3D`) | v2 `:88-93` (collision off on non-server; no `set_physics_process`), reads `settings.bind().graphics().shadow_mapping` once, pushes `Register` | `Unregister` | `#[rpc(authority, call_remote, unreliable)] explode` → `BulletFx::Explode`; `#[func] destroy` → `BulletDestroy` | `VELOCITY`; `explode`/`destroy` names; `mod pure` |
| `Part` (`RigidBody3D`) | v2 `:123-136` verbatim minus `set_process(false)` (upstream bug fix #2 comment kept), pushes `Register` with the three exports | `Unregister` | `#[func] pub(crate) set_fade_value` (setter, unchanged body); `#[rpc(authority, call_remote, unreliable)] destroy` → `PartFx::Destroy`; `explode` REMOVED (Complexity Tracking) | the four `#[export]`s; `destroy`, `set_fade_value` names |
| `EnemyRobot` (`CharacterBody3D`) | v2 `:110-125` minus `animate(0.0)` (subsumed by the first fixed run, R1), plus `aim_blend = anim_tree.get("parameters/aim/blend_position")`, pushes `Register` | `Unregister` | `#[signal] exploded` (emitted by `sync_out_robot`); `#[func] resume_approach` → `ResumeApproachRequested`; `#[rpc(authority, call_remote, unreliable)] hit` → `RobotHit`; `#[rpc(authority, call_remote, unreliable)] play_shoot` → `RobotFx::PlayShoot`; `#[func] shoot_check` → `ShootRequested`; `#[func] _on_area_body_entered/exited` → `RobotPlayerSeen` after `try_cast::<Player>` | the `#[export]`s, `#[var]`s (`test_shoot` written by nothing at runtime; `aim_preparing` projected), all names, `State` |

## Tests by name

Preserved (33): `bullet.rs` 3, `part.rs` 5, `red_robot/model.rs` 25.

New:

- `ecs/apply.rs` (8): `bullet_fx_explode_is_queued`, `bullet_destroy_marks_remove_only_on_simulates`,
  `part_fx_destroy_is_queued`, `robot_hit_is_counted_for_remote_peers`, `robot_fx_play_shoot_is_queued`,
  `shoot_requested_inserts_marker`, `resume_approach_requested_resets_state_and_counters`,
  `robot_player_seen_sets_tracked_player_and_state`.
- `bullet/system.rs` (5): `step_keeps_flying_and_counts_down`, `expiry_sets_explode_intent`,
  `collision_sets_hit_disables_and_explodes`, `expiry_and_collision_same_step_explode_once`,
  `exploded_bullet_is_inactive`.
- `part/system.rs` (5): `attached_part_does_nothing`, `waiting_timer_expiry_starts_fading`,
  `fading_writes_fade_curve_each_frame`, `fading_destroys_at_t_minus_0_2_once`,
  `destroyed_timer_expiry_marks_remove`.
- `red_robot/system.rs` (14): `no_player_branch_uses_idle_velocity_and_zero_target`,
  `approach_requests_raycast_only_when_facing_and_countdown_expiring`,
  `approach_to_aim_when_raycast_sees_player`, `aim_branch_clips_at_1000_when_laser_not_colliding`,
  `aim_to_shooting_emits_play_shoot`, `aim_lost_resumes_approach`,
  `animation_decided_from_the_post_step_state`, `aim_blend_steps_from_the_component`,
  `integrate_matches_model_twin`, `shoot_requested_sets_shoot_intent_once`,
  `replay_builds_animation_from_replicated_fields`, `robot_hit_apply_ignores_dead_robot`,
  `pending_trauma_expiry_flags_trauma_due`, `removal_timer_expiry_marks_remove`.
- `ecs/setup.rs` (1): `robot_hit_apply_runs_after_bullet_settle_in_the_same_run` (R4).

Total = 179 + 8 (commit 1) + 5 (commit 2) + 20 (commit 3) = 212.
