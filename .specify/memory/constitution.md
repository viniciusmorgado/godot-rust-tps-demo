<!--
SYNC IMPACT REPORT

Version change: 1.5.1 → 1.5.2 (PATCH — Principle I, v3 block, "Scope" bullet corrected against
its own criterion: `flying_forklift` moves from the list of modules rewritten in ECS to the list
of modules that stay as v2 code. The bullet itself declares that moving a module between the
lists is a PATCH amendment. No rule added, removed or redefined.)

Principles:
  I. Three-Phase Port — v3 "Scope" bullet: the ECS-rewritten list becomes the nine modules
     `player`, `player_input`, `camera_noise_shake`, `bullet`, `part`, `part_disappear`, `blast`,
     `red_robot`, `door`; `flying_forklift` joins the stay-as-v2 list with the reason "(one-shot
     `ready`: picks a model with `randf()` and applies the shadow setting; no per-tick state —
     fails the bullet's own criterion)".
  II. Verifiable Port Cycle — unchanged.
  III. Interface vs. Implementation (including "ECS shape (v3)") — unchanged.

Motivating case: V3-C scoping on 2026-09-18. `flying_forklift.rs:46-63` is the class's whole
`ICharacterBody3D` impl and its only callback is `ready` (`:48`): it applies the shadow setting
to its spot light and picks one of three models with `randf()`; the class has no `process`,
no `physics_process`, no signal or RPC handler and no per-tick state, so there is nothing a
system could operate on. The bullet's own criterion ("a module enters the ECS when it has
per-tick state that systems can operate on together") excludes it.

Sections: none changed (Governance, versioning policy, Ratified date unchanged; Last Amended
stays 2026-09-18 — same day as 1.5.0 and 1.5.1).

Templates: no template edit needed (the plan template's Constitution Check resolves the
principles dynamically).

Follow-ups outside this command: none. Deferred TODOs: none.
-->

# Oxide Godot Constitution

## Core Principles

### I. Three-Phase Port (v1 → v2 → v3)

This project is the port of the Godot TPS Demo (GDScript) to Rust via godot-rust/gdext, and it evolves in three sequential phases, each with a distinct, non-negotiable goal:

**v1 — Raw Port (branch `main`)**
- Goal: make the game run ENTIRELY in Rust, with behavior identical to the GDScript original, script by script.
- The code MUST be a direct translation of the GDScript. Explicitly FORBIDDEN in this phase: remodeling nodes, creating a scene manager, redesigning structs/enums, introducing abstractions, optimizing performance or "idiomatizing" the Rust.
- Functional but non-idiomatic code ("Rust that looks like GDScript", as happens in C++ → Rust ports) is the EXPECTED and accepted result. Improvements noticed during the port must be noted for v2, never applied in v1.
- v1 will NOT serve as a template or reference for future projects. It is preserved for historical purposes, consultation, benchmark baseline and as an example of a raw port.
- Completion criterion: no `.gd` script left in the project and the game playable end to end.

**v2 — Idiomatic Rust (branch `v2`, created from the point where v1 is complete)**
- Goal: remodel structs, enums and other constructs taking real advantage of Rust's type system and language features, improving performance and readability.
- Includes adding new infrastructure features, such as a centralized signal-driven scene manager, among others that make the project a usable template for a real project.
- v2 is one of the two definitive reference templates: the "pure" Rust one on top of Godot nodes.
- v2 has TWO inseparable pillars, both mandatory in the final result of every user story: (1) idiomatic Rust that leans on the type system to reduce or rule out bugs — enums instead of loose booleans/counters that admit invalid combinations, newtypes for identifiers, "parse, don't validate" (external data such as `ConfigFile` values is parsed ONCE into typed structs/enums and consumers read typed fields; no repeated string-keyed lookups with `.to::<T>()` at the point of use), `match` over `if`/`else if` chains on enums; (2) reduction of FFI crossings between Rust and the engine, achieved through the interface/implementation separation defined in Principle III.
- The two pillars are done together: an intermediate step inside a user story may refactor with the logic still in the glue code, but the user story is only complete when the logic lives in pure Rust per Principle III and is idiomatic.
- Behavioral parity: each user story ends with observable behavior identical to the baseline on branch `v1`, verified by the same regression means as v1 (headless validation plus a parity harness run on both branches) and by user visual checkpoints. The ONLY behavior changes allowed are items of `docs/v2-backlog.md` explicitly listed in the user story's spec; a backlog item is marked done in `docs/v2-backlog.md` in the commit that closes it.
- The replicated/exported surface is preserved (see Principle II): the internal model may be richer than the GDScript one (enums with data, typed structs), but every `#[export]`/`#[var]` referenced by the scenes keeps its name and a compatible type, as a projection of the model.
- Engine API gaps (a symbol the local Godot has but the gdext prebuilt API lacks — motivating case: `Scaling3DMode::NEAREST`, Godot 4.7, absent from gdext 0.5.5 / API 4.6, carried as a local integer constant) MUST be isolated behind one typed value, marked at the exact spot with the comment `// api-gap(godot-<version>): <symbol> — <reason>; replace when the binding ships it`, and catalogued in `docs/api-gaps.md` (symbol, introducing version, workaround, location) in the same commit. Changing the crate's API feature set (`api-custom`, `api-4-x`) to remove a gap is a user decision, never an implementer's default.

