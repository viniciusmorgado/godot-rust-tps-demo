use godot::classes::{CpuParticles3D, ICpuParticles3D};
use godot::prelude::*;

use crate::ecs::event::{InboundEvent, Initial};
use crate::ecs::{queue, Handles};

pub(crate) mod system;

/// Bridge (constitution 1.5.1, "ECS shape (v3)"): `ready` does v2's one-shot setup and registers
/// the entity, `exit_tree` unregisters it. The two waits of v2's async block are
/// `part_disappear::system::DisappearPhase`, stepped by the frame schedule; `emitting = true`
/// and `queue_free()` are written by the sync layer (`ecs::sync_out_puff`, `ecs::sync_out_remove`).
#[derive(GodotClass)]
#[class(init, base=CpuParticles3D)]
pub struct PartDisappear {
    base: Base<CpuParticles3D>,
    #[init(node = "MiniBlasts")]
    mini_blasts: OnReady<Gd<CpuParticles3D>>,
}

#[godot_api]
impl ICpuParticles3D for PartDisappear {
    fn ready(&mut self) {
        // v2 `part_disappear.rs:15`: one-shot engine setup at the same moment, not per-frame logic.
        self.mini_blasts.set_emitting(true);

        let lifetime = self.base().get_lifetime();
        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Puff { root: self.to_gd().upcast::<CpuParticles3D>() },
            initial: Initial::Puff { lifetime },
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}
