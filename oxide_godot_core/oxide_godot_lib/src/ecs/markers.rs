//! Resources and marker components shared by the sync layer and the gameplay systems
//! (data-model.md "Components and markers"). No engine handle lives here.

use bevy_ecs::prelude::{Component, Resource};
use godot::builtin::{Quaternion, Transform3D, Vector2, Vector3};

use super::event::PlayerFx;
use crate::player::model::{AnimPlan, InputFrame};
use crate::player::Animations;
use crate::player_input::model::{AimState, CameraCue, InputSnapshot};

/// Written by `EcsWorld::process` before the frame schedule runs; systems never read the engine's
/// delta themselves.
#[derive(Resource, Clone, Copy)]
pub struct FrameDelta(pub f64);

/// Written by `EcsWorld::physics_process` before the fixed schedule runs.
#[derive(Resource, Clone, Copy)]
pub struct FixedDelta(pub f64);

/// Consumed by `sync_out_door`: play `doorsimple_opening` once.
#[derive(Component)]
pub struct PlayOpen;

/// Consumed by `sync_out_puff`: `emitting = true` once.
#[derive(Component)]
pub struct StartEmitting;

/// Consumed by `sync_out_remove`: `queue_free()` the node, despawn the entity, drop both maps.
#[derive(Component)]
pub struct Remove;

/// Selects blast entities in `sync_in_blast`/`sync_out_blast`.
#[derive(Component)]
pub struct BlastTag;

/// The camera origin read by `sync_in_blast` (written only when it changed, so `Changed<_>` is a
/// real gate) and consumed by `sync_out_blast`'s `look_at`. `Vector3` is an FFI-free value type,
/// allowed in pure code by Principle III.
#[derive(Component, Clone, Copy, PartialEq)]
pub struct LookTarget(pub Vector3);

/// A model tuning struct as a resource (constitution 1.5.1: "tuning structs become
/// `Res<XxxTuning>`"), without editing the model files: `build_world` inserts
/// `Tuning(PlayerTuning::default())`, `Tuning(PlayerInputTuning::default())` and
/// `Tuning(CameraShakeTuning::default())` (specs/012 research R6). The three tuning types are
/// plain `Copy` structs of `f32`/`f64`, so the `Send + Sync + 'static` bound holds.
#[derive(Resource)]
pub struct Tuning<T: Send + Sync + 'static>(pub T);

// ---- The player entity (specs/012 data-model.md "Components") ----------------------------

// identity / authority (registration)

/// Selects player entities in the sync systems.
#[derive(Component)]
pub struct PlayerTag;
/// This peer simulates the entity (v2 `player.rs:98`, `multiplayer.is_server()`).
#[derive(Component)]
pub struct Simulates;
/// This peer owns the entity's input node (v2 `player_input.rs:61-62`).
#[derive(Component)]
pub struct OwnsInput;
/// `player_id` (v2 `player.rs:78`).
#[derive(Component, Clone, Copy)]
pub struct PeerId(pub i32);

// persistent tick state (player/system.rs)

/// v2 `player.rs:44`.
#[derive(Component, Clone, Copy)]
pub struct Motion(pub Vector2);
/// v2 `player.rs:41`.
#[derive(Component, Clone, Copy)]
pub struct Orientation(pub Transform3D);
/// v2 `player.rs:42`.
#[derive(Component, Clone, Copy)]
pub struct RootMotion(pub Transform3D);
/// v2 `player.rs:38-39` (starts at 0, backlog #10).
#[derive(Component, Clone, Copy)]
pub struct AirborneTime(pub f32);
/// v2 `player.rs:46`.
#[derive(Component, Clone, Copy)]
pub struct InitialPosition(pub Vector3);
/// v2 `player.rs:82`.
#[derive(Component, Clone, Copy, PartialEq)]
pub struct CurrentAnimation(pub Animations);
/// v2 `player_input.rs:22` — the model's enum wrapped so `player_input/model.rs` stays untouched.
#[derive(Component, Clone, Copy)]
pub struct AimStateC(pub AimState);

// queued by the drain (ecs/apply.rs), consumed by one system

