use godot::classes::{AnimationPlayer, Area3D, IArea3D, Node3D};
use godot::prelude::*;

use crate::ecs::event::{InboundEvent, Initial};
use crate::ecs::{queue, Handles};
use crate::player::Player;

pub(crate) mod system;

/// Bridge (constitution 1.5.1, "ECS shape (v3)"): `ready` registers the entity, `exit_tree`
/// unregisters it, the scene's signal handler only pushes an event. The door's decision lives
/// in `door::system::open_on_player`; the animation is started by `ecs::sync_out_door`.
#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct Door {
    base: Base<Area3D>,

    // upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (a node that does not exist);
    // the scene node is "DoorModel2" — the door never opened and Godot printed "Node not found".
    #[init(node = "DoorModel2/AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
}

#[godot_api]
impl IArea3D for Door {
    fn ready(&mut self) {
        let id = self.base().instance_id();
        queue::push(InboundEvent::Register {
            id,
            handles: Handles::Door {
                root: self.to_gd().upcast::<Area3D>(),
                anim: self.animation_player.clone(),
            },
            initial: Initial::Door,
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}

#[godot_api]
impl Door {
    #[func]
    fn _on_door_body_entered(&mut self, body: Gd<Node3D>) {
        // Backlog #14 (revised form — see FR-018): the boundary typing is this immediate
        // try_cast, NOT a `Gd<Player>` signal parameter (which would make the engine print a
        // conversion error for every non-player body entering — a behavior change).
        let is_player = body.try_cast::<Player>().is_ok();
        queue::push(InboundEvent::DoorBodyEntered { id: self.base().instance_id(), is_player });
    }
}
