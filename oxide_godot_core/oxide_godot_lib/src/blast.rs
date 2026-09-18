use godot::classes::{AnimationPlayer, CpuParticles3D, INode3D, Node3D};
use godot::prelude::*;

use crate::ecs::event::{InboundEvent, Initial};
use crate::ecs::{queue, Handles};

/// Bridge (constitution 1.5.1, "ECS shape (v3)"): a sync-only entity — no gameplay system. The
/// per-frame `look_at` that v2 ran in `process` is now `ecs::sync_in_blast` (camera origin →
/// `LookTarget`) + `ecs::sync_out_blast` (`look_at` when the target changed); the animation's end
/// reaches the ECS as `BlastAnimationFinished` → `Remove` → `ecs::sync_out_remove`.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct Blast {
    base: Base<Node3D>,
    #[init(node = "LightRays")]
    light_rays: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
}

#[godot_api]
impl INode3D for Blast {
    fn ready(&mut self) {
        // @onready var camera: Camera3D = get_tree().get_root().get_camera_3d() — resolved ONCE
        // (v2 `blast.rs:19`); `None` in a scene without a current camera.
        let camera = self.base().get_tree().get_root().unwrap().get_camera_3d();

        // The child emits the signal and this node receives it: `connect_other` (`connect_self`'s
        // receiver is the emitter). The strong `Gd<Blast>` it captures dies with the child.
        self.animation_player
            .signals()
            .animation_finished()
            .connect_other(&self.to_gd(), Self::_on_animation_finished);

        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Blast {
                root: self.to_gd().upcast::<Node3D>(),
                light_rays: self.light_rays.clone(),
                camera,
            },
            initial: Initial::Blast,
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}

#[godot_api]
impl Blast {
    /// Only pushes the event (the `StringName` is the signal's own parameter, unused).
    #[func]
    fn _on_animation_finished(&mut self, _name: StringName) {
        queue::push(InboundEvent::BlastAnimationFinished { id: self.base().instance_id() });
    }
}
