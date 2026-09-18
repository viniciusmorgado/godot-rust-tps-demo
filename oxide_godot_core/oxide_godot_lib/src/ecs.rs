//! ECS layer over the nodes (constitution 1.5.1, Principle III "ECS shape (v3)").
//!
//! Glue module: the engine-facing types (`Handles`, `NodeHandles`) live here because they name
//! `Gd<T>`; the pure core lives in the `ecs/*` submodules and is tested without Godot.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use godot::classes::{AnimationPlayer, Area3D, Camera3D, CpuParticles3D, INode, Node, Node3D};
use godot::obj::InstanceId;
use godot::prelude::*;

use event::{DoorBodyEntered, InboundEvent, Initial};
use index::{EntityIndex, Registration};
use markers::{BlastTag, FixedDelta, FrameDelta, Remove};
use setup::Phase;

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
}

impl Handles {
    /// ONE `is_instance_valid()` on the root handle (FR-009): children die with their root, so
    /// they are never checked separately.
    pub fn root_valid(&self) -> bool {
        match self {
            Handles::Door { root, .. } => root.is_instance_valid(),
            Handles::Puff { root } => root.is_instance_valid(),
            Handles::Blast { root, .. } => root.is_instance_valid(),
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
            // filled in commit 3: `DoorState::Closed`
        }
        Initial::Puff { lifetime: _ } => {
            // filled in commit 4: `DisappearPhase::start()` + `Lifetime(lifetime)`
        }
        Initial::Blast => {
            world.entity_mut(entity).insert(BlastTag);
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
            }
        }
        index.remove_entity(entity);
        commands.entity(entity).despawn();
    }
}

/// Adds the engine-touching systems to both schedules. The pure schedules come from `setup.rs`;
/// the per-module sync systems join here in commits 3 (door), 4 (puff) and 5 (blast).
pub fn add_engine_systems(fixed: &mut Schedule, frame: &mut Schedule) {
    for schedule in [fixed, frame] {
        schedule.add_systems(sweep_dead_nodes.in_set(Phase::SyncIn));
        schedule.add_systems(sync_out_remove.in_set(Phase::SyncOut));
    }
}
