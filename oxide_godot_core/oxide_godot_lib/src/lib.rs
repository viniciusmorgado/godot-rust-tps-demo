use godot::prelude::*;

mod debug_label;
mod part_disappear;
mod blast;
mod camera_noise_shake;
mod player_input;
mod player;
mod bullet;
mod door;
mod hittable;
mod part;
mod red_robot;
mod flying_forklift;
mod level;
mod menu;
mod main_scene;
mod settings;

struct OxideGodot;

#[gdextension]
unsafe impl ExtensionLibrary for OxideGodot {}
