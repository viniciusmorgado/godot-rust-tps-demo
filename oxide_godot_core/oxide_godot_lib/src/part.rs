use godot::classes::{
    CollisionShape3D, CpuParticles3D, IRigidBody3D, Material, MeshInstance3D,
    MultiplayerSynchronizer, Node, Os, PackedScene, RigidBody3D, ShaderMaterial,
};
use godot::global::randf;
use godot::prelude::*;

use pure::{fade_curve, random_angular_velocity, should_destroy, wait_time};

/// The fade/lifetime math (`v1`: `part.rs:54,57,90-93,95`) — all glam/std, no engine call,
/// verified against the 1.4.1 purity rule (research.md R1).
mod pure {
    use godot::prelude::*;

    /// v1: `part.rs:54`.
    pub fn fade_curve(counter: f32, disappearing_time: f32) -> f32 {
        (counter / disappearing_time).powi(2)
    }

    /// v1: `part.rs:57`.
    pub fn should_destroy(counter: f32, disappearing_time: f32) -> bool {
        counter >= disappearing_time - 0.2
    }

    /// v1: `part.rs:90-93`. Three already-sampled `[0.0, 1.0)` inputs; RNG sampling itself
    /// (`randf()`) stays in glue.
    pub fn random_angular_velocity(r1: f32, r2: f32, r3: f32) -> Vector3 {
        (Vector3::new(r1, r2, r3).normalized() * 2.0 - Vector3::ONE) * 10.0
    }

    /// v1: `part.rs:95`. One already-sampled `[0.0, 1.0)` input.
    pub fn wait_time(lifetime: f32, lifetime_random: f32, r: f32) -> f32 {
        lifetime + lifetime_random * r
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn fade_curve_is_the_squared_ratio() {
            assert_eq!(fade_curve(0.25, 0.5), 0.25);
            assert_eq!(fade_curve(0.5, 0.5), 1.0);
        }

        #[test]
        fn should_destroy_at_t_minus_0_2() {
            assert!(!should_destroy(0.29, 0.5));
            assert!(should_destroy(0.3, 0.5));
        }

        #[test]
        fn random_angular_velocity_reproduces_the_formula() {
            let v = random_angular_velocity(1.0, 1.0, 1.0);
            let n = 1.0_f32 / 3.0_f32.sqrt(); // Vector3::new(1,1,1).normalized() per axis
            let expected = (n * 2.0 - 1.0) * 10.0;
            assert!((v.x - expected).abs() < 1e-4);
            assert!((v.y - expected).abs() < 1e-4);
            assert!((v.z - expected).abs() < 1e-4);
        }

        #[test]
        fn random_angular_velocity_is_not_renormalized_after_the_shift() {
            // A markedly non-uniform sample: normalized() is (1,0,0), so the shifted result is
            // (10, -10, -10) -- length ~17.3, nowhere near a unit vector. If the implementation
            // accidentally renormalized afterward (v1 does NOT), this would fail.
            let v = random_angular_velocity(1.0, 0.0, 0.0);
            assert!((v.length() - 1.0).abs() > 1.0, "unexpectedly close to unit length: {v:?}");
        }

        #[test]
        fn wait_time_adds_the_random_component() {
            assert_eq!(wait_time(3.0, 3.0, 0.5), 4.5);
            assert_eq!(wait_time(3.0, 3.0, 0.0), 3.0);
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=RigidBody3D)]
pub struct Part {
    base: Base<RigidBody3D>,

    material: Option<Gd<Material>>,

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

    disappearing_counter: f32,

    #[init(node = "MultiplayerSynchronizer")]
    synchronizer: OnReady<Gd<MultiplayerSynchronizer>>,
    #[init(node = "Col1")]
    col1: OnReady<Gd<CollisionShape3D>>,
    #[init(node = "Col2")]
    col2: OnReady<Gd<CollisionShape3D>>,
    // The three Part instances wrap different models (PartShield1/2 -> part_shield.glb,
    // PartHead -> part_head.glb), so the Model node's first child's NAME differs per instance;
    // only the INDEX is stable (research.md R7) -- never a fixed #[init(node = "Model/<name>")].
    #[init(val = OnReady::from_base_fn(|base: &Gd<Node>| base
        .get_node_as::<Node>("Model")
        .get_child(0)
        .unwrap()
        .cast::<MeshInstance3D>()))]
    model_mesh: OnReady<Gd<MeshInstance3D>>,
    // was load::<PackedScene>(...) per destroy() call (V2-C's bullet_scene precedent).
    #[init(val = load("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn"))]
    part_disappear_scene: Gd<PackedScene>,
}

