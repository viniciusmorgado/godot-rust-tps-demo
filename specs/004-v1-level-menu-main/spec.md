# Feature Specification: Milestone D — forklift, level, menu and main (v1 raw port)

**Feature Branch**: `004-v1-level-menu-main` (work on `main`, as in Milestones A–C)

**Created**: 2026-09-15

**Status**: Draft

**Phase**: v1 — Raw Port (Principle I of constitution v1.3.1). No abstraction, refactoring or optimization; noticed improvements go to `docs/v2-backlog.md`. No upstream bug is known in the four scripts; if an objective defect surfaces during the port, the conservative fix clause applies (declare in spec, isolate with a comment, mention in the commit, record in `docs/upstream-bugs.md`) — otherwise, nothing changes.

**Input**: User description: "Port to Rust the flying forklift, the level, the menu and the main of the Godot TPS Demo, keeping the game playable and identical at each script. Only settings.gd (autoload) stays in GDScript (milestone E). Milestone D of docs/port-order.md (items 11–14)."

## Context

Penultimate milestone of v1. Milestones A–C (`specs/001`–`003`, commits up to `b8f7124`) left 5 scripts in the original and delivered `Player`, `PlayerInputSynchronizer`, `CameraNoiseShake`, `Bullet`, `Door`, `Blast`, `PartDisappear`, `Part` and `EnemyRobot` as native classes. This milestone ports the entire game flow — boot (`main`), menu (`menu`), stage (`level`) and the decorative forklift (`flying_forklift`) — leaving only the autoload `settings.gd` in GDScript, accessed dynamically via `/root/Settings` (the single exception of Principle II; v2 backlog item 1).

Class names checked (`CLAUDE.md` rule): `FlyingForklift`, `Level`, `Menu`, `Main` do not coincide with any engine class nor with top-level identifiers of the remaining `.gd` files (`menu.gd` has `var main` in lowercase — GDScript identifiers are case-sensitive; and `menu.gd` will already be ported when `Main` is registered).

| # | Original script | Script base | Root node type | Scene | Lines | Consumers that remain in the original |
|---|---|---|---|---|---|---|
| 1 | `level/forklift/flying_forklift.gd` | `Node3D` | **`CharacterBody3D`** (`flying_forklift.tscn:36`, child `Collider`) | `level/forklift/flying_forklift.tscn` (instantiated by `level.tscn:8`) | 21 | none (uses `Settings.config_file`) |
| 2 | `level/level.gd` | `Node3D` | `Node3D` (`level.tscn:40`) | `level/level.tscn` | 127 | `main.gd:30-31` (`has_signal("quit")` + connection, until port 4); `settings.gd` (`apply_graphics_settings` receives the level as `scene_root`) |
| 3 | `menu/menu.gd` | `Node` | `Node` (`menu.tscn:103`) | `menu/menu.tscn` | 460 | `main.gd:32-33` (`has_signal("replace_main_scene")` + connection, until port 4) |
| 4 | `main/main.gd` | `Node` | `Node` (`main.tscn:5`, node named `main`) | `main/main.tscn` (main scene, `project.godot:15`) | 33 | none |

