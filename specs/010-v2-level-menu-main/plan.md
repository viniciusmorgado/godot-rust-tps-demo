# Implementation Plan: Milestone V2-E — level, menu, main: the scene manager and the end of v2

**Branch**: `v2` | **Date**: 2026-09-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/010-v2-level-menu-main/spec.md`

**Phase**: v2 (Idiomatic Rust), constitution 1.4.1, Principles I, II, III.

## Summary

Remodel the last four v1-shaped modules — `main_scene.rs` (the constitution's own named goal,
"a centralized signal-driven scene manager"), `level.rs` + `flying_forklift.rs` (GI setup,
spawning, the respawn hitch), and `menu.rs` (the largest remaining `if`/`else if` surface) —
into the v2 shape every prior milestone established. The scene manager becomes typed
`try_cast` + `connect_other` dispatch, replacing `has_signal` + `Callable::
from_object_method`; the crate's last two by-name deferred calls become
`Callable::from_fn(...).call_deferred(&[])` (a GENERATED engine method confirmed to share
`Object::call_deferred`'s exact same underlying queue — zero timing delta, not an
approximation); `level.rs`'s three near-identical GI setup functions become one pure `gi_plan`
(with a discovered third input, `has_lightmap`, and a documented visibility asymmetry
`v1` itself never resolves); two previously-uncatalogued redundant `randomize()`/
`call_deferred("name")` instances (in `level.rs` and `menu.rs` respectively) are closed
alongside their originally-catalogued siblings; and `menu.rs`'s ~250 lines of settings
`if`/`else if` become 15 small pure row-mappings, one of which (`resolution_scale`) required
discovering that `godot::global::is_equal_approx` is itself engine-backed and must be
reimplemented pure. After this milestone, the ONLY by-name dynamic access anywhere in the
crate is `.rpc("name")` — v2 is complete.

## Technical Context

**Language/Version**: Rust (stable toolchain, cargo/rustc 1.98, per `CLAUDE.md`).

**Primary Dependencies**: `godot = "0.5.5"` (gdext), `experimental-threads` feature only — no
`Cargo.toml` change. `Callable::from_fn` (hand-written, `builtin/callable.rs:152-158`) and the
GENERATED `Callable::call_deferred` (`out/builtin_classes/callable.rs:198-206`) are both
unconditional in 0.5.5, confirmed in research.md R1.

**Storage**: N/A (no persisted state beyond `Settings`, unchanged, V2-A).

**Testing**: `cargo test` (pure modules, engine-free) + headless Godot 4.7.2 (`--headless`, per
`CLAUDE.md`, extended this milestone to an END-TO-END `main.tscn` boot, the scene manager's own
best validation) + FOUR parity harness cases (one reusing V2-A's existing settings harness
unchanged, three new) run on a `v1` git worktree and the `v2` branch with separate
`XDG_DATA_HOME` and `--fixed-fps 60` (established V2-A through V2-D pattern), one of which
additionally requires `seed()` (established V2-D pattern, since this milestone's `pick_spawn`/
`pick_model` are RNG-driven).

**Target Platform**: Linux (dev/CI headless target used for all validation in this repo).

**Project Type**: Godot 4.7 game project + Rust gdext extension crate (`oxide_godot_core/
oxide_godot_lib`, cdylib) — single crate, no workspace topology change, no new crate module
(two new pure submodules under existing modules: `level/model.rs`, `menu/model.rs`).

**Performance Goals**: N/A beyond Principle III's own goal (fewer FFI crossings: per-spawn
`load`/`get_node_as` resolved once, three duplicated GI setup functions collapsed to one pure
decision) — no numeric performance target is stated by the spec; SC-002 through SC-005 are
structural (line counts, greps), not a benchmark.

**Constraints**: Behavioral parity with `v1` except the pre-authorized backlog closures (#3,
#19 + its `level.rs` extension, #20, #21, #22, #23, #24; #32 conditionally) — see spec top
block; `player.rs`/`bullet.rs`/`door.rs`/`hittable.rs`/`red_robot.rs`/`part.rs`/`settings.rs`/
`settings/graphics.rs` are read-only references, none edited by this milestone; no `.tscn` edit
anywhere (`main.tscn`'s zero connections, `menu.tscn`'s 10, `level.tscn`'s child names, all
verified unaffected by a name-preserving Rust remodel — research.md's Context cross-checks).

**Scale/Scope**: 4 source files (74 + 210 + 593 + 37 = 914 lines today) plus 2 new pure
submodule files (`level/model.rs`, `menu/model.rs`), 0 edits to any other module, ≥ 20 new unit
tests across 3 pure surfaces (`level/model.rs`, `flying_forklift.rs`'s inline `mod pure`,
`menu/model.rs`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see "Post-Design
Re-check" below.*

**Principle I (Three-Phase Port)** — v2 block:
- Both pillars per user story: type-system idioms (`GiPlan`, `pick_spawn`, `pick_model`, 15
  option-row enums, `LoadingCmd`, all typed; typed `try_cast` scene dispatch replacing dynamic
  probing) AND FFI reduction via Principle III (preloaded scene fields instead of per-spawn
  `load`, `OnReady` instead of repeated `get_node_as`, ONE `gi_plan` instead of three
  near-duplicated functions, `Callable::call_deferred` instead of by-name `call_deferred`) —
  done together per story (each Independent Test in spec.md exercises both). PASS.
- Behavioral parity: every acceptance scenario maps to a harness case (research.md R8) or a
  unit test, plus a user visual checkpoint. The ONLY behavior deltas are the backlog items the
  spec's top block names as closed/conditionally-closed, ALL pre-authorized and all
  observable-only-under-a-seeded-harness (the two `randomize()` removals) or fully
  unobservable (the `call_deferred`→`Callable::call_deferred` swap, confirmed zero timing
  delta by research.md R1's source citation, not an assumption). PASS.
- Replicated/exported surface preserved: N/A for `main_scene.rs`/`level.rs`/
  `flying_forklift.rs` (none of the three has an `#[export]`/replicated field); `menu.rs` has
  none either (its ~50 `OnReady` fields are all private scene-structure references, never
  exported/replicated) — confirmed by re-reading all four files in full. PASS (N/A).
