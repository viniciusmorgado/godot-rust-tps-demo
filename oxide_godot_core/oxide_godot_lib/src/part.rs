//! The part's bridge (constitution 1.5.2 "ECS shape (v3)"; specs/013 contracts
//! enemy-entities.md §1): a scene child of the robot with its own class and synchronizer, so it
//! registers ITSELF in `ready` (it outlives the robot's death sequence, replicates and is freed
//! independently), unregisters in `exit_tree`, and its remote `destroy` handler pushes. No
//! per-frame logic: the phase machine is `part/system.rs` (pure) + `part/sync.rs` (engine).
//! `explode` is gone — the robot's `SyncOut` explodes the parts directly (research R5).

use godot::classes::{
    CollisionShape3D, IRigidBody3D, Material, MeshInstance3D, MultiplayerSynchronizer, Node, Os,
    PackedScene, RigidBody3D, ShaderMaterial,
};
use godot::prelude::*;

use crate::ecs::event::{InboundEvent, Initial, PartFx};
use crate::ecs::{Handles, PartHandles, queue};

pub(crate) mod sync;
pub(crate) mod system;

/// The fade/lifetime math (`v1`: `part.rs:54,57,90-93,95`) — all glam/std, no engine call,
/// verified against the 1.4.1 purity rule (research.md R1).
pub(crate) mod pure {
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
    /// v2 `part.rs:123-136` minus `set_process(false)` (there is no callback), then the
    /// registration with the three exports and `simulates = is_server()` (`:167`).
    fn ready(&mut self) {
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

        let simulates = self.base().get_multiplayer().unwrap().is_server();
        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Part(Box::new(PartHandles {
                root: self.to_gd(),
                synchronizer: self.synchronizer.clone(),
                col1: self.col1.clone(),
                col2: self.col2.clone(),
                puff_scene: self.part_disappear_scene.clone(),
            })),
            initial: Initial::Part {
                lifetime: self.lifetime,
                lifetime_random: self.lifetime_random,
                disappearing_time: self.disappearing_time,
                simulates,
            },
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}

#[godot_api]
impl Part {
    /// The projection setter (`fade_value`): written by `sync_out_part` on the server and by the
    /// engine's replication on the clients — the shader write stays inside the bridge.
    #[func]
    pub(crate) fn set_fade_value(&mut self, value: f32) {
        self.fade_value = value;
        if let Some(mat) = &self.material {
            mat.get_next_pass()
                .unwrap()
                .cast::<ShaderMaterial>()
                .set_shader_parameter("emission_cutout", &value.to_variant());
        }
    }

    /// Remote peers only (spec FR-004, option (b)): the server instances the puff inline in its
    /// frame `SyncOut`; here the effect is queued for this peer's frame run (`sync_out_part`).
    #[rpc(authority, call_remote, unreliable)]
    fn destroy(&mut self) {
        queue::push(InboundEvent::PartFx { root_id: self.base().instance_id(), fx: PartFx::Destroy });
    }
}
