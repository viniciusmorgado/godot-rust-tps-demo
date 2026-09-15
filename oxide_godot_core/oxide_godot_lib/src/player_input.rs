use godot::classes::input::MouseMode;
use godot::classes::{
    AnimationPlayer, Camera3D, ColorRect, IMultiplayerSynchronizer, Input, InputEvent,
    InputEventMouseMotion, MultiplayerSynchronizer, Node3D, PhysicsRayQueryParameters3D,
    TextureRect,
};
use godot::prelude::*;

const CAMERA_CONTROLLER_ROTATION_SPEED: f32 = 3.0;
const CAMERA_MOUSE_ROTATION_SPEED: f32 = 0.001;
// A minimum angle lower than or equal to -90 breaks movement if the player is looking upward.
const CAMERA_X_ROT_MIN: f32 = (-89.9_f32).to_radians();
const CAMERA_X_ROT_MAX: f32 = 70.0_f32.to_radians();

// Release aiming if the mouse/gamepad button was held for longer than 0.4 seconds.
// This works well for trackpads and is more accessible by not making long presses a requirement.
// If the aiming button was held for less than 0.4 seconds, keep aiming until the aiming button is pressed again.
const AIM_HOLD_THRESHOLD: f32 = 0.4;

#[derive(GodotClass)]
#[class(init, base=MultiplayerSynchronizer)]
pub struct PlayerInputSynchronizer {
    base: Base<MultiplayerSynchronizer>,

    // If `true`, the aim button was toggled checked by a short press (instead of being held down).
    toggled_aim: bool,

    // The duration the aiming button was held for (in seconds).
    aiming_timer: f32,

    // Synchronized controls
    #[export]
    pub(crate) aiming: bool,
    #[export]
    pub(crate) shoot_target: Vector3,
    #[export]
    pub(crate) motion: Vector2,
    #[export]
    pub(crate) shooting: bool,
    // This is handled via RPC for now
    #[export]
    pub(crate) jumping: bool,

    // Camera and effects
    #[export]
    camera_animation: Option<Gd<AnimationPlayer>>,
    #[export]
    crosshair: Option<Gd<TextureRect>>,
    #[export]
    camera_base: Option<Gd<Node3D>>,
    #[export]
    camera_rot: Option<Gd<Node3D>>,
    #[export]
    pub(crate) camera_camera: Option<Gd<Camera3D>>,
    #[export]
    color_rect: Option<Gd<ColorRect>>,
}

#[godot_api]
impl IMultiplayerSynchronizer for PlayerInputSynchronizer {
    fn ready(&mut self) {
        let unique_id = self.base().get_multiplayer().unwrap().get_unique_id();
        if self.base().get_multiplayer_authority() == unique_id {
            self.camera_camera.as_mut().unwrap().make_current();
            Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
        } else {
            self.base_mut().set_process(false);
            self.base_mut().set_process_input(false);
            self.color_rect.as_mut().unwrap().hide();
        }
    }