- Engine API gaps: none introduced (checked against `docs/api-gaps.md`'s existing single
  entry, `Scaling3DMode::NEAREST`, unrelated — `menu.rs`'s `scale_filter` mapping this
  milestone touches uses the ALREADY-TYPED `ScaleFilter` enum, never the raw gap workaround
  itself, which lives in `settings/graphics.rs`, untouched). PASS (N/A).

**Principle II (Verifiable Port Cycle)**:
- Binding by type: unchanged, no node's registered Rust class or base type changes. PASS (N/A).
- Bottom-up / infrastructure-first order: `main_scene.rs`/`level.rs`/`menu.rs`/
  `flying_forklift.rs` are the LAST modules — every module they depend on (`Settings`,
  `Player`, `EnemyRobot`) is already v2-shaped from V2-A through V2-D. PASS.
- Preservation of property names: N/A (no exported/replicated properties in any of the four
  modules, confirmed above); every `[connection]`/node-name reference re-verified directly
  against the four `.tscn` files in research.md's Context section. PASS.
- Residual dynamic access: after this milestone, ONLY `.rpc("name")` sites remain crate-wide
  (contracts/end-of-v2-api.md's "Crate-wide residual" table, 7 sites, all inherited unchanged
  from V2-C/V2-D) — the constitution's own named exception (no typed alternative exists for
  RPC dispatch). `has_signal`, `Callable::from_object_method`, and every by-name
  `call_deferred("...")` (a FORBIDDEN dynamic-access form, not a residual exception) are
  REMOVED, not introduced. No new dynamic access is added anywhere. PASS.
- Mandatory build / headless validation: enforced per commit (Commit Plan below), extended to
  an END-TO-END `main.tscn` boot for this milestone specifically (the scene manager's own best
  test). PASS.

**Principle III (Interface vs. Implementation)** — the milestone that closes v2 out:
- Glue-only trait/API impls: every `I<Base>` callback and every `#[func]`/`#[signal]` in the 4
  modules reads a snapshot, calls a pure function (where one exists), writes the result — no
  domain decision stays inside a callback body after this milestone. `main_scene.rs`
  specifically is confirmed to have NO domain logic beyond typed dispatch (spec Acceptance
  Scenario 4), so it correctly stays 100% glue per Principle III's own allowance ("pure
  separation only where domain logic exists"). PASS, enforced task-by-task at
  `/speckit-tasks`/`/speckit-implement` time.
- Pure logic engine-free + unit tested: `level/model.rs`, `flying_forklift.rs`'s inline
  `mod pure`, `menu/model.rs` — none uses `Gd<T>`, singletons, or engine-backed builtins; EVERY
  builtin/utility function this milestone's pure code would have used is verified against the
  1.4.1 rule — most critically, `godot::global::is_equal_approx` is confirmed ENGINE-BACKED
  (research.md R6, `utilities.rs:412-419`, dispatches via `sys::utility_function_table()`) and
  is explicitly NOT called from the pure `resolution_scale_button` function; it is reimplemented
  in pure Rust instead, with an empirical implementation-time cross-check against the real
  engine function specified as a required verification step (not skipped). PASS.
- Snapshot → step → apply: `gi_plan` (US2), the option table (US3), `loading_step` (US3) are
  all a snapshot-in/pure-decision/apply-once shape, the fourth-through-sixth concrete
  instances of the pattern established since V2-B.
- No per-frame/per-event lookups: preloaded `robot_scene`/`player_scene` fields (were per-spawn
  `load`), `voxel_gi`/`reflection_probes` `OnReady` fields (were per-`setup_*`-call
  `get_node_as`). SC-003 is this fact, verbatim.
