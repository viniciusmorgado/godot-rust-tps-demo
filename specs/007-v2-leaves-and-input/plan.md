# Implementation Plan: Milestone V2-B — Leaves and player input

**Branch**: `v2` | **Date**: 2026-09-16 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-v2-leaves-and-input/spec.md`

**Phase**: v2 (Idiomatic Rust), constitution 1.4.0, Principles I, II, III.

## Summary

Remodel the five remaining v1 leaves — `player_input.rs`, `camera_noise_shake.rs`,
`debug_label.rs`, `part_disappear.rs`, `blast.rs` — into the v2 shape: engine-free pure logic
(`AimState`, rotation/aim/fade math, camera-shake decay/offsets, debug-line composition) covered
by `cargo test`, glue trait impls that only snapshot → step → apply, node references resolved
once (`OnEditor`, `OnReady::from_base_fn`) instead of per-frame, and ONE `async`-timer pattern
(`godot::task::spawn` + `to_future`/`to_fallible_future`) replacing every nested `connect_other`
chain, reused verbatim by every later v2 milestone. `player.rs` itself stays v1 shape (V2-C);
only its one call site at `player.rs:138` is edited, in the same commit that changes
`camera_camera`'s type. Behavioral parity against branch `v1` is verified per user story via a
GDScript parity harness on two git worktrees (`XDG_DATA_HOME`-isolated, mirroring V2-A) plus a
user visual checkpoint after each of US1, US2+US3, and US4.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98, per `CLAUDE.md`).

**Primary Dependencies**: `godot = "0.5.5"` (gdext), `experimental-threads` feature only (no
change to `Cargo.toml` in this milestone — no new gdext feature is needed: `godot::task`,
`OnEditor`, `OnReady::from_base_fn`, `RenderingServer::get_rendering_info` are all unconditional
in 0.5.5, confirmed in research.md R1/R2/R6/R7).

**Storage**: N/A (no persisted state touched by these 5 modules; `Settings` — V2-A — is read
typed, not written).

**Testing**: `cargo test` (pure modules, engine-free, no Godot binary needed) + headless Godot
4.7.2 (`/usr/bin/godot.x86_64 --headless ...`, per `CLAUDE.md`) + the GDScript parity harness
(`zz_leaves_parity.tscn`/`.gd`, throwaway, deleted at milestone end) run on a `v1` git worktree
and the `v2` branch with separate `XDG_DATA_HOME`.

**Target Platform**: Linux (dev/CI headless target used for all validation in this repo).

**Project Type**: Godot 4.7 game project + Rust gdext extension crate (`oxide_godot_core/
oxide_godot_lib`, cdylib) — single crate, no workspace topology change.

**Performance Goals**: N/A beyond the constitution's own Principle III goal (fewer FFI
crossings per frame) — no numeric performance target is stated by the spec; SC-002/SC-003 are
structural (grep-verifiable), not a benchmark.

**Constraints**: Behavioral parity with `v1` except the pre-authorized backlog closures (#4,
#5, #7, #8, #9, #26 — see spec top block); `player.rs` outside scope except the one edited call
site; no `.tscn` `node_paths`/`SceneReplicationConfig`/`[connection]` edits required (verified:
none of the 5 scenes touched have `[connection]` blocks; `OnEditor` doesn't change what the
`.tscn` stores — research.md R1).

**Scale/Scope**: 5 source files (234 + 83 + 50 + 34 + 37 = 438 lines today), 1 call site in a
6th file (`player.rs:138`), ≥ 12 new unit tests across 3 pure modules (`player_input/model.rs`,
`camera_noise_shake/model.rs`, `debug_label.rs`'s inline pure `mod`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see "Post-Design
Re-check" below.*

**Principle I (Three-Phase Port)** — v2 block:
- Both pillars per user story: type-system idioms (`AimState`, `CameraCue`,
  `PlayerInputTuning`/`CameraShakeTuning`, `DebugStats`, all typed) AND FFI reduction via
  Principle III (node refs resolved once, one `InputSnapshot`/frame, no per-frame `set_seed`/
  `get_parent().cast()`) — done together per story, not split across stories. PASS (each user
  story's Independent Test in spec.md exercises both).
- Behavioral parity: every story's acceptance scenarios map to a harness case (research.md R9)
  or, where the harness cannot reach it (US1's pure aim/rotation math beyond what a scripted
  input sequence can isolate), to a unit test — plus a user visual checkpoint. The ONLY
  behavior deltas are the 6 backlog items the spec's top block names as closed. PASS.
- Replicated/exported surface preserved: `motion`, `shooting`, `shoot_target`, `aiming` keep
  their name/type (`bool`/`Vector2`/`Vector3`) in `SceneReplicationConfig`; `jumping` closes
  backlog #9 (see Data Model) but the spec records the decision and `player.rs` still reads/
  writes a field of that name. The 6 `node_paths` exports keep their names, only the storage
  type changes (`Option<Gd<T>>` → `OnEditor<Gd<T>>`), which `.tscn` cannot observe (R1). PASS.
- Engine API gaps: none introduced by this milestone (no new `api-gap` comment needed — checked
  against `docs/api-gaps.md`'s existing single entry, `Scaling3DMode::NEAREST`, unrelated).
  PASS (N/A).

**Principle II (Verifiable Port Cycle)**:
- Binding by type: unchanged, no node's registered Rust class or base type changes in this
  milestone (only field/method shapes inside already-Rust classes). PASS (N/A — no new binding).
- Bottom-up / infrastructure-first order: all 5 modules are leaves per `docs/port-order.md`
  (ports 1–5); none is a dependency of another among the five; `player.rs` (their one common
  consumer) is intentionally NOT remodeled here (V2-C) — the spec states this explicitly. PASS.
- Preservation of property names: see Principle I above; re-verified against `player.tscn:39-48`
  (`SceneReplicationConfig`) and `:339-341` (`node_paths`) directly in research.md R1. PASS.
- Residual dynamic access: `player_input.rs`'s `.rpc("jump")` is the only one touched by this
  milestone, and it is the constitution's own named exception (no typed alternative exists for
  RPC dispatch) — listed in spec.md's top block. No new dynamic access is introduced (the async
  pattern research explicitly avoids `.call()`/`has_method` for signal waiting). PASS.
- Mandatory build / headless validation: enforced per commit (Commit Plan below). PASS.

**Principle III (Interface vs. Implementation)** — the milestone's central bet:
- Glue-only trait/API impls: every `I<Base>` callback (`ready`, `process`, `input`) and every
  `#[func]`/`#[rpc]` in the 5 modules reads a snapshot, calls a pure function, writes the
  result — no state machine, no math, no decision tree stays inside a callback body after this
  milestone. PASS, enforced task-by-task at `/speckit-tasks`/`/speckit-implement` time.