**v3 — ECS layer over the nodes (branch `v3`, created from v2)**
- Goal: rewrite the GAMEPLAY code of v2 as ECS systems operating on entities, using the `bevy_ecs` crate on top of the Godot node tree, so that a developer writes gameplay thinking in systems and components, with the node reduced to a view of the entity (its physical/visual body), touched only by an explicit sync layer.
- v3 has TWO objectives, both mandatory in the final result of every user story: (1) the two pillars of v2 are KEPT — idiomatic Rust leaning on the type system, and the interface/implementation separation of Principle III (engine-facing glue thin, domain logic pure); (2) the ECS layer sits OVER the nodes: gameplay logic lives in systems scheduled by one World/one Schedule; no gameplay node runs per-frame logic of its own.
- Explicit NON-objectives, recorded so they are not re-litigated: cache locality / data-layout performance is NOT a goal of v3 and MUST NOT be used to justify a design choice; no engine subsystem is replaced (physics, rendering, animation, multiplayer replication, scene instancing remain Godot's); no full `bevy` App, no `godot-bevy`, no other engine brought into the project.
- Dependency policy: `bevy_ecs` only, pinned to a stable release line (no `-rc`), with `default-features = false, features = ["std"]` — no `bevy_reflect`, no `async_executor`, no `multi_threaded` (the Godot API is main-thread only; gdext's `experimental-threads` feature — enabled in this workspace since v1 milestone D for `ResourceLoader::load_threaded_*` — only widens gdext's own API surface and does NOT make the engine thread-safe or `Gd<T>` `Send`: `RawGd.obj: *mut T` has no `Send`/`Sync` impl under any feature). Any additional supporting crate requires justification in the spec that introduces it and is listed there.
- Scope (fixed by this constitution; moving a module between the lists is a PATCH amendment): modules rewritten in ECS — `player`, `player_input`, `camera_noise_shake`, `bullet`, `part`, `part_disappear`, `blast`, `red_robot`, `door`. Modules that stay as v2 code, untouched except for the calls required to integrate with the ECS layer — `settings` (configuration read once), `menu` (UI), `main_scene` (scene-tree manager), `level` (one-shot GI setup and prefab spawner — it keeps calling `instantiate`; the spawned node's bridge registers the entity), `debug_label` (engine metrics overlay), `flying_forklift` (one-shot `ready`: picks a model with `randf()` and applies the shadow setting; no per-tick state — fails the bullet's own criterion). Criterion: a module enters the ECS when it has per-tick state that systems can operate on together; configuration, UI and scene-tree infrastructure do not.
- Behavioral parity: the baseline of v3 is branch `v2` (which already contains the v1 and v2 bug fixes). Each user story ends with observable behavior identical to `v2`, verified by the same means (headless validation, parity harness run on a `v2` worktree and on `v3` with separate user-data directories and seeded RNG, user visual checkpoints). The ONLY behavior changes allowed are open items of `docs/v2-backlog.md` explicitly listed in the user story's spec (marked done in that file in the closing commit) — no separate v3 backlog is created. Timing shifts caused by moving `godot::task`-based awaits to tick-driven timer components count as behavior changes and are allowed only when listed in the spec with the measured difference.
- Preserved surfaces (Principle II still applies): scene files remain the prefabs; every `#[export]`/`#[var]` referenced by the scenes and every `SceneReplicationConfig` property keeps its name and a compatible type as a projection of the components; `#[func]`/`#[rpc]`/`#[signal]` names referenced by scenes stay.
- Where the ECS abstraction touches the engine (physics queries mid-tick, root motion from `AnimationTree`, replication through node properties, scene instancing), the touch point MUST be recorded in `docs/v3-tradeoffs.md` (module, what the engine owns, why the ECS cannot hide it, how the sync layer handles it) in the same commit — same role `docs/api-gaps.md` had in v2. The api-gap rule of v2 continues to apply unchanged.

**Phase governance rules**
- The phases are strictly sequential: no v2 work starts before v1 is declared complete, and no v3 work before v2 is declared complete. v2 was declared complete at commit `e2932b4` (branch `v2`), the point from which branch `v3` was created; from then on, v3 specs declare phase v3.
- Every spec, plan and task MUST declare which phase it belongs to and respect that phase's restrictions. A v1 task that introduces abstraction, refactoring or optimization violates this constitution and must be rejected or moved to the v2 backlog.
- Improvement notes identified during v1 are recorded as backlog for v2, without being implemented.
- Improvements identified during v1 MUST be recorded in `docs/v2-backlog.md` (one entry per improvement, with the originating script/scene and the motivation) in the same commit in which they were noticed. "Noting" without recording in that file does not satisfy this rule.
- During v2, the backlog is CONSUMED, not just written: each spec MUST list which `docs/v2-backlog.md` items it closes and which it defers. Items may still be added to the backlog during v2, only in the same one-entry-per-item form (originating script/scene and motivation).
- **Conservative fixing of upstream bugs (explicit exception to v1's "no improvements")**: an objective defect of the original demo — behavior the code itself clearly intended and does not deliver (e.g. a non-existent node path, a null reference, a wrong animation name) — MAY be fixed in v1. The fix MUST be conservative: only enough for the bug to stop occurring, without getting into large modifications relative to the original source; no restructuring, extracting, renaming or "taking the opportunity" to improve the surrounding code. Improvements and idiomatic refactorings remain reserved for v2.
- Criterion to distinguish bug from improvement: it is a bug when the intent of the original code is unambiguous and the observed result contradicts it (the door should open and does not). It is an improvement — and therefore forbidden in v1 — when the original works as written and the change would make it "better" (real exclusion in the raycast, recapturing the camera rotation, not recomputing hidden text). When in doubt, it is an improvement: it goes to the v2 backlog.
- Every bug fix in v1 MUST be (a) declared in the feature's spec with the description of the defect and of the minimal fix, (b) implemented in an isolated and identifiable way inside the port's commit (comment `// upstream bug fix: ...` at the exact spot), (c) mentioned in the commit message, and (d) recorded in `docs/upstream-bugs.md` (defect, script/scene, fix applied, commit) to serve as reference for v2 and for a possible upstream contribution.

### II. Verifiable Port Cycle

Every ported GDScript script follows a fixed cycle, and a port is only considered complete when all the steps below have been fulfilled and evidenced:

**Binding by type, never by script**
- Each `.gd` script MUST become exactly one Rust class registered via gdext, with the SAME base class as the original script (e.g. `extends CharacterBody3D` → `#[class(base=CharacterBody3D)]`). Clarification: when the script's `extends` is an ANCESTOR of the type of the node it is attached to in the scene (e.g. `extends Node3D` on a `CharacterBody3D` node), the Rust class's base is the node's type in the scene — the `type` swap cannot demote the node nor discard what the scene gave it (physics body, children that depend on the type). An ancestor `extends` is merely a looser GDScript declaration; the observable behavior comes from the node. The port's spec MUST record the case when it occurs.
- The Rust class is bound to the scene by swapping the node's `type` in the `.tscn` (equivalent to the editor's "Change Type") and removing the `script` and the `.gd` `ext_resource`. It is FORBIDDEN to keep a `.gd` attached to the node as a bridge, wrapper or fallback.
- Methods exposed with `#[func]` MUST preserve the original GDScript name, so that the scenes' `[connection]`s, existing `has_method()` calls and `.rpc()` calls remain valid without editing.
- The `.gd` and its `.gd.uid` MUST be removed in the same commit in which the node starts using the Rust class.

**Bottom-up port order**
- This order governed v1 (GDScript → Rust): a script could only be ported after all the scripts it depends on (via method, property, enum, signal or `is Type` check) were already in Rust. Rust code MUST NOT call custom API of a GDScript script (dynamic `.call()`, `.get()`, `.set()`, `get_script()` comparison), since that produces untyped code, impossible to validate and discarded in the next step.
- In v2 and later, the order is "infrastructure first": a module whose typed API other modules will consume (motivating case: the `Settings` autoload, root of 5 consumers) is remodeled before its consumers; after that, modules follow the leaf-to-root order recorded in `docs/port-order.md`.
- Calls that use only Godot's base API (methods and properties of the node's base class) do not constitute a dependency and are free in any direction.
- Duck typing that already exists in the original GDScript (`has_method`, `has_signal`, `.rpc()` by name) MAY be preserved as is in v1, since it is a direct translation and not a new dependency.
- Single, documented exception in v1: the `Settings` autoload was ported last, and until then Rust consumers MAY access it dynamically via `/root/Settings`, with the corresponding item recorded in the v2 backlog. The 10 dynamic accesses present in the v1 code exist precisely because `Settings` was ported last.
- v2 lifts that exception: those accesses are REMOVED. Typed `Gd<Settings>` access is mandatory, and dynamic access — `get_node_as::<Node>("/root/...").get(...)`, `.call(...)` by name, `has_method`/`has_signal` duck typing, `Callable::from_object_method` by name, `call_deferred` by name — is FORBIDDEN in v2 code except where the engine offers no typed alternative (today: `.rpc("name")`, which gdext exposes only by name). Such residual cases MUST be listed in the spec of the user story that touches them.
- The dependency graph and the order derived from it live in `docs/port-order.md`; that document is planning input, not part of this constitution, and may be revised without an amendment.

