//! The robot entity's engine systems (constitution 1.5.2 "ECS shape (v3)"; specs/013 research
//! R5/R6): `SyncIn`, the two `EngineQuery*` members and `SyncOut` of the FIXED schedule, and the
//! frame `SyncOut` for the timers' effects and the remote peers' RPC handlers. Every engine call
//! of v2's `physics_process`/`shoot`/`hit`/`play_shoot`/`animate`/`_clip_ray` lives here, in v2's
//! order; the decisions are in `red_robot/system.rs`.

use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use godot::classes::{AnimationTree, Node3D, PhysicsRayQueryParameters3D, ShaderMaterial};
use godot::global::{randf, randi};
use godot::obj::InstanceId;
use godot::prelude::*;

use super::model::{self, RobotTuning};
use super::system::{DEFAULT_MAX_DIST, remote_hits, should_advance};
use crate::ecs::event::{InboundEvent, RobotFx};
use crate::ecs::index::EntityIndex;
use crate::ecs::markers::{
    AnimDecision, Dead, FixedDelta, Health, LaserBuffer, Orientation, PartLifetimes, PartPhase, PartTag,
    PendingRobotFx, PendingRobotHits, PendingTrauma, RaycastAnswers, RemovalTimer, ReplayRobot,
    RobotCountersC, RobotFrame, RobotIntents, RobotState, RobotTag, RootMotion, ShotResult,
    Simulates, TargetPosition, TrackedPlayer, TraumaDue, Tuning, Velocity,
};
use crate::ecs::queue;
use crate::ecs::timer::Timer;
use crate::ecs::{Handles, NodeHandles, RobotHandles};
use crate::part::Part;
use crate::part::pure::{random_angular_velocity, wait_time};

// The `AnimationTree` parameter paths (v2 `red_robot.rs:469-488`): the one string-keyed engine
// surface of the robot, plus the three hit one-shots (`:285`).
const TRANSITION_REQUEST: &str = "parameters/state/transition_request";
const AIMING_BLEND_AMOUNT: &str = "parameters/aiming/blend_amount";
const AIM_BLEND_POSITION: &str = "parameters/aim/blend_position";

/// `SyncIn`, FIXED schedule, every live robot: one read per value v2's tick took from the nodes
/// (`red_robot.rs:153`, `:164`, `:180`, `:199-202`, `:249-250`, `:478`, `:493`). The tracked
/// player's origin is re-fetched by id (research R6). The laser `RayCast3D` is read every run
/// but the snapshot carries the PREVIOUS run's read (`LaserBuffer`, research R8's one-step
/// buffer): the ray updates between the robot's priority-0 slot and the driver, so v2's tick saw
/// the previous step's collision while `SyncIn` sees this step's — the buffer reproduces v2's
/// `max_dist` (the ray is live during the shoot animation, harness (d)). Non-`Simulates` robots
/// also get the replay input (`:134`).
type SyncInQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static TrackedPlayer, Has<Simulates>, Option<&'static LaserBuffer>),
    (With<RobotTag>, Without<Dead>),
>;

