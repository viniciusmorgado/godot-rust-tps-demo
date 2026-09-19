//! The World's resources and the two schedules with their chained phase sets (research.md R8).
//! Pure: engine-touching systems are added afterwards by `ecs::add_engine_systems`, so this file
//! is testable without Godot.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

use super::event::DoorBodyEntered;
use super::index::EntityIndex;
use super::markers::{FixedDelta, FrameDelta, Tuning};
use super::NodeHandles;
use crate::camera_noise_shake::model::CameraShakeTuning;
use crate::player::model::PlayerTuning;
use crate::player_input::model::PlayerInputTuning;

/// The tick phases (constitution 1.5.1, "ECS shape (v3)"). The FIXED schedule chains the seven
/// `SyncIn → Gameplay → EngineQueryOrient → GameplayIntegrate → EngineQueryMove →
/// GameplaySettle → SyncOut` (specs/012 research R4: the player's tick needs two engine answers
/// mid-tick, each followed by a pure step); the FRAME schedule keeps V3-A's four
/// `SyncIn → Gameplay → EngineQuery → SyncOut`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    SyncIn,
    Gameplay,
    EngineQueryOrient,
    GameplayIntegrate,
    EngineQueryMove,
    GameplaySettle,
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
    // The model tuning structs as resources (specs/012 research R6), their `Default`s untouched.
    world.insert_resource(Tuning(PlayerTuning::default()));
    world.insert_resource(Tuning(PlayerInputTuning::default()));
    world.insert_resource(Tuning(CameraShakeTuning::default()));
    world
}

// `ScheduleBuildSettings::auto_insert_apply_deferred` is deliberately left at its default
// `true` (bevy_ecs 0.19.1 `schedule/schedule.rs:1605`) in both builders below: with the sets
// chained, bevy inserts an `ApplyDeferred` sync point between ordered systems that use
// `Commands`, which is what makes a marker inserted in `Gameplay` visible to `SyncOut` (and to
// `EngineQueryOrient`) of the SAME run (research.md R6; pinned by
// `marker_inserted_in_gameplay_is_visible_in_sync_out_of_the_same_run` and
// `marker_inserted_in_gameplay_is_visible_in_engine_query_orient_of_the_same_run`).

pub fn build_fixed() -> Schedule {
    let mut schedule = Schedule::new(Fixed);
    schedule.configure_sets(
        (
            Phase::SyncIn,
            Phase::Gameplay,
            Phase::EngineQueryOrient,
            Phase::GameplayIntegrate,
            Phase::EngineQueryMove,
            Phase::GameplaySettle,
            Phase::SyncOut,
        )
            .chain(),
    );
    schedule.add_systems(crate::door::system::open_on_player.in_set(Phase::Gameplay));
    schedule.add_systems(crate::player::system::tick_decide.in_set(Phase::Gameplay));
    schedule.add_systems(crate::player::system::tick_integrate.in_set(Phase::GameplayIntegrate));
    schedule.add_systems(crate::player::system::tick_settle.in_set(Phase::GameplaySettle));
    schedule
}

pub fn build_frame() -> Schedule {
    let mut schedule = Schedule::new(Frame);
    schedule.configure_sets((Phase::SyncIn, Phase::Gameplay, Phase::EngineQuery, Phase::SyncOut).chain());
    schedule.add_systems(crate::part_disappear::system::advance.in_set(Phase::Gameplay));
    schedule.add_systems(crate::player_input::system::input_decide.in_set(Phase::Gameplay));
    schedule.add_systems(crate::camera_noise_shake::system::shake_decide.in_set(Phase::Gameplay));
    schedule
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
    fn probe_orient(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::EngineQueryOrient);
    }
    fn probe_integrate(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::GameplayIntegrate);
    }
    fn probe_move(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::EngineQueryMove);
    }
    fn probe_settle(mut trace: ResMut<Trace>) {
        trace.0.push(Phase::GameplaySettle);
    }

    #[test]
    fn phase_sets_are_chained_in_order() {
        let mut world = build_world();
        world.insert_resource(Trace::default());
        // V3-B: the four-set chain now lives in the FRAME schedule only (the fixed chain has no
        // `EngineQuery` member, so a probe in it would be unordered there).
        let mut schedule = build_frame();
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

    #[test]
    fn fixed_sets_are_chained_in_order_and_frame_sets_unchanged() {
        let mut world = build_world();
        world.insert_resource(Trace::default());
        // One probe per set of each schedule, added in REVERSE order.
        let mut fixed = build_fixed();
        fixed.add_systems(probe_sync_out.in_set(Phase::SyncOut));
        fixed.add_systems(probe_settle.in_set(Phase::GameplaySettle));
        fixed.add_systems(probe_move.in_set(Phase::EngineQueryMove));
        fixed.add_systems(probe_integrate.in_set(Phase::GameplayIntegrate));
        fixed.add_systems(probe_orient.in_set(Phase::EngineQueryOrient));
        fixed.add_systems(probe_gameplay.in_set(Phase::Gameplay));
        fixed.add_systems(probe_sync_in.in_set(Phase::SyncIn));
        fixed.run(&mut world);
        assert_eq!(
            world.resource::<Trace>().0,
            vec![
                Phase::SyncIn,
                Phase::Gameplay,
                Phase::EngineQueryOrient,
                Phase::GameplayIntegrate,
                Phase::EngineQueryMove,
                Phase::GameplaySettle,
                Phase::SyncOut
            ]
        );

        world.resource_mut::<Trace>().0.clear();
        let mut frame = build_frame();
        frame.add_systems(probe_sync_out.in_set(Phase::SyncOut));
        frame.add_systems(probe_engine_query.in_set(Phase::EngineQuery));
        frame.add_systems(probe_gameplay.in_set(Phase::Gameplay));
        frame.add_systems(probe_sync_in.in_set(Phase::SyncIn));
        frame.run(&mut world);
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

    #[test]
    fn marker_inserted_in_gameplay_is_visible_in_engine_query_orient_of_the_same_run() {
        let mut world = build_world();
        world.insert_resource(Seen::default());
        let mut schedule = build_fixed();
        schedule.add_systems(spawn_probe.in_set(Phase::Gameplay));
        schedule.add_systems(count_probes.in_set(Phase::EngineQueryOrient));
        schedule.run(&mut world);
        assert_eq!(world.resource::<Seen>().0, 1);
    }
}
