use godot::classes::{
    AnimationTree, AudioStreamPlayer, CharacterBody3D, CpuParticles3D, ICharacterBody3D, Marker3D,
    MultiplayerSynchronizer, Node3D, PackedScene, Timer,
};
use godot::prelude::*;

use crate::camera_noise_shake::CameraNoiseShake;
use crate::player_input::PlayerInputSynchronizer;

pub(crate) mod model;

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum Animations {
    JumpUp,
    JumpDown,
    Strafe,
    Walk,
}

const TRANSITION_REQUEST: &str = "parameters/state/transition_request";
const AIM_ADD_AMOUNT: &str = "parameters/aim/add_amount";
const STRAFE_BLEND: &str = "parameters/strafe/blend_position";
const WALK_BLEND: &str = "parameters/walk/blend_position";

const TRANSITION_JUMP_UP: &str = "jump_up";
const TRANSITION_JUMP_DOWN: &str = "jump_down";
const TRANSITION_STRAFE: &str = "strafe";
const TRANSITION_WALK: &str = "walk";

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Player {
    base: Base<CharacterBody3D>,

    // Backlog #10: starts at 0 (not v1's 100.0) so the first floor contact after spawn never
    // exceeds the land threshold and never fires a spurious `land` RPC/sound.
    #[init(val = 0.0)]
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
    #[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle")]
    shoot_particle: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/MuzzleFlash")]
    muzzle_particle: OnReady<Gd<CpuParticles3D>>,
    #[init(node = "FireCooldown")]
    fire_cooldown: OnReady<Gd<Timer>>,

    #[init(node = "SoundEffects/Jump")]
    sound_effect_jump: OnReady<Gd<AudioStreamPlayer>>,
    #[init(node = "SoundEffects/Land")]
    sound_effect_land: OnReady<Gd<AudioStreamPlayer>>,
    #[init(node = "SoundEffects/Shoot")]
    sound_effect_shoot: OnReady<Gd<AudioStreamPlayer>>,

    // Backlog #28's hypothesis (half of it): the bullet scene RESOURCE is loaded once here
    // instead of on every shot; a fresh instance is still created per shot below.
    #[init(val = load("res://player/bullet/bullet.tscn"))]
    bullet_scene: Gd<PackedScene>,

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
            // Non-authority: reproduce the replicated current_animation's AnimPlan from
            // self.motion (Player's own replicated field); player_input is bound at most once,
            // only when the target state is Strafe (the only case that needs aim_rotation).
            let plan = match self.current_animation {
                Animations::JumpUp => model::AnimPlan::JumpUp,
                Animations::JumpDown => model::AnimPlan::JumpDown,
                Animations::Strafe => {
                    let aim_rotation = self.player_input.bind().get_aim_rotation();
                    model::AnimPlan::Strafe {
                        aim_rotation,
                        blend_position: Vector2::new(self.motion.x, -self.motion.y),
                    }
                }
                Animations::Walk => model::AnimPlan::Walk {
                    blend_position: Vector2::new(self.motion.length(), 0.0),
                },
            };
            self.apply_anim(plan);
        }
    }
}

#[godot_api]
impl Player {
    #[func]
    pub(crate) fn set_player_id(&mut self, value: i32) {
        self.player_id = value;
        self.base()
            .get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")
            .set_multiplayer_authority(value);
    }

    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self) {
        self.apply_anim(model::AnimPlan::JumpUp);
        self.sound_effect_jump.play();
    }

    #[rpc(authority, call_local, unreliable)]
    fn land(&mut self) {
        self.apply_anim(model::AnimPlan::JumpDown);
        self.sound_effect_land.play();
    }

    #[rpc(authority, call_local, unreliable)]
    fn shoot(&mut self) {
        self.shoot_particle.restart();
        self.shoot_particle.set_emitting(true);
        self.muzzle_particle.restart();
        self.muzzle_particle.set_emitting(true);
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
        let camera = self.player_input.bind().camera_camera.clone();
        camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);
    }
}

impl Player {
    /// The one apply-step for `AnimPlan`: sets `current_animation` and writes the
    /// `AnimationTree` parameters, in the exact per-variant order `v1`'s `animate()` used.
    fn apply_anim(&mut self, plan: model::AnimPlan) {
        self.current_animation = match plan {
            model::AnimPlan::JumpUp => Animations::JumpUp,
            model::AnimPlan::JumpDown => Animations::JumpDown,
            model::AnimPlan::Strafe { .. } => Animations::Strafe,
            model::AnimPlan::Walk { .. } => Animations::Walk,
        };

        match plan {
            model::AnimPlan::JumpUp => {
                self.animation_tree.set(TRANSITION_REQUEST, &TRANSITION_JUMP_UP.to_variant());
            }
            model::AnimPlan::JumpDown => {
                self.animation_tree.set(TRANSITION_REQUEST, &TRANSITION_JUMP_DOWN.to_variant());
            }
            model::AnimPlan::Strafe { aim_rotation, blend_position } => {
                self.animation_tree.set(TRANSITION_REQUEST, &TRANSITION_STRAFE.to_variant());
                // Change aim according to camera rotation.
                self.animation_tree.set(AIM_ADD_AMOUNT, &aim_rotation.to_variant());
                // The animation's forward/backward axis is reversed.
                self.animation_tree.set(STRAFE_BLEND, &blend_position.to_variant());
            }
            model::AnimPlan::Walk { blend_position } => {
                // Aim to zero (no aiming while walking).
                self.animation_tree.set(AIM_ADD_AMOUNT, &0.to_variant());
                self.animation_tree.set(TRANSITION_REQUEST, &TRANSITION_WALK.to_variant());
                // Blend position for walk speed based checked motion.
                self.animation_tree.set(WALK_BLEND, &blend_position.to_variant());
            }
        }
    }

