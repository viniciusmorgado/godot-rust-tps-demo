use godot::classes::{
    CollisionShape3D, CpuParticles3D, IRigidBody3D, Material, MeshInstance3D,
    MultiplayerSynchronizer, Node, Os, PackedScene, RigidBody3D, ShaderMaterial,
};
use godot::global::randf;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=RigidBody3D)]
pub struct Part {
    base: Base<RigidBody3D>,

    _mat: Option<Gd<Material>>,

    #[export]
    #[init(val = 3.0)]
    lifetime: f32,
    #[export]
    #[init(val = 3.0)]
    lifetime_random: f32,
    #[export]
    #[init(val = 0.5)]
    disappearing_time: f32,
    #[export]
    #[var(set = set_fade_value)]
    fade_value: f32,

    _disappearing_counter: f32,
}

#[godot_api]
impl IRigidBody3D for Part {
    fn ready(&mut self) {
        self.base_mut().set_process(false);
        if !Os::singleton().has_feature("dedicated_server") {
            let mut mesh_inst = self
                .base()
                .get_node_as::<Node>("Model")
                .get_child(0)
                .unwrap()
                .cast::<MeshInstance3D>();
            let mesh = mesh_inst.get_mesh().unwrap();
            let mut mat: Gd<Material> = mesh.surface_get_material(0).unwrap().duplicate_resource();
            // upstream bug fix: part.gd installed the duplicated material on the Mesh resource shared by both shields
            // (surface_set_material), so the last shield to enter "won" and the other one's fade was never rendered; per-instance override.
            mesh_inst.set_surface_override_material(0, &mat);
            let next_pass: Gd<Material> = mat.get_next_pass().unwrap().duplicate_resource();
            mat.set_next_pass(&next_pass);
            self._mat = Some(mat);
        }
    }

    fn process(&mut self, delta: f64) {
        let fade = (self._disappearing_counter / self.disappearing_time).powi(2);
        self.set_fade_value(fade);
        self._disappearing_counter += delta as f32;
        if self._disappearing_counter >= self.disappearing_time - 0.2 {
            self.base_mut().rpc("destroy", &[]);
            self.base_mut().set_process(false);
        }
    }
}

#[godot_api]
impl Part {
    #[func]
    fn set_fade_value(&mut self, value: f32) {
        self.fade_value = value;
        if let Some(mat) = &self._mat {
            mat.get_next_pass()
                .unwrap()
                .cast::<ShaderMaterial>()
                .set_shader_parameter("emission_cutout", &value.to_variant());
        }
    }

    #[func]
    pub(crate) fn explode(&mut self) {
        // Start synching.
        self.base()
            .get_node_as::<MultiplayerSynchronizer>("MultiplayerSynchronizer")
            .set_visibility_public(true);
        self.base_mut().set_freeze_enabled(false);
        if !self.base().get_multiplayer().unwrap().is_server() {
            return;
        }
        self.base().get_node_as::<CollisionShape3D>("Col1").set_disabled(false);
        self.base().get_node_as::<CollisionShape3D>("Col2").set_disabled(false);
        self.base_mut().set_linear_velocity(3.0 * Vector3::UP);
        let angular = (Vector3::new(randf() as f32, randf() as f32, randf() as f32).normalized()
            * 2.0
            - Vector3::ONE)
            * 10.0;
        self.base_mut().set_angular_velocity(angular);
        let wait = self.lifetime + self.lifetime_random * randf() as f32;
        self.base()
            .get_tree()
            .create_timer(wait as f64)
            .signals()
            .timeout()
            .connect_other(&*self, |this: &mut Part| this.base_mut().set_process(true));
    }

    #[rpc(authority, call_local, unreliable)]
    fn destroy(&mut self) {
        let mut puff: Gd<CpuParticles3D> = load::<PackedScene>(
            "res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn",
        )
        .instantiate_as::<CpuParticles3D>();
        self.base().get_parent().unwrap().add_child(&puff);
        let origin = self.base().get_global_transform().origin;
        puff.set_global_position(origin);
        self.base()
            .get_tree()
            .create_timer(0.2)
            .signals()
            .timeout()
            .connect_other(&*self, |this: &mut Part| this.base_mut().queue_free());
    }
}
