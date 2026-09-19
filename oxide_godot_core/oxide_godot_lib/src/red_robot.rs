use godot::classes::{
    AnimationPlayer, AnimationTree, AudioStreamPlayer3D, BoneAttachment3D, CharacterBody3D,
    CollisionShape3D, CpuParticles3D, ICharacterBody3D, MeshInstance3D, Node3D, Object, Os,
    PackedScene, PhysicsRayQueryParameters3D, RayCast3D, ShaderMaterial,
};
use godot::global::randi;
use godot::prelude::*;

use crate::part::Part;
use crate::player::Player;

pub(crate) mod model;

/// The one raycast helper's typed result, replacing three duplicated raw `VarDictionary`
/// inspections. Glue-only (holds `Gd<Object>`) — not part of `red_robot::model`'s pure surface.
struct RayHit {
    position: Vector3,
    collider: Option<Gd<Object>>,
}

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

    #[var]
    test_shoot: bool,

    #[export]
    target_position: Vector3,
    #[export]
    #[init(val = 5)]
    health: i32,
    #[export]
    #[init(val = State::Idle)]
    state: State,
    #[export]
    dead: bool,
    #[var]
    #[init(val = model::RobotTuning::default().aim_prepare_time)]
    aim_preparing: f32,

    #[init(val = model::RobotTuning::default().shoot_wait)]
    shoot_countdown: f32,
    #[init(val = model::RobotTuning::default().aim_time)]
    aim_countdown: f32,

    // backlog #16: typed at the boundary (_on_area_body_entered), no try_cast needed elsewhere.
    player: Option<Gd<Player>>,
    orientation: Transform3D,

    // was Os::singleton().has_feature("dedicated_server") read every frame in _clip_ray.
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
    fn ready(&mut self) {
        self.orientation = self.base().get_global_transform();
        self.orientation.origin = Vector3::ZERO;
        self.animation_tree.set_active(true);
        self.rid = self.base().get_rid();
        if self.test_shoot {
            self.shoot_countdown = 0.0;
        }

        if self.dead {
            self.model.set_visible(false);
            self.collision_shape.set_disabled(true);
            self.animation_tree.set_active(false);
        }

        self.animate(0.0);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.dead {
            return;
        }

        if !self.base().get_multiplayer().unwrap().is_server() {
            self.animate(delta);
            return;
        }

        if self.test_shoot {
            self.shoot();
            self.test_shoot = false;
        }

        let Some(player) = self.player.clone() else {
            self.target_position = Vector3::ZERO;
            self.animate(delta);
            let velocity = model::idle_velocity(self.base().get_gravity(), delta as f32);
            self.base_mut().set_velocity(velocity);
            self.base_mut().set_up_direction(Vector3::UP);
            self.base_mut().move_and_slide();
            return;
        };

        self.target_position = player.get_global_transform().origin;

        let dt = delta as f32;
        let tuning = model::RobotTuning::default();
        // v1's if/else-if (:140-234) is ONE mutually-exclusive decision keyed to the state at
        // the START of this frame — capture it once, since `step` below may reassign `self.state`
        // before this function ends (and `animate`, further down, must see the NEW state).
        let state_at_frame_start = self.state;

        if state_at_frame_start == State::Approach {
            // v1:141-146
            let gt = self.base().get_global_transform();
            let local: Vector3 = gt.basis.transposed() * (self.target_position - gt.origin);
            let angle = model::angle_to_player(local);

            let mut counters = model::RobotCounters {
                aim_preparing: self.aim_preparing,
                shoot_countdown: self.shoot_countdown,
                aim_countdown: self.aim_countdown,
            };

            // research.md R3: raycast only when facing AND the countdown is about to expire —
            // the same gate v1 applies before ever decrementing shoot_countdown (:152-157).
            let mut sees_player = None;
            if model::facing(angle, tuning.player_aim_tolerance)
                && model::shoot_countdown_will_expire(counters.shoot_countdown, dt)
            {
                let ray_origin = self.ray_from.get_global_transform().origin;
                let ray_to = player.get_global_transform().origin + Vector3::UP;
                let hit = self.raycast_to(ray_origin, ray_to);
                sees_player = Some(hits_player(&hit, &player));
            }

            let inputs = model::RobotInputs {
                angle_to_player: Some(angle),
                sees_player,
            };
            let (new_state, cmds) =
                model::step(state_at_frame_start, &mut counters, dt, &inputs, &tuning);
            self.state = new_state;
            self.aim_preparing = counters.aim_preparing;
            self.shoot_countdown = counters.shoot_countdown;
            self.aim_countdown = counters.aim_countdown;
            self.apply_cmds(cmds);
        } else if state_at_frame_start == State::Aim || state_at_frame_start == State::Shooting {
            // v1:191-197 — laser clip, unconditional within this branch.
            let mut max_dist: f32 = 1000.0;
            if self.laser_raycast.is_colliding() {
                max_dist = (self.ray_from.get_global_transform().origin
                    - self.laser_raycast.get_collision_point())
                .length();
            }
            self._clip_ray(max_dist);

            let mut counters = model::RobotCounters {
                aim_preparing: self.aim_preparing,
                shoot_countdown: self.shoot_countdown,
                aim_countdown: self.aim_countdown,
            };

            // research.md R3: raycast only in Aim, only when aim_countdown is about to expire —
            // v1's `:206` gate excludes Shooting explicitly.
            let mut sees_player = None;
            if state_at_frame_start == State::Aim
                && model::aim_countdown_will_expire(counters.aim_countdown, dt)
            {
                let ray_origin = self.ray_from.get_global_transform().origin;
                let ray_to = self.target_position + Vector3::UP;
                let hit = self.raycast_to(ray_origin, ray_to);
                sees_player = Some(hits_player(&hit, &player));
            }

            let inputs = model::RobotInputs {
                angle_to_player: None,
                sees_player,
            };
            let (new_state, cmds) =
                model::step(state_at_frame_start, &mut counters, dt, &inputs, &tuning);
            self.state = new_state;
            self.aim_preparing = counters.aim_preparing;
            self.shoot_countdown = counters.shoot_countdown;
            self.aim_countdown = counters.aim_countdown;
            self.apply_cmds(cmds);
        }

        self.animate(delta);

        // Root motion (research.md R6 — the integrate_root_motion twin bundles v1's :239-257).
        let root_motion = Transform3D::new(
            Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
            self.animation_tree.get_root_motion_position(),
        );
        let (new_orientation, velocity) = model::integrate_root_motion(
            self.orientation,
            root_motion,
            dt,
            self.base().get_gravity(),
            self.base().get_velocity(),
        );
        self.orientation = new_orientation;
        self.base_mut().set_velocity(velocity);
        self.base_mut().set_up_direction(Vector3::UP);
        self.base_mut().move_and_slide();

        let basis = self.orientation.basis;
        self.base_mut().set_global_basis(basis);
    }
}

