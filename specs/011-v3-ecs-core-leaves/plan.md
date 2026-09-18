# Implementation Plan: Milestone V3-A — ECS core and the three leaf effects

**Branch**: `v3` | **Date**: 2026-09-18 | **Spec**: [spec.md](./spec.md) (committed at `e746e28`)

**Input**: Feature specification from `/specs/011-v3-ecs-core-leaves/spec.md`

**Phase**: v3 (ECS layer over the nodes), constitution 1.5.1 (`85186f6`), Principles I, II, III
including the "ECS shape (v3)" subsection.

## Summary

Add `bevy_ecs 0.19` (one workspace line, `default-features = false, features = ["std"]`) and
build the ECS core every later v3 milestone reuses verbatim: one autoload `EcsWorld` that owns
the `World` and drives a `Fixed` and a `Frame` schedule from its own `physics_process`/`process`
at priority `i32::MAX` (last in each phase — proven by experiment, research R1); a
`thread_local!` inbound queue that bridges push to without ever borrowing the World (R2);
`InstanceId → Entity` and `Entity → typed handles` maps, the second a `NonSend` resource (R4);
the four chained sets `SyncIn → Gameplay → EngineQuery → SyncOut` in both schedules (R8); an
engine-arithmetic `Timer` component (R5); consumed marker components for every engine-facing
decision (R6); and a Godot-free system-test recipe (R9). The three smallest gameplay modules
become bridges and prove the three mechanisms: `door` (signal → message → pure system →
`SyncOut` plays the animation), `part_disappear` (two `godot::task` awaits → a `DisappearPhase`
stepped by the frame schedule), `blast` (per-frame `SyncIn`/`SyncOut` pair with `look_at`
gated by real change, `animation_finished` → `Remove`, no gameplay system — R7). Parity against
branch `v2` is evidenced per story by a three-case headless harness on two worktrees with an
`i32::MIN`-priority observer (R10) and two user visual checkpoints; frame-level RAW-stamp
differences the harness measures go into the spec's timing table (R5 predicts exactly which).

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98, per `CLAUDE.md`), edition 2024.

**Primary Dependencies**: `godot = "0.5.5"` (gdext, `experimental-threads` on — unchanged);
NEW: `bevy_ecs = { version = "0.19", default-features = false, features = ["std"] }` in
`[workspace.dependencies]`, inherited with `{ workspace = true }` (FR-001). Resolves to 0.19.1
(`bevy_ecs-0.19.1` in the local registry; 0.20 is `-rc`, forbidden). MSRV 1.95 ≤ 1.98. No
`bevy_reflect`, `async_executor`, `multi_threaded`. Crate-graph delta (SC-007): baseline
`cargo tree --prefix none | sort -u | wc -l` = 22 on `85186f6` (2026-09-18); measured in commit 1
(2026-09-18): **77** after the pin, i.e. **+55** lines (unique crate names: 66; the extra lines are
second versions of crates already in `godot`'s graph). The spec's "at most 44" (SC-007) was the
probe crate's figure and is exceeded by the real graph — reported, not hidden.

**Storage**: N/A (no persisted state; `Settings` untouched).

**Testing**: `cargo test` (pure ECS submodules and gameplay systems via `World::new()` +
`RunSystemOnce`, no Godot binary) + headless Godot 4.7.2 (import, `main.tscn`, `level.tscn`, a
scratch autoload check) + the three-case GDScript parity harness (`contracts/zz_ecs_parity.gd`,
throwaway) on a `v2` worktree and on `v3` with separate `XDG_DATA_HOME`, `--fixed-fps 60`.

**Target Platform**: Linux (dev/CI headless target used for all validation in this repo).

**Project Type**: Godot 4.7 game project + Rust gdext extension crate (cdylib) — single crate,
no workspace topology change; one new autoload scene (`oxide-godot/ecs/ecs_world.tscn`).

**Performance Goals**: none numeric (constitution: cache locality is explicitly NOT a v3 goal).
The only count the spec fixes is FR-025's per-blast engine calls — see Complexity Tracking.

**Constraints**: behavioral parity with `v2` at `e2932b4` except measured frame-level timing
shifts recorded in the spec (FR-030); no backlog item closed, #30 deferred; scope lists of
Principle I v3 (only `door`, `part_disappear`, `blast`, the new `ecs` module, `project.godot`,
`Cargo.toml`, `CLAUDE.md`, docs change); preserved surfaces (`_on_door_body_entered(Gd<Node3D>)`,
type names `Door`/`PartDisappear`/`Blast`); no `.tscn` edits beyond the new autoload scene and
the `[autoload]` line.

**Scale/Scope**: 3 bridge files (83 + 48 + 49 lines today), 1 new module `ecs` with 7
submodules, 2 gameplay-system files, ≥ 15 new unit tests (SC-001), 1 new doc file
(`docs/v3-tradeoffs.md`, 4 entries), 1 `CLAUDE.md` section.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design — see "Post-Design Re-check".*

**Principle I (Three-Phase Port)** — v3 block:
- Two objectives per story: v2's pillars kept (typed `DoorState`/`DisappearPhase`/`Timer`
  enums and structs; glue thin, logic pure) AND the ECS over the nodes (every gameplay decision
  is a system in one World/one Schedule; no bridge runs per-frame logic). PASS — each story's
  Independent Test in spec.md exercises both.
