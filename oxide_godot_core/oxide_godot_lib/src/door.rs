use godot::classes::{AnimationPlayer, Area3D, IArea3D, Node3D};
use godot::prelude::*;

use crate::player::Player;

use pure::DoorState;

/// Replaces `open: bool` — a one-way flag, once `true`, never reset.
mod pure {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum DoorState {
        Closed,
        Open,
    }

    /// `v1`: `door.rs:26-29` — only a `Player` body on a currently-`Closed` door opens it
    /// (returning the `bool` "play the open animation now"); anything else is a no-op.
    pub fn on_body(state: DoorState, is_player: bool) -> (DoorState, bool) {
        match (state, is_player) {
            (DoorState::Closed, true) => (DoorState::Open, true),
            (state, _) => (state, false),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn closed_door_opens_for_a_player() {
            let (state, play) = on_body(DoorState::Closed, true);
            assert_eq!(state, DoorState::Open);
            assert_eq!(play, true);
        }

        #[test]
        fn closed_door_ignores_a_non_player_body() {
            let (state, play) = on_body(DoorState::Closed, false);
            assert_eq!(state, DoorState::Closed);
            assert_eq!(play, false);
        }

        #[test]
        fn open_door_does_not_retrigger_for_a_player() {
            let (state, play) = on_body(DoorState::Open, true);
            assert_eq!(state, DoorState::Open);
            assert_eq!(play, false);
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct Door {
    base: Base<Area3D>,

    #[init(val = DoorState::Closed)]
    state: DoorState,

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
        // Backlog #14 (revised form — see FR-018): the boundary typing is this immediate
        // try_cast, NOT a `Gd<Player>` signal parameter (which would make the engine print a
        // conversion error for every non-player body entering — a behavior change).
        let is_player = body.try_cast::<Player>().is_ok();
        let (new_state, play) = pure::on_body(self.state, is_player);
        self.state = new_state;
        if play {
            self.animation_player.play_ex().name("doorsimple_opening").done();
        }
    }
}
