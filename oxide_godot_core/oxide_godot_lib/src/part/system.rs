//! The part entity's pure gameplay system (constitution 1.5.2 "ECS shape (v3)"; specs/013
//! research R7): the phase machine of v2's `process` + the two `SceneTreeTimer` awaits
//! (`part.rs:138-146`, `:179-191`, `:202-214`) stepped by the FRAME schedule with V3-A's
//! `Timer` arithmetic. No engine call; `fade_curve`/`should_destroy` are `mod pure`'s, untouched.

use bevy_ecs::prelude::*;

use super::pure::{fade_curve, should_destroy};
use crate::ecs::markers::{FrameDelta, PartIntents, PartLifetimes, PartPhase, Remove};
use crate::ecs::timer::Timer;

/// v2 `part.rs:205` — `create_timer(0.2)` between `destroy` and the node's release.
pub const DESTROY_DELAY: f64 = 0.2;

/// `Gameplay`, FRAME schedule, every part: `Waiting(timer)` → expiry starts the fade (v2
/// `:179-191`: the timer callback re-enabled `process`, whose first run wrote `fade_curve(0)`);
/// `Fading { counter }` → the fade intent (`:139`), `counter += dt` (`:141`), then
/// `should_destroy` → the destroy intent and `Destroyed(Timer(0.2))` (`:142-144`, `:203-209`);
/// `Destroyed(timer)` → `Remove` on expiry (`:210-214`). `Attached`: nothing. The intents are
/// rebuilt every frame.
pub fn part_phase_tick(
    dt: Res<FrameDelta>,
    mut parts: Query<(Entity, &mut PartPhase, &PartLifetimes, &mut PartIntents)>,
    mut commands: Commands,
) {
    for (entity, mut phase, lifetimes, mut intents) in &mut parts {
        *intents = PartIntents::default();
        let next = match &mut *phase {
            PartPhase::Attached => None,
            PartPhase::Waiting(timer) => timer.step(dt.0).then_some(PartPhase::Fading { counter: 0.0 }),
            PartPhase::Fading { counter } => {
                intents.fade = Some(fade_curve(*counter, lifetimes.disappearing_time));
                *counter += dt.0 as f32;
                if should_destroy(*counter, lifetimes.disappearing_time) {
                    intents.destroy = true;
                    Some(PartPhase::Destroyed(Timer::new(DESTROY_DELAY)))
                } else {
                    None
                }
            }
            PartPhase::Destroyed(timer) => {
                if timer.step(dt.0) {
                    commands.entity(entity).insert(Remove);
                }
                None
            }
        };
        if let Some(next) = next {
            *phase = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;

    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn world_with(phase: PartPhase) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(FrameDelta(DT));
        let lifetimes = PartLifetimes { lifetime: 3.0, lifetime_random: 3.0, disappearing_time: 0.5 };
        let e = world.spawn((phase, lifetimes, PartIntents::default())).id();
        (world, e)
    }

    fn intents(world: &World, e: Entity) -> PartIntents {
        *world.get::<PartIntents>(e).unwrap()
    }

    #[test]
    fn attached_part_does_nothing() {
        let (mut world, e) = world_with(PartPhase::Attached);
        world.run_system_once(part_phase_tick).unwrap();
        assert_eq!(*world.get::<PartPhase>(e).unwrap(), PartPhase::Attached);
        let i = intents(&world, e);
        assert_eq!(i.fade, None);
        assert!(!i.destroy);
        assert!(world.get::<Remove>(e).is_none());
    }

    #[test]
    fn waiting_timer_expiry_starts_fading() {
        let (mut world, e) = world_with(PartPhase::Waiting(Timer::new(DT)));
        world.run_system_once(part_phase_tick).unwrap();
        assert_eq!(*world.get::<PartPhase>(e).unwrap(), PartPhase::Fading { counter: 0.0 });
        // The expiry frame only transitions: the first fade write is the next frame's (v2 `:190`).
        assert_eq!(intents(&world, e).fade, None);
    }

    #[test]
    fn fading_writes_fade_curve_each_frame() {
        let (mut world, e) = world_with(PartPhase::Fading { counter: 0.25 });
        world.run_system_once(part_phase_tick).unwrap();
        assert_eq!(intents(&world, e).fade, Some(fade_curve(0.25, 0.5)));
        assert_eq!(*world.get::<PartPhase>(e).unwrap(), PartPhase::Fading { counter: 0.25 + DT as f32 });
    }

    #[test]
    fn fading_destroys_at_t_minus_0_2_once() {
        let (mut world, e) = world_with(PartPhase::Fading { counter: 0.29 });
        world.run_system_once(part_phase_tick).unwrap();
        let i = intents(&world, e);
        assert!(i.destroy);
        assert_eq!(i.fade, Some(fade_curve(0.29, 0.5)));
        assert!(matches!(world.get::<PartPhase>(e).unwrap(), PartPhase::Destroyed(_)));
        world.run_system_once(part_phase_tick).unwrap();
        let i = intents(&world, e);
        assert!(!i.destroy);
        assert_eq!(i.fade, None);
        assert!(world.get::<Remove>(e).is_none());
    }

    #[test]
    fn destroyed_timer_expiry_marks_remove() {
        let (mut world, e) = world_with(PartPhase::Destroyed(Timer::new(DT)));
        world.run_system_once(part_phase_tick).unwrap();
        assert!(world.get::<Remove>(e).is_some());
    }
}
