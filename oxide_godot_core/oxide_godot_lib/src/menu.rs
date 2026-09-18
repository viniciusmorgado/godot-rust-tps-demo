mod model;

use crate::settings::{GiQuality, GiType, ScaleFilter, Settings, SsaoQuality, SsilQuality};
use godot::classes::display_server::VSyncMode;
use godot::classes::resource_loader::ThreadLoadStatus;
use godot::classes::viewport::{Msaa, ScreenSpaceAa};
use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    BaseButton, Button, ButtonGroup, Control, DisplayServer, ENetMultiplayerPeer, HBoxContainer,
    INode, LineEdit, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene, ProgressBar,
    RenderingServer, ResourceLoader, SpinBox, Timer, VBoxContainer, WorldEnvironment,
};
use godot::prelude::*;

const LEVEL_PATH: &str = "res://level/level.tscn";

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct Menu {
    base: Base<Node>,

    #[init(val = OfflineMultiplayerPeer::new_gd().upcast())]
    peer: Gd<MultiplayerPeer>,

    #[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == "metal")]
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
    loading_done_timer: OnReady<Gd<Timer>>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

#[godot_api]
impl INode for Menu {
    fn ready(&mut self) {
        // Apply relevant settings directly.
        let window = self.base().get_window().unwrap();
        let environment = self.world_environment.get_environment().unwrap();
        let scene_root: Gd<Node> = self.to_gd().upcast();
        self.settings.bind_mut().apply_graphics_settings(window, environment, scene_root);

        if DisplayServer::singleton().get_name() == "headless" {
            let mut this = self.to_gd();
            Callable::from_fn("_on_host_pressed", move |_args| {
                this.bind_mut()._on_host_pressed();
                Variant::nil()
            })
            .call_deferred(&[]);
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
        if !self.loading.is_visible() {
            return;
        }
        let progress = VarArray::new();
        let status: ThreadLoadStatus = ResourceLoader::singleton()
            .load_threaded_get_status_ex(LEVEL_PATH)
            .progress(&progress)
            .done();
        match model::loading_step(status, progress.at(0).to::<f64>()) {
            model::LoadingCmd::UpdateProgress(p) => self.loading_progress.set_value(p),
            model::LoadingCmd::Finished => {
                self.loading_progress.set_value(100.0);
                self.base_mut().set_process(false);
                self.loading_done_timer.start();
            }
            model::LoadingCmd::Failed => {
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
    pub fn replace_main_scene(scene: Gd<PackedScene>);

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

        let g = self.settings.bind().graphics();
        self.show_display_mode(g.display_mode);
        self.show_vsync(g.vsync);
        self.show_max_fps(g.max_fps);
        self.show_resolution_scale(g.resolution_scale);
        self.show_scale_filter(g.scale_filter);
        self.show_gi_type(g.gi_type);
        self.show_gi_quality(g.gi_quality);
        self.show_bool(g.taa, self.taa_enabled.clone(), self.taa_disabled.clone());
        self.show_msaa(g.msaa);
        self.show_screen_space_aa(g.screen_space_aa);
        self.show_bool(
            g.shadow_mapping,
            self.shadow_mapping_enabled.clone(),
            self.shadow_mapping_disabled.clone(),
        );
        self.show_ssao_quality(g.ssao_quality);
        self.show_ssil_quality(g.ssil_quality);
        self.show_bool(g.bloom, self.bloom_enabled.clone(), self.bloom_disabled.clone());
        self.show_bool(
            g.volumetric_fog,
            self.volumetric_fog_enabled.clone(),
            self.volumetric_fog_disabled.clone(),
        );
    }

    fn show_bool(&self, value: bool, mut enabled: Gd<Button>, mut disabled: Gd<Button>) {
        if value {
            enabled.set_pressed(true);
        } else {
            disabled.set_pressed(true);
        }
    }

    fn show_display_mode(&mut self, mode: WindowMode) {
        match model::display_mode_button(mode) {
            model::DisplayModeButton::Windowed => self.display_mode_windowed.set_pressed(true),
            model::DisplayModeButton::Fullscreen => self.display_mode_fullscreen.set_pressed(true),
            model::DisplayModeButton::ExclusiveFullscreen => {
                self.display_mode_exclusive_fullscreen.set_pressed(true)
            }
        }
    }

    fn show_vsync(&mut self, vsync: VSyncMode) {
        match model::vsync_button(vsync) {
            model::VsyncButton::Disabled => self.vsync_disabled.set_pressed(true),
            model::VsyncButton::Enabled => self.vsync_enabled.set_pressed(true),
            model::VsyncButton::Adaptive => self.vsync_adaptive.set_pressed(true),
            model::VsyncButton::Mailbox => self.vsync_mailbox.set_pressed(true),
        }
    }

    fn show_max_fps(&mut self, fps: i32) {
        match model::max_fps_button(fps) {
            model::MaxFpsButton::Fps30 => self.max_fps_30.set_pressed(true),
            model::MaxFpsButton::Fps40 => self.max_fps_40.set_pressed(true),
            model::MaxFpsButton::Fps60 => self.max_fps_60.set_pressed(true),
            model::MaxFpsButton::Fps72 => self.max_fps_72.set_pressed(true),
            model::MaxFpsButton::Fps90 => self.max_fps_90.set_pressed(true),
            model::MaxFpsButton::Fps120 => self.max_fps_120.set_pressed(true),
            model::MaxFpsButton::Fps144 => self.max_fps_144.set_pressed(true),
            model::MaxFpsButton::Unlimited => self.max_fps_unlimited.set_pressed(true),
        }
    }

    fn show_resolution_scale(&mut self, scale: f64) {
        match model::resolution_scale_button(scale) {
            model::ResolutionScaleButton::UltraPerformance => {
                self.resolution_scale_ultra_performance.set_pressed(true)
            }
            model::ResolutionScaleButton::Performance => self.resolution_scale_performance.set_pressed(true),
            model::ResolutionScaleButton::Balanced => self.resolution_scale_balanced.set_pressed(true),
            model::ResolutionScaleButton::Quality => self.resolution_scale_quality.set_pressed(true),
            model::ResolutionScaleButton::UltraQuality => {
                self.resolution_scale_ultra_quality.set_pressed(true)
            }
            model::ResolutionScaleButton::Native => self.resolution_scale_native.set_pressed(true),
        }
    }

    fn show_scale_filter(&mut self, filter: ScaleFilter) {
        match model::scale_filter_button(filter) {
            model::ScaleFilterButton::Nearest => self.scale_filter_nearest.set_pressed(true),
            model::ScaleFilterButton::Bilinear => self.scale_filter_bilinear.set_pressed(true),
            model::ScaleFilterButton::Fsr1 => self.scale_filter_fsr1.set_pressed(true),
            model::ScaleFilterButton::MetalFxSpatial => self.scale_filter_metalfx_spatial.set_pressed(true),
            model::ScaleFilterButton::Fsr2 => self.scale_filter_fsr2.set_pressed(true),
            model::ScaleFilterButton::MetalFxTemporal => {
                self.scale_filter_metalfx_temporal.set_pressed(true)
            }
        }
    }

    fn show_gi_type(&mut self, gi_type: GiType) {
        match model::gi_type_button(gi_type) {
            model::GiTypeButton::LightmapGi => self.gi_lightmapgi.set_pressed(true),
            model::GiTypeButton::VoxelGi => self.gi_voxelgi.set_pressed(true),
            model::GiTypeButton::Sdfgi => self.gi_sdfgi.set_pressed(true),
        }
    }

    fn show_gi_quality(&mut self, gi_quality: GiQuality) {
        match model::gi_quality_button(gi_quality) {
            model::GiQualityButton::Disabled => self.gi_disabled.set_pressed(true),
            model::GiQualityButton::Low => self.gi_low.set_pressed(true),
            model::GiQualityButton::High => self.gi_high.set_pressed(true),
        }
    }

    fn show_msaa(&mut self, msaa: Msaa) {
        match model::msaa_button(msaa) {
            Some(model::MsaaButton::Disabled) => self.msaa_disabled.set_pressed(true),
            Some(model::MsaaButton::Msaa2x) => self.msaa_2x.set_pressed(true),
            Some(model::MsaaButton::Msaa4x) => self.msaa_4x.set_pressed(true),
            Some(model::MsaaButton::Msaa8x) => self.msaa_8x.set_pressed(true),
            None => {}
        }
    }

    fn show_screen_space_aa(&mut self, aa: ScreenSpaceAa) {
        match model::screen_space_aa_button(aa) {
            Some(model::ScreenSpaceAaButton::Disabled) => self.screen_space_aa_disabled.set_pressed(true),
            Some(model::ScreenSpaceAaButton::Fxaa) => self.screen_space_aa_fxaa.set_pressed(true),
            Some(model::ScreenSpaceAaButton::Smaa) => self.screen_space_aa_smaa.set_pressed(true),
            None => {}
        }
    }

    fn show_ssao_quality(&mut self, quality: SsaoQuality) {
        match model::ssao_quality_button(quality) {
            model::SsaoQualityButton::Disabled => self.ssao_disabled.set_pressed(true),
            model::SsaoQualityButton::Medium => self.ssao_medium.set_pressed(true),
            model::SsaoQualityButton::High => self.ssao_high.set_pressed(true),
        }
    }

    fn show_ssil_quality(&mut self, quality: SsilQuality) {
        match model::ssil_quality_button(quality) {
            model::SsilQualityButton::Disabled => self.ssil_disabled.set_pressed(true),
            model::SsilQualityButton::Medium => self.ssil_medium.set_pressed(true),
            model::SsilQualityButton::High => self.ssil_high.set_pressed(true),
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

        let mut g = self.settings.bind().graphics();
        if let Some(b) = self.apply_display_mode() { g.display_mode = model::display_mode_value(b); }
        if let Some(b) = self.apply_vsync() { g.vsync = model::vsync_value(b); }
        if let Some(b) = self.apply_max_fps() { g.max_fps = model::max_fps_value(b); }
        if let Some(b) = self.apply_resolution_scale() { g.resolution_scale = model::resolution_scale_value(b); }
        if let Some(b) = self.apply_scale_filter() { g.scale_filter = model::scale_filter_value(b); }
        if let Some(b) = self.apply_gi_type() { g.gi_type = model::gi_type_value(b); }
        if let Some(b) = self.apply_gi_quality() { g.gi_quality = model::gi_quality_value(b); }
        g.taa = self.taa_enabled.is_pressed();
        if let Some(b) = self.apply_msaa() { g.msaa = model::msaa_value(b); }
        if let Some(b) = self.apply_screen_space_aa() { g.screen_space_aa = model::screen_space_aa_value(b); }
        g.shadow_mapping = self.shadow_mapping_enabled.is_pressed();
        if let Some(b) = self.apply_ssao_quality() { g.ssao_quality = model::ssao_quality_value(b); }
        if let Some(b) = self.apply_ssil_quality() { g.ssil_quality = model::ssil_quality_value(b); }
        g.bloom = self.bloom_enabled.is_pressed();
        g.volumetric_fog = self.volumetric_fog_enabled.is_pressed();

        self.settings.bind_mut().set_graphics(g);

        // Apply relevant settings directly.
        let window = self.base().get_window().unwrap();
        let environment = self.world_environment.get_environment().unwrap();
        let scene_root: Gd<Node> = self.to_gd().upcast();
        self.settings.bind_mut().apply_graphics_settings(window, environment, scene_root);

        self.settings.bind_mut().save_settings();
    }

    fn apply_display_mode(&self) -> Option<model::DisplayModeButton> {
        if self.display_mode_windowed.is_pressed() {
            Some(model::DisplayModeButton::Windowed)
        } else if self.display_mode_fullscreen.is_pressed() {
            Some(model::DisplayModeButton::Fullscreen)
        } else if self.display_mode_exclusive_fullscreen.is_pressed() {
            Some(model::DisplayModeButton::ExclusiveFullscreen)
        } else {
            None
        }
    }

    fn apply_vsync(&self) -> Option<model::VsyncButton> {
        if self.vsync_disabled.is_pressed() {
            Some(model::VsyncButton::Disabled)
        } else if self.vsync_enabled.is_pressed() {
            Some(model::VsyncButton::Enabled)
        } else if self.vsync_adaptive.is_pressed() {
            Some(model::VsyncButton::Adaptive)
        } else if self.vsync_mailbox.is_pressed() {
            Some(model::VsyncButton::Mailbox)
        } else {
            None
        }
    }

    fn apply_max_fps(&self) -> Option<model::MaxFpsButton> {
        if self.max_fps_30.is_pressed() {
            Some(model::MaxFpsButton::Fps30)
        } else if self.max_fps_40.is_pressed() {
            Some(model::MaxFpsButton::Fps40)
        } else if self.max_fps_60.is_pressed() {
            Some(model::MaxFpsButton::Fps60)
        } else if self.max_fps_72.is_pressed() {
            Some(model::MaxFpsButton::Fps72)
        } else if self.max_fps_90.is_pressed() {
            Some(model::MaxFpsButton::Fps90)
        } else if self.max_fps_120.is_pressed() {
            Some(model::MaxFpsButton::Fps120)
        } else if self.max_fps_144.is_pressed() {
            Some(model::MaxFpsButton::Fps144)
        } else if self.max_fps_unlimited.is_pressed() {
            Some(model::MaxFpsButton::Unlimited)
        } else {
            None
        }
    }

    fn apply_resolution_scale(&self) -> Option<model::ResolutionScaleButton> {
        if self.resolution_scale_ultra_performance.is_pressed() {
            Some(model::ResolutionScaleButton::UltraPerformance)
        } else if self.resolution_scale_performance.is_pressed() {
            Some(model::ResolutionScaleButton::Performance)
        } else if self.resolution_scale_balanced.is_pressed() {
            Some(model::ResolutionScaleButton::Balanced)
        } else if self.resolution_scale_quality.is_pressed() {
            Some(model::ResolutionScaleButton::Quality)
        } else if self.resolution_scale_ultra_quality.is_pressed() {
            Some(model::ResolutionScaleButton::UltraQuality)
        } else if self.resolution_scale_native.is_pressed() {
            Some(model::ResolutionScaleButton::Native)
        } else {
            None
        }
    }

    fn apply_scale_filter(&self) -> Option<model::ScaleFilterButton> {
        if self.scale_filter_nearest.is_pressed() {
            Some(model::ScaleFilterButton::Nearest)
        } else if self.scale_filter_bilinear.is_pressed() {
            Some(model::ScaleFilterButton::Bilinear)
        } else if self.scale_filter_fsr1.is_pressed() {
            Some(model::ScaleFilterButton::Fsr1)
        } else if self.scale_filter_fsr2.is_pressed() {
            Some(model::ScaleFilterButton::Fsr2)
        } else if self.scale_filter_metalfx_spatial.is_pressed() {
            Some(model::ScaleFilterButton::MetalFxSpatial)
        } else if self.scale_filter_metalfx_temporal.is_pressed() {
            Some(model::ScaleFilterButton::MetalFxTemporal)
        } else {
            None
        }
    }

    fn apply_gi_type(&self) -> Option<model::GiTypeButton> {
        if self.gi_lightmapgi.is_pressed() {
            Some(model::GiTypeButton::LightmapGi)
        } else if self.gi_voxelgi.is_pressed() {
            Some(model::GiTypeButton::VoxelGi)
        } else if self.gi_sdfgi.is_pressed() {
            Some(model::GiTypeButton::Sdfgi)
        } else {
            None
        }
    }

    fn apply_gi_quality(&self) -> Option<model::GiQualityButton> {
        if self.gi_disabled.is_pressed() {
            Some(model::GiQualityButton::Disabled)
        } else if self.gi_low.is_pressed() {
            Some(model::GiQualityButton::Low)
        } else if self.gi_high.is_pressed() {
            Some(model::GiQualityButton::High)
        } else {
            None
        }
    }

    fn apply_msaa(&self) -> Option<model::MsaaButton> {
        if self.msaa_disabled.is_pressed() {
            Some(model::MsaaButton::Disabled)
        } else if self.msaa_2x.is_pressed() {
            Some(model::MsaaButton::Msaa2x)
        } else if self.msaa_4x.is_pressed() {
            Some(model::MsaaButton::Msaa4x)
        } else if self.msaa_8x.is_pressed() {
            Some(model::MsaaButton::Msaa8x)
        } else {
            None
        }
    }

    fn apply_screen_space_aa(&self) -> Option<model::ScreenSpaceAaButton> {
        if self.screen_space_aa_disabled.is_pressed() {
            Some(model::ScreenSpaceAaButton::Disabled)
        } else if self.screen_space_aa_fxaa.is_pressed() {
            Some(model::ScreenSpaceAaButton::Fxaa)
        } else if self.screen_space_aa_smaa.is_pressed() {
            Some(model::ScreenSpaceAaButton::Smaa)
        } else {
            None
        }
    }

    fn apply_ssao_quality(&self) -> Option<model::SsaoQualityButton> {
        if self.ssao_disabled.is_pressed() {
            Some(model::SsaoQualityButton::Disabled)
        } else if self.ssao_medium.is_pressed() {
            Some(model::SsaoQualityButton::Medium)
        } else if self.ssao_high.is_pressed() {
            Some(model::SsaoQualityButton::High)
        } else {
            None
        }
    }

    fn apply_ssil_quality(&self) -> Option<model::SsilQualityButton> {
        if self.ssil_disabled.is_pressed() {
            Some(model::SsilQualityButton::Disabled)
        } else if self.ssil_medium.is_pressed() {
            Some(model::SsilQualityButton::Medium)
        } else if self.ssil_high.is_pressed() {
            Some(model::SsilQualityButton::High)
        } else {
            None
        }
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
