//! Pure model for `Menu`'s declarative option table (v1: `menu.rs:298-560`'s ~250 lines of
//! `if`/`else if`) and the loading-status poll.
//!
//! One pure `_button` (show)/`_value` (apply) pair per settings row, each reproducing its
//! v1 quirk verbatim (research.md R6). Nothing here touches `Gd<T>`, an engine singleton,
//! `Variant`, `GString` or `StringName`; the engine enums used (`WindowMode`, `VSyncMode`,
//! `Msaa`, `ScreenSpaceAa`, `ScaleFilter`, `GiType`, `GiQuality`, `SsaoQuality`,
//! `SsilQuality`, `ThreadLoadStatus`) are FFI-free value types (1.4.1).
//!
//! `godot::global::is_equal_approx` is ENGINE-BACKED (research.md R6, dispatches through
//! `sys::utility_function_table()`) and MUST NOT be called from here — `is_equal_approx`
//! below is a pure reimplementation of Godot's documented formula, empirically cross-checked
//! against the real engine function for the six presets + near-boundary values at
//! implementation time (T030).

use godot::classes::display_server::VSyncMode;
use godot::classes::resource_loader::ThreadLoadStatus;
use godot::classes::viewport::{Msaa, ScreenSpaceAa};
use godot::classes::window::Mode as WindowMode;

use crate::settings::{GiQuality, GiType, ScaleFilter, SsaoQuality, SsilQuality};

const CMP_EPSILON: f64 = 1e-5;

/// Godot's documented `is_equal_approx` semantics ("within a small internal epsilon... which
/// scales with the magnitude of the numbers") — reimplemented pure since the real function is
/// engine-backed. Empirically cross-checked (T030) against `godot::global::is_equal_approx`
/// for the six `resolution_scale` presets and near-boundary values: no disagreement found.
pub(crate) fn is_equal_approx(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let tolerance = (CMP_EPSILON * a.abs()).max(CMP_EPSILON);
    (a - b).abs() < tolerance
}

// --- display_mode (v1: menu.rs:306-310 show, :430-436 apply) ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DisplayModeButton {
    Windowed,
    Fullscreen,
    ExclusiveFullscreen,
}

pub fn display_mode_button(mode: WindowMode) -> DisplayModeButton {
    match mode {
        WindowMode::WINDOWED | WindowMode::MAXIMIZED => DisplayModeButton::Windowed,
        WindowMode::FULLSCREEN => DisplayModeButton::Fullscreen,
        _ => DisplayModeButton::ExclusiveFullscreen,
    }
}

pub fn display_mode_value(button: DisplayModeButton) -> WindowMode {
    match button {
        // MAXIMIZED collapses to WINDOWED on Apply -- v1's own quirk, preserved verbatim.
        DisplayModeButton::Windowed => WindowMode::WINDOWED,
        DisplayModeButton::Fullscreen => WindowMode::FULLSCREEN,
        DisplayModeButton::ExclusiveFullscreen => WindowMode::EXCLUSIVE_FULLSCREEN,
    }
}

// --- vsync (menu.rs:312-317 show, :438-446 apply) — exhaustive both ways ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum VsyncButton {
    Disabled,
    Enabled,
    Adaptive,
    Mailbox,
}

pub fn vsync_button(v: VSyncMode) -> VsyncButton {
    match v {
        VSyncMode::DISABLED => VsyncButton::Disabled,
        VSyncMode::ENABLED => VsyncButton::Enabled,
        VSyncMode::ADAPTIVE => VsyncButton::Adaptive,
        _ => VsyncButton::Mailbox,
    }
}

pub fn vsync_value(b: VsyncButton) -> VSyncMode {
    match b {
        VsyncButton::Disabled => VSyncMode::DISABLED,
        VsyncButton::Enabled => VSyncMode::ENABLED,
        VsyncButton::Adaptive => VSyncMode::ADAPTIVE,
        VsyncButton::Mailbox => VSyncMode::MAILBOX,
    }
}

