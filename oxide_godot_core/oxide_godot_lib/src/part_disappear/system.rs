//! The puff's pure core (constitution 1.5.1, Principle III): v2's two sequential `godot::task`
//! awaits as one explicit phase enum stepped by the frame schedule. No `Gd`, no engine.

use bevy_ecs::prelude::*;

use crate::ecs::markers::{FrameDelta, Remove, StartEmitting};
use crate::ecs::timer::Timer;

/// The two waits of v2 `part_disappear.rs:24-46`, made explicit.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum DisappearPhase {
    /// `await create_timer(0.2).timeout` → then `emitting = true`.
    WaitingToEmit(Timer),
    /// `await create_timer(lifetime * 2.0).timeout` → then `queue_free()`.
    Emitting(Timer),
}

/// The node's `CPUParticles3D.lifetime`, read once in `ready` (v2 `part_disappear.rs:35`;
/// `get_lifetime()` returns `f64` in the gdext 0.5.5 bindings).
#[derive(Component, Clone, Copy)]
pub struct Lifetime(pub f64);

#[derive(Debug, PartialEq)]
pub enum Transition {
    None,
    StartEmitting,
    Finished,
}

impl DisappearPhase {
    /// Tuning of this type (constitution Principle III: tuning constants belong to the type).
    /// v2 `part_disappear.rs:26` — `create_timer(0.2)`.
    pub const EMIT_DELAY: f64 = 0.2;
    /// v2 `part_disappear.rs:37` — `lifetime * 2.0` (an `f64` product: `get_lifetime()` is `f64`).
    pub const LIFETIME_FACTOR: f64 = 2.0;

    /// The registration path (`Initial::Puff`) calls this and never sees the literal.
    pub fn start() -> Self {
        DisappearPhase::WaitingToEmit(Timer::new(Self::EMIT_DELAY))
    }

    /// One frame step. The second timer's length is `lifetime * 2.0` in `f64`, exactly the value
    /// v2 passed to `create_timer`.
    pub fn step(&mut self, dt: f64, lifetime: f64) -> Transition {
        match self {
            DisappearPhase::WaitingToEmit(timer) => {
                if timer.step(dt) {
                    *self = DisappearPhase::Emitting(Timer::new(lifetime * Self::LIFETIME_FACTOR));
                    Transition::StartEmitting
                } else {
                    Transition::None
                }
            }
            DisappearPhase::Emitting(timer) => {
                if timer.step(dt) {
                    Transition::Finished
                } else {
                    Transition::None
                }
            }
        }
    }
}

/// `Phase::Gameplay` of the FRAME schedule (research.md R8): steps every puff's phase with the
/// frame delta and flags the sync layer — `StartEmitting` consumed by `sync_out_puff`, `Remove`
/// by `sync_out_remove` — in the same run.
pub fn advance(
    dt: Res<FrameDelta>,
    mut puffs: Query<(Entity, &mut DisappearPhase, &Lifetime)>,
    mut commands: Commands,
) {
    for (entity, mut phase, lifetime) in &mut puffs {
        match phase.step(dt.0, lifetime.0) {
            Transition::StartEmitting => {
                commands.entity(entity).insert(StartEmitting);
            }
            Transition::Finished => {
                commands.entity(entity).insert(Remove);
            }
            Transition::None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;

    use super::*;

    const DT: f64 = 1.0 / 60.0;
    /// The scene's `lifetime = 1.5` (`part_disappear.tscn:46`).
    const LIFETIME: f64 = 1.5;

    fn world_with_puff(phase: DisappearPhase) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(FrameDelta(DT));
        let puff = world.spawn((phase, Lifetime(LIFETIME))).id();
        (world, puff)
    }

    fn run(world: &mut World) {
        world.run_system_once(advance).unwrap();
    }

    fn is_emitting_phase(world: &World, puff: Entity) -> bool {
        matches!(world.get::<DisappearPhase>(puff), Some(DisappearPhase::Emitting(_)))
    }

    #[test]
    fn phases_occur_in_order() {
        let (mut world, puff) = world_with_puff(DisappearPhase::start());
        let mut order = Vec::new();
        for _ in 0..200 {
            run(&mut world);
            if world.get::<StartEmitting>(puff).is_some() && !order.contains(&"emitting") {
                assert!(is_emitting_phase(&world, puff));
                assert!(order.is_empty(), "StartEmitting must come first");
                order.push("emitting");
                world.entity_mut(puff).remove::<StartEmitting>();
            }
            if world.get::<Remove>(puff).is_some() && !order.contains(&"remove") {
                assert_eq!(order, vec!["emitting"], "Remove must follow StartEmitting");
                order.push("remove");
            }
        }
        assert_eq!(order, vec!["emitting", "remove"]);
    }

    #[test]
    fn waiting_expires_on_step_13_at_sixty_hz_and_starts_emitting_once() {
        let (mut world, puff) = world_with_puff(DisappearPhase::start());
        for _ in 1..=12 {
            run(&mut world);
            assert!(world.get::<StartEmitting>(puff).is_none());
        }
        run(&mut world); // step 13
        assert!(world.get::<StartEmitting>(puff).is_some());
        assert!(is_emitting_phase(&world, puff));
        // As `sync_out_puff` would: consume the marker, then step once more.
        world.entity_mut(puff).remove::<StartEmitting>();
        run(&mut world); // step 14
        assert!(world.get::<StartEmitting>(puff).is_none());
    }

    #[test]
    fn emitting_expires_after_lifetime_times_two_and_finishes_once() {
        let emitting = DisappearPhase::Emitting(Timer::new(LIFETIME * DisappearPhase::LIFETIME_FACTOR));
        let (mut world, puff) = world_with_puff(emitting);
        // 3.0 s at 1/60 reaches <= 0 on step 181 (data-model.md, research R5).
        for _ in 1..=180 {
            run(&mut world);
            assert!(world.get::<Remove>(puff).is_none());
        }
        run(&mut world); // step 181
        assert!(world.get::<Remove>(puff).is_some());
    }

    #[test]
    fn no_double_fire_past_the_end() {
        let (mut world, puff) = world_with_puff(DisappearPhase::start());
        let mut start_emitting_count = 0;
        let mut finished_seen = false;
        for _ in 0..(13 + 181 + 10) {
            run(&mut world);
            if world.get::<StartEmitting>(puff).is_some() {
                start_emitting_count += 1;
                world.entity_mut(puff).remove::<StartEmitting>();
            }
            if world.get::<Remove>(puff).is_some() {
                finished_seen = true;
            }
        }
        assert_eq!(start_emitting_count, 1);
        assert!(finished_seen);
        // The `Remove` marker is a single component: it cannot be "counted twice"; the timer's
        // `expired` flag guarantees `Transition::Finished` fired once, so 10 extra steps added no
        // second StartEmitting.
        assert!(matches!(world.get::<DisappearPhase>(puff), Some(DisappearPhase::Emitting(_))));
    }
}
