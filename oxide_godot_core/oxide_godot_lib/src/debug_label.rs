use godot::classes::display_server::VSyncMode;
use godot::classes::rendering_server::RenderingInfo;
use godot::classes::{
    DisplayServer, Engine, ILabel, Input, Label, OfflineMultiplayerPeer, Os, RenderingServer,
};
use godot::prelude::*;

use pure::DebugStats;

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

        if !self.base().is_visible() {
            return;
        }

        let multiplayer = self.base().get_multiplayer().unwrap();
        let online = !(multiplayer
            .get_multiplayer_peer()
            .map(|peer| peer.try_cast::<OfflineMultiplayerPeer>().is_ok())
            == Some(true));
        let stats = DebugStats {
            fps: Engine::singleton().get_frames_per_second(),
            vsync_enabled: DisplayServer::singleton().window_get_vsync_mode() != VSyncMode::DISABLED,
            ram_bytes: Os::singleton().get_static_memory_usage(),
            vram_bytes: RenderingServer::singleton().get_rendering_info(RenderingInfo::VIDEO_MEM_USED),
            multiplayer_id: online.then(|| multiplayer.get_unique_id() as i64),
        };

        self.base_mut().set_text(&pure::compose(&stats));
    }
}

mod pure {
    pub struct DebugStats {
        pub fps: f64,
        pub vsync_enabled: bool,
        pub ram_bytes: u64,
        pub vram_bytes: u64,
        pub multiplayer_id: Option<i64>,
    }

    pub fn compose(stats: &DebugStats) -> String {
        let mut text = String::from("FPS: ") + &format_godot_float(stats.fps);
        text += "\nVSync: ";
        text += if stats.vsync_enabled { "Enabled" } else { "Disabled" };
        text += "\nMemory: ";
        text += &format!("{:3.2}", stats.ram_bytes as f64 / 1_048_576.0);
        text += " MiB";
        text += "\nVRAM: ";
        text += &format!("{:3.2}", stats.vram_bytes as f64 / 1_048_576.0);
        text += " MiB";
        text += "\nOnline: ";
        text += if stats.multiplayer_id.is_some() { "Yes" } else { "No" };
        if let Some(id) = stats.multiplayer_id {
            text += "\nMultiplayer ID: ";
            text += &id.to_string();
        }
        text
    }

    /// Reproduces `Variant::from(f64).stringify()` for the values this module actually
    /// produces (`Engine.get_frames_per_second()` is always integer-valued): Rust's own
    /// shortest-round-trip `Display`, with a trailing `.0` appended when it would otherwise be
    /// dropped for a whole number (`60.0` -> Rust prints `"60"`, Godot prints `"60.0"`).
    fn format_godot_float(v: f64) -> String {
        let s = format!("{v}");
        if s.contains('.') { s } else { format!("{s}.0") }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn stats(multiplayer_id: Option<i64>) -> DebugStats {
            DebugStats {
                fps: 60.0,
                vsync_enabled: true,
                ram_bytes: 123_456_789,
                vram_bytes: 10_530_000,
                multiplayer_id,
            }
        }

        #[test]
        fn whole_number_fps_keeps_the_trailing_decimal() {
            let text = compose(&stats(None));
            assert!(text.starts_with("FPS: 60.0\n"), "text was: {text}");
        }

        #[test]
        fn vram_line_sits_right_after_memory_and_before_online() {
            let text = compose(&stats(None));
            let memory_pos = text.find("\nMemory: ").unwrap();
            let vram_pos = text.find("\nVRAM: ").unwrap();
            let online_pos = text.find("\nOnline: ").unwrap();
            assert!(memory_pos < vram_pos && vram_pos < online_pos, "text was: {text}");
        }

        #[test]
        fn offline_has_no_multiplayer_id_line() {
            let text = compose(&stats(None));
            assert!(text.ends_with("\nOnline: No"), "text was: {text}");
            assert!(!text.contains("Multiplayer ID"));
        }

        #[test]
        fn online_appends_the_multiplayer_id_line() {
            let text = compose(&stats(Some(7)));
            assert!(text.ends_with("\nOnline: Yes\nMultiplayer ID: 7"), "text was: {text}");
        }
    }
}
