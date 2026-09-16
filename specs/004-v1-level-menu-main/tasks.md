# Tasks: Milestone D — forklift, level, menu and main (v1 raw port)

**Input**: Design documents from `/specs/004-v1-level-menu-main/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D11, §E), data-model.md, contracts/, quickstart.md (all approved, commit `fe4975f`)

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.1). No task may introduce
abstraction, refactoring, optimization, unit tests or infrastructure. If something like that seems
necessary, it becomes an entry in `docs/v2-backlog.md`, not a task. **No bug fix** is
planned: objective upstream defect → STOP and report (the clause requires a spec before any
commit); `docs/upstream-bugs.md` stays with 2 entries. **One configuration change** of the crate,
decided by the user: the gdext feature `experimental-threads`, in US3 (research D1).

**Tests**: there are no automated tests in this phase. The validation of each story is the
quickstart cycle (build → headless import → headless scene → mechanical checks → contract →
user's visual validation).

**Organization**: one phase per user story, in the mandatory order US1 → US2 → US3 → US4 (spec
FR-028; `docs/port-order.md` items 11 → 14). The stories are **not** parallelizable among themselves: each
one ends with its own commit on `main` and the next one starts from the clean tree; US4 (`Main`)
connects by name the signals that US2/US3 register; the original `main.gd` keeps doing that until
port 4, so the game remains playable after each commit.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task) — rare
  in this milestone, because the build depends on the module, the scene depends on the build, validation depends on the scene.
- **[Story]**: US1..US4 (spec.md)
- Paths relative to the repository root (`oxide-godot/` = Godot project;
  `oxide_godot_core/oxide_godot_lib/src/` = Rust crate).

## Path Conventions

```
oxide_godot_core/Cargo.toml                        US3: godot = { version = "0.5.5", features = ["experimental-threads"] } (only change)
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` of each module (only that)
oxide_godot_core/oxide_godot_lib/src/<module>.rs   one class per script (research.md §"Map per script")
oxide_godot_core/oxide_godot_lib/src/player.rs, red_robot.rs   Milestones B/C — only visibility changes (US2)
oxide-godot/level/forklift/flying_forklift.tscn (+ .gd/.uid)   US1
oxide-godot/level/level.tscn (+ .gd/.uid)                      US2
oxide-godot/menu/menu.tscn (+ .gd/.uid)                        US3
oxide-godot/main/main.tscn (+ .gd/.uid)                        US4
oxide-godot/menu/settings.gd (+ .uid)              NEVER changes (Milestone E)
CLAUDE.md                                          US3: feature line + catalog of the intermittent error of headless main.tscn (operational edit)
docs/v2-backlog.md                                 items 19–24
docs/upstream-bugs.md                              does NOT change (2 entries)
../oxide_godot_origins/                            untouched reference of the original GDScript
```

Closed decisions (apply to all stories — instruction, not option; details in research.md):

