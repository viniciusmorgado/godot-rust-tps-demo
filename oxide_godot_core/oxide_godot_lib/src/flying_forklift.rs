use crate::settings::Settings;
use godot::classes::{CharacterBody3D, ICharacterBody3D, Node3D, SpotLight3D};
use godot::global::{randf, randomize};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct FlyingForklift {
    base: Base<CharacterBody3D>,

    #[init(node = "SpotLight3D")]
    spot_light: OnReady<Gd<SpotLight3D>>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

#[godot_api]
impl ICharacterBody3D for FlyingForklift {
    fn ready(&mut self) {
        if !self.settings.bind().graphics().shadow_mapping {
            self.spot_light.set_shadow(false);
        }

        // Randomize the forklift model.
        // We have 3 models, may as well use them.
        randomize();
        let children = self.base().get_child(0).unwrap().get_children();
        let child_count = children.len();
        let which_enabled = (randf() * child_count as f64).floor() as usize;
        for (i, child) in children.iter_shared().enumerate() {
            child.cast::<Node3D>().set_visible(i == which_enabled);
        }
    }
}

// TODO: We can maybe implement func hit():
