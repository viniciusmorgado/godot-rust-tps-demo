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
        // await get_tree().create_timer(0.2).timeout
        let timer = self.base().get_tree().create_timer(0.2);
        timer
            .signals()
            .timeout()
            .connect_other(&*self, |this: &mut PartDisappear| {
                this.base_mut().set_emitting(true);
                // await get_tree().create_timer(lifetime * 2.0).timeout
                let lifetime = this.base().get_lifetime();
                let timer = this.base().get_tree().create_timer(lifetime * 2.0);
                timer
                    .signals()
                    .timeout()
                    .connect_other(&*this, |this: &mut PartDisappear| {
                        this.base_mut().queue_free();
                    });
            });
    }
}
