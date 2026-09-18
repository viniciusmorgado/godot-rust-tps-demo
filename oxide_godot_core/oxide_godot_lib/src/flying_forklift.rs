use crate::settings::Settings;
use godot::classes::{CharacterBody3D, ICharacterBody3D, Node3D, SpotLight3D};
use godot::global::randf;
use godot::prelude::*;

mod pure {
    /// v1: `flying_forklift.rs:30`'s `(randf() * child_count as f64).floor() as usize` —
    /// `r` an already-sampled `randf()` value from glue.
    pub fn pick_model(r: f64, count: usize) -> usize {
        (r * count as f64).floor() as usize
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn r_zero_picks_first_index() {
            assert_eq!(pick_model(0.0, 3), 0);
        }

        #[test]
        fn r_just_below_one_picks_last_index() {
            assert_eq!(pick_model(0.999_999, 3), 2);
        }

        #[test]
        fn single_child_always_picks_index_zero() {
            assert_eq!(pick_model(0.5, 1), 0);
        }
    }
}

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

        // Pick one of the forklift models to display.
        // We have 3 models, may as well use them.
        let children = self.base().get_child(0).unwrap().get_children();
        let child_count = children.len();
        let which_enabled = pure::pick_model(randf(), child_count);
        for (i, child) in children.iter_shared().enumerate() {
            child.cast::<Node3D>().set_visible(i == which_enabled);
        }
    }
}

// TODO: We can maybe implement func hit():
