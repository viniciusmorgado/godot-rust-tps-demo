use crate::settings::Settings;
use godot::classes::{
    AnimationPlayer, CharacterBody3D, CollisionShape3D, ICharacterBody3D, KinematicCollision3D, Node3D,
    OmniLight3D,
};
use godot::prelude::*;

const BULLET_VELOCITY: f32 = 20.0;

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Bullet {
    base: Base<CharacterBody3D>,

    #[init(val = 5.0)]
    time_alive: f32,
    hit: bool,

    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
    #[init(node = "CollisionShape3D")]
    collision_shape: OnReady<Gd<CollisionShape3D>>,
    #[init(node = "OmniLight3D")]
    omni_light: OnReady<Gd<OmniLight3D>>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

#[godot_api]
impl ICharacterBody3D for Bullet {
    fn ready(&mut self) {
        if !self.base().get_multiplayer().unwrap().is_server() {
            self.base_mut().set_physics_process(false);
            self.collision_shape.set_disabled(true);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        if self.hit {
            return;
        }
        self.time_alive -= delta as f32;
        if self.time_alive < 0.0 {
            self.hit = true;
            self.base_mut().rpc("explode", &[]);
        }
        let displacement: Vector3 =
            -(delta as f32) * BULLET_VELOCITY * self.base().get_transform().basis.col_c();
        let col: Option<Gd<KinematicCollision3D>> = self.base_mut().move_and_collide(displacement);
        if let Some(col) = col {
            let collider: Option<Gd<Node3D>> =
                col.get_collider().and_then(|c| c.try_cast::<Node3D>().ok());
            if let Some(mut collider) = collider
                && collider.has_method("hit")
            {
                collider.rpc("hit", &[]);
            }
            self.collision_shape.set_disabled(true);
            self.base_mut().rpc("explode", &[]);
            self.hit = true;
        }
    }
}

#[godot_api]
impl Bullet {
    #[rpc(authority, call_local, unreliable)]
    fn explode(&mut self) {
        self.animation_player.play_ex().name("explode").done();

        // Only enable shadows for the explosion, as the moving light
        // is very small and doesn't noticeably benefit from shadow mapping.
        if self.settings.bind().graphics().shadow_mapping {
            self.omni_light.set_shadow(true);
        }
    }

    #[func]
    fn destroy(&mut self) {
        if !self.base().get_multiplayer().unwrap().is_server() {
            return;
        }
        self.base_mut().queue_free();
    }
}
