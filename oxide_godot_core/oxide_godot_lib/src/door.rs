use godot::classes::{AnimationPlayer, Area3D, IArea3D, Node3D};
use godot::prelude::*;

use crate::player::Player;

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct Door {
    base: Base<Area3D>,

    open: bool,

    // upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (a node that does not exist);
    // the scene node is "DoorModel2" — the door never opened and Godot printed "Node not found".
    #[init(node = "DoorModel2/AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
}

#[godot_api]
impl IArea3D for Door {}

#[godot_api]
impl Door {
    #[func]
    fn _on_door_body_entered(&mut self, body: Gd<Node3D>) {
        if !self.open && body.try_cast::<Player>().is_ok() {
            self.animation_player.play_ex().name("doorsimple_opening").done();
            self.open = true;
        }
    }
}
