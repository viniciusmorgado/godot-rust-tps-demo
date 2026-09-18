//! Pure model for the 15 graphics options persisted by `Settings` (parent module).
//!
//! Nothing in this file touches `Gd<T>`, an engine singleton, `Variant`, `GString` or
//! `StringName`. The engine enums used below (`WindowMode`, `VSyncMode`, `Msaa`,
//! `ScreenSpaceAa`, `EnvironmentSsaoQuality`, `EnvironmentSsilQuality`, `Scaling3DMode`) are
//! `#[repr(transparent)]` newtypes around an `i32` ordinal with no FFI behind their
//! construction/comparison (research.md R1) — they need no live engine, exactly like gdext's
//! math builtins.

use godot::classes::display_server::VSyncMode;
use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality};
use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa};
use godot::classes::window::Mode as WindowMode;
use godot::obj::EngineEnum;

/// A wire value read from (or to be written to) `user://settings.ini`, engine-free.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum WireValue {
    Int(i64),
    Real(f64),
    Bool(bool),
}

impl WireValue {
    fn as_int(self) -> Option<i64> {
        match self {
            WireValue::Int(v) => Some(v),
            _ => None,
        }
    }

    fn as_real(self) -> Option<f64> {
        match self {
            WireValue::Real(v) => Some(v),
            _ => None,
        }
    }

    fn as_bool(self) -> Option<bool> {
        match self {
            WireValue::Bool(v) => Some(v),
            _ => None,
        }
    }
}

/// Parses an engine enum from a wire int, restricted to its REAL enumerators (`values()`),
/// excluding any `MAX` sentinel that `try_from_ord` would otherwise accept as a bare ordinal
/// (research.md R1 — this is what makes an out-of-range or sentinel-only code "malformed").
fn parse_engine_enum<T: EngineEnum + PartialEq>(code: i64) -> Option<T> {
    let ord = i32::try_from(code).ok()?;
    T::try_from_ord(ord).filter(|value| T::values().contains(value))
}

/// Reads one wire slot: `None` (key absent) silently yields `default`; `Some(value)` that
/// fails to `parse` also yields `default`, but additionally records `name` as malformed
/// (data-model.md's `from_wire` semantics — the one deviation from `v1`, FR-009).
fn parse_field<T>(
    malformed: &mut Vec<&'static str>,
    name: &'static str,
    slot: Option<WireValue>,
    default: T,
    parse: impl FnOnce(WireValue) -> Option<T>,
) -> T {
    match slot {
        None => default,
        Some(value) => parse(value).unwrap_or_else(|| {
            malformed.push(name);
            default
        }),
    }
}

macro_rules! project_enum {
    ($name:ident { $($variant:ident = $code:expr),+ $(,)? }) => {
        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            fn to_wire(self) -> i64 {
                match self {
                    $(Self::$variant => $code),+
                }
            }

            fn from_wire(code: i64) -> Option<Self> {
                $(if code == $code { return Some(Self::$variant); })+
                None
            }
        }
    };
}

project_enum!(GiType {
    Sdfgi = 0,
    VoxelGi = 1,
    LightmapGi = 2,
});

project_enum!(GiQuality {
    Disabled = 0,
    Low = 1,
    High = 2,
});

// Wire codes tie directly to the engine's own SSAO/SSIL quality ordinals (research.md R3) so a
// future renumbering of those constants — unlikely, but this is "parse, don't validate" applied
// to the cross-check itself — cannot silently desync the wire format from the engine.
project_enum!(SsaoQuality {
    Disabled = -1,
    Medium = EnvironmentSsaoQuality::MEDIUM.ord() as i64,
    High = EnvironmentSsaoQuality::HIGH.ord() as i64,
});

project_enum!(SsilQuality {
    Disabled = -1,
    Medium = EnvironmentSsilQuality::MEDIUM.ord() as i64,
    High = EnvironmentSsilQuality::HIGH.ord() as i64,
});

