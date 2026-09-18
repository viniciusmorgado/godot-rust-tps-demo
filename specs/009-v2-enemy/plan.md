# Implementation Plan: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

**Branch**: `v2` | **Date**: 2026-09-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-v2-enemy/spec.md`

**Phase**: v2 (Idiomatic Rust), constitution 1.4.1, Principles I, II, III.

## Summary

Remodel the last two gameplay modules, `red_robot.rs` (the largest script, a 4-state machine
with loose counters and three duplicated raycasts) and its dependent `part.rs`, into the v2
shape V2-B/C established: engine-free pure logic (`step`, the `animate` decomposition,
`hit_step`, a root-motion integration twin, `part.rs`'s fade/lifetime functions) covered by
`cargo test`; one raycast helper replacing three duplicated
`PhysicsRayQueryParameters3D`/`intersect_ray` blocks; `Os::has_feature("dedicated_server")` read
once instead of every frame; per-shot/per-event `get_node_as`/`load` resolved once via `OnReady`/
preloaded fields; the four `connect_other` timer chains (two in each module) replaced by the
`godot::task::spawn` async pattern already established in `part_disappear.rs`/`blast.rs`. Four
backlog items close (#15, #16, #17, #18); #28 gains a second annotation (the robot's-laser-shot
half of the first-shot-hitch observation, V2-C having already closed the player's-bullet half).
Behavioral parity against `v1` is verified via a new GDScript parity harness on two git
worktrees, plus a user visual checkpoint after each module's glue commit. The internal robot
state stays a FLAT representation (not a richer per-variant enum) — research.md R2's finding
that `v1`'s own counter usage has real irregularities (an unconditional cross-state reset, an
asymmetric `Aim`-only raycast gate while `Shooting` still drains `aim_countdown`) that a richer
enum would have to special-case without eliminating any actual invalid state the flat shape
doesn't already avoid; the type-system win this milestone delivers is the PURE, TESTED `step`
function itself, not a richer state encoding.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98, per `CLAUDE.md`).

**Primary Dependencies**: `godot = "0.5.5"` (gdext), `experimental-threads` feature only — no
`Cargo.toml` change (no new gdext feature needed: `OnReady::from_base_fn` — confirmed at
`obj/on_ready.rs:192-199` — `godot::tools::load`, `godot::task::spawn`,
`TypedSignal::to_future()`, and every math builtin this milestone uses are all unconditional in
0.5.5, confirmed in research.md R1/R4/R7).

**Storage**: N/A (no persisted state; neither `red_robot.rs` nor `part.rs` reads `Settings` at
all, confirmed by grep — research.md's Context).

**Testing**: `cargo test` (pure modules, engine-free) + headless Godot 4.7.2 (`--headless`, per
`CLAUDE.md`) + the GDScript parity harness (`zz_enemy_parity.tscn`/`.gd`, throwaway, deleted at
milestone end) run on a `v1` git worktree and the `v2` branch with separate `XDG_DATA_HOME` and
`--fixed-fps 60` (established V2-B/C pattern).

**Target Platform**: Linux (dev/CI headless target used for all validation in this repo).

**Project Type**: Godot 4.7 game project + Rust gdext extension crate (`oxide_godot_core/
oxide_godot_lib`, cdylib) — single crate, no workspace topology change, no new crate module (the
milestone only touches the two existing files plus a new `red_robot/model.rs` pure submodule).

**Performance Goals**: N/A beyond Principle III's own goal (fewer FFI crossings per frame:
one raycast helper instead of three duplicated blocks, `has_feature` read once instead of every
frame) — no numeric performance target is stated by the spec; SC-002/SC-003/SC-004 are
structural (grep or code review), not a benchmark.

**Constraints**: Behavioral parity with `v1` except the four pre-authorized backlog closures
(#15, #16, #17, #18 — see spec top block); `player.rs`/`bullet.rs`/`hittable.rs`/`door.rs`
(V2-C) are read-only references (`integrate_root_motion`'s shape as a twin-not-reuse precedent,
`add_camera_shake_trauma`'s exact signature, `HitTarget`'s existing shape) — none of those files
are edited by this milestone (research.md R6 confirms `player.rs` specifically stays untouched:
reusing its private `mod model` would require a visibility edit to a file the spec's own
Assumptions name as out of scope, so `red_robot/model.rs` gets an independent twin instead); no
`.tscn` edits required anywhere (`red_robot.tscn`'s `SceneReplicationConfig`, `[connection]`s,
and method tracks, verified unaffected by a name-preserving Rust remodel — research.md's Context
cross-checks, including the grep-confirmed absence of any stored `aim_preparing`/`test_shoot`
override to lose when `#[export]` is removed).

