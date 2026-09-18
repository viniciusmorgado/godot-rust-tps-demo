//! The timer component (research.md R5, FR-011): the engine's own `SceneTreeTimer` arithmetic.

/// Counts `time_left` down exactly as `SceneTree::process_timers` does — `time_left -= p_delta;
/// if (time_left <= 0) emit` — so that a tick timer expires on the same step as the
/// `SceneTreeTimer` it replaces for EVERY duration, not only the ones that happen to agree under
/// an accumulate-and-compare form. `expired` makes the expiry fire exactly once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timer {
    time_left: f64,
    expired: bool,
}

impl Timer {
    pub fn new(seconds: f64) -> Self {
        Self { time_left: seconds, expired: false }
    }

    /// Returns `true` on the one step at which the countdown reaches zero (or below).
    pub fn step(&mut self, dt: f64) -> bool {
        self.time_left -= dt;
        if !self.expired && self.time_left <= 0.0 {
            self.expired = true;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn expires_on_the_step_that_reaches_zero() {
        // R1 fact 6: 0.2 s at 1/60 fires on the 13th step — after 12 subtractions the residual
        // is a positive 4.86e-17, exactly as the engine observed.
        let mut t = Timer::new(0.2);
        for _ in 1..=12 {
            assert!(!t.step(DT));
        }
        assert!(t.step(DT));
    }

    #[test]
    fn does_not_expire_one_step_before() {
        let mut t = Timer::new(0.2);
        let fired: Vec<bool> = (1..=12).map(|_| t.step(DT)).collect();
        assert!(fired.iter().all(|f| !f));
    }

    #[test]
    fn fires_exactly_once() {
        let mut t = Timer::new(0.2);
        for _ in 1..=12 {
            t.step(DT);
        }
        assert!(t.step(DT));
        assert!(!t.step(DT));
    }

    #[test]
    fn three_seconds_at_sixty_hz_fires_on_step_181() {
        let mut t = Timer::new(3.0);
        let mut fired_on = None;
        for step in 1..=200 {
            if t.step(DT) {
                fired_on = Some(step);
                break;
            }
        }
        assert_eq!(fired_on, Some(181));
    }
}