pub fn sync_in_robot(handles: NonSend<NodeHandles>, robots: SyncInQuery, mut commands: Commands) {
    for (entity, tracked, simulates, laser_prev) in &robots {
        let Some(Handles::Robot(p)) = handles.by_entity.get(&entity) else {
            continue;
        };
        let player_origin = tracked
            .0
            .and_then(|id| Gd::<Node3D>::try_from_instance_id(id).ok())
            .map(|player| player.get_global_transform().origin);
        let laser_now = LaserBuffer {
            colliding: p.laser_raycast.is_colliding(),
            point: p.laser_raycast.get_collision_point(),
        };
        let laser_prev = laser_prev.copied().unwrap_or(LaserBuffer { colliding: false, point: Vector3::ZERO });
        let frame = RobotFrame {
            global_transform: p.root.get_global_transform(),
            gravity: p.root.get_gravity(),
            velocity: p.root.get_velocity(),
            player_origin,
            ray_from: p.ray_from.get_global_transform(),
            ray_mesh: p.ray_mesh.get_global_transform(),
            ray_mesh_z: p.ray_mesh.get_position().z,
            laser_colliding: laser_prev.colliding,
            laser_point: laser_prev.point,
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((frame, laser_now));
        if !simulates {
            let (state, target_position, aim_preparing) = {
                let root = p.root.bind();
                (root.state, root.target_position, root.aim_preparing)
            };
            entity_commands.insert(ReplayRobot { state, target_position, aim_preparing });
        }
    }
}

/// The ONE raycast helper's typed result (v2 `red_robot.rs:14-19`, `:363-382`), with the
/// collider as an id so `RaycastAnswers` may carry it.
#[derive(Clone, Copy)]
struct RayHit {
    position: Vector3,
    collider: Option<InstanceId>,
}

/// v2 `raycast_to` (`red_robot.rs:363-382`) on the handles: the robot's own body excluded by RID.
fn raycast_to(p: &RobotHandles, from: Vector3, to: Vector3) -> Option<RayHit> {
    let params = PhysicsRayQueryParameters3D::create_ex(from, to)
        .collision_mask(0xFFFFFFFF)
        .exclude(&array![p.rid])
        .done()
        .unwrap();
    let col: VarDictionary = p
        .root
        .get_world_3d()
        .unwrap()
        .get_direct_space_state()
        .unwrap()
        .intersect_ray(&params);
    if col.is_empty() {
        return None;
    }
    let position = col.get("position").unwrap().to::<Vector3>();
    let collider = col
        .get("collider")
        .and_then(|v| v.try_to::<Gd<Object>>().ok())
        .map(|c| c.instance_id());
    Some(RayHit { position, collider })
}

/// `EngineQueryOrient`, FIXED schedule, `Simulates` live robots: the pre-check/aim raycast
/// (`:177-183`, `:216-222`, `hits_player` `:506-511`), the shoot raycast along the bone's Y
/// (`:399-409`), and the root motion the previous run's `advance` produced (`:241-244`; not on
/// the idle branch, which v2 never read).
type RobotQueryQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static RobotIntents, &'static TrackedPlayer, &'static mut RaycastAnswers, &'static mut RootMotion),
    (With<Simulates>, Without<Dead>),
>;

pub fn robot_query(mut handles: NonSendMut<NodeHandles>, mut robots: RobotQueryQuery) {
    for (entity, intents, tracked, mut answers, mut root_motion) in &mut robots {
        let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        *answers = RaycastAnswers::default();
        if let Some((from, to)) = intents.raycast {
            let hit = raycast_to(p, from, to);
            answers.sees_player = Some(hit.and_then(|h| h.collider).is_some_and(|c| Some(c) == tracked.0));
        }
        if intents.shoot {
            let gt: Transform3D = p.ray_from.get_global_transform();
            let ray_origin: Vector3 = gt.origin;
            // The RayCast3D is rotated 90 degrees inside the BoneAttachment3D.
            let ray_dir: Vector3 = gt.basis.col_b();
            let hit = raycast_to(p, ray_origin, ray_origin + ray_dir * DEFAULT_MAX_DIST);
            let max_dist = hit.map(|h| ray_origin.distance_to(h.position)).unwrap_or(DEFAULT_MAX_DIST);
            answers.shot = Some(ShotResult { max_dist, hit: hit.map(|h| (h.position, h.collider)) });
        }
        if !intents.idle_branch {
            root_motion.0 = Transform3D::new(
                Basis::from_quaternion(p.anim_tree.get_root_motion_rotation()),
                p.anim_tree.get_root_motion_position(),
            );
        }
    }
}

/// `EngineQueryMove`, FIXED schedule, `Simulates` live robots, ordered before the bullet's
/// `move_bullet` (v2's tree order): v2 `:147-149` / `:253-255`.
type MoveQuery<'w, 's> = Query<'w, 's, (Entity, &'static Velocity), (With<Simulates>, Without<Dead>)>;

pub fn move_robot(mut handles: NonSendMut<NodeHandles>, robots: MoveQuery) {
    for (entity, velocity) in &robots {
        let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        p.root.set_velocity(velocity.0);
        p.root.set_up_direction(Vector3::UP);
        p.root.move_and_slide();
    }
}

/// v2 `_clip_ray` (`red_robot.rs:492-501`): skipped on a dedicated server.
fn clip_ray(p: &mut RobotHandles, length: f32) {
    let mesh_offset: f32 = p.ray_mesh.get_position().z;
    if !p.is_dedicated_server {
        p.ray_mesh
            .get_surface_override_material(0)
            .unwrap()
            .cast::<ShaderMaterial>()
            .set_shader_parameter("clip", &(length + mesh_offset).to_variant());
    }
}

/// v2 `animate`'s tree writes (`red_robot.rs:469-488`), in v2's order, from the decision.
fn apply_anim(anim_tree: &mut Gd<AnimationTree>, anim: AnimDecision) {
    anim_tree.set(TRANSITION_REQUEST, &anim.request.to_variant());
    if let Some((blend_amount, blend_position)) = anim.aim {
        anim_tree.set(AIMING_BLEND_AMOUNT, &blend_amount.to_variant());
        anim_tree.set(AIM_BLEND_POSITION, &blend_position.to_variant());
    }
}

/// v2 `hit`'s every-peer death visuals (`red_robot.rs:293-300`): the `dead` projection (the
/// `bind_mut()` guard dropped in its own block), the tree inactive, the model hidden, `Death`
/// visible, the collision off, the two sparks.
fn death_visuals(p: &mut RobotHandles) {
    {
        p.root.bind_mut().dead = true;
    }
    p.anim_tree.set_active(false);
    p.model.set_visible(false);
    p.death.set_visible(true);
    p.collision_shape.set_disabled(true);
    for spark in &mut p.sparks {
        spark.set_emitting(true);
    }
}

/// `sync_out_robot`'s part query (research R5): disjoint from the robot query by the filters.
type PartQuery<'w, 's> = Query<'w, 's, (&'static mut PartPhase, &'static PartLifetimes), (With<PartTag>, Without<RobotTag>)>;

/// The cross-entity part explosion (research R5; v2 `part.rs:164-175` per part, in scene order
/// `PartShield1`, `PartShield2`, `PartHead`, `red_robot.rs:302-304`): the three `Gd<Part>` are
/// cloned out of the robot's entry BEFORE the loop so the map is free for the parts' own entries
/// (`synchronizer`, `col1`, `col2`). On the server the twelve random draws (three per angular
/// velocity, one per wait) happen here, in v2's order, and the part ENTITY's phase becomes
/// `Waiting(Timer(wait))`; on the other peers only the every-peer half (visibility public,
/// unfreeze) runs, as v2's `explode` did (`:167-169`).
fn explode_parts(
    handles: &mut NodeHandles,
    index: &EntityIndex,
    mut parts: Option<&mut PartQuery>,
    robot: Entity,
    server: bool,
) {
    let part_nodes: [Gd<Part>; 3] = {
        let Some(Handles::Robot(p)) = handles.by_entity.get(&robot) else {
            return;
        };
        p.parts.clone()
    };
    for mut part in part_nodes {
        let part_entity = index.entity(part.instance_id());
        let mut own = part_entity.and_then(|e| match handles.by_entity.get_mut(&e) {
            Some(Handles::Part(q)) => Some(q),
            _ => None,
        });
        // Start synching.
        if let Some(q) = own.as_mut() {
            q.synchronizer.set_visibility_public(true);
        }
        part.set_freeze_enabled(false);
        if !server {
            continue;
        }
        if let Some(q) = own.as_mut() {
            q.col1.set_disabled(false);
            q.col2.set_disabled(false);
        }
        part.set_linear_velocity(3.0 * Vector3::UP);
        let r1 = randf() as f32;
        let r2 = randf() as f32;
        let r3 = randf() as f32;
        part.set_angular_velocity(random_angular_velocity(r1, r2, r3));
        let r = randf() as f32;
        if let Some(entity) = part_entity
            && let Some(parts) = parts.as_deref_mut()
            && let Ok((mut phase, lifetimes)) = parts.get_mut(entity)
        {
            let wait = wait_time(lifetimes.lifetime, lifetimes.lifetime_random, r);
            *phase = PartPhase::Waiting(Timer::new(wait as f64));
        }
    }
}

/// `sync_out_robot`'s robot query (a `type` so clippy's `type_complexity` stays quiet).
type RobotSyncOutQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Orientation,
        &'static RobotState,
        &'static TargetPosition,
        &'static Health,
        &'static RobotCountersC,
        &'static RobotIntents,
        &'static RaycastAnswers,
        &'static TrackedPlayer,
        &'static mut PendingTrauma,
        &'static mut RemovalTimer,
        Has<Simulates>,
        Has<Dead>,
    ),
    With<RobotTag>,
>;

/// `SyncOut`, FIXED schedule, every robot (spec scenario 5, research R5/R6), in THIS order per
/// entity. On `Simulates` && !`Dead`: the model basis unless on the idle branch (`:257-258`); the
/// replicated projection through `root.bind_mut()` (guard dropped); `play_shoot` inline + the RPC
/// to the remote peers (`:331-332`); the shoot effects (`:411-425`, `:437-453`: clip, ember, the
/// blast under the tree root, the trauma timer when the collider is the tracked player); the
/// Aim/Shooting clip (`:205`); the hit reaction (`:285-287`, the reaction draw in glue BEFORE
/// the parts' draws); on `just_died` the death branch (`:293-326`: visuals, the parts, the explosion
/// sound, `exploded`, the removal timer, `Dead`). For EVERY live robot: the tree parameter
/// writes, then `advance(delta)` LAST — the tree is MANUAL (`red_robot.tscn:10782`, research R1
/// option B) — skipped in the run the robot dies (FR-022 as amended).
pub fn sync_out_robot(
    dt: Res<FixedDelta>,
    tuning: Res<Tuning<RobotTuning>>,
    index: Res<EntityIndex>,
    mut handles: NonSendMut<NodeHandles>,
    mut robots: RobotSyncOutQuery,
    mut parts: PartQuery,
    mut commands: Commands,
) {
    for (entity, orientation, state, target, health, counters, intents, answers, tracked, mut pending_trauma, mut removal, simulates, dead) in
        &mut robots
    {
        if dead {
            continue;
        }
        if simulates {
            {
                let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) else {
                    continue;
                };
                if !intents.idle_branch {
                    p.root.set_global_basis(orientation.0.basis);
                }
                // The replicated projection; the guard is dropped at the end of the block.
                {
                    let mut root = p.root.bind_mut();
                    root.state = state.0;
                    root.target_position = target.0;
                    root.health = health.0;
                    root.aim_preparing = counters.0.aim_preparing;
                }
                if intents.play_shoot {
                    p.shoot_anim.play_ex().name("shoot").done();
                    // Remote peers only (`call_remote`): no handler runs on this peer.
                    p.root.rpc("play_shoot", &[]);
                }
                if let Some(shot) = answers.shot {
                    // Clip ray in shader.
                    clip_ray(p, shot.max_dist);
                    // Position laser ember particles.
                    let mesh_offset: f32 = p.ray_mesh.get_position().z;
                    p.laser_ember.set_position(model::ember_position(shot.max_dist, mesh_offset));
                    let extents = p.laser_ember.get_emission_box_extents();
                    p.laser_ember
                        .set_emission_box_extents(model::ember_extents(extents, shot.max_dist, mesh_offset));
                    if let Some((position, collider)) = shot.hit {
                        let mut blast: Gd<Node3D> = p.impact_effect_scene.instantiate_as::<Node3D>();
                        p.root.get_tree().get_root().unwrap().add_child(&blast);
                        blast.set_global_position(position);
                        if let (Some(collider), Some(player)) = (collider, tracked.0)
                            && collider == player
                        {
                            pending_trauma.0 = Some((Timer::new(tuning.0.trauma_delay as f64), player));
                        }
                    }
                }
                if let Some(max_dist) = intents.clip {
                    clip_ray(p, max_dist);
                }
                if intents.hit {
                    // Hit reaction (RNG anim pick + sound) fires on EVERY live hit, before the
                    // death branch — v2 `:285-287`.
                    let param = format!("parameters/hit{}/request", randi() % 3 + 1);
                    p.anim_tree.set(&param, &1.to_variant());
                    p.hit_sound.play();
                }
                if intents.just_died {
                    death_visuals(p);
                }
            }
            if intents.just_died {
                explode_parts(&mut handles, &index, Some(&mut parts), entity, true);
                if let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) {
                    p.explosion_sound.play();
                    p.root.signals().exploded().emit();
                }
                // backlog #17's timer, now a tick timer stepped by the frame run (`:309-326`).
                removal.0 = Some(Timer::new(tuning.0.removal_delay as f64));
                commands.entity(entity).insert(Dead);
            }
        }
        let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        if let Some(anim) = intents.anim {
            apply_anim(&mut p.anim_tree, anim);
        }
        if should_advance(dead, intents.just_died) {
            p.anim_tree.advance(dt.0);
        }
    }
}