/// Wire codes `0..=5`, matching Godot 4.7's real `SCALING_3D_MODE_*` constants. `Nearest` (`5`)
/// is the API gap: gdext 0.5.5's prebuilt bindings (Godot 4.6) only know that ordinal as the
/// meaningless `Scaling3DMode::MAX` sentinel (research.md R2).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ScaleFilter {
    Bilinear,
    Fsr1,
    Fsr2,
    MetalFxSpatial,
    MetalFxTemporal,
    Nearest,
}

impl ScaleFilter {
    pub fn to_engine(self) -> Scaling3DMode {
        match self {
            Self::Bilinear => Scaling3DMode::BILINEAR,
            Self::Fsr1 => Scaling3DMode::FSR,
            Self::Fsr2 => Scaling3DMode::FSR2,
            Self::MetalFxSpatial => Scaling3DMode::METALFX_SPATIAL,
            Self::MetalFxTemporal => Scaling3DMode::METALFX_TEMPORAL,
            // api-gap(godot-4.7): Scaling3DMode::NEAREST — absent from the gdext 0.5.5 prebuilt API (4.6); replace when the binding ships it
            Self::Nearest => Scaling3DMode::from_ord(5),
        }
    }

    fn to_wire(self) -> i64 {
        match self {
            Self::Bilinear => 0,
            Self::Fsr1 => 1,
            Self::Fsr2 => 2,
            Self::MetalFxSpatial => 3,
            Self::MetalFxTemporal => 4,
            Self::Nearest => 5,
        }
    }

    fn from_wire(code: i64) -> Option<Self> {
        match code {
            0 => Some(Self::Bilinear),
            1 => Some(Self::Fsr1),
            2 => Some(Self::Fsr2),
            3 => Some(Self::MetalFxSpatial),
            4 => Some(Self::MetalFxTemporal),
            5 => Some(Self::Nearest),
            _ => None,
        }
    }
}

