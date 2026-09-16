use godot::classes::display_server::VSyncMode;
use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality};
use godot::classes::resource_loader::ThreadLoadStatus;
use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa};
use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    BaseButton, Button, ButtonGroup, ConfigFile, Control, DisplayServer, ENetMultiplayerPeer,
    HBoxContainer, INode, LineEdit, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene,
    ProgressBar, RenderingServer, ResourceLoader, SpinBox, Timer, VBoxContainer, WorldEnvironment,
};
use godot::global::is_equal_approx;
use godot::prelude::*;

const LEVEL_PATH: &str = "res://level/level.tscn";

// Viewport.SCALING_3D_MODE_NEAREST was added in Godot 4.7; absent from the prebuilt 4.6 API of
// gdext 0.5.5. Godot 4.7.2 = 5 (same technique as the Settings.GIType integers).
const SCALING_3D_MODE_NEAREST: i64 = 5;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct Menu {
    base: Base<Node>,

    #[init(val = OfflineMultiplayerPeer::new_gd().upcast())]
    peer: Gd<MultiplayerPeer>,

    #[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"))]
    metalfx_supported: bool,

    #[init(node = "WorldEnvironment")]
    world_environment: OnReady<Gd<WorldEnvironment>>,
    #[init(node = "UI")]
    ui: OnReady<Gd<Control>>,
    #[init(node = "UI/Main")]
    main: OnReady<Gd<Control>>,
    #[init(node = "UI/Main/Play")]
    play_button: OnReady<Gd<Button>>,
    #[init(node = "UI/Main/Settings")]
    settings_button: OnReady<Gd<Button>>,
    #[init(node = "UI/Main/Quit")]
    quit_button: OnReady<Gd<Button>>,
    #[init(node = "UI/Online")]
    online: OnReady<Gd<Control>>,
    #[init(node = "UI/Online/Port")]
    online_port: OnReady<Gd<SpinBox>>,
    #[init(node = "UI/Online/Address")]
    online_address: OnReady<Gd<LineEdit>>,
    #[init(node = "UI/Settings")]
    settings_menu: OnReady<Gd<VBoxContainer>>,
    #[init(node = "UI/Settings/Actions")]
    settings_actions: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/Actions/Apply")]
    settings_action_apply: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/Actions/Cancel")]
    settings_action_cancel: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/DisplayMode")]
    display_mode_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/DisplayMode/Windowed")]
    display_mode_windowed: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/DisplayMode/Fullscreen")]
    display_mode_fullscreen: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/DisplayMode/ExclusiveFullscreen")]
    display_mode_exclusive_fullscreen: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VSync")]
    vsync_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/VSync/Disabled")]
    vsync_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VSync/Enabled")]
    vsync_enabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VSync/Adaptive")]
    vsync_adaptive: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VSync/Mailbox")]
    vsync_mailbox: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS")]
    max_fps_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/MaxFPS/30")]
    max_fps_30: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/40")]
    max_fps_40: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/60")]
    max_fps_60: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/72")]
    max_fps_72: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/90")]
    max_fps_90: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/120")]
    max_fps_120: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/144")]
    max_fps_144: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MaxFPS/Unlimited")]
    max_fps_unlimited: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale")]
    resolution_scale_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/ResolutionScale/UltraPerformance")]
    resolution_scale_ultra_performance: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale/Performance")]
    resolution_scale_performance: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale/Balanced")]
    resolution_scale_balanced: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale/Quality")]
    resolution_scale_quality: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale/UltraQuality")]
    resolution_scale_ultra_quality: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ResolutionScale/Native")]
    resolution_scale_native: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter")]
    scale_filter_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/ScaleFilter/Nearest")]
    scale_filter_nearest: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter/Bilinear")]
    scale_filter_bilinear: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter/FSR1")]
    scale_filter_fsr1: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter/MetalFXSpatial")]
    scale_filter_metalfx_spatial: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter/FSR2")]
    scale_filter_fsr2: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScaleFilter/MetalFXTemporal")]
    scale_filter_metalfx_temporal: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/TAA")]
    taa_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/TAA/Disabled")]
    taa_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/TAA/Enabled")]
    taa_enabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MSAA")]
    msaa_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/MSAA/Disabled")]
    msaa_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MSAA/2X")]
    msaa_2x: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MSAA/4X")]
    msaa_4x: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/MSAA/8X")]
    msaa_8x: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScreenSpaceAA")]
    screen_space_aa_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/ScreenSpaceAA/Disabled")]
    screen_space_aa_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScreenSpaceAA/FXAA")]
    screen_space_aa_fxaa: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ScreenSpaceAA/SMAA")]
    screen_space_aa_smaa: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ShadowMapping")]
    shadow_mapping_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/ShadowMapping/Disabled")]
    shadow_mapping_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/ShadowMapping/Enabled")]
    shadow_mapping_enabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIType")]
    gi_type_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/GIType/LightmapGI")]
    gi_lightmapgi: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIType/VoxelGI")]
    gi_voxelgi: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIType/SDFGI")]
    gi_sdfgi: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIQuality")]
    gi_quality_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/GIQuality/Disabled")]
    gi_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIQuality/Low")]
    gi_low: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/GIQuality/High")]
    gi_high: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSAO")]
    ssao_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/SSAO/Disabled")]
    ssao_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSAO/Medium")]
    ssao_medium: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSAO/High")]
    ssao_high: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSIL")]
    ssil_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/SSIL/Disabled")]
    ssil_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSIL/Medium")]
    ssil_medium: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/SSIL/High")]
    ssil_high: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/Bloom")]
    bloom_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/Bloom/Disabled")]
    bloom_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/Bloom/Enabled")]
    bloom_enabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VolumetricFog")]
    volumetric_fog_menu: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Settings/VolumetricFog/Disabled")]
    volumetric_fog_disabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Settings/VolumetricFog/Enabled")]
    volumetric_fog_enabled: OnReady<Gd<Button>>,
    #[init(node = "UI/Loading")]
    loading: OnReady<Gd<HBoxContainer>>,
    #[init(node = "UI/Loading/Progress")]
    loading_progress: OnReady<Gd<ProgressBar>>,
    #[init(node = "UI/Loading/DoneTimer")]
    loading_done_timer: OnReady<Gd<Timer>>,}

