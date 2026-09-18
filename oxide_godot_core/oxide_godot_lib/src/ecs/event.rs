//! Inbound events (bridges → sync layer) and the gameplay message derived from them.

use bevy_ecs::prelude::{Entity, Message};
use godot::builtin::{Transform3D, Vector2, Vector3};
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
    /// The input node's `#[rpc] jump` (v2 `player_input.rs:161-164`), keyed by the root player.
    JumpPressed {
        root_id: InstanceId,
    },
    /// The input node's `input` callback on `InputEventMouseMotion` (v2 `player_input.rs:150-156`).
    MouseLook {
        root_id: InstanceId,
        screen_relative: Vector2,
    },
    /// `Player::add_camera_shake_trauma` (v2 `player.rs:161-165`; also `red_robot.rs:452`) and
    /// `Player::hit`, which pushes `amount: 0.75` (v2 `player.rs:157-159`).
    AddTrauma {
        root_id: InstanceId,
        amount: f64,
    },
    /// A `jump`/`land`/`shoot` RPC handler on a REMOTE peer (v2 `player.rs:133-154`; option (b):
    /// `call_remote`, so the simulating peer applies its local effects inline).
    PlayerFx {
        root_id: InstanceId,
        fx: PlayerFx,
    },
}

/// The remote-peer RPC effects. No `Hit`: `hit` pushes `AddTrauma { amount: 0.75 }`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerFx {
    Jump,
    Land,
    Shoot,
}

/// The pure half of a registration: which components the entity starts with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Initial {
    /// → `DoorState::Closed`
    Door,
    /// → `DisappearPhase::start()` + `Lifetime(lifetime)`; `lifetime` is the node's
    /// `CPUParticles3D.lifetime` read once in `ready` (v2 `part_disappear.rs:35`; the gdext
    /// binding `CpuParticles3D::get_lifetime` returns `f64`, so v2's `lifetime * 2.0` was an
    /// `f64` product — implement-time correction of data-model.md's `f32`).
    Puff { lifetime: f64 },
    /// → `BlastTag`
    Blast,
    /// → the player entity's persistent components (specs/012 data-model.md): `peer_id` is
    /// `player_id` (v2 `player.rs:78`), `simulates` is `is_server()` (`:98`), `owns_input` is
    /// the input node's authority check (`player_input.rs:61-62`), `initial_position` (`:88`),
    /// `orientation` the model's global transform with zero origin (`:90-91`), `start_rotation`
    /// the camera's `ready` capture (`camera_noise_shake.rs:41`).
    Player {
        peer_id: i32,
        simulates: bool,
        owns_input: bool,
        initial_position: Vector3,
        orientation: Transform3D,
        start_rotation: Vector3,
    },
}

/// The gameplay message the drain writes AFTER resolving `InboundEvent::DoorBodyEntered`'s
/// `InstanceId` to its entity (research.md R3): the door system never sees an `InstanceId`.
#[derive(Message, Clone, Copy)]
pub struct DoorBodyEntered {
    pub entity: Entity,
    pub is_player: bool,
}
