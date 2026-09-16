# Contract: `Settings` and the pure `graphics` model

Signatures only (no bodies) — the contract tasks implement against. Types and wire codes are
pinned in `../data-model.md`; design rationale is in `../research.md`.

## `oxide_godot_lib/src/settings/graphics.rs` (pure, no `Gd<T>`/engine singleton, unit-tested)

```rust
// Project enums (Copy, Clone, Eq, PartialEq, Debug)
pub enum GiType { Sdfgi, VoxelGi, LightmapGi }
pub enum GiQuality { Disabled, Low, High }
pub enum SsaoQuality { Disabled, Medium, High }
pub enum SsilQuality { Disabled, Medium, High }
pub enum ScaleFilter { Nearest, Bilinear, Fsr1, MetalFxSpatial, Fsr2, MetalFxTemporal }

impl ScaleFilter {
    pub fn to_engine(self) -> Scaling3DMode;          // owns the NEAREST api-gap
    pub fn from_wire(code: i64) -> Option<Self>;      // 0..=5, independent of the engine's own range
}

pub enum WireValue { Int(i64), Real(f64), Bool(bool) }

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GraphicsSettings {
    pub display_mode: WindowMode,
    pub vsync: VSyncMode,
    pub max_fps: i32,
    pub resolution_scale: f64,   // f64 end to end (SC-003 byte-identical); `as f32` only in apply()
    pub scale_filter: ScaleFilter,
    pub taa: bool,
    pub msaa: Msaa,
    pub screen_space_aa: ScreenSpaceAa,
    pub shadow_mapping: bool,
    pub gi_type: GiType,
    pub gi_quality: GiQuality,
    pub ssao_quality: SsaoQuality,
    pub ssil_quality: SsilQuality,
    pub bloom: bool,
    pub volumetric_fog: bool,
}

impl GraphicsSettings {
    pub fn default_for(metalfx_supported: bool) -> Self;
    pub fn to_wire(&self) -> [WireValue; 15];
    /// Returns the parsed model and the names of fields whose *present* wire value was
    /// malformed (fell back to default + warning) — as opposed to simply absent (silent default).
    pub fn from_wire(present: [Option<WireValue>; 15], metalfx_supported: bool) -> (Self, Vec<&'static str>);
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AoDecision<Q> { pub enabled: bool, pub quality: Q, pub half_size: bool }

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ApplyPlan {
    pub window_mode: WindowMode,
    pub vsync_mode: VSyncMode,
    pub max_fps: i32,
    pub scaling_3d_scale: f32,
    pub scaling_3d_mode: Scaling3DMode,
    pub use_taa: bool,
    pub msaa_3d: Msaa,
    pub screen_space_aa: ScreenSpaceAa,
    pub disable_shadows: bool,
    pub ssao: AoDecision<EnvironmentSsaoQuality>,
    pub ssil: AoDecision<EnvironmentSsilQuality>,
    pub glow_enabled: bool,
    pub volumetric_fog_enabled: bool,
}

/// Pure decision step — no engine call. Reproduces `v1`'s SSAO/SSIL quirks and upstream bug fix.
pub fn plan(settings: &GraphicsSettings) -> ApplyPlan;
```

**Required unit tests** (`#[cfg(test)] mod tests` in the same file, `cargo test`, ≥ 8):
round-trip `to_wire`/`from_wire` per field; `default_for(true)` and `default_for(false)`; merge of
a partial wire array (some `None` slots) keeps present values and defaults the rest; a malformed
`Some(WireValue)` per enum-backed field defaults + is named in the returned `Vec`; `plan()` for
all 3 `SsaoQuality` states and all 3 `SsilQuality` states (6 cases) matching the pinned values in
`data-model.md`.

## `oxide_godot_lib/src/settings.rs` (glue)

```rust
const CONFIG_FILE_PATH: &str = "user://settings.ini";
mod graphics;
pub use graphics::{GraphicsSettings, GiType, GiQuality, SsaoQuality, SsilQuality, ScaleFilter};

#[derive(GodotClass)]
#[class(base = Node)]
pub struct Settings {
    base: Base<Node>,
    graphics: GraphicsSettings,   // final shape — see transitional shape below for US1
    config_file: Gd<ConfigFile>,  // private I/O buffer (no #[var]): loaded once at ready, reused by save_settings
}

impl INode for Settings {
    fn init(base: Base<Node>) -> Self;   // metalfx_supported computed here, fed into default_for
    fn ready(&mut self);                  // final shape: parses `graphics` exactly once
    fn input(&mut self, event: Gd<InputEvent>); // unchanged toggle_fullscreen
}

impl Settings {
    pub fn graphics(&self) -> GraphicsSettings;
    pub fn set_graphics(&mut self, graphics: GraphicsSettings);
    pub fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>);
    pub fn save_settings(&mut self);

    /// Glue: pushes an `ApplyPlan` to the engine, including the typed `Light3D` shadow walk (R5):
    /// recursive over `get_children_ex().include_internal(true)` — `propagate_call` iterates
    /// internal children too, so the walk must as well for strict equivalence.
    fn apply(plan: &ApplyPlan, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>);
}
```

### Transitional shape, US1 only (see `research.md` R7 for the exact commit boundaries)

`#[var] config_file: Gd<ConfigFile>` stays a field; `#[func] load_settings`, `#[func]
save_settings`, `#[func] apply_graphics_settings` stay exposed by name; `apply_graphics_settings`
and `save_settings` internally do `config_file → wire array → GraphicsSettings::from_wire → plan
→ apply` on every call (parse-at-boundary) instead of reading a stored `graphics` field. This
shape is deleted in the last commit of US2, never present after this milestone closes.

## Consumer access pattern (all 5 consumers, from US2 onward)

```rust
// struct field, each of the 5 consumer structs (they all use the derived `#[class(init, ...)]`,
// so the closure goes in the field attribute, not in a hand-written `init()`):
#[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
settings: OnReady<Gd<Settings>>,

// read, anywhere from `ready()` onward (never in `init()`):
let graphics = self.settings.bind().graphics();

// write (menu.rs's Apply handler only):
self.settings.bind_mut().set_graphics(graphics);
self.settings.bind_mut().apply_graphics_settings(window, environment, scene_root);
self.settings.bind_mut().save_settings();
```
