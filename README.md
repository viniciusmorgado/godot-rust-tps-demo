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

The rewrite happens in three phases, each on its own branch. They are not successive drafts of
the same code: v2 and v3 are two independent reference templates, and v1 is kept as a baseline.

| | v1 — raw port | v2 — idiomatic Rust | v3 — ECS |
|---|---|---|---|
| **Branch** | `main` (mirrored on branch `v1`) | `v2` (branched from the end of v1) | `v3` (to be branched from v2) |
| **Goal** | Make the original game run entirely in Rust with identical behavior, script by script | Remodel the code around Rust's type system and idioms, and separate engine glue from testable game logic | Rebuild the v2 design on top of `bevy_ecs`, with ECS driving the Godot nodes |
| **Approach** | Direct translation of each GDScript file into one gdext class with the same base node; no abstractions, no optimizations — "Rust that still reads like GDScript" | Two inseparable pillars: (1) the type system does the validating — enums with data instead of loose flags and counters, typed config parsed once (*parse, don't validate*), typed signals and references; (2) *interface vs. implementation* — the engine-facing trait impls and `#[godot_api]` blocks are thin glue, and the game logic lives in plain Rust modules with no engine types, unit-tested without running Godot | Components and systems for gameplay state; Godot nodes as the presentation/physics layer synchronized from the ECS world |
| **Godot ↔ Rust boundary** | Whatever the original did dynamically stays dynamic (`has_method`, `has_signal`, `rpc("name")`, autoload reached by path); node types swapped in the scenes | Typed `Gd<T>` everywhere, typed signals, typed autoload; nodes and resources resolved once (`OnReady`/`OnEditor`/preloaded scenes), per-frame work as *snapshot → pure step → apply*; the only by-name calls left are `rpc("name")` (gdext has no typed RPC) | ECS world owned by a Rust singleton; nodes read/write components |
| **Verification** | `cargo build` with no warnings, headless runs of every scene, and a scratch-scene parity harness run on the original GDScript project and on the port, outputs diffed | The same, plus `cargo clippy` and `cargo test` as mandatory gates (133 unit tests on the pure logic), a parity harness run against the `v1` branch after every user story, and a visual checkpoint before each milestone advances | Inherits v2 |
| **Upstream behavior** | Preserved, including quirks; only three objective upstream bugs fixed, minimally (see [`docs/upstream-bugs.md`](docs/upstream-bugs.md)) | Parity with v1 by default; each deviation is a reviewed backlog item (`docs/v2-backlog.md` on the `v2` branch), and the `v2` README lists the baseline bugs it fixed | Inherits v2 |
| **Intended use** | Historical reference, benchmark baseline, example of a raw port | Reference template for Godot + Rust projects without ECS | Reference template for Godot + Rust projects with ECS |
| **Status** | **Complete** — 15/15 scripts ported, 0 `.gd` left, game playable end to end | **Complete** — all 15 modules remodeled over five milestones, 133 unit tests (see the [`v2` branch](https://github.com/viniciusmorgado/godot-rust-tps-demo/tree/v2)) | Not started |

> **Note:** for real projects, take examples only from **v2** and **v3**. v1 deliberately keeps
> GDScript idioms, dynamic calls and upstream quirks so that behavior could be compared script by
> script; it is a baseline, not a template.

The rules of each phase are in [`.specify/memory/constitution.md`](.specify/memory/constitution.md)
(this branch carries the version in force when v1 closed; the `v2` branch carries the amended one).

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
