# Implementation Plan: Milestone V3-C — the enemy: `bullet`, `part`, `red_robot` over the ECS core

**Branch**: `v3` | **Date**: 2026-09-19 | **Spec**: [spec.md](./spec.md) (committed at `e5ca11a`)

**Input**: Feature specification from `/specs/013-v3-bullet-part-robot/spec.md`

**Phase**: v3 (ECS layer over the nodes), constitution 1.5.2 (`cfd20be`), Principles I, II, III
including the "ECS shape (v3)" subsection. Code baseline `30d1a4d` (V3-B complete, 179 tests).

## Summary

The last three gameplay modules become entities over four bridges: `Bullet` (one entity), `Part`
(one entity per part node — each registers itself, they are not sub-bridges of the robot) and
`EnemyRobot` (one entity with the three `Gd<Part>` among its handles). The bullet runs the fixed
tick with `move_and_collide` in `EngineQueryMove` and its collider resolved right there into a
`Send` `HitKind` (an additive `hittable.rs` change); its `GameplaySettle` writes
`Messages<RobotHitLocal>` for a robot target, which the robot's `robot_hit_apply` — same set,
ordered after — consumes in the SAME fixed run (option (B), proven in a probe crate, R4), so the
robot's health, reaction, death and the parts' first moving step land on the step v2 put them.
The robot's death branch in `SyncOut` draws v2's thirteen RNG values in v2's order and explodes
the three parts directly through its handles and the parts' entities (R5); a part then lives on
the frame schedule as a phase machine (`Waiting` timer → `Fading` → `Destroyed` timer → `Remove`)
whose fade reaches the shader and the wire through the node's own setter. The robot's
`AnimationTree` goes MANUAL after the R1 twin experiment reproduced V3-B's verdict on this tree
(v2 = v3 PHYSICS = v3 MANUAL+advance, 601 lines; `M(n).rm == S(n+1).rm`); the laser `RayCast3D`
turned out to be disabled in the scene (a constant read), and a generic experiment pinned the
rule for live engine-updated children. Four RPC attributes change to `call_remote` (`explode`,
`destroy`, `play_shoot`, the robot's `hit`); the player's `hit` keeps `call_local`. Parity: five
harness cases on both trees (part positions comparable, R2) plus two visual checkpoints.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98), edition 2024.

**Primary Dependencies**: `godot = "0.5.5"` (gdext, `experimental-threads` on) and
`bevy_ecs = "0.19"` (`default-features = false, features = ["std"]`). No new crate. Bindings
used for the first time from a system: `PhysicsBody3D::move_and_collide` (`physics_body_3d.rs:37`),
`KinematicCollision3D::get_collider` (`kinematic_collision_3d.rs:259`), `RigidBody3D::
set_freeze_enabled/set_linear_velocity/set_angular_velocity` (`rigid_body_3d.rs:717/:276/:294`),
`MultiplayerSynchronizer::set_visibility_public` (`multiplayer_synchronizer.rs:281`),
`RayCast3D::is_colliding/get_collision_point` (`ray_cast_3d.rs:207/:257`), `ShaderMaterial::
set_shader_parameter` (`shader_material.rs:169`), `Gd::try_from_instance_id` (godot-core
`src/obj/gd.rs:255`), `AnimationMixer::advance` (V3-B).

**Storage**: N/A.

**Testing**: `cargo test` (the 33 pure tests preserved: bullet 3, part 5, robot model 25; new
`run_system_once` tests per system, data-model.md "Tests by name") + headless Godot (import,
`main.tscn`, `level.tscn`) + the five-case harness on the `v2` worktree and `v3` (R10) + two
visual checkpoints (the second multiplayer, two instances).

**Target Platform**: Linux (headless validation; the multiplayer checkpoint runs two local
instances).

**Project Type**: Godot 4.7 project + Rust gdext cdylib; one `.tscn` property edit
(`red_robot.tscn:10782`, R1).

**Performance Goals**: none numeric (constitution: cache locality is not a goal). R9 records the
engine-call budget: bullet 3 per step flying (v2 2), part 2 per fading frame (v2 1), robot 18 per
walking step (v2 16).

