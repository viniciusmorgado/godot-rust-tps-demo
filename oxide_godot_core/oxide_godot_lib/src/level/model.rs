//! Pure model for `Level`'s GI setup and random-spawn-point pick.
//!
//! Nothing here touches `Gd<T>`, an engine singleton, `Variant`, `GString` or `StringName`.
//! `EnvironmentSdfgiRayCount`/`VoxelGiQuality` are `#[repr(transparent)]` newtypes around an
//! `i32` ordinal with no FFI behind their construction/comparison (same rule as
//! `settings/graphics.rs`'s engine enums, research.md R3).

use godot::classes::rendering_server::{EnvironmentSdfgiRayCount, VoxelGiQuality};

use crate::settings::{GiQuality, GiType};

/// The observable engine effects of choosing a GI technique and quality — replaces
/// `level.rs`'s three near-identical `setup_sdfgi`/`setup_voxelgi`/`setup_lightmapgi`
/// functions (v1: `level.rs:94-162`).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct GiPlan {
    pub sdfgi_enabled: bool,
    pub voxel_visible: bool,
    pub probes_visible: bool,
    pub free_lightmap: bool,
    pub create_lightmap: bool,
    /// `Some(false)` only on `LightmapGi`/`Disabled` (v1 explicitly hides it, `:158-161`);
    /// `None` everywhere else — nothing in v1 ever explicitly RE-SHOWS a `LightmapGi` node
    /// (research.md R3's asymmetry).
    pub lightmap_visible: Option<bool>,
    pub sdfgi_rays: Option<EnvironmentSdfgiRayCount>,
    pub voxel_quality: Option<VoxelGiQuality>,
}

/// v1: `level.rs:94-162`'s 9-cell table (research.md R3), plus the `has_lightmap` axis that
/// only matters on the `LightmapGi` rows (`:149`, `if self.lightmap_gi.is_none()`).
pub fn gi_plan(gi_type: GiType, gi_quality: GiQuality, has_lightmap: bool) -> GiPlan {
    match gi_type {
        GiType::Sdfgi => GiPlan {
            sdfgi_enabled: gi_quality != GiQuality::Disabled,
            voxel_visible: false,
            probes_visible: false,
            free_lightmap: true,
            create_lightmap: false,
            lightmap_visible: None,
            sdfgi_rays: match gi_quality {
                GiQuality::High => Some(EnvironmentSdfgiRayCount::COUNT_96),
                GiQuality::Low => Some(EnvironmentSdfgiRayCount::COUNT_32),
                GiQuality::Disabled => None,
            },
            voxel_quality: None,
        },
        GiType::VoxelGi => GiPlan {
            sdfgi_enabled: false,
            voxel_visible: gi_quality != GiQuality::Disabled,
            probes_visible: false,
            free_lightmap: true,
            create_lightmap: false,
            lightmap_visible: None,
            sdfgi_rays: None,
            voxel_quality: match gi_quality {
                GiQuality::High => Some(VoxelGiQuality::HIGH),
                GiQuality::Low => Some(VoxelGiQuality::LOW),
                GiQuality::Disabled => None,
            },
        },
        GiType::LightmapGi => GiPlan {
            sdfgi_enabled: false,
            voxel_visible: false,
            probes_visible: gi_quality != GiQuality::Disabled,
            free_lightmap: false,
            create_lightmap: !has_lightmap,
            lightmap_visible: if gi_quality == GiQuality::Disabled { Some(false) } else { None },
            sdfgi_rays: None,
            voxel_quality: None,
        },
    }
}

