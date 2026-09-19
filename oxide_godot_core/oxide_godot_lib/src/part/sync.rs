//! The part entity's engine system (constitution 1.5.2 "ECS shape (v3)"; specs/013 research R7):
//! the FRAME `SyncOut` that writes the fade through the node's setter, instances the puff and
//! sends `destroy` on the server, and applies the remote `destroy` on the other peers. Every
//! engine call of v2's `process`/`destroy` (`part.rs:138-146`, `:194-215`) lives here.

use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use godot::classes::{CpuParticles3D, Node};
use godot::prelude::*;

use super::system::DESTROY_DELAY;
use crate::ecs::event::PartFx;
use crate::ecs::markers::{PartIntents, PartPhase, PartTag, PendingPartFx, Simulates};
use crate::ecs::timer::Timer;
use crate::ecs::{Handles, NodeHandles, PartHandles};
use crate::part::Part;

/// backlog #15: the puff's parent is the ROBOT's own parent, not `Death` (which decouples
/// the puff's lifetime from the robot's 10s-after-death removal — research.md R7). The
/// 3-hop walk from a `Part`: `Death` -> `EnemyRobot` -> the robot's own parent. Falls back
/// to the LAST successfully-resolved ancestor if the chain is shorter (e.g. a standalone
/// `Part` with no `Death`/`EnemyRobot` ancestors, as in the parity harness) -- never panics.
/// Resolved at destroy time from the handle (v2 `part.rs:224-232`).
pub(crate) fn puff_parent(root: &Gd<Part>) -> Gd<Node> {
    let death = root.get_parent();
    let robot = death.as_ref().and_then(|d| d.get_parent());
    let robots_parent = robot.as_ref().and_then(|r| r.get_parent());
    robots_parent
        .or(robot)
        .or(death)
        .expect("Part must have at least an immediate parent")
}

/// `SyncOut`, FRAME schedule, every part. On `Simulates`: the fade through the projection
/// setter (v2 `:140` → `:151-160`, the `bind_mut()` guard dropped in its own block), then on the
/// destroy intent the puff (`:196-200`) and `rpc("destroy")` to the remote peers (`call_remote`).
/// On the other peers: the remote `destroy` (`PartFx::Destroy`) — the same puff and the entity's
/// own `Destroyed(Timer(0.2))`, so the client frees its node through `Remove` as v2's
/// `call_local` handler did (`:202-214`; the parts are scene children of the spawned robot, not
/// spawner-spawned, so no despawn packet ever frees them — research R7).
/// `sync_out_part`'s query (a `type` so clippy's `type_complexity` stays quiet).
type PartSyncOutQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PartIntents, &'static mut PendingPartFx, &'static mut PartPhase, Has<Simulates>),
    With<PartTag>,
>;

pub fn sync_out_part(mut handles: NonSendMut<NodeHandles>, mut parts: PartSyncOutQuery) {
    for (entity, intents, mut pending, mut phase, simulates) in &mut parts {
        if simulates {
            if intents.fade.is_none() && !intents.destroy {
                continue;
            }
            let Some(Handles::Part(p)) = handles.by_entity.get_mut(&entity) else {
                continue;
            };
            if let Some(fade) = intents.fade {
                p.root.bind_mut().set_fade_value(fade);
            }
            if intents.destroy {
                spawn_puff(p);
                // Remote peers only (`call_remote`): no handler runs on this peer.
                p.root.rpc("destroy", &[]);
            }
        } else {
            if pending.0.is_empty() {
                continue;
            }
            let Some(Handles::Part(p)) = handles.by_entity.get_mut(&entity) else {
                continue;
            };
            for fx in pending.0.drain(..) {
                match fx {
                    PartFx::Destroy => {
                        spawn_puff(p);
                        *phase = PartPhase::Destroyed(Timer::new(DESTROY_DELAY));
                    }
                }
            }
        }
    }
}

/// v2 `destroy`'s puff (`part.rs:196-200`): instanced under `puff_parent` at the part's origin.
fn spawn_puff(p: &mut PartHandles) {
    let mut puff: Gd<CpuParticles3D> = p.puff_scene.instantiate_as::<CpuParticles3D>();
    let mut parent = puff_parent(&p.root);
    parent.add_child(&puff);
    let origin = p.root.get_global_transform().origin;
    puff.set_global_position(origin);
}