**Constraints**: behavioral parity with `v2` at `e2932b4` (harness + checkpoints); scope lists of
Principle I v3 under 1.5.2 (`flying_forklift` excluded) — touched: `bullet.rs`, `part.rs`,
`red_robot.rs` (bridges), new `bullet/{system,sync}.rs`, `part/{system,sync}.rs`,
`red_robot/{system,sync}.rs`, `hittable.rs` (additive only), `ecs.rs`, `ecs/*`,
`red_robot.tscn:10782`, docs; untouched: the three pure cores (`bullet.rs`'s `mod pure` gains
only `pub(crate)`), `player*`, `camera_noise_shake*`, `door*`, `part_disappear*`, `blast.rs`,
`level.rs`, `flying_forklift.rs`, `settings*`, `menu*`, `main_scene.rs`, `debug_label.rs`;
preserved surfaces of FR-024; backlog #31 default deferred (user decision at this review).

**Scale/Scope**: 3 bridges (155 + 233 + 511 lines today), 6 new files, `Handles`/`Initial` +3
variants each, 8 drain arms, 1 message type, ~16 systems, 33 new tests, 9 tradeoffs rows.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 — see "Post-Design Re-check".*

**Principle I (Three-Phase Port)** — v3 block (1.5.2):
- Two objectives per story: pillars kept (the three pure cores untouched and reused; typed
  components `BulletIntents`/`PartPhase`/`RobotIntents`/`HitKind`; glue thin) AND the ECS over
  the nodes (no bridge runs per-frame logic; three entity kinds in one World). PASS.
- Non-objectives: no data-layout argument; physics (`RigidBody3D`, `move_and_collide`,
  raycasts), animation playback, replication, instancing stay Godot's (the tree is ADVANCED,
  not replaced — R1). PASS.
- Dependency policy: no new crate. PASS.
- Scope: `bullet`, `part`, `red_robot` are on the 1.5.2 ECS list; `flying_forklift` is now on
  the stay-as-v2 list and is untouched; the excluded modules untouched (SC-006). `hittable.rs` is
  not on either list (a shared helper): it changes ADDITIVELY under FR-025 — Complexity
  Tracking. PASS.
- Parity baseline `v2`; behavior changes: none sanctioned by default (#31 deferred); the four
  `call_remote` attributes and the `.tscn` line are spec-sanctioned outcomes (Complexity
  Tracking); timing shifts only via the table. PASS.
- Preserved surfaces (Principle II): FR-024's list — replicated names, the `#[var]`/`#[export]`s,
  `exploded`, the `#[func]`/`#[rpc]` names, `Bullet::VELOCITY`, `State`, class names; ONE removal
  of a `pub(crate) #[func]` with no caller (`Part::explode`, R5) — Complexity Tracking. PASS.
- Engine touch points → `docs/v3-tradeoffs.md` rows in the introducing commit (R11). The api-gap
  rule: no new gap. PASS.

**Principle II (Verifiable Port Cycle)**: binding by type unchanged; infrastructure first
(commit 1 before 2–3); property names preserved; dynamic access limited to the residual list of
the spec's top block (`.rpc` sites, the `AnimationTree` parameter paths, the two shader parameter
names, the `Model` node path); build + headless per commit. PASS.

**Principle III + "ECS shape (v3)"**:
- One World / one driver: unchanged. PASS.
- Bridges: `Bullet`, `Part`, `EnemyRobot` have no `process`/`physics_process`; handlers push (the
  robot's `hit` handler exists for REMOTE peers only under `call_remote`; the player's
  `call_local` `hit` invoked from the bullet's `SyncOut` only pushes `AddTrauma`); `set_fade_value`
  is a projection-inbound setter that touches no component. Push-never-borrows: V3-A's queue.
  PASS.
- Tick phases: the bullet uses `Gameplay`, `EngineQueryMove`, `GameplaySettle`, `SyncOut`; the
  robot all seven; the part `Gameplay` + `SyncOut` of the frame chain. Each `EngineQuery*` member
  answers a later `Gameplay*`: the bullet's collision → `bullet_settle`; the robot's raycasts and
  root motion → `robot_step_and_animate`; `move_and_slide` → nothing needs its result for the
  robot (v2 reads no post-move value, `:255-258`), so `move_robot` is the last engine write
  before `SyncOut` and `GameplaySettle` holds only `robot_hit_apply`. No engine write inside a
  query set except `move_and_collide`/`move_and_slide` themselves and `set_velocity`/
  `set_up_direction` (their inputs). PASS.
- Node handles: three boxed `Handles` variants in the `NonSend` map; components hold no `Gd`
  (`HitKind`, `TrackedPlayer` are ids); dead roots despawn in `SyncIn`; `queue_free` only through
  `Remove` + `sync_out_remove` (bullet on the server, parts on every peer, robot on the server).
  PASS.
- Multiplayer: `Simulates` at registration on all three; the robot's `SyncOut` writes the
  replicated `state`/`target_position`/`health`/`dead`, the part's writes `fade_value` through the
  setter; `SyncIn` reads them on other peers; `.rpc("name")` residual. PASS.
- Timers: the part's `Waiting`/`Destroyed`, the robot's `PendingTrauma`/`RemovalTimer` are `Timer`
  components on the frame schedule (v2's `SceneTreeTimer`s); no async task in the three modules.
  PASS.
- Tests without Godot: every pure system with `run_system_once`; the same-run hit path with a
  schedule test (R4). PASS.
- Tuning: `Tuning(RobotTuning::default())` inserted by `build_world`; the bullet's `VELOCITY`
  and the part's exported lifetimes stay where they are (a const and per-instance exports, not a
  tuning struct). PASS.
- Layout: `x.rs` (bridge) + `x/system.rs` (pure) + `x/sync.rs` (glue) as V3-B; `bullet.rs`'s
  inline `mod pure` stays (made `pub(crate)`); `part.rs`'s inline `mod pure` likewise;
  `red_robot/model.rs` untouched. PASS.

**Governance**: SC-006's checklist (quickstart §5); parity evidence (five diffs + two
checkpoints) in the `/speckit-implement` summaries; tradeoffs rows per commit.

