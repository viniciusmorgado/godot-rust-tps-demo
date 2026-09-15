use godot::classes::{Camera3D, FastNoiseLite, ICamera3D};
use godot::global::randi;
use godot::prelude::*;

// Constant values of the effect.
const SPEED: f32 = 1.0;
const DECAY_RATE: f32 = 1.5;
const MAX_YAW: f32 = 0.05;
const MAX_PITCH: f32 = 0.05;
const MAX_ROLL: f32 = 0.1;
const MAX_TRAUMA: f32 = 1.2;

#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct CameraNoiseShake {
    base: Base<Camera3D>,
    // Default values.
    start_rotation: Vector3,
    trauma: f32,
    time: f64,
    #[init(val = FastNoiseLite::new_gd())]
    noise: Gd<FastNoiseLite>,
    #[init(val = (randi() as i32))]
    noise_seed: i32,
}

#[godot_api]
impl ICamera3D for CameraNoiseShake {
    fn ready(&mut self) {
        self.noise.set_seed(self.noise_seed);
        self.noise.set_fractal_octaves(1);
        self.noise.set_fractal_lacunarity(1.0);

        // This variable is reset if the camera position is changed by other scripts,
        // such as when zooming in/out or focusing checked a different position.
        // This should NOT be done when the camera shake is happening.
        self.start_rotation = self.base().get_rotation();
    }

    fn process(&mut self, delta: f64) {
        if self.trauma > 0.0 {
            self.decay_trauma(delta);
            self.apply_shake(delta);
        }
    }
}

#[godot_api]
impl CameraNoiseShake {
    // Add trauma to start/continue the shake.
    #[func]
    pub(crate) fn add_trauma(&mut self, amount: f64) {
        self.trauma = (self.trauma + amount as f32).min(MAX_TRAUMA);
    }
}

impl CameraNoiseShake {
    // Decay the trauma effect over time.
    fn decay_trauma(&mut self, delta: f64) {
        let change: f32 = DECAY_RATE * delta as f32;
        self.trauma = (self.trauma - change).max(0.0);
    }

    // Apply the random shake accoring to delta time.
    fn apply_shake(&mut self, delta: f64) {
        // Using a magic number here to get a pleasing effect at SPEED 1.0.
        self.time += delta * SPEED as f64 * 5000.0;
        let shake: f32 = self.trauma * self.trauma;
        let yaw: f32 = MAX_YAW * shake * self.get_noise_value(self.noise_seed, self.time);
        let pitch: f32 =
            MAX_PITCH * shake * self.get_noise_value(self.noise_seed.wrapping_add(1), self.time);
        let roll: f32 =
            MAX_ROLL * shake * self.get_noise_value(self.noise_seed.wrapping_add(2), self.time);
        let rotation = self.start_rotation + Vector3::new(pitch, yaw, roll);
        self.base_mut().set_rotation(rotation);
    }

    // Return a random float in range(-1, 1) using OpenSimplex noise.
    fn get_noise_value(&mut self, seed_value: i32, pos: f64) -> f32 {
        self.noise.set_seed(seed_value);
        self.noise.get_noise_1d(pos as f32)
    }
}
