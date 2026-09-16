use godot::classes::display_server::VSyncMode;
use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality};
use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa};
use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    ConfigFile, DisplayServer, Engine, Environment, INode, InputEvent, Node, RenderingServer, Window,
};
use godot::prelude::*;

const CONFIG_FILE_PATH: &str = "user://settings.ini";

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Settings {
    base: Base<Node>,

    // MetalFX is only supported when using the Metal rendering driver.
    #[var]
    metalfx_supported: bool,

    defaults: VarDictionary,

    #[var]
    config_file: Gd<ConfigFile>,
}

#[godot_api]
impl INode for Settings {
    fn init(base: Base<Node>) -> Self {
        let metalfx_supported =
            RenderingServer::singleton().get_current_rendering_driver_name() == "metal";
        let defaults = vdict! {
            "video" => &vdict! {
                "display_mode" => WindowMode::EXCLUSIVE_FULLSCREEN.ord() as i64,
                "vsync" => VSyncMode::ENABLED.ord() as i64,
                "max_fps" => 0_i64,
                "resolution_scale" => 1.0_f64,
                "scale_filter" => if metalfx_supported { Scaling3DMode::METALFX_TEMPORAL.ord() as i64 } else { Scaling3DMode::FSR2.ord() as i64 },
            },
            "rendering" => &vdict! {
                "taa" => false,
                "msaa" => Msaa::DISABLED.ord() as i64,
                "screen_space_aa" => ScreenSpaceAa::DISABLED.ord() as i64,
                "shadow_mapping" => true,
                "gi_type" => Self::GI_TYPE_VOXEL_GI,
                "gi_quality" => Self::GI_QUALITY_LOW,
                "ssao_quality" => EnvironmentSsaoQuality::MEDIUM.ord() as i64,
                "ssil_quality" => -1_i64,  // Disabled
                "bloom" => true,
                "volumetric_fog" => true,
            },
        };
        Self {
            base,
            metalfx_supported,
            defaults,
            config_file: ConfigFile::new_gd(),
        }
    }

    fn ready(&mut self) {
        self.load_settings();
    }

    fn input(&mut self, input_event: Gd<InputEvent>) {
        if input_event.is_action_pressed("toggle_fullscreen") {
            let mut window = self.base().get_window().unwrap();
            let mode = window.get_mode();
            window.set_mode(
                if !((mode == WindowMode::EXCLUSIVE_FULLSCREEN) || (mode == WindowMode::FULLSCREEN)) {
                    WindowMode::EXCLUSIVE_FULLSCREEN
                } else {
                    WindowMode::WINDOWED
                },
            );
            self.base().get_viewport().unwrap().set_input_as_handled();
        }
    }
}

#[godot_api]
impl Settings {
    #[constant]
    const GI_TYPE_SDFGI: i64 = 0;
    #[constant]
    const GI_TYPE_VOXEL_GI: i64 = 1;
    #[constant]
    const GI_TYPE_LIGHTMAP_GI: i64 = 2;
    #[constant]
    const GI_QUALITY_DISABLED: i64 = 0;
    #[constant]
    const GI_QUALITY_LOW: i64 = 1;
    #[constant]
    const GI_QUALITY_HIGH: i64 = 2;

    #[func]
    fn load_settings(&mut self) {
        self.config_file.load(CONFIG_FILE_PATH);
        // Initialize defaults for values not found in the existing configuration file,
        // so we don't have to specify them every time we use `ConfigFile.get_value()`.
        for section in self.defaults.keys_shared() {
            let section_defaults = self.defaults.at(&section).to::<VarDictionary>();
            for key in section_defaults.keys_shared() {
                let section_name = section.to::<GString>();
                let key_name = key.to::<GString>();
                if !self.config_file.has_section_key(&section_name, &key_name) {
                    self.config_file
                        .set_value(&section_name, &key_name, &section_defaults.at(&key));
                }
            }
        }
    }

    #[func]
    fn save_settings(&mut self) {
        self.config_file.save(CONFIG_FILE_PATH);
    }

    #[func]
    fn apply_graphics_settings(
        &mut self,
        mut window: Gd<Window>,
        mut environment: Gd<Environment>,
        mut scene_root: Gd<Node>,
    ) {
        self.base().get_window().unwrap().set_mode(WindowMode::from_ord(
            self.config_file.get_value("video", "display_mode").to::<i64>() as i32,
        ));
        DisplayServer::singleton().window_set_vsync_mode(VSyncMode::from_ord(
            self.config_file.get_value("video", "vsync").to::<i64>() as i32,
        ));
        Engine::singleton().set_max_fps(self.config_file.get_value("video", "max_fps").to::<i64>() as i32);
        window.set_scaling_3d_scale(self.config_file.get_value("video", "resolution_scale").to::<f64>() as f32);
        window.set_scaling_3d_mode(Scaling3DMode::from_ord(
            self.config_file.get_value("video", "scale_filter").to::<i64>() as i32,
        ));

        window.set_use_taa(self.config_file.get_value("rendering", "taa").to::<bool>());
        window.set_msaa_3d(Msaa::from_ord(self.config_file.get_value("rendering", "msaa").to::<i64>() as i32));
        window.set_screen_space_aa(ScreenSpaceAa::from_ord(
            self.config_file.get_value("rendering", "screen_space_aa").to::<i64>() as i32,
        ));

        if !self.config_file.get_value("rendering", "shadow_mapping").to::<bool>() {
            // Disable shadows for all lights present during level load,
            // reducing the number of draw calls significantly.
            // FIXME: In the main menu, shadows aren't enabled again after enabling shadows
            // if they were previously disabled. We can't enable shadows on all lights unconditionally,
            // as this would negatively affect the level's performance.
            scene_root
                .propagate_call_ex("set")
                .args(&varray!["shadow_enabled", false])
                .done();
        }

        if self.config_file.get_value("rendering", "ssao_quality").to::<i64>() == -1 {
            environment.set_ssao_enabled(false);
        // upstream bug fix: settings.gd used `if` instead of `elif` — "SSAO: Disabled" (-1) was re-enabled by the else
        } else if self.config_file.get_value("rendering", "ssao_quality").to::<i64>()
            == EnvironmentSsaoQuality::MEDIUM.ord() as i64
        {
            environment.set_ssao_enabled(true);
            RenderingServer::singleton()
                .environment_set_ssao_quality(EnvironmentSsaoQuality::HIGH, false, 0.5, 2, 50.0, 300.0);
        } else {
            environment.set_ssao_enabled(true);
            RenderingServer::singleton()
                .environment_set_ssao_quality(EnvironmentSsaoQuality::MEDIUM, true, 0.5, 2, 50.0, 300.0);
        }

        if self.config_file.get_value("rendering", "ssil_quality").to::<i64>() == -1 {
            environment.set_ssil_enabled(false);
        } else if self.config_file.get_value("rendering", "ssil_quality").to::<i64>()
            == EnvironmentSsilQuality::MEDIUM.ord() as i64
        {
            environment.set_ssil_enabled(true);
            RenderingServer::singleton()
                .environment_set_ssil_quality(EnvironmentSsilQuality::MEDIUM, false, 0.5, 2, 50.0, 300.0);
        } else {
            environment.set_ssil_enabled(true);
            RenderingServer::singleton()
                .environment_set_ssil_quality(EnvironmentSsilQuality::HIGH, true, 0.5, 2, 50.0, 300.0);
        }

        environment.set_glow_enabled(self.config_file.get_value("rendering", "bloom").to::<bool>());
        environment.set_volumetric_fog_enabled(self.config_file.get_value("rendering", "volumetric_fog").to::<bool>());
    }
}
