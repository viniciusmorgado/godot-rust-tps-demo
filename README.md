# godot-rust-tps-demo

A Rust rewrite of the official Godot [Third Person Shooter Demo](https://github.com/godotengine/tps-demo),
using [godot-rust (gdext)](https://github.com/godot-rust/gdext) — Godot 4.7, `godot` crate 0.5.5.

The original demo is written in GDScript. This repository replaces **every** script with a Rust
class registered through GDExtension; the scenes, models, textures, audio and lightmaps are the
original ones.

## Screenshots

| SDFGI | VoxelGI |
|---|---|
| ![SDFGI](repo/sdf_gi.png) | ![VoxelGI](repo/voxel_gi.png) |

<sub>Captured from the finished Rust port (global illumination selectable in the in-game settings).</sub>

## Versions

The rewrite happened in three phases, each on its own branch, all three complete. They are not
successive drafts of the same code: v2 and v3 are two independent reference templates — the same
game, one built on plain Rust over Godot nodes and one built on an ECS over those nodes — and v1
is kept as the baseline both were measured against.

| | v1 — raw port | v2 — idiomatic Rust | v3 — ECS |
|---|---|---|---|
| **Branch** | `main` (mirrored on branch `v1`) | `v2` (branched from the end of v1) | `v3` (branched from the end of v2) |
| **Goal** | Make the original game run entirely in Rust with identical behavior, script by script | Remodel the code around Rust's type system and idioms, and separate engine glue from testable game logic | Write the gameplay as ECS systems over entities, with `bevy_ecs` sitting *over* the Godot nodes: the node becomes a view of its entity, touched only by an explicit sync layer, and no gameplay node runs per-frame logic of its own |
| **Approach** | Direct translation of each GDScript file into one gdext class with the same base node; no abstractions, no optimizations — "Rust that still reads like GDScript" | Two inseparable pillars: (1) the type system does the validating — enums with data instead of loose flags and counters, typed config parsed once (*parse, don't validate*), typed signals and references; (2) *interface vs. implementation* — the engine-facing trait impls and `#[godot_api]` blocks are thin glue, and the game logic lives in plain Rust modules with no engine types, unit-tested without running Godot | Keeps v2's two pillars (typed Rust; thin glue over a pure core — the pure functions of v2 became the bodies of the systems) and adds the ECS layer: one `World` and two schedules owned by a single autoload that runs last in every physics step and frame; each scene root is one entity, registered from `ready` and unregistered from `exit_tree`; engine callbacks only push typed events to a queue that never borrows the world; every tick is `SyncIn → Gameplay → EngineQuery → SyncOut`, gameplay systems being pure and unit-tested |
| **Godot ↔ Rust boundary** | Whatever the original did dynamically stays dynamic (`has_method`, `has_signal`, `rpc("name")`, autoload reached by path); node types swapped in the scenes | Typed `Gd<T>` everywhere, typed signals, typed autoload; nodes and resources resolved once (`OnReady`/`OnEditor`/preloaded scenes), per-frame work as *snapshot → pure step → apply*; the only by-name calls left are `rpc("name")` (gdext has no typed RPC) | Node handles never enter components (`Gd<T>` is not `Send`): they live in a `NonSend` map keyed by entity, read only by the sync systems; replicated properties stay node fields, written by `SyncOut` on the authority and read by `SyncIn` elsewhere; the engine keeps physics, animation, replication and scene instancing — every point where the ECS has to touch it is a row of [`docs/v3-tradeoffs.md`](docs/v3-tradeoffs.md) |
| **Verification** | `cargo build` with no warnings, headless runs of every scene, and a scratch-scene parity harness run on the original GDScript project and on the port, outputs diffed | The same, plus `cargo clippy` and `cargo test` as mandatory gates (133 unit tests on the pure logic), a parity harness run against the `v1` branch after every user story, and a visual checkpoint before each milestone advances | Inherits v2's gates (214 unit tests, systems tested with a bare `World`), a parity harness run against the `v2` branch on every milestone — 14 scripted cases diffed frame by frame, seeded, with a physics-step probe — and single-player plus two-instance multiplayer checkpoints |
| **Upstream behavior** | Preserved, including quirks; only three objective upstream bugs fixed, minimally (see [`docs/upstream-bugs.md`](docs/upstream-bugs.md)) | Parity with v1 by default; each deviation is a reviewed item of [`docs/v2-backlog.md`](docs/v2-backlog.md) (33 items, 28 closed) — see the bug list below | Parity with v2 measured, not assumed: six read-order shifts recorded (all within the same rendered frame) and one deliberate behavior change (the robot's countdowns reset when the player re-enters its area, backlog #31) — see the trade-offs below |
| **Intended use** | Historical reference, benchmark baseline, example of a raw port | Reference template for Godot + Rust projects without ECS — the baseline v3 was built from | Reference template for Godot + Rust projects with ECS — the same game, systems instead of node scripts |
| **Status** | **Complete** — 15/15 scripts ported, 0 `.gd` left, game playable end to end | **Complete** — all 15 modules remodeled over five milestones (specs 006–010), 133 unit tests | **Complete** — all 9 gameplay modules on the ECS over three milestones (specs 011–013), 214 unit tests; configuration, menu, scene manager, level setup and the debug overlay deliberately stay v2 code |

> **Note:** for real projects, take examples only from **v2** and **v3**. v1 deliberately keeps
> GDScript idioms, dynamic calls and upstream quirks so that behavior could be compared script by
> script; it is a baseline, not a template.

The rules of each phase are in [`.specify/memory/constitution.md`](.specify/memory/constitution.md);
the per-module analysis that drove v2 is in [`docs/v2-catalog.md`](docs/v2-catalog.md), the engine
touch points of v3 in [`docs/v3-tradeoffs.md`](docs/v3-tradeoffs.md), and each milestone's
specification, plan and task list is under [`specs/`](specs/).

## Layout

```
oxide-godot/          Godot 4.7 project (scenes + assets from the original demo, extension.gdextension)
oxide_godot_core/     Cargo workspace; crate `oxide_godot` (cdylib) in oxide_godot_lib/
docs/                 port order, upstream bugs found and fixed, v2 backlog
specs/                Spec Kit specifications, plans and tasks for each milestone
```

## Building and running

```bash
cd oxide_godot_core && cargo build
```

Then open `oxide-godot/project.godot` in Godot 4.7 and run. The debug build of the library is
loaded by `oxide-godot/extension.gdextension`.

## Licenses

- **Rust code** (`oxide_godot_core/`) and everything else authored in this repository (docs, specs):
  [MIT](LICENSE).
- **Original assets, scenes and the GDScript code they derive from** (`oxide-godot/`): licensed by
  the upstream project. See the upstream license file directly:
  <https://github.com/godotengine/tps-demo?tab=License-1-ov-file> — a copy ships as
  [`oxide-godot/LICENSE.md`](oxide-godot/LICENSE.md) (assets and music under CC-BY 3.0 by their
  respective authors; original code under MIT by Juan Linietsky and the Godot Engine contributors).

## Template

The base project layout (`oxide-godot/` + `oxide_godot_core/` workspace, `extension.gdextension`,
crate wiring) was generated with the [godust](https://crates.io/crates/godust) tooling; the port
was built on top of that scaffold.
