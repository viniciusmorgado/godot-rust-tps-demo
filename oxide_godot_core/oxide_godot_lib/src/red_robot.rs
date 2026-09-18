use godot::classes::{
    AnimationPlayer, AnimationTree, AudioStreamPlayer3D, BoneAttachment3D, CharacterBody3D,
    CollisionShape3D, CpuParticles3D, ICharacterBody3D, MeshInstance3D, Node3D, Os, PackedScene,
    PhysicsRayQueryParameters3D, RayCast3D, ShaderMaterial,
};
use godot::global::randi;
use godot::prelude::*;

use crate::part::Part;
use crate::player::Player;

mod model;

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum State {
    Idle,
    Approach,
    Aim,
    Shooting,
}

const PLAYER_AIM_TOLERANCE_DEGREES: f32 = 15.0_f32.to_radians();

const SHOOT_WAIT: f32 = 6.0;
const AIM_TIME: f32 = 1.0;

const AIM_PREPARE_TIME: f32 = 0.5;
const BLEND_AIM_SPEED: f32 = 0.05;

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct EnemyRobot {
    base: Base<CharacterBody3D>,

    #[export]
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
    #[export]
    #[init(val = AIM_PREPARE_TIME)]
    aim_preparing: f32,

    #[init(val = SHOOT_WAIT)]
    shoot_countdown: f32,
    #[init(val = AIM_TIME)]
    aim_countdown: f32,

    player: Option<Gd<Node3D>>,
    orientation: Transform3D,

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
            let gravity_velocity = self.base().get_gravity() * delta as f32;
            self.base_mut().set_velocity(gravity_velocity);
            self.base_mut().set_up_direction(Vector3::UP);
            self.base_mut().move_and_slide();
            return;
        };

        self.target_position = player.get_global_transform().origin;

        if self.state == State::Approach {
            if self.aim_preparing > 0.0 {
                self.aim_preparing -= delta as f32;
                if self.aim_preparing < 0.0 {
                    self.aim_preparing = 0.0;
                }
            }

            let gt = self.base().get_global_transform();
            let to_player_local: Vector3 = gt.basis.transposed() * (self.target_position - gt.origin);
            // The front of the robot is +Z, and atan2 is zero at +X, so we need to use the Z for the X parameter (second one).
            let angle_to_player: f32 = to_player_local.x.atan2(to_player_local.z);
            if angle_to_player > -PLAYER_AIM_TOLERANCE_DEGREES
                && angle_to_player < PLAYER_AIM_TOLERANCE_DEGREES
            {
                // Facing player, try to shoot.
                self.shoot_countdown -= delta as f32;
                if self.shoot_countdown < 0.0 {
                    // See if player can be killed because in they're sight.
                    let ray_origin = self.ray_from.get_global_transform().origin;
                    let ray_to = player.get_global_transform().origin + Vector3::UP; // Above middle of player.
                    let rid = self.base().get_rid();
                    let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, ray_to)
                        .collision_mask(0xFFFFFFFF)
                        .exclude(&array![rid])
                        .done();
                    let col: VarDictionary = self
                        .base()
                        .get_world_3d()
                        .unwrap()
                        .get_direct_space_state()
                        .unwrap()
                        .intersect_ray(&params.unwrap());
                    let hit_player = !col.is_empty()
                        && col
                            .get("collider")
                            .and_then(|v| v.try_to::<Gd<Object>>().ok())
                            .map(|c| c.instance_id() == player.instance_id())
                            .unwrap_or(false);

                    if hit_player {
                        self.state = State::Aim;
                        self.aim_countdown = AIM_TIME;
                        self.aim_preparing = 0.0;
                    } else {
                        // Player not in sight, do nothing.
                        self.shoot_countdown = SHOOT_WAIT;
                    }
                }
            }
        } else if self.state == State::Aim || self.state == State::Shooting {
            let mut max_dist: f32 = 1000.0;
            if self.laser_raycast.is_colliding() {
                max_dist = (self.ray_from.get_global_transform().origin
                    - self.laser_raycast.get_collision_point())
                .length();
            }
            self._clip_ray(max_dist);
            if self.aim_preparing < AIM_PREPARE_TIME {
                self.aim_preparing += delta as f32;
                if self.aim_preparing > AIM_PREPARE_TIME {
                    self.aim_preparing = AIM_PREPARE_TIME;
                }
            }

            self.aim_countdown -= delta as f32;
            if self.aim_countdown < 0.0 && self.state == State::Aim {
                let ray_origin: Vector3 = self.ray_from.get_global_transform().origin;
                let ray_to: Vector3 = self.target_position + Vector3::UP;
                let rid = self.base().get_rid();
                let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, ray_to)
                    .collision_mask(0xFFFFFFFF)
                    .exclude(&array![rid])
                    .done();
                let col: VarDictionary = self
                    .base()
                    .get_world_3d()
                    .unwrap()
                    .get_direct_space_state()
                    .unwrap()
                    .intersect_ray(&params.unwrap());
                let hit_player = !col.is_empty()
                    && col
                        .get("collider")
                        .and_then(|v| v.try_to::<Gd<Object>>().ok())
                        .map(|c| c.instance_id() == player.instance_id())
                        .unwrap_or(false);
                if hit_player {
                    self.state = State::Shooting;
                    self.shoot_countdown = SHOOT_WAIT;
                    self.base_mut().rpc("play_shoot", &[]);
                } else {
                    self.resume_approach();
                }
            }
        }

        self.animate(delta);
        // Apply root motion to orientation.
        self.orientation = self.orientation
            * Transform3D::new(
                Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
                self.animation_tree.get_root_motion_position(),
            );

        let h_velocity: Vector3 = self.orientation.origin / delta as f32;
        let mut velocity = self.base().get_velocity();
        velocity.x = h_velocity.x;
        velocity.z = h_velocity.z;
        velocity += self.base().get_gravity() * delta as f32;
        self.base_mut().set_velocity(velocity);
        self.base_mut().set_up_direction(Vector3::UP);
        self.base_mut().move_and_slide();

        // Clear accumulated root motion displacement (was applied to speed).
        self.orientation.origin = Vector3::ZERO;
        // orthonormalize orientation.
        self.orientation = self.orientation.orthonormalized();

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
        self.state = State::Approach;
        self.aim_preparing = AIM_PREPARE_TIME;
        self.shoot_countdown = SHOOT_WAIT;
    }

    #[rpc(authority, call_local, unreliable)]
    fn hit(&mut self) {
        if self.dead {
            return;
        }
        let param = format!("parameters/hit{}/request", randi() % 3 + 1);
        self.animation_tree.set(&param, &1.to_variant());
        self.hit_sound.play();
        self.health -= 1;
        if self.health == 0 {
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
                self.base()
                    .get_tree()
                    .create_timer(10.0)
                    .signals()
                    .timeout()
                    .connect_other(&*self, |this: &mut EnemyRobot| this.base_mut().queue_free());
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
        if body.clone().try_cast::<Player>().is_ok() || body.get_name() == "Target" {
            self.player = Some(body);
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
    fn shoot(&mut self) {
        let gt: Transform3D = self.ray_from.get_global_transform();
        let ray_origin: Vector3 = self.ray_from.get_global_transform().origin;
        // The RayCast3D is rotated 90 degrees inside the BoneAttachment3D.
        let ray_dir: Vector3 = gt.basis.col_b();
        let mut max_dist: f32 = 1000.0;

        let rid = self.base().get_rid();
        let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, ray_origin + ray_dir * max_dist)
            .collision_mask(0xFFFFFFFF)
            .exclude(&array![rid])
            .done();
        let col: VarDictionary = self
            .base()
            .get_world_3d()
            .unwrap()
            .get_direct_space_state()
            .unwrap()
            .intersect_ray(&params.unwrap());
        if !col.is_empty() {
            let position = col.get("position").unwrap().to::<Vector3>();
            max_dist = ray_origin.distance_to(position);
            // `if col.collider == player: pass # Kill.` — sem efeito no original.
        }
        // Clip ray in shader.
        self._clip_ray(max_dist);
        // Position laser ember particles
        let mesh_offset: f32 = self.ray_mesh.get_position().z;
        let mut laser_ember = self
            .base()
            .get_node_as::<CpuParticles3D>("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber");
        laser_ember.set_position(Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset));
        let mut extents = laser_ember.get_emission_box_extents();
        extents.z = (max_dist - mesh_offset.abs()) / 2.0;
        laser_ember.set_emission_box_extents(extents);
        if !col.is_empty() {
            let position = col.get("position").unwrap().to::<Vector3>();
            let mut blast: Gd<Node3D> = load::<PackedScene>(
                "res://enemies/red_robot/laser/impact_effect/impact_effect.tscn",
            )
            .instantiate_as::<Node3D>();
            self.base().get_tree().get_root().unwrap().add_child(&blast);
            blast.set_global_position(position);
            if let Some(player) = self.player.clone() {
                let hit_player = col
                    .get("collider")
                    .and_then(|v| v.try_to::<Gd<Object>>().ok())
                    .map(|c| c.instance_id() == player.instance_id())
                    .unwrap_or(false);
                if hit_player
                    && let Ok(player) = player.try_cast::<Player>()
                {
                    self.base()
                        .get_tree()
                        .create_timer(0.1)
                        .signals()
                        .timeout()
                        .connect_other(&*self, move |_this: &mut EnemyRobot| {
                            player.clone().bind_mut().add_camera_shake_trauma(13.0);
                        });
                }
            }
        }
    }

    fn animate(&mut self, delta: f64) {
        if self.state == State::Approach {
            let gt = self.base().get_global_transform();
            let to_player_local: Vector3 = gt.basis.transposed() * (self.target_position - gt.origin);
            // The front of the robot is +Z, and atan2 is zero at +X, so we need to use the Z for the X parameter (second one).
            let angle_to_player: f32 = to_player_local.x.atan2(to_player_local.z);
            if angle_to_player > PLAYER_AIM_TOLERANCE_DEGREES {
                self.animation_tree
                    .set("parameters/state/transition_request", &"turn_left".to_variant());
            } else if angle_to_player < -PLAYER_AIM_TOLERANCE_DEGREES {
                self.animation_tree
                    .set("parameters/state/transition_request", &"turn_right".to_variant());
            } else if self.target_position == Vector3::ZERO {
                self.animation_tree
                    .set("parameters/state/transition_request", &"idle".to_variant());
            } else {
                self.animation_tree
                    .set("parameters/state/transition_request", &"walk".to_variant());
            }
        } else {
            self.animation_tree
                .set("parameters/state/transition_request", &"idle".to_variant());
        }

        // Aiming or shooting
        if self.target_position != Vector3::ZERO {
            self.animation_tree.set(
                "parameters/aiming/blend_amount",
                &(self.aim_preparing / AIM_PREPARE_TIME).clamp(0.0, 1.0).to_variant(),
            );

            let mt = self.ray_mesh.get_global_transform();
            let to_cannon_local: Vector3 =
                mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin);
            let h_angle: f32 = to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees();
            let v_angle: f32 = to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees();
            let mut blend_pos: Vector2 = self
                .animation_tree
                .get("parameters/aim/blend_position")
                .to::<Vector2>();
            let h_motion: f32 = BLEND_AIM_SPEED * delta as f32 * -h_angle;
            blend_pos.x += h_motion;
            blend_pos.x = blend_pos.x.clamp(-1.0, 1.0);

            let v_motion: f32 = BLEND_AIM_SPEED * delta as f32 * v_angle;
            blend_pos.y += v_motion;
            blend_pos.y = blend_pos.y.clamp(-1.0, 1.0);

            self.animation_tree
                .set("parameters/aim/blend_position", &blend_pos.to_variant());
        }
    }

    fn _clip_ray(&mut self, length: f32) {
        let mesh_offset: f32 = self.ray_mesh.get_position().z;
        if !Os::singleton().has_feature("dedicated_server") {
            self.ray_mesh
                .get_surface_override_material(0)
                .unwrap()
                .cast::<ShaderMaterial>()
                .set_shader_parameter("clip", &(length + mesh_offset).to_variant());
        }
    }
}