/// The 15 graphics options, typed. Parsed once from `user://settings.ini` (see `settings.rs`);
/// every consumer reads a cheap `Copy` of this instead of raw `ConfigFile` lookups.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GraphicsSettings {
    pub display_mode: WindowMode,
    pub vsync: VSyncMode,
    pub max_fps: i32,
    pub resolution_scale: f64,
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
    pub fn default_for(metalfx_supported: bool) -> Self {
        Self {
            display_mode: WindowMode::EXCLUSIVE_FULLSCREEN,
            vsync: VSyncMode::DISABLED,
            max_fps: 0, // Unlimited
            resolution_scale: 1.0,
            scale_filter: if metalfx_supported {
                ScaleFilter::MetalFxTemporal
            } else {
                ScaleFilter::Fsr2
            },
            taa: false,
            msaa: Msaa::MSAA_8X,
            screen_space_aa: ScreenSpaceAa::SMAA,
            shadow_mapping: true,
            gi_type: GiType::Sdfgi,
            gi_quality: GiQuality::High,
            ssao_quality: SsaoQuality::High,
            ssil_quality: SsilQuality::High,
            bloom: true,
            volumetric_fog: true,
        }
    }

    /// Fixed order, `v1`-identical (research.md R3): `video/{display_mode, vsync, max_fps,
    /// resolution_scale, scale_filter}`, `rendering/{taa, msaa, screen_space_aa, shadow_mapping,
    /// gi_type, gi_quality, ssao_quality, ssil_quality, bloom, volumetric_fog}`.
    pub fn to_wire(self) -> [WireValue; 15] {
        [
            WireValue::Int(self.display_mode.ord() as i64),
            WireValue::Int(self.vsync.ord() as i64),
            WireValue::Int(self.max_fps as i64),
            WireValue::Real(self.resolution_scale),
            WireValue::Int(self.scale_filter.to_wire()),
            WireValue::Bool(self.taa),
            WireValue::Int(self.msaa.ord() as i64),
            WireValue::Int(self.screen_space_aa.ord() as i64),
            WireValue::Bool(self.shadow_mapping),
            WireValue::Int(self.gi_type.to_wire()),
            WireValue::Int(self.gi_quality.to_wire()),
            WireValue::Int(self.ssao_quality.to_wire()),
            WireValue::Int(self.ssil_quality.to_wire()),
            WireValue::Bool(self.bloom),
            WireValue::Bool(self.volumetric_fog),
        ]
    }

    /// `present[i]` is `None` when that key is absent from the file (silent default, matching
    /// `v1`'s merge) or `Some(value)` when it exists. A `Some` that fails to parse into the
    /// field's type also defaults, but its name is appended to the returned `Vec` — the caller
    /// (`settings.rs`) emits one `godot_warn!` per name (the one deviation from `v1`, FR-009).
    pub fn from_wire(present: [Option<WireValue>; 15], metalfx_supported: bool) -> (Self, Vec<&'static str>) {
        let default = Self::default_for(metalfx_supported);
        let mut malformed = Vec::new();

        let settings = Self {
            display_mode: parse_field(&mut malformed, "display_mode", present[0], default.display_mode, |v| {
                v.as_int().and_then(parse_engine_enum::<WindowMode>)
            }),
            vsync: parse_field(&mut malformed, "vsync", present[1], default.vsync, |v| {
                v.as_int().and_then(parse_engine_enum::<VSyncMode>)
            }),
            max_fps: parse_field(&mut malformed, "max_fps", present[2], default.max_fps, |v| {
                v.as_int().and_then(|i| i32::try_from(i).ok())
            }),
            resolution_scale: parse_field(&mut malformed, "resolution_scale", present[3], default.resolution_scale, |v| {
                v.as_real()
            }),
            scale_filter: parse_field(&mut malformed, "scale_filter", present[4], default.scale_filter, |v| {
                v.as_int().and_then(ScaleFilter::from_wire)
            }),
            taa: parse_field(&mut malformed, "taa", present[5], default.taa, |v| v.as_bool()),
            msaa: parse_field(&mut malformed, "msaa", present[6], default.msaa, |v| {
                v.as_int().and_then(parse_engine_enum::<Msaa>)
            }),
            screen_space_aa: parse_field(&mut malformed, "screen_space_aa", present[7], default.screen_space_aa, |v| {
                v.as_int().and_then(parse_engine_enum::<ScreenSpaceAa>)
            }),
            shadow_mapping: parse_field(&mut malformed, "shadow_mapping", present[8], default.shadow_mapping, |v| {
                v.as_bool()
            }),
            gi_type: parse_field(&mut malformed, "gi_type", present[9], default.gi_type, |v| {
                v.as_int().and_then(GiType::from_wire)
            }),
            gi_quality: parse_field(&mut malformed, "gi_quality", present[10], default.gi_quality, |v| {
                v.as_int().and_then(GiQuality::from_wire)
            }),
            ssao_quality: parse_field(&mut malformed, "ssao_quality", present[11], default.ssao_quality, |v| {
                v.as_int().and_then(SsaoQuality::from_wire)
            }),
            ssil_quality: parse_field(&mut malformed, "ssil_quality", present[12], default.ssil_quality, |v| {
                v.as_int().and_then(SsilQuality::from_wire)
            }),
            bloom: parse_field(&mut malformed, "bloom", present[13], default.bloom, |v| v.as_bool()),
            volumetric_fog: parse_field(&mut malformed, "volumetric_fog", present[14], default.volumetric_fog, |v| {
                v.as_bool()
            }),
        };

        (settings, malformed)
    }
}

/// One ambient-occlusion decision: whether it's on, at which engine quality, and `half_size`.
/// Generic because `RenderingServer`'s SSAO/SSIL setters take distinct engine enum types.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AoDecision<Q> {
    pub enabled: bool,
    pub quality: Q,
    pub half_size: bool,
}

/// What `apply()` (glue, in `settings.rs`) must push to the engine — the pure output of `plan()`.
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

