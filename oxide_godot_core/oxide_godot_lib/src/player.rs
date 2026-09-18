//! The player's root bridge (constitution 1.5.1 "ECS shape (v3)"; specs/012 contracts
//! §1): `ready` registers the ONE entity over the three nodes with every handle, `exit_tree`
//! unregisters, the RPC handlers push events. No per-frame logic: the tick is
//! `player/system.rs` (pure) + `player/sync.rs` (engine).

use godot::classes::{
    AnimationTree, AudioStreamPlayer, CharacterBody3D, CpuParticles3D, ICharacterBody3D, Marker3D,
    MultiplayerSynchronizer, Node3D, PackedScene, Timer,
};
use godot::prelude::*;

use crate::camera_noise_shake::CameraNoiseShake;
use crate::ecs::event::{InboundEvent, Initial, PlayerFx};
use crate::ecs::{Handles, PlayerHandles, queue};
use crate::player_input::PlayerInputSynchronizer;

pub(crate) mod model;
pub(crate) mod sync;
pub(crate) mod system;

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum Animations {
    JumpUp,
    JumpDown,
    Strafe,
    Walk,
}

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Player {
    base: Base<CharacterBody3D>,

    // The replicated projection of the entity's `Motion` component (`player.tscn`
    // `.:motion`), written by the fixed `SyncOut` on the simulating peer and read back by the
    // fixed `SyncIn` on the others.
    #[var]
    pub(crate) motion: Vector2,

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
    // instead of on every shot; a fresh instance is still created per shot by the tick.
    #[init(val = load("res://player/bullet/bullet.tscn"))]
    bullet_scene: Gd<PackedScene>,

    #[export]
    #[var(set = set_player_id)]
    #[init(val = 1)]
    player_id: i32,

    // The replicated projection of the entity's `CurrentAnimation` component
    // (`.:current_animation`), written and read as `motion` is.
    #[export]
    #[init(val = Animations::Walk)]
    pub(crate) current_animation: Animations,
}

#[godot_api]
impl ICharacterBody3D for Player {
    /// Registers the entity (specs/012 research R5). Children are ready first, so the input
    /// node's `OnEditor` refs and the camera's seeded noises / `start_rotation` are readable.
    fn ready(&mut self) {
        let mp = self.base().get_multiplayer().unwrap();
        let simulates = mp.is_server();
        let owns_input = self.player_input.get_multiplayer_authority() == mp.get_unique_id();
        let initial_position = self.base().get_transform().origin;
        // Pre-initialize orientation transform.
        let mut orientation = self.player_model.get_global_transform();
        orientation.origin = Vector3::ZERO;

        let (camera_base, camera_rot, camera, camera_anim, crosshair, color_rect, parent_rid) = {
            let pi = self.player_input.bind();
            (
                pi.camera_base.clone(),
                pi.camera_rot.clone(),
                pi.camera_camera.clone(),
                pi.camera_animation.clone(),
                pi.crosshair.clone(),
                pi.color_rect.clone(),
                *pi.parent_rid,
            )
        };
        let cam = camera.clone().cast::<CameraNoiseShake>();
        let (noise, start_rotation) = {
            let c = cam.bind();
            (c.noises(), c.start_rotation())
        };

        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Player(Box::new(PlayerHandles {
                root: self.to_gd(),
                input: self.player_input.clone(),
                anim_tree: self.animation_tree.clone(),
                model: self.player_model.clone(),
                shoot_from: self.shoot_from.clone(),
                shoot_particle: self.shoot_particle.clone(),
                muzzle_particle: self.muzzle_particle.clone(),
                fire_cooldown: self.fire_cooldown.clone(),
                snd_jump: self.sound_effect_jump.clone(),
                snd_land: self.sound_effect_land.clone(),
                snd_shoot: self.sound_effect_shoot.clone(),
                camera_base,
                camera_rot,
                camera,
                camera_anim,
                crosshair,
                color_rect,
                noise,
                bullet_scene: self.bullet_scene.clone(),
                parent_rid,
            })),
            initial: Initial::Player {
                peer_id: self.player_id,
                simulates,
                owns_input,
                initial_position,
                orientation,
                start_rotation,
            },
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
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

    // `jump`/`land`/`shoot` reach REMOTE peers only (spec FR-004, option (b)): the simulating
    // peer applies their local effects inline in the fixed `SyncOut`, so these handlers run on
    // the peers that replay the player and only push the effect for their frame run.
    #[rpc(authority, call_remote, unreliable)]
    fn jump(&mut self) {
        self.push_fx(PlayerFx::Jump);
    }

    #[rpc(authority, call_remote, unreliable)]
    fn land(&mut self) {
        self.push_fx(PlayerFx::Land);
    }

    #[rpc(authority, call_remote, unreliable)]
    fn shoot(&mut self) {
        self.push_fx(PlayerFx::Shoot);
    }

    #[rpc(authority, call_local, unreliable)]
    fn hit(&mut self) {
        self.add_camera_shake_trauma(0.75);
    }

    // Commit 2 keeps v2's typed trauma path (research R10); commit 3 pushes `AddTrauma`.
    #[rpc(authority, call_local, unreliable)]
    pub(crate) fn add_camera_shake_trauma(&mut self, amount: f64) {
        let camera = self.player_input.bind().camera_camera.clone();
        camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);
    }
}

impl Player {
    fn push_fx(&self, fx: PlayerFx) {
        queue::push(InboundEvent::PlayerFx { root_id: self.base().instance_id(), fx });
    }
}
