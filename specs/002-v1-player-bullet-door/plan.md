# Implementation Plan: Milestone B — player, bullet and door (v1 raw port)

**Branch**: `main` (v1 lives on `main`; each port is an atomic commit that leaves the game playable) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-v1-player-bullet-door/spec.md`

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.0). Direct translation; no abstraction,
refactoring or optimization. Uses for the first time the **conservative upstream bug fix**
clause — a single fix, in the door.

## Summary

Port `player.gd` (211 lines, `class_name Player`), `bullet.gd` (51) and `door.gd` (12) to three
gdext 0.5.5 classes with the same base (`CharacterBody3D`, `CharacterBody3D`, `Area3D`), swapping the
`type` of the root node in `player.tscn`, `bullet.tscn` and `door.tscn` and deleting `.gd` + `.gd.uid` in the
same commit — in the order player → bullet → door, one commit per script. The remaining 7 scripts
stay byte-for-byte intact and keep finding `Player`, `player_id`, `hit`,
`add_camera_shake_trauma` by their original names. Technical approach: the `Player` consumes
`PlayerInputSynchronizer` and `CameraNoiseShake` (Milestone A) via **typed** access
(`OnReady<Gd<PlayerInputSynchronizer>>`, `bind()`/`bind_mut()`, `cast::<CameraNoiseShake>()`), which
only requires making the consumed fields/methods `pub(crate)`; `player_id` uses
`#[var(set = set_player_id)]` with a setter that works outside the tree; `current_animation` is a
derived enum (`GodotConvert, Var, Export`, `via = i64`); `motion` is `#[var]` to remain
replicable by name; the bullet preserves the duck typing `has_method("hit")` + `rpc` and makes the first
use of the `Settings` exception (`/root/Settings` → `config_file` typed as `ConfigFile`); the door
references `DoorModel2/AnimationPlayer` with the comment `// upstream bug fix` at the exact spot and
creates `docs/upstream-bugs.md`. All signatures were confirmed by compiling a draft
module (0 warnings) and by a headless probe — see [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), already in
`[workspace.dependencies]` — do NOT change version or features.

**Primary Dependencies**: gdext 0.5.5 (prebuilt API 4.6 — keep); Godot 4.7.2 stable at
`/usr/bin/godot.x86_64` (there is no `godot` on PATH). `.gdextension` with `reloadable = true`, debug
lib at `oxide_godot_core/target/debug/liboxide_godot.so`. Bindings generated in
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` — on 2026-09-15 it is the only
`godot-core-*/out` directory; if there is more than one, the valid one is `ls -dt .../godot-core-*/out | head -1`.

**Storage**: N/A

**Testing**: `cargo build` (debug profile, 0 warnings) + Godot headless (import + scene run).
No Rust unit tests in this phase (new infrastructure — v2). Visual validation by the user.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + Godot project
(`oxide-godot/`)

**Performance Goals**: parity with the original (do not optimize — Principle I)

**Constraints**: Principles I and II of constitution v1.3.0; method/property/RPC names
identical to GDScript; one commit per script; `.gd` + `.gd.uid` deleted in the same commit;
improvements only in `docs/v2-backlog.md`; exactly ONE bug fix (door) with the 4 requirements
of the clause.

**Scale/Scope**: 3 scripts, 274 lines of GDScript, 3 scenes edited (once each), 3 commits,
+2 existing Rust files touched only in visibility, 1 new document (`docs/upstream-bugs.md`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I — Three-Phase Port

| Rule | Status | Evidence |
|---|---|---|
| Phase declared in spec/plan/tasks | ✅ | spec.md "Phase: v1"; this plan "Phase: v1" |
| Direct translation, no remodeling of nodes/scenes | ✅ | The 3 scenes only get a `type` swap and removal of `script`/`ext_resource`; `DoorModel2` is **not** renamed |
| No abstraction/refactoring/optimization | ✅ | One module per script, no common module/trait/helper. Quirks preserved (spec Assumptions; research D16): `airborne_time = 100`; `jumping` zeroed by the player; double `explode` possible; `velocity` not zeroed on respawn; `crosshair` never used; `preload` → `load` at the point of use |
| Improvements → `docs/v2-backlog.md` in the same commit | ✅ | Candidates 10–14 in research.md §"Candidate v2 backlog", assigned per script; existing items 1 and 2 are not duplicated |
| **Bug fix: it is a bug, not an improvement** | ✅ | `door.gd:6` references `DoorModel/AnimationPlayer`; the scene has `DoorModel2` (`door.tscn:13`); result: `ERROR: Node not found` and a door that never opens — unambiguous intent contradicted by the result. Reproduced headless (research §E.2: exactly 1 error) |
| **Conservative** fix | ✅ | Only the node path changes (`DoorModel2/AnimationPlayer`); nothing renamed, extracted or "taken advantage of" (FR-031) |
| Requirement (a) — declared in the spec | ✅ | spec.md US3 + FR-030–FR-035 |
| Requirement (b) — isolated with `// upstream bug fix: ...` at the exact spot | ✅ planned | research D15: comment on the line above the `#[init(node = "DoorModel2/AnimationPlayer")]` in `src/door.rs` |
| Requirement (c) — mentioned in the commit | ✅ planned | quickstart.md §8, mandatory format of the port 3 commit |
| Requirement (d) — `docs/upstream-bugs.md` | ✅ planned | Created in the port 3 commit with header + entry #1 (defect, script/scene, fix, commit) — data-model.md §"Bug register" |
| No other fix | ✅ | FR-035; all other quirks stay (research D16) |

