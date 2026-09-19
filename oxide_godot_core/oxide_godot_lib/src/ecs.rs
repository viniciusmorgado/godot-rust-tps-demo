//! ECS layer over the nodes (constitution 1.5.1, Principle III "ECS shape (v3)").
//!
//! Glue module: the engine-facing types (`Handles`, `NodeHandles`) live here because they name
//! `Gd<T>`; the pure core lives in the `ecs/*` submodules and is tested without Godot.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use godot::classes::{
    AnimationPlayer, AnimationTree, Area3D, AudioStreamPlayer, AudioStreamPlayer3D, BoneAttachment3D,
    Camera3D, CollisionShape3D, ColorRect, CpuParticles3D, FastNoiseLite, INode, Marker3D,
    MeshInstance3D, MultiplayerSynchronizer, Node, Node3D, OmniLight3D, PackedScene, RayCast3D,
    TextureRect, Timer,
};
use godot::obj::InstanceId;
use godot::prelude::*;

use event::{DoorBodyEntered, InboundEvent, Initial, RobotHitLocal};
use index::{EntityIndex, Registration};
use markers::{
    AimBlend, AimStateC, AirborneTime, BlastTag, BulletBasisZ, BulletIntents, BulletStateC,
    BulletTag, Collided, CurrentAnimation, Dead, FixedDelta, FrameDelta, FrameIntents, Health,
    InitialPosition, JumpQueued, LookTarget, Motion, Orientation, OwnsInput, PartIntents,
    PartLifetimes, PartPhase, PartTag, PeerId, PendingBulletFx, PendingFx, PendingMouseLook,
    PendingPartFx, PendingRobotFx, PendingRobotHits, PendingTrauma, PlayOpen, PlayerTag,
    RaycastAnswers, RemovalTimer, Remove, ReplicatedInput, RobotCountersC, RobotIntents,
    RobotState, RobotTag, RootMotion, ShakePending, ShakeTime, ShootRequested, Simulates,
    StartEmitting, StartRotation, TargetPosition, TickIntents, TrackedPlayer, Trauma, Tuning,
    Velocity,
};
use setup::Phase;

use crate::bullet::pure::BulletState;
use crate::bullet::Bullet;
use crate::door::system::DoorState;
use crate::part::Part;
use crate::part_disappear::system::{DisappearPhase, Lifetime};
use crate::player::{Animations, Player};
use crate::player_input::model::AimState;
use crate::player_input::PlayerInputSynchronizer;
use crate::red_robot::model::{RobotCounters, RobotTuning};
use crate::red_robot::EnemyRobot;

pub mod apply;
pub mod event;
pub mod index;
pub mod markers;
pub mod queue;
pub mod setup;
pub mod timer;

/// The engine handles a bridge resolved ONCE in `ready` (research.md R4), typed per bridge kind.
/// `Gd<T>` is `!Send`, so these never live in a component: only in `NodeHandles`, a `NonSend`
/// resource read by the sync systems.
pub enum Handles {
    Door {
        root: Gd<Area3D>,
        anim: Gd<AnimationPlayer>,
    },
    Puff {
        root: Gd<CpuParticles3D>,
    },
    Blast {
        root: Gd<Node3D>,
        light_rays: Gd<CpuParticles3D>,
        camera: Option<Gd<Camera3D>>,
    },
    /// The player entity over three nodes (specs/012 research R5), boxed: twenty handles would
    /// otherwise dwarf the other variants (clippy `large_enum_variant`, gate at zero warnings).
    Player(Box<PlayerHandles>),
    /// The bullet entity (specs/013 data-model.md), boxed like the player's.
    Bullet(Box<BulletHandles>),
    /// A part entity — one per part node of a robot (specs/013 data-model.md).
    Part(Box<PartHandles>),
    /// The robot entity (specs/013 data-model.md).
    Robot(Box<RobotHandles>),
}

