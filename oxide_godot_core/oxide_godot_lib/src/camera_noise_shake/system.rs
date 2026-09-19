//! The camera shake's pure core as a gameplay system (constitution 1.5.1, Principle III;
//! specs/012 research R7): v2's `process` decisions (`camera_noise_shake.rs:45-49`) on the
//! player entity's `Trauma`/`ShakeTime`, calling `camera_noise_shake/model.rs` unchanged. No
//! `Gd`, no `NonSend`, no engine.

use bevy_ecs::prelude::*;

use super::model::{CameraShakeTuning, advance_time, decay, shake};
use crate::ecs::markers::{FrameDelta, PlayerTag, ShakePending, ShakeTime, Trauma, Tuning};

/// `Phase::Gameplay` of the FRAME schedule: while `Trauma > 0` (v2 `:45`), `decay`,
/// `advance_time`, `shake` in v2's order (`:47-49`) → `ShakePending(Some((shake, time)))`,
/// consumed by `sync_out_shake` in the same run; otherwise `None` and nothing is written.
pub fn shake_decide(
    tuning: Res<Tuning<CameraShakeTuning>>,
    dt: Res<FrameDelta>,
    mut players: Query<(&mut Trauma, &mut ShakeTime, &mut ShakePending), With<PlayerTag>>,
) {
    let tuning = &tuning.0;
    for (mut trauma, mut time, mut pending) in &mut players {
        if trauma.0 > 0.0 {
            trauma.0 = decay(trauma.0, dt.0 as f32, tuning);
            time.0 = advance_time(time.0, dt.0, tuning);
            pending.0 = Some((shake(trauma.0), time.0));
        } else {
            pending.0 = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;

    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn world_with(trauma: f32, time: f64) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Tuning(CameraShakeTuning::default()));
        world.insert_resource(FrameDelta(DT));
        let player = world.spawn((Trauma(trauma), ShakeTime(time), ShakePending::default(), PlayerTag)).id();
        (world, player)
    }

    #[test]
    fn shake_runs_only_while_trauma_positive() {
        let (mut world, player) = world_with(0.0, 3.0);
        world.run_system_once(shake_decide).unwrap();
        assert_eq!(world.get::<ShakePending>(player).unwrap().0, None);
        assert_eq!(world.get::<ShakeTime>(player).unwrap().0, 3.0);

        let (mut world, player) = world_with(0.5, 3.0);
        world.run_system_once(shake_decide).unwrap();
        assert!(world.get::<ShakePending>(player).unwrap().0.is_some());
    }

    #[test]
    fn decay_and_time_advance_in_v2_order() {
        let tuning = CameraShakeTuning::default();
        let (mut world, player) = world_with(0.5, 3.0);
        world.run_system_once(shake_decide).unwrap();
        let expected_trauma = decay(0.5, DT as f32, &tuning);
        let expected_time = advance_time(3.0, DT, &tuning);
        assert_eq!(world.get::<Trauma>(player).unwrap().0, expected_trauma);
        assert_eq!(world.get::<ShakeTime>(player).unwrap().0, expected_time);
        assert_eq!(
            world.get::<ShakePending>(player).unwrap().0,
            Some((shake(expected_trauma), expected_time))
        );
    }
}
