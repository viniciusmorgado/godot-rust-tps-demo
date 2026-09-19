//! The bullet entity's engine systems (constitution 1.5.2 "ECS shape (v3)"; specs/013 research
//! R7): `SyncIn`, the `EngineQueryMove` member and `SyncOut` of the FIXED schedule on `Simulates`
//! entities, and the frame `SyncOut` applier of the remote `explode` on the others. Every engine
//! call of v2's `physics_process`/`explode` (`bullet.rs:95-146`) lives here, in v2's order; the
//! decisions are in `bullet/system.rs`.

use bevy_ecs::prelude::*;
use godot::classes::Node3D;
use godot::prelude::*;

use crate::bullet::Bullet;
use crate::ecs::event::BulletFx;
use crate::ecs::markers::{
    BulletBasisZ, BulletIntents, BulletTag, Collided, FixedDelta, PendingBulletFx, Simulates,
};
use crate::ecs::{BulletHandles, Handles, NodeHandles};
use crate::hittable;

/// `SyncIn`, FIXED schedule, `Simulates` bullets: the one body read of the tick — the transform's
/// Z column (v2 `bullet.rs:112`), which `move_bullet` displaces along.
pub fn sync_in_bullet(
    handles: NonSend<NodeHandles>,
    bullets: Query<Entity, (With<BulletTag>, With<Simulates>)>,
    mut commands: Commands,
) {
    for entity in &bullets {
        let Some(Handles::Bullet(p)) = handles.by_entity.get(&entity) else {
            continue;
        };
        commands.entity(entity).insert(BulletBasisZ(p.root.get_transform().basis.col_c()));
    }
}

/// `EngineQueryMove`, `Simulates` bullets: v2 `bullet.rs:111-116` — `move_and_collide` along
/// `-dt · VELOCITY · basis_z` when the bullet is active this run (the expiry run itself still
/// moves, `:109-110`), and the collider resolved once into a `HitKind` (research R3), which
/// `bullet_settle` decides on. An inactive bullet reports no collision.
pub fn move_bullet(
    dt: Res<FixedDelta>,
    mut handles: NonSendMut<NodeHandles>,
    mut bullets: Query<(Entity, &BulletBasisZ, &BulletIntents, &mut Collided), With<Simulates>>,
) {
    let dt = dt.0 as f32;
    for (entity, basis_z, intents, mut collided) in &mut bullets {
        let Some(Handles::Bullet(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        if !intents.active {
            *collided = Collided::default();
            continue;
        }
        let displacement: Vector3 = -dt * Bullet::VELOCITY * basis_z.0;
        let col = p.root.move_and_collide(displacement);
        collided.collided = col.is_some();
        collided.hit = col
            .and_then(|c| c.get_collider())
            .and_then(|c| c.try_cast::<Node3D>().ok())
            .and_then(hittable::kind_of);
    }
}

/// `SyncOut`, FIXED schedule, `Simulates` bullets (order-insensitive per the spec): the local
/// effects of `explode` once per fired intent (v2 `bullet.rs:139-145`, inline — the RPC is
/// `call_remote`), the hit RPC through `HitKind::rpc_hit` (`:117-121`: the player's `call_local`
/// handler pushes `AddTrauma`; the robot's handler is v2's `call_local` `hit` in commit 2 and the
/// `call_remote` one from commit 3 on, its local path being `RobotHitLocal`), the collision shape
/// off (`:122`), and the `explode` RPC to the remote peers (`:106`, `:126`).
pub fn sync_out_bullet(
    mut handles: NonSendMut<NodeHandles>,
    bullets: Query<(Entity, &BulletIntents), With<Simulates>>,
) {
    for (entity, intents) in &bullets {
        let Some(Handles::Bullet(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        if intents.explode {
            explode_effects(p);
        }
        if let Some(kind) = intents.hit {
            kind.rpc_hit();
        }
        if intents.disable_collision {
            p.collision.set_disabled(true);
        }
        if intents.explode {
            // Remote peers only (`call_remote`): no handler runs on this peer.
            p.root.rpc("explode", &[]);
        }
    }
}

/// `SyncOut`, FRAME schedule, non-`Simulates` bullets: the `explode` RPC handler of a REMOTE peer,
/// applied in arrival order exactly as v2's handler did (`bullet.rs:137-146`).
pub fn sync_out_bullet_frame(
    mut handles: NonSendMut<NodeHandles>,
    mut bullets: Query<(Entity, &mut PendingBulletFx), Without<Simulates>>,
) {
    for (entity, mut pending) in &mut bullets {
        if pending.0.is_empty() {
            continue;
        }
        let Some(Handles::Bullet(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        for fx in pending.0.drain(..) {
            match fx {
                BulletFx::Explode => explode_effects(p),
            }
        }
    }
}

/// v2 `explode`'s body (`bullet.rs:139-145`): the animation and, when the registration's
/// `Settings` read said so, the explosion light's shadow.
fn explode_effects(p: &mut BulletHandles) {
    p.anim.play_ex().name("explode").done();
    // Only enable shadows for the explosion, as the moving light
    // is very small and doesn't noticeably benefit from shadow mapping.
    if p.shadow_mapping {
        p.light.set_shadow(true);
    }
}
