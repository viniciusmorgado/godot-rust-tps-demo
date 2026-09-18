//! The World's resources and the two schedules with their chained phase sets (research.md R8).
//! Pure: engine-touching systems are added afterwards by `ecs::add_engine_systems`, so this file
//! is testable without Godot.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

use super::event::DoorBodyEntered;
use super::index::EntityIndex;
use super::markers::{FixedDelta, FrameDelta};
use super::NodeHandles;

/// The tick phases (constitution 1.5.1, "ECS shape (v3)"), chained in this order in BOTH
/// schedules. `EngineQuery` has no member in V3-A.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    SyncIn,
    Gameplay,
    EngineQuery,
    SyncOut,
}

/// Run by `EcsWorld::physics_process`.
#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fixed;

/// Run by `EcsWorld::process`.
#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame;

pub fn build_world() -> World {
    let mut world = World::new();
    world.insert_resource(EntityIndex::default());
    world.insert_non_send(NodeHandles::default());
    world.insert_resource(Messages::<DoorBodyEntered>::default());
    world.insert_resource(FrameDelta(0.0));
    world.insert_resource(FixedDelta(0.0));
    world
}

fn chained(label: impl ScheduleLabel) -> Schedule {
    let mut schedule = Schedule::new(label);
    // `ScheduleBuildSettings::auto_insert_apply_deferred` is deliberately left at its default
    // `true` (bevy_ecs 0.19.1 `schedule/schedule.rs:1605`): with the sets chained, bevy inserts
    // an `ApplyDeferred` sync point between ordered systems that use `Commands`, which is what
    // makes a marker inserted in `Gameplay` visible to `SyncOut` of the SAME run (research.md R6;
    // pinned by `marker_inserted_in_gameplay_is_visible_in_sync_out_of_the_same_run`).
    schedule.configure_sets((Phase::SyncIn, Phase::Gameplay, Phase::EngineQuery, Phase::SyncOut).chain());
    schedule
}

pub fn build_fixed() -> Schedule {
    chained(Fixed)
}

pub fn build_frame() -> Schedule {
    chained(Frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_schedules_run_on_an_empty_world() {
        let mut world = build_world();
        build_fixed().run(&mut world);
        build_frame().run(&mut world);
    }

    #[derive(Resource, Default)]
    struct Trace(Vec<Phase>);

    fn probe_sync_in(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::SyncIn);
    }
    fn probe_gameplay(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::Gameplay);
    }
    fn probe_engine_query(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::EngineQuery);
    }
    fn probe_sync_out(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::SyncOut);
    }

    #[test]
    fn phase_sets_are_chained_in_order() {
        let mut world = build_world();
        world.insert_resource(Trace::default());
        let mut schedule = build_fixed();
        // Added in REVERSE order so the observed order can only come from the chained sets.
        schedule.add_systems(probe_sync_out.in_set(Phase::SyncOut));
        schedule.add_systems(probe_engine_query.in_set(Phase::EngineQuery));
        schedule.add_systems(probe_gameplay.in_set(Phase::Gameplay));
        schedule.add_systems(probe_sync_in.in_set(Phase::SyncIn));
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<Trace>().0,
            vec![Phase::SyncIn, Phase::Gameplay, Phase::EngineQuery, Phase::SyncOut]
        );
    }

    #[derive(Component)]
    struct ProbeMarker;

    #[derive(Resource, Default)]
    struct Seen(usize);

    fn spawn_probe(mut commands: Commands) {
        commands.spawn(ProbeMarker);
    }
    fn count_probes(probes: Query<(), With<ProbeMarker>>, mut seen: ResMut<Seen>) {
        seen.0 = probes.iter().count();
    }

    #[test]
    fn marker_inserted_in_gameplay_is_visible_in_sync_out_of_the_same_run() {
        let mut world = build_world();
        world.insert_resource(Seen::default());
        let mut schedule = build_frame();
        schedule.add_systems(spawn_probe.in_set(Phase::Gameplay));
        schedule.add_systems(count_probes.in_set(Phase::SyncOut));
        schedule.run(&mut world);
        assert_eq!(world.resource::<Seen>().0, 1);
    }
}
