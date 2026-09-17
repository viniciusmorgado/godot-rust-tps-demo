use godot::classes::{Camera3D, FastNoiseLite, ICamera3D};
use godot::global::randi;
use godot::prelude::*;

mod model;

use model::CameraShakeTuning;

#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct CameraNoiseShake {
    base: Base<Camera3D>,
    // Default values.
    start_rotation: Vector3,
    trauma: f32,
    time: f64,
    #[init(val = FastNoiseLite::new_gd())]
    noise_yaw: Gd<FastNoiseLite>,
    #[init(val = FastNoiseLite::new_gd())]
    noise_pitch: Gd<FastNoiseLite>,
    #[init(val = FastNoiseLite::new_gd())]
    noise_roll: Gd<FastNoiseLite>,
    #[init(val = (randi() as i32))]
    noise_seed: i32,
}

#[godot_api]
impl ICamera3D for CameraNoiseShake {
    fn ready(&mut self) {
        self.noise_yaw.set_seed(self.noise_seed);
        self.noise_pitch.set_seed(self.noise_seed.wrapping_add(1));
        self.noise_roll.set_seed(self.noise_seed.wrapping_add(2));
        for noise in [&mut self.noise_yaw, &mut self.noise_pitch, &mut self.noise_roll] {
            noise.set_fractal_octaves(1);
            noise.set_fractal_lacunarity(1.0);
        }

        // This variable is reset if the camera position is changed by other scripts,
        // such as when zooming in/out or focusing checked a different position.
        // This should NOT be done when the camera shake is happening.
        self.start_rotation = self.base().get_rotation();
    }

    fn process(&mut self, delta: f64) {
        if self.trauma > 0.0 {
            let tuning = CameraShakeTuning::default();
            self.trauma = model::decay(self.trauma, delta as f32, &tuning);
            self.time = model::advance_time(self.time, delta, &tuning);
            let shake = model::shake(self.trauma);
            let samples = [
                self.noise_yaw.get_noise_1d(self.time as f32),
                self.noise_pitch.get_noise_1d(self.time as f32),
                self.noise_roll.get_noise_1d(self.time as f32),
            ];
            let offset = model::offsets(shake, samples, &tuning);
            let rotation = self.start_rotation + offset;
            self.base_mut().set_rotation(rotation);
        }
    }
}

impl CameraNoiseShake {
    // Add trauma to start/continue the shake.
    pub(crate) fn add_trauma(&mut self, amount: f64) {
        let tuning = CameraShakeTuning::default();
        self.trauma = model::add_trauma(self.trauma, amount as f32, &tuning);
    }
}
