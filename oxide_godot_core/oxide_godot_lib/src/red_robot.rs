//! The robot's bridge (constitution 1.5.2 "ECS shape (v3)"; specs/013 contracts
//! enemy-entities.md §1): `ready` registers the entity with the twelve handles, the three parts,
//! the impact scene, the RID and the dedicated-server flag; `exit_tree` unregisters; the RPC,
//! method-track and area handlers push events. No per-tick logic: the tick is
//! `red_robot/system.rs` (pure) + `red_robot/sync.rs` (engine); `model.rs` is v2's pure core.

use godot::classes::{
    AnimationPlayer, AnimationTree, AudioStreamPlayer3D, BoneAttachment3D, CharacterBody3D,
    CollisionShape3D, CpuParticles3D, ICharacterBody3D, MeshInstance3D, Node3D, Os, PackedScene,
    RayCast3D,
};
use godot::prelude::*;

use crate::ecs::event::{InboundEvent, Initial, RobotFx};
use crate::ecs::{Handles, RobotHandles, queue};
use crate::part::Part;
use crate::player::Player;

pub(crate) mod model;
pub(crate) mod sync;
pub(crate) mod system;

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum State {
    Idle,
    Approach,
    Aim,
    Shooting,
}

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct EnemyRobot {
    base: Base<CharacterBody3D>,

    // Read once at registration (`Initial::Robot`); written by nothing at runtime.
    #[var]
    pub(crate) test_shoot: bool,

    // The replicated projection of the entity's components (`red_robot.tscn:27-42`), written
    // by the fixed `SyncOut` on the simulating peer and read back by the fixed `SyncIn` on the
    // others (`state`, `target_position`, `aim_preparing`); `dead` once, in the death branch.
    #[export]
    pub(crate) target_position: Vector3,
    #[export]
    #[init(val = 5)]
    pub(crate) health: i32,
    #[export]
    #[init(val = State::Idle)]
    pub(crate) state: State,
    #[export]
    pub(crate) dead: bool,
    #[var]
    #[init(val = model::RobotTuning::default().aim_prepare_time)]
    pub(crate) aim_preparing: f32,

    // was Os::singleton().has_feature("dedicated_server") read every frame by the laser clip write.
    #[init(val = Os::singleton().has_feature("dedicated_server"))]
    is_dedicated_server: bool,
    // was self.base().get_rid() re-read at all three raycast call sites.
    #[init(val = Rid::Invalid)]
    rid: Rid,
    // was load::<PackedScene>(...) per shot (V2-C's bullet_scene precedent — no tree dependency).
    #[init(val = load("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn"))]
    impact_effect_scene: Gd<PackedScene>,

    #[init(node = "AnimationTree")]
    animation_tree: OnReady<Gd<AnimationTree>>,
    #[init(node = "ShootAnimation")]
    shoot_animation: OnReady<Gd<AnimationPlayer>>,

    #[init(node = "RedRobotModel")]
    model: OnReady<Gd<Node3D>>,
    #[init(node = "RedRobotModel/Armature/Skeleton3D/RayFrom")]
    ray_from: OnReady<Gd<BoneAttachment3D>>,
    #[init(node = "RedRobotModel/Armature/Skeleton3D/RayFrom/RayMesh")]
    ray_mesh: OnReady<Gd<MeshInstance3D>>,
    #[init(node = "RedRobotModel/Armature/Skeleton3D/RayFrom/RayCast")]
    laser_raycast: OnReady<Gd<RayCast3D>>,
    #[init(node = "RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber")]
    laser_ember: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "CollisionShape3D")]
    collision_shape: OnReady<Gd<CollisionShape3D>>,

    #[init(node = "SoundEffects/Explosion")]
    explosion_sound: OnReady<Gd<AudioStreamPlayer3D>>,
    #[init(node = "SoundEffects/Hit")]
    hit_sound: OnReady<Gd<AudioStreamPlayer3D>>,

    #[init(node = "Death")]
    death: OnReady<Gd<Node3D>>,
    #[init(node = "Death/PartShield1")]
    death_shield1: OnReady<Gd<Part>>,
    #[init(node = "Death/PartShield2")]
    death_shield2: OnReady<Gd<Part>>,
    #[init(node = "Death/PartHead")]
    death_head: OnReady<Gd<Part>>,
    #[init(node = "Death/DetachSpark1")]
    death_detach_spark1: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "Death/DetachSpark2")]
    death_detach_spark2: OnReady<Gd<CpuParticles3D>>,
}