- Pure logic engine-free + unit tested: `AimState`/`step_aim` and friends (US1),
  `CameraShakeTuning`/`decay`/`shake`/`offsets` (US2), `DebugStats`/`compose` (US3) — none uses
  `Gd<T>`, singletons, or engine-backed builtins; `Vector2`/`Vector3` only (constitution's named
  exception). US4 has no domain logic to extract (confirmed in spec US4 scenario 5) — its
  compliance is the async-pattern shape itself, not a pure/impure split.
- Snapshot → step → apply: `InputSnapshot` (US1) is the concrete instance the constitution's
  general rule names; US2/US3 read their inputs (trauma, dt, `Os`/`RenderingServer` stats) once
  per frame into locals before computing.
- No per-frame/per-event lookups: `OnEditor` (6 node refs) + `OnReady::from_base_fn` (parent
  handle + RID) eliminate `player_input.rs`'s two per-frame `get_parent().cast()`/`.unwrap()`
  sites (backlog #8); 3 pre-seeded `FastNoiseLite` instances eliminate `camera_noise_shake.rs`'s
  3 per-frame `set_seed()` calls; `blast.rs`'s `camera: Option<Gd<Camera3D>>` was ALREADY
  resolved once in `v1` (confirmed by reading the file) and only changes container type here if
  at all — no regression risk. PASS, this is SC-002/SC-003 verbatim.
- Gates: `cargo build && cargo clippy && cargo test` before every commit (Commit Plan). PASS.
- Tuning constants on a struct: `PlayerInputTuning`, `CameraShakeTuning`, both `Default` — see
  data-model.md. PASS.

**Governance** (v2-specific review criteria, self-checked ahead of the formal review): backlog
bookkeeping happens in the milestone's final commit (`docs/v2-backlog.md` rows #4, #5, #7, #8,
#9, #26 → done citing this milestone's commits; #6 stays open with the deferral reason already
in spec.md); behavioral parity evidenced by the harness diff + 3 user visual checkpoints,
reported in the `/speckit-implement` final summary, per V2-A's precedent.

