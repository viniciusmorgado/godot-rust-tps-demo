use godot::builtin::VariantType;
use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    ConfigFile, DisplayServer, Engine, Environment, INode, InputEvent, Light3D, Node, RenderingServer, Window,
};
use godot::prelude::*;

mod graphics;
// Re-exported for the 5 consumers to name as `settings::GiType` etc. Unused within this crate
// until Phase 3 (User Story 2, tasks T019-T023) migrates them off dynamic access — allowed here
// because that migration is explicitly a later, separate commit (research.md R7), not a gap.
#[allow(unused_imports)]
pub use graphics::{GiQuality, GiType, GraphicsSettings, ScaleFilter, SsaoQuality, SsilQuality};
use graphics::{ApplyPlan, WireValue, plan};

const CONFIG_FILE_PATH: &str = "user://settings.ini";

/// `(section, key)` for each of the 15 wire slots, in the fixed `v1`-identical order that
/// `GraphicsSettings::to_wire`/`from_wire` also use (research.md R3).
const WIRE_KEYS: [(&str, &str); 15] = [
    ("video", "display_mode"),
    ("video", "vsync"),
    ("video", "max_fps"),
    ("video", "resolution_scale"),
    ("video", "scale_filter"),
    ("rendering", "taa"),
    ("rendering", "msaa"),
    ("rendering", "screen_space_aa"),
    ("rendering", "shadow_mapping"),
    ("rendering", "gi_type"),
    ("rendering", "gi_quality"),
    ("rendering", "ssao_quality"),
    ("rendering", "ssil_quality"),
    ("rendering", "bloom"),
    ("rendering", "volumetric_fog"),
];

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Settings {
    base: Base<Node>,

    // MetalFX is only supported when using the Metal rendering driver. Private: only
    // `GraphicsSettings::default_for`/`from_wire` need it, never exposed to the engine.
    metalfx_supported: bool,

    // TRANSITIONAL (US1): kept `#[var]` and read/written directly by `graphics()`/
    // `set_graphics()`/`apply_graphics_settings`/`save_settings` at each call, exactly as `v1`
    // did, because the 5 consumers have not migrated off dynamic access yet. Loses `#[var]` and
    // becomes the private save/load I/O buffer only in the last commit of User Story 2
    // (research.md R7).
    #[var]
    config_file: Gd<ConfigFile>,
}