/// Replaces v2 `player_input.rs:41` `jumping`.
#[derive(Component, Default)]
pub struct JumpQueued(pub bool);
/// Mouse-motion deltas in arrival order, consumed by `input_decide`.
#[derive(Component, Default)]
pub struct PendingMouseLook(pub Vec<Vector2>);
/// Remote-peer RPC effects in arrival order, consumed by `apply_player_fx`.
#[derive(Component, Default)]
pub struct PendingFx(pub Vec<PlayerFx>);

// per-run snapshots (written by SyncIn)

/// v2 `player/model.rs:38-48` wrapped.
#[derive(Component, Clone, Copy)]
pub struct InputFrameC(pub InputFrame);
/// The body reads of one physics step; `origin_y` is written ONLY by `move_body`, post-move
/// (v2 read the origin once at `player.rs:329`).
#[derive(Component, Clone, Copy)]
pub struct BodyState {
    pub on_floor: bool,
    pub velocity: Vector3,
    pub gravity: Vector3,
    pub cooldown_left: f64,
    pub origin_y: f32,
}
/// v2 `player_input.rs:32-39` — frame run, `OwnsInput` only (the values this peer projects);
/// the fixed run reads the node directly into `InputFrameC`.
#[derive(Component, Clone, Copy)]
pub struct ReplicatedInput {
    pub aiming: bool,
    pub shoot_target: Vector3,
    pub motion: Vector2,
    pub shooting: bool,
}
/// v2 `player.rs:104-117` (non-`Simulates` replay input).
#[derive(Component, Clone, Copy)]
pub struct ReplayState {
    pub current_animation: Animations,
    pub motion: Vector2,
    pub aim_rotation: f64,
}
/// v2 `player_input/model.rs:64-72` wrapped.
#[derive(Component, Clone, Copy)]
pub struct InputSnapshotC(pub InputSnapshot);
/// v2 `player_input.rs:144-146`; the live camera rotations are read per delta by
/// `camera_and_ray`, so they are not snapshotted.
#[derive(Component, Clone, Copy)]
pub struct CameraFrame {
    pub parent_y: f32,
    pub fade_alpha: f32,
}

// per-run outputs (research R6)

/// `tick_decide`'s output, read by `orient_and_anim`, `move_body` and `sync_out_player`.
#[derive(Component, Clone, Copy, Default)]
pub struct TickIntents {
    pub land: bool,
    pub jump: bool,
    pub shoot: bool,
    pub respawn: bool,
    pub jump_velocity_y: Option<f32>,
    pub orient: Option<OrientTarget>,
    pub plan: Option<AnimPlan>,
    pub read_root_motion: bool,
}
/// v2 `player.rs:262` (aiming: the camera base quaternion) / `:294` (walking: `walk_target`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrientTarget {
    Camera(Quaternion),
    Walk(Vector3),
}
/// `tick_integrate`'s output (v2 `player.rs:316`), consumed by `move_body`.
#[derive(Component, Clone, Copy)]
pub struct Velocity(pub Vector3);
/// `input_decide`'s output for the frame run.
#[derive(Component, Clone, Default)]
pub struct FrameIntents {
    pub camera_deltas: Vec<Vector2>,
    pub cue: Option<CameraCue>,
    pub jump_pressed: bool,
    pub shooting: bool,
    pub fade_alpha: f32,
}

// shake (camera_noise_shake/system.rs)

/// v2 `camera_noise_shake.rs:15`.
#[derive(Component, Clone, Copy)]
pub struct Trauma(pub f32);
/// v2 `camera_noise_shake.rs:16`.
#[derive(Component, Clone, Copy)]
pub struct ShakeTime(pub f64);
/// v2 `camera_noise_shake.rs:14`, captured at `:41`.
#[derive(Component, Clone, Copy)]
pub struct StartRotation(pub Vector3);
/// `(shake, time)` from `shake_decide`; consumed by `sync_out_shake` (the shake is sync-only:
/// no `ShakeOffset`, no `EngineQuery` member).
#[derive(Component, Clone, Copy, Default)]
pub struct ShakePending(pub Option<(f32, f64)>);
