use godot::classes::{CpuParticles3D, ICpuParticles3D};
use godot::prelude::*;

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
        self.mini_blasts.set_emitting(true);

        // await get_tree().create_timer(0.2).timeout; await get_tree().create_timer(lifetime *
        // 2.0).timeout — one async block instead of nested connect_other chains (backlog #5).
        // `this` (Gd<Self>, not &mut self) is captured at spawn time; every step below is a
        // CpuParticles3D/Node engine method reached directly through Gd<Self>'s Deref to the
        // base class, so no bind_mut() is ever needed here — nothing custom to this type is
        // touched after ready() returns.
        let mut this = self.to_gd();
        godot::task::spawn(async move {
            this.get_tree()
                .create_timer(0.2)
                .signals()
                .timeout()
                .to_future()
                .await;
            if !this.is_instance_valid() {
                return;
            }
            this.set_emitting(true);
            let lifetime = this.get_lifetime();
            this.get_tree()
                .create_timer(lifetime * 2.0)
                .signals()
                .timeout()
                .to_future()
                .await;
            if !this.is_instance_valid() {
                return;
            }
            this.queue_free();
        });
    }
}
