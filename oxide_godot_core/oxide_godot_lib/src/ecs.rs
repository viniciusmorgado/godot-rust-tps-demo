//! ECS layer over the nodes (constitution 1.5.1, Principle III "ECS shape (v3)").
//!
//! Glue module: the engine-facing types (`Handles`, `NodeHandles`) live here because they name
//! `Gd<T>`; the pure core lives in the `ecs/*` submodules and is tested without Godot.

use std::collections::HashMap;

use bevy_ecs::prelude::Entity;
use godot::classes::{AnimationPlayer, Area3D, Camera3D, CpuParticles3D, Node3D};
use godot::prelude::*;

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
