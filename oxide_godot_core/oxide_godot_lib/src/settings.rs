use godot::builtin::VariantType;
use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    ConfigFile, DisplayServer, Engine, Environment, INode, InputEvent, Light3D, Node, RenderingServer, Window,
};
use godot::prelude::*;

mod graphics;
// Re-exported so the 5 consumers can name these as `settings::GiType` etc.
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

    // The single in-memory source of truth, parsed once at `ready` from `config_file`.
    graphics: GraphicsSettings,

    // Private I/O buffer only: loaded once at `ready`, reused (not recreated) by `save_settings`
    // so a hand-edited file's unknown sections/keys and key order survive a save (research.md R7).
    config_file: Gd<ConfigFile>,
}

#[godot_api]
impl INode for Settings {
    fn init(base: Base<Node>) -> Self {
        let metalfx_supported = RenderingServer::singleton().get_current_rendering_driver_name() == "metal";
        Self {
            base,
            metalfx_supported,
            graphics: GraphicsSettings::default_for(metalfx_supported),
            config_file: ConfigFile::new_gd(),
        }
    }

    fn ready(&mut self) {
        self.config_file.load(CONFIG_FILE_PATH);
        let present = read_wire(&self.config_file);
        let (graphics, malformed) = GraphicsSettings::from_wire(present, self.metalfx_supported);
        warn_malformed(&malformed);
        self.graphics = graphics;
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

impl Settings {
    pub fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>) {
        let plan = plan(&self.graphics);
        Self::apply(&plan, window, environment, scene_root);
    }

    /// `save_settings` writes the model into the SAME loaded `config_file` object (never a fresh
    /// `ConfigFile`), so unknown sections/keys and the key order of a hand-edited file survive
    /// (research.md R7).
    pub fn save_settings(&mut self) {
        write_wire(&mut self.config_file, self.graphics.to_wire());
        self.config_file.save(CONFIG_FILE_PATH);
    }

    pub fn graphics(&self) -> GraphicsSettings {
        self.graphics
    }

    pub fn set_graphics(&mut self, graphics: GraphicsSettings) {
        self.graphics = graphics;
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
/// crosses into `settings/graphics.rs`). A key that is PRESENT but whose Variant type is none
/// of int/real/bool is not silently treated the same as an absent key: it warns here (once,
/// naming the key) and is left `None`, so `GraphicsSettings::from_wire` still defaults it —
/// same outcome as a malformed value, but the warning is specific to "unsupported type" rather
/// than "out of range" (FR-009).
fn read_wire(config_file: &Gd<ConfigFile>) -> [Option<WireValue>; 15] {
    let mut present = [None; 15];
    for (i, (section, key)) in WIRE_KEYS.into_iter().enumerate() {
        if config_file.has_section_key(section, key) {
            let value = config_file.get_value(section, key);
            match variant_to_wire(&value) {
                Some(wire) => present[i] = Some(wire),
                None => godot_warn!(
                    "Settings: 'user://settings.ini' has an unsupported value type for '{key}'; using the default instead."
                ),
            }
        }
    }
    present
}

/// Writes the 15 wire values into `config_file`, preserving each key's Variant type.
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
