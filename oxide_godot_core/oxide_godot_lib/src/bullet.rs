use crate::hittable;
use crate::settings::Settings;
use godot::classes::{
    AnimationPlayer, CharacterBody3D, CollisionShape3D, ICharacterBody3D, KinematicCollision3D,
    Node3D, OmniLight3D,
};
use godot::prelude::*;

use pure::BulletState;

/// Replaces `hit: bool` + `time_alive: f32` (an invalid-state-admitting pair — nothing stopped
/// `time_alive` from continuing to count down after `hit` was already `true`).
pub(crate) mod pure {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum BulletState {
        Flying { time_alive: f32 },
        Exploded,
    }

    /// `v1`: `bullet.rs:43-47` — decrement `time_alive`; if it drops below `0.0`, transition to
    /// `Exploded` and report "explode now" (the `bool`); an already-`Exploded` state is a no-op.
    pub fn step(state: BulletState, dt: f32) -> (BulletState, bool) {
        match state {
            BulletState::Exploded => (BulletState::Exploded, false),
            BulletState::Flying { time_alive } => {
                let time_alive = time_alive - dt;
                if time_alive < 0.0 {
                    (BulletState::Exploded, true)
                } else {
                    (BulletState::Flying { time_alive }, false)
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn time_alive_decrements_and_stays_flying() {
            let (state, expired) = step(BulletState::Flying { time_alive: 5.0 }, 0.1);
            assert_eq!(state, BulletState::Flying { time_alive: 4.9 });
            assert_eq!(expired, false);
        }

        #[test]
        fn crossing_zero_transitions_to_exploded_and_reports_expiry() {
            let (state, expired) = step(BulletState::Flying { time_alive: 0.05 }, 0.1);
            assert_eq!(state, BulletState::Exploded);
            assert_eq!(expired, true);
        }

        #[test]
        fn exploded_stays_exploded_no_retrigger() {
            let (state, expired) = step(BulletState::Exploded, 0.1);
            assert_eq!(state, BulletState::Exploded);
            assert_eq!(expired, false);
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Bullet {
    base: Base<CharacterBody3D>,

    #[init(val = BulletState::Flying { time_alive: 5.0 })]
    state: BulletState,

    #[init(node = "AnimationPlayer")]
    animation_player: OnReady<Gd<AnimationPlayer>>,
    #[init(node = "CollisionShape3D")]
    collision_shape: OnReady<Gd<CollisionShape3D>>,
    #[init(node = "OmniLight3D")]
    omni_light: OnReady<Gd<OmniLight3D>>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

impl Bullet {
    pub const VELOCITY: f32 = 20.0;
}

#[godot_api]
impl ICharacterBody3D for Bullet {
    fn ready(&mut self) {
        if !self.base().get_multiplayer().unwrap().is_server() {
            self.base_mut().set_physics_process(false);
            self.collision_shape.set_disabled(true);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // Mirrors v1's `if self.hit { return; }` — an already-exploded bullet does nothing on
        // later frames.
        if self.state == BulletState::Exploded {
            return;
        }

        let dt = delta as f32;
        let (new_state, expired) = pure::step(self.state, dt);
        self.state = new_state;
        if expired {
            self.base_mut().rpc("explode", &[]);
        }

        // The expiry frame itself still moves/collides — v1 never returns early here, only on
        // LATER frames once `hit` (now `state == Exploded`) was already true at frame start.
        let displacement: Vector3 =
            -dt * Self::VELOCITY * self.base().get_transform().basis.col_c();
        let col: Option<Gd<KinematicCollision3D>> = self.base_mut().move_and_collide(displacement);
        if let Some(col) = col {
            let collider: Option<Gd<Node3D>> =
                col.get_collider().and_then(|c| c.try_cast::<Node3D>().ok());
            if let Some(collider) = collider
                && let Some(mut target) = hittable::resolve(collider)
            {
                target.rpc_hit();
            }
            self.collision_shape.set_disabled(true);
            // Backlog #13: suppress the duplicate `explode` when this same tick already
            // exploded the bullet via expiry above.
            if matches!(self.state, BulletState::Flying { .. }) {
                self.base_mut().rpc("explode", &[]);
            }
            // v1's trailing `self.hit = true` (bullet.rs:61) — unconditional, so a
            // non-expired bullet that just collided stops moving/colliding from here on.
            self.state = BulletState::Exploded;
        }
    }
}

#[godot_api]
impl Bullet {
    #[rpc(authority, call_local, unreliable)]
    fn explode(&mut self) {
        self.animation_player.play_ex().name("explode").done();

        // Only enable shadows for the explosion, as the moving light
        // is very small and doesn't noticeably benefit from shadow mapping.
        if self.settings.bind().graphics().shadow_mapping {
            self.omni_light.set_shadow(true);
        }
    }

    #[func]
    fn destroy(&mut self) {
        if !self.base().get_multiplayer().unwrap().is_server() {
            return;
        }
        self.base_mut().queue_free();
    }
}