**Result**: PASS. Four spec-sanctioned outcomes recorded in Complexity Tracking. One decision
for the user at this review: backlog #31 (FR-023), planned as DEFERRED — say so to close it.

## Project Structure

### Documentation (this feature)

```text
specs/013-v3-bullet-part-robot/
├── plan.md              # This file
├── research.md          # R1–R4, R8 experiments; R5–R7, R9–R11 decisions
├── data-model.md        # components, events, message, handles, initials, systems, tests by name
├── quickstart.md        # gates, headless, harness, checkpoints, review greps
├── contracts/
│   ├── enemy-entities.md    # the three bridges, the same-run hit path, the cross-entity explosion, projections
│   ├── zz_ecs_parity.gd     # five-case harness (scratch; never in oxide-godot/)
│   ├── zz_r1_robot.gd       # the robot AnimationTree experiment + the R8 laser probes (scratch)
│   ├── zz_r2_parts.gd       # the parts determinism / RNG-order experiment (scratch)
│   └── zz_r3_order.gd       # the engine-updated-child ordering experiment (scratch)
└── tasks.md             # /speckit-tasks — not this command
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── hittable.rs                   # + HitKind, kind_of, HitKind::rpc_hit/robot_id (additive)
├── ecs.rs                        # + BulletHandles/PartHandles/RobotHandles, apply_register arms, sync_out_remove arms, Messages<RobotHitLocal>::update, registration of the engine systems
├── ecs/
│   ├── setup.rs                  # + Messages<RobotHitLocal>, Tuning(RobotTuning); pure systems registered; .after(bullet_settle)
│   ├── markers.rs                # + the enemy components (data-model.md)
│   ├── event.rs                  # + BulletFx, BulletDestroy, PartFx, RobotHit, RobotFx, ShootRequested, ResumeApproachRequested, RobotPlayerSeen; Initial::{Bullet, Part, Robot}; RobotHitLocal (Message)
│   └── apply.rs                  # + the eight drain arms (+ tests)
├── bullet.rs                     # bridge (ready registers; explode → BulletFx; destroy → BulletDestroy); `pub(crate) mod pure` UNCHANGED inside
├── bullet/system.rs              # pure: bullet_step, bullet_settle (+ tests)
├── bullet/sync.rs                # glue: sync_in_bullet, move_bullet, sync_out_bullet, sync_out_bullet_frame
├── part.rs                       # bridge (ready registers; set_fade_value pub(crate); destroy → PartFx); `mod pure` UNCHANGED; explode REMOVED
├── part/system.rs                # pure: part_phase_tick (+ tests)
├── part/sync.rs                  # glue: sync_out_part (+ puff_parent)
├── red_robot.rs                  # bridge (ready registers incl. AimBlend from the tree; handlers push; area signals push)
├── red_robot/model.rs            # UNTOUCHED
├── red_robot/system.rs           # pure: robot_decide, robot_step_and_animate, robot_hit_apply, robot_timers (+ tests)
└── red_robot/sync.rs             # glue: sync_in_robot, robot_query, move_robot, sync_out_robot, sync_out_robot_frame
oxide-godot/enemies/red_robot/red_robot.tscn   # :10782 callback_mode_process = 2 (R1, option B)
docs/v3-tradeoffs.md              # + 9 rows
CLAUDE.md                         # v3 section: messages vs queue, engine-updated children rule, RNG in glue, cross-entity access
```

