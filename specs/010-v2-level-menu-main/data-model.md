# Data Model: Milestone V2-E — level, menu, main: the scene manager and the end of v2

Every signature below carries the exact `v1` line range it reproduces (research.md's evidence).
Pure functions are `Gd`-free, `Variant`-free, tested with `cargo test`.

## `main_scene.rs` — glue only, no pure submodule

No domain decision exists here beyond `try_cast`-based dispatch (research.md R2's
`change_scene_to_packed`), which needs no separate pure function — the constitution requires
pure separation only where domain logic exists (Principle III), and `try_cast`'s own
compile-time type safety already is the "decision."

```rust
// Loses #[func] — nothing calls any of these three by name after this milestone (R2).
impl Main {
    fn go_to_main_menu(&mut self) { /* unchanged body */ }
    fn replace_main_scene(&mut self, resource: Gd<PackedScene>) {
        // R1: Callable::from_fn(...).call_deferred(&[]) instead of
        // self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()])
    }
    fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) {
        // R2: try_cast::<Level>/<Menu>, connect_other typed, instead of has_signal +
        // Callable::from_object_method
    }
}
```

## `level/model.rs`

```rust
/// v1: level.rs:94-162's three setup_* functions, unified. research.md R3's 9-cell table.
pub struct GiPlan {
    pub sdfgi_enabled: bool,
    pub voxel_visible: bool,
    pub probes_visible: bool,
    pub free_lightmap: bool,
    pub create_lightmap: bool,
    /// `Some(false)` only when gi_type==LightmapGi && gi_quality==Disabled (v1 explicitly
    /// hides it, :158-161); `None` everywhere else -- v1 has NO explicit "show" write for the
    /// LightmapGi node anywhere; a fresh one is visible by the engine's own default.
    pub lightmap_visible: Option<bool>,
    pub sdfgi_rays: Option<EnvironmentSdfgiRayCount>,
    pub voxel_quality: Option<VoxelGiQuality>,
}

pub fn gi_plan(gi_type: GiType, gi_quality: GiQuality, has_lightmap: bool) -> GiPlan
// research.md R3's full 9-cell table (+ has_lightmap gating create_lightmap when gi_type ==
// LightmapGi). has_lightmap only matters for that one cell-family; in v1's own only call site
// (Level::ready) it is always false (lightmap_gi starts None), so 9 of the 9 catalogued cells
// are what the harness (research.md R8b) actually exercises -- has_lightmap=true is a
// correctness/testability completion, not an observed-in-game path (spec Edge Cases).
```

```rust
/// v1: level.rs:199 (`randi() % count as i64`), add_player's inline spawn-point pick.
pub fn pick_spawn(r: i64, count: i64) -> i64 {
    r.rem_euclid(count)
}
```

## `part.rs`-style inline `mod pure` in `flying_forklift.rs`

```rust
/// v1: flying_forklift.rs:30 (`(randf() * child_count as f64).floor() as usize`).
pub fn pick_model(r: f64, count: usize) -> usize {
    (r * count as f64).floor() as usize
}
```

## `menu/model.rs`

Each row is a small pure pair (`_button` for the show side, `_value` for the apply side); the
button-side enum names one variant per `OnReady<Gd<Button>>` field the row already has.

```rust
// --- display_mode (v1: menu.rs:306-310 show, :430-436 apply) ---
pub enum DisplayModeButton { Windowed, Fullscreen, ExclusiveFullscreen }
pub fn display_mode_button(mode: WindowMode) -> DisplayModeButton {
    match mode {
        WindowMode::WINDOWED | WindowMode::MAXIMIZED => DisplayModeButton::Windowed,
        WindowMode::FULLSCREEN => DisplayModeButton::Fullscreen,
        _ => DisplayModeButton::ExclusiveFullscreen,
    }
}
pub fn display_mode_value(button: DisplayModeButton) -> WindowMode {
    match button {
        DisplayModeButton::Windowed => WindowMode::WINDOWED, // MAXIMIZED collapses here (quirk kept)
        DisplayModeButton::Fullscreen => WindowMode::FULLSCREEN,
        DisplayModeButton::ExclusiveFullscreen => WindowMode::EXCLUSIVE_FULLSCREEN,
    }
}

// --- vsync (menu.rs:312-317 show, :438-446 apply) — exhaustive both ways ---
pub enum VsyncButton { Disabled, Enabled, Adaptive, Mailbox }
pub fn vsync_button(v: VSyncMode) -> VsyncButton { /* DISABLED/ENABLED/ADAPTIVE/else->Mailbox */ }
pub fn vsync_value(b: VsyncButton) -> VSyncMode { /* 1:1 */ }

// --- max_fps (menu.rs:319-328 show, :448-464 apply) — 8 literal values, exhaustive ---
pub enum MaxFpsButton { Fps30, Fps40, Fps60, Fps72, Fps90, Fps120, Fps144, Unlimited }
pub fn max_fps_button(fps: i32) -> MaxFpsButton { /* 30/40/60/72/90/120/144/else->Unlimited */ }
pub fn max_fps_value(b: MaxFpsButton) -> i32 { /* Unlimited -> 0 */ }

// --- resolution_scale (menu.rs:330-342 show, :466-478 apply) — epsilon fallback (R6) ---
pub enum ResolutionScaleButton { UltraPerformance, Performance, Balanced, Quality, UltraQuality, Native }
pub fn resolution_scale_button(scale: f64) -> ResolutionScaleButton {
    // is_equal_approx reimplemented pure (R6): tolerance = max(CMP_EPSILON, CMP_EPSILON*|a|),
    // CMP_EPSILON = 1e-5, checked in the SAME order as v1 (ultra_performance first ... else
    // Native) so an unlisted value falls to Native, exactly as v1 does.
}
pub fn resolution_scale_value(b: ResolutionScaleButton) -> f64 { /* the 6 literals, 1:1 */ }

// --- scale_filter (menu.rs:344-351 show, :480-492 apply) — exhaustive both ways ---
pub fn scale_filter_button(f: ScaleFilter) -> ScaleFilterButton { /* 1:1, 6 variants */ }
pub fn scale_filter_value(b: ScaleFilterButton) -> ScaleFilter { /* 1:1 */ }
// metalfx_supported's visibility gate (menu.rs:217-220) stays in glue, ready()-only, untouched.

// --- gi_type / gi_quality (menu.rs:353-363 show, :494-508 apply) — exhaustive both ways ---
pub fn gi_type_button(t: GiType) -> GiTypeButton { /* 1:1, 3 variants */ }
pub fn gi_type_value(b: GiTypeButton) -> GiType { /* 1:1 */ }
pub fn gi_quality_button(q: GiQuality) -> GiQualityButton { /* 1:1, 3 variants */ }
pub fn gi_quality_value(b: GiQualityButton) -> GiQuality { /* 1:1 */ }

// --- msaa / screen_space_aa (menu.rs:371-384 show, :512-528 apply) — NO catch-all on show (v1 quirk) ---
pub fn msaa_button(m: Msaa) -> Option<MsaaButton> {
    // DISABLED/2X/4X/8X -> Some(...); any other value -> None (v1's own gap, :371-377 has no
    // `_ => ...` press -- NOTHING gets pressed; preserved verbatim, not "fixed")
}
pub fn msaa_value(b: MsaaButton) -> Msaa { /* 1:1, apply-side has no catch-all either (unchanged
    field if none pressed -- unreachable in practice per ButtonGroup's own invariant) */ }
pub fn screen_space_aa_button(a: ScreenSpaceAa) -> Option<ScreenSpaceAaButton> { /* same shape, 3 named values */ }
pub fn screen_space_aa_value(b: ScreenSpaceAaButton) -> ScreenSpaceAa { /* 1:1 */ }

// --- ssao_quality / ssil_quality (menu.rs:392-402 show, :532-546 apply) — exhaustive both ways ---
pub fn ssao_quality_button(q: SsaoQuality) -> SsaoQualityButton { /* 1:1, 3 variants */ }
pub fn ssao_quality_value(b: SsaoQualityButton) -> SsaoQuality { /* 1:1 */ }
pub fn ssil_quality_button(q: SsilQuality) -> SsilQualityButton { /* 1:1, 3 variants */ }
pub fn ssil_quality_value(b: SsilQualityButton) -> SsilQuality { /* 1:1 */ }

// --- taa / shadow_mapping / bloom / volumetric_fog (4 plain booleans, identical shape) ---
// show: if field { enabled.set_pressed(true) } else { disabled.set_pressed(true) } -- trivial,
// no pure function needed beyond the bool itself; apply: field = enabled_button.is_pressed().
```

```rust
/// v1: menu.rs:246-256's three-way branch.
pub enum LoadingCmd { UpdateProgress(f64), Finished, Failed }
pub fn loading_step(status: ThreadLoadStatus, progress: f64) -> LoadingCmd {
    match status {
        ThreadLoadStatus::IN_PROGRESS => LoadingCmd::UpdateProgress(progress * 100.0),
        ThreadLoadStatus::LOADED => LoadingCmd::Finished,
        _ => LoadingCmd::Failed,
    }
}
```

## Cross-reference: what stays exactly as it is

| Surface | Stays |
|---|---|
| `menu.tscn`'s 10 `[connection]`s | Every `#[func]` name/signature unchanged (FR-016) |
| `Menu::replace_main_scene(scene: Gd<PackedScene>)` signal | Unchanged, already typed |
| `Level::quit()` signal, `peer_connected`/`peer_disconnected` connections | Unchanged, already typed |
| `_make_button_group`, all `OnReady<Gd<Button>>`/`OnReady<Gd<HBoxContainer>>` fields | Unchanged, no `.tscn` edit |
| `GraphicsSettings` struct, `apply_graphics_settings`, `save_settings` | Unchanged (V2-A) |
| `metalfx_supported`'s one-time button-hiding gate | Unchanged, `ready()`-only |
| `Marker3D` cast, `Array::shuffle()` | Unchanged, already typed/engine-appropriate |