/// `Handles::Bullet`'s payload (v2 `bullet.rs:71-79`). `shadow_mapping` is the `Settings` read of
/// `:143`, done once in `ready` — a glue-only value, so it lives here and not in a component.
pub struct BulletHandles {
    pub root: Gd<Bullet>,
    pub anim: Gd<AnimationPlayer>,
    pub collision: Gd<CollisionShape3D>,
    pub light: Gd<OmniLight3D>,
    pub shadow_mapping: bool,
}

/// `Handles::Part`'s payload (v2 `part.rs:101-118`). `root` is the user class: `set_fade_value`
/// (the projection setter) is reached through `bind_mut()`.
pub struct PartHandles {
    pub root: Gd<Part>,
    pub synchronizer: Gd<MultiplayerSynchronizer>,
    pub col1: Gd<CollisionShape3D>,
    pub col2: Gd<CollisionShape3D>,
    pub puff_scene: Gd<PackedScene>,
}

/// `Handles::Robot`'s payload (v2 `red_robot.rs:62-105`): the twelve `OnReady` handles, the
/// three parts (for the death branch, research R5), the two sparks, the impact scene, the RID and
/// the dedicated-server flag read once. `root` is the user class: the five replicated fields
/// are projected through `bind_mut()`, the guard dropped before any engine call.
pub struct RobotHandles {
    pub root: Gd<EnemyRobot>,
    pub anim_tree: Gd<AnimationTree>,
    pub shoot_anim: Gd<AnimationPlayer>,
    pub model: Gd<Node3D>,
    pub ray_from: Gd<BoneAttachment3D>,
    pub ray_mesh: Gd<MeshInstance3D>,
    pub laser_raycast: Gd<RayCast3D>,
    pub laser_ember: Gd<CpuParticles3D>,
    pub collision_shape: Gd<CollisionShape3D>,
    pub explosion_sound: Gd<AudioStreamPlayer3D>,
    pub hit_sound: Gd<AudioStreamPlayer3D>,
    pub death: Gd<Node3D>,
    pub parts: [Gd<Part>; 3],
    pub sparks: [Gd<CpuParticles3D>; 2],
    pub impact_effect_scene: Gd<PackedScene>,
    pub rid: Rid,
    pub is_dedicated_server: bool,
}

/// `Handles::Player`'s payload. `root` and `input` are the USER classes because their
/// `#[var]`/`#[export]` projection fields are written through `bind_mut()` (the guard is dropped
/// before any engine call that can invoke a callback); `Deref` reaches `CharacterBody3D`/`Node`
/// for the engine calls.
pub struct PlayerHandles {
    pub root: Gd<Player>,
    pub input: Gd<PlayerInputSynchronizer>,
    pub anim_tree: Gd<AnimationTree>,
    pub model: Gd<Node3D>,
    pub shoot_from: Gd<Marker3D>,
    pub shoot_particle: Gd<CpuParticles3D>,
    pub muzzle_particle: Gd<CpuParticles3D>,
    pub fire_cooldown: Gd<Timer>,
    pub snd_jump: Gd<AudioStreamPlayer>,
    pub snd_land: Gd<AudioStreamPlayer>,
    pub snd_shoot: Gd<AudioStreamPlayer>,
    pub camera_base: Gd<Node3D>,
    pub camera_rot: Gd<Node3D>,
    pub camera: Gd<Camera3D>,
    pub camera_anim: Gd<AnimationPlayer>,
    pub crosshair: Gd<TextureRect>,
    pub color_rect: Gd<ColorRect>,
    pub noise: [Gd<FastNoiseLite>; 3],
    pub bullet_scene: Gd<PackedScene>,
    pub parent_rid: Rid,
}

impl Handles {
    /// ONE `is_instance_valid()` on the root handle (FR-009): children die with their root, so
    /// they are never checked separately.
    pub fn root_valid(&self) -> bool {
        match self {
            Handles::Door { root, .. } => root.is_instance_valid(),
            Handles::Puff { root } => root.is_instance_valid(),
            Handles::Blast { root, .. } => root.is_instance_valid(),
            Handles::Player(p) => p.root.is_instance_valid(),
            Handles::Bullet(p) => p.root.is_instance_valid(),
            Handles::Part(p) => p.root.is_instance_valid(),
            Handles::Robot(p) => p.root.is_instance_valid(),
        }
    }
}