#[godot_api]
impl ICharacterBody3D for EnemyRobot {
    /// v2 `red_robot.rs:110-125` minus `animate(0.0)` (the first fixed run writes the parameters
    /// before the first `advance`, research R1) and the `test_shoot` countdown (registration
    /// zeroes it); the tree's `aim/blend_position` is read ONCE into `AimBlend` (spec FR-016).
    fn ready(&mut self) {
        let mut orientation = self.base().get_global_transform();
        orientation.origin = Vector3::ZERO;
        self.animation_tree.set_active(true);
        self.rid = self.base().get_rid();

        if self.dead {
            self.model.set_visible(false);
            self.collision_shape.set_disabled(true);
            self.animation_tree.set_active(false);
        }

        let aim_blend = self.animation_tree.get("parameters/aim/blend_position").to::<Vector2>();
        let simulates = self.base().get_multiplayer().unwrap().is_server();
        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Robot(Box::new(RobotHandles {
                root: self.to_gd(),
                anim_tree: self.animation_tree.clone(),
                shoot_anim: self.shoot_animation.clone(),
                model: self.model.clone(),
                ray_from: self.ray_from.clone(),
                ray_mesh: self.ray_mesh.clone(),
                laser_raycast: self.laser_raycast.clone(),
                laser_ember: self.laser_ember.clone(),
                collision_shape: self.collision_shape.clone(),
                explosion_sound: self.explosion_sound.clone(),
                hit_sound: self.hit_sound.clone(),
                death: self.death.clone(),
                parts: [self.death_shield1.clone(), self.death_shield2.clone(), self.death_head.clone()],
                sparks: [self.death_detach_spark1.clone(), self.death_detach_spark2.clone()],
                impact_effect_scene: self.impact_effect_scene.clone(),
                rid: self.rid,
                is_dedicated_server: self.is_dedicated_server,
            })),
            initial: Initial::Robot {
                state: self.state,
                health: self.health,
                dead: self.dead,
                test_shoot: self.test_shoot,
                orientation,
                aim_blend,
                simulates,
            },
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}

#[godot_api]
impl EnemyRobot {
    /// Emitted by `sync_out_robot`'s death branch (and by the remote death on clients) for
    /// `level.rs`'s respawn.
    #[signal]
    pub(crate) fn exploded();

    /// The shoot animation's method track (`red_robot.tscn:10297`; v2 `:267-274`): the drain
    /// applies `Approach` + `resume_approach_reset`.
    #[func]
    fn resume_approach(&mut self) {
        queue::push(InboundEvent::ResumeApproachRequested { root_id: self.base().instance_id() });
    }

    /// Remote peers only (spec option (B)): the simulating peer applies the hit in the same fixed
    /// run through `RobotHitLocal`; here the hit is counted for this peer's frame run, which runs
    /// v2's client-side handler on this peer's own health.
    #[rpc(authority, call_remote, unreliable)]
    fn hit(&mut self) {
        queue::push(InboundEvent::RobotHit { root_id: self.base().instance_id() });
    }

    /// Remote peers only (spec FR-004, option (b)): the simulating peer plays inline.
    #[rpc(authority, call_remote, unreliable)]
    fn play_shoot(&mut self) {
        queue::push(InboundEvent::RobotFx { root_id: self.base().instance_id(), fx: RobotFx::PlayShoot });
    }

    /// The shoot animation's method track (`red_robot.tscn:10294`; v2 `:335-338`): the marker
    /// the next fixed run's `robot_decide` consumes.
    #[func]
    fn shoot_check(&mut self) {
        queue::push(InboundEvent::ShootRequested { root_id: self.base().instance_id() });
    }

    /// backlog #16: the dead `|| body.get_name() == "Target"` branch is removed — no `.tscn`
    /// in the project has ever had a node named "Target" (grep-confirmed, research.md
    /// Context). `try_cast` happens once, here, at the boundary; the entity tracks an id.
    #[func]
    fn _on_area_body_entered(&mut self, body: Gd<Node3D>) {
        if let Ok(player) = body.try_cast::<Player>() {
            queue::push(InboundEvent::RobotPlayerSeen {
                root_id: self.base().instance_id(),
                player: Some(player.instance_id()),
            });
        }
    }

    #[func]
    fn _on_area_body_exited(&mut self, body: Gd<Node3D>) {
        if body.try_cast::<Player>().is_ok() {
            queue::push(InboundEvent::RobotPlayerSeen { root_id: self.base().instance_id(), player: None });
        }
    }
}
