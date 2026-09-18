//! Inbound events (bridges → sync layer) and the gameplay message derived from them.

use bevy_ecs::prelude::{Entity, Message};
use godot::obj::InstanceId;

use super::Handles;

/// What a bridge pushes to the queue from an engine callback (data-model.md "Events"). The
/// `Register` variant carries `Gd<T>` handles, so this type has no `Send` bound — it only ever
/// travels through the main-thread `thread_local!` queue.
pub enum InboundEvent {
    Register {
        id: InstanceId,
        handles: Handles,
        initial: Initial,
    },
    Unregister {
        id: InstanceId,
    },
    DoorBodyEntered {
        id: InstanceId,
        is_player: bool,
    },
    BlastAnimationFinished {
        id: InstanceId,
    },
}

/// The pure half of a registration: which components the entity starts with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Initial {
    /// → `DoorState::Closed`
    Door,
    /// → `DisappearPhase::start()` + `Lifetime(lifetime)`; `lifetime` is the node's
    /// `CPUParticles3D.lifetime` read once in `ready` (v2 `part_disappear.rs:35`, an `f32`).
    Puff { lifetime: f32 },
    /// → `BlastTag`
    Blast,
}

/// The gameplay message the drain writes AFTER resolving `InboundEvent::DoorBodyEntered`'s
/// `InstanceId` to its entity (research.md R3): the door system never sees an `InstanceId`.
#[derive(Message, Clone, Copy)]
pub struct DoorBodyEntered {
    pub entity: Entity,
    pub is_player: bool,
}