/// `Entity → engine handles`, the `NonSend` half of the identity maps (FR-007). Populated only by
/// the sync layer's registration path, read only by `SyncIn`/`SyncOut` systems.
#[derive(Default)]
pub struct NodeHandles {
    pub by_entity: HashMap<Entity, Handles>,
}

/// The one World, one driver (constitution 1.5.1, "ECS shape (v3)"; research.md R8). Registered
/// as the `EcsWorld` autoload through `res://ecs/ecs_world.tscn`. `physics_process` runs the
/// `Fixed` schedule, `process` runs the `Frame` schedule; no other node calls `Schedule::run`.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct EcsWorld {
    base: Base<Node>,
    world: World,
    fixed: Schedule,
    frame: Schedule,
}

#[godot_api]
impl INode for EcsWorld {
    // Hand-written: the two schedules must be built and then handed TOGETHER to
    // `add_engine_systems`, which two independent `#[init(val = ...)]` expressions cannot do.
    fn init(base: Base<Node>) -> Self {
        let mut fixed = setup::build_fixed();
        let mut frame = setup::build_frame();
        add_engine_systems(&mut fixed, &mut frame);
        Self { base, world: setup::build_world(), fixed, frame }
    }

    fn ready(&mut self) {
        // R1: last in BOTH phases — after every scene node's callback of the same phase and after
        // the AnimationPlayer's internal processing, so events pushed during a phase are consumed
        // by that phase's schedule in the same frame.
        self.base_mut().set_process_priority(i32::MAX);
        self.base_mut().set_physics_process_priority(i32::MAX);
    }

    fn physics_process(&mut self, delta: f64) {
        self.world.insert_resource(FixedDelta(delta));
        // R3: the door message buffers advance once per FIXED run, before the drain, so a message
        // is read exactly once and never survives more than two fixed runs.
        self.world.resource_mut::<Messages<DoorBodyEntered>>().update();
        // specs/013 research R4: the same-run hit message likewise — written by the bullet's
        // `GameplaySettle` and read by the robot's in the same run; one unread survives one run.
        self.world.resource_mut::<Messages<RobotHitLocal>>().update();
        for ev in queue::drain() {
            match ev {
                InboundEvent::Register { id, handles, initial } => {
                    apply_register(&mut self.world, id, handles, initial)
                }
                other => apply::apply_non_register(&mut self.world, other),
            }
        }
        self.fixed.run(&mut self.world);
    }

    fn process(&mut self, delta: f64) {
        self.world.insert_resource(FrameDelta(delta));
        for ev in queue::drain() {
            match ev {
                InboundEvent::Register { id, handles, initial } => {
                    apply_register(&mut self.world, id, handles, initial)
                }
                other => apply::apply_non_register(&mut self.world, other),
            }
        }
        self.frame.run(&mut self.world);
    }
}

#[godot_api]
impl EcsWorld {}

