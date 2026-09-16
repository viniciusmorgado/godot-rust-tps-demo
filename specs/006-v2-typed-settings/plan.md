# Implementation Plan: Milestone V2-A — Typed `Settings` and its consumers

**Branch**: `v2` (worked directly, per constitution 1.4.0 Principle II's "infrastructure first"
order — no per-milestone feature branch; local commits only, never pushed) | **Date**: 2026-09-16
| **Spec**: [specs/006-v2-typed-settings/spec.md](./spec.md)

**Input**: Feature specification from `specs/006-v2-typed-settings/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Remodel `settings.rs` (188 lines) so its 15 graphics options are parsed once from
`user://settings.ini` into a typed, `Copy` model (`GraphicsSettings` + 5 project enums), with the
`apply_graphics_settings` decision logic split into a pure `plan()` (unit-tested, no engine) and
a thin `apply()` glue step, per constitution Principle III. Move the 5 consumers
(`flying_forklift`, `bullet`, `level`, `menu`, `main_scene`) off dynamic `/root/Settings` access
onto a typed `Gd<Settings>` resolved once per consumer (`godot::tools::get_autoload_by_name`),
closing `docs/v2-backlog.md` item #1. Bring `cargo build`/`clippy`/`test` to green, catalogue the
`Scaling3DMode::NEAREST` engine-API gap, and update `CLAUDE.md`/`docs/v2-backlog.md`/`README.md`.
Behavioral parity with `v1` is mandatory and is verified headless plus with a scratch-scene
parity harness run against a separate `v1` worktree; the one sanctioned deviation is that an
out-of-range enum code in a hand-edited `settings.ini` now defaults with a logged warning instead
of silently falling into whichever `else` branch matched in `v1`.

Full research decisions (engine-enum parsing semantics, the `NEAREST` gap mechanism, the
engine-free wire representation, the `plan`/`apply` split, the typed shadow walk, the consumer
access pattern, the US1↔US2 transitional contract, module layout, the 9 clippy fixes, and the
parity harness) are in [research.md](./research.md); the resulting types are in
[data-model.md](./data-model.md); public signatures are in
[contracts/settings-api.md](./contracts/settings-api.md).

## Technical Context

**Language/Version**: Rust 1.98.1, edition 2024 (`oxide_godot_lib/Cargo.toml`)

**Primary Dependencies**: `godot` (gdext) 0.5.5 with the `experimental-threads` feature (workspace
dependency, unchanged by this milestone); target engine Godot 4.7.2 stable, prebuilt API surface
4.6 (source of the `Scaling3DMode::NEAREST` gap, R2)

**Storage**: `user://settings.ini` (Godot `ConfigFile`, plain-text INI) — the only persisted state
this milestone touches; format, section/key order and per-key Variant types are preserved exactly

**Testing**: `cargo test` for the new pure `graphics` module (no Godot binary, ≥ 8 unit tests);
headless Godot (`--headless --import`, `--headless --path . <scene>.tscn`) for `main.tscn` /
`menu.tscn` / `level.tscn`; a scratch GDScript parity harness (`contracts/zz_settings_parity.gd`
skeleton) run against a separate `v1` git worktree and diffed

