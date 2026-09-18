//! The input node's engine systems (constitution 1.5.1 "ECS shape (v3)"; specs/012 research
//! R7): `SyncIn`, the `EngineQuery` member and `SyncOut` of the FRAME schedule, for the entity
//! whose input this peer owns. Every engine call of v2's `process`/`input`/`rotate_camera` lives
//! here, in v2's order; the decisions are in `player_input/system.rs`.

use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use godot::classes::{Input, PhysicsRayQueryParameters3D};
use godot::prelude::*;

use super::model::{CameraCue, InputSnapshot, PlayerInputTuning, clamp_pitch};
use crate::ecs::markers::{
    CameraFrame, FrameIntents, InputSnapshotC, OwnsInput, PlayerTag, ReplicatedInput, Tuning,
};
use crate::ecs::{Handles, NodeHandles};

/// `SyncIn`, FRAME schedule, `OwnsInput` only (FR-012, FR-013): the ten `Input` reads in v2's
/// order (`player_input.rs:76-90`), the parent's global y and the fade alpha (`:144-145`).
/// Nothing is read for other players: the fixed `sync_in_player` reads the projection.
pub fn sync_in_input(
    handles: NonSend<NodeHandles>,
    players: Query<(Entity, Has<OwnsInput>), With<PlayerTag>>,
    mut commands: Commands,
) {
    for (entity, owns_input) in &players {
        if !owns_input {
            continue;
        }
        let Some(Handles::Player(p)) = handles.by_entity.get(&entity) else {
            continue;
        };
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
        let camera = CameraFrame {
            parent_y: p.root.get_global_transform().origin.y,
            fade_alpha: p.color_rect.get_modulate().a,
        };
        commands.entity(entity).insert((InputSnapshotC(snapshot), camera));
    }
}

/// `EngineQuery`, FRAME schedule, `OwnsInput` only (FR-014): each camera delta applied as v2's
/// `rotate_camera` did (`player_input.rs:184-191`, `clamp_pitch` on the live rotation, in
/// order — mouse first, then controller), THEN the crosshair raycast when shooting
/// (`:117-138`), which needs the camera rotated THIS frame.
pub fn camera_and_ray(
    tuning: Res<Tuning<PlayerInputTuning>>,
    mut handles: NonSendMut<NodeHandles>,
    mut players: Query<(Entity, &FrameIntents, &mut ReplicatedInput), With<OwnsInput>>,
) {
    for (entity, intents, mut replicated) in &mut players {
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        for mv in &intents.camera_deltas {
            p.camera_base.rotate_y(-mv.x);
            // After relative transforms, camera needs to be renormalized.
            p.camera_base.orthonormalize();
            let mut rotation = p.camera_rot.get_rotation();
            rotation.x = clamp_pitch(rotation.x, mv.y, &tuning.0);
            p.camera_rot.set_rotation(rotation);
        }

        if intents.shooting {
            let ch_pos = p.crosshair.get_position() + p.crosshair.get_size() * 0.5;
            let ray_from = p.camera.project_ray_origin(ch_pos);
            let ray_dir = p.camera.project_ray_normal(ch_pos);

            let params = PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0)
                .collision_mask(0b11)
                .exclude(&array![p.parent_rid])
                .done()
                .unwrap();
            let col = p
                .root
                .get_world_3d()
                .unwrap()
                .get_direct_space_state()
                .unwrap()
                .intersect_ray(&params);
            if col.is_empty() {
                replicated.shoot_target = ray_from + ray_dir * 1000.0;
            } else {
                replicated.shoot_target = col.get("position").unwrap().to::<Vector3>();
            }
        }
    }
}

/// `SyncOut`, FRAME schedule, `OwnsInput` only (FR-012): the camera cue (`player_input.rs:100-109`),
/// the four replicated fields through `input.bind_mut()` (the guard dropped before the next
/// engine call; `:92`, `:99`, `:115`, `:135-137`), the fade (`:145-147`) and the jump RPC
/// (`:111-113`), whose local handler only pushes `JumpPressed`.
pub fn sync_out_input(
    mut handles: NonSendMut<NodeHandles>,
    players: Query<(Entity, &FrameIntents, &ReplicatedInput), With<OwnsInput>>,
) {
    for (entity, intents, replicated) in &players {
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        if let Some(cue) = intents.cue {
            match cue {
                CameraCue::Shoot => {
                    p.camera_anim.play_ex().name("shoot").done();
                }
                CameraCue::Far => {
                    p.camera_anim.play_ex().name("far").done();
                }
            }
        }

        {
            let mut input = p.input.bind_mut();
            input.aiming = replicated.aiming;
            input.shoot_target = replicated.shoot_target;
            input.motion = replicated.motion;
            input.shooting = replicated.shooting;
        }

        // Fade out to black if falling out of the map. -17 is lower than
        // the lowest valid position checked the map (which is a bit under -16).
        // At 15 units below -17 (so -32), the screen turns fully black.
        let mut modulate = p.color_rect.get_modulate();
        modulate.a = intents.fade_alpha;
        p.color_rect.set_modulate(modulate);

        if intents.jump_pressed {
            p.input.rpc("jump", &[]);
        }
    }
}
