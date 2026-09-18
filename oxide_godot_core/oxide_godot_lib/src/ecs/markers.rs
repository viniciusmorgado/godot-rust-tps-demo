//! Resources and marker components shared by the sync layer and the gameplay systems
//! (data-model.md "Components and markers"). No engine handle lives here.

use bevy_ecs::prelude::{Component, Resource};
use godot::builtin::Vector3;

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
