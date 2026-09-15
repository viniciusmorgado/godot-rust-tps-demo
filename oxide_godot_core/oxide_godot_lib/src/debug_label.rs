use godot::classes::display_server::VSyncMode;
use godot::classes::{DisplayServer, Engine, ILabel, Input, Label, OfflineMultiplayerPeer, Os};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Label)]
pub struct DebugLabel {
    base: Base<Label>,
}

#[godot_api]
impl ILabel for DebugLabel {
    fn process(&mut self, _delta: f64) {
        if Input::singleton().is_action_just_pressed("toggle_debug") {
            let visible = self.base().is_visible();
            self.base_mut().set_visible(!visible);
        }

        let mut text = String::from("FPS: ")
            + &Variant::from(Engine::singleton().get_frames_per_second())
                .stringify()
                .to_string();
        text += "\nVSync: ";
        text += if DisplayServer::singleton().window_get_vsync_mode() != VSyncMode::DISABLED {
            "Enabled"
        } else {
            "Disabled"
        };
        text += "\nMemory: ";
        text += &format!(
            "{:3.2}",
            Os::singleton().get_static_memory_usage() as f64 / 1048576.0
        );
        text += " MiB";

        let multiplayer = self.base().get_multiplayer().unwrap();
        let online: bool = !(multiplayer
            .get_multiplayer_peer()
            .map(|peer| peer.try_cast::<OfflineMultiplayerPeer>().is_ok())
            == Some(true));
        text += "\nOnline: ";
        text += if online { "Yes" } else { "No" };
        if online {
            text += "\nMultiplayer ID: ";
            text += &multiplayer.get_unique_id().to_string();
        }

        self.base_mut().set_text(&text);
    }
}