### Principle II — Verifiable Port Cycle

| Rule | Status | Evidence |
|---|---|---|
| One class per script, same base | ✅ | `Player: CharacterBody3D`, `Bullet: CharacterBody3D`, `Door: Area3D` |
| Binding by `type` swap in the `.tscn`; no bridge `.gd` | ✅ | "Scene editing" table below, lines verified on 2026-09-15 |
| Identical `#[func]`/RPC names | ✅ | `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma`, `explode`, `destroy`, `_on_door_body_entered` — [contracts/](contracts/) |
| Identical exported/replicated property names (verified in the `.tscn`) | ✅ | `player.tscn:17-30` replicates `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`, `.:current_animation` → `player_id` (`#[export]`), `current_animation` (`#[export]`), `motion` (`#[var]`) — research D2–D4, probe §E.1; `bullet.tscn:12` `.:global_transform` (base) |
| `cargo build` with no new warnings | ✅ | quickstart.md §1; baseline 0; research draft compiled with 0 |
| Headless validation (import + scene) | ✅ | quickstart.md §2–3; baseline = the 3 from `CLAUDE.md`; door: `Node not found` 1 → 0 |
| Commit per port with script + scene in the message | ✅ | quickstart.md §8 |
| `.gd` + `.gd.uid` deleted in the same commit | ✅ | "Scene editing" table; `grep` of the uid after removal |
| Bottom-up order | ✅ | `docs/port-order.md` items 6→7→8. Player first: consumes only classes already in Rust (`PlayerInputSynchronizer`, `CameraNoiseShake`); instantiates the bullet via **base API** (`CharacterBody3D`), so it does not depend on it. Bullet next: calls `hit` via duck typing (allowed). Door last: `is Player` requires `Player` in Rust |
| Rust does not call custom GDScript API | ✅ | Only dynamic calls: `has_method("hit")` + `rpc("hit")` in the bullet (duck typing from the original — allowed), `AnimationTree.set("parameters/...")` (base `Object` API), and the `Settings` exception |
| `Settings` exception | ✅ | Bullet `explode`: `get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>()` (research D14); v2 backlog item 1 already exists |
| `CLAUDE.md` catalog reflects the baseline | ✅ | The door's `Node not found` **was never** in the catalog (`CLAUDE.md:39-40` only lists the 3 import errors) → nothing to remove; finding recorded in the port 3 commit message (quickstart §8) |

**`pub(crate)` visibility in `player_input.rs`/`camera_noise_shake.rs`**: it is neither abstraction nor
refactoring — no line moves, no body changes; it is the minimum Rust requires for the typed
access mandated by FR-010/FR-011. Recorded in Complexity Tracking as a justified
non-violation.