/// `sync_out_robot_frame`'s query.
type RobotFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static TraumaDue>,
        &'static mut PendingRobotFx,
        &'static mut PendingRobotHits,
        &'static mut Health,
        Has<Simulates>,
        Has<Dead>,
    ),
    With<RobotTag>,
>;

/// `SyncOut`, FRAME schedule, every robot: the expired trauma timer pushes `AddTrauma` for the
/// player entity (v2 `:452`; the drain applies it at the next run); on non-`Simulates` robots
/// the remote `hit`s — v2's per-peer `call_local` handler logic on the client's OWN `Health`
/// (`health`/`dead` are spawn-only replicated, `red_robot.tscn:31-33`/`:40-42`): one reaction
/// per live hit, and on the death the visuals, the parts' every-peer half, the explosion sound
/// and `exploded` (`:278-307`) — and the remote `play_shoot` (`:330-333`).
pub fn sync_out_robot_frame(
    tuning: Res<Tuning<RobotTuning>>,
    index: Res<EntityIndex>,
    mut handles: NonSendMut<NodeHandles>,
    mut robots: RobotFrameQuery,
    mut commands: Commands,
) {
    for (entity, trauma_due, mut pending_fx, mut pending_hits, mut health, simulates, dead) in &mut robots {
        if let Some(TraumaDue(player)) = trauma_due {
            queue::push(InboundEvent::AddTrauma { root_id: *player, amount: tuning.0.trauma_amount });
            commands.entity(entity).remove::<TraumaDue>();
        }
        if simulates {
            continue;
        }
        let hits = std::mem::take(&mut pending_hits.0);
        if hits > 0 {
            let (new_health, reactions, just_died) = remote_hits(hits, health.0, dead);
            health.0 = new_health;
            if let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) {
                for _ in 0..reactions {
                    let param = format!("parameters/hit{}/request", randi() % 3 + 1);
                    p.anim_tree.set(&param, &1.to_variant());
                    p.hit_sound.play();
                }
                if just_died {
                    death_visuals(p);
                }
            }
            if just_died {
                explode_parts(&mut handles, &index, None, entity, false);
                if let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) {
                    p.explosion_sound.play();
                    p.root.signals().exploded().emit();
                }
                commands.entity(entity).insert(Dead);
            }
        }
        if pending_fx.0.is_empty() {
            continue;
        }
        let Some(Handles::Robot(p)) = handles.by_entity.get_mut(&entity) else {
            continue;
        };
        for fx in pending_fx.0.drain(..) {
            match fx {
                RobotFx::PlayShoot => {
                    p.shoot_anim.play_ex().name("shoot").done();
                }
            }
        }
    }
}
