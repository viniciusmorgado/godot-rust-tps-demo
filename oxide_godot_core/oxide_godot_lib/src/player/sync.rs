//! The player entity's engine systems (constitution 1.5.1 "ECS shape (v3)"; specs/012 research
//! R7): `SyncIn`, the two `EngineQuery*` members and `SyncOut` of the FIXED schedule. Every engine
//! call of v2's `apply_input` lives here, in v2's order; the decisions are in `player/system.rs`.

use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use godot::classes::{AnimationTree, CharacterBody3D};
use godot::prelude::*;

use super::Animations;
use super::model::{AnimPlan, InputFrame};
use super::system::replay_plan;
use crate::camera_noise_shake::model::{CameraShakeTuning, add_trauma};
use crate::ecs::event::PlayerFx;
use crate::ecs::markers::{
    BodyState, CurrentAnimation, FixedDelta, InitialPosition, InputFrameC, JumpQueued, Motion,
    Orientation, OrientTarget, PendingFx, PlayerTag, ReplayState, RootMotion, Simulates,
    TickIntents, Trauma, Tuning, Velocity,
};
use crate::ecs::{Handles, NodeHandles};
use crate::player::model::PlayerTuning;
use crate::player_input::model::{PlayerInputTuning, aim_rotation};

// The `AnimationTree` parameter paths and transition names (v2 `player.rs:21-29`): the one
// string-keyed engine surface of the player, listed as a residual in the spec's top block.
const TRANSITION_REQUEST: &str = "parameters/state/transition_request";
const AIM_ADD_AMOUNT: &str = "parameters/aim/add_amount";
const STRAFE_BLEND: &str = "parameters/strafe/blend_position";
const WALK_BLEND: &str = "parameters/walk/blend_position";

const TRANSITION_JUMP_UP: &str = "jump_up";
const TRANSITION_JUMP_DOWN: &str = "jump_down";
const TRANSITION_STRAFE: &str = "strafe";
const TRANSITION_WALK: &str = "walk";

