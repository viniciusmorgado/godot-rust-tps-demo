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

        // await $AnimationPlayer.animation_finished — one async block instead of connect_other
        // (backlog #5). The child AnimationPlayer is freed together with this node, so its
        // emitter is NOT guaranteed to outlive the wait if something frees Blast early: the
        // fallible future is required here (research.md R7), unlike part_disappear.rs's
        // SceneTreeTimer waits, whose emitter the SceneTree itself keeps alive.
        let mut this = self.to_gd();
        let animation_player = self.animation_player.clone();
        godot::task::spawn(async move {
            let result = animation_player
                .signals()
                .animation_finished()
                .to_fallible_future()
                .await;
            if result.is_err() || !this.is_instance_valid() {
                return;
            }
            this.queue_free();
        });
    }

    fn process(&mut self, _delta: f64) {
        if let Some(camera) = &self.camera
            && camera.is_instance_valid()
        {
            let origin = camera.get_global_transform().origin;
            self.light_rays.look_at(origin);
        }
    }
}