**Target Platform**: Linux desktop (the project's only validated platform, per `CLAUDE.md`)

**Project Type**: Godot 4 game — single-crate gdext `cdylib` extension (`oxide_godot_core`
workspace, member `oxide_godot_lib`)

**Performance Goals**: none newly introduced; the FFI-crossing reduction (Principle I pillar 2)
is a code-quality/idiom goal for this milestone, not a measured throughput/latency target

**Constraints**: `settings.ini` byte-identical to `v1` for defaults and every menu option
(SC-003); zero dynamic `/root/Settings` access left in the crate after User Story 2 (SC-001);
`cargo build`/`clippy`/`test` all clean (SC-002); no crate API-feature change (`api-custom`,
`api-4-x`); local commits only, never pushed

**Scale/Scope**: one autoload class (`settings.rs`, growing into `settings.rs` + `settings/graphics.rs`)
and 5 consumer modules touched; no new scenes, nodes, or `.tscn` structural changes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Evaluated against constitution **1.4.0**. All three principles gate this plan (it is a v2
milestone); no v1 or v3 rule applies.

| Gate | Rule | Status | Evidence |
|---|---|---|---|
| **Principle I — two pillars, done together** | Idiomatic type-system usage (enums, parse-don't-validate) AND FFI reduction via Principle III both land by the end of the *spec* | **PASS** | `GraphicsSettings` + 5 project enums replace raw `ConfigFile` string/int lookups (data-model.md); `plan()`/`apply()` split moves domain logic to pure Rust (contracts/settings-api.md). US1 alone keeps the `#[var] config_file`/`#[func]` compatibility surface — an explicitly declared, spec-sanctioned intermediate step (spec.md top block, "Transitional contract"; research.md R7), not a milestone-final state; both pillars hold once User Story 2's last commit lands. |
| **Principle I — behavioral parity** | Every user story's observable effect matches `v1` except backlog-listed deviations | **PASS** | Parity verified by headless + the R10 harness against a `v1` worktree; the ONE deviation (malformed wire value → default + warning) is declared in the spec and added to `docs/v2-backlog.md` as a new, closed item (FR-009, US3 scenario 4). |
| **Principle I — replicated/exported surface preserved** | `#[export]`/`#[var]` names referenced by scenes keep name + compatible type | **PASS** (vacuous) | `Settings` has zero `#[export]` properties and is referenced by no `.tscn` beyond its own autoload scene (`menu/settings.tscn`, a bare root node with no properties set) — confirmed by the R8 grep (`GI_TYPE_*`/`GI_QUALITY_*`/`metalfx_supported`/`config_file`: zero `.tscn`/`.gd` references). Nothing to preserve; nothing at risk. |
| **Principle I — engine API gap protocol** | Isolated behind one typed value, `// api-gap(...)` comment, `docs/api-gaps.md` entry, no feature-set change | **PASS** | `ScaleFilter::to_engine()` is the one spot (R2); `docs/api-gaps.md` entry content is pinned in research.md; `api-custom`/`api-4-x` explicitly not touched (FR-018). |
| **Principle I — backlog consumed** | Spec states which items it closes/defers; new items in one-entry form | **PASS** | Spec top block: closes #1 + 1 new item; defers #22/#23/#25 with target milestones (already present in spec.md, carried unchanged into this plan). |
| **Principle II — typed access mandatory, dynamic access forbidden except residual cases** | No `get_node_as`/`.call`/`.get` by name in v2 code outside listed residual cases | **PASS**, milestone-final | Zero after US2's last commit (SC-001, grep in quickstart.md §5). Residual cases NOT touched by this milestone (`bullet.rs` `.rpc`/`has_method`, `menu.rs`/`main_scene.rs` `call_deferred`/`has_signal`) are the spec's own pre-declared list, unchanged by this plan — out of scope here, assigned to V2-C/V2-E. |
| **Principle II — infrastructure-first order** | A module whose typed API others consume is remodeled before its consumers | **PASS** | This is the textbook case the rule names (`Settings`, root of 5 consumers) — US1 (the model) strictly precedes US2 (the consumers) in the commit plan below. |
| **Principle II — exported/replicated property names preserved** | Checked against affected `.tscn`s | **PASS** (vacuous, see above) | Same grep as the API-gap/surface checks; `Settings` carries no scene-visible property today and none is added. |
| **Principle II — class-name collision** | New/renamed class checked against engine classes and remaining `.gd` top-level identifiers | **N/A** | No class is renamed or added; `Settings` keeps its name. `GraphicsSettings`, `ScaleFilter`, etc. are plain Rust types, not registered `GodotClass`es, so the collision rule (which guards GDScript's `type=` resolution) does not apply to them. |
| **Principle III — glue-only trait/API impls** | `impl INode for Settings` and `#[godot_api] impl Settings` are thin, delegate, no domain logic | **PASS** | `init`/`ready`/`input` delegate to `graphics`/`plan`/`apply` (data-model.md, "Final shape"); the kept `#[func]`s during US1 delegate too (contracts/settings-api.md, "Transitional shape") — thin by the letter of the rule even while transitional. |
| **Principle III — pure logic, engine-free, unit-tested** | No `Gd<T>`/singleton/Variant-family in pure code; `cargo test` green | **PASS** | `GraphicsSettings`, the 5 enums, `WireValue`, `plan()` use only plain Rust + gdext engine-enum newtypes, which are verified engine-free (R1: no `unsafe`, no live engine needed to construct/compare — same test the constitution applies to its named math-builtin example). ≥ 8 tests specified (contracts/settings-api.md). Flagged as an explicit reading of an implicit gap (engine enums aren't named alongside math builtins in the constitution's text) for reviewer sign-off, not as a violation. |
| **Principle III — resolve once per frame/event** | No `get_node_as`/`load(...)` inside `process`/`physics_process` or per-event handlers | **PASS** | Each consumer resolves `Gd<Settings>` once via `OnReady::new(get_autoload_by_name)` (R6); `bullet.rs` resolves at its own `ready()`, not inside `explode()` (spec Edge Cases, FR-010). The `Light3D` walk (R5) runs once per `apply_graphics_settings` call (a per-event handler itself), not per-frame. |
| **Principle III — three gates** | `cargo build` + `cargo clippy` (no warnings) + `cargo test` green before validation/commit | **PASS** | The 9 pre-existing clippy warnings are fixed in commit 0, BEFORE any other v2 code commit (R9), so the gate holds literally for every commit of the milestone; US1 adds the tests (R4); quickstart.md §1 is the exact command sequence, run before every commit. |
| **Principle III — tuning constants** | Belong to the type or a `Default` tuning struct, not bare module `const` | **PASS** | `CONFIG_FILE_PATH` (a path) is the only module `const` kept; the SSAO/SSIL call literals (`0.5, 2, 50.0, 300.0`) are left as literal `apply()` arguments with an explicit rationale (research.md R4 — not tunable by anything in this milestone's scope, promoting them to a struct nobody varies yet would be premature abstraction). |
| **Governance — compliance review readiness** | Reviewable evidence for parity, Principle III, backlog closure | **PASS** | This plan + research.md + data-model.md give the reviewer the parity method, the glue/pure split, and the exact backlog bookkeeping up front. |

**No gate failures.** The two items marked "transitional"/"explicit reading" above are called out
for reviewer visibility, not because they fail a rule — see Complexity Tracking below for why
neither warrants an entry there.

## Project Structure

### Documentation (this feature)

```text
specs/006-v2-typed-settings/
├── plan.md                          # This file (/speckit-plan command output)
├── research.md                      # Phase 0 output — R1-R10 decisions
├── data-model.md                    # Phase 1 output — GraphicsSettings, enums, ApplyPlan
├── quickstart.md                    # Phase 1 output — gates, headless, worktree, harness, greps
├── contracts/
│   ├── settings-api.md              # Public Rust signatures, transitional + final shape
│   └── zz_settings_parity.gd        # Parity harness skeleton (research.md R10)
├── checklists/requirements.md       # From /speckit-specify
└── tasks.md                         # Phase 2 output (/speckit-tasks — NOT created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/
└── oxide_godot_lib/
    └── src/
        ├── settings.rs              # MODIFIED — glue only from here on (see below)
        ├── settings/
        │   └── graphics.rs          # NEW — GraphicsSettings, 5 enums, WireValue, plan(), #[cfg(test)] mod tests
        ├── flying_forklift.rs       # MODIFIED — access path only (US2)
        ├── bullet.rs                # MODIFIED — access path only, resolved at ready not in explode (US2)
        ├── level.rs                 # MODIFIED — access path + GI match (US2)
        ├── menu.rs                  # MODIFIED — access path + SCALING_3D_MODE_NEAREST const removed (US2)
        └── main_scene.rs            # MODIFIED — access path only (US2)

oxide-godot/                          # Godot project — NOT touched structurally
                                       # (no .tscn edited: Settings has no exported/replicated
                                       # property and menu/settings.tscn's binding is unchanged)

docs/
├── api-gaps.md                      # NEW — Scaling3DMode::NEAREST entry (US3)
└── v2-backlog.md                    # MODIFIED — #1 + new malformed-values item marked done (US3)

CLAUDE.md                            # MODIFIED — cargo clippy/test, Principle III layout note,
                                      # api-gap convention, v1-worktree parity note (US3)
README.md                            # MODIFIED — Versions table, v2 row → "In progress" (US3)
```

**Structure Decision**: single-crate Godot extension (no web/mobile/multi-project structure
applies). All Rust changes stay inside `oxide_godot_lib/src/`; the only new file is the
`settings/graphics.rs` submodule housing the pure model (Principle III's mandated separation).
No `.tscn`/`.gd`/scene file changes are needed — confirmed by the R8 grep, since `Settings`
carries no scene-visible property and its autoload binding (`menu/settings.tscn` +
`project.godot:25`) was already established in `v1` (`specs/005-v1-settings-autoload`) and is
untouched here.

## Commit Plan

One local commit per unit of work (FR-024), each message naming what was remodeled, never pushed:

| # | User Story | Commit | Scope |
|---|---|---|---|
| 0 | Setup (US3 task, run FIRST) | `Fix 9 clippy warnings crate-wide (no behavior change)` | the R9 table, one crate-wide pass — first so Principle III's clippy gate holds for every commit that follows |
| 1 | US1 | `Remodel settings.rs: typed GraphicsSettings model + plan/apply split` | `settings.rs` (glue) + `settings/graphics.rs` (new) — model, 5 enums, wire conversion, `plan()`, unit tests; `#[var] config_file`/3× `#[func]` kept (transitional); `GI_TYPE_*`/`GI_QUALITY_*`/`metalfx_supported` var removed |
| 2 | US2 | `flying_forklift.rs: typed Settings access` | access path only |
| 3 | US2 | `bullet.rs: typed Settings access` | access path only, resolved at `ready` |
| 4 | US2 | `main_scene.rs: typed Settings access` | access path only |
| 5 | US2 | `level.rs: typed Settings access + GI match` | access path + `match` on `GiType`/`GiQuality`, 5 local consts removed |
| 6 | US2 | `menu.rs: typed Settings access, drop SCALING_3D_MODE_NEAREST const` | access path; local scaling const removed in favor of `ScaleFilter::Nearest` |
| 7 | US2 (last) | `settings.rs: remove compatibility surface, parse once at ready` | drop `#[var] config_file` + 3× `#[func]`; `ready` becomes the single parse point; grep (quickstart.md §5) goes to zero |
| 9 | US3 | `docs/api-gaps.md + CLAUDE.md: catalogue Scaling3DMode::NEAREST, document v2 gates/layout` | new file + operational-guide update |
| 10 | US3 | `docs/v2-backlog.md + README.md: close #1 and the malformed-values item, v2 → In progress` | backlog bookkeeping + Versions table |

Headless validation (quickstart.md §2) and the relevant gate commands (§1) run after every
commit above; the parity harness (§4, separate `XDG_DATA_HOME` per branch) runs at minimum after commit 1 (US1
checkpoint), after commit 7 (US2 checkpoint), and once more after commit 10 (final).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No entries. The Constitution Check above found no gate failures. The two items flagged there for
reviewer visibility are not violations:
- **US1's transitional compatibility surface** is an explicit, spec-declared intermediate step
  (spec.md top block; constitution Principle I's "an intermediate step ... may refactor with the
  logic still in the glue code" clause), scoped to resolve by User Story 2's last commit — not a
  permanent deviation needing justification.
- **Engine enums used inside pure logic (`ApplyPlan`, `GraphicsSettings`)** are a reading of an
  implicit gap in Principle III's text (which names math builtins as its example of "pure Rust
  that needs no engine" but does not explicitly list engine enums), backed by source-level
  evidence in research.md R1 that they satisfy the same test. It is a clarification applied, not
  a rule broken — nothing here trades a constitutional requirement for convenience.