- Non-objectives respected: no data-layout argument anywhere in research.md (R7's `Changed`
  gate is justified by FFI count parity, not locality); physics, rendering, animation,
  instancing stay Godot's (R3, R7, tradeoffs entry d); no `bevy` App, no `godot-bevy`. PASS.
- Dependency policy: exactly `bevy_ecs`, stable line, pinned features (FR-001); no supporting
  crate. PASS.
- Scope lists: ECS-rewritten here — `door`, `part_disappear`, `blast` (three of the ten listed);
  the five excluded modules and `part`/`bullet`/`red_robot` are diff-empty (SC-006; the three
  scenes are instanced through `PackedScene`, never through typed calls — spec Context). PASS.
- Parity baseline `v2`, verified by headless + seeded harness on both trees + visual
  checkpoints (R10, R11). Behavior changes: none sanctioned beyond measured timing shifts; R5
  predicts RAW-stamp shifts for the puff (observer-invisible), to be recorded per FR-030. PASS.
- Preserved surfaces (Principle II): `door.tscn:35` connection kept, type names kept, no
  `#[export]`/`#[var]`/`SceneReplicationConfig` involved (spec Context). PASS.
- Engine touch points → `docs/v3-tradeoffs.md` in the same commit (R11 commits 3-5). The v2
  api-gap rule: no new gap (nothing missing from the bindings; `set_process_priority`,
  `set_physics_process_priority`, `queue_free` present). PASS.
- Phase governance: v2 complete at `e2932b4`; this spec declares v3. PASS.

**Principle II (Verifiable Port Cycle)**:
- Binding by type unchanged (no node's class or base changes; the new `EcsWorld` is bound by
  type through `ecs_world.tscn`, like `Settings`). PASS.
- Order: infrastructure first (`ecs` core, commit 1-2) before its consumers (commits 3-5). PASS.
- Property names: none exported/replicated among the three (spec Context). PASS.
- Dynamic access: none introduced; the only engine-name residual (`.rpc`) does not occur.
  `get_autoload_by_name::<EcsWorld>` is not needed by any bridge (bridges push to the queue;
  R8), so FR-003 is satisfied vacuously; a GDScript `/root/EcsWorld` lookup exists only in the
  scratch autoload check. PASS.
- Mandatory build + headless validation per commit (R11). PASS.

**Principle III (Interface vs. Implementation) + "ECS shape (v3)"**:
- One World, one Schedule (two schedule objects, one owner, one driver), reached by typed
  autoload lookup only; exactly one node calls `Schedule::run` (`ecs.rs`; SC-002). PASS.
- Bridges: no `process`/`physics_process`; `ready` registers, `exit_tree` unregisters, handlers
  only push (data-model.md "Bridges"). Re-entrancy: the push path is a module-private
  `thread_local!` `RefCell` borrowed for one `Vec::push`; no `bind_mut` on `EcsWorld`, no
  `&mut World` (R2) — by construction. PASS.