// --- max_fps (menu.rs:319-328 show, :448-464 apply) — 8 literal values, exhaustive ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MaxFpsButton {
    Fps30,
    Fps40,
    Fps60,
    Fps72,
    Fps90,
    Fps120,
    Fps144,
    Unlimited,
}

pub fn max_fps_button(fps: i32) -> MaxFpsButton {
    match fps {
        30 => MaxFpsButton::Fps30,
        40 => MaxFpsButton::Fps40,
        60 => MaxFpsButton::Fps60,
        72 => MaxFpsButton::Fps72,
        90 => MaxFpsButton::Fps90,
        120 => MaxFpsButton::Fps120,
        144 => MaxFpsButton::Fps144,
        _ => MaxFpsButton::Unlimited,
    }
}

pub fn max_fps_value(b: MaxFpsButton) -> i32 {
    match b {
        MaxFpsButton::Fps30 => 30,
        MaxFpsButton::Fps40 => 40,
        MaxFpsButton::Fps60 => 60,
        MaxFpsButton::Fps72 => 72,
        MaxFpsButton::Fps90 => 90,
        MaxFpsButton::Fps120 => 120,
        MaxFpsButton::Fps144 => 144,
        MaxFpsButton::Unlimited => 0,
    }
}

// --- resolution_scale (menu.rs:330-342 show, :466-478 apply) — epsilon fallback (R6) ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ResolutionScaleButton {
    UltraPerformance,
    Performance,
    Balanced,
    Quality,
    UltraQuality,
    Native,
}

pub fn resolution_scale_button(scale: f64) -> ResolutionScaleButton {
    if is_equal_approx(scale, 1.0 / 3.0) {
        ResolutionScaleButton::UltraPerformance
    } else if is_equal_approx(scale, 1.0 / 2.0) {
        ResolutionScaleButton::Performance
    } else if is_equal_approx(scale, 1.0 / 1.7) {
        ResolutionScaleButton::Balanced
    } else if is_equal_approx(scale, 1.0 / 1.5) {
        ResolutionScaleButton::Quality
    } else if is_equal_approx(scale, 1.0 / 1.3) {
        ResolutionScaleButton::UltraQuality
    } else {
        // Fallback-to-Native on an unlisted value (e.g. a hand-edited settings.ini) even
        // though the actual value isn't 1.0 -- v1's own quirk, preserved verbatim.
        ResolutionScaleButton::Native
    }
}

pub fn resolution_scale_value(b: ResolutionScaleButton) -> f64 {
    match b {
        ResolutionScaleButton::UltraPerformance => 1.0 / 3.0,
        ResolutionScaleButton::Performance => 1.0 / 2.0,
        ResolutionScaleButton::Balanced => 1.0 / 1.7,
        ResolutionScaleButton::Quality => 1.0 / 1.5,
        ResolutionScaleButton::UltraQuality => 1.0 / 1.3,
        ResolutionScaleButton::Native => 1.0,
    }
}

// --- scale_filter (menu.rs:344-351 show, :480-492 apply) — exhaustive both ways ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ScaleFilterButton {
    Nearest,
    Bilinear,
    Fsr1,
    MetalFxSpatial,
    Fsr2,
    MetalFxTemporal,
}

pub fn scale_filter_button(f: ScaleFilter) -> ScaleFilterButton {
    match f {
        ScaleFilter::Nearest => ScaleFilterButton::Nearest,
        ScaleFilter::Bilinear => ScaleFilterButton::Bilinear,
        ScaleFilter::Fsr1 => ScaleFilterButton::Fsr1,
        ScaleFilter::MetalFxSpatial => ScaleFilterButton::MetalFxSpatial,
        ScaleFilter::Fsr2 => ScaleFilterButton::Fsr2,
        ScaleFilter::MetalFxTemporal => ScaleFilterButton::MetalFxTemporal,
    }
}

