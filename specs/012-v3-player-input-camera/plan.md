# Implementation Plan: Milestone V3-B — the player as one entity over three nodes

**Branch**: `v3` | **Date**: 2026-09-18 | **Spec**: [spec.md](./spec.md) (committed at `b309e2f`)

**Input**: Feature specification from `/specs/012-v3-player-input-camera/spec.md`

**Phase**: v3 (ECS layer over the nodes), constitution 1.5.1 (`85186f6`), Principles I, II, III
including the "ECS shape (v3)" subsection. Baseline `abfe35a` (V3-A complete).

## Summary

The player becomes ONE entity over three nodes. `Player.ready` registers it with every handle
(its own eleven children, the six camera-side nodes reached through the input node's exported
refs, the camera's three noises) and the initial components; `PlayerInputSynchronizer` and
`CameraNoiseShake` are sub-bridges that keep v2's one-shot `ready` setup and only push events
keyed by the root's `InstanceId` (resolved once through `get_owner()`, R5). The fixed schedule
gains three sets so the tick runs v2's nine steps in order with two engine-query points —
`SyncIn → Gameplay → EngineQueryOrient → GameplayIntegrate → EngineQueryMove → GameplaySettle →
SyncOut` (R4, proven in a probe crate) — with the 16 `player/model.rs` functions reused verbatim
in the pure sets and every engine touch in a query or sync set (R7). The `AnimationTree` goes
MANUAL and is advanced at the end of the fixed `SyncOut` for every player, because a driver at
`i32::MAX` in PHYSICS mode reads the root motion one step early at every step, while
MANUAL+advance reproduces v2 line by line (R1, three runs on two trees). Replication is a
projection: the fixed `SyncOut` writes `motion`/`current_animation` on `Simulates` entities and
the frame `SyncOut` writes the four input fields on `OwnsInput` entities; the engine samples them
in `SceneMultiplayer::poll()` at the top of `SceneTree::process`, after the physics steps and
before the frame run, once per rendered frame (R2) — which is why the `Jump`/`Land` handlers'
transient animation writes are dropped on `Simulates` entities (FR-015's fallback). Input and
camera run on the frame schedule with the camera rotation applied inside the frame
`EngineQuery` before the raycast (spec FR-014); the shake is `Trauma`/`ShakeTime` on the player
entity. Parity: a six-case harness on both trees (R9) plus two visual checkpoints, the second a
two-instance multiplayer session.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98), edition 2024.

**Primary Dependencies**: `godot = "0.5.5"` (gdext, `experimental-threads` on) and
`bevy_ecs = "0.19"` (`default-features = false, features = ["std"]`, pinned in V3-A). No new
crate. Bindings used for the first time: `AnimationMixer::advance(f64)`
(`animation_mixer.rs:347`), `AnimationCallbackModeProcess::MANUAL` (ord 2, `:529-531`),
`CharacterBody3D::move_and_slide`, `PhysicsDirectSpaceState3D::intersect_ray` (already used by
v2 in `process`), `FastNoiseLite::get_noise_1d`, `Node::get_owner`.

**Storage**: N/A.

**Testing**: `cargo test` (the 36 model tests preserved; new `run_system_once` tests per system,
data-model.md) + headless Godot (import, `main.tscn`, `level.tscn`) + the six-case harness on the
`v2` worktree and `v3` (R9) + two visual checkpoints (the second multiplayer, two instances).

**Target Platform**: Linux (headless validation; the multiplayer checkpoint runs two local
instances).

**Project Type**: Godot 4.7 project + Rust gdext cdylib; one `.tscn` property edit
(`player.tscn:592`, R1).

**Performance Goals**: none numeric (constitution: cache locality is not a goal). R8 records the
engine-call budget: 21 per player per physics step walking (v2: 18), 17/24 per frame idle/shooting
(v2: 17/24), 4 per frame while shaking (v2: 4).

**Constraints**: behavioral parity with `v2` at `e2932b4` (harness + checkpoints); scope lists of
Principle I v3 — only `player.rs`, `player_input.rs`, `camera_noise_shake.rs` (their `model.rs`
files untouched), new `player/system.rs`, `player/sync.rs`, `player_input/system.rs`,
`player_input/sync.rs`, `camera_noise_shake/system.rs`, `camera_noise_shake/sync.rs`, `ecs.rs`,
`ecs/*`, `player.tscn:592`, docs; preserved surfaces of FR-019; backlog #6/#29 deferred.

**Scale/Scope**: 3 bridges (336 + 192 + 68 lines today), 6 new files, `Phase` +3 variants,
`Handles`/`Initial` +1 variant each, 4 drain arms, ~17 systems, ≥ 18 new tests, 9 tradeoffs rows.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 — see "Post-Design Re-check".*

**Principle I (Three-Phase Port)** — v3 block:
- Two objectives per story: pillars kept (model files untouched and reused; typed components
  `TickIntents`/`OrientTarget`/`AimStateC`; glue thin) AND the ECS over the nodes (no bridge runs
  per-frame logic; the tick is systems in one World). PASS.
- Non-objectives: no data-layout argument anywhere; physics, animation playback, replication,
  instancing stay Godot's (the tree is ADVANCED by the sync layer, not replaced — R1). PASS.
- Dependency policy: no new crate. PASS.
- Scope: `player`, `player_input`, `camera_noise_shake` are on the constitution's ECS list; the
  excluded modules and `bullet`/`part`/`red_robot`/`flying_forklift`/`hittable`/`level`/`door`
  untouched (SC-006). PASS.
- Parity baseline `v2`; behavior changes: none sanctioned; the dropped transient animation
  writes (R2) reproduce v2's observable state and are recorded in Complexity Tracking as the
  spec's own FR-015 fallback; timing shifts only via the table. PASS.
- Preserved surfaces (Principle II): FR-019's list — replicated names, `set_player_id`, the
  five RPCs, `add_camera_shake_trauma(f64)`, `node_paths`, `Animations`, class names. PASS.
- Engine touch points → `docs/v3-tradeoffs.md` rows in the introducing commit (R10). The api-gap
  rule: no new gap. PASS.

**Principle II (Verifiable Port Cycle)**: binding by type unchanged; infrastructure first (commit
1 before 2–3); property names preserved; dynamic access limited to the five `.rpc("name")` sites
of the spec's top block; build + headless per commit. PASS.

**Principle III + "ECS shape (v3)"**:
- One World / one driver: unchanged (V3-A's `EcsWorld`). PASS.
- Bridges: `Player`, `PlayerInputSynchronizer`, `CameraNoiseShake` have no
  `process`/`physics_process`; `input` exists only as the push-only callback on the input node,
  gated by `set_process_input(false)` as v2 (FR-013); handlers push; `set_player_id` is a
  one-shot engine write before `ready` (FR-003). Push-never-borrows: V3-A's queue. PASS.
- Tick phases: seven fixed sets / four frame sets (R4). The constitution's sentence names
  "`EngineQuery` sets" in the plural and confines them to decisions needing an engine answer
  mid-tick; the two fixed query sets are exactly those (orientation + root motion before
  integration; the move result before the respawn decision), each pure step is in a `Gameplay*`
  set, and no engine access exists outside `EngineQuery*`/sync sets and bridges. PASS.
- Node handles: `Handles::Player` in the `NonSend` map; components hold no `Gd`; dead roots
  despawn in `SyncIn` (V3-A sweep). `queue_free`: none needed (no player node is freed here).
  PASS.
- Multiplayer: authority as components — `Simulates` and `OwnsInput` markers plus `PeerId`;
  `SyncOut` writes the replicated node properties on the authority, `SyncIn` reads them on every
  peer; `.rpc("name")` residual. PASS — the constitution's bullet, implemented for the first time.
- Timers: none new (`FireCooldown` stays an engine `Timer` read at `SyncIn`, started by
  `apply_player_fx` — v2's shape). PASS.
- Tests without Godot: every pure system with `run_system_once` (data-model.md). PASS.
- Tuning constants: `PlayerTuning`, `PlayerInputTuning`, `CameraShakeTuning` become resources
  through the generic `Tuning<T>` wrapper (R6) without editing the model files; systems take
  `Res<Tuning<…>>`. PASS.
- Layout: glue may now be split into `x.rs` (bridge) + `x/sync.rs` (the entity's engine
  systems), pure into `x/model.rs` + `x/system.rs` — an extension of V3-A's "sync systems live in
  `ecs.rs`" recorded in `CLAUDE.md` (commit 4) because `ecs.rs` would otherwise grow by ~600
  lines. Principle III requires the separation, not one file. PASS.

**Governance**: SC-006's checklist (quickstart §5); parity evidence (six diffs + two
checkpoints) in the `/speckit-implement` summaries; tradeoffs rows per commit.

**Result**: PASS. Two spec-sanctioned outcomes recorded in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/012-v3-player-input-camera/
├── plan.md              # This file
├── research.md          # R1–R3 experiments, R4–R10 decisions
├── data-model.md        # components, events, Handles::Player, Initial::Player, Phase, systems, tests
├── quickstart.md        # gates, headless, harness, checkpoints, review greps
├── contracts/
│   ├── player-entity.md # bridge/sub-bridge contract, projections per peer role, the seven-set tick
│   ├── zz_ecs_parity.gd # six-case harness (scratch; never in oxide-godot/)
│   ├── zz_r1_probe.gd   # the AnimationTree ordering experiment (scratch)
│   └── zz_r3_probe.gd   # the headless input experiment (scratch)
└── tasks.md             # /speckit-tasks — not this command
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── ecs.rs                        # + Handles::Player, Initial::Player arm in apply_register, registration of the new engine systems
├── ecs/
│   ├── setup.rs                  # Phase +3 variants; seven-set fixed chain; Tuning<T> resources in build_world
│   ├── markers.rs                # + Tuning<T>, JumpQueued, PendingMouseLook, PendingFx, PlayerTag, Simulates, OwnsInput, PeerId, …
│   ├── event.rs                  # + JumpPressed, MouseLook, AddTrauma, PlayerFx variants
│   └── apply.rs                  # + the four drain arms (+ tests)
├── player.rs                     # bridge (ready registers everything; handlers push; set_player_id kept)
├── player/model.rs               # UNTOUCHED
├── player/system.rs              # pure: tick_decide, tick_integrate, tick_settle, replay_plan (+ tests)
├── player/sync.rs                # glue: sync_in_player, orient_and_anim, move_body, sync_out_player (+ advance), apply_player_fx
├── player_input.rs               # sub-bridge (ready one-shot setup; input → MouseLook; #[rpc] jump → JumpPressed)
├── player_input/model.rs         # UNTOUCHED
├── player_input/system.rs        # pure: input_decide (+ tests)
├── player_input/sync.rs          # glue: sync_in_input, camera_and_ray, sync_out_input
├── camera_noise_shake.rs         # sub-bridge (init randi(), ready seeding + start_rotation; nothing else)
├── camera_noise_shake/model.rs   # UNTOUCHED
├── camera_noise_shake/system.rs  # pure: shake_decide (+ tests)
└── camera_noise_shake/sync.rs    # glue: shake_sample, sync_out_shake
oxide-godot/player/player.tscn    # :592 callback_mode_process = 2 (R1, option B)
docs/v3-tradeoffs.md              # + 9 rows
CLAUDE.md                         # v3 section: sub-bridge pattern, seven-set tick, x/sync.rs layout
```

**Structure Decision**: the V3-A layout generalized — bridge in `x.rs`, pure core in `x/model.rs`
(unchanged) + `x/system.rs` (systems), engine systems in `x/sync.rs`; `ecs.rs` only registers
them. R9's rationale from V3-A holds: a reviewer opens one directory per module.

## Commit Plan

Four local commits on `v3` (research R10); the harness runs after commits 2 and 3; two STOPs after
commit 3 (single player, then two-instance multiplayer); docs last.

| # | Commit | Gate + validation |
|---|---|---|
| 1 | `ecs: seven fixed-schedule sets (Phase extension), Tuning<T> resources, drain arms for JumpPressed/MouseLook/AddTrauma/PlayerFx (tests)` | gates, 156 + 5 |
| 2 | `player + player_input: bridges, Player entity (Handles::Player), fixed tick (7 sets) and frame input systems; AnimationTree MANUAL + advance (R1 option B)` | gates; headless; harness (a), (b), (c), (e), (f) both trees |
| 3 | `camera_noise_shake: sub-bridge, shake systems on the player entity, AddTrauma/PlayerFx trauma path` | gates; headless; harness (d) + rerun (b), (c); **STOP 1** single player; **STOP 2** multiplayer |
| 4 | `CLAUDE.md: v3 sub-bridge pattern + two-EngineQuery tick; spec: measured timing differences filled; tradeoffs complete` | docs-only |

## Complexity Tracking

*No constitution violation. Two outcomes the spec itself provides for, recorded so they are
reviewed, not discovered:*

| Item | Why it exists | Resolution |
|---|---|---|
| `player.tscn:592` `callback_mode_process` 0 → 2 (MANUAL) | R1: a driver at `i32::MAX` reads the root motion one step early in PHYSICS mode (`M(n).rm == S(n+1).rm` at every step); MANUAL + `advance(delta)` after the tick reproduces v2 line by line on both trees | Spec FR-018 option (B), the milestone's one `.tscn` edit — an engine property, not an exported script property or a replicated name (Principle II); listed in `docs/v3-tradeoffs.md` |
| `PlayerFx::Jump`/`Land` animation writes dropped on `Simulates` entities | R2: replication samples node values at the top of `SceneTree::process`, after the physics steps; a frame-run write could be sampled in a physics-less iteration (VSync off) and would leave a stale `transition_request`; in v2 the writes were always overwritten inside the same physics step and never observable | Spec FR-015's own fallback ("dropped on `Simulates` entities with the reason recorded"); sounds kept; non-`Simulates` behavior unchanged; harness case (b) logs `current_animation` every frame to prove parity |

## Post-Design Re-check

Re-evaluated with data-model.md and the contracts written: components hold no `Gd`
(`Handles::Player` lives in `NodeHandles`); the three pure system files import nothing from
`godot::classes`; every engine call sits in `*/sync.rs` or a bridge; the seven-set chain keeps
V3-A's systems in place (R4's probe); `advance` is the last engine write of `sync_out_player`;
the two STOPs precede the docs commit. Nothing moves to Complexity Tracking beyond the two rows
above.
