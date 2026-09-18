//! The input node's pure core as a gameplay system (constitution 1.5.1, Principle III;
//! specs/012 research R7): v2's `process` decisions (`player_input.rs:92-115`, `:146`) and the
//! mouse look of `input` (`:150-156`) on the entity's components, calling
//! `player_input/model.rs` unchanged. No `Gd`, no `NonSend`, no engine.

use bevy_ecs::prelude::*;
use godot::builtin::Vector2;

use super::model::{PlayerInputTuning, alpha_for_height, scaled_look, scaled_mouse_look, step_aim};
use crate::ecs::markers::{
    AimStateC, CameraFrame, FrameDelta, FrameIntents, InputSnapshotC, OwnsInput, PendingMouseLook,
    ReplicatedInput, Tuning,
};

/// `input_decide`'s query (a `type` so clippy's `type_complexity` stays quiet).
type InputDecideQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static InputSnapshotC,
        &'static CameraFrame,
        &'static mut PendingMouseLook,
        &'static mut AimStateC,
        &'static mut FrameIntents,
        &'static mut ReplicatedInput,
    ),
    With<OwnsInput>,
>;

/// `Phase::Gameplay` of the FRAME schedule, `OwnsInput` entities only. The camera deltas come
/// out in v2's application order — every queued mouse motion first (v2's `input` ran before
/// `process`, with the aim state the previous frame left), then the controller look (`:94`) —
/// and `camera_and_ray` applies them in that order with `clamp_pitch` on the live rotation.
pub fn input_decide(
    tuning: Res<Tuning<PlayerInputTuning>>,
    dt: Res<FrameDelta>,
    mut players: InputDecideQuery,
) {
    let tuning = &tuning.0;
    let dt = dt.0 as f32;
    for (snapshot, camera, mut pending, mut aim_state, mut intents, mut replicated) in &mut players {
        let snapshot = snapshot.0;
        let aiming_before = aim_state.0.is_aiming();

        let mut camera_deltas: Vec<Vector2> = pending
            .0
            .drain(..)
            .map(|screen_relative| scaled_mouse_look(screen_relative, aiming_before, tuning))
            .collect();
        camera_deltas.push(scaled_look(snapshot.camera_move, aiming_before, dt, tuning));

        replicated.motion = snapshot.motion;

        let (next_state, cue) = step_aim(aim_state.0, &snapshot, dt, tuning);
        aim_state.0 = next_state;
        replicated.aiming = next_state.is_aiming();

        replicated.shooting = snapshot.shoot_pressed;

        *intents = FrameIntents {
            camera_deltas,
            cue,
            jump_pressed: snapshot.jump_just_pressed,
            shooting: snapshot.shoot_pressed,
            fade_alpha: alpha_for_height(camera.parent_y, camera.fade_alpha, dt, tuning),
        };
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use godot::builtin::Vector3;

    use super::*;
    use crate::player_input::model::{AimState, CameraCue, InputSnapshot};

    const DT: f64 = 1.0 / 60.0;

    fn snapshot() -> InputSnapshot {
        InputSnapshot {
            motion: Vector2::ZERO,
            camera_move: Vector2::ZERO,
            aim_just_pressed: false,
            aim_pressed: false,
            aim_just_released: false,
            jump_just_pressed: false,
            shoot_pressed: false,
        }
    }

    fn camera() -> CameraFrame {
        CameraFrame { parent_y: 0.0, fade_alpha: 0.0 }
    }

    fn world_with(snapshot: InputSnapshot, camera: CameraFrame, aim: AimState) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Tuning(PlayerInputTuning::default()));
        world.insert_resource(FrameDelta(DT));
        let player = world
            .spawn((
                InputSnapshotC(snapshot),
                camera,
                PendingMouseLook::default(),
                AimStateC(aim),
                FrameIntents::default(),
                ReplicatedInput {
                    aiming: false,
                    shoot_target: Vector3::ZERO,
                    motion: Vector2::ZERO,
                    shooting: false,
                },
                OwnsInput,
            ))
            .id();
        (world, player)
    }

    /// Runs one frame with the given snapshot, returning the intents.
    fn step(world: &mut World, player: Entity, snapshot: InputSnapshot) -> FrameIntents {
        world.entity_mut(player).insert(InputSnapshotC(snapshot));
        world.run_system_once(input_decide).unwrap();
        world.get::<FrameIntents>(player).unwrap().clone()
    }

    #[test]
    fn controller_look_is_scaled() {
        let tuning = PlayerInputTuning::default();
        let mut snap = snapshot();
        snap.camera_move = Vector2::new(1.0, -0.5);

        let (mut world, player) = world_with(snap, camera(), AimState::Idle);
        let intents = step(&mut world, player, snap);
        assert_eq!(intents.camera_deltas, vec![scaled_look(snap.camera_move, false, DT as f32, &tuning)]);

        let (mut world, player) = world_with(snap, camera(), AimState::Toggled { seconds: 1.0 });
        let intents = step(&mut world, player, snap);
        assert_eq!(intents.camera_deltas, vec![scaled_look(snap.camera_move, true, DT as f32, &tuning)]);
    }

    #[test]
    fn mouse_look_is_applied_before_controller_look() {
        let tuning = PlayerInputTuning::default();
        let mut snap = snapshot();
        snap.camera_move = Vector2::new(1.0, 0.0);
        let (mut world, player) = world_with(snap, camera(), AimState::Idle);
        let mouse = Vector2::new(20.0, -10.0);
        world.get_mut::<PendingMouseLook>(player).unwrap().0.push(mouse);

        let intents = step(&mut world, player, snap);
        assert_eq!(intents.camera_deltas[0], scaled_mouse_look(mouse, false, &tuning));
        assert_eq!(intents.camera_deltas[1], scaled_look(snap.camera_move, false, DT as f32, &tuning));
        assert_eq!(intents.camera_deltas.len(), 2);
        assert!(world.get::<PendingMouseLook>(player).unwrap().0.is_empty());
    }

    #[test]
    fn aim_hold_and_toggle_reach_v2_cues() {
        // Hold: press, keep pressed past the 0.4 s threshold, release → `Shoot` then `Far`.
        let (mut world, player) = world_with(snapshot(), camera(), AimState::Idle);
        let mut press = snapshot();
        press.aim_just_pressed = true;
        press.aim_pressed = true;
        assert_eq!(step(&mut world, player, press).cue, Some(CameraCue::Shoot));
        let mut hold = snapshot();
        hold.aim_pressed = true;
        for _ in 0..30 {
            assert_eq!(step(&mut world, player, hold).cue, None);
        }
        let mut release = snapshot();
        release.aim_just_released = true;
        let intents = step(&mut world, player, release);
        assert_eq!(intents.cue, Some(CameraCue::Far));
        assert!(!world.get::<ReplicatedInput>(player).unwrap().aiming);

        // Tap: press, release within the threshold → stays aiming (`Toggled`), no cue.
        let (mut world, player) = world_with(snapshot(), camera(), AimState::Idle);
        assert_eq!(step(&mut world, player, press).cue, Some(CameraCue::Shoot));
        let intents = step(&mut world, player, release);
        assert_eq!(intents.cue, None);
        assert!(matches!(world.get::<AimStateC>(player).unwrap().0, AimState::Toggled { .. }));
        assert!(world.get::<ReplicatedInput>(player).unwrap().aiming);
    }

    #[test]
    fn fade_alpha_follows_height() {
        let (mut world, player) = world_with(snapshot(), CameraFrame { parent_y: -32.0, fade_alpha: 0.0 }, AimState::Idle);
        assert_eq!(step(&mut world, player, snapshot()).fade_alpha, 1.0);

        let (mut world, player) = world_with(snapshot(), CameraFrame { parent_y: 0.0, fade_alpha: 1.0 }, AimState::Idle);
        let alpha = step(&mut world, player, snapshot()).fade_alpha;
        assert!(alpha < 1.0 && alpha > 0.0);
        assert_eq!(alpha, alpha_for_height(0.0, 1.0, DT as f32, &PlayerInputTuning::default()));
    }

    #[test]
    fn jump_just_pressed_sets_intent_for_one_frame() {
        let (mut world, player) = world_with(snapshot(), camera(), AimState::Idle);
        let mut jump = snapshot();
        jump.jump_just_pressed = true;
        assert!(step(&mut world, player, jump).jump_pressed);
        assert!(!step(&mut world, player, snapshot()).jump_pressed);
    }
}
