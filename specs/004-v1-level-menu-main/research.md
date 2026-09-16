# Research: Milestone D — forklift, level, menu and main (gdext 0.5.5)

**Phase**: v1 — Raw Port. Exact gdext 0.5.5 signatures used by the four ports and translation
decisions, with discarded alternatives. Sources: crate `~/.cargo/registry/src/*/godot-core-0.5.5/`,
`godot-codegen-0.5.5/`, `godot-macros-0.5.5/`; bindings generated in
`oxide_godot_core/target/debug/build/godot-core-*/out/`. **Attention**: this milestone enables the feature
`experimental-threads` (D1), which generates a **second** bindings directory — on 2026-09-16
there exist `godot-core-aea5c50e7fda9d57/out` (without the feature, used by Milestones A–C) and
`godot-core-4eba5d49e15a0d7e/out` (with the feature). The lines cited below are from the directory
**with** the feature; `ls -dt .../godot-core-*/out | head -1` returns the most recent one.

**Confirmation method**: the whole mapping was written as a temporary module
(`zz_research.rs`, classes `ZzFlyingForklift`/`ZzLevel`/`ZzMenu`/`ZzMain` — the menu with a
representative subset of the 85 `@onready`, but with all the API *forms* the handlers
use) inside the crate **with the feature temporarily enabled**, compiled with `cargo build` →
**0 errors, 0 warnings** after four corrections imposed by the compiler (D3, D9, D10 — recorded
because the command input had them differently), and exercised in headless via `-s` script (§E).
Module, `Cargo.toml` and the temporary visibility changes were reverted (`git status`
clean).

## D1 — Feature `experimental-threads` (crate configuration change, decided by the user)

- **Decision**: in `oxide_godot_core/Cargo.toml`, `[workspace.dependencies]`:
  `godot = { version = "0.5.5", features = ["experimental-threads"] }`. Goes into the
  **port 3 (menu)** commit, cited in the message; `CLAUDE.md` §Toolchain gains a line recording the
  feature and the reason. Crate version unchanged; no other feature.
- **Rationale**: `godot-codegen-0.5.5/src/special_cases/special_cases.rs:83-86` excludes
  `ResourceLoader::load_threaded_request`, `load_threaded_get_status` and `load_threaded_get` from the
  codegen when the feature is off (`#[cfg(not(feature = "experimental-threads"))]`);
  `menu.gd:130,158,163` uses all three (loading bar). Without the feature, the only alternative would be
  `ResourceLoader::singleton().call("load_threaded_request", …)` — a dynamic call to the engine,
  contrary to the spirit of Principle II (untyped code). `godot-0.5.5/Cargo.toml:78`:
  `experimental-threads = ["godot-core/experimental-threads"]`.
- **Confirmed**: with the feature, `resource_loader.rs` (new bindings) has
  `load_threaded_request(path) -> Error` (l.37) / `load_threaded_request_ex(path)` with
  `.use_sub_threads(bool)` (l.42, 307); `load_threaded_get_status(path) -> ThreadLoadStatus`
  (l.58) / `_ex(path)` with `.progress(&AnyArray)` (l.63, 341);
  `load_threaded_get(path) -> Option<Gd<Resource>>` (l.67). `ThreadLoadStatus` has
  `INVALID_RESOURCE`, `IN_PROGRESS`, `FAILED`, `LOADED` (l.452+). Crate build with the feature:
  0 warnings; the bindings without the feature remain cached (switching back is instantaneous).
- **Accepted side effect**: the feature also enables other APIs marked as thread-unsafe
  in gdext; none is used. It is the only configuration change of v1 so far — recorded in
  plan.md §Complexity Tracking.

## D2 — Structure and visibility

- **Decision**: `src/flying_forklift.rs` (`FlyingForklift: CharacterBody3D`), `src/level.rs`
  (`Level: Node3D`), `src/menu.rs` (`Menu: Node`), `src/main_scene.rs` (`Main: Node` — file name
  `main_scene.rs` so as not to suggest a binary `main.rs`; `mod main_scene;`).
- **Visibility (only the keyword, port 2 commit — Level)**: `player.rs`
  `fn set_player_id` → `pub(crate)` (the level does `player.bind_mut().set_player_id(id)`);
  `red_robot.rs` `#[signal] fn exploded();` → `pub(crate) fn exploded();` — **imposed by the
  compiler**: the accessor `robot.signals().exploded()` inherits the visibility of the signal's `fn`
  (`error[E0624]: method exploded is private`). Precedents Milestones B/C.
