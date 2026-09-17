use godot::prelude::*;

/// Tuning constants for the camera shake effect — the exact `v1` literals from
/// `camera_noise_shake.rs`.
#[derive(Clone, Copy, Debug)]
pub struct CameraShakeTuning {
    pub speed: f32,
    pub decay_rate: f32,
    pub max_yaw: f32,
    pub max_pitch: f32,
    pub max_roll: f32,
    pub max_trauma: f32,
}

impl Default for CameraShakeTuning {
    fn default() -> Self {
        Self {
            speed: 1.0,
            decay_rate: 1.5,
            max_yaw: 0.05,
            max_pitch: 0.05,
            max_roll: 0.1,
            max_trauma: 1.2,
        }
    }
}

/// `v1`'s `decay_trauma` (`camera_noise_shake.rs:59-62`).
pub fn decay(trauma: f32, dt: f32, tuning: &CameraShakeTuning) -> f32 {
    let change = tuning.decay_rate * dt;
    (trauma - change).max(0.0)
}

/// `v1`'s `apply_shake`'s time accumulator (`camera_noise_shake.rs:67`). The `× 5000.0` is a
/// magic number kept verbatim from `v1` ("using a magic number here to get a pleasing effect at
/// SPEED 1.0").
pub fn advance_time(time: f64, dt: f64, tuning: &CameraShakeTuning) -> f64 {
    time + dt * tuning.speed as f64 * 5000.0
}

/// `v1`'s `apply_shake`'s shake magnitude (`camera_noise_shake.rs:68`).
pub fn shake(trauma: f32) -> f32 {
    trauma * trauma
}

/// `v1`'s exact, non-alphabetical axis mapping (`camera_noise_shake.rs:69-74`): `samples[0]` is
/// the noise sampled at `noise_seed`, `samples[1]` at `noise_seed.wrapping_add(1)`, `samples[2]`
/// at `noise_seed.wrapping_add(2)`. `yaw` reads `samples[0]`, `pitch` reads `samples[1]`, `roll`
/// reads `samples[2]`; the resulting rotation is `Vector3(pitch, yaw, roll)`.
pub fn offsets(shake: f32, samples: [f32; 3], tuning: &CameraShakeTuning) -> Vector3 {
    let yaw = tuning.max_yaw * shake * samples[0];
    let pitch = tuning.max_pitch * shake * samples[1];
    let roll = tuning.max_roll * shake * samples[2];
    Vector3::new(pitch, yaw, roll)
}

/// `v1`'s `add_trauma` clamp (`camera_noise_shake.rs:52-54`).
pub fn add_trauma(current: f32, amount: f32, tuning: &CameraShakeTuning) -> f32 {
    (current + amount).min(tuning.max_trauma)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_subtracts_decay_rate_times_dt() {
        let tuning = CameraShakeTuning::default();
        let result = decay(1.0, 0.1, &tuning);
        assert_eq!(result, 1.0 - tuning.decay_rate * 0.1);
    }

    #[test]
    fn decay_floors_at_zero() {
        let tuning = CameraShakeTuning::default();
        let result = decay(0.01, 1.0, &tuning);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn advance_time_reproduces_the_5000_magic_number_formula() {
        let tuning = CameraShakeTuning::default();
        let result = advance_time(10.0, 0.1, &tuning);
        assert_eq!(result, 10.0 + 0.1 * tuning.speed as f64 * 5000.0);
    }

    #[test]
    fn shake_is_trauma_squared() {
        assert_eq!(shake(0.5), 0.25);
    }

    #[test]
    fn offsets_maps_each_axis_to_its_own_sample_not_swapped() {
        let tuning = CameraShakeTuning::default();
        // Three DISTINCT sample values so a swapped axis mapping fails this test.
        let samples = [0.1, 0.2, 0.3];
        let result = offsets(1.0, samples, &tuning);
        assert_eq!(result.x, tuning.max_pitch * 1.0 * samples[1]);
        assert_eq!(result.y, tuning.max_yaw * 1.0 * samples[0]);
        assert_eq!(result.z, tuning.max_roll * 1.0 * samples[2]);
    }

    #[test]
    fn add_trauma_accumulates_below_the_ceiling() {
        let tuning = CameraShakeTuning::default();
        let result = add_trauma(0.2, 0.3, &tuning);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn add_trauma_clamps_at_max_trauma() {
        let tuning = CameraShakeTuning::default();
        let result = add_trauma(1.0, 1.0, &tuning);
        assert_eq!(result, tuning.max_trauma);
    }
}