**Scale/Scope**: 2 source files (470 + 120 = 590 lines today) plus 1 new file
(`red_robot/model.rs`, the pure submodule), 0 edits to any other module (`level.rs`/`bullet.rs`/
`hittable.rs`/`player.rs`/`door.rs`/`red_robot.tscn` all verified unchanged), ≥ 15 new unit tests
across 2 pure surfaces (`red_robot/model.rs`, `part.rs`'s inline `mod pure`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see "Post-Design
Re-check" below.*

**Principle I (Three-Phase Port)** — v2 block:
- Both pillars per user story: type-system idioms (`RobotTuning`, `RobotInputs`, `Cmd`,
  `RobotCounters`, `RayHit`, all typed; `player: Option<Gd<Player>>` typed, backlog #16) AND FFI
  reduction via Principle III (ONE raycast helper instead of three duplicated blocks, one
  `has_feature` read instead of per-frame, `OnReady`/preloaded fields instead of per-shot/
  per-event `get_node_as`/`load`, `godot::task::spawn` instead of four `connect_other` chains) —
  done together per story (each Independent Test in spec.md exercises both). PASS.
- Behavioral parity: every acceptance scenario maps to a harness case (research.md R8) or a unit
  test, plus a user visual checkpoint. The ONLY behavior deltas are the four backlog items the
  spec's top block names as closed (#15, #16, #17, #18) — #16/#18 are, by verification, not
  observable at all (dead code / unused export); #15 is a scene-tree-structure change with no
  visible difference under normal play; #17 is a timing-preserving implementation swap. PASS.
- Replicated/exported surface preserved: `target_position`, `health`, `state`, `dead`
  (`red_robot.rs`) and `fade_value`, `lifetime`, `lifetime_random`, `disappearing_time`
  (`part.rs`) keep their exact names/types/`SceneReplicationConfig` modes (research.md Context,
  data-model.md's cross-reference table). `aim_preparing`/`test_shoot` LOSE `#[export]` — this
  is backlog #18's explicit closure, pre-authorized by the spec's top block, not an
  unauthorized surface change. PASS.
- Engine API gaps: none introduced (checked against `docs/api-gaps.md`'s existing single entry,
  `Scaling3DMode::NEAREST`, unrelated). PASS (N/A).

**Principle II (Verifiable Port Cycle)**:
- Binding by type: unchanged, no node's registered Rust class or base type changes. PASS (N/A).
- Bottom-up / infrastructure-first order: `red_robot.rs` is the LAST gameplay hub — every module
  it depends on (`player.rs`, `hittable.rs`/`HitTarget`) is already v2-shaped from V2-C;
  `part.rs` depends only on `red_robot.rs` (typed, unchanged direction). Remodeling both now,
  with `red_robot.rs` before `part.rs` (research.md R9's commit order — `part.rs`'s `explode`
  contract is a dependency of `red_robot.rs::hit`, but the ROUTE this milestone takes commits
  the pure model first regardless of which glue file depends on which), matches "infrastructure
  first" exactly as V2-A/B/C did before it. PASS.
- Preservation of property names: see Principle I above; re-verified directly against
  `red_robot.tscn`'s `SceneReplicationConfig_h6xi0`/`_hqtbc`, `[connection]`s, and method-track
  names in research.md's Context section. PASS.
- Residual dynamic access: `red_robot.rs`'s `.rpc("play_shoot")` and `part.rs`'s
  `.rpc("destroy")` are the only ones touched by this milestone, both the constitution's own
  named exception (no typed alternative exists for RPC dispatch) — listed in spec.md's top
  block. The inherited `HitTarget::rpc_hit()`'s `.rpc("hit")` (V2-C) is unmodified. The dead
  `body.get_name() == "Target"` string comparison (backlog #16, a FORBIDDEN dynamic-name check,
  not a residual exception) is REMOVED, not introduced. No new dynamic access is added anywhere.
  PASS.
- Mandatory build / headless validation: enforced per commit (Commit Plan below). PASS.

**Principle III (Interface vs. Implementation)** — the milestone's central bet, and its richest
test yet (the largest state machine in v2 so far):
- Glue-only trait/API impls: every `I<Base>` callback (`ready`, `physics_process`) and every
  `#[func]`/`#[rpc]` in the 2 modules reads a snapshot, calls a pure function, writes the result
  — no state machine, no math, no decision tree stays inside a callback body after this
  milestone. PASS, enforced task-by-task at `/speckit-tasks`/`/speckit-implement` time.
- Pure logic engine-free + unit tested: `red_robot/model.rs` (US1), `part.rs`'s inline `mod pure`
  (US2) — neither uses `Gd<T>`, singletons, or engine-backed builtins; every builtin math method
  either module calls is confirmed pure under the 1.4.1 rule with source citations (research.md
  R1 — `Basis::transposed`/`Mul<Vector3>`/`orthonormalized`/`from_quaternion`,
  `Transform3D`'s `Mul` impls, `Vector2/3`'s `length`/`normalized`/`distance_to`/`clamp`, all
  glam-based; `f32::atan2`/`to_degrees` are plain `std`). `RayHit` (glue-facing, contains
  `Option<Gd<Object>>`) is explicitly GLUE, like V2-C's `HitTarget` — Principle III's pure-module
  ban does not apply to it, not counted toward the ≥15 new tests.
- Snapshot → step → apply: `RobotInputs` (US1) is a third concrete instance of the pattern
  `player_input/model.rs`'s `InputSnapshot` and `player/model.rs`'s `InputFrame` established;
  `part.rs` reads its inputs once per event before computing.
- No per-frame/per-event lookups: ONE `raycast_to` helper (was 3 duplicated blocks), one
  `is_dedicated_server` field read at `ready` (was every frame), `OnReady`/preloaded-`PackedScene`
  fields (were per-shot/per-event `get_node_as`/`load` — 1 in `red_robot.rs::shoot`'s
  `impact_effect.tscn` load + 1 `get_node_as` for `LaserEmber`; 4 in `part.rs`'s
  `MultiplayerSynchronizer`/`Col1`/`Col2`/`Model`-child + 1 `load` for `part_disappear.tscn`).
  SC-002/SC-003 are these facts, verbatim.
- Gates: `cargo build && cargo clippy && cargo test` before every commit (Commit Plan). PASS.
- Tuning constants on a struct: `RobotTuning` — 8 fields with `Default`/const values matching
  `v1`'s literals — see data-model.md. `part.rs` has no NEW tunable constants beyond its
  existing `#[export]`s (FR-013, unchanged). PASS.

**Governance** (v2-specific review criteria, self-checked ahead of the formal review): backlog
bookkeeping happens in the milestone's final commit (`docs/v2-backlog.md` rows #15, #16, #17,
#18 → done citing this milestone's commits; #28's existing open row gets a SECOND annotation,
not a status change); `docs/v2-catalog.md`'s stale "gdext math types... never cross the FFI"
claim (predating the 1.4.1 constitution correction) is fixed in the same docs commit, since this
milestone is the first to touch that file's "Facts that shape the design" section since 1.4.1
landed; behavioral parity evidenced by the harness diff + 2 user visual checkpoints, reported in
the `/speckit-implement` final summary, per V2-A/B/C's precedent.

**Result**: PASS, no violations to record in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/009-v2-enemy/
├── plan.md              # This file
├── research.md          # Phase 0 output — R1-R9
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── enemy-api.md
│   └── zz_enemy_parity.gd
└── tasks.md             # Phase 2 output (/speckit-tasks — not this command)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── red_robot.rs                 # glue: I<CharacterBody3D>, #[signal]/#[rpc]/#[func]s,
│                                 #   raycast_to helper, OnReady node refs + preloaded
│                                 #   impact_effect_scene: Gd<PackedScene>, is_dedicated_server
├── red_robot/
│   └── model.rs                 # pure: RobotTuning, RobotInputs, Cmd, RobotCounters, step,
│                                 #   resume_approach_reset, the animate decomposition
│                                 #   (transition_request, aim_blend_amount, cannon_angles,
│                                 #   aim_blend_step, ember_position, ember_extents), hit_step,
│                                 #   integrate_root_motion (twin), idle_velocity,
│                                 #   shoot_countdown_will_expire, aim_countdown_will_expire,
│                                 #   + #[cfg(test)]
└── part.rs                       # glue + inline `mod pure { fade_curve, should_destroy,
                                   #   random_angular_velocity, wait_time, #[cfg(test)] }`
```

No `lib.rs` edit needed (`mod red_robot;`/`mod part;` already exist; `red_robot/model.rs` is a
private submodule of the existing `red_robot` module, exactly like V2-C's `player/model.rs`
under `player`).

**Structure Decision**: mirrors V2-C's `<module>.rs` (glue) + `<module>/model.rs` (pure) split
for `red_robot.rs`, whose pure surface is the largest in v2 so far (research.md R2/R3) and
warrants its own file and its own review-sized commit, matching `player`'s precedent exactly.
`part.rs` keeps its pure surface (four small functions) as an inline `mod pure`, matching
`bullet.rs`'s V2-C precedent that file boundary follows the pure surface's size, not a fixed
rule — smaller than `red_robot`'s pure surface, comparable to `bullet.rs`'s/`door.rs`'s.

## Commit Plan

One commit per module in dependency order (pure model before its glue), docs last.

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `red_robot: extract RobotTuning, State-projected step, animate decomposition, hit_step and a root-motion integration twin into red_robot/model.rs` | `red_robot/model.rs` (new) | `cargo build && cargo clippy && cargo test` (new tests pass) |
| 2 | `red_robot: raycast_to helper, OnReady/preloaded resources, step-driven physics_process, async removal/trauma waits; closes backlog #16, #17, #18` | `red_robot.rs` | gates + headless (`main.tscn`) |
| 3 | `part: pure fade/lifetime, OnReady resources incl. from_base_fn Model child, async waits, puff parented under the robot's own parent; closes backlog #15` | `part.rs` | gates + headless |
| 4 | `docs: close backlog #15, #16, #17, #18 citing this milestone's commits; annotate #28 with the robot-laser half; add #31 (reset counters on Approach entry); correct docs/v2-catalog.md's stale pure-builtins claim to the 1.4.1 rule` | `docs/v2-backlog.md`, `docs/v2-catalog.md` | none (docs-only) — #31 row: origin `enemies/red_robot/red_robot.gd` / `red_robot.rs` (found in V2-D research R2); improvement "reset `aim_preparing`/`shoot_countdown`/`aim_countdown` when the player enters the detection area (`_on_area_body_entered` → `Approach`) instead of resuming with the stale values left by the previous state; once fixed, the internal state can become an enum with per-variant counters (spec 009 FR-002's intended shape)"; motivation "v1 quirk kept for parity in V2-D — the counters outlive `Idle`, so a per-state enum cannot represent v1 faithfully today"; status `open` |

Parity-harness runs (throwaway, uncommitted per V2-B/C's convention — `zz_*` files are never
committed) happen after commit 2 (US1 checkpoint) and after commit 3 (US2 checkpoint) — each
followed by a STOP for the user's visual confirmation, exactly as V2-C's two checkpoints did.

## Complexity Tracking

No Constitution Check violation, but ONE justified deviation from the spec's own FR-002 (not a
constitutional rule — a spec requirement whose premise the research disproved):

| Deviation | Why needed | Simpler alternative rejected because |
|---|---|---|
| FR-002 asks that each counter's validity be "tied to the states in which v1 uses it" (an enum with per-variant counters was the sketched shape). The plan keeps `aim_preparing`/`shoot_countdown`/`aim_countdown` FLAT (research.md R2). | v1's counters are NOT per-state: they survive `Idle`, and `_on_area_body_entered` re-enters `Approach` WITHOUT resetting them, so the robot resumes with the stale `shoot_countdown`; `resume_approach()` (animation method track) resets unconditionally from any state. A per-variant enum could only reproduce this by carrying the counters through `Idle` — which makes the enum meaningless — or by resetting on entry, which is a behavior change (parity break). | Resetting on `Approach` entry is the right FIX, but it is an improvement, not a port: recorded as backlog #31 (open) in commit 4, so the enum shape can be adopted once #31 is deliberately closed. The type-system delivery of this milestone is the pure, tested `step` over the flat `State` (FR-007), not the counter encoding. |