- Tick phases: `SyncIn → Gameplay → EngineQuery → SyncOut` chained in both schedules (R8);
  gameplay systems pure (`door::system::open_on_player`, `part_disappear::system::advance` —
  `Query`/`Res`/`MessageReader`/`Commands` only); `EngineQuery` empty; engine access only in
  the `SyncIn`/`SyncOut` systems of `ecs.rs` and in bridges; `Changed<LookTarget>` gates the
  one idempotent write (R7). PASS.
- Node handles: `Gd<T>` only inside `Handles` in the `NonSend` `NodeHandles`; components hold no
  handle; a dead root despawns in `SyncIn` (both schedules); nodes are freed only by
  `sync_out_remove` via `queue_free()` (R4, R6). PASS.
- Multiplayer: no replicated property among the three modules; no authority component needed
  (spec Out of scope). PASS (N/A).
- Timers: `Timer` component stepped in `Frame` by `advance`; `godot::task::spawn` removed from
  the three modules (SC-002). PASS.
- Tests without Godot: every pure submodule and both gameplay systems, `World::new()` +
  `run_system_once` (R9, data-model.md test names). Gates: build/clippy/test per commit. PASS.
- Tuning constants: the puff's two literals belong to the type that uses them —
  `DisappearPhase::EMIT_DELAY: f64 = 0.2` and `DisappearPhase::LIFETIME_FACTOR: f32 = 2.0`
  (data-model.md), reached through `DisappearPhase::start()` so the registration path never
  sees the literal; no tuning struct for two values. Required by the Principle III rule (tuning
  constants belong to the type), not optional — plan review, 2026-09-18. PASS.

**Governance** (v3 review criteria, self-checked): the SC-006 grep list in quickstart.md §5
implements the compliance-review sentence of the constitution one item at a time; parity
evidence (three harness diffs + two checkpoints) is reported in the `/speckit-implement`
summary; `docs/v3-tradeoffs.md` rows land in the commits that introduce each touch point.

**Result**: PASS on every constitution item. One SPEC-INTERNAL tension (not a constitution
violation) was recorded in Complexity Tracking and resolved by amending FR-025.

## Project Structure

### Documentation (this feature)

```text
specs/011-v3-ecs-core-leaves/
├── plan.md              # This file
├── research.md          # Phase 0 — R1 (experiment) … R11 (commit plan)
├── data-model.md        # Phase 1 — exact Rust surface, v2 line per item, test names
├── quickstart.md        # Phase 1 — gates, headless, harness, checkpoints, review greps
├── contracts/
│   ├── ecs-api.md       # bridge contract, system-author contract, tradeoffs table header
│   ├── zz_ecs_parity.gd # three-case parity harness + observer (scratch; never in oxide-godot/)
│   └── zz_order_probe.gd# the R1 experiment, for re-verification (scratch)
└── tasks.md             # Phase 2 output (/speckit-tasks — not this command)
```

### Source Code (repository root)

```text
oxide_godot_core/
├── Cargo.toml                       # + bevy_ecs in [workspace.dependencies]
└── oxide_godot_lib/
    ├── Cargo.toml                   # + bevy_ecs = { workspace = true }
    └── src/
        ├── lib.rs                   # + mod ecs;
        ├── ecs.rs                   # glue: EcsWorld (autoload + driver, priorities i32::MAX), Handles,
        │                            #   NodeHandles, apply_register, sweep_dead_nodes, sync_in_blast,
        │                            #   sync_out_{door,puff,blast,remove}, add_engine_systems
        ├── ecs/
        │   ├── queue.rs             # pure: thread_local queue, push/drain + tests
        │   ├── event.rs             # InboundEvent, Initial, DoorBodyEntered (Message)
        │   ├── timer.rs             # pure: Timer + tests
        │   ├── index.rs             # pure: EntityIndex + tests
        │   ├── apply.rs             # pure: apply_non_register (Unregister / DoorBodyEntered / BlastAnimationFinished) + tests
        │   ├── setup.rs             # pure: Phase, Fixed/Frame labels, build_world/build_fixed/build_frame + tests
        │   └── markers.rs           # PlayOpen, StartEmitting, Remove, BlastTag, LookTarget, FrameDelta, FixedDelta
        ├── door.rs                  # bridge
        ├── door/system.rs           # pure: DoorState, on_body (v2 verbatim), open_on_player + tests
        ├── part_disappear.rs        # bridge
        ├── part_disappear/system.rs # pure: DisappearPhase, Lifetime, Transition, advance + tests
        └── blast.rs                 # bridge (sync-only entity; no system file)
oxide-godot/
├── project.godot                    # [autoload] + EcsWorld="*res://ecs/ecs_world.tscn"
└── ecs/ecs_world.tscn               # [node name="EcsWorld" type="EcsWorld"]
docs/v3-tradeoffs.md                 # new (commit 3), 4 entries by commit 5
CLAUDE.md                            # + "Port conventions (v3)" (commit 6)
```