**Structure Decision**: V3-B's layout per module. `ecs.rs` registers the engine systems and
`setup.rs` the pure ones (with the one cross-module ordering `robot_hit_apply.after(bullet_settle)`
inside `build_fixed`, both pure).

## Commit Plan

Four local commits on `v3` (research R11); the harness runs after commits 2 and 3; two STOPs
after commit 3 (single player, then two-instance multiplayer); docs last.

| # | Commit | Gate + validation |
|---|---|---|
| 1 | `ecs + hittable: HitKind, enemy events/drain arms, RobotHitLocal message, components, Handles/Initial variants (tests)` | gates, 179 + 8 = 187 |
| 2 | `bullet: bridge over the ECS core — fixed tick with move_and_collide in EngineQueryMove, HitKind, explode call_remote` | gates 192; headless; harness (c) both trees; bullets still kill v2 robots (the robot's `hit` is v2's `call_local` handler until commit 3) |
| 3 | `part + red_robot: bridges, part phase machine on the frame schedule, robot seven-set tick with same-run hit (RobotHitLocal), robot AnimationTree MANUAL + advance (R1 option B)` | gates 212; headless; harness (a), (b), (d), (e) both trees; **STOP 1** single player; **STOP 2** multiplayer |
| 4 | `CLAUDE.md: entity-to-entity messages vs queue, engine-updated children rule, RNG in glue; spec: timing table; tradeoffs complete` | docs-only |

## Complexity Tracking

*No constitution violation. Spec-sanctioned outcomes recorded so they are reviewed, not
discovered:*

| Item | Why it exists | Resolution |
|---|---|---|
| `red_robot.tscn:10782` `callback_mode_process` 0 → 2 (MANUAL) | R1: the driver at `i32::MAX` reads the robot's root motion one step early in PHYSICS mode (`M(n).rm == S(n+1).rm`); MANUAL + `advance` reproduces v2 line by line (601 lines, both trees) | Spec FR-022 option (B), the milestone's one `.tscn` edit — an engine property, not an exported script property or a replicated name (Principle II); `docs/v3-tradeoffs.md` row |
| Four RPCs `call_local` → `call_remote`: `Bullet::explode`, `Part::destroy`, `EnemyRobot::play_shoot`, `EnemyRobot::hit` (FR-004, option (b)/(B), user-approved at spec review 2026-09-19) | Their local effects must land in the same physics step as their cause; a `call_local` handler could only push for the next run (V3-B's argument). The robot's `hit` additionally needs the bullet's collision and the parts' velocities in the SAME run (option (B)): the local path is `Messages<RobotHitLocal>` across two ordered systems, not the queue | The simulating peer's `SyncOut` applies the local effects; remote handlers push `BulletFx`/`PartFx`/`RobotFx`/`RobotHit` for their frame run. Names/signatures unchanged; `docs/v3-tradeoffs.md` row |
| `hittable.rs` additive change (`HitKind`, `kind_of`, `HitKind::rpc_hit`/`robot_id`) | A component cannot carry `HitTarget` (`Gd`); the collider must be resolved where `move_and_collide` runs and consumed two sets later | FR-025 permits additive changes; `resolve`/`rpc_hit`/`HitTarget` keep their signatures; the by-name RPC string stays spelled once |
| `Part::explode` (`#[func] pub(crate)`, `part.rs:162-192`) REMOVED | Its only caller was the robot's typed `bind_mut().explode()` (`red_robot.rs:302-304`), now the robot's `SyncOut` death branch (R5); a `#[func]` that cannot reach the part entity's phase would be a trap. No scene or script references it (grep: only the three Rust call sites) | FR-024 lists the name among preserved `#[func]`s while FR-004 lets the plan remove it; the plan removes it and records the deviation here and in `docs/v3-tradeoffs.md` |

## Post-Design Re-check

Re-evaluated with data-model.md and the contracts written: components hold no `Gd` (`HitKind`
and `TrackedPlayer` are `InstanceId`s; the three handle structs live in `NodeHandles`); the three
pure system files import nothing from `godot::classes`; every engine call sits in `*/sync.rs` or
a bridge; the seven-set chain keeps V3-A/V3-B's systems in place; `advance` is the last engine
write of `sync_out_robot`; the same-run hit path is a proven `Messages` pattern (R4); the two
STOPs precede the docs commit. Nothing moves to Complexity Tracking beyond the four rows above.