**Preservation of property names**
- Besides method names, every exported (`@export`) property or property replicated by `MultiplayerSynchronizer` (listed in `SceneReplicationConfig` in the scenes) MUST keep the original GDScript name when it becomes `#[export]`/`#[var]`. Renaming one of these properties silently breaks values saved in the scenes and multiplayer replication, with no error at compile time nor at import.
- Before concluding a port, the author MUST check in the affected `.tscn` which script properties are referenced (exported values, replication `properties/N/path`, `node_paths`) and ensure all of them exist in the Rust class with the same name and a compatible type.
- The NAME of the registered Rust class MUST NOT coincide with any engine class nor with a top-level identifier (`const`, `class_name`, `var`) of any `.gd` still present in the project: GDScript rejects the whole script ("The member X shadows a native class") and the game stops loading. Check before naming (command in `CLAUDE.md`); in case of collision, the port's name changes — never the remaining `.gd`.

**Mandatory build**
- Every change to Rust code MUST be followed by a successful `cargo build` (debug profile) with no new warnings, before any validation or commit. The dynamic library in `target/debug/` is the artifact Godot loads; uncompiled code does not exist for the game.

**Headless validation as definition of done**
- No port is considered complete without validation executed in headless Godot: at minimum (a) the project import confirming the extension loads (`Initialize godot-rust ...`) and (b) running the affected scene with no new errors relative to the known baseline of the upstream demo.
- Pre-existing errors of the upstream demo do not count as regressions, but MUST be catalogued in the operational guide (`CLAUDE.md`) so they can be told apart from errors introduced by the port.
- Each port's commit MUST record in its message which script was ported and which scene(s) had the node type swapped.
- Upstream errors that cease to exist because of a bug fix MUST be removed from the pre-existing error catalog in `CLAUDE.md` in the same commit, so that the catalog always reflects the current baseline.

