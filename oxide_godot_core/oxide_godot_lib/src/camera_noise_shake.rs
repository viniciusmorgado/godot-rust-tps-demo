//! The camera's sub-bridge (constitution 1.5.1 "ECS shape (v3)"; specs/012 contracts §1):
//! registers nothing and pushes nothing — it keeps v2's one-shot setup (the three seeded noises
//! and the rest rotation), which `Player.ready` reads once into the entity's handles. The shake
//! itself is `camera_noise_shake/system.rs` (pure) + `camera_noise_shake/sync.rs` (engine) on
//! the player entity's `Trauma`/`ShakeTime`.

use godot::classes::{Camera3D, FastNoiseLite, ICamera3D};
use godot::global::randi;
use godot::prelude::*;

pub(crate) mod model;
pub(crate) mod sync;
pub(crate) mod system;

#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct CameraNoiseShake {
    base: Base<Camera3D>,
    // Default values.
    start_rotation: Vector3,
    #[init(val = FastNoiseLite::new_gd())]
    noise_yaw: Gd<FastNoiseLite>,
    #[init(val = FastNoiseLite::new_gd())]
    noise_pitch: Gd<FastNoiseLite>,
    #[init(val = FastNoiseLite::new_gd())]
    noise_roll: Gd<FastNoiseLite>,
    // Drawn at init, in the same call order as v2 (seeded parity, spec FR-017).
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
}

impl CameraNoiseShake {
    /// The three noises (`noise_yaw`, `noise_pitch`, `noise_roll`), read once by `Player.ready`
    /// into the entity's handles (specs/012 research R5).
    pub(crate) fn noises(&self) -> [Gd<FastNoiseLite>; 3] {
        [self.noise_yaw.clone(), self.noise_pitch.clone(), self.noise_roll.clone()]
    }

    /// The rest rotation captured in `ready`, read once by `Player.ready` (specs/012 research R5).
    pub(crate) fn start_rotation(&self) -> Vector3 {
        self.start_rotation
    }
}
