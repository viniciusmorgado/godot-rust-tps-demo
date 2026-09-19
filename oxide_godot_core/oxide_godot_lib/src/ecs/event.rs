//! Inbound events (bridges → sync layer) and the gameplay message derived from them.

use bevy_ecs::prelude::{Entity, Message};
use godot::builtin::{Transform3D, Vector2, Vector3};
use godot::obj::InstanceId;

use super::Handles;
use crate::red_robot::State;

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
    /// The bullet's `explode` RPC on a REMOTE peer (v2 `bullet.rs:137-146`; `call_remote`, the
    /// simulating peer applies the effects inline in its fixed `SyncOut`).
    BulletFx {
        root_id: InstanceId,
        fx: BulletFx,
    },
    /// The bullet animation's method track `destroy` (v2 `bullet.rs:148-154`), fired on every
    /// peer; the drain frees only a `Simulates` entity.
    BulletDestroy {
        root_id: InstanceId,
    },
    /// The part's `destroy` RPC on a REMOTE peer (v2 `part.rs:194-215`).
    PartFx {
        root_id: InstanceId,
        fx: PartFx,
    },
    /// The robot's `hit` RPC on a REMOTE peer (v2 `red_robot.rs:276-328`; option (B): the local
    /// path is `RobotHitLocal`, written by the bullet's `GameplaySettle`).
    RobotHit {
        root_id: InstanceId,
    },
    /// The robot's `play_shoot` RPC on a REMOTE peer (v2 `red_robot.rs:330-333`).
    RobotFx {
        root_id: InstanceId,
        fx: RobotFx,
    },
    /// The shoot animation's method track `shoot_check` (v2 `red_robot.rs:335-338`).
    ShootRequested {
        root_id: InstanceId,
    },
    /// The shoot animation's method track `resume_approach` (v2 `red_robot.rs:267-274`).
    ResumeApproachRequested {
        root_id: InstanceId,
    },
    /// `PlayerDetectionArea`'s `body_entered`/`body_exited` after the boundary `try_cast::<Player>`
    /// (v2 `red_robot.rs:340-358`): `Some(id)` on entry, `None` on exit.
    RobotPlayerSeen {
        root_id: InstanceId,
        player: Option<InstanceId>,
    },
}

/// The bullet's remote RPC effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BulletFx {
    Explode,
}

/// The part's remote RPC effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartFx {
    Destroy,
}

/// The robot's remote RPC effects (`hit` has its own event: it changes state).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RobotFx {
    PlayShoot,
}

/// The same-run hit (specs/013 research R4): written by the bullet's `bullet_settle`
/// (`GameplaySettle`) and read by the robot's `robot_hit_apply` in the same set, ordered after
/// it; `update()`d once at the start of every FIXED run, before the drain.
#[derive(Message, Clone, Copy)]
pub struct RobotHitLocal {
    pub robot: InstanceId,
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
    /// → the bullet entity (specs/013 data-model.md): `shadow_mapping` is the `Settings` read of
    /// v2 `bullet.rs:143`, done once in `ready`; `simulates` is `is_server()` (`:89`).
    Bullet {
        shadow_mapping: bool,
        simulates: bool,
    },
    /// → the part entity: the three exports (v2 `part.rs:86-94`); `simulates` (`:167`).
    Part {
        lifetime: f32,
        lifetime_random: f32,
        disappearing_time: f32,
        simulates: bool,
    },
    /// → the robot entity: the scene fields (v2 `red_robot.rs:35-47`), `test_shoot` (`:115-117`,
    /// `:138-141`), the model's global transform with zero origin (`:111-112`), `aim_blend` = the
    /// tree's `parameters/aim/blend_position` at `ready` (`red_robot.tscn:10785`, replacing the
    /// per-step `get` of `:482-485`); `simulates` (`:133`).
    Robot {
        state: State,
        health: i32,
        dead: bool,
        test_shoot: bool,
        orientation: Transform3D,
        aim_blend: Vector2,
        simulates: bool,
    },
}

/// The gameplay message the drain writes AFTER resolving `InboundEvent::DoorBodyEntered`'s
/// `InstanceId` to its entity (research.md R3): the door system never sees an `InstanceId`.
#[derive(Message, Clone, Copy)]
pub struct DoorBodyEntered {
    pub entity: Entity,
    pub is_player: bool,
}