    fn process(&mut self, delta: f64) {
        let input = Input::singleton();
        self.motion = Vector2::new(
            input.get_action_strength("move_right") - input.get_action_strength("move_left"),
            input.get_action_strength("move_back") - input.get_action_strength("move_forward"),
        );
        let camera_move = Vector2::new(
            input.get_action_strength("view_right") - input.get_action_strength("view_left"),
            input.get_action_strength("view_up") - input.get_action_strength("view_down"),
        );
        let mut camera_speed_this_frame: f32 = delta as f32 * CAMERA_CONTROLLER_ROTATION_SPEED;
        if self.aiming {
            camera_speed_this_frame *= 0.5;
        }
        self.rotate_camera(camera_move * camera_speed_this_frame);
        let current_aim: bool;

        // Keep aiming if the mouse wasn't held for long enough.
        if input.is_action_just_released("aim") && self.aiming_timer <= AIM_HOLD_THRESHOLD {
            current_aim = true;
            self.toggled_aim = true;
        } else {
            current_aim = self.toggled_aim || input.is_action_pressed("aim");
            if input.is_action_just_pressed("aim") {
                self.toggled_aim = false;
            }
        }

        if current_aim {
            self.aiming_timer += delta as f32;
        } else {
            self.aiming_timer = 0.0;
        }

        if self.aiming != current_aim {
            self.aiming = current_aim;
            if self.aiming {
                self.camera_animation.as_mut().unwrap().play_ex().name("shoot").done();
            } else {
                self.camera_animation.as_mut().unwrap().play_ex().name("far").done();
            }
        }

        if input.is_action_just_pressed("jump") {
            self.base_mut().rpc("jump", &[]);
        }

        self.shooting = input.is_action_pressed("shoot");
        if self.shooting {
            let crosshair = self.crosshair.as_ref().unwrap();
            let ch_pos = crosshair.get_position() + crosshair.get_size() * 0.5;
            let camera_camera = self.camera_camera.as_ref().unwrap();
            let ray_from = camera_camera.project_ray_origin(ch_pos);
            let ray_dir = camera_camera.project_ray_normal(ch_pos);

            let params = PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0)
                .collision_mask(0b11)
                .exclude(&array![Rid::Invalid])
                .done()
                .unwrap();
            let col = self
                .base()
                .get_parent()
                .unwrap()
                .cast::<Node3D>()
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
        let player_transform: Transform3D = self
            .base()
            .get_parent()
            .unwrap()
            .cast::<Node3D>()
            .get_global_transform();
        let color_rect = self.color_rect.as_mut().unwrap();
        let mut modulate = color_rect.get_modulate();
        if player_transform.origin.y < -17.0 {
            modulate.a = ((-17.0 - player_transform.origin.y) / 15.0).min(1.0);
        } else {
            // Fade out the black ColorRect progressively after being teleported back.
            modulate.a *= 1.0 - delta as f32 * 4.0;
        }
        color_rect.set_modulate(modulate);
    }

    fn input(&mut self, input_event: Gd<InputEvent>) {
        if let Ok(mouse_motion) = input_event.try_cast::<InputEventMouseMotion>() {
            let mut camera_speed_this_frame: f32 = CAMERA_MOUSE_ROTATION_SPEED;
            if self.aiming {
                camera_speed_this_frame *= 0.75;
            }
            self.rotate_camera(mouse_motion.get_screen_relative() * camera_speed_this_frame);
        }
    }
}

#[godot_api]
impl PlayerInputSynchronizer {
    #[func]
    pub(crate) fn get_aim_rotation(&self) -> f64 {
        let camera_x_rot: f32 = self
            .camera_rot
            .as_ref()
            .unwrap()
            .get_rotation()
            .x
            .clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX);
        // Change aim according to camera rotation.
        if camera_x_rot >= 0.0 {
            // Aim up.
            (-camera_x_rot / CAMERA_X_ROT_MAX) as f64
        } else {
            // Aim down.
            (camera_x_rot / CAMERA_X_ROT_MIN) as f64
        }
    }

    #[func]
    pub(crate) fn get_camera_base_quaternion(&self) -> Quaternion {
        self.camera_base
            .as_ref()
            .unwrap()
            .get_global_transform()
            .basis
            .get_quaternion()
    }

    #[func]
    pub(crate) fn get_camera_rotation_basis(&self) -> Basis {
        self.camera_rot.as_ref().unwrap().get_global_transform().basis
    }

    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self) {
        self.jumping = true;
    }
}

impl PlayerInputSynchronizer {
    fn rotate_camera(&mut self, mv: Vector2) {
        let camera_base = self.camera_base.as_mut().unwrap();
        camera_base.rotate_y(-mv.x);
        // After relative transforms, camera needs to be renormalized.
        camera_base.orthonormalize();
        let camera_rot = self.camera_rot.as_mut().unwrap();
        let mut rotation = camera_rot.get_rotation();
        rotation.x = (rotation.x + mv.y).clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX);
        camera_rot.set_rotation(rotation);
    }
}
