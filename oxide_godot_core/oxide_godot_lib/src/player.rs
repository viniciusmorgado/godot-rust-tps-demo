use godot::classes::{
    AnimationTree, AudioStreamPlayer, CharacterBody3D, CpuParticles3D, ICharacterBody3D, Marker3D,
    MultiplayerSynchronizer, Node3D, PackedScene, TextureRect, Timer,
};
use godot::prelude::*;

use crate::camera_noise_shake::CameraNoiseShake;
use crate::player_input::PlayerInputSynchronizer;

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum Animations {
    JumpUp,
    JumpDown,
    Strafe,
    Walk,
}

const MOTION_INTERPOLATE_SPEED: f32 = 10.0;
const ROTATION_INTERPOLATE_SPEED: f32 = 10.0;

const MIN_AIRBORNE_TIME: f32 = 0.1;
const JUMP_SPEED: f32 = 5.0;

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Player {
    base: Base<CharacterBody3D>,

    #[init(val = 100.0)]
    airborne_time: f32,

    orientation: Transform3D,
    root_motion: Transform3D,
    #[var]
    motion: Vector2,

    initial_position: Vector3,

    #[init(node = "InputSynchronizer")]
    player_input: OnReady<Gd<PlayerInputSynchronizer>>,
    #[init(node = "AnimationTree")]
    animation_tree: OnReady<Gd<AnimationTree>>,
    #[init(node = "PlayerModel")]
    player_model: OnReady<Gd<Node3D>>,
    #[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom")]
    shoot_from: OnReady<Gd<Marker3D>>,
    #[init(node = "Crosshair")]
    crosshair: OnReady<Gd<TextureRect>>,
    #[init(node = "FireCooldown")]
    fire_cooldown: OnReady<Gd<Timer>>,

    #[init(node = "SoundEffects/Jump")]
    sound_effect_jump: OnReady<Gd<AudioStreamPlayer>>,
    #[init(node = "SoundEffects/Land")]
    sound_effect_land: OnReady<Gd<AudioStreamPlayer>>,
    #[init(node = "SoundEffects/Shoot")]
    sound_effect_shoot: OnReady<Gd<AudioStreamPlayer>>,

    #[export]
    #[var(set = set_player_id)]
    #[init(val = 1)]
    player_id: i32,

    #[export]
    #[init(val = Animations::Walk)]
    current_animation: Animations,
}

#[godot_api]
impl ICharacterBody3D for Player {
    fn ready(&mut self) {
        self.initial_position = self.base().get_transform().origin;
        // Pre-initialize orientation transform.
        self.orientation = self.player_model.get_global_transform();
        self.orientation.origin = Vector3::ZERO;
        if !self.base().get_multiplayer().unwrap().is_server() {
            self.base_mut().set_process(false);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        if self.base().get_multiplayer().unwrap().is_server() {
            self.apply_input(delta);
        } else {
            let anim = self.current_animation;
            self.animate(anim, delta);
        }
    }
}

#[godot_api]
impl Player {
    #[func]
    fn set_player_id(&mut self, value: i32) {
        self.player_id = value;
        self.base()
            .get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")
            .set_multiplayer_authority(value);
    }

    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self) {
        self.animate(Animations::JumpUp, 0.0);
        self.sound_effect_jump.play();
    }

    #[rpc(authority, call_local, unreliable)]
    fn land(&mut self) {
        self.animate(Animations::JumpDown, 0.0);
        self.sound_effect_land.play();
    }

    #[rpc(authority, call_local, unreliable)]
    fn shoot(&mut self) {
        let mut shoot_particle = self.base().get_node_as::<CpuParticles3D>(
            "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle",
        );
        shoot_particle.restart();
        shoot_particle.set_emitting(true);
        let mut muzzle_particle = self.base().get_node_as::<CpuParticles3D>(
            "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/MuzzleFlash",
        );
        muzzle_particle.restart();
        muzzle_particle.set_emitting(true);
        self.fire_cooldown.start();
        self.sound_effect_shoot.play();
        self.add_camera_shake_trauma(0.35);
    }

    #[rpc(authority, call_local, unreliable)]
    fn hit(&mut self) {
        self.add_camera_shake_trauma(0.75);
    }

    #[rpc(authority, call_local, unreliable)]
    pub(crate) fn add_camera_shake_trauma(&mut self, amount: f64) {
        let camera = self.player_input.bind().camera_camera.clone().unwrap();
        camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);
    }
}

impl Player {
    fn animate(&mut self, anim: Animations, _delta: f64) {
        self.current_animation = anim;

        if anim == Animations::JumpUp {
            self.animation_tree
                .set("parameters/state/transition_request", &"jump_up".to_variant());
        } else if anim == Animations::JumpDown {
            self.animation_tree
                .set("parameters/state/transition_request", &"jump_down".to_variant());
        } else if anim == Animations::Strafe {
            self.animation_tree
                .set("parameters/state/transition_request", &"strafe".to_variant());
            // Change aim according to camera rotation.
            let aim = self.player_input.bind().get_aim_rotation();
            self.animation_tree
                .set("parameters/aim/add_amount", &aim.to_variant());
            // The animation's forward/backward axis is reversed.
            self.animation_tree.set(
                "parameters/strafe/blend_position",
                &Vector2::new(self.motion.x, -self.motion.y).to_variant(),
            );
        } else if anim == Animations::Walk {
            // Aim to zero (no aiming while walking).
            self.animation_tree
                .set("parameters/aim/add_amount", &0.to_variant());
            // Change state to walk.
            self.animation_tree
                .set("parameters/state/transition_request", &"walk".to_variant());
            // Blend position for walk speed based checked motion.
            self.animation_tree.set(
                "parameters/walk/blend_position",
                &Vector2::new(self.motion.length(), 0.0).to_variant(),
            );
        }
    }

