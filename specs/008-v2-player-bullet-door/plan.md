# Implementation Plan: Milestone V2-C — player, bullet, door

**Branch**: `v2` | **Date**: 2026-09-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-v2-player-bullet-door/spec.md`

**Phase**: v2 (Idiomatic Rust), constitution 1.4.0, Principles I, II, III.

## Summary

Remodel the hub module `player.rs` and its two direct consumers `bullet.rs`/`door.rs` into the
v2 shape established by V2-B: engine-free pure logic (`airborne_step`, orientation slerp,
root-motion integration, `AnimPlan` selection, `BulletState::step`, `DoorState::on_body`)
covered by `cargo test`; glue that snapshots `player_input` exactly once per physics frame into
one `InputFrame` instead of six-plus separate binds; per-shot resource lookups (`get_node_as`,
`load::<PackedScene>`) resolved once via `OnReady`/a plain preloaded field; `bullet.rs`'s
`has_method("hit")` duck typing replaced by a new crate-level `HitTarget` dispatch that V2-D's
`EnemyRobot` will also adopt. Five backlog items close (#2, #10, #11, #12, #13); #14 closes in a
deliberately revised form (keeping `_on_door_body_entered`'s `Gd<Node3D>` parameter, since
typing it as `Gd<Player>` would make the engine print a conversion warning for every non-player
body — a behavior change the spec explicitly rejects). Behavioral parity against `v1` is
verified per user story via a new GDScript parity harness (a self-built flat floor, not
`level.tscn`, which is too nondeterministic for a scripted trace) on two git worktrees, plus a
user visual checkpoint after US1 and after US2+US3.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98, per `CLAUDE.md`).

**Primary Dependencies**: `godot = "0.5.5"` (gdext), `experimental-threads` feature only — no
change to `Cargo.toml` (no new gdext feature needed: `OnReady`, `godot::tools::load`, `Gd<T>`
`Deref`-to-base-class dispatch, and every math builtin this milestone uses are unconditional in
0.5.5, confirmed in research.md R1/R4/R5/R2).

**Storage**: N/A (no persisted state; `Settings` — read typed since V2-A — is untouched by this
milestone's changes to `bullet.rs`'s `explode`).

**Testing**: `cargo test` (pure modules, engine-free) + headless Godot 4.7.2 (`--headless`, per
`CLAUDE.md`) + the GDScript parity harness (`zz_player_parity.tscn`/`.gd`, throwaway, deleted at
milestone end) run on a `v1` git worktree and the `v2` branch with separate `XDG_DATA_HOME` and
`--fixed-fps 60` (both established in V2-B).

**Target Platform**: Linux (dev/CI headless target used for all validation in this repo).

**Project Type**: Godot 4.7 game project + Rust gdext extension crate (`oxide_godot_core/
oxide_godot_lib`, cdylib) — single crate, no workspace topology change. One NEW crate module
(`hittable.rs`) added at the crate root alongside the existing flat module list in `lib.rs`.

**Performance Goals**: N/A beyond Principle III's own goal (fewer FFI crossings per frame,
concretely: one `player_input` bind per physics frame instead of six-plus) — no numeric
performance target is stated by the spec; SC-002/SC-003/SC-004 are structural (grep or code
review), not a benchmark.

**Constraints**: Behavioral parity with `v1` except the five pre-authorized backlog closures
(#2, #10, #11, #12, #13) and #14's revised closure (see spec top block); `red_robot.rs`/`part.rs`
outside scope except being a read-only reference for `HitTarget`'s `EnemyRobot` variant and the
harness's robot-hit scenario; no `.tscn` edits required anywhere (`player.tscn`'s
`SceneReplicationConfig`, `bullet.tscn`'s method-call track, and `door.tscn`'s `[connection]`
are all verified unaffected by a name-preserving Rust remodel — research.md's Context
cross-checks).

**Scale/Scope**: 3 source files (309 + 87 + 31 = 427 lines today) plus 1 new file
(`hittable.rs`, ~25 lines), 0 edits to any other module (`level.rs`/`red_robot.rs`/`door.tscn`/
`bullet.tscn`/`player.tscn` all verified unchanged), ≥ 15 new unit tests across 3 pure surfaces
(`player/model.rs`, `bullet.rs`'s inline `mod pure`, `door.rs`'s inline `mod pure`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see "Post-Design
Re-check" below.*

**Principle I (Three-Phase Port)** — v2 block:
- Both pillars per user story: type-system idioms (`AirborneOutcome`, `AnimPlan`,
  `PlayerTuning`, `BulletState`, `HitTarget`, `BulletTuning`, `DoorState`, all typed) AND FFI
  reduction via Principle III (one `InputFrame`/frame instead of six-plus binds, `OnReady`/
  preloaded resources instead of per-shot `get_node_as`/`load`, `match` over `AnimPlan` instead
  of an `if`/`else if` chain) — done together per story, not split across stories. PASS (each
  user story's Independent Test in spec.md exercises both).
- Behavioral parity: every story's acceptance scenarios map to a harness case (research.md R8)
  or a unit test, plus a user visual checkpoint. The ONLY behavior deltas are the backlog items
  the spec's top block names as closed (#10, #11, #13) plus #14's revised closure. PASS.
- Replicated/exported surface preserved: `player_id`, `motion`, `current_animation` keep their
  exact name/type/attributes (`SceneReplicationConfig` unedited, confirmed in spec Context);
  `bullet.tscn`'s `destroy` method-call track and `door.tscn`'s `[connection]` are both
  unedited. PASS.
- Engine API gaps: none introduced by this milestone (checked against `docs/api-gaps.md`'s
  existing single entry, `Scaling3DMode::NEAREST`, unrelated). PASS (N/A).

**Principle II (Verifiable Port Cycle)**:
- Binding by type: unchanged, no node's registered Rust class or base type changes. PASS (N/A).
- Bottom-up / infrastructure-first order: `player.rs` is the hub `door.rs`/`bullet.rs` (and,
  outside this milestone, `red_robot.rs`/`level.rs`) already consume typed; remodeling the hub
  before its already-typed consumers matches "infrastructure first" exactly as V2-A did for
  `Settings`. PASS.
- Preservation of property names: see Principle I above; re-verified directly against
  `player.tscn`'s `SceneReplicationConfig`, `bullet.tscn:103`'s method-track, `door.tscn:35`'s
  `[connection]` in research.md's Context section. PASS.
- Residual dynamic access: `player.rs`'s `.rpc("jump"/"land"/"shoot")` and `bullet.rs`'s
  `.rpc("explode"/"hit")` are the only ones touched by this milestone, both the constitution's
  own named exception (no typed alternative exists for RPC dispatch) — listed in spec.md's top
  block. `has_method("hit")` (a FORBIDDEN dynamic-access form, not a residual exception) is
  REMOVED by this milestone (FR-015), not introduced. No new dynamic access is added anywhere.
  PASS.
- Mandatory build / headless validation: enforced per commit (Commit Plan below). PASS.

**Principle III (Interface vs. Implementation)** — the milestone's central bet:
- Glue-only trait/API impls: every `I<Base>` callback (`ready`, `physics_process`) and every
  `#[func]`/`#[rpc]` in the 3 modules reads a snapshot, calls a pure function, writes the
  result — no state machine, no math, no decision tree stays inside a callback body after this
  milestone. PASS, enforced task-by-task at `/speckit-tasks`/`/speckit-implement` time.