- Gates: `cargo build && cargo clippy && cargo test` before every commit (Commit Plan). PASS.
- Tuning constants: N/A — no module introduces a new tunable constant this milestone (the GI
  quality/ray-count/voxel-quality values are already-typed engine enums with no numeric
  literal of this project's own choosing to extract). PASS (N/A).

**Governance** (v2-specific review criteria, self-checked ahead of the formal review): backlog
bookkeeping happens in the milestone's final commit (`docs/v2-backlog.md` rows #3, #19, #20,
#21, #22, #23, #24 → done citing this milestone's commits; #32 → done or annotated per the
checkpoint; #6, #25, #29, #30, #31 → each gains a one-line "post-v2 review" note; a header note
"v2 complete on `<date>`"); `README.md`'s Versions table v2 `Status` cell updated to
"**Complete**" with a summary paragraph, and its stale "26 items" backlog count corrected;
behavioral parity evidenced by four harness diffs + 2 user visual checkpoints, reported in the
`/speckit-implement` final summary, per V2-A through V2-D's precedent. This is the LAST v2
milestone — the formal review this constitution names should treat v2 as closed once this
milestone's final commit lands.

**Result**: PASS, no violations to record in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/010-v2-level-menu-main/
├── plan.md              # This file
├── research.md          # Phase 0 output — R1-R9
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── end-of-v2-api.md
│   └── zz_end_parity.gd
└── tasks.md             # Phase 2 output (/speckit-tasks — not this command)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── main_scene.rs                 # glue only: typed try_cast + connect_other dispatch,
│                                  #   Callable::call_deferred; no pure submodule
├── level.rs                       # glue: I<Node3D>, #[signal] quit, OnReady GI nodes +
│                                  #   preloaded robot_scene/player_scene
├── level/
│   └── model.rs                   # pure: GiPlan, gi_plan, pick_spawn, + #[cfg(test)]
├── flying_forklift.rs             # glue + inline `mod pure { pick_model, #[cfg(test)] }`
├── menu.rs                        # glue: I<Node>, 10 #[func]s (unchanged names), ~50
│                                  #   OnReady fields (unchanged)
└── menu/
    └── model.rs                   # pure: 15 option-row functions, LoadingCmd,
                                    #   loading_step, + #[cfg(test)]
```

No `lib.rs` edit needed beyond what already exists (`mod main_scene;`/`mod level;`/`mod menu;`/
`mod flying_forklift;` already present; `level/model.rs` and `menu/model.rs` are private
submodules of their existing parent modules, exactly like every prior milestone's `<module>/
model.rs`).

**Structure Decision**: mirrors every prior milestone's `<module>.rs` (glue) + `<module>/
model.rs` (pure) split for `level.rs` and `menu.rs`, whose pure surfaces (a 9-cell decision
table; 15 settings rows plus a loading-status decision) each warrant their own file and
review-sized commit. `flying_forklift.rs` keeps its pure surface (one function) as an inline
`mod pure`, matching `bullet.rs`'s/`door.rs`'s V2-C precedent. `main_scene.rs` gets NO pure
submodule at all — the first v2 module for which this is true — since Principle III's
separation requirement is explicitly conditional on domain logic existing, and this module has
none beyond type-safe dispatch.

## Commit Plan

One commit per module, `level.rs`/`flying_forklift.rs` sequential (both close backlog #19's
two halves but are otherwise independent — kept as separate commits for review size), docs
last.

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `main_scene: typed try_cast + connect_other scene dispatch, Callable::call_deferred instead of call_deferred("name"); closes backlog #3, #23, #24` | `main_scene.rs` | `cargo build && cargo clippy && cargo test`; headless (`main.tscn` end-to-end boot) |
| 2 | `level: extract GiPlan, pick_spawn into level/model.rs; preloaded scenes, OnReady GI nodes, uniform add_child, async respawn, redundant randomize() removed; closes backlog #19 (level half), #20, #21, #32 (load half)` | `level/model.rs` (new), `level.rs` | gates + headless (`level.tscn`) |
| 3 | `flying_forklift: pick_model, redundant randomize() removed; closes backlog #19 (forklift half)` | `flying_forklift.rs` | gates + headless |
| 4 | `menu: declarative option table + loading_step in menu/model.rs, Callable::call_deferred for headless auto-host; closes backlog #22` | `menu/model.rs` (new), `menu.rs` | gates + headless (`menu.tscn`, auto-hosted `main.tscn`) |
| 5 | `docs: close backlog #3, #19, #20, #21, #22, #23, #24 citing this milestone's commits; #32 per the checkpoint observation; #6/#25/#29/#30/#31 post-v2 review notes; header "v2 complete on <date>"; README.md v2 Status -> Complete + summary` | `docs/v2-backlog.md`, `README.md` | none (docs-only) |

Parity-harness runs (throwaway, uncommitted per every prior milestone's convention — `zz_*`
files are never committed) happen after commit 3 (US1+US2 checkpoint) and after commit 4 (US3
checkpoint) — each followed by a STOP for the user's visual confirmation.

## Complexity Tracking

*No Constitution Check violations — table intentionally empty.*
