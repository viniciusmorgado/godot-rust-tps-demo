# Implementation Plan: Milestone C — enemy: part and red robot (v1 raw port)

**Branch**: `main` (v1 lives on `main`; each port is an atomic commit that leaves the game playable) | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-v1-enemy/spec.md`

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.0). Direct translation; no abstraction,
refactoring or optimization. No bug fix planned — `docs/upstream-bugs.md` remains
with 1 entry.

## Summary

Port `part.gd` (57 lines, `RigidBody3D`, no scene of its own — 3 nodes of `red_robot.tscn`) and
`red_robot.gd` (283 lines, `CharacterBody3D`, root of `red_robot.tscn`) to two gdext 0.5.5
classes, swapping the `type` of the 3 part nodes (port 1) and of the root (port 2) in the same 11,053-line
scene, deleting `.gd` + `.gd.uid` in the same commit — in the order part → red_robot, one commit per
script. The 5 remaining scripts stay intact and keep finding `exploded` (`level.gd:99`)
and `hit` (bullet). Technical approach: `fade_value` with `#[var(set = set_fade_value)]` applying to the
`next_pass` shader (setter also triggered by the replication's `set_indexed` — confirmed);
material duplication via `Gd::duplicate_resource()` (the generated `duplicate()` is deprecated);
`await` → `create_timer(..).signals().timeout().connect_other(..)`; `#[signal] exploded` in the main
`#[godot_api]` block, emitted via `signals().exploded().emit()` and connected by name by
`level.gd`; derived `State` enum (`via = i64`); the inverse transformation `Vector3 * Transform3D`
translated as `basis.transposed() * (v − origin)` (`affine_inverse` diverges with scale —
confirmed numerically); raycasts with **effective** exclusion by the robot's RID; `collider ==
player` by `instance_id()`; typed access to `Part::explode` (`pub(crate)` since port 1) and to
`Player::add_camera_shake_trauma` (`pub(crate)` in the robot's commit); `Blast` and `PartDisappear`
instantiated through base API. Everything compiled in a draft (0 warnings) and exercised headless —
[research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext), already in
`[workspace.dependencies]` — do NOT change version or features.

**Primary Dependencies**: gdext 0.5.5 (prebuilt API 4.6 — keep); Godot 4.7.2 stable at
`/usr/bin/godot.x86_64`. `.gdextension` with `reloadable = true`, debug lib at
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings generated in
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (single directory on
2026-09-15; if there is more than one, `ls -dt .../godot-core-*/out | head -1`).

**Storage**: N/A

**Testing**: `cargo build` (debug profile, 0 warnings) + Godot headless (import + `red_robot.tscn`
+ `level.tscn`). No Rust unit tests in this phase. Visual validation by the user.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + Godot project
(`oxide-godot/`)

**Performance Goals**: parity with the original (do not optimize — Principle I)

**Constraints**: Principles I and II of constitution v1.3.0; method/property/RPC/signal names
identical to the GDScript; one commit per script; `.gd` + `.gd.uid` deleted in the same commit;
improvements only in `docs/v2-backlog.md`; no bug fix (if an objective defect appears, stop and
declare it in the spec before any commit).

**Scale/Scope**: 2 scripts, 340 lines of GDScript, 1 scene edited twice (4 nodes), 2
commits, +1 existing Rust file touched only in visibility (`player.rs`).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I — Three-Phase Port

| Rule | Status | Evidence |
|---|---|---|
| Phase declared in spec/plan/tasks | ✅ | spec.md "Phase: v1"; this plan "Phase: v1" |
| Direct translation, without remodeling nodes/scenes | ✅ | `red_robot.tscn` only receives a `type` swap on 4 nodes and removal of `script`/`ext_resource`; no node renamed or moved |
| No abstraction/refactoring/optimization | ✅ | One module per script; the original's 3 repeated raycasts stay **inline** (research D12 — helper discarded for being an extraction); quirks preserved (research D15): `body.name == "Target"`, `pass # Kill.`, `player: Node3D`, 10 s `await` in `hit`, puff on the part's parent, non-replicated exports |
| Improvements → `docs/v2-backlog.md` in the same commit | ✅ | Candidates 15–18 in research.md §"Candidate v2 backlog", assigned per script |
| Bug fixes | N/A | No objective defect known; `docs/upstream-bugs.md` stays with 1 entry. If one appears: stop, declare in the spec (requirement a), and only then comment/commit/record (b–d) |

### Principle II — Verifiable Port Cycle

| Rule | Status | Evidence |
|---|---|---|
| One class per script, same base | ✅ | `Part: RigidBody3D`, `EnemyRobot: CharacterBody3D` |
| Binding by `type` swap in the `.tscn`; no bridge `.gd` | ✅ | Table "Scene edits" below (port 1: 3 nodes + `ext_resource id="24"`; port 2: root + `id="1"`) |
| Identical `#[func]`/RPC/signal names | ✅ | `explode`, `destroy`; `exploded`, `hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered`, `_on_area_body_exited` — [contracts/](contracts/); probe §E.2 confirms `has_method`/`has_signal` |
| Identical exported/replicated property names (checked in the `.tscn`) | ✅ | Part: `red_robot.tscn:10419` `.:fade_value` → `#[export] #[var(set)]` (setter triggered by `set_indexed`, §E.1). Robot: l.33/36/39/42 `health`/`state`/`target_position`/`dead` → `#[export]`; `aim_preparing`/`test_shoot` exported and not replicated, as in the original |
| `cargo build` with no new warnings | ✅ | Baseline 0; draft compiled with 0 (after swapping the deprecated `duplicate()` for `duplicate_resource()`) |
| Headless validation (import + scene) | ✅ | quickstart.md §2–3; baseline measured at `4bb8f7f` (§E.3) |
| Commit per port with script + scene in the message | ✅ | quickstart.md §8 |
| `.gd` + `.gd.uid` deleted in the same commit | ✅ | Table "Scene edits"; `grep` of the uid after removal |
| Bottom-up order | ✅ | `docs/port-order.md` items 9 → 10. Part first: consumes only `PartDisappear` (Milestone A) through base API. Robot afterwards: consumes `Part` (typed), `Player` (typed), `Blast` (base API) — all already in Rust |
| Rust does not call custom GDScript API | ✅ | The robot consumes no remaining GDScript script; the only dynamic calls are `AnimationTree.set/get("parameters/…")`, `col.get("position"/"collider")` (raycast Dictionary) — base API. No `.call(` (final verification in the quickstart) |
| `Settings` exception | N/A | Not used by either script |
| `CLAUDE.md` catalogue reflects the baseline | ✅ | No error eliminated; `CLAUDE.md` untouched (final verification) |

**`pub(crate)` visibility**: `Part::explode` (`#[func] pub(crate)`, port 1) and
`Player::add_camera_shake_trauma` (port 2, only the visibility keyword in `player.rs`) —
justified non-violation, Milestone B precedent; see Complexity Tracking.

**Gate result (pre-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/003-v1-enemy/
├── plan.md              # This file
├── spec.md              # Specification (commit 4bb8f7f)
├── research.md          # Phase 0: signatures confirmed by compilation + headless probes + baseline
├── data-model.md        # Phase 1: Part and EnemyRobot (contract, internal, state machine)
├── quickstart.md        # Phase 1: validation commands, baseline, commit format
├── contracts/
│   ├── part.md          # explode, destroy, 4 exports; consumer red_robot.gd:96-98; replication l.10419
│   └── red-robot.md     # exploded, hit, play_shoot, shoot_check, resume_approach, _on_area_body_*, 6 exports
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — not created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/oxide_godot_lib/src/
├── lib.rs                      # +2 `mod` lines
├── debug_label.rs, part_disappear.rs, blast.rs, camera_noise_shake.rs, player_input.rs   # unchanged
├── player.rs                   # port 2: `add_camera_shake_trauma` → pub(crate) (visibility only)
├── bullet.rs, door.rs          # unchanged
├── part.rs                     # struct Part,     base=RigidBody3D      (port 1)  NEW
└── red_robot.rs                # struct EnemyRobot, base=CharacterBody3D (port 2)  NEW

oxide-godot/enemies/red_robot/
├── red_robot.tscn              # port 1: Death/PartShield1|2, Death/PartHead → type="Part"; port 2: root → type="EnemyRobot"
├── red_robot.gd (+ .uid)       # port 2: DELETE
└── parts/part.gd (+ .uid)      # port 1: DELETE

docs/v2-backlog.md              # items 15 (port 1), 16–18 (port 2)
```

**Structure Decision**: one Rust module per script, no shared module (Principle I).
`Part` and `EnemyRobot` are free names (no `class_name`) — `RedRobot` was discarded in T024 for colliding with `const RedRobot` in `level.gd:6` ("shadows a native class"; see research D1), checked without collision in the bindings.
`Part::explode` is born `#[func] pub(crate)` in port 1 so that the robot's commit does not touch
`part.rs`. Details of each class in [research.md](research.md) §"Map per script".

## Scene edits (`enemies/red_robot/red_robot.tscn`, 11,053 lines; lines checked on 2026-09-15 — re-check with `grep -n` before editing)

| Port | Node (line) | `type` before → after | Remove | Keep | Delete |
|---|---|---|---|---|---|
| 1 | `Death/PartShield1` (l.10833) | `RigidBody3D` → `Part` | l.10841 `script = ExtResource("24")` | l.10834–10840: `transform`, `collision_layer = 3`, `collision_mask = 3`, `mass = 2000.0`, `physics_material_override`, `freeze = true`, `angular_damp = 0.3`; children l.10843+ (`MultiplayerSynchronizer` with `replication_config` + `public_visibility = false`, `Model`, `Col1`, `Col2`) | — |
| 1 | `Death/PartShield2` (l.10885) | `RigidBody3D` → `Part` | l.10892 `script = ExtResource("24")` | same (l.10886–10891; children l.10894+) | — |
| 1 | `Death/PartHead` (l.10936) | `RigidBody3D` → `Part` | l.10944 `script = ExtResource("24")` | same (l.10937–10943; children l.10946+) | — |
| 1 | — | — | l.26 `[ext_resource type="Script" uid="uid://c3vo80hyj6w6c" path="res://enemies/red_robot/parts/part.gd" id="24"]` | all the other `ext_resource` | `parts/part.gd`, `parts/part.gd.uid` |
| 2 | root `RedRobot` (l.10584 → **10583** after port 1: only −1 from the `ext_resource` l.26 — the 3 removed `script` lines are below the root and do not shift it) | `CharacterBody3D` → `EnemyRobot` | `script = ExtResource("1")` (l.10587 → 10583); l.3 `[ext_resource type="Script" uid="uid://bf14mo0lrrvjl" path="res://enemies/red_robot/red_robot.gd" id="1"]` | `collision_layer/mask = 3`; `MultiplayerSynchronizer` (`replication_config`); `AnimationTree`; `ShootAnimation` (method tracks); `PlayerDetectionArea`; the 2 `[connection …]` (end of file) | `red_robot.gd`, `red_robot.gd.uid` |

After port 1, **all** lines ≥ 26 shift −1 and those ≥ 10841 shift up to −4; after port 2,
those ≥ 3 shift −1 again. Edit via `sed` only after `grep -n` in the same session. Checked:
`ExtResource("24")` occurs exactly 3 times and `ExtResource("1")` exactly 1 time in the scene
(no other resource uses those ids). After each removal: `grep -rn "<uid>" oxide-godot/ | grep -v /.godot/`
must return empty.

## Visual validation per script (SC-002 — done by the user in the editor/game)

| Port | What to check in the game (compare with `../oxide_godot_origins/`) |
|---|---|
| 1 | Kill a robot (5 shots): the two shields and the head come loose upward with random rotation, fall and bounce with physics, stay 3 to 6 s on the ground, vanish in a fade (~0.3 s, with the `emission_cutout` glow) and end with the puff (Milestone A). Each part vanishes at its own time, without affecting the others. In the editor: the 3 nodes with type `Part`, no script, `freeze` checked |
| 2 | Robot standing still far away (IDLE, idle animation); when approaching, it turns (`turn_left/right`) and walks (`walk`) until facing; ~6 s facing → aims (red laser appears and is clipped against the scenery/player, aim animation follows the player); ~1 s → shoots ("shoot" animation, laser sparks, impact at the point, **strong shake** if it hits); goes back to approaching. Each shot received: damage animation (one of three) + sound; 5th shot: death as in port 1 + sparks + explosion sound; 10 s later the robot disappears; 15 s after the death, another robot spawns at the same point (level). Leaving the detection area → robot goes back to IDLE |

## Complexity Tracking

> Filled in to record **justified non-violations** (the gate has no violations).

| Item | Why it is necessary | Simpler alternative rejected because |
|---|---|---|
| `Part::explode` as `#[func] pub(crate)` (port 1) | `#[func]` because `red_robot.gd` calls it by name until port 2 (Principle II, names preserved); `pub(crate)` because FR-018 requires typed access from the robot in port 2 — declared already in port 1 so that the robot's commit does not edit `part.rs` | Only `#[func]` and `bind_mut().call("explode")` in port 2 — dynamic access between Rust classes (forbidden) |
| `Player::add_camera_shake_trauma` → `pub(crate)` in `player.rs` (robot's commit) | FR-018: `player.bind_mut().add_camera_shake_trauma(13.0)` typed after `try_cast::<Player>`; only the visibility keyword changes (Milestone B precedent) | `player.call("add_camera_shake_trauma", …)` or `rpc` — dynamic; the original calls it directly |

## Constitution Check — post-design re-evaluation (Phase 1)

Re-evaluated after research.md, data-model.md, contracts/ and quickstart.md:

- No artifact introduces a common module, trait or helper: the repeated raycasts stay inline
  (research D12 discarded the helper as refactoring). ✅
- The contracts reproduce all the names checked in the scene and in the consumers: part
  (`explode` — `red_robot.gd:96-98`; `fade_value` — `red_robot.tscn:10419`), robot (`exploded` —
  `level.gd:99`; `hit` — `bullet.rs`; method tracks l.10296-10299; connections l.11050-11051;
  replication l.30-42). ✅
- All signatures compiled with 0 warnings; probes confirmed setter via `set_indexed`,
  material/`next_pass` duplication, signal by name, exports/defaults, and the identity
  `v * t = transposed * (v − origin)`. ✅
- Decisions that depart from the text of the command's input, imposed by the compiler or by
  fidelity: `duplicate_resource()` instead of `duplicate()` (deprecated, warning); `create_timer`
  without `unwrap` (returns `Gd` directly); `VarDictionary` for the raycast result; `StringName::from("Target")`
  in the name comparison; inline raycasts instead of helpers. ✅
- No bug fixed; `docs/upstream-bugs.md` and `CLAUDE.md` untouched. ✅

**Gate result (post-Phase 1)**: PASS.