/// `SyncIn`, FIXED schedule, every player (FR-006): one read per value — v2 step (1)
/// (`player.rs:207-223`, the input node's fields and the three camera reads of
/// `player_input.rs:171-182`), the body reads of steps (4), (6) and the shoot decision (`:235`,
/// `:241`, `:314`, `:274`), and for non-`Simulates` entities the replay input (`:104-116`).
/// `origin_y` is written ONLY by `move_body`, post-move (v2 read the origin once, at `:329`).
pub fn sync_in_player(
    input_tuning: Res<Tuning<PlayerInputTuning>>,
    handles: NonSend<NodeHandles>,
    mut players: Query<(Entity, &mut JumpQueued, Has<Simulates>), With<PlayerTag>>,
    mut commands: Commands,
) {
    for (entity, mut jump_queued, simulates) in &mut players {
        let Some(Handles::Player(p)) = handles.by_entity.get(&entity) else {
            continue;
        };
        let (aiming, shoot_target, motion, shooting) = {
            let input = p.input.bind();
            (input.aiming, input.shoot_target, input.motion, input.shooting)
        };
        let jumping = jump_queued.0;
        jump_queued.0 = false;
        let camera_rotation_basis = p.camera_rot.get_global_transform().basis;
        let camera_base_quaternion = p.camera_base.get_global_transform().basis.get_quaternion();
        let aim_rotation = aim_rotation(p.camera_rot.get_rotation().x, &input_tuning.0);
        let frame = InputFrame {
            motion,
            aiming,
            shooting,
            jumping,
            shoot_target,
            camera_rotation_basis,
            camera_base_quaternion,
            aim_rotation,
        };
        let body = BodyState {
            on_floor: p.root.is_on_floor(),
            velocity: p.root.get_velocity(),
            gravity: p.root.get_gravity(),
            cooldown_left: p.fire_cooldown.get_time_left(),
            origin_y: 0.0,
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((InputFrameC(frame), body));
        if !simulates {
            let (current_animation, motion) = {
                let root = p.root.bind();
                (root.current_animation, root.motion)
            };
            entity_commands.insert(ReplayState { current_animation, motion, aim_rotation });
        }
    }
}

/// `EngineQueryOrient`, `Simulates` only (FR-008): the engine-backed rotation math of step (5)
/// (`Quaternion::slerp`, `Basis::looking_at`, `player.rs:261-264`/`:296-300`), the root-motion
/// read (`:269-272`/`:306-309`; never while airborne) and the bullet spawn (`:275-291`), which
/// MUST precede `move_and_slide` (the bullet takes `ShootFrom`'s pre-move transform). The
/// `AnimationTree` parameter writes are in `sync_out_player` (analyze finding 3).
pub fn orient_and_anim(
    tuning: Res<Tuning<PlayerTuning>>,
    dt: Res<FixedDelta>,
    mut handles: NonSendMut<NodeHandles>,
    mut players: Query<
        (Entity, &mut Orientation, &mut RootMotion, &TickIntents, &InputFrameC),
        With<Simulates>,
    >,
) {
    let dt = dt.0 as f32;
    for (entity, mut orientation, mut root_motion, intents, frame) in &mut players {
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        if let Some(orient) = intents.orient {
            let q_from: Quaternion = orientation.0.basis.get_quaternion();
            let q_to: Quaternion = match orient {
                OrientTarget::Camera(q_to) => q_to,
                OrientTarget::Walk(target) => Basis::looking_at(target).get_quaternion(),
            };
            orientation.0.basis =
                Basis::from_quaternion(q_from.slerp(q_to, dt * tuning.0.rotation_interpolate_speed));
        }

        if intents.read_root_motion {
            root_motion.0 = Transform3D::new(
                Basis::from_quaternion(p.anim_tree.get_root_motion_rotation()),
                p.anim_tree.get_root_motion_position(),
            );
        }

        if intents.shoot {
            let shoot_origin: Vector3 = p.shoot_from.get_global_transform().origin;
            let shoot_dir: Vector3 = (frame.0.shoot_target - shoot_origin).normalized();

            let mut bullet: Gd<CharacterBody3D> = p.bullet_scene.instantiate_as::<CharacterBody3D>();
            // The player's parent: `SpawnedNodes` in the game (`level.rs:179`), the harness root
            // in the parity scene.
            p.root
                .get_parent()
                .unwrap()
                .add_child_ex(&bullet)
                .force_readable_name(true)
                .done();
            bullet.set_global_position(shoot_origin);
            // If we don't rotate the bullets there is no useful way to control the particles ..
            bullet.look_at(shoot_origin + shoot_dir);
            bullet.add_collision_exception_with(&p.root);
        }
    }
}

/// v2 `apply_anim`'s `AnimationTree` writes (`player.rs:179-200`), in the exact per-variant
/// order `v1`'s `animate()` used. Called by `sync_out_player` before `advance` and, from commit 3,
/// by `apply_player_fx` on remote peers.
pub(crate) fn apply_anim(anim_tree: &mut Gd<AnimationTree>, plan: AnimPlan) {
    match plan {
        AnimPlan::JumpUp => {
            anim_tree.set(TRANSITION_REQUEST, &TRANSITION_JUMP_UP.to_variant());
        }
        AnimPlan::JumpDown => {
            anim_tree.set(TRANSITION_REQUEST, &TRANSITION_JUMP_DOWN.to_variant());
        }
        AnimPlan::Strafe { aim_rotation, blend_position } => {
            anim_tree.set(TRANSITION_REQUEST, &TRANSITION_STRAFE.to_variant());
            // Change aim according to camera rotation.
            anim_tree.set(AIM_ADD_AMOUNT, &aim_rotation.to_variant());
            // The animation's forward/backward axis is reversed.
            anim_tree.set(STRAFE_BLEND, &blend_position.to_variant());
        }
        AnimPlan::Walk { blend_position } => {
            // Aim to zero (no aiming while walking).
            anim_tree.set(AIM_ADD_AMOUNT, &0.to_variant());
            anim_tree.set(TRANSITION_REQUEST, &TRANSITION_WALK.to_variant());
            // Blend position for walk speed based checked motion.
            anim_tree.set(WALK_BLEND, &blend_position.to_variant());
        }
    }
}

/// v2 `apply_anim`'s first half (`player.rs:172-177`): the `Animations` value a plan projects.
fn animation_for(plan: AnimPlan) -> Animations {
    match plan {
        AnimPlan::JumpUp => Animations::JumpUp,
        AnimPlan::JumpDown => Animations::JumpDown,
        AnimPlan::Strafe { .. } => Animations::Strafe,
        AnimPlan::Walk { .. } => Animations::Walk,
    }
}

/// `EngineQueryMove`, `Simulates` only (FR-009): v2 step (7) (`player.rs:320-322`) and the
/// post-move origin read of step (9) (`:329`), which `tick_settle` decides on.
pub fn move_body(
    mut handles: NonSendMut<NodeHandles>,
    mut players: Query<(Entity, &Velocity, &mut BodyState), With<Simulates>>,
) {
    for (entity, velocity, mut body) in &mut players {
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        p.root.set_velocity(velocity.0);
        p.root.set_up_direction(Vector3::UP);
        p.root.move_and_slide();
        body.origin_y = p.root.get_transform().origin.y;
    }
}

/// `sync_out_player`'s query (a `type` so clippy's `type_complexity` stays quiet).
type SyncOutPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Orientation,
        &'static InitialPosition,
        &'static Motion,
        &'static mut CurrentAnimation,
        &'static TickIntents,
        &'static mut Trauma,
        Has<Simulates>,
        Option<&'static ReplayState>,
    ),
    With<PlayerTag>,
>;