- Pure logic engine-free + unit tested: `player/model.rs` (US1), `bullet.rs`'s inline
  `mod pure` (US2), `door.rs`'s inline `mod pure` (US3) — none uses `Gd<T>`, singletons, or
  engine-backed builtins; `Vector2`/`Vector3`/`Basis`/`Quaternion`/`Transform3D` only
  (constitution's named exception, re-confirmed for this milestone's specific method calls in
  research.md R2). `HitTarget` (`hittable.rs`) is explicitly GLUE (uses `Gd<T>` by design,
  research.md R5) — Principle III's pure-module ban does not apply to it; it is not counted
  toward the ≥15 new tests.
- Snapshot → step → apply: `InputFrame` (US1) is a second concrete instance of the pattern
  `player_input/model.rs`'s `InputSnapshot` established in V2-B; `bullet.rs`/`door.rs` read
  their inputs once per frame/event before computing.
- No per-frame/per-event lookups: `OnReady` node fields + a preloaded `Gd<PackedScene>` field
  eliminate `player.rs`'s two per-shot `get_node_as` calls and its per-shot
  `load::<PackedScene>` call (backlog is implicitly closed by this — no separate backlog # was
  filed for it, it is simply Principle III compliance); SC-002 is this fact, verbatim.
- Gates: `cargo build && cargo clippy && cargo test` before every commit (Commit Plan). PASS.
- Tuning constants on a struct: `PlayerTuning`, `BulletTuning`/associated const — both with
  `Default`/const values matching `v1`'s literals — see data-model.md. PASS.

