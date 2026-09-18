# v2 catalog — what changes in each module

Analysis of the 16 Rust modules delivered by v1 (2 658 lines, branch `v1`) against the two
pillars of v2: **idiomatic Rust that leans on the type system** (parse, don't validate) and
**interface vs. implementation** (engine callbacks and exposed API as thin glue; domain logic in
pure Rust with no FFI). Input for the v2 specs, in the same role `docs/port-order.md` had for v1.
The rules are in the constitution; this file only says *where* each rule bites.

## Facts that shape the design

- **gdext math builtins are pure only when their methods are implemented in Rust** (constitution
  1.4.1, Principle III). The TYPES (`Vector2`, `Vector3`, `Basis`, `Quaternion`, `Transform3D`,
  ...) live in `godot::builtin` and never cross the FFI themselves, but some of their METHODS
  delegate to the engine: a method whose body goes through `as_inner()` or that is generated
  under the bindings' `out/builtin_classes/**` (e.g. `Quaternion::slerp*`, `Basis::looking_at`)
  calls the engine and belongs in glue, not the pure model. Operators, `from_quaternion`,
  `get_quaternion`, `from_euler`, `orthonormalized`, `transposed` and vector arithmetic are
  glam-based and pure. The practical check is a `#[test]`: an engine-backed method panics with
  "Godot engine not available" under `cargo test`. There is no need for "game-side" vector types.
- **These types need a running engine:** `Gd<T>` and every call on it, singletons (`Input`, `Os`,
  `RenderingServer`, ...), `Variant`, `GString`, `StringName`, `VarDictionary`, `VarArray`.
  Pure logic must not touch them; they belong to the glue.
- **The replicated/exported surface stays flat.** `SceneReplicationConfig` entries, `#[export]`
  values saved in the scenes and `node_paths` keep the original names (constitution, Principle II).
  The internal model may be richer (enums with data); the flat properties become a projection.
- **Per-frame pattern:** `snapshot` (read engine state once) → `step` (pure) → `apply` (write back).
  This is where FFI is saved and where unit tests catch regressions.
- **Timers / `await`:** v1 has 9 nested `connect_other` chains reproducing GDScript `await`.
  The first v2 plan decides once (async `godot::task` if stable in 0.5.5, `Timer` node, or keep)
  and every module follows the same choice.

## Per-module catalog

Columns: **Type system** = where the type system enters; **FFI to cut** = calls that cross the
bridge needlessly; **Pure logic** = what becomes engine-free, testable Rust; **Backlog** = items
of `docs/v2-backlog.md` closed by the change.

| Module | Type system (idiomatic / parse-don't-validate) | FFI to cut | Pure logic | Backlog |
|---|---|---|---|---|
| `settings.rs` (root) | `VarDictionary` of defaults + `get_value("rendering", "x").to::<T>()` at every use → `struct GraphicsSettings { display_mode: WindowMode, vsync: VSyncMode, gi: GiType, gi_quality: GiQuality, ssao: Option<SsaoQuality>, … }` with `Default`, `from_config(&ConfigFile)` / `to_config`; `#[constant] GI_TYPE_*` → `enum GiType` / `GiQuality` (`GodotConvert, Var, Export`); `ssao_quality == -1` → `Option`/enum (parsed once; consumers read fields) | the 5 consumers (`flying_forklift`, `bullet`, `level` ×4, `menu`, `main`) do `get_node_as::<Node>("/root/Settings").get("config_file")` + `get_value` per read → typed `Gd<Settings>` + `settings.bind().graphics` | defaults/merge, SSAO/SSIL mapping as a pure `match` returning the parameters; the engine receives the result | 1, 25 |
| `debug_label.rs` | text built through `Variant::stringify` → `format!` | text rebuilt every frame while `visible = false`; `get_multiplayer()` per frame | `fn compose(stats: &DebugStats) -> String` | 4, **26 (VRAM line right below RAM)** |
| `part_disappear.rs`, `blast.rs` | nested `connect_other` → the timer decision above | — | — | 5 |
| `camera_noise_shake.rs` | 6 module consts → tuning struct (see "Constants" below) | `noise.set_seed()` 3× per frame (one seed per axis) → 3 `FastNoiseLite` or offset in `pos` | `decay(trauma, dt)`, `shake_amount(trauma)`, `rotation_from_noise(...)` | 6 |
| `player_input.rs` | 6 × `Option<Gd<T>>` + `unwrap()` per frame → `OnEditor<Gd<T>>`; `aiming` / `toggled_aim` / `aiming_timer` → `enum AimState { Idle, Held(f32), Toggled }` (the 3 fields allow invalid combinations today) | `get_parent().cast()` 2× per frame → cached; `array![Rid::Invalid]` per frame; 10 `get_action_strength` (inherent, but read once into an input snapshot) | aim state machine (`update_aim(snapshot, dt) -> (AimState, Option<AnimCue>)`), rotation clamp, `ColorRect` fade (`alpha_for_height(y, dt, prev)`) | 7, 8, 9 |
| `player.rs` | `animate()` with 4 `if anim ==` → `match`; `player_id: i32` → newtype `PeerId` (`#[godot(transparent)]`); `set("parameters/…", Variant)` by string → named consts | `load::<PackedScene>("bullet.tscn")` **on every shot** and 2 `get_node_as` of particles per shot → `OnReady`; `self.player_input.bind()` 6× per frame → one `InputFrame` snapshot | orientation / slerp / root motion (`integrate_orientation(...)`), jump/land logic (`airborne_step`) — all on pure `Transform3D` / `Quaternion` | 10, 11, 12 |
| `bullet.rs` | `has_method("hit")` → `Hittable` (trait or `enum HitTarget` via `try_cast` to `Player` / `EnemyRobot`). Note: `rpc("hit")` by name stays (gdext has no typed rpc); what changes is the **dispatch** being by type | `get_node_as("/root/Settings")` inside `explode` | `time_alive` countdown, the "double explode" decision | 2, 13 |
| `door.rs` | `open: bool` → `enum DoorState`; `try_cast` at the signal boundary | — | trivial | 14 |
| `part.rs` | `_mat` / `_disappearing_counter` (GDScript names) → Rust names; `Option<Gd<Material>>` → resolved in `ready` | `get_node_as("Col1" / "Col2" / "MultiplayerSynchronizer")` on every `explode` → `OnReady`; `load(part_disappear.tscn)` on every `destroy` | `fade_curve(counter, disappearing_time)`, `random_angular_velocity(rng)` | 15 |
| `red_robot.rs` | `State` + 4 loose counters → `enum State { Idle, Approach { aim_preparing, shoot_countdown }, Aim { countdown }, Shooting { … } }` with a **flat projection** for the replicated `#[export]`s (`state`, `target_position`, `health`, `dead` keep their `.tscn` names); `PLAYER_AIM_TOLERANCE_DEGREES` holds radians → rename; `player: Option<Gd<Node3D>>` → `Option<Gd<Player>>` | raycast code duplicated 3× → one function; `Os::has_feature("dedicated_server")` inside `_clip_ray` **every frame** → bool at `init`; `get("parameters/aim/blend_position")` + `set` per frame; `get_global_transform()` 2–3× per frame; `get_node_as("…/LaserEmber")` per shot | `angle_to_player(basis, origin, target)`, `aim_blend_step(...)`, state transition (`step(state, dt, sees_player) -> (State, Vec<Cmd>)`) — the robot's core becomes testable without the engine | 16, 17, 18 |
| `flying_forklift.rs` | — | dynamic `Settings`; `randomize()` per instance | `pick_model(rng, n)` | 19 |
| `level.rs` | 5 consts `SDFGI` / `GI_*` disappear (use the `Settings` enums); `match gi_type` | `get_node_as("VoxelGI" / "ReflectionProbes")` repeated → `OnReady`; dynamic `Settings` 4× | `fn gi_plan(gi: GiType, q: GiQuality) -> GiPlan { sdfgi, voxel, probes, rays, … }` pure; `ready` only applies it | 20, 21 |
| `menu.rs` | ~50 `OnReady<Gd<Button>>` + ~250 lines of `if / else if` → declarative table `option enum ↔ button`; `SCALING_3D_MODE_NEAREST` local const → marked API gap | dynamic `Settings` | pure option ↔ value mapping (where tests catch a "Medium applies HIGH" slip) | 22, 23 |
| `main_scene.rs` | `has_signal` + `Callable::from_object_method` by name → typed connection (`try_cast::<Level>` / `Menu` → `.signals().quit().connect_other`); `call_deferred("…")` by string | — | `enum Scene { Menu, Level }` — the scene manager Principle I already names | 3, 23, 24 |

## Constants

v1 has 28 module-level `const`s, a direct port of GDScript `const`. Module `const` is idiomatic
Rust for a compile-time constant of local scope; the problem here is different: most of them are
**tuning parameters of one component** declared outside it (`JUMP_SPEED` is the `Player`'s, not
the module's), and the pure functions need them as input to be tested with different values.

| Kind | Examples | v2 form |
|---|---|---|
| Belongs to one type | `JUMP_SPEED`, `BULLET_VELOCITY`, `AIM_HOLD_THRESHOLD` | associated const: `Player::JUMP_SPEED` |
| Coherent tuning group | the 6 of `camera_noise_shake`, the 5 of `red_robot`, the 4 of `player_input` | `struct XxxTuning` with `Default`, a field of the node and a parameter of the pure functions |
| Designer-tunable in the scene | (none by default) | `#[export]` only with an explicit backlog item — it changes the `.tscn` surface |
| Paths / strings | `LEVEL_PATH`, `CONFIG_FILE_PATH` | stay module `const` |
| Engine API gap | `SCALING_3D_MODE_NEAREST = 5` | see below |

## Engine API gaps

`Scaling3DMode::NEAREST` exists from Godot 4.7; gdext 0.5.5 compiles against the prebuilt 4.6
API, so `menu.rs` carries a local `const SCALING_3D_MODE_NEAREST: i64 = 5` (same technique the
v1 `GIType` integers used). v2 convention:

- comment at the exact spot: `// api-gap(godot-4.7): Scaling3DMode::NEAREST absent from the gdext 0.5.5 prebuilt API (4.6); replace when the binding ships it`;
- one entry in `docs/api-gaps.md` (symbol, version that introduces it, workaround, location);
- the workaround isolated behind a typed value (`ScaleFilter::Nearest` with a `to_engine()` that
  owns the `from_ord(5)`), so replacing it is one line.

Alternative on record: gdext's `api-custom` feature generates the bindings from the local Godot
4.7.2 binary and the symbol appears. It changes the crate configuration the template generates,
so it is a user decision, not a default.

## Suggested milestones

Mirror of the v1 milestones with `Settings` moved to the front — it is the root of 5 consumers.

- **V2-A** — typed `Settings` (`GraphicsSettings`, enums) + the 5 consumers on typed access +
  `docs/api-gaps.md` + gates (`cargo test`, `cargo clippy`).
- **V2-B** — leaves (`debug_label` with VRAM, `part_disappear`, `blast`, `camera_noise_shake`) +
  `player_input`: the milestone that **fixes the interface/implementation pattern** and the timer
  decision.
- **V2-C** — `player` + `bullet` (`Hittable`) + `door`.
- **V2-D** — `part` + `red_robot` (state machine).
- **V2-E** — `level` + `menu` (option table) + `main` (scene manager).

Every user story keeps the visual checkpoints and the parity harness, now against branch `v1`.