#[godot_api]
impl INode for Settings {
    fn init(base: Base<Node>) -> Self {
        let metalfx_supported = RenderingServer::singleton().get_current_rendering_driver_name() == "metal";
        Self {
            base,
            metalfx_supported,
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
    #[func]
    fn load_settings(&mut self) {
        self.config_file.load(CONFIG_FILE_PATH);
        // Initialize defaults for values not found in the existing configuration file, so we
        // don't have to specify them every time we use `ConfigFile.get_value()` — same outcome
        // as `v1`, now derived from the single typed source of truth instead of a duplicated
        // literal dictionary.
        let defaults = GraphicsSettings::default_for(self.metalfx_supported).to_wire();
        for ((section, key), value) in WIRE_KEYS.into_iter().zip(defaults) {
            if !self.config_file.has_section_key(section, key) {
                self.config_file.set_value(section, key, &wire_to_variant(value));
            }
        }
    }

    #[func]
    pub fn save_settings(&mut self) {
        self.config_file.save(CONFIG_FILE_PATH);
    }

    #[func]
    pub fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>) {
        let settings = self.graphics();
        let plan = plan(&settings);
        Self::apply(&plan, window, environment, scene_root);
    }

    /// Typed read. TRANSITIONAL (US1): re-parses `config_file` at every call, exactly like
    /// `apply_graphics_settings` already must — becomes a plain field copy once `ready` parses
    /// the model exactly once (last commit of US2, research.md R7).
    pub fn graphics(&self) -> GraphicsSettings {
        let present = read_wire(&self.config_file);
        let (settings, malformed) = GraphicsSettings::from_wire(present, self.metalfx_supported);
        warn_malformed(&malformed);
        settings
    }

    /// Typed write. TRANSITIONAL (US1): writes straight into `config_file`, so the still-dynamic
    /// `apply_graphics_settings`/`save_settings` (which read `config_file`) observe the change.
    /// Unused until `menu.rs`'s Apply handler calls it (Phase 3, T023) — allowed here for the
    /// same reason as the re-exports above.
    #[allow(dead_code)]
    pub fn set_graphics(&mut self, graphics: GraphicsSettings) {
        write_wire(&mut self.config_file, graphics.to_wire());
    }

    /// Glue: pushes an `ApplyPlan` to the engine, including the typed `Light3D` shadow walk
    /// (research.md R5) that replaces `v1`'s dynamic `propagate_call("set", ["shadow_enabled",
    /// false])`.
    fn apply(plan: &ApplyPlan, mut window: Gd<Window>, mut environment: Gd<Environment>, scene_root: Gd<Node>) {
        window.set_mode(plan.window_mode);
        DisplayServer::singleton().window_set_vsync_mode(plan.vsync_mode);
        Engine::singleton().set_max_fps(plan.max_fps);
        window.set_scaling_3d_scale(plan.scaling_3d_scale);
        window.set_scaling_3d_mode(plan.scaling_3d_mode);
        window.set_use_taa(plan.use_taa);
        window.set_msaa_3d(plan.msaa_3d);
        window.set_screen_space_aa(plan.screen_space_aa);

        if plan.disable_shadows {
            // Disable shadows for all lights present during level load, reducing the number of
            // draw calls significantly.
            // FIXME: In the main menu, shadows aren't enabled again after enabling shadows
            // if they were previously disabled. We can't enable shadows on all lights
            // unconditionally, as this would negatively affect the level's performance.
            disable_shadows_recursive(&scene_root);
        }

        environment.set_ssao_enabled(plan.ssao.enabled);
        if plan.ssao.enabled {
            RenderingServer::singleton()
                .environment_set_ssao_quality(plan.ssao.quality, plan.ssao.half_size, 0.5, 2, 50.0, 300.0);
        }

        environment.set_ssil_enabled(plan.ssil.enabled);
        if plan.ssil.enabled {
            RenderingServer::singleton()
                .environment_set_ssil_quality(plan.ssil.quality, plan.ssil.half_size, 0.5, 2, 50.0, 300.0);
        }

        environment.set_glow_enabled(plan.glow_enabled);
        environment.set_volumetric_fog_enabled(plan.volumetric_fog_enabled);
    }
}

/// Recursive typed equivalent of `propagate_call("set", ["shadow_enabled", false])` — walks
/// every descendant INCLUDING internal children (`Node::propagate_call` does too, so this must
/// match for strict equivalence, research.md R5), disabling `Light3D::shadow_enabled` wherever
/// found; a no-op on any other node type.
fn disable_shadows_recursive(node: &Gd<Node>) {
    if let Ok(mut light) = node.clone().try_cast::<Light3D>() {
        light.set_shadow(false);
    }
    for child in node.get_children_ex().include_internal(true).done().iter_shared() {
        disable_shadows_recursive(&child);
    }
}

/// Reads the 15 wire slots from `config_file`, engine-free from here on (`Variant` never
/// crosses into `settings/graphics.rs`).
fn read_wire(config_file: &Gd<ConfigFile>) -> [Option<WireValue>; 15] {
    let mut present = [None; 15];
    for (i, (section, key)) in WIRE_KEYS.into_iter().enumerate() {
        if config_file.has_section_key(section, key) {
            present[i] = variant_to_wire(&config_file.get_value(section, key));
        }
    }
    present
}

/// Writes the 15 wire values into `config_file`, preserving each key's Variant type. Only
/// caller is `set_graphics` (see its doc comment for why it's `#[allow(dead_code)]` for now).
#[allow(dead_code)]
fn write_wire(config_file: &mut Gd<ConfigFile>, wire: [WireValue; 15]) {
    for ((section, key), value) in WIRE_KEYS.into_iter().zip(wire) {
        config_file.set_value(section, key, &wire_to_variant(value));
    }
}

fn variant_to_wire(value: &Variant) -> Option<WireValue> {
    match value.get_type() {
        VariantType::INT => Some(WireValue::Int(value.to::<i64>())),
        VariantType::FLOAT => Some(WireValue::Real(value.to::<f64>())),
        VariantType::BOOL => Some(WireValue::Bool(value.to::<bool>())),
        _ => None,
    }
}

fn wire_to_variant(value: WireValue) -> Variant {
    match value {
        WireValue::Int(v) => v.to_variant(),
        WireValue::Real(v) => v.to_variant(),
        WireValue::Bool(v) => v.to_variant(),
    }
}

fn warn_malformed(names: &[&str]) {
    for name in names {
        godot_warn!(
            "Settings: 'user://settings.ini' has an out-of-range value for '{name}'; using the default instead."
        );
    }
}
