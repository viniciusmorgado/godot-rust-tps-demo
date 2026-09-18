//! The input node's sub-bridge (constitution 1.5.1 "ECS shape (v3)"; specs/012 contracts §1):
//! registers nothing, keeps v2's one-shot `ready` setup, and pushes `MouseLook`/`JumpPressed`
//! keyed by the root player's id. No per-frame logic: the frame run is
//! `player_input/system.rs` (pure) + `player_input/sync.rs` (engine).

use godot::classes::input::MouseMode;
use godot::classes::{
    AnimationPlayer, Camera3D, CharacterBody3D, ColorRect, IMultiplayerSynchronizer, Input,
    InputEvent, InputEventMouseMotion, MultiplayerSynchronizer, Node3D, TextureRect,
};
use godot::obj::InstanceId;
use godot::prelude::*;

use crate::ecs::event::InboundEvent;
use crate::ecs::queue;

pub(crate) mod model;
pub(crate) mod sync;
pub(crate) mod system;

#[derive(GodotClass)]
#[class(init, base=MultiplayerSynchronizer)]
pub struct PlayerInputSynchronizer {
    base: Base<MultiplayerSynchronizer>,

    // The root `Player` (this scene's owner, research R1's `OWNER` check), resolved once: every
    // event this node pushes is keyed by it.
    #[init(val = OnReady::from_base_fn(|base| base.get_owner().unwrap().instance_id()))]
    root_id: OnReady<InstanceId>,

    // The parent `CharacterBody3D`'s physics RID, resolved once before `ready()`; the raycast
    // in `player_input/sync.rs` excludes it.
    #[init(val = OnReady::from_base_fn(|base| base.get_parent().unwrap().cast::<CharacterBody3D>().get_rid()))]
    pub(crate) parent_rid: OnReady<Rid>,

    // Synchronized controls: the replicated projection of the entity's `ReplicatedInput`,
    // written by the frame `SyncOut` on the owning peer, read by the fixed `SyncIn` everywhere.
    #[export]
    pub(crate) aiming: bool,
    #[export]
    pub(crate) shoot_target: Vector3,
    #[export]
    pub(crate) motion: Vector2,
    #[export]
    pub(crate) shooting: bool,

    // Camera and effects (`node_paths` in `player.tscn`), read once by `Player.ready` into the
    // entity's handles.
    #[export]
    pub(crate) camera_animation: OnEditor<Gd<AnimationPlayer>>,
    #[export]
    pub(crate) crosshair: OnEditor<Gd<TextureRect>>,
    #[export]
    pub(crate) camera_base: OnEditor<Gd<Node3D>>,
    #[export]
    pub(crate) camera_rot: OnEditor<Gd<Node3D>>,
    #[export]
    pub(crate) camera_camera: OnEditor<Gd<Camera3D>>,
    #[export]
    pub(crate) color_rect: OnEditor<Gd<ColorRect>>,
}

#[godot_api]
impl IMultiplayerSynchronizer for PlayerInputSynchronizer {
    /// v2's one-shot setup. `set_process_input(false)` on non-authority nodes is FR-013's gate:
    /// the `input` callback (and so `MouseLook`) exists only on the node whose entity owns the
    /// input.
    fn ready(&mut self) {
        let unique_id = self.base().get_multiplayer().unwrap().get_unique_id();
        if self.base().get_multiplayer_authority() == unique_id {
            self.camera_camera.make_current();
            Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
        } else {
            self.base_mut().set_process_input(false);
            self.color_rect.hide();
        }
    }

    fn input(&mut self, input_event: Gd<InputEvent>) {
        if let Ok(mouse_motion) = input_event.try_cast::<InputEventMouseMotion>() {
            queue::push(InboundEvent::MouseLook {
                root_id: *self.root_id,
                screen_relative: mouse_motion.get_screen_relative(),
            });
        }
    }
}

#[godot_api]
impl PlayerInputSynchronizer {
    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self) {
        queue::push(InboundEvent::JumpPressed { root_id: *self.root_id });
    }
}
