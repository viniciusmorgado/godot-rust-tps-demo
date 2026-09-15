use godot::prelude::*;

mod debug_label;
mod part_disappear;
mod blast;
mod camera_noise_shake;
mod player_input;

struct OxideGodot;

#[gdextension]
unsafe impl ExtensionLibrary for OxideGodot {}