#[godot_api]
impl IRigidBody3D for Part {
    fn ready(&mut self) {
        self.base_mut().set_process(false);
        if !Os::singleton().has_feature("dedicated_server") {
            let mut mesh_inst = self.model_mesh.clone();
            let mesh = mesh_inst.get_mesh().unwrap();
            let mut mat: Gd<Material> = mesh.surface_get_material(0).unwrap().duplicate_resource();
            // upstream bug fix: part.gd installed the duplicated material on the Mesh resource shared by both shields
            // (surface_set_material), so the last shield to enter "won" and the other one's fade was never rendered; per-instance override.
            mesh_inst.set_surface_override_material(0, &mat);
            let next_pass: Gd<Material> = mat.get_next_pass().unwrap().duplicate_resource();
            mat.set_next_pass(&next_pass);
            self.material = Some(mat);
        }
    }

    fn process(&mut self, delta: f64) {
        let fade = fade_curve(self.disappearing_counter, self.disappearing_time);
        self.set_fade_value(fade);
        self.disappearing_counter += delta as f32;
        if should_destroy(self.disappearing_counter, self.disappearing_time) {
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
        if let Some(mat) = &self.material {
            mat.get_next_pass()
                .unwrap()
                .cast::<ShaderMaterial>()
                .set_shader_parameter("emission_cutout", &value.to_variant());
        }
    }

    #[func]
    pub(crate) fn explode(&mut self) {
        // Start synching.
        self.synchronizer.set_visibility_public(true);
        self.base_mut().set_freeze_enabled(false);
        if !self.base().get_multiplayer().unwrap().is_server() {
            return;
        }
        self.col1.set_disabled(false);
        self.col2.set_disabled(false);
        self.base_mut().set_linear_velocity(3.0 * Vector3::UP);
        let angular = random_angular_velocity(randf() as f32, randf() as f32, randf() as f32);
        self.base_mut().set_angular_velocity(angular);
        let wait = wait_time(self.lifetime, self.lifetime_random, randf() as f32);

        // backlog #5's async pattern (part_disappear.rs's established shape) instead of
        // connect_other.
        let mut this = self.to_gd();
        godot::task::spawn(async move {
            this.get_tree()
                .create_timer(wait as f64)
                .signals()
                .timeout()
                .to_future()
                .await;
            if !this.is_instance_valid() {
                return;
            }
            this.set_process(true);
        });
    }

    #[rpc(authority, call_local, unreliable)]
    fn destroy(&mut self) {
        let mut puff: Gd<CpuParticles3D> = self.part_disappear_scene.instantiate_as::<CpuParticles3D>();
        let mut parent = self.puff_parent();
        parent.add_child(&puff);
        let origin = self.base().get_global_transform().origin;
        puff.set_global_position(origin);

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
            this.queue_free();
        });
    }
}

impl Part {
    /// backlog #15: the puff's parent is the ROBOT's own parent, not `Death` (which decouples
    /// the puff's lifetime from the robot's 10s-after-death removal — research.md R7). The
    /// 3-hop walk from a `Part`: `Death` -> `EnemyRobot` -> the robot's own parent. Falls back
    /// to the LAST successfully-resolved ancestor if the chain is shorter (e.g. a standalone
    /// `Part` with no `Death`/`EnemyRobot` ancestors, as in the parity harness) -- never panics.
    fn puff_parent(&self) -> Gd<Node> {
        let death = self.base().get_parent();
        let robot = death.as_ref().and_then(|d| d.get_parent());
        let robots_parent = robot.as_ref().and_then(|r| r.get_parent());
        robots_parent
            .or(robot)
            .or(death)
            .expect("Part must have at least an immediate parent")
    }
}