#[godot_api]
impl INode for Menu {
    fn ready(&mut self) {
        // Apply relevant settings directly.
        let window = self.base().get_window().unwrap();
        let environment = self.world_environment.get_environment().unwrap();
        self.base().get_node_as::<Node>("/root/Settings").call(
            "apply_graphics_settings",
            &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()],
        );

        if DisplayServer::singleton().get_name() == GString::from("headless") {
            self.base_mut().call_deferred("_on_host_pressed", &[]);
        }

        self.play_button.grab_focus();

        if !self.metalfx_supported {
            self.scale_filter_metalfx_spatial.hide();
            self.scale_filter_metalfx_temporal.hide();
        }

        self._make_button_group(self.display_mode_menu.clone().upcast());
        self._make_button_group(self.vsync_menu.clone().upcast());
        self._make_button_group(self.max_fps_menu.clone().upcast());
        self._make_button_group(self.resolution_scale_menu.clone().upcast());
        self._make_button_group(self.scale_filter_menu.clone().upcast());
        self._make_button_group(self.taa_menu.clone().upcast());
        self._make_button_group(self.msaa_menu.clone().upcast());
        self._make_button_group(self.screen_space_aa_menu.clone().upcast());
        self._make_button_group(self.shadow_mapping_menu.clone().upcast());
        self._make_button_group(self.gi_type_menu.clone().upcast());
        self._make_button_group(self.gi_quality_menu.clone().upcast());
        self._make_button_group(self.ssao_menu.clone().upcast());
        self._make_button_group(self.ssil_menu.clone().upcast());
        self._make_button_group(self.bloom_menu.clone().upcast());
        self._make_button_group(self.volumetric_fog_menu.clone().upcast());
    }

    fn process(&mut self, _delta: f64) {
        if self.loading.is_visible() {
            let progress = VarArray::new();
            let status: ThreadLoadStatus = ResourceLoader::singleton()
                .load_threaded_get_status_ex(LEVEL_PATH)
                .progress(&progress)
                .done();
            if status == ThreadLoadStatus::IN_PROGRESS {
                self.loading_progress.set_value(progress.at(0).to::<f64>() * 100.0);
            } else if status == ThreadLoadStatus::LOADED {
                self.loading_progress.set_value(100.0);
                self.base_mut().set_process(false);
                self.loading_done_timer.start();
            } else {
                godot_print!("Error while loading level: {}", status.ord());
                self.main.show();
                self.loading.hide();
            }
        }
    }
}