**Separation between rule and operation**
- This constitution defines WHAT is mandatory. Binary paths, exact commands, tool versions and API notes belong in the `CLAUDE.md` at the repository root and MUST NOT be incorporated here; changes to those details do not constitute an amendment.

### III. Interface vs. Implementation (v2 and later)

Starting in v2, every Rust class enforces a hard separation between engine-facing glue and pure domain logic, stated in the vocabulary of gdext:

- `#[godot_api] impl I<Base> for X` (engine virtual trait: `INode3D`, `ICharacterBody3D`, ...) contains ONLY lifecycle callbacks (`init`, `ready`, `process`, `physics_process`, `input`, ...), and each callback delegates: it reads engine state, calls the model, writes the result back. No domain logic (no state machines, no math, no decisions) inside the trait impl.
- `#[godot_api] impl X` contains ONLY the API exposed to the engine/scenes: `#[func]`, `#[signal]`, `#[rpc]`, `#[constant]`, property setters. These are thin and delegate as well.
- Domain logic lives in plain `impl X` blocks (no macro), free functions, or dedicated modules, in PURE Rust: it MUST NOT use `Gd<T>`, engine singletons (`Input`, `Os`, `RenderingServer`, ...), or engine-backed builtins (`Variant`, `GString`, `StringName`, `VarDictionary`, `VarArray`, `Callable`). It MAY use gdext's builtin math types (`Vector2/3`, `Basis`, `Quaternion`, `Transform3D`, `Color`, ...) and those of their methods that are implemented in Rust (operators, `from_quaternion`, `get_quaternion`, `from_euler`, `orthonormalized`, vector `lerp`, ...). A builtin method whose body goes through `as_inner()` or that is generated under the bindings' `out/builtin_classes/**` (e.g. `Quaternion::slerp*`, `Basis::looking_at`) calls the engine and belongs in glue: the pure step returns the data the decision needs (a target vector, a weight) and the glue makes the engine call. The practical check is the unit test itself — an engine-backed method panics with 'Godot engine not available' under `cargo test`.
- Pure logic MUST be covered by unit tests (`#[cfg(test)]`, plain `#[test]`) that run with `cargo test` and no Godot binary. A user story that extracts pure logic without tests is incomplete.
- Per-frame and per-event shape: engine state is read ONCE into a snapshot, the pure step runs, results are applied ONCE. References to nodes and resources are resolved once (`OnReady`, `OnEditor`, preloaded `Gd<PackedScene>`), never with `get_node_as`, `get_parent().cast()` or `load(...)` inside `process`/`physics_process` or inside handlers that fire per event (shots, hits, explosions).
- Mandatory gates (extending Principle II's build gate): `cargo build`, `cargo clippy` with no warnings, and `cargo test` green — all three before validation or commit.
- Constants that are tuning parameters of a type belong to the type (associated consts) or to a tuning struct with `Default` that the pure functions receive as input; module-level `const` is reserved for paths/strings and true module-scope constants. Design choices beyond that (which enum, which struct) are matters for specs and plans, not for this constitution.

**ECS shape (v3)**

In v3 the separation above is kept and takes the following shape for the gameplay modules listed in Principle I:

- One World, one Schedule, one driver: the `bevy_ecs::World` lives in a single autoload node (`EcsWorld`), reached by typed `godot::tools::get_autoload_by_name`, never through dynamic paths. Exactly one driver node runs the tick: `physics_process` runs the fixed-step schedule, `process` runs the frame schedule. No other node calls `run_schedule`.
- Bridge classes: the Rust class of each gameplay scene root becomes a bridge. It has NO `process`/`physics_process`. Its `ready` registers the entity (components built from its exported properties and from child node handles resolved once); its `exit_tree` despawns it; its `#[func]`/`#[rpc]`/signal handlers ONLY translate the engine event into a typed event pushed to an inbound queue drained at the start of the next tick. Re-entrancy rule: an engine callback (signal, RPC with `call_local`, `#[func]` invoked by an animation track) NEVER borrows the World; violating this is a double-borrow panic and is rejected in review.
- Tick phases, in this order, as labeled schedule sets: `SyncIn` (engine → components, one read per value) → gameplay systems (PURE, under the existing Principle III rule: no `Gd<T>`, no singletons, no engine-backed builtins, same builtin-math rule) → `EngineQuery` sets, only where a decision needs an engine answer mid-tick (raycasts, `move_and_slide`/`move_and_collide`, root-motion reads), each one recorded in `docs/v3-tradeoffs.md` → `SyncOut` (components → engine, one write per value, using change detection (`Changed<T>`) wherever it removes writes without changing behavior). Engine access is confined to `SyncIn`/`EngineQuery`/`SyncOut` systems and bridges.
- Node handles: `Gd<T>` is `!Send` and cannot be a `Component`. Components hold NO engine handles; the `Entity ↔ Gd<T>` map is a `NonSend` resource maintained by the bridges on register/unregister and read only by sync systems. A node that dies (handle no longer valid) despawns its entity in `SyncIn`; a despawned entity's node is freed through the sync layer, never from a gameplay system.
- Multiplayer: authority is a component (marker or `PeerId`-carrying) set at registration; gameplay systems filter by it. On the authority, `SyncOut` writes components into the replicated node properties; on non-authority peers, `SyncIn` reads the replicated properties into components. `.rpc("name")` remains the one residual dynamic access (Principle II).
- Timers and delays: v2's `godot::task` futures become timer components stepped by a system; no async tasks in v3 gameplay code.
- Tests: systems are tested without Godot — `World::new()`, insert resources/entities, run the system (`run_system_once`/`run_system_cached`) and assert on components. Gates unchanged: `cargo build`, `cargo clippy` with no warnings, `cargo test` green.
- Tuning constants: the v2 rule holds; tuning structs become resources (`Res<XxxTuning>`) with `Default`.

## Governance

This constitution takes precedence over any other practice, convention, documentation or ad-hoc decision within the project. In case of conflict between this constitution and any other document or coding habit, this constitution prevails.

**Amendment process**: any change to this constitution is made through the `/speckit-constitution` command, requires an explicit justification for the change and results in a new version of the document, accompanied by the corresponding Sync Impact Report.

**Versioning policy**: this constitution follows semantic versioning (MAJOR.MINOR.PATCH):
- MAJOR: removal or incompatible redefinition of a principle or of one of the v1/v2/v3 phases (e.g. eliminating a phase, changing its order, or removing a non-negotiable restriction).
- MINOR: addition of a new principle or section, or material expansion of the rules of an existing phase.
- PATCH: clarifications, wording fixes or non-semantic adjustments.

**Compliance review**: every spec, plan and task MUST explicitly declare the phase (v1, v2 or v3) it belongs to, per Principle I. Planning and code reviews MUST verify that the work respects the restrictions of the declared phase — in particular, that no abstraction, refactoring or optimization is introduced during v1. Reviews MUST likewise verify compliance with Principle II (Verifiable Port Cycle) in every GDScript script port. Work that violates the current phase must be rejected or redirected to the correct phase's backlog. Port reviews MUST further confirm that the dependency order was respected, that no exported or replicated property name was changed, and that noticed improvements were recorded in `docs/v2-backlog.md`. Reviews MUST confirm that every bug fix in v1 meets the four requirements of Principle I (spec, isolation in code, commit, `docs/upstream-bugs.md`) and that no improvement was introduced under the label of a fix. v2 reviews MUST additionally verify compliance with Principle III (Interface vs. Implementation) — that `I<Base>` trait impls and `#[godot_api] impl X` blocks contain only glue, that pure logic carries unit tests, that no per-frame or per-event node/resource lookup was introduced, and that no dynamic access appears outside the residual cases listed in the touching spec — that behavioral parity with `v1` was evidenced (headless validation, parity harness run on both branches, user visual checkpoints), and that every `docs/v2-backlog.md` item a spec claims to close was actually closed and marked done in that file. v3 reviews MUST additionally verify that no gameplay bridge has `process`/`physics_process`, that no engine callback borrows the World, that engine access appears only in `SyncIn`/`EngineQuery`/`SyncOut` systems and bridges, that each `EngineQuery` set and each engine touch point is recorded in `docs/v3-tradeoffs.md`, that the excluded modules (`settings`, `menu`, `main_scene`, `level`, `debug_label`) were not rewritten in ECS, that parity with `v2` was evidenced (headless, seeded harness on both trees, visual checkpoints), and that the only behavior changes are open `docs/v2-backlog.md` items listed in the spec.

**Version**: 1.5.2 | **Ratified**: 2026-09-15 | **Last Amended**: 2026-09-18