**Structure Decision**: `ecs.rs` (glue) + `ecs/*.rs` (pure) generalizes v2's `x.rs` +
`x/model.rs`; gameplay systems sit beside their bridges (`door/system.rs`,
`part_disappear/system.rs`) because that is where a reviewer already looks for a module's pure
core (research R9). `blast` gets no system file: its only decisions (unknown id → drop, known →
`Remove`) are mechanical and applied by the drain (R3), which keeps the spec's "no pure gameplay
system for blast" literally true.

## Commit Plan

Six local commits on `v3` (research R11), infrastructure first, one bridge per commit, docs
last; the harness runs after commits 3, 4, 5 with a STOP for the user's visual checkpoint after
4 and after 5 (none for the door — orphaned scene).

| # | Commit | Gate + validation |
|---|---|---|
| 1 | `ecs: pure core — queue, timer, index, apply, setup (tests); bevy_ecs 0.19 pinned` | gates; `cargo tree` delta recorded in this plan's Technical Context, so `plan.md` is part of commit 1 (SC-007) |
| 2 | `ecs: EcsWorld autoload + driver (priorities i32::MAX) + sync systems; ecs_world.tscn; project.godot autoload` | gates + headless import/`main.tscn`/`level.tscn` + scratch autoload check; R1 output in the commit body |
| 3 | `door: bridge + open_on_player system (v2 on_body preserved, 3 tests + round-trip); docs/v3-tradeoffs.md created (entry d)` | gates + headless + harness (a) on both trees |
| 4 | `part_disappear: bridge + DisappearPhase/Timer system; tradeoffs entry (a)` | gates + headless + harness (b); **STOP: checkpoint (1)** |
| 5 | `blast: bridge + SyncIn/SyncOut pair, animation_finished → Remove; tradeoffs entries (b), (c)` | gates + headless + harness (c); **STOP: checkpoint (2)** |
| 6 | `CLAUDE.md: Port conventions (v3); spec: measured timing differences filled` | docs-only |

## Complexity Tracking

*No constitution violation. One spec-internal tension, recorded and resolved by the user at plan
review:*

| Item | Why it exists | Resolution |
|---|---|---|
| FR-025 ("per blast per frame the engine calls MUST be at most v2's three") vs FR-009 (`SyncIn` validity sweep of every registered entity) | v2 makes 3 explicit engine calls per blast per frame (`blast.rs:42-47`). v3 makes the same 3 (camera validity + transform, both shared among blasts caching the same camera; `look_at` only when the target changed) PLUS the root validity check FR-009 mandates — 4 for a lone blast, ≤ 3 amortized from two concurrent blasts (research R7). Dropping the frame-schedule sweep would leave `look_at` unguarded against a node freed between the physics step and the frame's `SyncOut`. | RESOLVED at plan review (2026-09-18, user-approved): FR-009 kept; FR-025 amended in spec.md with the clause "excluding the FR-009 root validity sweep". The implementation still reports the measured count (3 + sweep for a lone blast, ≤ 3 + sweep amortized). |

## Post-Design Re-check

Re-evaluated after research.md, data-model.md and contracts were written: every Constitution
Check item above still holds with the concrete types (data-model.md) — components hold no
`Gd<T>` (`Handles` lives only in `NodeHandles`, `NonSend`); gameplay systems take no `NonSend`
parameter; bridges declare no per-frame callback; the driver is the only `Schedule::run` caller;
`queue_free()` is the only release path; `Timer` is a component; tests need no engine. The R1
experiment closed the one open premise (driver-last is achievable, including relative to the
`AnimationPlayer`'s internal processing), so nothing moves to Complexity Tracking from it.
