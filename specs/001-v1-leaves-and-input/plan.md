# Implementation Plan: Milestone A — leaves and player input (v1 raw port)

**Branch**: `main` (v1 lives on `main`; each port is an atomic commit that leaves the game playable) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-v1-leaves-and-input/spec.md`

**Phase**: v1 — Raw Port (Principle I). Direct translation; no abstraction, refactoring or optimization.

## Summary

Port the 5 leaf scripts of the TPS demo (`debug.gd`, `part_disappear.gd`, `blast.gd`,
`camera_noise_shake_effect.gd`, `player_input.gd`; 242 lines of GDScript) to 5 gdext
0.5.5 classes, one per script and with the same base class, swapping the node's `type` in the scenes
(`level.tscn`, `part_disappear.tscn`, `impact_effect.tscn`, `player.tscn` ×2) and deleting the `.gd` +
`.gd.uid` in the same commit. The 10 remaining scripts stay intact and keep consuming the ported
API by name (`add_trauma`, `PlayerInputSynchronizer.*`). Each port follows the cycle of
Principle II: `cargo build` with no new warnings → headless import with `Initialize godot-rust` →
headless run of the affected scene with no new errors → its own commit. Technical approach: `await`
becomes a typed signal connection (`connect_other` with a callable *linked* to the node, invalidated
automatically if the node is freed), `@onready` becomes `OnReady` with `#[init(node = ...)]`,
node `@export` becomes `#[export] Option<Gd<T>>`, `@rpc("call_local")` becomes
`#[rpc(authority, call_local, unreliable)]` (the GDScript defaults). All signatures were
checked against the crate and the bindings generated in `target/debug/build/godot-core-*/out/`
(see [research.md](research.md)).

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), already in
`[workspace.dependencies]` — do NOT change version or features.

**Primary Dependencies**: gdext 0.5.5 (default prebuilt API 4.6 — confirmed in
`godot-bindings-0.5.5/src/import.rs:72`, keep); Godot 4.7.2 stable at `/usr/bin/godot.x86_64`
(there is no `godot` on the PATH). `.gdextension` with `reloadable = true`, debug lib at
`oxide_godot_core/target/debug/liboxide_godot.so`.

**Storage**: N/A

**Testing**: `cargo build` (debug profile) + Godot headless (import + scene run). There are no
Rust unit tests in this phase (they would be new infrastructure — v2). Visual validation by the user.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + Godot project
(`oxide-godot/`)

**Performance Goals**: parity with the original (do not optimize — Principle I)

**Constraints**: Principles I and II of the constitution v1.2.0; method/property names
identical to the GDScript; one commit per script; `.gd` + `.gd.uid` deleted in the same commit;
improvements only in `docs/v2-backlog.md`.

**Scale/Scope**: 5 scripts, 242 lines of GDScript, 4 scenes edited (one of them twice),
5 commits.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I — Three-Phase Port

| Rule | Status | Evidence |
|---|---|---|
| Phase declared in spec/plan/tasks | ✅ | spec.md "Phase: v1"; this plan "Phase: v1" |
| Direct translation, without remodeling nodes/scenes | ✅ | Structure of the 4 scenes untouched beyond the `type` swap and removal of `script`/`ext_resource` |
| No abstraction/refactoring/optimization | ✅ | One module per script, no common module, no shared helpers, no traits (see Structure Decision). Quirks preserved: overlay text recomputed while hidden; `start_rotation` captured once; raycast without effective exclusion (FR-017); `jumping` exported but not replicated |
| Improvements → `docs/v2-backlog.md` in the same commit | ✅ | Candidates already identified in research.md §"Candidate v2 backlog"; each one goes into the commit of the script where it was noticed |

### Principle II — Verifiable Port Cycle

