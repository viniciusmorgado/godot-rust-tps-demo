use godot::prelude::*;

mod debug_label;
mod part_disappear;
mod blast;
mod camera_noise_shake;
mod player_input;
mod player;
mod bullet;
mod door;
mod part;
mod red_robot;
mod flying_forklift;

struct OxideGodot;

#[gdextension]
unsafe impl ExtensionLibrary for OxideGodot {}