/// v1: `level.rs:199`'s `randi() % count as i64` (`add_player`'s random-spawn-point pick,
/// when no explicit spawn point is passed) — `r` is an already-sampled `randi()` value from
/// glue, `count` the number of spawn points; `randi()` is never negative, so this matches the
/// v1 text (`%`) exactly.
pub fn pick_spawn(r: i64, count: i64) -> i64 {
    r % count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdfgi_high() {
        let plan = gi_plan(GiType::Sdfgi, GiQuality::High, false);
        assert!(plan.sdfgi_enabled);
        assert!(!plan.voxel_visible);
        assert!(!plan.probes_visible);
        assert!(plan.free_lightmap);
        assert!(!plan.create_lightmap);
        assert_eq!(plan.lightmap_visible, None);
        assert_eq!(plan.sdfgi_rays, Some(EnvironmentSdfgiRayCount::COUNT_96));
        assert_eq!(plan.voxel_quality, None);
    }

    #[test]
    fn sdfgi_low() {
        let plan = gi_plan(GiType::Sdfgi, GiQuality::Low, false);
        assert!(plan.sdfgi_enabled);
        assert!(!plan.voxel_visible);
        assert!(!plan.probes_visible);
        assert!(plan.free_lightmap);
        assert_eq!(plan.sdfgi_rays, Some(EnvironmentSdfgiRayCount::COUNT_32));
    }

    #[test]
    fn sdfgi_disabled() {
        let plan = gi_plan(GiType::Sdfgi, GiQuality::Disabled, false);
        assert!(!plan.sdfgi_enabled);
        assert!(!plan.voxel_visible);
        assert!(!plan.probes_visible);
        assert!(plan.free_lightmap);
        assert_eq!(plan.sdfgi_rays, None);
    }

    #[test]
    fn voxelgi_high() {
        let plan = gi_plan(GiType::VoxelGi, GiQuality::High, false);
        assert!(!plan.sdfgi_enabled);
        assert!(plan.voxel_visible);
        assert!(!plan.probes_visible);
        assert!(plan.free_lightmap);
        assert_eq!(plan.voxel_quality, Some(VoxelGiQuality::HIGH));
    }

    #[test]
    fn voxelgi_low() {
        let plan = gi_plan(GiType::VoxelGi, GiQuality::Low, false);
        assert!(plan.voxel_visible);
        assert!(plan.free_lightmap);
        assert_eq!(plan.voxel_quality, Some(VoxelGiQuality::LOW));
    }

    #[test]
    fn voxelgi_disabled() {
        let plan = gi_plan(GiType::VoxelGi, GiQuality::Disabled, false);
        assert!(!plan.sdfgi_enabled);
        assert!(!plan.voxel_visible);
        assert!(!plan.probes_visible);
        assert!(plan.free_lightmap);
        assert_eq!(plan.voxel_quality, None);
    }

    #[test]
    fn lightmapgi_high_without_existing_lightmap() {
        let plan = gi_plan(GiType::LightmapGi, GiQuality::High, false);
        assert!(!plan.sdfgi_enabled);
        assert!(!plan.voxel_visible);
        assert!(plan.probes_visible);
        assert!(!plan.free_lightmap);
        assert!(plan.create_lightmap);
        assert_eq!(plan.lightmap_visible, None);
    }

    #[test]
    fn lightmapgi_low_without_existing_lightmap() {
        let plan = gi_plan(GiType::LightmapGi, GiQuality::Low, false);
        assert!(plan.probes_visible);
        assert!(plan.create_lightmap);
        assert_eq!(plan.lightmap_visible, None);
    }

    #[test]
    fn lightmapgi_disabled_without_existing_lightmap() {
        let plan = gi_plan(GiType::LightmapGi, GiQuality::Disabled, false);
        assert!(!plan.probes_visible);
        assert!(!plan.free_lightmap);
        assert!(plan.create_lightmap);
        assert_eq!(plan.lightmap_visible, Some(false));
    }

    #[test]
    fn lightmapgi_high_with_existing_lightmap_does_not_recreate() {
        let plan = gi_plan(GiType::LightmapGi, GiQuality::High, true);
        assert!(!plan.create_lightmap);
        assert!(plan.probes_visible);
    }

    #[test]
    fn lightmapgi_disabled_with_existing_lightmap_does_not_recreate_but_still_hides() {
        let plan = gi_plan(GiType::LightmapGi, GiQuality::Disabled, true);
        assert!(!plan.create_lightmap);
        assert_eq!(plan.lightmap_visible, Some(false));
    }

    #[test]
    fn pick_spawn_first_index() {
        assert_eq!(pick_spawn(0, 4), 0);
    }

    #[test]
    fn pick_spawn_last_index() {
        assert_eq!(pick_spawn(3, 4), 3);
    }

    #[test]
    fn pick_spawn_wraps() {
        assert_eq!(pick_spawn(4, 4), 0);
    }
}