| Rule | Status | Evidence |
|---|---|---|
| One class per script, same base | ✅ | `DebugLabel: Label`, `PartDisappear: CpuParticles3D`, `Blast: Node3D`, `CameraNoiseShake: Camera3D`, `PlayerInputSynchronizer: MultiplayerSynchronizer` |
| Binding by `type` swap in the `.tscn`; no bridge `.gd` | ✅ | Table "Scene edits" below, lines checked on 2026-09-15 |
| Identical `#[func]` names | ✅ | `add_trauma`, `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis`, `jump` — [contracts/](contracts/) |
| Identical exported/replicated property names (checked in the `.tscn`) | ✅ | `player.tscn` lines 42–53 replicate `InputSynchronizer:shoot_target/motion/shooting/aiming`; lines 343–351 `node_paths` `camera_animation/crosshair/camera_base/camera_rot/camera_camera/color_rect` — all exist in the contract with the same name |
| `cargo build` with no new warnings before validating/committing | ✅ | quickstart.md step 1; current baseline: 0 warnings |
| Headless validation (import + scene) | ✅ | quickstart.md steps 2–3; error baseline = the 3 from `CLAUDE.md` |
| Commit per port with script + scene in the message | ✅ | Format in quickstart.md §"Commit" |
| `.gd` + `.gd.uid` deleted in the same commit | ✅ | Table "Scene edits"; `grep` check of the uid after removal |
| Bottom-up order | ✅ | The 5 are leaves (`docs/port-order.md`); none depends on another. Is item 4 called by 5? No: it is called by `player.gd` (GDScript) via `player_input.camera_camera.add_trauma()` — GDScript→Rust call, allowed |
| Rust does not call custom GDScript API | ✅ | None of the 5 classes calls `.call()`/`.get()` on a script: `get_parent()` uses only `global_transform`/`get_world_3d()` (Node3D base API) |
| `Settings` exception | N/A | None of the 5 scripts uses `Settings` |

**Gate result (pre-Phase 0)**: PASS — no violations, Complexity Tracking empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-v1-leaves-and-input/
├── plan.md              # This file
├── spec.md              # Specification (already validated)
├── research.md          # Phase 0: confirmed gdext 0.5.5 signatures + decisions
├── data-model.md        # Phase 1: state of PlayerInputSynchronizer and camera trauma
├── quickstart.md        # Phase 1: validation commands in the exact order
├── contracts/
│   ├── player-input-synchronizer.md   # surface consumed by player.gd / player.tscn
│   └── camera-noise-shake.md          # surface consumed by player.gd (via camera_camera)
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — not created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/                       # Cargo workspace (unchanged)
└── oxide_godot_lib/
    ├── Cargo.toml                      # unchanged (godot = { workspace = true })
    └── src/
        ├── lib.rs                      # ExtensionLibrary + `mod` for each module (only that)
        ├── debug_label.rs              # struct DebugLabel,              base=Label                  (port 1)
        ├── part_disappear.rs           # struct PartDisappear,           base=CpuParticles3D         (port 2)
        ├── blast.rs                    # struct Blast,                   base=Node3D                 (port 3)
        ├── camera_noise_shake.rs       # struct CameraNoiseShake,        base=Camera3D               (port 4)
        └── player_input.rs             # struct PlayerInputSynchronizer, base=MultiplayerSynchronizer (port 5)

oxide-godot/                            # Godot project
├── level/level.tscn                    # port 1: node Debug → type="DebugLabel"
├── level/debug.gd (+ .uid)             # port 1: DELETE
├── enemies/red_robot/parts/part_disappear_effect/
│   ├── part_disappear.tscn             # port 2: root → type="PartDisappear"
│   └── part_disappear.gd (+ .uid)      # port 2: DELETE
├── enemies/red_robot/laser/impact_effect/
│   ├── impact_effect.tscn              # port 3: root → type="Blast"
│   └── blast.gd (+ .uid)               # port 3: DELETE
└── player/
    ├── player.tscn                     # port 4: Camera3D → type="CameraNoiseShake"; port 5: InputSynchronizer → type="PlayerInputSynchronizer"
    ├── camera_noise_shake_effect.gd (+ .uid)  # port 4: DELETE
    └── player_input.gd (+ .uid)        # port 5: DELETE