/// Pure decision step — no engine call. Reproduces `v1`'s `apply_graphics_settings` exactly,
/// including both preserved SSAO/SSIL naming quirks (backlog #25) and the upstream bug fix on
/// the SSAO `-1` branch (research.md R4).
pub fn plan(settings: &GraphicsSettings) -> ApplyPlan {
    let ssao = match settings.ssao_quality {
        SsaoQuality::Disabled => AoDecision { enabled: false, quality: EnvironmentSsaoQuality::MEDIUM, half_size: false },
        // upstream bug fix: settings.gd used `if` instead of `elif` — "SSAO: Disabled" (-1) was re-enabled by the else
        SsaoQuality::Medium => AoDecision { enabled: true, quality: EnvironmentSsaoQuality::HIGH, half_size: false },
        SsaoQuality::High => AoDecision { enabled: true, quality: EnvironmentSsaoQuality::MEDIUM, half_size: true },
    };
    let ssil = match settings.ssil_quality {
        SsilQuality::Disabled => AoDecision { enabled: false, quality: EnvironmentSsilQuality::MEDIUM, half_size: false },
        SsilQuality::Medium => AoDecision { enabled: true, quality: EnvironmentSsilQuality::MEDIUM, half_size: false },
        SsilQuality::High => AoDecision { enabled: true, quality: EnvironmentSsilQuality::HIGH, half_size: true },
    };

    ApplyPlan {
        window_mode: settings.display_mode,
        vsync_mode: settings.vsync,
        max_fps: settings.max_fps,
        scaling_3d_scale: settings.resolution_scale as f32,
        scaling_3d_mode: settings.scale_filter.to_engine(),
        use_taa: settings.taa,
        msaa_3d: settings.msaa,
        screen_space_aa: settings.screen_space_aa,
        disable_shadows: !settings.shadow_mapping,
        ssao,
        ssil,
        glow_enabled: settings.bloom,
        volumetric_fog_enabled: settings.volumetric_fog,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_default_settings() -> GraphicsSettings {
        GraphicsSettings {
            display_mode: WindowMode::WINDOWED,
            vsync: VSyncMode::ADAPTIVE,
            max_fps: 60,
            resolution_scale: 1.0 / 1.7,
            scale_filter: ScaleFilter::Nearest,
            taa: true,
            msaa: Msaa::MSAA_4X,
            screen_space_aa: ScreenSpaceAa::SMAA,
            shadow_mapping: false,
            gi_type: GiType::LightmapGi,
            gi_quality: GiQuality::High,
            ssao_quality: SsaoQuality::High,
            ssil_quality: SsilQuality::Medium,
            bloom: false,
            volumetric_fog: false,
        }
    }

    fn all_present(wire: [WireValue; 15]) -> [Option<WireValue>; 15] {
        wire.map(Some)
    }

    #[test]
    fn round_trip_all_fields() {
        let original = non_default_settings();
        let (parsed, malformed) = GraphicsSettings::from_wire(all_present(original.to_wire()), false);
        assert!(malformed.is_empty());
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_defaults_metalfx_true() {
        let original = GraphicsSettings::default_for(true);
        let (parsed, malformed) = GraphicsSettings::from_wire(all_present(original.to_wire()), true);
        assert!(malformed.is_empty());
        assert_eq!(parsed, original);
    }

    #[test]
    fn default_for_metalfx_supported_true_uses_metalfx_temporal() {
        assert_eq!(GraphicsSettings::default_for(true).scale_filter, ScaleFilter::MetalFxTemporal);
    }

    #[test]
    fn default_for_metalfx_supported_false_uses_fsr2() {
        assert_eq!(GraphicsSettings::default_for(false).scale_filter, ScaleFilter::Fsr2);
    }

    #[test]
    fn from_wire_missing_keys_default_silently_with_no_warning() {
        let (parsed, malformed) = GraphicsSettings::from_wire([None; 15], false);
        assert!(malformed.is_empty());
        assert_eq!(parsed, GraphicsSettings::default_for(false));
    }

    #[test]
    fn from_wire_partial_merge_keeps_present_and_defaults_missing() {
        let mut present = [None; 15];
        present[5] = Some(WireValue::Bool(true)); // taa
        present[9] = Some(WireValue::Int(GiType::LightmapGi.to_wire())); // gi_type

        let (parsed, malformed) = GraphicsSettings::from_wire(present, false);

        assert!(malformed.is_empty());
        assert!(parsed.taa);
        assert_eq!(parsed.gi_type, GiType::LightmapGi);
        // everything else still the default
        assert_eq!(parsed.shadow_mapping, GraphicsSettings::default_for(false).shadow_mapping);
        assert_eq!(parsed.gi_quality, GraphicsSettings::default_for(false).gi_quality);
    }

    #[test]
    fn from_wire_out_of_range_enum_code_defaults_and_warns() {
        let mut present = [None; 15];
        present[9] = Some(WireValue::Int(7)); // gi_type has no variant for 7

        let (parsed, malformed) = GraphicsSettings::from_wire(present, false);

        assert_eq!(parsed.gi_type, GraphicsSettings::default_for(false).gi_type);
        assert_eq!(malformed, vec!["gi_type"]);
    }

    #[test]
    fn from_wire_engine_enum_max_sentinel_is_malformed_not_a_real_value() {
        // Msaa::MAX (ord 4) is accepted by `try_from_ord` but excluded from `values()` — it
        // must NOT be treated as a valid wire value (research.md R1's core decision).
        let mut present = [None; 15];
        present[6] = Some(WireValue::Int(Msaa::MAX.ord() as i64));

        let (parsed, malformed) = GraphicsSettings::from_wire(present, false);

        assert_eq!(parsed.msaa, GraphicsSettings::default_for(false).msaa);
        assert_eq!(malformed, vec!["msaa"]);
    }

    #[test]
    fn from_wire_wrong_wire_value_type_is_malformed() {
        let mut present = [None; 15];
        present[2] = Some(WireValue::Bool(true)); // max_fps expects an Int, not a Bool

        let (parsed, malformed) = GraphicsSettings::from_wire(present, false);

        assert_eq!(parsed.max_fps, GraphicsSettings::default_for(false).max_fps);
        assert_eq!(malformed, vec!["max_fps"]);
    }

    #[test]
    fn scale_filter_nearest_round_trips_through_the_api_gap_workaround() {
        assert_eq!(ScaleFilter::Nearest.to_wire(), 5);
        assert_eq!(ScaleFilter::from_wire(5), Some(ScaleFilter::Nearest));
        assert_eq!(ScaleFilter::Nearest.to_engine().ord(), 5);
    }

    #[test]
    fn plan_ssao_disabled_turns_off_and_stays_off() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssao_quality = SsaoQuality::Disabled;
        assert_eq!(plan(&settings).ssao, AoDecision { enabled: false, quality: EnvironmentSsaoQuality::MEDIUM, half_size: false });
    }

    #[test]
    fn plan_ssao_medium_applies_high_quality_without_half_size() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssao_quality = SsaoQuality::Medium;
        assert_eq!(plan(&settings).ssao, AoDecision { enabled: true, quality: EnvironmentSsaoQuality::HIGH, half_size: false });
    }

    #[test]
    fn plan_ssao_high_applies_medium_quality_with_half_size() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssao_quality = SsaoQuality::High;
        assert_eq!(plan(&settings).ssao, AoDecision { enabled: true, quality: EnvironmentSsaoQuality::MEDIUM, half_size: true });
    }

    #[test]
    fn plan_ssil_disabled_turns_off() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssil_quality = SsilQuality::Disabled;
        assert_eq!(plan(&settings).ssil, AoDecision { enabled: false, quality: EnvironmentSsilQuality::MEDIUM, half_size: false });
    }

    #[test]
    fn plan_ssil_medium_applies_medium_quality_without_half_size() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssil_quality = SsilQuality::Medium;
        assert_eq!(plan(&settings).ssil, AoDecision { enabled: true, quality: EnvironmentSsilQuality::MEDIUM, half_size: false });
    }

    #[test]
    fn plan_ssil_high_applies_high_quality_with_half_size() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.ssil_quality = SsilQuality::High;
        assert_eq!(plan(&settings).ssil, AoDecision { enabled: true, quality: EnvironmentSsilQuality::HIGH, half_size: true });
    }

    #[test]
    fn plan_disable_shadows_mirrors_shadow_mapping_flag() {
        let mut settings = GraphicsSettings::default_for(false);
        settings.shadow_mapping = false;
        assert!(plan(&settings).disable_shadows);
        settings.shadow_mapping = true;
        assert!(!plan(&settings).disable_shadows);
    }
}