- **Names**: `FlyingForklift` (base **`CharacterBody3D`** — the node's type, Principle II v1.3.1),
  `Level` (`Node3D`), `Menu` (`Node`), `Main` (`Node`, file `src/main_scene.rs`, `mod main_scene;`).
  Re-check collisions with the `CLAUDE.md` grep before each port; collision → stop.
- **Strings**: compare with `GString::from("metal")` / `GString::from("headless")` (D3 — `"x".into()` is ambiguous).
- **Settings** (Principle II exception, 3 forms): `self.base().get_node_as::<Node>("/root/Settings")` →
  `.get("config_file").to::<Gd<ConfigFile>>()`; `.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()])`;
  `.call("save_settings", &[])`.
- **US2**: dedicated visibility task BEFORE the build — `pub(crate) fn set_player_id` (`player.rs`)
  and `pub(crate) fn exploded();` (`red_robot.rs`), 2 −/+ pairs checked via `git diff --stat`,
  cited in the commit message (D2). `add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)`
  and `del_player(&mut self, id: i32)` **private**, connected via typed closure
  (`|this, id: i64| this.add_player(id as i32, None)`); `spawned_nodes.add_child(&player)` **without**
  `force_readable_name` (quirk `level.gd:121`); `spawn_robot` typed `Gd<EnemyRobot>` with
  `add_child_ex(&robot).force_readable_name(true).done()`; `lightmap_gi` kept `Some` after
  `queue_free` (**without** `take()`); GI via local integer constants; `EnvironmentSdfgiRayCount::COUNT_96/COUNT_32`,
  `VoxelGiQuality::HIGH/LOW`; `LightmapGi::new_alloc()` + `set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"))`;
  `#[signal] fn quit();` in the main block (D5–D6).
- **US3**: dedicated task for `Cargo.toml` BEFORE the build (regenerates bindings — note the new hash) and
  dedicated task for `CLAUDE.md`; both cited in the commit message. `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);`;
  `#[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>`; 85 `OnReady`
  exactly as the `contracts/menu.md` table; `_make_button_group` private, 15 explicit
  calls; `load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done()`,
  `load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done()` (`progress: VarArray`),
  `load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>()`; engine enums via `.ord() as i64`
  (D9); `godot::global::is_equal_approx`; `call_deferred("_on_host_pressed", &[])` in headless;
  9 `#[func]` handlers (10 connections) (D7–D9).
- **US4**: `get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false)`;
  `Engine::singleton().set_max_fps(60)` in headless; `window::Mode::from_ord(x as i32)`;
  `go_to_main_menu`/`replace_main_scene`/`change_scene_to_packed` **all `#[func]`**;
  `call_deferred("change_scene_to_packed", &[resource.to_variant()])` by name; `has_signal` +
  `connect(name, &Callable::from_object_method(&self.to_gd(), "…"))` — the original's duck typing,
  do **not** replace with `try_cast`; `ResourceLoader::singleton().load("res://menu/menu.tscn")` (faithful
  to `ResourceLoader.load`); `remove_child` before `queue_free`, in order (D10).
- One commit per script on `main`, message in the quickstart §8 format; fix after checkpoint =
  commit `Fix port …`. **Before any commit other than the port's, check that there are no
  `.gd` deletions in staging** (lesson from Milestone C).

---

## Phase 1: Setup

**Purpose**: record the baseline against which each port is compared — including the rule of the
intermittent error of headless `main.tscn`. No infrastructure (Principle I).

- [x] T001 Confirm preconditions: no Godot editor open (`pgrep -a godot` empty — if there is one, warn the user and wait, never kill); `git status --short` empty on `main`; note `git rev-parse --short HEAD` (expected `fe4975f` or later without "Port …" commits). The base commit `a866428` used by `quickstart.md`/T054 remains valid; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 5; `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = "0.5.5"` (no feature yet)
- [x] T002 Record the build baseline: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → expected `0`; note it in `specs/004-v1-level-menu-main/quickstart.md` §"Baseline" if it differs
- [x] T003 Record the import baseline: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirm `grep -n 'Initialize godot-rust' /tmp/import.log` (line 1) and `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` empty (or only the 3 upstream errors from `CLAUDE.md`)
- [x] T004 [P] Confirm the bindings directories: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out` → expected two (`4eba5d49e15a0d7e` with the feature, from the plan's test build; `aea5c50e7fda9d57` without); note it. Check name collisions with the `CLAUDE.md` grep: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | awk '{print $NF}' | sort -u | grep -x 'FlyingForklift\|Level\|Menu\|Main'` → empty; `ls $(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1)/classes/ | grep -x 'flying_forklift.rs\|level.rs\|menu.rs\|main.rs'` → empty
- [x] T005 Measure the baseline of the 4 scenes (quickstart §"Baseline"): `cd oxide-godot && for s in level/forklift/flying_forklift.tscn level/level.tscn menu/menu.tscn main/main.tscn; do timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/base_$(basename $s .tscn).log 2>&1; done`; expected exit 124 in all, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class'` empty, WARNINGs = 1/2/1/2 (HDR; Physics interpolation in the scenes with Player). **`main.tscn`**: if exactly the 5 dummy-renderer lines appear (`Initializing already initialized RID`, `Parameter "mem" is null.`, 3× `Parameter "m" is null.`), run again — if they disappear = baseline (intermittent engine error, research §E.2); any other `ERROR` line → stop and report. Confirm `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2 and `grep -c '^| [0-9]' docs/v2-backlog.md` = 18

**Checkpoint**: baseline known — 0 warnings, extension loads, 0 `ERROR` on import and in the 4 scenes (apart from the intermittent one of `main.tscn`).

---

## Phase 2: Foundational

**Does not exist in this milestone.** The four ports are strictly sequential and the only shared
edit is the `mod <module>;` line in `lib.rs`, done inside each story. The `pub(crate)`
visibility openings (`Player::set_player_id`, `EnemyRobot::exploded`) **belong to US2**;
the feature `experimental-threads` **belongs to US3** (it is the menu that requires it). No common module,
helper, trait or shared constant may be created (Principle I).

---

## Phase 3: User Story 1 — Flying forklift ported (Priority: P1) 🎯 MVP

**Goal**: `level/forklift/flying_forklift.gd` (21 l., `extends Node3D`) → `FlyingForklift: CharacterBody3D`
(root node type, `flying_forklift.tscn:36` — rule of Principle II v1.3.1, declared in spec
US1). Headlight shadow according to `shadow_mapping`; model pick.

**Independent Test**: `flying_forklift.tscn` and `level.tscn` headless without `ERROR`; in the game,
forklifts with varied models and headlight without shadow when `shadow_mapping` is off.

- [x] T006 [US1] Create `oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs` translating line by line `oxide-godot/level/forklift/flying_forklift.gd`: `use godot::classes::{CharacterBody3D, ConfigFile, ICharacterBody3D, Node, Node3D, SpotLight3D}; use godot::global::{randf, randomize}; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct FlyingForklift { base: Base<CharacterBody3D>, #[init(node = "SpotLight3D")] spot_light: OnReady<Gd<SpotLight3D>> }`; `#[godot_api] impl ICharacterBody3D for FlyingForklift { fn ready(&mut self) { let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>(); if !config_file.get_value("rendering", "shadow_mapping").to::<bool>() { self.spot_light.set_shadow(false); } // Randomize the forklift model. // We have 3 models, may as well use them. randomize(); let children = self.base().get_child(0).unwrap().get_children(); let child_count = children.len(); let which_enabled = (randf() * child_count as f64).floor() as usize; for (i, child) in children.iter_shared().enumerate() { child.cast::<Node3D>().set_visible(i == which_enabled); } } }`. Keep the original's comment `// TODO: We can maybe implement func hit():`. No `#[func]` (research D4)
- [x] T007 [US1] Add `mod flying_forklift;` to `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod red_robot;`; nothing else changes in lib.rs)
- [x] T008 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. In case of error, consult research.md D3–D4 and the bindings before improvising; do not change `Cargo.toml` in this story
- [x] T009 [US1] Edit `oxide-godot/level/forklift/flying_forklift.tscn` as text (re-check with `grep -n 'name="FlyingForklift" type=\|^script = ExtResource("3")\|flying_forklift.gd' oxide-godot/level/forklift/flying_forklift.tscn` → expected l.36, l.37, l.5): at the root replace `type="CharacterBody3D"` with `type="FlyingForklift"`; remove `script = ExtResource("3")`; remove `[ext_resource type="Script" uid="uid://dcqnfagy55nrx" path="res://level/forklift/flying_forklift.gd" id="3"]`. **KEEP** `FlyingForkliftModel2` (l.39) and the models' `visible = false`, `Collider` (l.47), `SpotLight3D` (l.114) and all the other `ext_resource`s. Expected diff: `4 +---` (plan.md "Scene editing", port 1)
- [x] T010 [US1] Delete `oxide-godot/level/forklift/flying_forklift.gd` and `oxide-godot/level/forklift/flying_forklift.gd.uid` (`git rm`); verify `grep -rn 'uid://dcqnfagy55nrx' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'flying_forklift.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T011 [US1] Headless validation (quickstart §2–3; no editor open): import with `Initialize godot-rust` and without new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . level/forklift/flying_forklift.tscn 2>&1 | tee /tmp/run.log` and then `level/level.tscn` → exit 124; regression grep (including `shadows a native class`) empty in both (only the baseline WARNINGs). The isolated forklift and the level's instances run `ready` (dynamic Settings, `get_child(0).get_children()`, pick)
- [x] T012 [US1] Mechanical checks and contract (`contracts/flying-forklift.md`): `grep -n 'name="FlyingForklift" type=' oxide-godot/level/forklift/flying_forklift.tscn` = `type="FlyingForklift"`; `grep -c 'ExtResource("3")' …` = 0; `grep -n 'name="Collider"\|name="SpotLight3D"\|name="FlyingForkliftModel2"' …` = 3 untouched lines; `grep -n 'flying_forklift.tscn' oxide-godot/level/level.tscn` = l.8; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `flying_forklift.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = 12 files; `grep -c '#\[func\]' oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs` = 0
- [x] T013 [US1] Record in `docs/v2-backlog.md` row no. 19 (research.md §"v2 backlog candidates"): `level/forklift/flying_forklift.gd` (port 1) — do not call `randomize()` per instance (`Main` already re-seeds at boot); motivation: re-seeding the global generator for every forklift is redundant. Do not duplicate items 1–18
- [x] T014 [US1] Single commit of port 1 on `main` (check `git status` first: only this story's files in staging), including `src/flying_forklift.rs`, `src/lib.rs`, `oxide-godot/level/forklift/flying_forklift.tscn`, the deletions of `flying_forklift.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port flying_forklift.gd → FlyingForklift (CharacterBody3D); flying_forklift.tscn: node FlyingForklift type="CharacterBody3D"→"FlyingForklift"` + body: `- base CharacterBody3D = the node's type in the scene (the script declared extends Node3D — constitution v1.3.1, Principle II)`; notes (dynamic Settings; `randomize()` + `floor(randf × n)` preserved); `- v2 backlog: item 19`. Note the hash
- [x] T015 [US1] **User checkpoint (visual validation, plan.md port 1)** — done by the user in the game, comparing with `../oxide_godot_origins/`: enter the level a few times (Play → ESC → Play) — the flying forklifts appear with models/colors varied among them and across re-entries; with `Shadow mapping` off in Settings, the headlight casts no shadow (on, it does); player and robots keep colliding with them. In the editor, `flying_forklift.tscn`: root `FlyingForklift` (type derived from `CharacterBody3D`), no script, `Collider` intact. Divergence → commit `Fix port flying_forklift.gd …`. Only proceed to US2 with the explicit OK

**Checkpoint**: 4 `.gd` remaining; `flying_forklift.tscn` with `FlyingForklift`; game playable.

---

## Phase 4: User Story 2 — Level ported (Priority: P2)