/// The sync layer's registration path (research.md R4, FR-008): the ONLY place that inserts into
/// `NodeHandles.by_entity`. A registration whose root already died is dropped; an
/// already-registered id keeps its first entity (idempotent).
fn apply_register(world: &mut World, id: InstanceId, handles: Handles, initial: Initial) {
    if !handles.root_valid() {
        return;
    }
    // `resource_scope` takes the index out of the World for the closure's duration, so the spawn
    // closure can borrow the World while the index is borrowed too — no wasted spawn on a doubled
    // `ready`.
    let registration = world.resource_scope::<EntityIndex, _>(|world, mut index| {
        index.register_if_absent(id, || world.spawn_empty().id())
    });
    let Registration::Registered(entity) = registration else {
        return;
    };
    match initial {
        Initial::Door => {
            world.entity_mut(entity).insert(DoorState::Closed);
        }
        Initial::Puff { lifetime } => {
            world.entity_mut(entity).insert((DisappearPhase::start(), Lifetime(lifetime)));
        }
        Initial::Blast => {
            world.entity_mut(entity).insert(BlastTag);
        }
        Initial::Player { peer_id, simulates, owns_input, initial_position, orientation, start_rotation } => {
            let mut e = world.entity_mut(entity);
            e.insert((
                PlayerTag,
                PeerId(peer_id),
                // v2 `player.rs:44`: zero at init.
                Motion(Vector2::ZERO),
                Orientation(orientation),
                RootMotion(Transform3D::IDENTITY),
                // Backlog #10: starts at 0 (not v1's 100.0) so the first floor contact after
                // spawn never exceeds the land threshold and never fires a spurious `land`
                // RPC/sound (v2 `player.rs:36-39`).
                AirborneTime(0.0),
                InitialPosition(initial_position),
                // v2 `player.rs:81`.
                CurrentAnimation(Animations::Walk),
                // v2 `player_input.rs:21-22`.
                AimStateC(AimState::Idle),
                Trauma(0.0),
                ShakeTime(0.0),
                StartRotation(start_rotation),
            ));
            e.insert((
                JumpQueued::default(),
                PendingMouseLook::default(),
                PendingFx::default(),
                TickIntents::default(),
                FrameIntents::default(),
                ShakePending::default(),
                ReplicatedInput {
                    aiming: false,
                    shoot_target: Vector3::ZERO,
                    motion: Vector2::ZERO,
                    shooting: false,
                },
                Velocity(Vector3::ZERO),
            ));
            if simulates {
                e.insert(Simulates);
            }
            if owns_input {
                e.insert(OwnsInput);
            }
        }
        Initial::Bullet { shadow_mapping: _, simulates } => {
            let mut e = world.entity_mut(entity);
            e.insert((
                BulletTag,
                // v2 `bullet.rs:68`.
                BulletStateC(BulletState::Flying { time_alive: 5.0 }),
                BulletBasisZ(Vector3::ZERO),
                Collided::default(),
                BulletIntents::default(),
                PendingBulletFx::default(),
            ));
            if simulates {
                e.insert(Simulates);
            }
        }
        Initial::Part { lifetime, lifetime_random, disappearing_time, simulates } => {
            let mut e = world.entity_mut(entity);
            e.insert((
                PartTag,
                PartPhase::Attached,
                PartLifetimes { lifetime, lifetime_random, disappearing_time },
                PartIntents::default(),
                PendingPartFx::default(),
            ));
            if simulates {
                e.insert(Simulates);
            }
        }
        Initial::Robot { state, health, dead, test_shoot, orientation, aim_blend, simulates } => {
            let tuning = world.resource::<Tuning<RobotTuning>>().0;
            let mut e = world.entity_mut(entity);
            e.insert((
                RobotTag,
                RobotState(state),
                Health(health),
                TargetPosition(Vector3::ZERO),
                // v2 `red_robot.rs:49-55`; `test_shoot` zeroes the countdown at `ready` (`:115-117`).
                RobotCountersC(RobotCounters {
                    aim_preparing: tuning.aim_prepare_time,
                    shoot_countdown: if test_shoot { 0.0 } else { tuning.shoot_wait },
                    aim_countdown: tuning.aim_time,
                }),
                TrackedPlayer(None),
                Orientation(orientation),
                RootMotion(Transform3D::IDENTITY),
                Velocity(Vector3::ZERO),
                AimBlend(aim_blend),
            ));
            e.insert((
                RobotIntents::default(),
                RaycastAnswers::default(),
                PendingTrauma::default(),
                RemovalTimer::default(),
                PendingRobotFx::default(),
                PendingRobotHits::default(),
            ));
            // v2 `:119-123`: a robot that is dead at `ready` is never ticked nor advanced.
            if dead {
                e.insert(Dead);
            }
            // v2 `:138-141`: `test_shoot` also fires `shoot()` on the first physics step.
            if test_shoot {
                e.insert(ShootRequested);
            }
            if simulates {
                e.insert(Simulates);
            }
        }
    }
    world.non_send_mut::<NodeHandles>().by_entity.insert(entity, handles);
}

