use godot::prelude::*;

mod debug_label;
mod part_disappear;
mod blast;
mod camera_noise_shake;

struct OxideGodot;

#[gdextension]
unsafe impl ExtensionLibrary for OxideGodot {}