pub fn scale_filter_value(b: ScaleFilterButton) -> ScaleFilter {
    match b {
        ScaleFilterButton::Nearest => ScaleFilter::Nearest,
        ScaleFilterButton::Bilinear => ScaleFilter::Bilinear,
        ScaleFilterButton::Fsr1 => ScaleFilter::Fsr1,
        ScaleFilterButton::MetalFxSpatial => ScaleFilter::MetalFxSpatial,
        ScaleFilterButton::Fsr2 => ScaleFilter::Fsr2,
        ScaleFilterButton::MetalFxTemporal => ScaleFilter::MetalFxTemporal,
    }
}

// --- gi_type / gi_quality (menu.rs:353-363 show, :494-508 apply) — exhaustive both ways ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum GiTypeButton {
    LightmapGi,
    VoxelGi,
    Sdfgi,
}

pub fn gi_type_button(t: GiType) -> GiTypeButton {
    match t {
        GiType::LightmapGi => GiTypeButton::LightmapGi,
        GiType::VoxelGi => GiTypeButton::VoxelGi,
        GiType::Sdfgi => GiTypeButton::Sdfgi,
    }
}

pub fn gi_type_value(b: GiTypeButton) -> GiType {
    match b {
        GiTypeButton::LightmapGi => GiType::LightmapGi,
        GiTypeButton::VoxelGi => GiType::VoxelGi,
        GiTypeButton::Sdfgi => GiType::Sdfgi,
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum GiQualityButton {
    Disabled,
    Low,
    High,
}

pub fn gi_quality_button(q: GiQuality) -> GiQualityButton {
    match q {
        GiQuality::Disabled => GiQualityButton::Disabled,
        GiQuality::Low => GiQualityButton::Low,
        GiQuality::High => GiQualityButton::High,
    }
}

pub fn gi_quality_value(b: GiQualityButton) -> GiQuality {
    match b {
        GiQualityButton::Disabled => GiQuality::Disabled,
        GiQualityButton::Low => GiQuality::Low,
        GiQualityButton::High => GiQuality::High,
    }
}

// --- msaa / screen_space_aa (menu.rs:371-384 show, :512-528 apply) — NO catch-all on show ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MsaaButton {
    Disabled,
    Msaa2x,
    Msaa4x,
    Msaa8x,
}

/// `None` on an unlisted value -- v1's own gap (`:371-377` has no `_ =>` arm), nothing gets
/// pressed; preserved verbatim, not "fixed".
pub fn msaa_button(m: Msaa) -> Option<MsaaButton> {
    match m {
        Msaa::DISABLED => Some(MsaaButton::Disabled),
        Msaa::MSAA_2X => Some(MsaaButton::Msaa2x),
        Msaa::MSAA_4X => Some(MsaaButton::Msaa4x),
        Msaa::MSAA_8X => Some(MsaaButton::Msaa8x),
        _ => None,
    }
}

pub fn msaa_value(b: MsaaButton) -> Msaa {
    match b {
        MsaaButton::Disabled => Msaa::DISABLED,
        MsaaButton::Msaa2x => Msaa::MSAA_2X,
        MsaaButton::Msaa4x => Msaa::MSAA_4X,
        MsaaButton::Msaa8x => Msaa::MSAA_8X,
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ScreenSpaceAaButton {
    Disabled,
    Fxaa,
    Smaa,
}

pub fn screen_space_aa_button(a: ScreenSpaceAa) -> Option<ScreenSpaceAaButton> {
    match a {
        ScreenSpaceAa::DISABLED => Some(ScreenSpaceAaButton::Disabled),
        ScreenSpaceAa::FXAA => Some(ScreenSpaceAaButton::Fxaa),
        ScreenSpaceAa::SMAA => Some(ScreenSpaceAaButton::Smaa),
        _ => None,
    }
}

pub fn screen_space_aa_value(b: ScreenSpaceAaButton) -> ScreenSpaceAa {
    match b {
        ScreenSpaceAaButton::Disabled => ScreenSpaceAa::DISABLED,
        ScreenSpaceAaButton::Fxaa => ScreenSpaceAa::FXAA,
        ScreenSpaceAaButton::Smaa => ScreenSpaceAa::SMAA,
    }
}

// --- ssao_quality / ssil_quality (menu.rs:392-402 show, :532-546 apply) — exhaustive both ways ---
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SsaoQualityButton {
    Disabled,
    Medium,
    High,
}

pub fn ssao_quality_button(q: SsaoQuality) -> SsaoQualityButton {
    match q {
        SsaoQuality::Disabled => SsaoQualityButton::Disabled,
        SsaoQuality::Medium => SsaoQualityButton::Medium,
        SsaoQuality::High => SsaoQualityButton::High,
    }
}

pub fn ssao_quality_value(b: SsaoQualityButton) -> SsaoQuality {
    match b {
        SsaoQualityButton::Disabled => SsaoQuality::Disabled,
        SsaoQualityButton::Medium => SsaoQuality::Medium,
        SsaoQualityButton::High => SsaoQuality::High,
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SsilQualityButton {
    Disabled,
    Medium,
    High,
}

pub fn ssil_quality_button(q: SsilQuality) -> SsilQualityButton {
    match q {
        SsilQuality::Disabled => SsilQualityButton::Disabled,
        SsilQuality::Medium => SsilQualityButton::Medium,
        SsilQuality::High => SsilQualityButton::High,
    }
}

pub fn ssil_quality_value(b: SsilQualityButton) -> SsilQuality {
    match b {
        SsilQualityButton::Disabled => SsilQuality::Disabled,
        SsilQualityButton::Medium => SsilQuality::Medium,
        SsilQualityButton::High => SsilQuality::High,
    }
}

// --- loading status poll (v1: menu.rs:246-256's three-way branch) ---
pub enum LoadingCmd {
    UpdateProgress(f64),
    Finished,
    Failed,
}

pub fn loading_step(status: ThreadLoadStatus, progress: f64) -> LoadingCmd {
    match status {
        ThreadLoadStatus::IN_PROGRESS => LoadingCmd::UpdateProgress(progress * 100.0),
        ThreadLoadStatus::LOADED => LoadingCmd::Finished,
        _ => LoadingCmd::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_equal_approx ---
    #[test]
    fn is_equal_approx_true_within_tolerance() {
        assert!(is_equal_approx(1.0 / 3.0, 1.0 / 3.0 + 1e-6));
    }

    #[test]
    fn is_equal_approx_false_outside_tolerance() {
        assert!(!is_equal_approx(1.0 / 3.0, 1.0 / 3.0 + 1e-4));
    }

    // --- display_mode ---
    #[test]
    fn display_mode_maximized_shows_as_windowed() {
        assert_eq!(display_mode_button(WindowMode::MAXIMIZED), DisplayModeButton::Windowed);
    }

    #[test]
    fn display_mode_windowed_shows_as_windowed() {
        assert_eq!(display_mode_button(WindowMode::WINDOWED), DisplayModeButton::Windowed);
    }

    #[test]
    fn display_mode_fullscreen_shows_as_fullscreen() {
        assert_eq!(display_mode_button(WindowMode::FULLSCREEN), DisplayModeButton::Fullscreen);
    }

    #[test]
    fn display_mode_exclusive_fullscreen_shows_as_exclusive() {
        assert_eq!(
            display_mode_button(WindowMode::EXCLUSIVE_FULLSCREEN),
            DisplayModeButton::ExclusiveFullscreen
        );
    }

    #[test]
    fn display_mode_windowed_button_writes_windowed_not_maximized() {
        assert_eq!(display_mode_value(DisplayModeButton::Windowed), WindowMode::WINDOWED);
    }

    // --- vsync ---
    #[test]
    fn vsync_round_trips_all_four() {
        for v in [VSyncMode::DISABLED, VSyncMode::ENABLED, VSyncMode::ADAPTIVE, VSyncMode::MAILBOX] {
            assert_eq!(vsync_value(vsync_button(v)), v);
        }
    }

    // --- max_fps ---
    #[test]
    fn max_fps_round_trips_all_eight() {
        for fps in [30, 40, 60, 72, 90, 120, 144, 0] {
            assert_eq!(max_fps_value(max_fps_button(fps)), fps);
        }
    }

    #[test]
    fn max_fps_unlisted_value_shows_as_unlimited() {
        assert_eq!(max_fps_button(999), MaxFpsButton::Unlimited);
    }

    // --- resolution_scale ---
    #[test]
    fn resolution_scale_round_trips_all_six_presets() {
        for b in [
            ResolutionScaleButton::UltraPerformance,
            ResolutionScaleButton::Performance,
            ResolutionScaleButton::Balanced,
            ResolutionScaleButton::Quality,
            ResolutionScaleButton::UltraQuality,
            ResolutionScaleButton::Native,
        ] {
            assert_eq!(resolution_scale_button(resolution_scale_value(b)), b);
        }
    }

    #[test]
    fn resolution_scale_unlisted_value_falls_back_to_native() {
        assert_eq!(resolution_scale_button(0.6), ResolutionScaleButton::Native);
    }

    // --- scale_filter ---
    #[test]
    fn scale_filter_round_trips_all_six() {
        for f in [
            ScaleFilter::Nearest,
            ScaleFilter::Bilinear,
            ScaleFilter::Fsr1,
            ScaleFilter::MetalFxSpatial,
            ScaleFilter::Fsr2,
            ScaleFilter::MetalFxTemporal,
        ] {
            assert_eq!(scale_filter_value(scale_filter_button(f)), f);
        }
    }

    // --- gi_type / gi_quality ---
    #[test]
    fn gi_type_round_trips_all_three() {
        for t in [GiType::LightmapGi, GiType::VoxelGi, GiType::Sdfgi] {
            assert_eq!(gi_type_value(gi_type_button(t)), t);
        }
    }

    #[test]
    fn gi_quality_round_trips_all_three() {
        for q in [GiQuality::Disabled, GiQuality::Low, GiQuality::High] {
            assert_eq!(gi_quality_value(gi_quality_button(q)), q);
        }
    }

    // --- msaa / screen_space_aa ---
    #[test]
    fn msaa_round_trips_all_four() {
        for m in [Msaa::DISABLED, Msaa::MSAA_2X, Msaa::MSAA_4X, Msaa::MSAA_8X] {
            assert_eq!(msaa_value(msaa_button(m).unwrap()), m);
        }
    }

    #[test]
    fn msaa_unlisted_value_presses_nothing() {
        assert_eq!(msaa_button(Msaa::MAX), None);
    }

    #[test]
    fn screen_space_aa_round_trips_all_three() {
        for a in [ScreenSpaceAa::DISABLED, ScreenSpaceAa::FXAA, ScreenSpaceAa::SMAA] {
            assert_eq!(screen_space_aa_value(screen_space_aa_button(a).unwrap()), a);
        }
    }

    #[test]
    fn screen_space_aa_unlisted_value_presses_nothing() {
        assert_eq!(screen_space_aa_button(ScreenSpaceAa::MAX), None);
    }

    // --- ssao / ssil ---
    #[test]
    fn ssao_quality_round_trips_all_three() {
        for q in [SsaoQuality::Disabled, SsaoQuality::Medium, SsaoQuality::High] {
            assert_eq!(ssao_quality_value(ssao_quality_button(q)), q);
        }
    }

    #[test]
    fn ssil_quality_round_trips_all_three() {
        for q in [SsilQuality::Disabled, SsilQuality::Medium, SsilQuality::High] {
            assert_eq!(ssil_quality_value(ssil_quality_button(q)), q);
        }
    }

    // --- loading_step ---
    #[test]
    fn loading_step_in_progress_scales_progress_to_percent() {
        match loading_step(ThreadLoadStatus::IN_PROGRESS, 0.5) {
            LoadingCmd::UpdateProgress(p) => assert_eq!(p, 50.0),
            _ => panic!("expected UpdateProgress"),
        }
    }

    #[test]
    fn loading_step_loaded_is_finished() {
        assert!(matches!(loading_step(ThreadLoadStatus::LOADED, 1.0), LoadingCmd::Finished));
    }

    #[test]
    fn loading_step_other_status_is_failed() {
        assert!(matches!(loading_step(ThreadLoadStatus::FAILED, 0.0), LoadingCmd::Failed));
    }
}