#[godot_api]
impl EnemyRobot {
    #[signal]
    pub(crate) fn exploded();

    #[func]
    fn resume_approach(&mut self) {
        let tuning = model::RobotTuning::default();
        let (aim_preparing, shoot_countdown) = model::resume_approach_reset(&tuning);
        self.state = State::Approach;
        self.aim_preparing = aim_preparing;
        self.shoot_countdown = shoot_countdown;
    }

    #[rpc(authority, call_local, unreliable)]
    fn hit(&mut self) {
        if self.dead {
            return;
        }
        let tuning = model::RobotTuning::default();

        // Hit reaction (RNG anim pick + sound) fires on EVERY live hit, before the decrement —
        // spec US1 Acceptance Scenario 11, red_robot.rs:281-283.
        let param = format!("parameters/hit{}/request", randi() % 3 + 1);
        self.animation_tree.set(&param, &1.to_variant());
        self.hit_sound.play();

        let (new_health, just_died) = model::hit_step(self.health);
        self.health = new_health;

        if just_died {
            self.dead = true;
            self.animation_tree.set_active(false);
            self.model.set_visible(false);
            self.death.set_visible(true);
            self.collision_shape.set_disabled(true);

            self.death_detach_spark1.set_emitting(true);
            self.death_detach_spark2.set_emitting(true);

            self.death_shield1.bind_mut().explode();
            self.death_shield2.bind_mut().explode();
            self.death_head.bind_mut().explode();

            self.explosion_sound.play();
            self.signals().exploded().emit();

            if self.base().get_multiplayer().unwrap().is_server() {
                // backlog #17: godot::task::spawn + SceneTreeTimer::to_future() instead of
                // connect_other (part_disappear.rs's established async pattern).
                let mut this = self.to_gd();
                let removal_delay = tuning.removal_delay as f64;
                godot::task::spawn(async move {
                    this.get_tree()
                        .create_timer(removal_delay)
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
    }

    #[rpc(authority, call_local, unreliable)]
    fn play_shoot(&mut self) {
        self.shoot_animation.play_ex().name("shoot").done();
    }

    #[func]
    fn shoot_check(&mut self) {
        self.test_shoot = true;
    }

    #[func]
    fn _on_area_body_entered(&mut self, body: Gd<Node3D>) {
        // backlog #16: the dead `|| body.get_name() == "Target"` branch is removed — no `.tscn`
        // in the project has ever had a node named "Target" (grep-confirmed, research.md
        // Context). `try_cast` happens once, here, at the boundary; `self.player` is typed.
        if let Ok(player) = body.try_cast::<Player>() {
            self.player = Some(player);
            self.state = State::Approach;
        }
    }

    #[func]
    fn _on_area_body_exited(&mut self, body: Gd<Node3D>) {
        if body.try_cast::<Player>().is_ok() {
            self.player = None;
            self.state = State::Idle;
        }
    }
}

impl EnemyRobot {
    /// The ONE raycast helper (research.md R4/FR-005), replacing three duplicated
    /// `PhysicsRayQueryParameters3D`/`intersect_ray` blocks.
    fn raycast_to(&self, from: Vector3, to: Vector3) -> Option<RayHit> {
        let params = PhysicsRayQueryParameters3D::create_ex(from, to)
            .collision_mask(0xFFFFFFFF)
            .exclude(&array![self.rid])
            .done()
            .unwrap();
        let col: VarDictionary = self
            .base()
            .get_world_3d()
            .unwrap()
            .get_direct_space_state()
            .unwrap()
            .intersect_ray(&params);
        if col.is_empty() {
            return None;
        }
        let position = col.get("position").unwrap().to::<Vector3>();
        let collider = col.get("collider").and_then(|v| v.try_to::<Gd<Object>>().ok());
        Some(RayHit { position, collider })
    }

    fn apply_cmds(&mut self, cmds: Vec<model::Cmd>) {
        for cmd in cmds {
            match cmd {
                model::Cmd::RpcPlayShoot => {
                    self.base_mut().rpc("play_shoot", &[]);
                }
                model::Cmd::ResumeApproach => {
                    self.resume_approach();
                }
            }
        }
    }

    fn shoot(&mut self) {
        let tuning = model::RobotTuning::default();
        let gt: Transform3D = self.ray_from.get_global_transform();
        let ray_origin: Vector3 = gt.origin;
        // The RayCast3D is rotated 90 degrees inside the BoneAttachment3D.
        let ray_dir: Vector3 = gt.basis.col_b();
        let default_max_dist: f32 = 1000.0;

        let hit = self.raycast_to(ray_origin, ray_origin + ray_dir * default_max_dist);
        let max_dist = hit
            .as_ref()
            .map(|h| ray_origin.distance_to(h.position))
            .unwrap_or(default_max_dist);

        // Clip ray in shader.
        self._clip_ray(max_dist);

        // Position laser ember particles.
        let mesh_offset: f32 = self.ray_mesh.get_position().z;
        self.laser_ember.set_position(model::ember_position(max_dist, mesh_offset));
        let extents = self.laser_ember.get_emission_box_extents();
        self.laser_ember
            .set_emission_box_extents(model::ember_extents(extents, max_dist, mesh_offset));

        let Some(hit) = hit else { return };

        let mut blast: Gd<Node3D> = self.impact_effect_scene.instantiate_as::<Node3D>();
        self.base().get_tree().get_root().unwrap().add_child(&blast);
        blast.set_global_position(hit.position);

        let Some(player) = self.player.clone() else { return };
        let hit_player = hit
            .collider
            .as_ref()
            .map(|c| c.instance_id() == player.instance_id())
            .unwrap_or(false);
        if !hit_player {
            return;
        }

        // backlog #17: async pattern instead of connect_other; guard both handles after the
        // await (the player can disconnect/despawn independently of the robot).
        let this = self.to_gd();
        let trauma_delay = tuning.trauma_delay as f64;
        let trauma_amount = tuning.trauma_amount;
        godot::task::spawn(async move {
            this.get_tree()
                .create_timer(trauma_delay)
                .signals()
                .timeout()
                .to_future()
                .await;
            if !this.is_instance_valid() || !player.is_instance_valid() {
                return;
            }
            player.clone().bind_mut().add_camera_shake_trauma(trauma_amount);
        });
    }

    fn animate(&mut self, delta: f64) {
        let tuning = model::RobotTuning::default();
        let dt = delta as f32;

        let angle_to_player = if self.state == State::Approach {
            let gt = self.base().get_global_transform();
            let local: Vector3 = gt.basis.transposed() * (self.target_position - gt.origin);
            Some(model::angle_to_player(local))
        } else {
            None
        };
        let target_is_zero = self.target_position == Vector3::ZERO;
        let request = model::transition_request(self.state, angle_to_player, target_is_zero, &tuning);
        self.animation_tree
            .set("parameters/state/transition_request", &request.to_variant());

        // Aiming or shooting.
        if self.target_position != Vector3::ZERO {
            let blend_amount = model::aim_blend_amount(self.aim_preparing, &tuning);
            self.animation_tree
                .set("parameters/aiming/blend_amount", &blend_amount.to_variant());

            let mt = self.ray_mesh.get_global_transform();
            let to_cannon_local: Vector3 =
                mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin);
            let (h_angle, v_angle) = model::cannon_angles(to_cannon_local);
            let blend_pos: Vector2 = self
                .animation_tree
                .get("parameters/aim/blend_position")
                .to::<Vector2>();
            let blend_pos = model::aim_blend_step(blend_pos, h_angle, v_angle, dt, &tuning);
            self.animation_tree
                .set("parameters/aim/blend_position", &blend_pos.to_variant());
        }
    }

    fn _clip_ray(&mut self, length: f32) {
        let mesh_offset: f32 = self.ray_mesh.get_position().z;
        if !self.is_dedicated_server {
            self.ray_mesh
                .get_surface_override_material(0)
                .unwrap()
                .cast::<ShaderMaterial>()
                .set_shader_parameter("clip", &(length + mesh_offset).to_variant());
        }
    }
}

/// v1's repeated `col.get("collider").and_then(...).map(|c| c.instance_id() ==
/// player.instance_id()).unwrap_or(false)` pattern, now against `RayHit`'s typed `collider`.
fn hits_player(hit: &Option<RayHit>, player: &Gd<Player>) -> bool {
    hit.as_ref()
        .and_then(|h| h.collider.as_ref())
        .map(|c| c.instance_id() == player.instance_id())
        .unwrap_or(false)
}
