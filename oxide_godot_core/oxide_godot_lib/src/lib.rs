use godot::prelude::*;

mod debug_label;
mod part_disappear;
mod blast;

struct OxideGodot;

#[gdextension]
unsafe impl ExtensionLibrary for OxideGodot {}