**Goal**: `level/level.gd` (127 l.) → `Level: Node3D`; signal `quit`; dynamic Settings + GI
(SDFGI/VoxelGI/lightmap); typed spawn of `EnemyRobot` (15 s respawn via `exploded`) and `Player`
(`set_player_id`); `add_player`/`del_player` connected to the `MultiplayerAPI` signals; ESC emits `quit`.

**Independent Test**: `level.tscn` headless without `ERROR` (4 robots + player 1 spawned, signals
connected) and `main.tscn` (the original `main.gd` connects `quit` via `has_signal` on the Rust `Level`);
in the game, GI according to the option, respawn, ESC returns to the menu.

- [x] T016 [US2] Change **only the visibility** in `oxide_godot_core/oxide_godot_lib/src/player.rs` (`fn set_player_id(&mut self, value: i32)` → `pub(crate) fn set_player_id(…)`, inside `#[godot_api] impl Player`, `#[func]` stays) and in `oxide_godot_core/oxide_godot_lib/src/red_robot.rs` (`#[signal] fn exploded();` → `#[signal] pub(crate) fn exploded();`). Check: `git diff --stat` = `player.rs | 2 +-` and `red_robot.rs | 2 +-` (2 −/+ pairs, nothing else). The accessor `robot.signals().exploded()` inherits the visibility of the signal's `fn` — without this it does not compile (research D2)
- [x] T017 [US2] Create `oxide_godot_core/oxide_godot_lib/src/level.rs` — part 1 (declarations), translating `level.gd:1-14`: `use godot::classes::input::MouseMode; use godot::classes::rendering_server::{EnvironmentSdfgiRayCount, VoxelGiQuality}; use godot::classes::{ConfigFile, INode3D, Input, InputEvent, LightmapGi, LightmapGiData, Marker3D, Node, Node3D, PackedScene, RenderingServer, WorldEnvironment}; use godot::global::{randi, randomize}; use godot::prelude::*; use crate::player::Player; use crate::red_robot::EnemyRobot;`; `i64` consts: `SDFGI = 0`, `VOXEL_GI = 1` (values of `Settings.GIType`), `GI_DISABLED = 0`, `GI_LOW = 1`, `GI_HIGH = 2` (`Settings.GIQuality`) — `LIGHTMAP_GI` is the `else` branch; `#[derive(GodotClass)] #[class(init, base=Node3D)] pub struct Level { base: Base<Node3D>, lightmap_gi: Option<Gd<LightmapGi>>, #[init(node = "WorldEnvironment")] world_environment: OnReady<Gd<WorldEnvironment>>, #[init(node = "RobotSpawnpoints")] robot_spawn_points: OnReady<Gd<Node3D>>, #[init(node = "PlayerSpawnpoints")] player_spawn_points: OnReady<Gd<Node3D>>, #[init(node = "SpawnedNodes")] spawned_nodes: OnReady<Gd<Node3D>> }`. `RedRobot`/`PlayerScene` (preload) do not become fields: `load` at the point of use (research D5)
- [x] T018 [US2] `level.rs` — part 2 (virtuals and privates), translating `level.gd:17-42,45-93,96-121,124-127`: `#[godot_api] impl INode3D for Level` with `fn ready(&mut self)`: `let window = self.base().get_window().unwrap(); let environment = self.world_environment.get_environment().unwrap(); let mut settings = self.base().get_node_as::<Node>("/root/Settings"); settings.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); let config_file = settings.get("config_file").to::<Gd<ConfigFile>>(); let gi_type = config_file.get_value("rendering", "gi_type").to::<i64>(); if gi_type == SDFGI { self.setup_sdfgi(); } else if gi_type == VOXEL_GI { self.setup_voxelgi(); } else { self.setup_lightmapgi(); }`; `let multiplayer = self.base().get_multiplayer().unwrap(); if multiplayer.is_server() { for child in self.robot_spawn_points.get_children().iter_shared() { self.spawn_robot(child.cast::<Node3D>()); } randomize(); let mut spawn_points = self.player_spawn_points.get_children(); spawn_points.shuffle(); let first = spawn_points.pop_front().map(|n| n.cast::<Marker3D>()); self.add_player(1, first); for id in multiplayer.get_peers().as_slice() { let next = spawn_points.pop_front().map(|n| n.cast::<Marker3D>()); self.add_player(*id, next); } multiplayer.signals().peer_connected().connect_other(&*self, |this: &mut Level, id: i64| this.add_player(id as i32, None)); multiplayer.signals().peer_disconnected().connect_other(&*self, |this: &mut Level, id: i64| this.del_player(id as i32)); }` (keep the original's comments); `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("quit") { Input::singleton().set_mouse_mode(MouseMode::VISIBLE); self.signals().quit().emit(); } }`. `impl Level` block **without** `#[godot_api]`: `fn gi_quality(&self) -> i64` NO — read `gi_quality` inline in each `setup_*` (`self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("rendering", "gi_quality").to::<i64>()`, as the original repeats `Settings.config_file.get_value(...)`); `fn setup_sdfgi(&mut self)` (`self.world_environment.get_environment().unwrap().set_sdfgi_enabled(true); self.base().get_node_as::<Node3D>("VoxelGI").hide(); self.base().get_node_as::<Node3D>("ReflectionProbes").hide(); if let Some(lightmap_gi) = &mut self.lightmap_gi { lightmap_gi.queue_free(); }` (**without** `take()`) `+ gi_quality: == GI_HIGH → RenderingServer::singleton().environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_96); == GI_LOW → COUNT_32; else → set_sdfgi_enabled(false)`); `fn setup_voxelgi` (sdfgi false; `VoxelGI` show; `ReflectionProbes` hide; queue_free likewise; `GI_HIGH → RenderingServer::singleton().voxel_gi_set_quality(VoxelGiQuality::HIGH)`; `GI_LOW → LOW`; else `VoxelGI` hide); `fn setup_lightmapgi` (sdfgi false; `VoxelGI` hide; `ReflectionProbes` show; `if self.lightmap_gi.is_none() { let mut new_gi = LightmapGi::new_alloc(); new_gi.set_light_data(&load::<LightmapGiData>("res://level/level.lmbake")); new_gi.set_name("LightmapGI"); self.lightmap_gi = Some(new_gi.clone()); self.base_mut().add_child(&new_gi); }`; `if gi_quality == GI_DISABLED { self.lightmap_gi.as_mut().unwrap().hide(); ReflectionProbes hide }`); `fn spawn_robot(&mut self, spawn_point: Gd<Node3D>)` (`let mut robot: Gd<EnemyRobot> = load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>(); robot.set_transform(spawn_point.get_transform()); robot.signals().exploded().connect_other(&*self, move |this: &mut Level| this._respawn_robot(spawn_point.clone())); self.spawned_nodes.add_child_ex(&robot).force_readable_name(true).done();`); `fn _respawn_robot(&mut self, spawn_point: Gd<Node3D>)` (`self.base().get_tree().create_timer(15.0).signals().timeout().connect_other(&*self, move |this: &mut Level| this.spawn_robot(spawn_point.clone()));`); `fn del_player(&mut self, id: i32)` (`let name = id.to_string(); if !self.spawned_nodes.has_node(&name) { return; } self.spawned_nodes.get_node_as::<Node>(&name).queue_free();`); `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` (`let spawn_point = spawn_point.unwrap_or_else(|| { let count = self.player_spawn_points.get_child_count(); self.player_spawn_points.get_child((randi() % count as i64) as i32).unwrap().cast::<Marker3D>() }); let mut player: Gd<Player> = load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>(); player.set_name(&id.to_string()); player.bind_mut().set_player_id(id); player.set_transform(spawn_point.get_transform()); self.spawned_nodes.add_child(&player);` — **without** `force_readable_name`) (research D5–D6)
- [x] T019 [US2] `level.rs` — part 3 (Godot block), translating `level.gd:4`: `#[godot_api] impl Level { #[signal] fn quit(); }` — the **only** `#[godot_api] impl Level` block; no `#[func]` (`add_player`/`del_player`/`spawn_robot`/`_respawn_robot`/`setup_*` are private — no script calls them by name; the connection to the `MultiplayerAPI` signals is via typed closure, T018) (research D5–D6, contracts/level.md)
- [x] T020 [US2] Add `mod level;` to `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod flying_forklift;`)
- [x] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. In case of error, research.md D2, D5–D6 and the bindings before improvising
- [x] T022 [US2] Edit `oxide-godot/level/level.tscn` as text (re-check with `grep -n 'name="Level" type=\|^script = ExtResource("1")\|level.gd' oxide-godot/level/level.tscn` → expected l.40, l.41, l.3): at the root replace `type="Node3D"` with `type="Level"`; remove `script = ExtResource("1")`; remove `[ext_resource type="Script" uid="uid://ccxbls23ev7u3" path="res://level/level.gd" id="1"]`. **KEEP** `SpawnedNodes`, `RobotSpawnpoints` (4 children), `PlayerSpawnpoints` (4 `Marker3D`), `MultiplayerSpawner` (`_spawnable_scenes`, `spawn_path = NodePath("../SpawnedNodes")`), `WorldEnvironment`, `VoxelGI`, `ReflectionProbes`, the `flying_forklift.tscn` instances and all the other `ext_resource`s. Expected diff: `4 +---`
- [x] T023 [US2] Delete `oxide-godot/level/level.gd` and `oxide-godot/level/level.gd.uid` (`git rm`); verify `grep -rn 'uid://ccxbls23ev7u3' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'level.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T024 [US2] Headless validation (no editor open): import with `Initialize godot-rust` and without new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . level/level.tscn 2>&1 | tee /tmp/run.log` → exit 124, regression grep (incl. `shadows a native class`) empty — isolated level: dynamic `apply_graphics_settings`, GI, 4 `EnemyRobot` and `Player` 1 spawned (typed), `peer_*` connections; `main/main.tscn` → exit 124, empty grep (baseline's intermittent-error rule: the 5 dummy-renderer lines do not count if they disappear on the 2nd run; 3 in a row = regression) — the original `main.gd` instantiates the Rust `Level` via `replace_main_scene` and connects `quit` via `has_signal`
- [x] T025 [US2] Mechanical checks and contract (`contracts/level.md`): `grep -c 'type="Level"' oxide-godot/level/level.tscn` = 1; `grep -c 'ExtResource("1")' …` = 0; `grep -n 'spawn_path\|_spawnable_scenes' …` = 2 untouched lines; `grep -n 'name="RobotSpawnpoints"\|name="PlayerSpawnpoints"\|name="SpawnedNodes"\|name="WorldEnvironment"\|name="VoxelGI"\|name="ReflectionProbes"' …` = 6; `grep -n 'has_signal(&"quit")\|node.quit.connect' oxide-godot/main/main.gd` = l.30-31 (`main.gd` remains intact); `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `level.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'` = exactly 4 lines (`set_player_id`, `exploded`); `ls src/` = 13 files; `grep -n '#\[signal\]\|#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/level.rs | grep 'fn '` = only `quit`
- [x] T026 [US2] Record in `docs/v2-backlog.md` rows no. 20 and 21 (research.md §"v2 backlog candidates"): 20 — `level/level.gd` (port 2): make `add_child(player, true)` uniform as in the robot (or `spawn_robot` without a readable name) — the two spawns use different forms; 21 — expose `add_player`/`del_player` with default parameter via two `#[func]`s (or remove the default) — gdext has no default parameter; v1 connects via closure. Do not duplicate items 1–19
- [x] T027 [US2] Single commit of port 2 on `main` (check `git status` first), including `src/level.rs`, `src/lib.rs`, `src/player.rs`, `src/red_robot.rs`, `oxide-godot/level/level.tscn`, the deletions of `level.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port level.gd → Level (Node3D); level.tscn: node Level type="Node3D"→"Level"` + body: notes (signal `quit` connected by name by `main.gd`; `EnemyRobot`/`Player` typed — `exploded` via `signals()`, `set_player_id`; dynamic Settings — `apply_graphics_settings`, `config_file`; GI via `settings.gd` integers; `add_player`/`del_player` private, connected to `peer_connected`/`peer_disconnected` via typed closure; quirks: `add_child(player)` without a readable name, `randomize()`, `lightmap_gi` kept after `queue_free`); `- player.rs / red_robot.rs: only pub(crate) visibility on set_player_id / exploded (typed access, FR-025); no logic moved`; `- v2 backlog: items 20, 21`. Note the hash
- [x] T028 [US2] **User checkpoint (visual validation, plan.md port 2)** — done by the user in the game, comparing with `../oxide_godot_origins/`: menu → Play → level: 4 robots spawn at the points, the player spawns at a random point (landing sound of the quirk), kill a robot → another spawns at the same point 15 s later; **ESC** releases the mouse and returns to the menu; Settings → change `GI type` (SDFGI / VoxelGI / LightmapGI) and `GI quality` (Disabled / Low / High), Apply, Play again → lighting/`ReflectionProbes` according to the option, no error in the console. In the editor, `level.tscn`: root `Level`, no script, `MultiplayerSpawner` intact. Divergence → commit `Fix port level.gd …`. Only proceed to US3 with the explicit OK

**Checkpoint**: 3 `.gd` remaining; `level.tscn` with `Level`; game playable.

---

## Phase 5: User Story 3 — Menu ported (Priority: P3)

**Goal**: `menu/menu.gd` (460 l.) → `Menu: Node`; signal `replace_main_scene(PackedScene)`; 85
`OnReady`; 15 `ButtonGroup`s; threaded loading with bar (**feature `experimental-threads`**);
9 handlers (10 connections) with the exact option ↔ `config_file` integer mappings;
ENet host/connect; dynamic Settings.

**Independent Test**: `menu.tscn` headless without `ERROR` (hosts by itself → loading → `DoneTimer` →
emits `replace_main_scene`) and `main.tscn` (full flow with the Rust `Menu` and `main.gd`); in the game,
every button and every Settings row behaves like the original, with persistence in `user://settings.ini`.

- [x] T029 [US3] Edit `oxide_godot_core/Cargo.toml` l.7: `godot = "0.5.5"` → `godot = { version = "0.5.5", features = ["experimental-threads"] }` (only change; no other feature, version unchanged — research D1; `CLAUDE.md` §Toolchain forbids changing the version, not a feature). Check `git diff --stat oxide_godot_core/Cargo.toml` = `1 +-`
- [x] T030 [US3] Regeneration build: `cd oxide_godot_core && cargo build 2>&1 | tail -3` → `Finished` (regenerates the `godot-core` bindings with the feature; may take minutes the first time — in the current environment the cache `4eba5d49e15a0d7e` from the plan's test build already exists); `grep -c '^warning'` = 0; note `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` (expected `…/godot-core-4eba5d49e15a0d7e/out`) and confirm `grep -c 'pub fn load_threaded_request\b' $(ls -dt …/godot-core-*/out | head -1)/classes/resource_loader.rs` = 1
- [x] T031 [US3] Create `oxide_godot_core/oxide_godot_lib/src/menu.rs` — part 1 (declarations), translating `menu.gd:1-104`: `use godot::classes::display_server::VSyncMode; use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality}; use godot::classes::resource_loader::ThreadLoadStatus; use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa}; use godot::classes::window::Mode as WindowMode; use godot::classes::{BaseButton, Button, ButtonGroup, ConfigFile, Control, DisplayServer, ENetMultiplayerPeer, HBoxContainer, INode, LineEdit, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene, ProgressBar, RenderingServer, ResourceLoader, SpinBox, Timer, VBoxContainer, WorldEnvironment}; use godot::global::is_equal_approx; use godot::prelude::*;`; `const LEVEL_PATH: &str = "res://level/level.tscn";`; `#[derive(GodotClass)] #[class(init, base=Node)] pub struct Menu { base: Base<Node>, #[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>, #[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"))] metalfx_supported: bool,` + the **85** fields `#[init(node = "<path>")] <name>: OnReady<Gd<<Type>>>` **exactly** as the "Scene references" table of `contracts/menu.md` (name, type and full path from the `Menu` — e.g.: `#[init(node = "UI/Settings/MaxFPS/30")] max_fps_30: OnReady<Gd<Button>>`), in the same order as the original `}`. Check at the end: `grep -c '#\[init(node = ' menu.rs` = 85 (research D7)
- [x] T032 [US3] `menu.rs` — part 2 (virtuals and privates), translating `menu.gd:107-160`: `#[godot_api] impl INode for Menu` with `fn ready(&mut self)`: `// Apply relevant settings directly.` + `let window = self.base().get_window().unwrap(); let environment = self.world_environment.get_environment().unwrap(); self.base().get_node_as::<Node>("/root/Settings").call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); if DisplayServer::singleton().get_name() == GString::from("headless") { self.base_mut().call_deferred("_on_host_pressed", &[]); } self.play_button.grab_focus(); if !self.metalfx_supported { self.scale_filter_metalfx_spatial.hide(); self.scale_filter_metalfx_temporal.hide(); }` + **15 explicit calls** `self._make_button_group(self.display_mode_menu.clone().upcast());` … in the original's order (`display_mode_menu, vsync_menu, max_fps_menu, resolution_scale_menu, scale_filter_menu, taa_menu, msaa_menu, screen_space_aa_menu, shadow_mapping_menu, gi_type_menu, gi_quality_menu, ssao_menu, ssil_menu, bloom_menu, volumetric_fog_menu`); `fn process(&mut self, _delta: f64)`: `if self.loading.is_visible() { let progress = VarArray::new(); let status: ThreadLoadStatus = ResourceLoader::singleton().load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done(); if status == ThreadLoadStatus::IN_PROGRESS { self.loading_progress.set_value(progress.at(0).to::<f64>() * 100.0); } else if status == ThreadLoadStatus::LOADED { self.loading_progress.set_value(100.0); self.base_mut().set_process(false); self.loading_done_timer.start(); } else { godot_print!("Error while loading level: {}", status.ord()); self.main.show(); self.loading.hide(); } }`. `impl Menu` block **without** `#[godot_api]`: `fn _make_button_group(&mut self, common_parent: Gd<Node>) { let group = ButtonGroup::new_gd(); for btn in common_parent.get_children().iter_shared() { if let Ok(mut btn) = btn.try_cast::<BaseButton>() { btn.set_button_group(&group); } } }` (research D3, D8)
- [x] T033 [US3] `menu.rs` — part 3 (Godot block), translating `menu.gd:4,163-460`: the **only** `#[godot_api] impl Menu` with `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);` and the 9 `#[func]` handlers: `_on_loading_done_timer_timeout` (`let peer = self.peer.clone(); self.base().get_multiplayer().unwrap().set_multiplayer_peer(&peer); let scene = ResourceLoader::singleton().load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>(); self.signals().replace_main_scene().emit(&scene);`); `_on_play_pressed` (`self.main.hide(); self.loading.show(); ResourceLoader::singleton().load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done();`); `_on_settings_pressed` — **line-by-line** translation of `menu.gd:175-311`: `self.main.hide(); self.settings_menu.show(); self.settings_action_cancel.grab_focus(); let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>();` and then each `if/elif/else` chain with `config_file.get_value(s, k).to::<i64>()` compared to `WindowMode::WINDOWED.ord() as i64` / `MAXIMIZED` / `FULLSCREEN`; `VSyncMode::DISABLED/ENABLED/ADAPTIVE` (else MAILBOX); `max_fps` 30/40/60/72/90/120/144 (else Unlimited); `is_equal_approx(get_value(..).to::<f64>(), 1.0 / 3.0)` / `1.0 / 2.0` / `1.0 / 1.7` / `1.0 / 1.5` / `1.0 / 1.3` (else Native); `Scaling3DMode::BILINEAR/FSR/FSR2/METALFX_SPATIAL/METALFX_TEMPORAL` via `.ord()` and, for the Nearest branch, the local constant `const SCALING_3D_MODE_NEAREST: i64 = 5; // absent from the prebuilt 4.6 API of gdext 0.5.5; Godot 4.7.2 = 5` (read and write — `Viewport.SCALING_3D_MODE_NEAREST` was added in 4.7; same technique as the GIType integers) (else: `metalfx_supported` → metalfx_temporal otherwise fsr2); `gi_type` 2/1/0 → `gi_lightmapgi/gi_voxelgi/gi_sdfgi`; `gi_quality` 0/1/2 → `gi_disabled/gi_low/gi_high`; `taa` bool; `Msaa::DISABLED/MSAA_2X/MSAA_4X/MSAA_8X`; `ScreenSpaceAa::DISABLED/FXAA/SMAA`; `shadow_mapping` bool; `ssao_quality` −1 / `EnvironmentSsaoQuality::MEDIUM` / `HIGH`; `ssil_quality` −1 / `EnvironmentSsilQuality::MEDIUM` / `HIGH`; `bloom`, `volumetric_fog` bool — each branch `btn.set_pressed(true)`; `_on_quit_pressed` (`self.base().get_tree().quit();`); `_on_apply_pressed` — line-by-line translation of `menu.gd:318-441`: `self.main.show(); self.play_button.grab_focus(); self.settings_menu.hide(); let mut settings = …; let mut config_file = settings.get("config_file").to::<Gd<ConfigFile>>();` and each chain `if btn.is_pressed() { config_file.set_value(s, k, &v.to_variant()) }` with `v`: `WindowMode::WINDOWED.ord() as i64` / `FULLSCREEN` / `EXCLUSIVE_FULLSCREEN`; vsync 4 `.ord() as i64`; fps `30_i64…144_i64`, Unlimited `0_i64`; scales `1.0 / 3.0` … `1.0_f64`; filters 6 `.ord() as i64`; gi_type `2_i64/1_i64/0_i64`; gi_quality `1/2/0` (order `low, high, disabled` like the original); `taa` `self.taa_enabled.is_pressed()`; msaa 4; screen_space_aa 3; `shadow_mapping` bool; ssao `-1_i64`/MEDIUM/HIGH; ssil likewise; `bloom`/`volumetric_fog` bool; then `// Apply relevant settings directly.` + `settings.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); settings.call("save_settings", &[]);`; `_on_cancel_pressed` (`main.show(); play_button.grab_focus(); settings_menu.hide(); online.hide();`); `_on_play_online_pressed` (`online.show(); main.hide();`); `_on_host_pressed` (`let mut peer = ENetMultiplayerPeer::new_gd(); peer.create_server(self.online_port.get_value() as i32); self.peer = peer.upcast(); self._on_play_pressed(); self.online.hide();`); `_on_connect_pressed` (`create_client(&self.online_address.get_text(), self.online_port.get_value() as i32)`, rest likewise). No other `#[func]`/`#[signal]` (research D7, D9; data-model.md options table)
- [x] T034 [US3] Add `mod menu;` to `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod level;`)
- [x] T035 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. In case of error, research.md D1, D3, D7–D9 and the bindings **with the feature** (`ls -dt … | head -1`) before improvising
- [x] T036 [US3] Edit `oxide-godot/menu/menu.tscn` as text (re-check with `grep -n 'name="Menu" type=\|^script = ExtResource("1")\|menu.gd' oxide-godot/menu/menu.tscn` → expected l.103, l.104, l.3): at the root replace `type="Node"` with `type="Menu"`; remove `script = ExtResource("1")`; remove `[ext_resource type="Script" uid="uid://4pwshyfo5i0d" path="res://menu/menu.gd" id="1"]`. **KEEP** the whole `UI/…` tree (85 paths), `WorldEnvironment`, `SpotLight3D`, `DoneTimer` (`wait_time = 0.5`, `one_shot = true`) and the **10** `[connection]`s at the end (l.836–845 → 835–844). Expected diff: `4 +---`
- [x] T037 [US3] Delete `oxide-godot/menu/menu.gd` and `oxide-godot/menu/menu.gd.uid` (`git rm`); verify `grep -rn 'uid://4pwshyfo5i0d' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'menu/menu.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty. **`menu/settings.gd` stays** (check that it was not touched: `git status --short oxide-godot/menu/` shows only `menu.gd`/`.uid` deleted and `menu.tscn` modified)
- [x] T038 [US3] Edit `CLAUDE.md` (operational edit): in §Toolchain, after the crate line `godot = "0.5.5"`, add: `- gdext feature `experimental-threads` enabled since Milestone D (Cargo.toml): without it the codegen omits `ResourceLoader::load_threaded_request/get_status/get` (godot-codegen `special_cases.rs:83-86`), which the menu uses for the loading bar. No other feature.`; in §Work cycle, in the paragraph on pre-existing errors, add: `In headless `main.tscn` the set `Initializing already initialized RID` / `Parameter "mem" is null.` / 3× `Parameter "m" is null.` may appear intermittently (a race between the level loading in a sub-thread and the dummy renderer) — it is not a regression if it disappears on a second run (research 004 §E.2).` Check `git diff --stat CLAUDE.md` = only insertions
- [x] T039 [US3] Headless validation (no editor open): import with `Initialize godot-rust` and without new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . menu/menu.tscn 2>&1 | tee /tmp/run.log` → exit 124, regression grep (incl. `shadows a native class`) empty — in headless the Rust `Menu` applies settings, schedules `_on_host_pressed`, creates the ENet server, requests the threaded loading (feature), `process` follows the status, `DoneTimer` fires and `replace_main_scene` is emitted (no consumer); `main/main.tscn` → exit 124, empty grep (intermittent-error rule) — the original `main.gd` connects the Rust `Menu`'s `replace_main_scene` via `has_signal`, receives the `PackedScene`, instantiates the Rust `Level`
- [x] T040 [US3] Mechanical checks and contract (`contracts/menu.md`): `grep -c '^\[connection' oxide-godot/menu/menu.tscn` = 10; `grep -o 'method="[^"]*"' oxide-godot/menu/menu.tscn | sort -u | wc -l` = 9 and each name exists as `#[func]` in `menu.rs`; `grep -n 'replace_main_scene' oxide-godot/main/main.gd` = l.20, 32, 33 (intact); `grep -c '#\[init(node = ' oxide_godot_core/oxide_godot_lib/src/menu.rs` = 85; `grep -c 'type="Menu"' oxide-godot/menu/menu.tscn` = 1; `grep -c 'ExtResource("1")' …` = 0; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `menu.gd` (**not** `settings.gd`); `git diff HEAD -- oxide_godot_core/Cargo.toml | grep '^[-+]godot'` = 1 pair with `features = ["experimental-threads"]`; `git diff --stat HEAD -- CLAUDE.md` = only insertions; `ls src/` = 14 files; `grep -n '#\[signal\]\|#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/menu.rs | grep 'fn ' | wc -l` = 10 (1 signal + 9 handlers)
- [x] T041 [US3] Record in `docs/v2-backlog.md` rows no. 22 and 23 (research.md §"v2 backlog candidates"): 22 — `menu/menu.gd` (port 3): declarative option→value table instead of ~30 `if/elif` chains in `_on_settings_pressed`/`_on_apply_pressed`; 23 — declare `replace_main_scene` with the parameter in the original (already done in the port) and connect `quit`/`replace_main_scene` in `Main` by type instead of `has_signal`. Do not duplicate items 1–21
- [x] T042 [US3] Single commit of port 3 on `main` (check `git status` first), including `src/menu.rs`, `src/lib.rs`, `oxide_godot_core/Cargo.toml`, `CLAUDE.md`, `oxide-godot/menu/menu.tscn`, the deletions of `menu.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port menu.gd → Menu (Node); menu.tscn: node Menu type="Node"→"Menu"` + body: notes (signal `replace_main_scene(PackedScene)` — parameter declared, the original emitted 1 arg on a parameterless signal; 85 OnReady with full paths; 15 ButtonGroups; threaded loading with bar; 9 handlers/10 connections; engine enums written/read via the `.ord()` integers; dynamic Settings — `apply_graphics_settings`, `save_settings`, `config_file`; ENet host/connect); `- Cargo.toml: gdext experimental-threads feature (ResourceLoader::load_threaded_* is not generated without it — godot-codegen special_cases.rs:83-86); version 0.5.5 unchanged`; `- CLAUDE.md: feature line + catalog of the intermittent dummy-renderer error in headless main.tscn`; `- v2 backlog: items 22, 23`. Note the hash
- [x] T043 [US3] **User checkpoint (visual validation, plan.md port 3)** — done by the user in the game, comparing with `../oxide_godot_origins/`: full menu — Play → the loading bar progresses to 100 % and the level opens ~0.5 s later; Settings → each of the 15 rows shows the saved value pressed; change a few options (e.g.: VSync, Max FPS, MSAA, GI type, Shadow mapping) and Apply → they apply immediately and persist in `user://settings.ini` (reopening Settings shows the new values; closing and reopening the game keeps them); Cancel/Back do not write; Play Online → Host starts the level as server, Back goes back; Quit closes; MetalFX buttons hidden (Linux); F11 (`settings.gd`) keeps working; keyboard navigation with focus on Play/Cancel as in the original. In the editor, `menu.tscn`: root `Menu`, no script, 10 connections in the signals panel. Divergence → commit `Fix port menu.gd …`. Only proceed to US4 with the explicit OK

**Checkpoint**: 2 `.gd` remaining; `menu.tscn` with `Menu`; feature enabled; game playable.

---

## Phase 6: User Story 4 — Main ported (Priority: P4)

**Goal**: `main/main.gd` (33 l.) → `Main: Node` (file `main_scene.rs`); boot (relay off, 60 fps
in headless, saved window mode), `go_to_main_menu`, `replace_main_scene` (deferred by name),
`change_scene_to_packed` with `has_signal`/`connect` by name — the original's duck typing.

**Independent Test**: `main.tscn` headless (Rust boot → `Menu` → host → `Level`) and `menu.tscn`
without `ERROR`; in the game, boot → menu → Play → level → ESC → menu → Play again, without duplicating scenes.

- [x] T044 [US4] Create `oxide_godot_core/oxide_godot_lib/src/main_scene.rs` translating line by line `oxide-godot/main/main.gd`: `use godot::classes::window::Mode as WindowMode; use godot::classes::{ConfigFile, DisplayServer, Engine, INode, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene, ResourceLoader, SceneMultiplayer}; use godot::global::randomize; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=Node)] pub struct Main { base: Base<Node> }`; `#[godot_api] impl INode for Main { fn ready(&mut self) { self.base().get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false); if DisplayServer::singleton().get_name() == GString::from("headless") { Engine::singleton().set_max_fps(60); } randomize(); let display_mode = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("video", "display_mode").to::<i64>(); self.base().get_window().unwrap().set_mode(WindowMode::from_ord(display_mode as i32)); self.go_to_main_menu(); } }`; the **only** `#[godot_api] impl Main`: `#[func] fn go_to_main_menu(&mut self) { let menu = ResourceLoader::singleton().load("res://menu/menu.tscn").unwrap().cast::<PackedScene>(); let mut multiplayer = self.base().get_multiplayer().unwrap(); multiplayer.get_multiplayer_peer().unwrap().close(); multiplayer.set_multiplayer_peer(&OfflineMultiplayerPeer::new_gd().upcast::<MultiplayerPeer>()); self.change_scene_to_packed(menu); }`; `#[func] fn replace_main_scene(&mut self, resource: Gd<PackedScene>) { self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()]); }`; `#[func] fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) { let mut node = resource.instantiate().unwrap(); for mut child in self.base().get_children().iter_shared() { self.base_mut().remove_child(&child); child.queue_free(); } self.base_mut().add_child(&node); if node.has_signal("quit") { node.connect("quit", &Callable::from_object_method(&self.to_gd(), "go_to_main_menu")); } if node.has_signal("replace_main_scene") { node.connect("replace_main_scene", &Callable::from_object_method(&self.to_gd(), "replace_main_scene")); } }`. Do **not** replace the `has_signal`/`connect` by name with `try_cast::<Level>`/`<Menu>` (the original's duck typing) (research D3, D10)
- [x] T045 [US4] Add `mod main_scene;` to `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod menu;`)
- [x] T046 [US4] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [x] T047 [US4] Edit `oxide-godot/main/main.tscn` as text (re-check with `grep -n 'name="main" type=\|^script = ExtResource("1")\|main.gd' oxide-godot/main/main.tscn` → expected l.5, l.6, l.3): at the root replace `type="Node"` with `type="Main"` **keeping `name="main"`**; remove `script = ExtResource("1")`; remove `[ext_resource type="Script" uid="uid://chrcwbh6kvb7i" path="res://main/main.gd" id="1"]`. Result: 3-line file (`[gd_scene …]`, empty, `[node name="main" type="Main" unique_id=1021137562]`). Expected diff: `4 +---`
- [x] T048 [US4] Delete `oxide-godot/main/main.gd` and `oxide-godot/main/main.gd.uid` (`git rm`); verify `grep -rn 'uid://chrcwbh6kvb7i' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'main/main.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty; `grep -n 'run/main_scene' oxide-godot/project.godot` = `res://main/main.tscn` (untouched)
- [x] T049 [US4] Headless validation (no editor open): import with `Initialize godot-rust` and without new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . main/main.tscn 2>&1 | tee /tmp/run.log` → exit 124, regression grep (incl. `shadows a native class`) empty (intermittent-error rule) — Rust boot → `go_to_main_menu` → Rust `Menu` hosts → loading → `replace_main_scene` received via `Callable` by name → `call_deferred("change_scene_to_packed")` → Rust `Level` instantiated, `quit` connected; `menu/menu.tscn` → exit 124, empty grep
- [x] T050 [US4] Mechanical checks and contract (`contracts/main.md`): `grep -n 'name="main" type=' oxide-godot/main/main.tscn` = `type="Main"` with `name="main"`; `grep -c 'ExtResource("1")' …` = 0; `grep -n '#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/main_scene.rs | grep 'fn '` = `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` (3, no other); `grep -n 'has_signal\|from_object_method\|call_deferred' oxide_godot_core/oxide_godot_lib/src/main_scene.rs` = `"quit"`, `"replace_main_scene"` ×2, `"change_scene_to_packed"`; `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = only `oxide-godot/menu/settings.gd`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `main.gd`; `ls src/` = 15 files
- [x] T051 [US4] Record in `docs/v2-backlog.md` row no. 24 (research.md §"v2 backlog candidates"): `main/main.gd` (port 4) — call `change_scene_to_packed` directly instead of `call_deferred` by name; motivation: string call to its own method, kept in v1 for fidelity. Do not duplicate items 1–23
- [x] T052 [US4] Single commit of port 4 on `main` (check `git status` first), including `src/main_scene.rs`, `src/lib.rs`, `oxide-godot/main/main.tscn`, the deletions of `main.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port main.gd → Main (Node); main.tscn: node main type="Node"→"Main"` + body: notes (`server_relay` via `SceneMultiplayer`; 60 fps in headless; window mode from the `config_file`; `has_signal`/`connect` by name and `call_deferred` by name preserved — the original's duck typing; node still named `main`; file `main_scene.rs`); `- v2 backlog: item 24`. Note the hash
- [x] T053 [US4] **User checkpoint (visual validation, plan.md port 4)** — done by the user in the game, comparing with `../oxide_godot_origins/`: boot straight into the menu with the saved window mode applied; Play → level; ESC → menu (offline peer recreated, mouse visible); Play again → level again (previous scene freed — no duplicated player/robots); Quit closes; in headless (`timeout 20 godot --headless --path . main/main.tscn`) the boot hosts and reaches the level by itself. In the editor, `main.tscn`: node `main` of type `Main`, no script. Divergence → commit `Fix port main.gd …`. Only proceed to Polish with the explicit OK

**Checkpoint**: 1 `.gd` remaining (`settings.gd`); entire game flow in Rust; game playable.

---

## Phase 7: Polish — final verification of the milestone

**Purpose**: only the quickstart's final verification and the full visual validation. No
extra documentation, no README, no refactoring.

- [x] T054 Mechanical verification of the milestone (quickstart §"Final verification"): `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = only `oxide-godot/menu/settings.gd`; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 1; `git diff --stat a866428 -- 'oxide-godot/**/*.gd'` shows exactly 4 deletions (`flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`) and **no** `settings.gd` line (SC-001); `git log --oneline a866428..HEAD | grep -c '^[0-9a-f]* Port '` = 4 (`Fix port …` do not count); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 14 modules (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot flying_forklift level menu main_scene`); `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = { version = "0.5.5", features = ["experimental-threads"] }`; `grep -c 'experimental-threads' CLAUDE.md` ≥ 1 and `grep -c 'already initialized RID' CLAUDE.md` = 1; `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2; `grep -c '^| [0-9]' docs/v2-backlog.md` = 24; FR-025: `grep -nE '\.call\(' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs` → only `"apply_graphics_settings"` (level, menu ×2) and `"save_settings"` (menu); `grep -nE 'call_deferred\(' …/{menu,main_scene}.rs` → only `"_on_host_pressed"` and `"change_scene_to_packed"`; `grep -nE '\.get\("' …/{flying_forklift,level,menu,main_scene}.rs` → only `"config_file"`; `grep -nE 'has_signal|from_object_method' …/main_scene.rs` → only `"quit"`/`"replace_main_scene"`/`"go_to_main_menu"`; `grep -n 'try_cast::<Level>\|try_cast::<Menu>' …/main_scene.rs` empty
- [x] T055 Final headless validation (no editor open): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; headless import with `Initialize godot-rust` and without `ERROR`; `main/main.tscn`, `menu/menu.tscn`, `level/level.tscn`, `level/forklift/flying_forklift.tscn`, `player/player.tscn`, `enemies/red_robot/red_robot.tscn` headless with empty regression grep (intermittent-error rule for `main.tscn`)
- [x] T056 **Full visual validation by the user (SC-002)**: boot → menu (Play, Play Online, Settings with the 15 rows reflecting and writing each option — apply, cancel, reopen, check `user://settings.ini`, restart the game —, Quit) → loading with bar → playable level (player, robots with 15 s respawn, forklifts with varied models, GI according to the option) → ESC returns to the menu → Play again — everything indistinguishable from `../oxide_godot_origins/` in a side-by-side session; Milestones A–C remain the same (shooting, shake, parts, door in isolated test). In the editor: the 4 scenes with the root of the Rust type and no script. Milestone D completed with the user's OK; then mark T001–T056 `[x]` and commit only `tasks.md` (`Tasks 004: marco D concluído`)

---

## Dependencies & Execution Order

### Mandatory order

```
Phase 1 (Setup: T001–T005)          — baseline of the 4 scenes + intermittent-error rule of main.tscn
  → Phase 3 US1 (T006–T015)  → commit 1  (FlyingForklift; base CharacterBody3D)
  → Phase 4 US2 (T016–T028)  → commit 2  (Level; + pub(crate) in player.rs/red_robot.rs)
  → Phase 5 US3 (T029–T043)  → commit 3  (Menu; + Cargo.toml feature + CLAUDE.md)
  → Phase 6 US4 (T044–T053)  → commit 4  (Main; main.gd ceases to exist — settings.gd is the only .gd)
  → Phase 7 Polish (T054–T056)
```

- **Phase 2 (Foundational)**: does not exist — nothing blocks the stories besides Setup.
- **Stories are not parallelizable among themselves**: each one ends with a commit on `main` and the
  next starts from the clean tree (FR-028, SC-004). US4 connects by name the signals that US2/US3
  register; until then, the original `main.gd` does the same — the game is playable after each commit.
- **Internal order of each story** (strict dependencies): [US2: `pub(crate)` visibility] /
  [US3: `Cargo.toml` → regeneration build] → `.rs` module → `mod` in `lib.rs` → `cargo build` →
  edit `.tscn` → delete `.gd`/`.uid` → [US3: `CLAUDE.md`] → headless → mechanical checks +
  contract → backlog → commit → user checkpoint. The `.tscn` is only edited after the build
  because the class needs to exist in the loaded lib for the `type` to resolve on the headless import.
- **User checkpoint** (T015, T028, T043, T053, T056) is blocking: the next story only
  starts with the explicit OK.

### Parallel Opportunities

Practically none, by construction:

- Setup: T004 is [P] relative to T002/T003/T005 (it only reads directories and runs greps).
- Within each story, no task is [P]: each step consumes the result of the previous one.
  T017→T018→T019 and T031→T032→T033 write the same file in sequence; T029→T030 (feature →
  regeneration) precede T031.
- Backlog (T013, T026, T041, T051) and `CLAUDE.md` (T038) could be written at any time
  before the story's commit, but they edit shared files — keep sequential.

### Parallel Example

```bash
# The only truly independent pair (Phase 1):
Task: "T002 cargo build → count warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out; name-collision grep"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline recorded, including the intermittent one of `main.tscn`.
2. Phase 3: US1 (T006–T015) — `flying_forklift.gd` ported, commit 1, user's OK.
3. **STOP AND VALIDATE**: first port in which the class base differs from the script's `extends`
   (rule v1.3.1); any problem with `CharacterBody3D` + `Collider` shows up here.

### Incremental Delivery

Each story is a complete port and the game remains playable after each commit:

1. US1 → 4 `.gd` remaining → playable
2. US2 → 3 → playable (`main.gd` connects the Rust `Level`'s `quit` via `has_signal`)
3. US3 → 2 → playable (feature enabled; `main.gd` connects the Rust `Menu`'s `replace_main_scene`)
4. US4 → 1 (`settings.gd`) → playable — entire flow in Rust
5. Polish → milestone D completed

### If something fails in the middle of a story

Do not commit partially. Either the whole port (module + scene + deletion [+ visibility / + `Cargo.toml`
+ `CLAUDE.md`]) goes into the commit, or nothing:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ CLAUDE.md && git clean -f oxide_godot_core/oxide_godot_lib/src/<module>.rs`
returns to the previous story's clean tree, which is always playable. If the implementation requires something not
foreseen in the tasks (another file, another visibility, another feature, a fix), stop and report.

---

## Notes

- No task creates a helper, trait, common module, test or new log. If it seems necessary, it is
  an entry in `docs/v2-backlog.md` — not a task. The menu keeps the 15 `_make_button_group` calls
  and the ~30 `if/elif` chains of the original (D8–D9).
- **No bug fix** in this milestone. Objective defect → stop and report; when in doubt, it is an improvement.
- Every `.tscn` is edited as text; re-check lines with `grep -n` immediately before each
  `sed` (each scene is edited exactly once in this milestone).
- Names of `#[func]`s, signals and the 85 node paths are contract (FR-024) — copy from the
  GDScript/`contracts/menu.md`, never "translate" (`_on_loading_done_timer_timeout`,
  `change_scene_to_packed`, `UI/Settings/ScreenSpaceAA/FXAA`).
- The headless regression grep is `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`;
  the baseline WARNINGs (`HDR output`, `Physics interpolation`) do not count; in `main.tscn`, the 5
  dummy-renderer lines do not count if they disappear on the 2nd run (3 in a row = regression).
- No Godot editor open during headless validations — warn the user first; never kill their
  process.
- The commit (T014, T027, T042, T052) comes **before** the visual checkpoint; a divergence found
  by the user is fixed in a `Fix port …` commit in the same story (never `Port …`, so that
  `git log | grep -c '^[0-9a-f]* Port '` remains = 4).
- **Staging**: before any commit other than the port's (e.g.: a spec commit by the
  user), `git status` must not have `.gd` deletions in staging (lesson from Milestone C).
- Files that **never** change in this milestone: `.gdextension`, `project.godot`,
  `docs/upstream-bugs.md`, `oxide-godot/menu/settings.gd` (+ `.uid`), `specs/` (except the `[x]` in
  `tasks.md` at the end), and in `player.rs`/`red_robot.rs` anything beyond the
  `pub(crate)` keywords of T016. `Cargo.toml` and `CLAUDE.md` change **only** in T029/T038.