**Gate result (pre-Phase 0)**: PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/002-v1-player-bullet-door/
├── plan.md              # This file
├── spec.md              # Specification (already validated, commit 108584e)
├── research.md          # Phase 0: gdext 0.5.5 signatures confirmed by compilation + headless probe
├── data-model.md        # Phase 1: state of Player (contract + internal), Bullet, Door, bug register
├── quickstart.md        # Phase 1: validation commands in order, door baseline, commit format
├── contracts/
│   ├── player.md        # class name, player_id/current_animation/motion, 5 RPCs, consumers
│   ├── bullet.md        # explode, destroy (method track), global_transform
│   └── door.md          # _on_door_body_entered, connection, corrected reference
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — not created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/                       # Cargo workspace (unchanged)
└── oxide_godot_lib/
    ├── Cargo.toml                      # unchanged
    └── src/
        ├── lib.rs                      # ExtensionLibrary + `mod` of each module (+3 `mod` lines)
        ├── debug_label.rs              # Milestone A (unchanged)
        ├── part_disappear.rs           # Milestone A (unchanged)
        ├── blast.rs                    # Milestone A (unchanged)
        ├── camera_noise_shake.rs       # Milestone A — port 1: `add_trauma` → pub(crate) (visibility only)
        ├── player_input.rs             # Milestone A — port 1: 6 fields + 3 methods → pub(crate) (visibility only)
        ├── player.rs                   # struct Player, base=CharacterBody3D  (port 1)  NEW
        ├── bullet.rs                   # struct Bullet, base=CharacterBody3D  (port 2)  NEW
        └── door.rs                     # struct Door,   base=Area3D           (port 3)  NEW

oxide-godot/                            # Godot project
├── player/
│   ├── player.tscn                     # port 1: root Player → type="Player"
│   ├── player.gd (+ .uid)              # port 1: DELETE
│   └── bullet/
│       ├── bullet.tscn                 # port 2: root Bullet → type="Bullet"
│       └── bullet.gd (+ .uid)          # port 2: DELETE
└── door/
    ├── door.tscn                       # port 3: root Door → type="Door" (DoorModel2 untouched)
    └── door.gd (+ .uid)                # port 3: DELETE

docs/
├── v2-backlog.md                       # items 10–14, one commit per origin
└── upstream-bugs.md                    # NEW in port 3: header + entry #1 (door)
```

**Structure Decision**: one Rust module per GDScript script, with no shared module and no
common helpers (Principle I). `lib.rs` only gains three `mod`. The name `Player` is mandatory
(`is Player` in `red_robot.gd:131,275,281` and `door.gd:10`); `Bullet` and `Door` are free and do not
collide with engine classes (verified in the bindings). The Milestone A modules consumed by the
Player change **only** the visibility keyword on the items listed in research.md D1, in the
port 1 commit.

Details of each class (fields, virtuals, RPCs, signatures) are in [research.md](research.md)
§"Map per script" and D1–D15; they are not repeated here.

## Scene editing (lines verified on 2026-09-15 — re-verify with `grep -n` before editing)

| Port | Scene | Node (line) | `type` before → after | Remove | Keep | Delete |
|---|---|---|---|---|---|---|
| 1 | `player/player.tscn` | root `Player` (l.333) | `CharacterBody3D` → `Player` | l.336 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://ctlx3bqglonsx" path="res://player/player.gd" id="1"]` | `collision_layer`/`collision_mask` (l.334–335); `ServerSynchronizer` (l.338–339, `replication_config`); `InputSynchronizer` (l.341–348); `BulletCache` (l.679–683) and `[editable path="BulletCache"]` | `player/player.gd`, `player/player.gd.uid` |
| 2 | `player/bullet/bullet.tscn` | root `Bullet` (l.481) | `CharacterBody3D` → `Bullet` | l.485 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://iybteh2g0be4" path="res://player/bullet/bullet.gd" id="1"]` | `transform`/`collision_layer`/`collision_mask` (l.482–484); `MultiplayerSynchronizer` (l.487–488); method track `destroy` (l.93–105) | `player/bullet/bullet.gd`, `.uid` |
| 3 | `door/door.tscn` | root `Door` (l.10) | `Area3D` → `Door` | l.11 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://7v3r683kok5s" path="res://door/door.gd" id="1"]` | `DoorModel2` (l.13) — **do NOT rename**; `AnimationPlayer` (l.25); `[connection ...]` (l.37); `[editable path="DoorModel2"]` (l.39) | `door/door.gd`, `door/door.gd.uid` |

