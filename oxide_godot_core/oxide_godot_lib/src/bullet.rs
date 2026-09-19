//! The bullet's bridge (constitution 1.5.2 "ECS shape (v3)"; specs/013 contracts
//! enemy-entities.md): `ready` registers the entity with its handles, `exit_tree` unregisters, the
//! RPC/method-track handlers push events. No per-tick logic: the tick is `bullet/system.rs`
//! (pure) + `bullet/sync.rs` (engine).

use crate::ecs::event::{BulletFx, InboundEvent, Initial};
use crate::ecs::{BulletHandles, Handles, queue};
use crate::settings::Settings;
use godot::classes::{AnimationPlayer, CharacterBody3D, CollisionShape3D, ICharacterBody3D, OmniLight3D};
use godot::prelude::*;

pub(crate) mod sync;
pub(crate) mod system;

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
    /// v2's one-shot setup (`bullet.rs:88-93`: the collision shape off on a non-server peer; the
    /// `set_physics_process(false)` is moot — there is no callback) and the registration. The
    /// `Settings` read of v2's `explode` (`:143`) happens ONCE here and travels with the handles.
    fn ready(&mut self) {
        let simulates = self.base().get_multiplayer().unwrap().is_server();
        if !simulates {
            self.collision_shape.set_disabled(true);
        }
        let shadow_mapping = self.settings.bind().graphics().shadow_mapping;

        queue::push(InboundEvent::Register {
            id: self.base().instance_id(),
            handles: Handles::Bullet(Box::new(BulletHandles {
                root: self.to_gd(),
                anim: self.animation_player.clone(),
                collision: self.collision_shape.clone(),
                light: self.omni_light.clone(),
                shadow_mapping,
            })),
            initial: Initial::Bullet { shadow_mapping, simulates },
        });
    }

    fn exit_tree(&mut self) {
        queue::push(InboundEvent::Unregister { id: self.base().instance_id() });
    }
}

#[godot_api]
impl Bullet {
    /// Remote peers only (spec FR-004, option (b)): the simulating peer applies the explosion's
    /// local effects inline in its fixed `SyncOut`; here the effect is queued for this peer's
    /// frame run (`sync_out_bullet_frame`).
    #[rpc(authority, call_remote, unreliable)]
    fn explode(&mut self) {
        queue::push(InboundEvent::BulletFx { root_id: self.base().instance_id(), fx: BulletFx::Explode });
    }

    /// The `explode` animation's method track (`bullet.tscn:92-104`), on every peer; the drain
    /// frees only a `Simulates` entity (v2 `bullet.rs:150-153`).
    #[func]
    fn destroy(&mut self) {
        queue::push(InboundEvent::BulletDestroy { root_id: self.base().instance_id() });
    }
}