/// `SyncIn`, both schedules (FR-009): an entity whose root node is no longer valid is despawned
/// and dropped from both maps, with no other engine call on the dead handle.
fn sweep_dead_nodes(
    mut handles: NonSendMut<NodeHandles>,
    mut index: ResMut<EntityIndex>,
    mut commands: Commands,
) {
    let dead: Vec<Entity> = handles
        .by_entity
        .iter()
        .filter(|(_, h)| !h.root_valid())
        .map(|(e, _)| *e)
        .collect();
    for entity in dead {
        handles.by_entity.remove(&entity);
        index.remove_entity(entity);
        commands.entity(entity).despawn();
    }
}

/// `SyncOut`, both schedules (FR-010): the ONLY release path — `queue_free()` on the root handle,
/// never `free()` (which would run `exit_tree` inside the schedule), then despawn and drop both
/// map entries.
fn sync_out_remove(
    query: Query<Entity, With<Remove>>,
    mut handles: NonSendMut<NodeHandles>,
    mut index: ResMut<EntityIndex>,
    mut commands: Commands,
) {
    for entity in &query {
        if let Some(taken) = handles.by_entity.remove(&entity) {
            match taken {
                Handles::Door { mut root, .. } => root.queue_free(),
                Handles::Puff { mut root } => root.queue_free(),
                Handles::Blast { mut root, .. } => root.queue_free(),
                // No player is ever marked `Remove` by V3-B; the arm keeps the match exhaustive.
                Handles::Player(mut p) => p.root.queue_free(),
                // The bullet on the server (`destroy`), the parts on every peer (0.2 s after
                // `destroy`), the robot on the server (10 s after death) — specs/013.
                Handles::Bullet(mut p) => p.root.queue_free(),
                Handles::Part(mut p) => p.root.queue_free(),
                Handles::Robot(mut p) => p.root.queue_free(),
            }
        }
        index.remove_entity(entity);
        commands.entity(entity).despawn();
    }
}

/// `SyncOut`, FIXED schedule (FR-017): plays `doorsimple_opening` exactly once per `PlayOpen`
/// flag (v2 `door.rs:80`) and consumes the flag.
fn sync_out_door(
    query: Query<Entity, With<PlayOpen>>,
    mut handles: NonSendMut<NodeHandles>,
    mut commands: Commands,
) {
    for entity in &query {
        if let Some(Handles::Door { anim, .. }) = handles.by_entity.get_mut(&entity) {
            anim.play_ex().name("doorsimple_opening").done();
        }
        commands.entity(entity).remove::<PlayOpen>();
    }
}

/// `SyncOut`, FRAME schedule (FR-021): `emitting = true` exactly once per `StartEmitting` flag
/// (v2 `part_disappear.rs:34`) and consumes the flag.
fn sync_out_puff(
    query: Query<Entity, With<StartEmitting>>,
    mut handles: NonSendMut<NodeHandles>,
    mut commands: Commands,
) {
    for entity in &query {
        if let Some(Handles::Puff { root }) = handles.by_entity.get_mut(&entity) {
            root.set_emitting(true);
        }
        commands.entity(entity).remove::<StartEmitting>();
    }
}

/// `SyncIn`, FRAME schedule (FR-024; research.md R7): the cached camera's global origin, read ONCE
/// per distinct camera instance per frame, becomes the blast's `LookTarget` — written only when
/// it changed (`set_if_neq`), so `Changed<LookTarget>` is a real gate for `sync_out_blast`. An
/// invalid camera yields no target and no write (v2 `blast.rs:42-44`).
fn sync_in_blast(
    blasts: Query<Entity, With<BlastTag>>,
    handles: NonSend<NodeHandles>,
    mut targets: Query<&mut LookTarget>,
    mut cache: Local<HashMap<InstanceId, Option<Vector3>>>,
    mut commands: Commands,
) {
    cache.clear();
    for entity in &blasts {
        let Some(Handles::Blast { camera: Some(cam), .. }) = handles.by_entity.get(&entity) else {
            continue;
        };
        let key = cam.instance_id(); // no engine call: the id is stored in the Gd
        let origin = *cache
            .entry(key)
            .or_insert_with(|| cam.is_instance_valid().then(|| cam.get_global_transform().origin));
        let Some(origin) = origin else {
            continue;
        };
        match targets.get_mut(entity) {
            Ok(mut target) => {
                target.set_if_neq(LookTarget(origin));
            }
            Err(_) => {
                commands.entity(entity).insert(LookTarget(origin));
            }
        }
    }
}