    fn apply_input(&mut self, delta: f64) {
        let input_motion = self.player_input.bind().motion;
        self.motion = self
            .motion
            .lerp(input_motion, MOTION_INTERPOLATE_SPEED * delta as f32);

        let camera_basis: Basis = self.player_input.bind().get_camera_rotation_basis();
        let mut camera_z: Vector3 = camera_basis.col_c();
        let mut camera_x: Vector3 = camera_basis.col_a();

        camera_z.y = 0.0;
        camera_z = camera_z.normalized();
        camera_x.y = 0.0;
        camera_x = camera_x.normalized();

        // Jump/in-air logic.
        self.airborne_time += delta as f32;
        if self.base().is_on_floor() {
            if self.airborne_time > 0.5 {
                self.base_mut().rpc("land", &[]);
            }
            self.airborne_time = 0.0;
        }

        let mut on_air: bool = self.airborne_time > MIN_AIRBORNE_TIME;

        if !on_air && self.player_input.bind().jumping {
            let mut velocity = self.base().get_velocity();
            velocity.y = JUMP_SPEED;
            self.base_mut().set_velocity(velocity);
            on_air = true;
            // Increase airborne time so next frame on_air is still true
            self.airborne_time = MIN_AIRBORNE_TIME;
            self.base_mut().rpc("jump", &[]);
        }

        self.player_input.bind_mut().jumping = false;

        if on_air {
            if self.base().get_velocity().y > 0.0 {
                self.animate(Animations::JumpUp, delta);
            } else {
                self.animate(Animations::JumpDown, delta);
            }
        } else if self.player_input.bind().aiming {
            // Convert orientation to quaternions for interpolating rotation.
            let q_from: Quaternion = self.orientation.basis.get_quaternion();
            let q_to: Quaternion = self.player_input.bind().get_camera_base_quaternion();
            // Interpolate current rotation with desired one.
            self.orientation.basis = Basis::from_quaternion(
                q_from.slerp(q_to, delta as f32 * ROTATION_INTERPOLATE_SPEED),
            );

            // Change state to strafe.
            self.animate(Animations::Strafe, delta);

            self.root_motion = Transform3D::new(
                Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
                self.animation_tree.get_root_motion_position(),
            );

            if self.player_input.bind().shooting && self.fire_cooldown.get_time_left() == 0.0 {
                let shoot_origin: Vector3 = self.shoot_from.get_global_transform().origin;
                let shoot_target: Vector3 = self.player_input.bind().shoot_target;
                let shoot_dir: Vector3 = (shoot_target - shoot_origin).normalized();

                let mut bullet: Gd<CharacterBody3D> =
                    load::<PackedScene>("res://player/bullet/bullet.tscn")
                        .instantiate_as::<CharacterBody3D>();
                self.base()
                    .get_parent()
                    .unwrap()
                    .add_child_ex(&bullet)
                    .force_readable_name(true)
                    .done();
                bullet.set_global_position(shoot_origin);
                // If we don't rotate the bullets there is no useful way to control the particles ..
                bullet.look_at(shoot_origin + shoot_dir);
                bullet.add_collision_exception_with(&self.to_gd());
                self.base_mut().rpc("shoot", &[]);
            }
        } else {
            // Not in air or aiming, idle.
            // Convert orientation to quaternions for interpolating rotation.
            let target: Vector3 = camera_x * self.motion.x + camera_z * self.motion.y;
            if target.length() > 0.001 {
                let q_from: Quaternion = self.orientation.basis.get_quaternion();
                let q_to: Quaternion = Basis::looking_at(target).get_quaternion();
                // Interpolate current rotation with desired one.
                self.orientation.basis = Basis::from_quaternion(
                    q_from.slerp(q_to, delta as f32 * ROTATION_INTERPOLATE_SPEED),
                );
            }

            self.animate(Animations::Walk, delta);

            self.root_motion = Transform3D::new(
                Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
                self.animation_tree.get_root_motion_position(),
            );
        }

        // Apply root motion to orientation.
        self.orientation = self.orientation * self.root_motion;

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
        // Orthonormalize orientation.
        self.orientation = self.orientation.orthonormalized();

        let basis = self.orientation.basis;
        self.player_model.set_global_basis(basis);

        // If we're below -40, respawn (teleport to the initial position).
        if self.base().get_transform().origin.y < -40.0 {
            let mut transform = self.base().get_transform();
            transform.origin = self.initial_position;
            self.base_mut().set_transform(transform);
        }
    }
}