Behavior reference: the untouched original project in `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Flying forklift ported (Priority: P1)

The forklifts that float through the level stay the same: each one randomly picks one of the available models when it spawns and, if the player turned shadows off in the settings, its headlight stops casting a shadow.

**Why this priority**: Smallest script of the milestone, a leaf (only reads `Settings`), instantiated by `level.tscn` — proves, before the level, the base rule declared below (`Node3D` script on a `CharacterBody3D` node).

**Base rule declared in this spec**: the script says `extends Node3D`, but the scene's root node is `CharacterBody3D` (with a child `CollisionShape3D`). When the script's `extends` is an **ancestor** of the node's type in the scene, the ported class MUST use the node's type (`CharacterBody3D`): the `type` swap cannot demote the node nor discard the physics body the scene declares. The "same base" of Principle II is read as "same effective base of the node".

**Independent Test**: `flying_forklift.tscn` and `level.tscn` headless without new errors; in the game, the forklifts appear with varied models (different color among them) and, with shadows off, the headlight casts no shadow.

**Acceptance Scenarios**:

1. **Given** the forklift enters the scene with `rendering/shadow_mapping` false in the settings, **When** `ready` runs, **Then** the `SpotLight3D` has its shadow turned off; with true, it stays as it is in the scene.
2. **Given** the forklift's first child (`FlyingForkliftModel2`) has *n* children (models), **When** `ready` runs, **Then** exactly one of them — the one at index `floor(random × n)` — is visible and the others invisible.
3. **Given** the level instantiates several forklifts, **When** the game runs, **Then** each one picks independently (varied models among instances).
4. **Given** the root node is `CharacterBody3D` in the scene, **When** the `type` is swapped, **Then** the child `Collider` remains valid and the physics body keeps colliding with the player and the robots as before.

---

### User Story 2 - Level ported (Priority: P2)

The stage stays the same: on load, it applies the graphics settings and picks the global illumination technique (SDFGI, VoxelGI or lightmap) according to the settings; the server spawns four robots and the players at random points, and respawns each robot 15 s after it explodes; ESC releases the mouse and returns to the menu.

**Why this priority**: Consumes `EnemyRobot` (signal `exploded`) and `Player` (`player_id`) with **typed** access — closes the dependency graph of the already-ported classes — and emits `quit` to the `main` (still GDScript until port 4, connecting via `has_signal`).

**Independent Test**: `level.tscn` headless without new errors (spawn of the 4 robots and of player 1, typed `exploded` connection); `main.tscn` headless still with the original `main.gd` reaching the level; in the game, lighting according to each GI option, robot respawn, ESC returns to the menu.

**Acceptance Scenarios**:

1. **Given** the level enters the scene, **When** `ready` runs, **Then** `Settings.apply_graphics_settings(window, WorldEnvironment's environment, level)` is called (dynamic) and, according to `rendering/gi_type` (0 = SDFGI, 1 = VoxelGI, otherwise lightmap), the corresponding setup runs.
2. **Given** `gi_type` = SDFGI, **When** the setup runs, **Then** `sdfgi_enabled` is turned on in the environment, `VoxelGI` and `ReflectionProbes` are hidden, a previously created `LightmapGI` is freed, and `gi_quality` sets the ray count (2 = 96, 1 = 32, otherwise SDFGI off).
3. **Given** `gi_type` = VoxelGI, **When** the setup runs, **Then** SDFGI turns off, `VoxelGI` visible, `ReflectionProbes` hidden, previous `LightmapGI` freed, and `gi_quality` sets the VoxelGI quality (2 = high, 1 = low, otherwise `VoxelGI` hidden).
4. **Given** `gi_type` = lightmap, **When** the setup runs, **Then** SDFGI turns off, `VoxelGI` hidden, `ReflectionProbes` visible; if it does not exist, a `LightmapGI` named "LightmapGI" is created with `light_data = res://level/level.lmbake` and added to the level; if `gi_quality` = 0, the `LightmapGI` and the `ReflectionProbes` are hidden.
5. **Given** the peer is server, **When** `ready` runs, **Then** a robot is spawned at each child of `RobotSpawnpoints` (the point's transform, `exploded` connected to `_respawn_robot` with the point bound, child of `SpawnedNodes` with a readable name), the `PlayerSpawnpoints` points are shuffled, player 1 and each already-connected peer receive a point, and `peer_connected`/`peer_disconnected` are connected to `add_player`/`del_player`.
6. **Given** a robot emits `exploded`, **When** 15 s pass, **Then** another robot spawns at the same point.
7. **Given** `add_player(id)` without a point, **When** it runs, **Then** a random child of `PlayerSpawnpoints` is chosen; the player is instantiated with `name = str(id)`, `player_id = id` (typed access to `Player`), the point's transform, child of `SpawnedNodes`.
8. **Given** `del_player(id)`, **When** `SpawnedNodes` has a child named `str(id)`, **Then** it is removed; otherwise nothing happens.
9. **Given** the player presses the `quit` action (ESC), **When** `_input` receives the event, **Then** the mouse becomes visible and the `quit` signal is emitted — the `main` returns to the menu.
10. **Given** `main.gd` stays in the original until port 4, **When** it calls `node.has_signal("quit")` and connects, **Then** it resolves by name without editing.
11. **Given** the scene's `MultiplayerSpawner` (`level.tscn:73-75`), **When** robots and players are added to `SpawnedNodes`, **Then** replication remains valid (base API).
12. **Given** the peer is not server, **When** `ready` runs, **Then** only the graphics settings and the GI are applied (branch by code reading).

---

### User Story 3 - Menu ported (Priority: P3)

The menu stays the same: Play loads the level with a progress bar and swaps the scene; Play Online shows host/connect; Settings shows 15 rows of graphics options that reflect the current configuration, with Apply saving and applying and Cancel discarding; Quit closes the game. In headless mode, the menu hosts automatically.

**Why this priority**: Largest script of the milestone (460 lines, ~90 UI references, 15 button groups, ~30 options mapped to engine integers). Consumes only `Settings` (dynamic) and emits `replace_main_scene` to the `main`.

**Independent Test**: `menu.tscn` headless without new errors (in headless the menu hosts and loads the level by itself — exercises `_on_host_pressed`, `_on_play_pressed`, `_process`, `_on_loading_done_timer_timeout`); in the game, every menu button and every Settings row behaves as in the original, with persistence in `user://settings.ini`.

**Acceptance Scenarios**:

1. **Given** the menu enters the scene, **When** `ready` runs, **Then** `Settings.apply_graphics_settings` is called; in headless, `_on_host_pressed` is scheduled (deferred); `Play` receives focus; without MetalFX (driver ≠ "metal"), the `MetalFXSpatial` and `MetalFXTemporal` buttons are hidden; each of the 15 option rows receives its own `ButtonGroup` assigned to all child `BaseButton`s.
2. **Given** `Loading` is visible, **When** frames pass, **Then** the status of the threaded load of `res://level/level.tscn` is polled: in progress → bar = progress × 100; loaded → bar = 100, processing off, `DoneTimer` (0.5 s) started; error → message printed, `Main` visible, `Loading` hidden.
3. **Given** Play is pressed, **When** `_on_play_pressed` runs, **Then** `Main` hidden, `Loading` visible, threaded load of the level requested (with sub-threads).
4. **Given** `DoneTimer` expires, **When** `_on_loading_done_timer_timeout` runs, **Then** the `multiplayer_peer` receives the menu's `peer` and `replace_main_scene` is emitted with the loaded scene.
5. **Given** Settings is pressed, **When** `_on_settings_pressed` runs, **Then** `Main` hidden, `Settings` visible, `Cancel` focused, and in each row the button matching the current `config_file` value is pressed, with the exact mappings of `menu.gd:175-311` (window mode: Windowed/Maximized → Windowed, Fullscreen, otherwise ExclusiveFullscreen; vsync 4 values; max_fps 30/40/60/72/90/120/144/otherwise Unlimited; resolution scale by approximate comparison with 1/3, 1/2, 1/1.7, 1/1.5, 1/1.3, otherwise Native; scale filter 6 modes with fallback MetalFX temporal if supported or FSR2; GI type 3; GI quality 3; TAA; MSAA 4; screen-space AA 3; shadows; SSAO −1/medium/high; SSIL likewise; bloom; volumetric fog).
6. **Given** Apply is pressed, **When** `_on_apply_pressed` runs, **Then** `Main` visible, `Play` focused, `Settings` hidden; each option is written to the `config_file` according to the pressed button, with the exact values of `menu.gd:318-441` (Unlimited writes 0; scales write 1/3, 1/2, 1/1.7, 1/1.5, 1/1.3, 1.0; TAA/shadows/bloom/fog write the boolean of the "Enabled" button; SSAO/SSIL off write −1); then `Settings.apply_graphics_settings` and `Settings.save_settings` (dynamic).
7. **Given** Cancel or Back is pressed, **When** `_on_cancel_pressed` runs, **Then** `Main` visible, `Play` focused, `Settings` and `Online` hidden — nothing written.
8. **Given** Play Online, **When** pressed, **Then** `Online` visible and `Main` hidden; Host creates an ENet server on the `SpinBox` port and calls `_on_play_pressed`; Connect creates an ENet client with address and port and calls `_on_play_pressed`; both hide `Online` (branches verified by code reading and by the automatic host in headless).
9. **Given** Quit is pressed, **When** it runs, **Then** the game exits.
10. **Given** the 10 `[connection]`s of `menu.tscn:836-845`, **When** the buttons/timer fire, **Then** the handlers `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed` (Back **and** Cancel), `_on_apply_pressed`, `_on_loading_done_timer_timeout` resolve by name.
11. **Given** `main.gd` stays in the original until port 4, **When** it calls `node.has_signal("replace_main_scene")` and connects to a 1-argument method, **Then** the signal is emitted with the `PackedScene` and arrives.
12. **Given** all the engine enum values (window mode, vsync, 3D scale modes, MSAA, screen-space AA, SSAO/SSIL qualities) and the `settings.gd` ones (GI type/quality), **When** read from or written to the `config_file`, **Then** they are the same integers the original and `settings.gd` use — a `user://settings.ini` written by the ported menu is read by `settings.gd` and vice versa.

---

### User Story 4 - Main ported (Priority: P4)

The game boot stays the same: the main scene turns off the multiplayer relay, caps at 60 fps in headless, applies the saved window mode and goes to the menu; switching scenes removes the previous one and connects the signals `quit` (→ back to the menu) and `replace_main_scene` (→ switch to the received scene) of the new node, if it has them.

**Why this priority**: Last and smallest; depends on `Level` and `Menu` having the signals as native classes (the `has_signal` remains dynamic — the original's duck typing). Closes the full flow in ported code.

**Independent Test**: `main.tscn` headless (boot → menu → automatic host → level) without new errors; in the game, Play → level, ESC → menu, Quit exits.

**Acceptance Scenarios**:

1. **Given** `main.tscn` starts, **When** `ready` runs, **Then** `server_relay` turns off; in headless `max_fps = 60`; the window mode receives `video/display_mode` from the `config_file` (dynamic); `go_to_main_menu` runs.
2. **Given** `go_to_main_menu`, **When** it runs, **Then** `menu.tscn` is loaded, the current `multiplayer_peer` is closed and replaced by an `OfflineMultiplayerPeer`, and `change_scene_to_packed(menu)` runs.
3. **Given** `replace_main_scene(scene)` is called (by the menu's signal), **When** it runs, **Then** `change_scene_to_packed` is called **deferred, by name** — which is why `change_scene_to_packed` MUST be exposed with that name.
4. **Given** `change_scene_to_packed(scene)`, **When** it runs, **Then** the scene is instantiated, all current children are removed and freed, the new node is added; if it has a `quit` signal (`has_signal` — duck typing preserved), connects it to `go_to_main_menu`; if it has `replace_main_scene`, connects it to `replace_main_scene`.
5. **Given** `Level` and `Menu` are already native classes with those signals, **When** the dynamic `has_signal` runs, **Then** it finds both (checked in headless).
6. **Given** the root node of `main.tscn` is named `main` (lowercase), **When** the `type` becomes `Main`, **Then** the node's name does not change (node name ≠ class name).

---

### Edge Cases

- **Forklift: script base ≠ node type** — rule declared in US1: the node's type prevails (`CharacterBody3D`). It is the first time this occurs in the project; recorded for the reviewers.
- **Forklift: pick with `floor(random × n)`** — for *n* = 3, each model has 1/3; `randomize()` is called beforehand (as in the original — the global generator is re-seeded for every forklift; preserve).
- **Level: `LightmapGI` created at runtime** — the node only exists after a `setup_lightmapgi`; `setup_sdfgi`/`setup_voxelgi` free it if it exists (the level is recreated on every Play, so in practice it starts null).
- **Level: `add_player` connected to `peer_connected(id)`** — the signal passes 1 argument; the null-default `spawn_point` parameter covers the case (the method MUST accept the call with 1 argument coming from the signal and with 2 coming from `ready`).
- **Level: `_respawn_robot` after the level is destroyed** — the 15 s timer is created by the tree; if the level has already left (ESC → menu), the callback must not touch a freed level (behavior equivalent to the original's, whose `await` on a freed object is discarded).
- **Level: `spawn_robot(spawn_point)` untyped** — the original receives `Variant`; it only uses `.transform`; the port types it as `Node3D` (which the scene guarantees).
- **Menu: `signal replace_main_scene` declared without parameters** and emitted with 1 argument (`emit_signal("replace_main_scene", scene)`). The port declares the signal **with** the parameter (`PackedScene`), because typed emission requires it — the consumer (`main.gd`/`Main`) always received 1 argument; nothing observable changes. Recorded here for transparency, it is not a bug fix.
- **Menu: `_on_host_pressed` deferred in headless** — in `--headless`, the menu hosts and loads the level by itself: the headless validation of `menu.tscn` and `main.tscn` exercises the whole Play flow (loading, timer, `replace_main_scene`).
- **Menu: threaded loading** — `load_threaded_get_status` receives an array for progress; the port uses the API form that returns the progress (same value).
- **Menu: `metalfx_supported`** — false outside macOS; the two MetalFX buttons are hidden and the scale filter fallback is FSR2 (original).
- **Menu: `_make_button_group`** — skips children that are not `BaseButton` (the row labels).
- **Main: `remove_child` + `queue_free`** of the current children, in the original's order (remove before freeing).
- **Main: `randomize()`** — re-seeds the global generator at boot (besides the level's and the forklifts' calls).
- **Settings**: the 3 dynamic calls to the autoload (`config_file` get/set, `apply_graphics_settings(window, environment, scene_root)`, `save_settings()`) are the Principle II exception; `settings.gd` stays intact and is Milestone E.

## Requirements *(mandatory)*

### Functional Requirements

**Behavior — forklift (US1)**

- **FR-001**: The forklift MUST have base `CharacterBody3D` (root node type in `flying_forklift.tscn:36`; rule "the node's type prevails when the script's `extends` is its ancestor") and a reference to the `SpotLight3D`.
- **FR-002**: On entering the scene it MUST turn off the `SpotLight3D` shadow if `Settings.config_file` `rendering/shadow_mapping` is false; re-seed the random generator; and, among the *n* children of the first child, leave visible only the one at index `floor(random × n)`.

**Behavior — level (US2)**

- **FR-003**: The level MUST have base `Node3D`, signal `quit` (no arguments) and references to `WorldEnvironment`, `RobotSpawnpoints`, `PlayerSpawnpoints`, `SpawnedNodes`; `lightmap_gi` starts null.
- **FR-004**: On entering the scene it MUST call `Settings.apply_graphics_settings(window, environment, level)` (dynamic) and choose `setup_sdfgi` (`gi_type` = 0), `setup_voxelgi` (1) or `setup_lightmapgi` (others), reading `rendering/gi_type` from the `config_file` as an integer (the values of the `GIType` enum of `settings.gd`; integer literals in v1 — backlog item 1).
- **FR-005**: `setup_sdfgi`, `setup_voxelgi` and `setup_lightmapgi` MUST reproduce `level.gd:45-93` (visibility of `VoxelGI`/`ReflectionProbes`, freeing/creation of the `LightmapGI` with `light_data = res://level/level.lmbake` and name "LightmapGI", calls to the rendering server for ray count 96/32 and VoxelGI high/low quality, with `gi_quality` read as integer 0/1/2).
- **FR-006**: On the server, `ready` MUST spawn one robot per child of `RobotSpawnpoints`, re-seed, shuffle the children of `PlayerSpawnpoints`, `add_player(1, first)` and `add_player(id, next)` for each peer, and connect `peer_connected` → `add_player` and `peer_disconnected` → `del_player`.
- **FR-007**: `spawn_robot(spawn_point)` MUST instantiate `red_robot.tscn` with **typed** access to `EnemyRobot`, copy the point's `transform`, connect `exploded` to `_respawn_robot` with the point bound, and add to `SpawnedNodes` with a readable name; `_respawn_robot(spawn_point)` MUST wait 15 s and call `spawn_robot`.
- **FR-008**: `add_player(id, spawn_point = null)` MUST pick a random child of `PlayerSpawnpoints` when it receives no point, instantiate `player.tscn` with **typed** access to `Player`, set `name = str(id)`, `player_id = id`, copy the `transform` and add to `SpawnedNodes`; `del_player(id)` MUST remove the child `str(id)` from `SpawnedNodes` if it exists. Both MUST be invoked by the `MultiplayerAPI` signals: `peer_connected(id)` → `add_player(id)` without a point (random pick) and `peer_disconnected(id)` → `del_player(id)`. The form of the connection (by name, or a typed callable that calls the method with the point absent — the target platform has no default parameter in exposed methods) is the plan's decision; the observable behavior is the same.
- **FR-009**: `_input` MUST, on the `quit` action, make the mouse visible and emit `quit`.
- **FR-010**: `res://enemies/red_robot/red_robot.tscn` and `res://player/player.tscn` MUST be loaded by `load` at the point of use (`preload` → `load` pattern of the previous milestones); the internal `setup_*` methods and `_respawn_robot`/`spawn_robot` MUST keep their names.

**Behavior — menu (US3)**

- **FR-011**: The menu MUST have base `Node`, signal `replace_main_scene(scene: PackedScene)`, constant `LEVEL_PATH = "res://level/level.tscn"`, state `peer` (initially `OfflineMultiplayerPeer`) and `metalfx_supported` (current rendering driver == "metal"), and the references of `menu.gd:11-104` with the same node paths.
- **FR-012**: On entering the scene it MUST reproduce `menu.gd:107-125` (apply settings; `_on_host_pressed` deferred in headless; focus on Play; hide MetalFX when unsupported; `_make_button_group` on the 15 rows).
- **FR-013**: Per frame it MUST reproduce `menu.gd:128-141` (threaded load status → bar, timer, or printed error + back to Main).
- **FR-014**: The 9 handlers `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`, `_on_apply_pressed`, `_on_loading_done_timer_timeout` MUST exist with those names (10 connections in `menu.tscn:836-845`) and the effects of `menu.gd:153-460`.
- **FR-015**: `_on_settings_pressed` and `_on_apply_pressed` MUST use exactly the values and comparison orders of `menu.gd:175-311` and `318-441` (integers of the engine enums — window mode, vsync, 3D scale modes, MSAA, screen-space AA, SSAO/SSIL qualities — and of `settings.gd` — GI type/quality; `−1` for SSAO/SSIL off; `0` for unlimited fps; scales `1/3`, `1/2`, `1/1.7`, `1/1.5`, `1/1.3`, `1.0` with approximate comparison on read).
- **FR-016**: `_on_host_pressed`/`_on_connect_pressed` MUST create an `ENetMultiplayerPeer` (server on the `SpinBox` port; client with address and port), call `_on_play_pressed` and hide `Online`; `_on_loading_done_timer_timeout` MUST assign `peer` to the `multiplayer_peer` and emit `replace_main_scene` with the loaded level.
- **FR-017**: `_make_button_group` MUST be internal (not consumed externally) and assign a new `ButtonGroup` to each child `BaseButton` of the row, skipping the other children.

**Behavior — main (US4)**

- **FR-018**: The main MUST have base `Node` and reproduce `main.gd:4-10` in `ready` (`server_relay = false`; `max_fps = 60` in headless; re-seed; window mode = `video/display_mode` from the `config_file`; `go_to_main_menu`).
- **FR-019**: `go_to_main_menu`, `replace_main_scene(scene)` and `change_scene_to_packed(scene)` MUST exist with those names; `replace_main_scene` MUST call `change_scene_to_packed` deferred **by name** (like the original), which requires `change_scene_to_packed` to be exposed.
- **FR-020**: `change_scene_to_packed` MUST instantiate, remove and free all current children, add the new node and connect `quit` → `go_to_main_menu` and `replace_main_scene` → `replace_main_scene` **if** the node has those signals (`has_signal` — the original's duck typing, preserved).

**Port cycle — common to all four (Principle II)**

- **FR-021**: Each script MUST become exactly one native class: `FlyingForklift: CharacterBody3D`, `Level: Node3D`, `Menu: Node`, `Main: Node`. The names MUST be checked against engine classes and top-level identifiers of the remaining `.gd` files before each port (`CLAUDE.md` rule); collision → stop.
- **FR-022**: The binding MUST be by `type` swap at the root of each scene (`flying_forklift.tscn:36`, `level.tscn:40`, `menu.tscn:103`, `main.tscn:5`), with removal of `script` and of the orphaned `ext_resource`; no bridge `.gd`; no node renamed (including `main`).
- **FR-023**: `.gd` and `.gd.uid` MUST be deleted in the same commit as the port.
- **FR-024**: Names of signals, exposed methods and handlers MUST be identical to the GDScript: `quit`, `replace_main_scene`, the 9 menu handlers, `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed`, `add_player`, `del_player`, `_respawn_robot`, `spawn_robot`. Checked against `menu.tscn:836-845`, `main.gd:30-33` and the `MultiplayerAPI` signals.
- **FR-025**: The ported code MUST NOT call custom GDScript API, except the `Settings` autoload (`config_file` get/set, `apply_graphics_settings`, `save_settings` — dynamic via `/root/Settings`, Principle II exception, backlog item 1). Accesses to `EnemyRobot.exploded` and `Player.player_id` MUST be typed. The `main`'s `has_signal` is the original's duck typing (allowed).
- **FR-026**: Every code change MUST be followed by a debug build without new warnings.
- **FR-027**: Each port MUST be validated in headless: import with extension loading + `flying_forklift.tscn`, `level.tscn`, `menu.tscn`, `main.tscn` (those the story affects) without new errors beyond the baseline (the 3 from `CLAUDE.md`).
- **FR-028**: One commit per script, order 1 → 2 → 3 → 4; the game MUST remain playable after each commit (the original `main.gd` connects the native classes' signals via `has_signal` until port 4).
- **FR-029**: `settings.gd` MUST remain byte-for-byte intact; noticed improvements MUST go to `docs/v2-backlog.md` in the same commit; no bug fix is planned (objective defect → stop, declare in spec, then the 4 requirements of the clause).

### Key Entities

- **FlyingForklift**: reference `spot_light`; no external contract beyond the scene (no method/property consumed by other scripts).
- **Level (contract consumed by `main.gd`/`Main`, `settings.gd`, `MultiplayerAPI`)**: signal `quit`; methods `add_player(id, spawn_point = null)`, `del_player(id)` (connected to `MultiplayerAPI` signals), `spawn_robot`, `_respawn_robot`, `setup_sdfgi/voxelgi/lightmapgi`; state `lightmap_gi`; references `WorldEnvironment`, `RobotSpawnpoints`, `PlayerSpawnpoints`, `SpawnedNodes`, `VoxelGI`, `ReflectionProbes`.
- **Menu (contract consumed by `menu.tscn` and `main.gd`/`Main`)**: signal `replace_main_scene(scene)`; 9 handlers (10 connections); state `peer`, `metalfx_supported`; ~90 UI references (`menu.gd:11-104`); internal method `_make_button_group`.
- **Main (game root)**: methods `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` (the latter called by name, deferred).
- **Settings (`user://settings.ini`, via `settings.gd`)**: sections `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) and `rendering` (`gi_type`, `gi_quality`, `taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`) — read/written with the same integers/booleans/floats as the original.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the milestone the project contains exactly **1** `.gd` file (`menu/settings.gd`, and 1 `.uid`), byte-for-byte identical to the state before the milestone.
- **SC-002**: The full game — boot → menu (Play, Play Online, Settings with the 15 rows reflecting and writing each option, Quit) → loading with bar → playable level (player, robots, 15 s respawn, forklifts with varied models, lighting according to the chosen GI) → ESC returns to the menu — is indistinguishable from the original in `../oxide_godot_origins/` in a side-by-side comparison done by the user, including applying/canceling/reopening Settings and checking persistence in `user://settings.ini`.
- **SC-003**: For each of the 4 ports, the headless validation (import + affected scenes) reports zero new errors beyond the 3 cataloged; `main.tscn` headless goes through boot → menu → automatic host → level without error.
- **SC-004**: After each of the 4 commits the game is playable end to end.
- **SC-005**: The milestone's history has exactly 4 `Port …` commits and none touches `settings.gd`.
- **SC-006**: Debug build without any new warning in all commits.
- **SC-007**: No ported line introduces abstraction, refactoring, optimization or fix; `docs/upstream-bugs.md` remains with 2 entries.
- **SC-008**: `settings.gd` keeps working with the ported scenes: `apply_graphics_settings` receives window/environment/level from both callers, and the `settings.ini` written by the ported menu is read without value changes.

## Assumptions

- Phase v1 (constitution v1.3.1); no bug fix planned.
- Visual validation (SC-002) by the user, including the full settings menu; automated validation exclusively headless. The reviewer may run a parity harness outside the repository.
- Single-player validation: the local peer is server (`OfflineMultiplayerPeer`); real host/connect and "client" branches (FR-006, FR-016) are verified by code reading and by the automatic host in headless.
- Out of scope: `settings.gd` (Milestone E); typed access to `Settings`; any improvement; real multiplayer with two peers.
- Base rule declared (US1): when the script's `extends` is an ancestor of the root node's type, the class uses the node's type — `FlyingForklift: CharacterBody3D`.
- The menu's `signal replace_main_scene` now declares the `PackedScene` parameter the original already emits; it is not a fix, it is a requirement of typed emission (documented in Edge Cases).
- Quirks preserved on purpose (candidates for the v2 backlog, not for a fix): `randomize()` called in three places (main, level, each forklift); `spawn_robot(spawn_point)` untyped in the original; `add_player` without `force_readable_name` (the name is already `str(id)`) while `spawn_robot` uses it; `_on_apply_pressed` with `if/elif` chains that write nothing if no button of the row is pressed; reading `gi_type`/`gi_quality` as integer literals instead of the `settings.gd` enum.
- The root node of `main.tscn` is named `main` (lowercase) and stays that way; the class is named `Main`.