/// `SyncOut`, FRAME schedule (FR-025): `light_rays.look_at(target)` (v2 `blast.rs:46`) for every
/// blast whose target changed this frame — engine-backed `Basis::looking_at`, glue by the 1.4.1
/// rule, which is why the blast has no pure gameplay system.
fn sync_out_blast(
    query: Query<(Entity, &LookTarget), Changed<LookTarget>>,
    mut handles: NonSendMut<NodeHandles>,
) {
    for (entity, target) in &query {
        if let Some(Handles::Blast { light_rays, .. }) = handles.by_entity.get_mut(&entity) {
            light_rays.look_at(target.0);
        }
    }
}

/// Adds the engine-touching systems to both schedules. The pure schedules come from `setup.rs`;
/// the per-module sync systems join here in commits 3 (door), 4 (puff) and 5 (blast). Within
/// `SyncOut` the rule is act, then release: every acting system runs `.before(sync_out_remove)`.
pub fn add_engine_systems(fixed: &mut Schedule, frame: &mut Schedule) {
    for schedule in [&mut *fixed, &mut *frame] {
        schedule.add_systems(sweep_dead_nodes.in_set(Phase::SyncIn));
        schedule.add_systems(sync_out_remove.in_set(Phase::SyncOut));
    }
    fixed.add_systems(sync_out_door.in_set(Phase::SyncOut).before(sync_out_remove));
    frame.add_systems(sync_out_puff.in_set(Phase::SyncOut).before(sync_out_remove));
    frame.add_systems(sync_in_blast.in_set(Phase::SyncIn).after(sweep_dead_nodes));
    frame.add_systems(sync_out_blast.in_set(Phase::SyncOut).before(sync_out_remove));
    // The player (specs/012 research R7): the fixed tick's sync/query members and the frame run's.
    fixed.add_systems(crate::player::sync::sync_in_player.in_set(Phase::SyncIn).after(sweep_dead_nodes));
    fixed.add_systems(crate::player::sync::orient_and_anim.in_set(Phase::EngineQueryOrient));
    fixed.add_systems(crate::player::sync::move_body.in_set(Phase::EngineQueryMove));
    fixed.add_systems(crate::player::sync::sync_out_player.in_set(Phase::SyncOut).before(sync_out_remove));
    frame.add_systems(
        crate::player_input::sync::sync_in_input.in_set(Phase::SyncIn).after(sweep_dead_nodes),
    );
    frame.add_systems(crate::player_input::sync::camera_and_ray.in_set(Phase::EngineQuery));
    frame.add_systems(
        crate::player_input::sync::sync_out_input.in_set(Phase::SyncOut).before(sync_out_remove),
    );
    // The camera shake (sync-only pair) and the remote-peer RPC effects. No ordering among the
    // frame `SyncOut` systems is needed: `sync_out_input` writes the input node, `color_rect`
    // and `camera_anim`; `apply_player_fx` writes the `Player` node's `current_animation`, the
    // tree, the sounds, the particles and `fire_cooldown`; `sync_out_shake` writes the camera's
    // rotation — disjoint nodes and disjoint components.
    frame.add_systems(
        crate::camera_noise_shake::sync::sync_out_shake.in_set(Phase::SyncOut).before(sync_out_remove),
    );
    frame.add_systems(crate::player::sync::apply_player_fx.in_set(Phase::SyncOut).before(sync_out_remove));
}