- **Names**: checked with the `CLAUDE.md` grep and against `out/classes/`: `FlyingForklift`,
  `Level`, `Menu`, `Main` without collision (`menu.gd` has `var main` — lowercase, and it is ported before
  `Main` exists).
- **Discarded alternatives**: `player.set("player_id", …)` (dynamic between Rust classes, violates
  FR-025); `robot.connect("exploded", …)` by name (likewise — the robot is already Rust).

## D3 — String comparisons and `GString`

- **Correction imposed by the compiler**: `x == "metal".into()` is ambiguous (`GString: PartialEq<_>`
  has several impls). Use `GString::from("metal")` / `GString::from("headless")`:
  `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")`
  (`rendering_server.rs:5391`), `DisplayServer::singleton().get_name() == GString::from("headless")`
  (`display_server.rs:35`). Same lesson as Milestone C's `StringName::from("Target")`.
- `metalfx_supported` can be initialized in `#[init(val = …)]` — the singleton is available
  at construction (it compiled; the original also evaluates it in the `var` initializer).

## D4 — FlyingForklift

- **Base `CharacterBody3D`** (constitution v1.3.1, Principle II: `extends Node3D` is an ancestor of
  the node's type `flying_forklift.tscn:36`). `#[init(node = "SpotLight3D")] spot_light: OnReady<Gd<SpotLight3D>>`;
  `Light3D::set_shadow(false)` (`light_3d.rs:62`).
- `Settings.config_file.get_value(...)`: `self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("rendering", "shadow_mapping").to::<bool>()`
  (pattern D14 of Milestone B).
- `randomize()` → `godot::global::randomize()` (`utilities.rs:792`); `get_child(0)` →
  `self.base().get_child(0).unwrap()` (`node.rs:368`, `Option`); `get_children()` →
  `Array<Gd<Node>>` (`node.rs:347`) with `.len()` and `.iter_shared()`;
  `floori(randf() * float(n))` → `(randf() * n as f64).floor() as usize`;
  `children[i].visible = …` → `child.cast::<Node3D>().set_visible(i == which)` (`node_3d.rs:620`;
  the model's children are `Node3D`).
- **Confirmed (§E.1)**: `ZzFlyingForklift.new() is CharacterBody3D == true`.

## D5 — Level: signal, dynamic Settings, GI

- `#[signal] fn quit();` in the main `#[godot_api] impl Level` block; emission
  `self.signals().quit().emit()` (pattern D6 of Milestone C). **Confirmed (§E.1)**: `has_signal("quit")`,
  0 arguments, connection by name from GDScript receives the emission (= what `main.gd:30-31` does).
- `Settings.apply_graphics_settings(get_window(), world_environment.environment, self)` →
  ```rust
  let window = self.base().get_window().unwrap();                       // node.rs:1163, Option
  let environment = self.world_environment.get_environment().unwrap();  // world_environment.rs:179, Option
  let mut settings = self.base().get_node_as::<Node>("/root/Settings");
  settings.call("apply_graphics_settings",
      &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]);   // object.rs:419
  ```
  — `Settings` exception of Principle II (dynamic call to the autoload). `save_settings` likewise:
  `settings.call("save_settings", &[])`.
- `Settings.GIType.SDFGI` etc. → local constants `const SDFGI: i64 = 0; VOXEL_GI = 1;`
  (`LIGHTMAP_GI = 2` is the `else` branch, needs no constant) and `GI_DISABLED = 0; GI_LOW = 1;
  GI_HIGH = 2` (values from `settings.gd:3-13`); read `get_value(..).to::<i64>()`. v2 backlog
  item 1 already covers typed access to the enum.
- `environment.sdfgi_enabled = x` → `environment.set_sdfgi_enabled(x)` (`environment.rs:822`);
  `$VoxelGI.hide()` → `self.base().get_node_as::<Node3D>("VoxelGI").hide()` (`node_3d.rs:659`;
  `VoxelGI` and `ReflectionProbes` are looked up inline with `$` in the original — kept inline);
  `RenderingServer.environment_set_sdfgi_ray_count(ENV_SDFGI_RAY_COUNT_96)` →
  `RenderingServer::singleton().environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_96)`
  (`rendering_server.rs:3493`; constants `COUNT_32/64/96/128`, l.14262+ — gdext spelling without the
  `RAY_` prefix); `voxel_gi_set_quality(VoxelGiQuality::HIGH | LOW)` (`:1795`, consts l.9967/9972).
- **`lightmap_gi` after `queue_free`**: the original calls `lightmap_gi.queue_free()` and **keeps**
  the reference (`lightmap_gi != null` remains true), and `setup_lightmapgi` only creates a new one
  if `== null`. Reproduce literally: `if let Some(lightmap_gi) = &mut self.lightmap_gi { lightmap_gi.queue_free(); }`
  **without** `take()`; `setup_lightmapgi` creates only `if self.lightmap_gi.is_none()`. (In practice the
  level is recreated on every Play and each `ready` calls a single `setup_*`, so the reference never
  dangles; fidelity is to the code.) Creation: `LightmapGi::new_alloc()` (gdext spelling
  `LightmapGi`/`LightmapGiData`), `set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"))`
  (`lightmap_gi.rs:173`), `set_name("LightmapGI")` (`node.rs:243`), `self.base_mut().add_child(&new_gi)`;
  store `Some(new_gi.clone())` **before** the `add_child` (the variable is moved into the tree only
  by reference, but the clone avoids a borrow conflict).

## D6 — Level: spawn, players, MultiplayerAPI signals

- Server: `let multiplayer = self.base().get_multiplayer().unwrap(); if multiplayer.is_server() { … }`.
- `for child in robot_spawn_points.get_children(): spawn_robot(child)` →
  `for child in self.robot_spawn_points.get_children().iter_shared() { self.spawn_robot(child.cast::<Node3D>()); }`
  (`spawn_point` untyped in the original; the port types it `Gd<Node3D>` — only `.transform` is used).
- `spawn_points.shuffle()` → `Array<Gd<Node>>` does `Deref` to `AnyArray`, which has `shuffle()`
  (`any_array.rs:366`); `pop_front() -> Option<Gd<Node>>` (`array.rs:386`);
  `multiplayer.get_peers() -> PackedInt32Array` (`multiplayer_api.rs:134`) → `.as_slice()`.
- `spawn_robot(spawn_point)`: `let mut robot: Gd<EnemyRobot> = load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>();`
  (**typed** — the robot is Rust since Milestone C); `robot.set_transform(spawn_point.get_transform())`;
  `robot.exploded.connect(_respawn_robot.bind(spawn_point))` →
  `robot.signals().exploded().connect_other(&*self, move |this: &mut Level| this._respawn_robot(spawn_point.clone()))`
  (the `bind` becomes a `move` capture; requires `pub(crate) fn exploded()` — D2);
  `spawned_nodes.add_child(robot, true)` → `self.spawned_nodes.add_child_ex(&robot).force_readable_name(true).done()`.
- `_respawn_robot(spawn_point)`: `self.base().get_tree().create_timer(15.0).signals().timeout().connect_other(&*self, move |this: &mut Level| this.spawn_robot(spawn_point.clone()))`
  — *linked* callable: if the level is freed (ESC → menu) before the 15 s, it does not run
  (equivalent to the original's `await` on a freed object).
- `add_player(id: int, spawn_point: Marker3D = null)` → `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)`
  **private** (no `#[func]`): GDScript connects `peer_connected` (1 argument) to the method with a
  default parameter; gdext has no default parameter in `#[func]`, so the connection is via typed
  closure — `multiplayer.signals().peer_connected().connect_other(&*self, |this: &mut Level, id: i64| this.add_player(id as i32, None))`
  and `peer_disconnected().connect_other(&*self, |this, id: i64| this.del_player(id as i32))`
  (`multiplayer_api.rs:388,394`; the signal's argument is **`i64`**, l.418 — `id as i32` at the
  boundary, like Milestone B's `player_id: i32`). No script calls `add_player`/`del_player` by
  name (only the signals), so FR-008 ("connectable to the signals by their names") is satisfied by the
  typed connection; the spec's Key Entities lists them as Level methods — a behavior contract,
  not a `has_method` one. Recorded in plan.md.
- Body of `add_player`: `let spawn_point = spawn_point.unwrap_or_else(|| { let count = self.player_spawn_points.get_child_count(); self.player_spawn_points.get_child((randi() % count as i64) as i32).unwrap().cast::<Marker3D>() });`
  (`randi() -> i64`, `get_child_count() -> i32`); `let mut player: Gd<Player> = load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>();`
  (**typed**); `player.set_name(&id.to_string())` (`node.rs:243`, `AsArg<StringName>` accepts `&String`);
  `player.bind_mut().set_player_id(id)` (Rust setter, `pub(crate)` — D2; it is exactly what
  `player.player_id = id` triggers in the original); `player.set_transform(spawn_point.get_transform())`;
  `self.spawned_nodes.add_child(&player)` — **without** `force_readable_name` (quirk: `level.gd:121`).
- `del_player(id)`: `let name = id.to_string(); if !self.spawned_nodes.has_node(&name) { return; } self.spawned_nodes.get_node_as::<Node>(&name).queue_free();`
  (`node.rs:377`).
- `_input`: `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("quit") { Input::singleton().set_mouse_mode(MouseMode::VISIBLE); self.signals().quit().emit(); } }`.

## D7 — Menu: signal with parameter, state, 85 references

- `signal replace_main_scene` (declared without a parameter, emitted with 1 — spec Edge Cases) →
  `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);`; emission
  `self.signals().replace_main_scene().emit(&scene)`. **Confirmed (§E.1)**: the signal registers 1
  argument `scene: PackedScene`; connection by name from GDScript (like `main.gd:33`)
  receives the `PackedScene`.
- `var peer: MultiplayerPeer = OfflineMultiplayerPeer.new()` →
  `#[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>`;
  `metalfx_supported` → `#[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"))]` (D3).
- `const LEVEL_PATH: &str = "res://level/level.tscn"`.
- The **85** `@onready` of `menu.gd:11-104` become `#[init(node = "<path>")] OnReady<Gd<T>>`
  with the **full** path from the menu (the original's `x.get_node(^"Y")` chains
  become `X/Y`, as in the previous milestones). Full table (name, type, path) in
  [contracts/menu.md](contracts/menu.md) — it is a scene contract. Types: `WorldEnvironment`,
  `Control`, `Button`, `SpinBox`, `LineEdit`, `VBoxContainer`, `HBoxContainer`, `ProgressBar`,
  `Timer`. Fields never read (`ui`, `settings_button`, `quit_button`, `settings_actions`,
  `settings_action_apply`, the 15 `*_menu` only used in `_make_button_group`) stay for
  fidelity — the derive references the field, no warning (precedent `crosshair`, Milestone B).

## D8 — Menu: ready, process, `_make_button_group`

- `_on_host_pressed.call_deferred()` → `self.base_mut().call_deferred("_on_host_pressed", &[])`
  (`object.rs:438`; call by name to its own `#[func]` — it is what the original does).
- `play_button.grab_focus()` (`control.rs:793`); `hide()`/`show()`/`is_visible()` from
  `CanvasItem` (`canvas_item.rs:66,75`).
- `_make_button_group(menu)` ×15 → private method `fn _make_button_group(&mut self, common_parent: Gd<Node>)`:
  `let group = ButtonGroup::new_gd(); for btn in common_parent.get_children().iter_shared() { if let Ok(mut btn) = btn.try_cast::<BaseButton>() { btn.set_button_group(&group); } }`
  (`base_button.rs:418`; `try_cast` reproduces `if not btn is BaseButton: continue`). The 15
  calls stay explicit (`self._make_button_group(self.display_mode_menu.clone().upcast())` …)
  — the original's `for menu in [...]` becomes 15 lines; no additional helper.
- `_process`: ```rust
  if self.loading.is_visible() {
      let progress = VarArray::new();                       // &VarArray deref-coerce → &AnyArray
      let status = ResourceLoader::singleton().load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done();
      if status == ThreadLoadStatus::IN_PROGRESS { self.loading_progress.set_value(progress.at(0).to::<f64>() * 100.0); }
      else if status == ThreadLoadStatus::LOADED { self.loading_progress.set_value(100.0); self.base_mut().set_process(false); self.loading_done_timer.start(); }
      else { godot_print!("Error while loading level: {}", status.ord()); self.main.show(); self.loading.hide(); }
  }
  ``` (`Range::set_value(f64)`, `range.rs:276`; `print("…" + str(status))` → `godot_print!` with the
  enum's integer via `.ord()`, the same text that `str()` of an enum produces).

## D9 — Menu: handlers, `config_file` and enum integers

- `#[func]` handlers: `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`,
  `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`,
  `_on_apply_pressed`, `_on_loading_done_timer_timeout` (10 connections, `menu.tscn:836-845`).
  **Confirmed (§E.1)**: all visible via `has_method`; `_make_button_group` is not.
- `_on_play_pressed`: `ResourceLoader::singleton().load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done()`
  (`Error` return ignored, as in the original).
- `_on_loading_done_timer_timeout`: `let peer = self.peer.clone(); self.base().get_multiplayer().unwrap().set_multiplayer_peer(&peer);`
  (`multiplayer_api.rs:43`); `let scene = ResourceLoader::singleton().load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>(); self.signals().replace_main_scene().emit(&scene);`.
- `_on_quit_pressed`: `self.base().get_tree().quit()` (`scene_tree.rs:377`).
- `_on_host_pressed`: `let mut peer = ENetMultiplayerPeer::new_gd(); peer.create_server(self.online_port.get_value() as i32); self.peer = peer.upcast(); self._on_play_pressed(); self.online.hide();`
  (`enet_multiplayer_peer.rs:135`; `Range::get_value() -> f64`, `int(x)` → `as i32`).
  `_on_connect_pressed`: `peer.create_client(&self.online_address.get_text(), self.online_port.get_value() as i32)` (`:156`; `line_edit.rs:445`).
- `_on_settings_pressed` / `_on_apply_pressed`: line-by-line translation of `menu.gd:175-311` and
  `318-441`. `Settings.config_file` is read **once** at the start of each handler
  (`let config_file = …get("config_file").to::<Gd<ConfigFile>>()`) — the original repeats
  `Settings.config_file` on every line, but it is the same object (the autoload holds a single
  `ConfigFile`); reading the handle once does not change behavior nor is it an abstraction (it is the
  local variable any translation needs). Reads: `.get_value(s, k).to::<i64>()` (enums and
  `max_fps`), `.to::<f64>()` (`resolution_scale`), `.to::<bool>()` (flags). Writes:
  `config_file.set_value(s, k, &v.to_variant())` with `v: i64` for enums/fps, `f64` for scale,
  `bool` for flags.
- **Engine enum integers** — use `.ord()` (`obj/traits.rs:199`, `EngineEnum::ord() -> i32`)
  of the gdext enums, converted `as i64` to compare/write: `window::Mode::{WINDOWED, MAXIMIZED, FULLSCREEN, EXCLUSIVE_FULLSCREEN}`
  (`window.rs:2342+`), `display_server::VSyncMode::{DISABLED, ENABLED, ADAPTIVE, MAILBOX}`,
  `viewport::Scaling3DMode::{BILINEAR, FSR, FSR2, METALFX_SPATIAL, METALFX_TEMPORAL}` — **`NEAREST` does NOT exist in the prebuilt 4.6 API** (it was added in Godot 4.7; `viewport.rs` only has BILINEAR=0, FSR=1, FSR2=2, METALFX_SPATIAL=3, METALFX_TEMPORAL=4, MAX=5): use the local constant `const SCALING_3D_MODE_NEAREST: i64 = 5;` (runtime 4.7.2 value, checked in headless by /speckit-analyze) to read and write the Nearest branch — same technique as the GIType integers,
  `viewport::Msaa::{DISABLED, MSAA_2X, MSAA_4X, MSAA_8X}`, `viewport::ScreenSpaceAa::{DISABLED, FXAA, SMAA}`,
  `rendering_server::EnvironmentSsaoQuality::{MEDIUM, HIGH}`, `EnvironmentSsilQuality::{MEDIUM, HIGH}`.
  **Confirmed (§E.1)**: Godot's integers match the `.ord()`s (`MODE_WINDOWED=0, MAXIMIZED=2,
  FULLSCREEN=3, EXCLUSIVE_FULLSCREEN=4, VSYNC_MAILBOX=3, SCALING_3D_MODE_FSR2=2, MSAA_8X=3,
  SCREEN_SPACE_AA_SMAA=2, ENV_SSAO_QUALITY_HIGH=3, ENV_SSIL_QUALITY_MEDIUM=2`). `-1` (SSAO/SSIL
  off) and `0` (unlimited fps) are `i64` literals. `Settings.GIType.*`/`GIQuality.*` → the
  literals from D5.
- `is_equal_approx(a, b)` → `godot::global::is_equal_approx(a: f64, b: f64)` (`utilities.rs:412`);
  `1.0 / 1.7` etc. as `f64`.
- `btn.button_pressed = true` → `set_pressed(true)`; `btn.button_pressed` → `is_pressed()`
  (`base_button.rs:226,235`).
- **Correction imposed by the compiler**: none besides D3 in this section; everything compiled in the
  subset (see §E.0 for what the subset covered).

## D10 — Main

- `multiplayer.server_relay = false` → `self.base().get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false)`
  (`scene_multiplayer.rs:262`; the property belongs to `SceneMultiplayer`, the concrete subclass of the
  default `MultiplayerApi` — the `cast` panics if the project used another implementation, which is not
  the case).
- `Engine.max_fps = 60` → `Engine::singleton().set_max_fps(60)` (`engine.rs:88`);
  `get_window().mode = config` → `self.base().get_window().unwrap().set_mode(window::Mode::from_ord(display_mode as i32))`
  (`window.rs:372`; `EngineEnum::from_ord(i32)`, `obj/traits.rs:201`).
- `go_to_main_menu` (`#[func]` — it is the target of `connect` by name): `let menu = ResourceLoader::singleton().load("res://menu/menu.tscn").unwrap().cast::<PackedScene>();`
  (`resource_loader.rs:89` — the original uses `ResourceLoader.load`, not the global `load()`; keep);
  `multiplayer.get_multiplayer_peer().unwrap().close()` (`multiplayer_api.rs:34`, `multiplayer_peer.rs:111`);
  `multiplayer.set_multiplayer_peer(&OfflineMultiplayerPeer::new_gd().upcast::<MultiplayerPeer>())`;
  `self.change_scene_to_packed(menu)`.
- `replace_main_scene(resource)` (`#[func]`): `self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()])`
  — by name, like the original; that is why `change_scene_to_packed` is `#[func]`.
- `change_scene_to_packed(resource)`: `let mut node = resource.instantiate().unwrap(); for mut child in self.base().get_children().iter_shared() { self.base_mut().remove_child(&child); child.queue_free(); } self.base_mut().add_child(&node);`
  (`node.rs:283`); duck typing preserved:
  `if node.has_signal("quit") { node.connect("quit", &Callable::from_object_method(&self.to_gd(), "go_to_main_menu")); }`
  (`object.rs:494`; `Object::connect(signal, &Callable) -> Error` in
  `classes/type_safe_replacements.rs:40`; `Callable::from_object_method(&Gd<T>, name)`,
  `builtin/callable.rs:50`); likewise `"replace_main_scene"`.
- **Correction imposed by the compiler**: none besides D3.

## D11 — What does NOT change (quirks preserved, FR-029)

`randomize()` in three places; `add_child(player)` without a readable name vs `add_child(robot, true)`;
`has_signal` + `connect` by name in `Main` (the original's duck typing — `Level`/`Menu` are already
Rust, but `Main` does not know them by type, exactly like the original); `call_deferred` by
name; `if/elif` chains of `_on_apply_pressed` that write nothing if no button of the row is
pressed; `pass` equivalents; `lightmap_gi` kept after `queue_free`. Backlog candidates
in §"v2 backlog candidates".

## §E — Empirical verification (2026-09-16, headless, no editor open)

### E.0 Draft compilation (feature enabled)

`ZzFlyingForklift` and `ZzMain` complete; `ZzLevel` complete (signal, dynamic Settings, 3
`setup_*`, typed spawn/respawn, `add_player`/`del_player`, `peer_*` connections, `input`);
`ZzMenu` with 20 of the 85 `OnReady` (at least one of each type/root path), signal with parameter +
emission, `process` with the threaded loader, `_make_button_group`, and the handlers `_on_play_pressed`,
`_on_settings_pressed` (one example of each read form: `.ord()` of `window::Mode`,
`VSyncMode`, `Msaa`, `ScreenSpaceAa`, `Scaling3DMode`, `EnvironmentSsaoQuality`/`Ssil`,
`is_equal_approx`, `bool`), `_on_apply_pressed` (one example of each write form + the two
dynamic `call`s), `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`,
`_on_loading_done_timer_timeout`. Result: **0 errors, 0 warnings** (`cargo clean -p` + build).

### E.1 `-s` probe (draft classes, no scene)

```
forklift: is CharacterBody3D=true is Node3D=true
level: has_signal quit=true args=0 ; GDScript connection by name + emit → received
menu: has_signal replace_main_scene=true args=1 arg0=scene class=PackedScene ; connection by name receives the PackedScene
menu has_method: _on_play_pressed, _on_settings_pressed, _on_quit_pressed, _on_host_pressed,
                 _on_connect_pressed, _on_apply_pressed, _on_loading_done_timer_timeout = true; _make_button_group = false
main has_method: go_to_main_menu, replace_main_scene, change_scene_to_packed = true
Window.MODE_WINDOWED=0 MAXIMIZED=2 FULLSCREEN=3 EXCLUSIVE=4 VSYNC_MAILBOX=3 FSR2=2 MSAA_8X=3 SMAA=2 SSAO_HIGH=3 SSIL_MEDIUM=2
```

### E.2 Baseline (commit `a866428`, all 4 scripts still in GDScript, without the feature)

`cargo build` 0 warnings; import with `Initialize godot-rust` and 0 `ERROR`.
`flying_forklift.tscn` exit 124, empty grep, 1 WARNING (HDR). `level.tscn` exit 124, empty grep,
2 WARNINGs. `menu.tscn` exit 124, empty grep, 1 WARNING (in headless the menu hosts by itself and
loads the level in a thread; without `main`, the signal `replace_main_scene` is only emitted).
`main.tscn` exit 124, 2 WARNINGs; in **6 runs**, 5 without any `ERROR` and **1 (the first,
right after `--import`) with 5 `ERROR` lines from the dummy renderer**: `Initializing already
initialized RID` (`rid_owner.h:236`), `Parameter "mem" is null.` (`rid_owner.h:284`), 3×
`Parameter "m" is null.` (`dummy/storage/mesh_storage.h:74,81,110`). The untouched original project
(`../oxide_godot_origins/`, 4 runs) has 0 `ERROR` but the same architecture (level loading
in a sub-thread with the dummy renderer); the race is the engine's in headless, not the port's.
**Rule for the quickstart**: those exact 5 lines, when they appear in headless `main.tscn`,
are not a regression **if** they disappear on a second run; any other `ERROR` line, or those
same ones reproducible in 3 consecutive runs, is a regression. To be cataloged in `CLAUDE.md` (operational
edit) together with the feature line.

## Map per script (summary for the tasks)

### 1. `flying_forklift.gd` → `src/flying_forklift.rs` — `FlyingForklift: CharacterBody3D` (D4)
### 2. `level.gd` → `src/level.rs` — `Level: Node3D` (D5–D6); `player.rs`/`red_robot.rs` visibility only (D2)
### 3. `menu.gd` → `src/menu.rs` — `Menu: Node` (D7–D9); `Cargo.toml` feature + `CLAUDE.md` line (D1)
### 4. `main.gd` → `src/main_scene.rs` — `Main: Node` (D10)

## v2 backlog candidates (record in `docs/v2-backlog.md` in the corresponding script's commit)

Items 1–18 already exist (item 1 covers typed access to `Settings`, including `GIType`/`GIQuality`). New:

| # | Origin | Improvement | Motivation |
|---|---|---|---|
| 19 | `level/forklift/flying_forklift.gd` (port 1) | Do not call `randomize()` per instance (`Main` already re-seeds at boot) | Re-seeding the global generator for every forklift is redundant and makes the pick depend on the clock at every spawn |
| 20 | `level/level.gd` (port 2) | `add_child(player, true)` as in the robot, or `spawn_robot` without a readable name — make uniform | The two spawns use different forms of `add_child`; it works because `name = str(id)` is already unique |
| 21 | `level/level.gd` (port 2) | Expose `add_player`/`del_player` with default parameter via two `#[func]`s (or remove the default) | gdext has no default parameter; v1 connects via closure, which changes the form (not the effect) of the connection |
| 22 | `menu/menu.gd` (port 3) | Declarative option→value table instead of 30 `if/elif` chains in `_on_settings_pressed`/`_on_apply_pressed` | ~250 repetitive lines; each new option requires editing two places |
| 23 | `menu/menu.gd` (port 3) | Declare `replace_main_scene` with the parameter in the original (already done in the port) and connect `quit`/`replace_main_scene` in `Main` by type instead of `has_signal` | Inherited duck typing; after v1 all scenes are Rust |
| 24 | `main/main.gd` (port 4) | `change_scene_to_packed` called directly instead of `call_deferred` by name | String call to its own method; kept in v1 for fidelity |