**Result**: PASS, no violations to record in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/007-v2-leaves-and-input/
├── plan.md              # This file
├── research.md          # Phase 0 output — R1-R10
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── leaves-api.md
│   └── zz_leaves_parity.gd
└── tasks.md             # Phase 2 output (/speckit-tasks — not this command)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── player_input.rs              # glue: I<CharacterBody3D-sibling base>, #[rpc] jump, #[export]/OnEditor fields
├── player_input/
│   └── model.rs                 # pure: AimState, CameraCue, PlayerInputTuning, InputSnapshot,
│                                 #   step_aim, scaled_look, scaled_mouse_look, clamp_pitch,
│                                 #   alpha_for_height, aim_rotation + #[cfg(test)]
├── camera_noise_shake.rs         # glue: ready (3× FastNoiseLite seeded once), apply_shake
├── camera_noise_shake/
│   └── model.rs                  # pure: CameraShakeTuning, decay, shake, offsets + #[cfg(test)]
├── debug_label.rs                # glue: process (visibility + stat reads) + inline `mod pure`
│                                 #   { DebugStats, compose } + nested #[cfg(test)]
├── part_disappear.rs             # glue only: one async fn spawned from ready/a trigger
├── blast.rs                      # glue only: one async fn spawned from ready
└── player.rs                     # UNCHANGED except line 138 (camera_camera .unwrap() removal)
```

**Structure Decision**: mirrors V2-A's `settings.rs` + `settings/graphics.rs` split, generalized
to the naming convention `<module>.rs` (glue) + `<module>/model.rs` (pure), used for the two
modules whose pure surface is more than a few lines (`player_input`, `camera_noise_shake`).
`debug_label.rs` keeps its pure surface as an inline `mod pure { ... }` inside the same file
(research.md R8: one struct + one function does not justify a separate file — Principle III
requires a distinct, engine-free `mod`, not a distinct file). `part_disappear.rs`/`blast.rs`
stay single-file, unchanged in shape, since neither has domain logic to separate (spec US4
scenario 5) — their v2-ness is entirely the async-pattern rewrite of their glue.

## Commit Plan

One commit per module in leaf-independence order (no module here depends on another), `US1`
split into pure-model then glue (matching V2-A's precedent), the `player.rs:138` edit riding in
the `player_input` glue commit (the commit that actually changes what `player.rs` sees), the
`CLAUDE.md` paragraph riding in the first US4 commit, backlog bookkeeping last.

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `player_input: extract AimState and pure rotation/aim/fade math into player_input/model.rs` | `player_input/model.rs` (new) | `cargo build && cargo clippy && cargo test` (new tests pass) |
| 2 | `player_input: OnEditor node refs, OnReady parent handle, snapshot/step/apply frame shape; player.rs:138 camera_camera unwrap removal` | `player_input.rs`, `player.rs` | gates + headless (`main.tscn` or a player-bearing scene) |
| 3 | `camera_noise_shake: extract CameraShakeTuning/decay/shake/offsets into camera_noise_shake/model.rs; 3 pre-seeded FastNoiseLite instances` | `camera_noise_shake/model.rs` (new), `camera_noise_shake.rs` | gates + headless |
| 4 | `debug_label: DebugStats/compose pure mod, VRAM line, hidden-frame skip (closes backlog #4, #26)` | `debug_label.rs` | gates + headless |
| 5 | `part_disappear + blast: async task pattern replaces nested connect_other (closes backlog #5); document the pattern in CLAUDE.md` | `part_disappear.rs`, `blast.rs`, `CLAUDE.md` | gates + headless |
| 6 | `docs/v2-backlog.md: close #4, #5, #7, #8, #9, #26 citing this milestone's commits; #6 stays open` | `docs/v2-backlog.md` | none (docs-only) |

Parity-harness commits (throwaway, uncommitted per spec's own convention — `zz_*` files are
never committed, matching V2-A) run after commit 2 (US1 checkpoint), after commit 4 (US2+US3
checkpoint), and after commit 5 (US4 checkpoint) — each followed by a STOP for the user's visual
confirmation, exactly as V2-A's three `/speckit-implement` sessions did.

## Complexity Tracking

*No Constitution Check violations — table intentionally empty.*
