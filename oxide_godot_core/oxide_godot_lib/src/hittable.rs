use godot::classes::Node3D;
use godot::obj::InstanceId;
use godot::prelude::*;

use crate::player::Player;
use crate::red_robot::EnemyRobot;

/// Replaces `has_method("hit")` duck typing: a bullet's collider is either a `Player` or an
/// `EnemyRobot`, both of which expose a same-named, no-argument `#[rpc] fn hit(&mut self)`.
/// Introduced by `bullet.rs`; V2-D's `EnemyRobot`/`red_robot.rs` reuses this unchanged.
pub enum HitTarget {
    Player(Gd<Player>),
    Robot(Gd<EnemyRobot>),
}

/// Two `try_cast`s, order irrelevant — a node cannot satisfy both.
pub fn resolve(node: Gd<Node3D>) -> Option<HitTarget> {
    if let Ok(player) = node.clone().try_cast::<Player>() {
        return Some(HitTarget::Player(player));
    }
    if let Ok(robot) = node.try_cast::<EnemyRobot>() {
        return Some(HitTarget::Robot(robot));
    }
    None
}

impl HitTarget {
    /// The RPC dispatch itself stays by name (`.rpc("hit", &[])`) — gdext has no typed RPC
    /// dispatch, the permanent residual named in the milestone's spec. This is the ONLY place
    /// that string is spelled.
    pub fn rpc_hit(&mut self) {
        match self {
            HitTarget::Player(p) => {
                p.rpc("hit", &[]);
            }
            HitTarget::Robot(r) => {
                r.rpc("hit", &[]);
            }
        }
    }
}

/// The `Send` projection of a `HitTarget` (specs/013 research R3): which class and which node,
/// as instance ids — a component may carry it (no `Gd`). Resolved once, in the bullet's
/// `EngineQueryMove`, right after `move_and_collide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    Player(InstanceId),
    Robot(InstanceId),
}

/// `resolve` + `instance_id()`.
pub fn kind_of(node: Gd<Node3D>) -> Option<HitKind> {
    resolve(node).map(|target| match target {
        HitTarget::Player(player) => HitKind::Player(player.instance_id()),
        HitTarget::Robot(robot) => HitKind::Robot(robot.instance_id()),
    })
}

impl HitKind {
    /// Re-fetches the target by id and sends the by-name RPC through the one spelling above
    /// (`HitTarget::rpc_hit`). The attribute on each `hit` decides the local delivery: the
    /// player's `call_local` handler runs locally too (it pushes `AddTrauma`); the robot's is
    /// `call_remote` from V3-C commit 3 on, so nothing runs locally — the local path is
    /// `Messages<RobotHitLocal>` (research R4). A freed target is a no-op.
    pub fn rpc_hit(self) {
        let target = match self {
            HitKind::Player(id) => Gd::<Player>::try_from_instance_id(id).ok().map(HitTarget::Player),
            HitKind::Robot(id) => Gd::<EnemyRobot>::try_from_instance_id(id).ok().map(HitTarget::Robot),
        };
        if let Some(mut target) = target {
            target.rpc_hit();
        }
    }

    pub fn robot_id(self) -> Option<InstanceId> {
        match self {
            HitKind::Robot(id) => Some(id),
            HitKind::Player(_) => None,
        }
    }
}