impl Menu {
    fn _make_button_group(&mut self, common_parent: Gd<Node>) {
        let group = ButtonGroup::new_gd();
        for btn in common_parent.get_children().iter_shared() {
            if let Ok(mut btn) = btn.try_cast::<BaseButton>() {
                btn.set_button_group(&group);
            }
        }
    }
}

#[godot_api]
impl Menu {
    #[signal]
    fn replace_main_scene(scene: Gd<PackedScene>);

    #[func]
    fn _on_loading_done_timer_timeout(&mut self) {
        let peer = self.peer.clone();
        self.base().get_multiplayer().unwrap().set_multiplayer_peer(&peer);
        let scene = ResourceLoader::singleton()
            .load_threaded_get(LEVEL_PATH)
            .unwrap()
            .cast::<PackedScene>();
        self.signals().replace_main_scene().emit(&scene);
    }

    #[func]
    fn _on_play_pressed(&mut self) {
        self.main.hide();
        self.loading.show();
        ResourceLoader::singleton()
            .load_threaded_request_ex(LEVEL_PATH)
            .use_sub_threads(true)
            .done();
    }

    #[func]
    fn _on_settings_pressed(&mut self) {
        self.main.hide();
        self.settings_menu.show();
        self.settings_action_cancel.grab_focus();

        let config_file = self
            .base()
            .get_node_as::<Node>("/root/Settings")
            .get("config_file")
            .to::<Gd<ConfigFile>>();

        let display_mode = config_file.get_value("video", "display_mode").to::<i64>();
        if display_mode == WindowMode::WINDOWED.ord() as i64
            || display_mode == WindowMode::MAXIMIZED.ord() as i64
        {
            self.display_mode_windowed.set_pressed(true);
        } else if display_mode == WindowMode::FULLSCREEN.ord() as i64 {
            self.display_mode_fullscreen.set_pressed(true);
        } else {
            self.display_mode_exclusive_fullscreen.set_pressed(true);
        }

        let vsync = config_file.get_value("video", "vsync").to::<i64>();
        if vsync == VSyncMode::DISABLED.ord() as i64 {
            self.vsync_disabled.set_pressed(true);
        } else if vsync == VSyncMode::ENABLED.ord() as i64 {
            self.vsync_enabled.set_pressed(true);
        } else if vsync == VSyncMode::ADAPTIVE.ord() as i64 {
            self.vsync_adaptive.set_pressed(true);
        } else {
            self.vsync_mailbox.set_pressed(true);
        }

        let max_fps = config_file.get_value("video", "max_fps").to::<i64>();
        if max_fps == 30 {
            self.max_fps_30.set_pressed(true);
        } else if max_fps == 40 {
            self.max_fps_40.set_pressed(true);
        } else if max_fps == 60 {
            self.max_fps_60.set_pressed(true);
        } else if max_fps == 72 {
            self.max_fps_72.set_pressed(true);
        } else if max_fps == 90 {
            self.max_fps_90.set_pressed(true);
        } else if max_fps == 120 {
            self.max_fps_120.set_pressed(true);
        } else if max_fps == 144 {
            self.max_fps_144.set_pressed(true);
        } else {
            self.max_fps_unlimited.set_pressed(true);
        }

        let resolution_scale = config_file.get_value("video", "resolution_scale").to::<f64>();
        if is_equal_approx(resolution_scale, 1.0 / 3.0) {
            self.resolution_scale_ultra_performance.set_pressed(true);
        } else if is_equal_approx(resolution_scale, 1.0 / 2.0) {
            self.resolution_scale_performance.set_pressed(true);
        } else if is_equal_approx(resolution_scale, 1.0 / 1.7) {
            self.resolution_scale_balanced.set_pressed(true);
        } else if is_equal_approx(resolution_scale, 1.0 / 1.5) {
            self.resolution_scale_quality.set_pressed(true);
        } else if is_equal_approx(resolution_scale, 1.0 / 1.3) {
            self.resolution_scale_ultra_quality.set_pressed(true);
        } else {
            self.resolution_scale_native.set_pressed(true);
        }

        let scale_filter = config_file.get_value("video", "scale_filter").to::<i64>();
        if scale_filter == SCALING_3D_MODE_NEAREST {
            self.scale_filter_nearest.set_pressed(true);
        } else if scale_filter == Scaling3DMode::BILINEAR.ord() as i64 {
            self.scale_filter_bilinear.set_pressed(true);
        } else if scale_filter == Scaling3DMode::FSR.ord() as i64 {
            self.scale_filter_fsr1.set_pressed(true);
        } else if scale_filter == Scaling3DMode::FSR2.ord() as i64 {
            self.scale_filter_fsr2.set_pressed(true);
        } else if scale_filter == Scaling3DMode::METALFX_SPATIAL.ord() as i64 {
            self.scale_filter_metalfx_spatial.set_pressed(true);
        } else if scale_filter == Scaling3DMode::METALFX_TEMPORAL.ord() as i64 {
            self.scale_filter_metalfx_temporal.set_pressed(true);
        } else if self.metalfx_supported {
            self.scale_filter_metalfx_temporal.set_pressed(true);
        } else {
            self.scale_filter_fsr2.set_pressed(true);
        }

        let gi_type = config_file.get_value("rendering", "gi_type").to::<i64>();
        if gi_type == 2 {
            self.gi_lightmapgi.set_pressed(true);
        } else if gi_type == 1 {
            self.gi_voxelgi.set_pressed(true);
        } else if gi_type == 0 {
            self.gi_sdfgi.set_pressed(true);
        }

        let gi_quality = config_file.get_value("rendering", "gi_quality").to::<i64>();
        if gi_quality == 0 {
            self.gi_disabled.set_pressed(true);
        } else if gi_quality == 1 {
            self.gi_low.set_pressed(true);
        } else if gi_quality == 2 {
            self.gi_high.set_pressed(true);
        }

        if !config_file.get_value("rendering", "taa").to::<bool>() {
            self.taa_disabled.set_pressed(true);
        } else {
            self.taa_enabled.set_pressed(true);
        }

        let msaa = config_file.get_value("rendering", "msaa").to::<i64>();
        if msaa == Msaa::DISABLED.ord() as i64 {
            self.msaa_disabled.set_pressed(true);
        } else if msaa == Msaa::MSAA_2X.ord() as i64 {
            self.msaa_2x.set_pressed(true);
        } else if msaa == Msaa::MSAA_4X.ord() as i64 {
            self.msaa_4x.set_pressed(true);
        } else if msaa == Msaa::MSAA_8X.ord() as i64 {
            self.msaa_8x.set_pressed(true);
        }

        let screen_space_aa = config_file.get_value("rendering", "screen_space_aa").to::<i64>();
        if screen_space_aa == ScreenSpaceAa::DISABLED.ord() as i64 {
            self.screen_space_aa_disabled.set_pressed(true);
        } else if screen_space_aa == ScreenSpaceAa::FXAA.ord() as i64 {
            self.screen_space_aa_fxaa.set_pressed(true);
        } else if screen_space_aa == ScreenSpaceAa::SMAA.ord() as i64 {
            self.screen_space_aa_smaa.set_pressed(true);
        }

        if !config_file.get_value("rendering", "shadow_mapping").to::<bool>() {
            self.shadow_mapping_disabled.set_pressed(true);
        } else {
            self.shadow_mapping_enabled.set_pressed(true);
        }

        let ssao_quality = config_file.get_value("rendering", "ssao_quality").to::<i64>();
        if ssao_quality == -1 {
            self.ssao_disabled.set_pressed(true);
        } else if ssao_quality == EnvironmentSsaoQuality::MEDIUM.ord() as i64 {
            self.ssao_medium.set_pressed(true);
        } else if ssao_quality == EnvironmentSsaoQuality::HIGH.ord() as i64 {
            self.ssao_high.set_pressed(true);
        }

        let ssil_quality = config_file.get_value("rendering", "ssil_quality").to::<i64>();
        if ssil_quality == -1 {
            self.ssil_disabled.set_pressed(true);
        } else if ssil_quality == EnvironmentSsilQuality::MEDIUM.ord() as i64 {
            self.ssil_medium.set_pressed(true);
        } else if ssil_quality == EnvironmentSsilQuality::HIGH.ord() as i64 {
            self.ssil_high.set_pressed(true);
        }

        if !config_file.get_value("rendering", "bloom").to::<bool>() {
            self.bloom_disabled.set_pressed(true);
        } else {
            self.bloom_enabled.set_pressed(true);
        }

        if !config_file.get_value("rendering", "volumetric_fog").to::<bool>() {
            self.volumetric_fog_disabled.set_pressed(true);
        } else {
            self.volumetric_fog_enabled.set_pressed(true);
        }
    }

