//! The camera shake's engine system (constitution 1.5.1 "ECS shape (v3)"; specs/012 research
//! R7): ONE `SyncOut` member of the FRAME schedule — the three noise samples answer no mid-tick
//! decision, so the shake is a sync-only pair (`shake_decide` + this), like V3-A's blast.

use bevy_ecs::prelude::*;

use super::model::{CameraShakeTuning, offsets};
use crate::ecs::markers::{ShakePending, StartRotation, Tuning};
use crate::ecs::{Handles, NodeHandles};

/// `SyncOut`, FRAME schedule: when `shake_decide` left `Some((shake, time))`, the three
/// `get_noise_1d(time)` samples (v2 `camera_noise_shake.rs:50-54`), `offsets` (`:55`, pure,
/// called from glue) and `camera.set_rotation(start_rotation + offset)` (`:56-57`); nothing
/// otherwise (v2 `:45`).
pub fn sync_out_shake(
    tuning: Res<Tuning<CameraShakeTuning>>,
    mut handles: NonSendMut<NodeHandles>,
    players: Query<(Entity, &StartRotation, &ShakePending)>,
) {
    for (entity, start, pending) in &players {
        let Some((shake, time)) = pending.0 else {
            continue;
        };
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        let samples = [
            p.noise[0].get_noise_1d(time as f32),
            p.noise[1].get_noise_1d(time as f32),
            p.noise[2].get_noise_1d(time as f32),
        ];
        let offset = offsets(shake, samples, &tuning.0);
        p.camera.set_rotation(start.0 + offset);
    }
}