    fn apply_input(&mut self, delta: f64) {
        let dt = delta as f32;
        let tuning = model::PlayerTuning::default();

        // (1) ONE player_input acquisition: build the InputFrame, clear `jumping`, drop the
        // guard before anything else touches player_input.
        let frame = {
            let mut input = self.player_input.bind_mut();
            let frame = model::InputFrame {
                motion: input.motion,
                aiming: input.aiming,
                shooting: input.shooting,
                jumping: input.jumping,
                shoot_target: input.shoot_target,
                camera_rotation_basis: input.get_camera_rotation_basis(),
                camera_base_quaternion: input.get_camera_base_quaternion(),
                aim_rotation: input.get_aim_rotation(),
            };
            input.jumping = false;
            frame
        };

        // (2) motion lerp.
        self.motion = model::lerp_motion(self.motion, frame.motion, dt, &tuning);

        // (3) flattened camera axes.
        let (camera_x, camera_z) = model::flatten_camera_axes(frame.camera_rotation_basis);

        // (4) airborne step (engine read: is_on_floor()).
        let outcome = model::airborne_step(
            self.airborne_time,
            dt,
            self.base().is_on_floor(),
            frame.jumping,
            &tuning,
        );
        self.airborne_time = outcome.airborne_time;
        if let Some(jump_velocity_y) = outcome.jump_velocity_y {
            let mut velocity = self.base().get_velocity();
            velocity.y = jump_velocity_y;
            self.base_mut().set_velocity(velocity);
        }
        // `land` and `jump` are independent — both may fire in the same call.
        if outcome.land {
            self.base_mut().rpc("land", &[]);
        }
        if outcome.jump {
            self.base_mut().rpc("jump", &[]);
        }

        // (5) branch.
        if outcome.on_air {
            let velocity_y = self.base().get_velocity().y;
            let plan = model::anim_plan(true, velocity_y, false, self.motion, 0.0);
            self.apply_anim(plan);
            // root_motion is NOT reassigned while airborne (v1's own field-persistence).
        } else if frame.aiming {
            // Convert orientation to quaternions for interpolating rotation.
            let q_from: Quaternion = self.orientation.basis.get_quaternion();
            let q_to: Quaternion = frame.camera_base_quaternion;
            self.orientation.basis =
                Basis::from_quaternion(q_from.slerp(q_to, dt * tuning.rotation_interpolate_speed));

            let plan = model::anim_plan(false, 0.0, true, self.motion, frame.aim_rotation);
            self.apply_anim(plan);

            self.root_motion = Transform3D::new(
                Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
                self.animation_tree.get_root_motion_position(),
            );

            if frame.shooting && self.fire_cooldown.get_time_left() == 0.0 {
                let shoot_origin: Vector3 = self.shoot_from.get_global_transform().origin;
                let shoot_dir: Vector3 = (frame.shoot_target - shoot_origin).normalized();

                let mut bullet: Gd<CharacterBody3D> =
                    self.bullet_scene.instantiate_as::<CharacterBody3D>();
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
            let walk_target = model::walk_target(camera_x, camera_z, self.motion);
            if let Some(target) = walk_target {
                let q_from: Quaternion = self.orientation.basis.get_quaternion();
                let q_to: Quaternion = Basis::looking_at(target).get_quaternion();
                self.orientation.basis = Basis::from_quaternion(
                    q_from.slerp(q_to, dt * tuning.rotation_interpolate_speed),
                );
            }

            let plan = model::anim_plan(false, 0.0, false, self.motion, 0.0);
            self.apply_anim(plan);

            self.root_motion = Transform3D::new(
                Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()),
                self.animation_tree.get_root_motion_position(),
            );
        }

        // (6) integrate root motion.
        let velocity_in = self.base().get_velocity();
        let gravity = self.base().get_gravity();
        let (new_orientation, new_velocity) =
            model::integrate_root_motion(self.orientation, self.root_motion, dt, gravity, velocity_in);
        self.orientation = new_orientation;

        // (7)
        self.base_mut().set_velocity(new_velocity);
        self.base_mut().set_up_direction(Vector3::UP);
        self.base_mut().move_and_slide();

        // (8)
        let basis = self.orientation.basis;
        self.player_model.set_global_basis(basis);

        // (9) Backlog #11: respawn also zeroes velocity (v1 only reset the transform origin).
        if model::should_respawn(self.base().get_transform().origin.y, &tuning) {
            let mut transform = self.base().get_transform();
            transform.origin = self.initial_position;
            self.base_mut().set_transform(transform);
            self.base_mut().set_velocity(Vector3::ZERO);
        }
    }
}