    #[func]
    fn _on_quit_pressed(&mut self) {
        self.base().get_tree().quit();
    }

    #[func]
    fn _on_apply_pressed(&mut self) {
        self.main.show();
        self.play_button.grab_focus();
        self.settings_menu.hide();

        let mut settings = self.base().get_node_as::<Node>("/root/Settings");
        let mut config_file = settings.get("config_file").to::<Gd<ConfigFile>>();

        if self.display_mode_windowed.is_pressed() {
            config_file.set_value("video", "display_mode", &(WindowMode::WINDOWED.ord() as i64).to_variant());
        } else if self.display_mode_fullscreen.is_pressed() {
            config_file.set_value("video", "display_mode", &(WindowMode::FULLSCREEN.ord() as i64).to_variant());
        } else if self.display_mode_exclusive_fullscreen.is_pressed() {
            config_file.set_value("video", "display_mode", &(WindowMode::EXCLUSIVE_FULLSCREEN.ord() as i64).to_variant());
        }

        if self.vsync_disabled.is_pressed() {
            config_file.set_value("video", "vsync", &(VSyncMode::DISABLED.ord() as i64).to_variant());
        } else if self.vsync_enabled.is_pressed() {
            config_file.set_value("video", "vsync", &(VSyncMode::ENABLED.ord() as i64).to_variant());
        } else if self.vsync_adaptive.is_pressed() {
            config_file.set_value("video", "vsync", &(VSyncMode::ADAPTIVE.ord() as i64).to_variant());
        } else if self.vsync_mailbox.is_pressed() {
            config_file.set_value("video", "vsync", &(VSyncMode::MAILBOX.ord() as i64).to_variant());
        }

        if self.max_fps_30.is_pressed() {
            config_file.set_value("video", "max_fps", &30_i64.to_variant());
        } else if self.max_fps_40.is_pressed() {
            config_file.set_value("video", "max_fps", &40_i64.to_variant());
        } else if self.max_fps_60.is_pressed() {
            config_file.set_value("video", "max_fps", &60_i64.to_variant());
        } else if self.max_fps_72.is_pressed() {
            config_file.set_value("video", "max_fps", &72_i64.to_variant());
        } else if self.max_fps_90.is_pressed() {
            config_file.set_value("video", "max_fps", &90_i64.to_variant());
        } else if self.max_fps_120.is_pressed() {
            config_file.set_value("video", "max_fps", &120_i64.to_variant());
        } else if self.max_fps_144.is_pressed() {
            config_file.set_value("video", "max_fps", &144_i64.to_variant());
        } else if self.max_fps_unlimited.is_pressed() {
            config_file.set_value("video", "max_fps", &0_i64.to_variant());
        }

        if self.resolution_scale_ultra_performance.is_pressed() {
            config_file.set_value("video", "resolution_scale", &(1.0 / 3.0_f64).to_variant());
        } else if self.resolution_scale_performance.is_pressed() {
            config_file.set_value("video", "resolution_scale", &(1.0 / 2.0_f64).to_variant());
        } else if self.resolution_scale_balanced.is_pressed() {
            config_file.set_value("video", "resolution_scale", &(1.0 / 1.7_f64).to_variant());
        } else if self.resolution_scale_quality.is_pressed() {
            config_file.set_value("video", "resolution_scale", &(1.0 / 1.5_f64).to_variant());
        } else if self.resolution_scale_ultra_quality.is_pressed() {
            config_file.set_value("video", "resolution_scale", &(1.0 / 1.3_f64).to_variant());
        } else if self.resolution_scale_native.is_pressed() {
            config_file.set_value("video", "resolution_scale", &1.0_f64.to_variant());
        }

        if self.scale_filter_nearest.is_pressed() {
            config_file.set_value("video", "scale_filter", &SCALING_3D_MODE_NEAREST.to_variant());
        } else if self.scale_filter_bilinear.is_pressed() {
            config_file.set_value("video", "scale_filter", &(Scaling3DMode::BILINEAR.ord() as i64).to_variant());
        } else if self.scale_filter_fsr1.is_pressed() {
            config_file.set_value("video", "scale_filter", &(Scaling3DMode::FSR.ord() as i64).to_variant());
        } else if self.scale_filter_fsr2.is_pressed() {
            config_file.set_value("video", "scale_filter", &(Scaling3DMode::FSR2.ord() as i64).to_variant());
        } else if self.scale_filter_metalfx_spatial.is_pressed() {
            config_file.set_value("video", "scale_filter", &(Scaling3DMode::METALFX_SPATIAL.ord() as i64).to_variant());
        } else if self.scale_filter_metalfx_temporal.is_pressed() {
            config_file.set_value("video", "scale_filter", &(Scaling3DMode::METALFX_TEMPORAL.ord() as i64).to_variant());
        }

        if self.gi_lightmapgi.is_pressed() {
            config_file.set_value("rendering", "gi_type", &2_i64.to_variant());
        } else if self.gi_voxelgi.is_pressed() {
            config_file.set_value("rendering", "gi_type", &1_i64.to_variant());
        } else if self.gi_sdfgi.is_pressed() {
            config_file.set_value("rendering", "gi_type", &0_i64.to_variant());
        }

        if self.gi_low.is_pressed() {
            config_file.set_value("rendering", "gi_quality", &1_i64.to_variant());
        } else if self.gi_high.is_pressed() {
            config_file.set_value("rendering", "gi_quality", &2_i64.to_variant());
        } else if self.gi_disabled.is_pressed() {
            config_file.set_value("rendering", "gi_quality", &0_i64.to_variant());
        }

        config_file.set_value("rendering", "taa", &self.taa_enabled.is_pressed().to_variant());

        if self.msaa_disabled.is_pressed() {
            config_file.set_value("rendering", "msaa", &(Msaa::DISABLED.ord() as i64).to_variant());
        } else if self.msaa_2x.is_pressed() {
            config_file.set_value("rendering", "msaa", &(Msaa::MSAA_2X.ord() as i64).to_variant());
        } else if self.msaa_4x.is_pressed() {
            config_file.set_value("rendering", "msaa", &(Msaa::MSAA_4X.ord() as i64).to_variant());
        } else if self.msaa_8x.is_pressed() {
            config_file.set_value("rendering", "msaa", &(Msaa::MSAA_8X.ord() as i64).to_variant());
        }

        if self.screen_space_aa_disabled.is_pressed() {
            config_file.set_value("rendering", "screen_space_aa", &(ScreenSpaceAa::DISABLED.ord() as i64).to_variant());
        } else if self.screen_space_aa_fxaa.is_pressed() {
            config_file.set_value("rendering", "screen_space_aa", &(ScreenSpaceAa::FXAA.ord() as i64).to_variant());
        } else if self.screen_space_aa_smaa.is_pressed() {
            config_file.set_value("rendering", "screen_space_aa", &(ScreenSpaceAa::SMAA.ord() as i64).to_variant());
        }

        config_file.set_value("rendering", "shadow_mapping", &self.shadow_mapping_enabled.is_pressed().to_variant());

        if self.ssao_disabled.is_pressed() {
            config_file.set_value("rendering", "ssao_quality", &(-1_i64).to_variant());
        } else if self.ssao_medium.is_pressed() {
            config_file.set_value("rendering", "ssao_quality", &(EnvironmentSsaoQuality::MEDIUM.ord() as i64).to_variant());
        } else if self.ssao_high.is_pressed() {
            config_file.set_value("rendering", "ssao_quality", &(EnvironmentSsaoQuality::HIGH.ord() as i64).to_variant());
        }

        if self.ssil_disabled.is_pressed() {
            config_file.set_value("rendering", "ssil_quality", &(-1_i64).to_variant());
        } else if self.ssil_medium.is_pressed() {
            config_file.set_value("rendering", "ssil_quality", &(EnvironmentSsilQuality::MEDIUM.ord() as i64).to_variant());
        } else if self.ssil_high.is_pressed() {
            config_file.set_value("rendering", "ssil_quality", &(EnvironmentSsilQuality::HIGH.ord() as i64).to_variant());
        }

        config_file.set_value("rendering", "bloom", &self.bloom_enabled.is_pressed().to_variant());
        config_file.set_value("rendering", "volumetric_fog", &self.volumetric_fog_enabled.is_pressed().to_variant());

        // Apply relevant settings directly.
        let window = self.base().get_window().unwrap();
        let environment = self.world_environment.get_environment().unwrap();
        settings.call(
            "apply_graphics_settings",
            &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()],
        );

        settings.call("save_settings", &[]);
    }

    #[func]
    fn _on_cancel_pressed(&mut self) {
        self.main.show();
        self.play_button.grab_focus();
        self.settings_menu.hide();
        self.online.hide();
    }

    #[func]
    fn _on_play_online_pressed(&mut self) {
        self.online.show();
        self.main.hide();
    }

    #[func]
    fn _on_host_pressed(&mut self) {
        let mut peer = ENetMultiplayerPeer::new_gd();
        peer.create_server(self.online_port.get_value() as i32);
        self.peer = peer.upcast();
        self._on_play_pressed();
        self.online.hide();
    }

    #[func]
    fn _on_connect_pressed(&mut self) {
        let mut peer = ENetMultiplayerPeer::new_gd();
        peer.create_client(&self.online_address.get_text(), self.online_port.get_value() as i32);
        self.peer = peer.upcast();
        self._on_play_pressed();
        self.online.hide();
    }
}