/// `SyncOut`, FIXED schedule, every player (FR-010, FR-011, FR-018): in this order per entity —
/// on `Simulates`: the model basis (v2 step (8), `player.rs:326`), the respawn reset (step (9),
/// `:330-333`), the replicated projection (`motion`, `current_animation`) through
/// `root.bind_mut()` with the guard dropped before any engine call that can invoke a callback,
/// the LOCAL effects of `land`/`jump`/`shoot` inline in v2's order (option (b), FR-004:
/// `:134-154`) and the three RPCs, which reach remote peers only (`:246-251`, `:290`); then the
/// `AnimationTree` parameter writes for the `Simulates` plan or the replay plan (`:179-200`,
/// `:118`); LAST, for EVERY player, `advance(delta)` — the tree is MANUAL (`player.tscn:592`,
/// research R1 option (B)), so this is where v2's child processing happened.
pub fn sync_out_player(
    dt: Res<FixedDelta>,
    shake_tuning: Res<Tuning<CameraShakeTuning>>,
    mut handles: NonSendMut<NodeHandles>,
    mut players: SyncOutPlayerQuery,
) {
    for (
        entity,
        orientation,
        initial_position,
        motion,
        mut current_animation,
        intents,
        mut trauma,
        simulates,
        replay,
    ) in &mut players
    {
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        let mut plan: Option<AnimPlan> = None;
        if simulates {
            // (8)
            p.model.set_global_basis(orientation.0.basis);

            // (9) Backlog #11: respawn also zeroes velocity (v1 only reset the transform origin).
            if intents.respawn {
                let mut transform = p.root.get_transform();
                transform.origin = initial_position.0;
                p.root.set_transform(transform);
                p.root.set_velocity(Vector3::ZERO);
            }

            // The replicated projection; the guard is dropped at the end of the block.
            if let Some(next_plan) = intents.plan {
                current_animation.0 = animation_for(next_plan);
            }
            {
                let mut root = p.root.bind_mut();
                root.motion = motion.0;
                root.current_animation = current_animation.0;
            }

            // The local effects of the RPCs, inline in v2's order: `land` then `jump` (step (4),
            // `:246-251`), then `shoot` (`:290` → `:146-154`).
            if intents.land {
                p.snd_land.play();
            }
            if intents.jump {
                p.snd_jump.play();
            }
            if intents.shoot {
                p.shoot_particle.restart();
                p.shoot_particle.set_emitting(true);
                p.muzzle_particle.restart();
                p.muzzle_particle.set_emitting(true);
                p.fire_cooldown.start();
                p.snd_shoot.play();
                // v2 `:153` → `camera_noise_shake.rs:64-67`: the trauma lives on the entity now;
                // this frame's `shake_decide` sees it.
                trauma.0 = add_trauma(trauma.0, 0.35, &shake_tuning.0);
            }

            // Remote peers only (`call_remote`): no handler runs on this peer.
            if intents.land {
                p.root.rpc("land", &[]);
            }
            if intents.jump {
                p.root.rpc("jump", &[]);
            }
            if intents.shoot {
                p.root.rpc("shoot", &[]);
            }
            plan = intents.plan;
        } else if let Some(replay) = replay {
            plan = Some(replay_plan(replay));
        }

        if let Some(plan) = plan {
            apply_anim(&mut p.anim_tree, plan);
        }
        p.anim_tree.advance(dt.0);
    }
}

/// `SyncOut`, FRAME schedule, non-`Simulates` entities only (FR-015; option (b)): the
/// `jump`/`land`/`shoot` RPC handlers of a REMOTE peer, applied in arrival order exactly as v2's
/// handlers did (`player.rs:134-154`): `Jump`/`Land` write the animation (the tree parameters
/// through `apply_anim` and the node's `current_animation` field, as `apply_anim` set
/// `self.current_animation`, `:172-177`) and play the sound; `Shoot` restarts both particles,
/// starts the cooldown and plays the sound — its trauma was applied by the drain
/// (`ecs/apply.rs`), so this frame's `shake_decide` already saw it. The `bind_mut()` guard is
/// dropped before the sound plays.
pub fn apply_player_fx(
    mut handles: NonSendMut<NodeHandles>,
    mut players: Query<(Entity, &mut PendingFx), Without<Simulates>>,
) {
    for (entity, mut pending) in &mut players {
        if pending.0.is_empty() {
            continue;
        }
        let Some(Handles::Player(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        for fx in pending.0.drain(..) {
            match fx {
                PlayerFx::Jump => {
                    apply_anim(&mut p.anim_tree, AnimPlan::JumpUp);
                    {
                        p.root.bind_mut().current_animation = Animations::JumpUp;
                    }
                    p.snd_jump.play();
                }
                PlayerFx::Land => {
                    apply_anim(&mut p.anim_tree, AnimPlan::JumpDown);
                    {
                        p.root.bind_mut().current_animation = Animations::JumpDown;
                    }
                    p.snd_land.play();
                }
                PlayerFx::Shoot => {
                    p.shoot_particle.restart();
                    p.shoot_particle.set_emitting(true);
                    p.muzzle_particle.restart();
                    p.muzzle_particle.set_emitting(true);
                    p.fire_cooldown.start();
                    p.snd_shoot.play();
                }
            }
        }
    }
}