**Governance** (v2-specific review criteria, self-checked ahead of the formal review): backlog
bookkeeping happens in the milestone's final commit (`docs/v2-backlog.md` rows #2, #10, #11,
#12, #13, #14 → done citing this milestone's commits; #28's existing open row gets an
ANNOTATION, not a status change, recording the user's checkpoint observation); behavioral
parity evidenced by the harness diff + 2 user visual checkpoints, reported in the
`/speckit-implement` final summary, per V2-A/V2-B's precedent.

**Result**: PASS, no violations to record in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/008-v2-player-bullet-door/
├── plan.md              # This file
├── research.md          # Phase 0 output — R1-R9
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── player-bullet-door-api.md
│   └── zz_player_parity.gd
└── tasks.md             # Phase 2 output (/speckit-tasks — not this command)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── player.rs                    # glue: I<CharacterBody3D>, #[rpc]s, #[func] set_player_id,
│                                 #   OnReady node refs + preloaded bullet_scene: Gd<PackedScene>
├── player/
│   └── model.rs                 # pure: PlayerTuning, InputFrame, AirborneOutcome, AnimPlan,
│                                 #   airborne_step, lerp_motion, flatten_camera_axes,
│                                 #   slerp_toward, walk_target, integrate_root_motion,
│                                 #   should_respawn, anim_plan + #[cfg(test)]
├── hittable.rs                   # NEW, glue: HitTarget, resolve(), rpc_hit() — no tests
├── bullet.rs                     # glue + inline `mod pure { BulletState, step, #[cfg(test)] }`
├── door.rs                       # glue + inline `mod pure { DoorState, on_body, #[cfg(test)] }`
└── lib.rs                        # + `mod hittable;`
```

**Structure Decision**: mirrors V2-B's `<module>.rs` (glue) + `<module>/model.rs` (pure) split
for `player.rs`, whose pure surface is large enough to warrant its own file and its own
review-sized commit (matching `player_input`'s precedent exactly). `bullet.rs` and `door.rs`
keep their pure surfaces (one small enum + one small function each) as an inline `mod pure`
inside the same file, per research.md R8's established V2-B precedent that file boundary
follows the pure surface's size, not a fixed rule — both are smaller than `camera_noise_shake`'s
pure surface (which got its own file in V2-B) and closer to `debug_label`'s (which stayed
inline). `hittable.rs` is a new, small, crate-level GLUE module (uses `Gd<T>` by design,
research.md R5) — not pure, not part of any existing module's file, since it is shared
infrastructure `bullet.rs` introduces and V2-D's `red_robot.rs` will also depend on.

## Commit Plan

One commit per module in dependency order (the pure model before the glue that consumes it;
`hittable.rs` alongside its first consumer), `door.rs` last among the code commits since it is
smallest and has no shared infrastructure to introduce, backlog bookkeeping last.

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `player: extract PlayerTuning, InputFrame, AirborneOutcome, AnimPlan and pure motion/orientation/root-motion math into player/model.rs` | `player/model.rs` (new) | `cargo build && cargo clippy && cargo test` (new tests pass) |
| 2 | `player: InputFrame snapshot (one player_input bind per frame), OnReady/preloaded resources, AnimPlan-driven animate(); closes backlog #10, #11, #12` | `player.rs` | gates + headless (`main.tscn`) |
| 3 | `hittable: crate-level HitTarget dispatch; bullet: BulletState, no double explode (closes backlog #2, #13)` | `hittable.rs` (new), `bullet.rs`, `lib.rs` | gates + headless |
| 4 | `door: DoorState, pure on_body decision (closes backlog #14 in revised form)` | `door.rs` | gates + headless |
| 5 | `docs/v2-backlog.md: close #2, #10, #11, #12, #13, #14 citing this milestone's commits; annotate #28 with the checkpoint observation` | `docs/v2-backlog.md` | none (docs-only) |

Parity-harness commits (throwaway, uncommitted per V2-B's convention — `zz_*` files are never
committed) run after commit 2 (US1 checkpoint) and after commit 4 (US2+US3 checkpoint) — each
followed by a STOP for the user's visual confirmation, exactly as V2-B's two checkpoints did
(V2-B grouped US2+US3 into one checkpoint after both landed; this milestone groups US2+US3 the
same way, checkpointing after `door.rs` rather than after `bullet.rs` alone).

## Complexity Tracking

*No Constitution Check violations — table intentionally empty.*