After removing the `ext_resource` on l.3, the following lines shift by −1 (the root of `player.tscn`
becomes l.332, etc.). After each removal: `grep -rn "<uid>" oxide-godot/` must return empty
(pre-verified: each uid is referenced by exactly one scene). In the 3 ports the script id is
`"1"`, and no other `ExtResource("1")` exists in those scenes besides the `script` line.

Note: Godot writes `type="<registered class name>"`; when opening the scene in the editor after
the port, the editor may rewrite the `.tscn` (property order, `unique_id`) — such diffs
are acceptable as long as the `type` and the properties remain.

## Visual validation per script (SC-002/SC-009 — done by the user in the editor/game)

| Port | What to check in the game (compare with `../oxide_godot_origins/`) |
|---|---|
| 1 | Enter the level: spawn with immediate **landing sound** (quirk `airborne_time = 100`); walk in all directions with the model orientation following the camera; run/stop with smooth blend; jump (Jump sound, rising/falling animation) and land after > 0.5 s in the air (Land sound); aim (lateral strafe, aim follows the camera pitch) and shoot (barrel particles, sound, light shake, 0.4 s cooldown); get hit by the robot's laser (strong shake 13.0); fall off the map → reappears at the initial position. The bullet at this point is still GDScript |
| 2 | Shoot: visible blue bullet leaves the barrel, flies straight, explodes on hitting wall/floor (animation + light) and on hitting the robot (robot reacts to `hit`); a stray bullet explodes on its own after 5 s; the bullet disappears after the explosion; with `Shadow mapping` enabled in the settings, the explosion light casts a shadow; `BulletCache` invisible at spawn |
| 3 | Nothing visible in the game (orphan asset). Open `door/door.tscn` in the editor: root of type `Door`, no script, `DoorModel2` intact, no error in the Output. Optional (SC-009): **uncommitted** test scene with `door.tscn` + `player.tscn`; when walking up to the door, the `doorsimple_opening` animation plays once |

## Complexity Tracking

> Filled in to record a **justified non-violation** (the gate has no violations).

| Item | Why it is needed | Simpler alternative rejected because |
|---|---|---|
| `pub(crate)` on 6 fields + 3 methods of `player_input.rs` and on `add_trauma` of `camera_noise_shake.rs` (port 1 commit) | FR-010/FR-011 require **typed** access from the `Player` to `PlayerInputSynchronizer` and `CameraNoiseShake`; in Rust, items without `pub` are private to the module. Only the visibility keyword changes — no logic moved, extracted or rewritten (Principle I intact) | Access via `Variant` (`.get("motion")`, `.call("add_trauma")`) — violates FR-011 and the Principle II rule against dynamic access between classes already in Rust; moving the classes into a single module — unnecessary refactoring of already committed code |

## Constitution Check — post-design re-evaluation (Phase 1)

Re-evaluated after research.md, data-model.md, contracts/ and quickstart.md:

- No artifact introduces a common module, trait, helper or enum beyond the `Animations` that the
  original already has (Principle I). ✅
- The `Player` contract reproduces the class name, the 3 properties (`player_id`,
  `current_animation`, `motion`) and the 5 RPCs, verified against `player.gd`, `player.tscn:17-30`,
  `red_robot.gd:131,133,275,281`, `level.gd:118-119`, `bullet.gd:31-32`, `door.gd:10`. ✅
- The contracts of `Bullet` (`explode`, `destroy` — `bullet.tscn:104`; `.:global_transform`) and
  `Door` (`_on_door_body_entered` — `door.tscn:37`) reproduce the scene names. ✅
- The bug fix is confined to one line of `src/door.rs` (+ comment), to the port
  3 commit and to `docs/upstream-bugs.md`; `door.tscn` only swaps `type`/removes script (4/4 requirements). ✅
- All signatures of the mapping compiled with 0 warnings and the headless probe confirmed
  setter outside the tree, `#[var] motion` by name and enum with default 3 (research §E). ✅
- Decisions that depart from the text of the command input, all in favor of fidelity or as
  required by gdext: `col_c()`/`col_a()` for `basis.z`/`basis.x` (D9); intermediate
  `sound_effects` omitted in favor of full paths (D5); direct `try_cast` on `body`
  without `clone()` (D15); integer `0.to_variant()` in `aim/add_amount` (D8). ✅

**Gate result (post-Phase 1)**: PASS.
