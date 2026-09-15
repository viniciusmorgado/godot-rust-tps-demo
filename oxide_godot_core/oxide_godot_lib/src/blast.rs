use godot::classes::{AnimationPlayer, Camera3D, CpuParticles3D, INode3D, Node3D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct Blast {
    base: Base<Node3D>,
    #[init(node = "LightRays")]
    light_rays: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
    // @onready var camera: Camera3D = get_tree().get_root().get_camera_3d()
    camera: Option<Gd<Camera3D>>,
}

#[godot_api]
impl INode3D for Blast {
    fn ready(&mut self) {
        self.camera = self.base().get_tree().get_root().unwrap().get_camera_3d();
        // await $AnimationPlayer.animation_finished
        self.animation_player
            .signals()
            .animation_finished()
            .connect_other(&*self, |this: &mut Blast, _anim_name: StringName| {
                this.base_mut().queue_free();
            });
    }

    fn process(&mut self, _delta: f64) {
        if let Some(camera) = &self.camera {
            if camera.is_instance_valid() {
                let origin = camera.get_global_transform().origin;
                self.light_rays.look_at(origin);
            }
        }
    }
}
