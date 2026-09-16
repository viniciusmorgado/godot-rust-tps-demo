# Implementation Plan: Milestone D — forklift, level, menu and main (v1 raw port)

**Branch**: `main` (v1 lives on `main`; each port is an atomic commit that leaves the game playable) | **Date**: 2026-09-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-v1-level-menu-main/spec.md`

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.1). Direct translation; no abstraction,
refactoring or optimization. No bug fix planned — `docs/upstream-bugs.md` remains
with 2 entries.

## Summary

Port `flying_forklift.gd` (21 l.), `level.gd` (127), `menu.gd` (460) and `main.gd` (33) to
four gdext 0.5.5 classes — `FlyingForklift: CharacterBody3D` (base = the node's type, rule of
Principle II v1.3.1), `Level: Node3D`, `Menu: Node`, `Main: Node` — swapping the root `type` of
`flying_forklift.tscn`, `level.tscn`, `menu.tscn` and `main.tscn` and deleting `.gd` + `.gd.uid` in
the same commit, in the order forklift → level → menu → main. At the end only `menu/settings.gd` (autoload)
remains in GDScript, accessed dynamically via `/root/Settings` (Principle II exception).
Technical approach: signals `quit` and `replace_main_scene(PackedScene)` in the main `#[godot_api]`
block, connected **by name** by `main.gd`/`Main` (the original's duck typing); `Level`
consumes `EnemyRobot` (`signals().exploded()`) and `Player` (`set_player_id`) with typed access —
two `pub(crate)` visibility openings; `add_player`/`del_player` connected to
`peer_connected`/`peer_disconnected` via typed closure (gdext has no default parameter);
**gdext feature `experimental-threads` enabled in port 3** — the only way to obtain
`ResourceLoader::load_threaded_request/get_status/get`, which the menu requires for the loading bar
(user decision; `Cargo.toml` + a line in `CLAUDE.md`); engine enums written to the
`config_file` by their integers (`.ord()`); `Main` does `has_signal` + `connect` by name and
`call_deferred` by name, like the original. The whole mapping compiled in a draft with the feature
enabled (0 warnings) and the signals/methods/integers were checked in headless — [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext) in
`[workspace.dependencies]`. **Single configuration change** (port 3, decided by the user):
`godot = { version = "0.5.5", features = ["experimental-threads"] }` — version unchanged, no
other feature (research D1).

**Primary Dependencies**: gdext 0.5.5 (prebuilt 4.6 API — keep); Godot 4.7.2 stable at
`/usr/bin/godot.x86_64`. `.gdextension` with `reloadable = true`, debug lib at
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings generated in
`oxide_godot_core/target/debug/build/godot-core-*/out/` — **two** directories after the feature
(`aea5c50e7fda9d57` without, `4eba5d49e15a0d7e` with); `ls -dt … | head -1` returns the valid one.

**Storage**: N/A (the `config_file` belongs to `settings.gd`, out of scope).

**Testing**: `cargo build` (debug profile, 0 warnings) + Godot headless (import + `flying_forklift`,
`level`, `menu`, `main`). No unit tests. Visual validation by the user, including the full
settings menu.

**Target Platform**: Linux x86_64 desktop

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + Godot project (`oxide-godot/`)

**Performance Goals**: parity with the original (do not optimize — Principle I)

**Constraints**: Principles I and II of constitution v1.3.1; names of signals/methods/handlers and the
85 menu node paths identical to the GDScript; one commit per script; `.gd` + `.gd.uid` in the
same commit; improvements only in `docs/v2-backlog.md`; no bug fix (objective defect →
stop and declare in spec); `settings.gd` untouched.

**Scale/Scope**: 4 scripts, 641 lines of GDScript, 4 scenes edited (once each), 4 commits,
2 existing Rust files touched only in visibility, `Cargo.toml` (1 line) and `CLAUDE.md`
(operational lines) touched once.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I — Three-Phase Port

| Rule | Status | Evidence |
|---|---|---|
| Phase declared in spec/plan/tasks | ✅ | spec.md "Phase: v1 (constitution v1.3.1)"; this plan "Phase: v1" |
| Direct translation, without remodeling nodes/scenes | ✅ | The 4 scenes only get the `type` swap and the removal of `script`/`ext_resource`; the `main` node is not renamed |
| No abstraction/refactoring/optimization | ✅ | One module per script, no shared helper; the 15 `_make_button_group` calls and the ~30 `if/elif` chains of the menu stay as in the original (research D8–D9); quirks preserved (D11): `randomize()` ×3, `add_child(player)` without a readable name, `has_signal`/`connect`/`call_deferred` by name, `lightmap_gi` kept after `queue_free` |
| Improvements → `docs/v2-backlog.md` in the same commit | ✅ | Candidates 19–24 in research.md §"v2 backlog candidates", assigned per script |
| Bug fixes | N/A | No known objective defect; `docs/upstream-bugs.md` stays with 2 entries. The `replace_main_scene` signal parameter (declared without, emitted with 1) is a requirement of typed emission, documented in the spec — it is not a fix |

### Principle II — Verifiable Port Cycle (v1.3.1)

| Rule | Status | Evidence |
|---|---|---|
| One class per script, same **effective base of the node** | ✅ | `FlyingForklift: CharacterBody3D` (`extends Node3D` is an ancestor of the node `flying_forklift.tscn:36` — rule clarified in v1.3.1; declared in spec US1); `Level: Node3D`, `Menu: Node`, `Main: Node` |
| Class name without collision (engine + identifiers of the remaining `.gd` files) | ✅ | `CLAUDE.md` grep at `a866428`: `FlyingForklift`, `Level`, `Menu`, `Main` absent; `menu.gd` has `var main` (lowercase; and it is ported before `Main`) |
| Binding by `type` swap in the `.tscn`; no bridge `.gd` | ✅ | "Scene editing" table below |
| Identical `#[func]`/signal names | ✅ | `quit`, `replace_main_scene`, 9 handlers (10 connections), `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` — [contracts/](contracts/); probe §E.1 |
| Exported/replicated properties | N/A | None of the 4 scripts exports or replicates properties (`level.tscn` `MultiplayerSpawner` replicates **scenes**, base API) |
| `cargo build` without new warnings | ✅ | Baseline 0; draft with the feature compiled with 0 |
| Headless validation (import + scene) | ✅ | quickstart.md §2–3; baseline measured at `a866428` (§E.2), including the **intermittent error of the dummy renderer** in headless `main.tscn` (re-run rule; to be cataloged in `CLAUDE.md`) |
| Commit per port with script + scene in the message | ✅ | quickstart.md §8 (the port 3 one cites the feature and `CLAUDE.md`) |
| `.gd` + `.gd.uid` deleted in the same commit | ✅ | "Scene editing" table; `grep` of the uid after removal; **staging checked before any intermediate commit** (lesson from Milestone C) |
| Bottom-up order | ✅ | `docs/port-order.md` 11→14: forklift (leaf), level (consumes `EnemyRobot`/`Player`, emits `quit`), menu (emits `replace_main_scene`), main (consumes both signals by name) |
| Rust does not call custom GDScript API | ✅ | Only dynamic calls: `Settings` exception (`config_file` get/set, `call("apply_graphics_settings")`, `call("save_settings")`), `has_signal`/`connect` by name in `Main` and `call_deferred` by name (the original's duck typing/calls), nothing `AnimationTree`-like. `EnemyRobot`/`Player` typed. Final verification by grep in the quickstart |
| `Settings` exception | ✅ | Used by the 4 scripts; `settings.gd` is Milestone E; backlog item 1 |
| `CLAUDE.md` catalog reflects the baseline | ✅ | No error eliminated; the intermittent error of headless `main.tscn` **is added** (operational edit, port 3) |

**Justified non-violations** (Complexity Tracking): feature `experimental-threads`;
`pub(crate)` on `Player::set_player_id` and `EnemyRobot::exploded`; `add_player`/`del_player` via
closure instead of `#[func]` with default.

**Gate result (pre-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/004-v1-level-menu-main/
├── plan.md              # This file
├── spec.md              # Specification (commit dee6af2, updated to v1.3.1 at a866428)
├── research.md          # Phase 0: feature, signatures by compilation, probes, baseline (includes the intermittent error)
├── data-model.md        # Phase 1: 4 classes, flows, config_file options table
├── quickstart.md        # Phase 1: cycle, baseline, intermittent-error rule, format of the 4 commits
├── contracts/
│   ├── flying-forklift.md
│   ├── level.md         # quit; add_player/del_player via closure; pub(crate) visibilities
│   ├── menu.md          # replace_main_scene(PackedScene); 9 handlers/10 connections; table of the 85 OnReady
│   └── main.md          # go_to_main_menu/replace_main_scene/change_scene_to_packed
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — not created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/
├── Cargo.toml                          # port 3: godot = { version = "0.5.5", features = ["experimental-threads"] }
└── oxide_godot_lib/src/
    ├── lib.rs                          # +4 `mod` lines
    ├── player.rs                       # port 2: `set_player_id` → pub(crate) (visibility only)
    ├── red_robot.rs                    # port 2: `#[signal] fn exploded()` → pub(crate) (visibility only)
    ├── flying_forklift.rs              # struct FlyingForklift, base=CharacterBody3D (port 1)  NEW
    ├── level.rs                        # struct Level,          base=Node3D          (port 2)  NEW
    ├── menu.rs                         # struct Menu,           base=Node            (port 3)  NEW
    └── main_scene.rs                   # struct Main,           base=Node            (port 4)  NEW  (`mod main_scene;`)

oxide-godot/
├── level/forklift/flying_forklift.tscn (+ .gd/.uid DELETE)   # port 1
├── level/level.tscn                    (+ .gd/.uid DELETE)   # port 2
├── menu/menu.tscn                      (+ .gd/.uid DELETE)   # port 3
├── main/main.tscn                      (+ .gd/.uid DELETE)   # port 4
└── menu/settings.gd (+ .uid)           # UNTOUCHED — Milestone E

CLAUDE.md                               # port 3: feature line (Toolchain) + catalog of the intermittent error of headless main.tscn
docs/v2-backlog.md                      # items 19 (p1), 20–21 (p2), 22–23 (p3), 24 (p4)
```

**Structure Decision**: one module per script (Principle I). `main_scene.rs` instead of `main.rs`
so as not to suggest a binary; the class is called `Main` and the node remains `main`. The 85 `OnReady`
of the menu all live in the struct, with full paths (`UI/Settings/MSAA/4X` etc.) —
table in [contracts/menu.md](contracts/menu.md). Details of each class in [research.md](research.md)
D4–D10.

## Scene editing (lines checked on 2026-09-16 — re-check with `grep -n` before editing)

| Port | Scene | Node (line) | `type` before → after | Remove | Keep | Delete |
|---|---|---|---|---|---|---|
| 1 | `level/forklift/flying_forklift.tscn` (127 l.) | root `FlyingForklift` (l.36) | `CharacterBody3D` → `FlyingForklift` | l.37 `script = ExtResource("3")`; l.5 `[ext_resource type="Script" uid="uid://dcqnfagy55nrx" path="res://level/forklift/flying_forklift.gd" id="3"]` | `FlyingForkliftModel2` (l.39) and its `visible = false` (l.42, 45); `Collider` (l.47); `SpotLight3D` (l.114) | `flying_forklift.gd`, `.uid` |
| 2 | `level/level.tscn` (262 l.) | root `Level` (l.40) | `Node3D` → `Level` | l.41 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://ccxbls23ev7u3" path="res://level/level.gd" id="1"]` | `SpawnedNodes` (l.43), `RobotSpawnpoints` (l.45), `PlayerSpawnpoints` (l.59), `MultiplayerSpawner` (l.73–75), `WorldEnvironment` (l.85), `VoxelGI` (l.88), `ReflectionProbes` (l.94), forklift instances | `level.gd`, `.uid` |
| 3 | `menu/menu.tscn` (845 l.) | root `Menu` (l.103) | `Node` → `Menu` | l.104 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://4pwshyfo5i0d" path="res://menu/menu.gd" id="1"]` | the whole `UI/…` tree (85 paths), `WorldEnvironment`, `DoneTimer` (l.832–834), the **10** `[connection]`s (l.836–845) | `menu.gd`, `.uid` |
| 4 | `main/main.tscn` (6 l.) | root `main` (l.5) | `Node` → `Main` | l.6 `script = ExtResource("1")`; l.3 `[ext_resource type="Script" uid="uid://chrcwbh6kvb7i" path="res://main/main.gd" id="1"]` | `name="main"` (do not rename) | `main.gd`, `.uid` |

Each scene is edited exactly once; after removing the `ext_resource` the following lines shift by −1
(the forklift root → 35; the level's → 39; the menu's → 102; the main's → 4). Checked: `ExtResource("3")`
occurs 1 time in `flying_forklift.tscn` and `ExtResource("1")` 1 time in each of the other three
(no other resource uses those ids). After each removal: `grep -rn "<uid>" oxide-godot/ | grep -v /.godot/`
empty.

Also in port 3: `oxide_godot_core/Cargo.toml` l.7 `godot = "0.5.5"` →
`godot = { version = "0.5.5", features = ["experimental-threads"] }`; `CLAUDE.md` §Toolchain: one
line "feature `experimental-threads` enabled since Milestone D — without it gdext does not generate
`ResourceLoader::load_threaded_*` (godot-codegen `special_cases.rs:83-86`), required by the menu"; and
§Work cycle: the intermittent error of headless `main.tscn` (research §E.2) in the catalog of
pre-existing errors.

## Visual validation per script (SC-002 — done by the user in the editor/game)

| Port | What to check in the game (compare with `../oxide_godot_origins/`) |
|---|---|
| 1 | Enter the level: the flying forklifts appear with models/colors that vary among them (re-pick on every Play); with `Shadow mapping` off in Settings, the forklift's headlight casts no shadow; the player and the robots keep colliding with them. In the editor: root `FlyingForklift` (type derived from `CharacterBody3D`), `Collider` intact |
| 2 | Play → the level loads and applies the settings; change `GI type`/`GI quality` in Settings and re-enter: SDFGI / VoxelGI / lightmap (with `ReflectionProbes`) reflect the option; 4 robots spawn, respawn 15 s after dying; the player spawns at a random point with `player_id` 1 (landing sound of the quirk); **ESC** releases the mouse and returns to the menu. In the editor: root `Level`, `MultiplayerSpawner` intact |
| 3 | Menu: Play → the loading bar progresses to 100 % and the level opens in ~0.5 s; Settings → each row shows the current value pressed; change options and Apply → applies immediately and persists (`user://settings.ini`; reopening Settings shows the new values; restarting the game keeps them); Cancel/Back discard; Play Online shows Host/Connect (Host starts the game as server); Quit closes; MetalFX buttons hidden (Linux); keyboard navigates with focus on Play/Cancel as in the original |
| 4 | Boot straight into the menu (saved window mode applied); Play → level; ESC → menu (offline peer recreated); Play again → level again (previous scene freed, no duplication); everything indistinguishable from the original. In the editor: `main.tscn` with node `main` of type `Main` |

## Complexity Tracking

> Filled in to record **justified non-violations** (the gate has no violations).

| Item | Why it is needed | Simpler alternative rejected because |
|---|---|---|
| Feature `experimental-threads` in `Cargo.toml` (port 3) — crate configuration change, decided by the user | `menu.gd:130,158,163` uses `ResourceLoader.load_threaded_request/get_status/get`; gdext's codegen omits them without the feature (`special_cases.rs:83-86`). It is the only way to reproduce threaded loading with a typed API | `ResourceLoader::singleton().call("load_threaded_request", …)` — dynamic call to the engine, untyped (against the spirit of Principle II); loading synchronously — would change the behavior (no progress bar) |
| `pub(crate)` on `Player::set_player_id` and on `EnemyRobot::exploded` (`#[signal]`) — port 2 commit | FR-025: `player.bind_mut().set_player_id(id)` and `robot.signals().exploded().connect_other(..)` typed; the signal accessor inherits the `fn`'s visibility (error E0624 in the draft). Only the visibility keyword changes | `player.set("player_id", …)` / `robot.connect("exploded", …)` — dynamic between Rust classes |
| `add_player`/`del_player` private, connected to `peer_connected`/`peer_disconnected` via typed closure | gdext has no default parameter in `#[func]`; the original connects the 2-parameter method (1 default) to a 1-argument signal. The closure `|this, id| this.add_player(id as i32, None)` reproduces the effect; no script calls the methods by name (spec FR-008) | Two `#[func]`s (`add_player` and `add_player_at`) — a new name does not exist in the original; exposing `add_player(id, spawn_point)` with 2 args — the connection by name would fail (arity) |

## Constitution Check — post-design re-evaluation (Phase 1)

Re-evaluated after research.md, data-model.md, contracts/ and quickstart.md:

- No artifact introduces a common module, trait or helper; the menu keeps the 15 calls and the
  `if/elif` chains of the original. ✅
- Contracts reproduce the verified names: `quit` (`main.gd:30-31`), `replace_main_scene`
  with 1 parameter (`main.gd:32-33`, `menu.gd:163`), 9 handlers/10 connections (`menu.tscn:836-845`),
  3 `Main` methods (one of them the target of `call_deferred` by name), 85 menu paths derived
  mechanically from `menu.gd:11-104`. ✅
- The whole mapping compiled with the feature enabled and 0 warnings; probes confirmed the forklift's
  base, signals (arity and argument type) connectable by name from GDScript,
  exposed/private methods and the integers of the engine enums. ✅
- The measured baseline includes an **intermittent engine error** in headless `main.tscn` (research
  §E.2) — the re-run rule is in the quickstart and the `CLAUDE.md` catalog is updated in
  port 3 (Principle II, pre-existing errors cataloged). ✅
- Decisions that depart from the text of the command input, imposed by the compiler or by
  fidelity: `GString::from("…")` in string comparisons (D3); `pub(crate)` also on
  `EnemyRobot::exploded` (D2); `EnvironmentSdfgiRayCount::COUNT_96/32` (without `RAY_`) (D5);
  `lightmap_gi` kept `Some` after `queue_free` (D5); `set_name(&id.to_string())` (D6);
  `progress.at(0)` from a `VarArray` (D8); `ResourceLoader::singleton().load(..)` in `Main`
  as in the original, not the global `load()` (D10). ✅

**Gate result (post-Phase 1)**: PASS.