docs/v2-backlog.md                      # new entries per commit, when noticed
```

**Structure Decision**: one Rust module per GDScript script, file name = script name
(except redundant suffixes), no shared module and no common helpers — any common utility
would be abstraction (Principle I). `lib.rs` contains only the `ExtensionLibrary` and the `mod`
declarations. The class name `PlayerInputSynchronizer` is mandatory: `player.gd:26` declares
`@onready var player_input: PlayerInputSynchronizer = $InputSynchronizer` and, without the `.gd`, the
GDScript analyzer resolves that type to the registered native class. The other 4 names are free
(no `class_name` in the original); the choice above avoids colliding with engine classes
(`Label`, `Blast` does not exist in the engine — checked against the bindings list).

Details of each class (fields, virtuals, signals) are in [research.md](research.md) §"Map per
script"; they are not repeated here.

## Scene edits (lines checked on 2026-09-15 — re-check with `grep -n` before editing)

| Port | Scene | Node (line) | `type` before → after | Remove | Delete |
|---|---|---|---|---|---|
| 1 | `level/level.tscn` | `Debug` (l.148) | `Label` → `DebugLabel` | l.156 `script = ExtResource("9")`; l.7 `[ext_resource type="Script" uid="uid://6ec6m14rhsxi" ... id="9"]` | `level/debug.gd`, `level/debug.gd.uid` |
| 2 | `.../part_disappear.tscn` | root `PartDisappearPuff` (l.43) | `CPUParticles3D` → `PartDisappear` | l.59 `script = ExtResource("3")`; l.5 ext_resource `uid://dxd6xoeg627y6` id="3" | `part_disappear.gd`, `.uid` |
| 3 | `.../impact_effect.tscn` | root `Blast` (l.170) | `Node3D` → `Blast` | l.171 `script = ExtResource("5")`; l.7 ext_resource `uid://bk20efkdq4v3m` id="5" | `blast.gd`, `.uid` |
| 4 | `player/player.tscn` | `Camera3D` (l.630, parent `CameraBase/CameraRot/SpringArm3D`) | `Camera3D` → `CameraNoiseShake` | l.633 `script = ExtResource("8")`; l.11 ext_resource `uid://byrvr71jmaisi` id="8" | `camera_noise_shake_effect.gd`, `.uid` |
| 5 | `player/player.tscn` | `InputSynchronizer` (l.343) | `MultiplayerSynchronizer` → `PlayerInputSynchronizer` | l.345 `script = ExtResource("2_g11dy")`; l.5 ext_resource `uid://m1xn31x0lssx` id="2_g11dy". **KEEP** `node_paths=PackedStringArray(...)` on l.343, `replication_config` (l.344) and the 6 lines `xxx = NodePath(...)` (l.346–351) | `player_input.gd`, `.uid` |

After port 4, the lines of port 5 shift (−1 due to the removal of the ext_resource id="8" on l.11):
re-check. After each removal: `grep -rn "<uid>" oxide-godot/` must return empty
(pre-checked: each uid is referenced by exactly one scene).

Note on the `type` in the `.tscn`: Godot writes `type="<registered class name>"`; for
GDExtension classes that is the name of the Rust struct. When opening the scene in the editor after the port, the editor
may rewrite the `.tscn` (property order, `unique_id`) — such diffs are acceptable as long
as the `type` and the properties remain.

## Visual validation per script (SC-002 — done by the user in the editor/game)

| Port | What to check in the game (compare with `../oxide_godot_origins/`) |
|---|---|
| 1 | Enter the level; F3 toggles the overlay; lines `FPS: 60.0` (with `.0`), `VSync: Enabled/Disabled`, `Memory: xx.xx MiB`, `Online: No` (no ID line in single-player); values change every frame |
| 2 | Kill a robot: each part, when disappearing, fires mini-blasts immediately and the smoke puff 0.2 s later; the effect goes away on its own (~3.2 s) |
| 3 | Let the robot shoot: the impact animates, the light rays stay facing the camera while moving around, and the effect goes away at the end of the animation |
| 4 | Shoot (light shake, 0.35) and get hit by the robot's laser (strong, 13.0 saturated at 1.2); the medium level (0.75, RPC `hit`) only occurs in multiplayer with another player's bullet — not verifiable single-player; the camera returns exactly to the rest position; intensity/duration equal to the original |
| 5 | Move (WASD/analog stick), look (mouse and analog stick; slower while aiming), pitch locks at −89.9°/70°, aim by short tap (stays) and by hold (releases), shoot/far camera animations, jump, shooting hits the point under the crosshair, falling off the map darkens the screen (black at −32) and comes back with fade-out |

## Complexity Tracking

> Empty — the Constitution Check has no violations to justify.

## Constitution Check — post-design re-evaluation (Phase 1)

Re-evaluated after research.md, data-model.md, contracts/ and quickstart.md:

- No artifact introduces a common module, trait, state enum or helper — each class is
  self-contained (Principle I). ✅
- The `PlayerInputSynchronizer` contract reproduces the 11 property names and 4 method names of the
  original, checked against `player.gd` (lines 26, 73, 87, 89, 107, 114, 121, 124, 133, 135,
  211) and `player.tscn` (lines 42–53, 343–351). ✅
- The `CameraNoiseShake` contract reproduces `add_trauma(amount: float)`, called by
  `player.gd:211` and, transitively, by `red_robot.gd:133`. ✅
- The only decision that departs from the text of the command input (RPC transfer mode: `unreliable`
  instead of `reliable`) was taken **in favor** of fidelity to the original — see research.md D5. ✅
- Improvements noticed during the research were listed as backlog candidates, not
  applied. ✅

**Gate result (post-Phase 1)**: PASS.
