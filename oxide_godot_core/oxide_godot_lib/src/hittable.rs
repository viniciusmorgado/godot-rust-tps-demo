use godot::classes::Node3D;
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
