use godot::classes::input::MouseMode;
use godot::classes::{
    AnimationPlayer, Camera3D, CharacterBody3D, ColorRect, IMultiplayerSynchronizer, Input,
    InputEvent, InputEventMouseMotion, MultiplayerSynchronizer, Node3D,
    PhysicsRayQueryParameters3D, TextureRect,
};
use godot::prelude::*;

mod model;

use model::{
    AimState, CameraCue, InputSnapshot, PlayerInputTuning, aim_rotation, alpha_for_height,
    clamp_pitch, scaled_look, scaled_mouse_look, step_aim,
};

#[derive(GodotClass)]
#[class(init, base=MultiplayerSynchronizer)]
pub struct PlayerInputSynchronizer {
    base: Base<MultiplayerSynchronizer>,

    #[init(val = AimState::Idle)]
    aim_state: AimState,

    // The parent `CharacterBody3D` (this node's own parent in `player.tscn`) and its physics
    // RID, resolved once before `ready()` instead of looked up per frame.
    #[init(val = OnReady::from_base_fn(|base| base.get_parent().unwrap().cast::<CharacterBody3D>()))]
    parent: OnReady<Gd<CharacterBody3D>>,
    #[init(val = OnReady::from_base_fn(|base| base.get_parent().unwrap().cast::<CharacterBody3D>().get_rid()))]
    parent_rid: OnReady<Rid>,

    // Synchronized controls
    #[export]
    pub(crate) aiming: bool,
    #[export]
    pub(crate) shoot_target: Vector3,
    #[export]
    pub(crate) motion: Vector2,
    #[export]
    pub(crate) shooting: bool,
    // This is handled via RPC for now; not replicated or stored by any scene (backlog #9).
    pub(crate) jumping: bool,

    // Camera and effects
    #[export]
    camera_animation: OnEditor<Gd<AnimationPlayer>>,
    #[export]
    crosshair: OnEditor<Gd<TextureRect>>,
    #[export]
    camera_base: OnEditor<Gd<Node3D>>,
    #[export]
    camera_rot: OnEditor<Gd<Node3D>>,
    #[export]
    pub(crate) camera_camera: OnEditor<Gd<Camera3D>>,
    #[export]
    color_rect: OnEditor<Gd<ColorRect>>,
}

#[godot_api]
impl IMultiplayerSynchronizer for PlayerInputSynchronizer {
    fn ready(&mut self) {
        let unique_id = self.base().get_multiplayer().unwrap().get_unique_id();
        if self.base().get_multiplayer_authority() == unique_id {
            self.camera_camera.make_current();
            Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
        } else {
            self.base_mut().set_process(false);
            self.base_mut().set_process_input(false);
            self.color_rect.hide();
        }
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let tuning = PlayerInputTuning::default();
        let input = Input::singleton();
        let snapshot = InputSnapshot {
            motion: Vector2::new(
                input.get_action_strength("move_right") - input.get_action_strength("move_left"),
                input.get_action_strength("move_back") - input.get_action_strength("move_forward"),
            ),
            camera_move: Vector2::new(
                input.get_action_strength("view_right") - input.get_action_strength("view_left"),
                input.get_action_strength("view_up") - input.get_action_strength("view_down"),
            ),
            aim_just_pressed: input.is_action_just_pressed("aim"),
            aim_pressed: input.is_action_pressed("aim"),
            aim_just_released: input.is_action_just_released("aim"),
            jump_just_pressed: input.is_action_just_pressed("jump"),
            shoot_pressed: input.is_action_pressed("shoot"),
        };

        self.motion = snapshot.motion;

        let camera_move = scaled_look(snapshot.camera_move, self.aiming, dt, &tuning);
        self.rotate_camera(camera_move, &tuning);

        let (next_state, cue) = step_aim(self.aim_state, &snapshot, dt, &tuning);
        self.aim_state = next_state;
        self.aiming = self.aim_state.is_aiming();
        if let Some(cue) = cue {
            match cue {
                CameraCue::Shoot => {
                    self.camera_animation.play_ex().name("shoot").done();
                }
                CameraCue::Far => {
                    self.camera_animation.play_ex().name("far").done();
                }
            }
        }

        if snapshot.jump_just_pressed {
            self.base_mut().rpc("jump", &[]);
        }

        self.shooting = snapshot.shoot_pressed;
        if self.shooting {
            let ch_pos = self.crosshair.get_position() + self.crosshair.get_size() * 0.5;
            let ray_from = self.camera_camera.project_ray_origin(ch_pos);
            let ray_dir = self.camera_camera.project_ray_normal(ch_pos);

            let params =
                PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0)
                    .collision_mask(0b11)
                    .exclude(&array![*self.parent_rid])
                    .done()
                    .unwrap();
            let col = self
                .parent
                .get_world_3d()
                .unwrap()
                .get_direct_space_state()
                .unwrap()
                .intersect_ray(&params);
            if col.is_empty() {
                self.shoot_target = ray_from + ray_dir * 1000.0;
            } else {
                self.shoot_target = col.get("position").unwrap().to::<Vector3>();
            }
        }

        // Fade out to black if falling out of the map. -17 is lower than
        // the lowest valid position checked the map (which is a bit under -16).
        // At 15 units below -17 (so -32), the screen turns fully black.
        let player_y = self.parent.get_global_transform().origin.y;
        let mut modulate = self.color_rect.get_modulate();
        modulate.a = alpha_for_height(player_y, modulate.a, dt, &tuning);
        self.color_rect.set_modulate(modulate);
    }

    fn input(&mut self, input_event: Gd<InputEvent>) {
        if let Ok(mouse_motion) = input_event.try_cast::<InputEventMouseMotion>() {
            let tuning = PlayerInputTuning::default();
            let mv = scaled_mouse_look(mouse_motion.get_screen_relative(), self.aiming, &tuning);
            self.rotate_camera(mv, &tuning);
        }
    }
}

#[godot_api]
impl PlayerInputSynchronizer {
    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self) {
        self.jumping = true;
    }
}

impl PlayerInputSynchronizer {
    // These three lose `#[func]` in V2-C (specs/008-v2-player-bullet-door, research.md R1):
    // nothing calls them by name (grep-confirmed empty across `.gd`/`.tscn`); `player.rs` calls
    // them typed via `self.player_input.bind()...`.
    pub(crate) fn get_aim_rotation(&self) -> f64 {
        let tuning = PlayerInputTuning::default();
        aim_rotation(self.camera_rot.get_rotation().x, &tuning)
    }

    pub(crate) fn get_camera_base_quaternion(&self) -> Quaternion {
        self.camera_base.get_global_transform().basis.get_quaternion()
    }

    pub(crate) fn get_camera_rotation_basis(&self) -> Basis {
        self.camera_rot.get_global_transform().basis
    }

    fn rotate_camera(&mut self, mv: Vector2, tuning: &PlayerInputTuning) {
        self.camera_base.rotate_y(-mv.x);
        // After relative transforms, camera needs to be renormalized.
        self.camera_base.orthonormalize();
        let mut rotation = self.camera_rot.get_rotation();
        rotation.x = clamp_pitch(rotation.x, mv.y, tuning);
        self.camera_rot.set_rotation(rotation);
    }
}
