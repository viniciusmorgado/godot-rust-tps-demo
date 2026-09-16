# GDScript → Rust port order (v1)

Analysis of the dependency graph between the 15 scripts of the TPS demo and the migration order
derived from it. Input for the v1 specs. The rules that govern the port are in the constitution
(`.specify/memory/constitution.md`); operational details are in `CLAUDE.md`.

## Dependency graph

`A → B` means "A uses custom API of B" (method, property, enum or `is B`).
Calls that use only Godot's base API (`look_at`, `add_child`, `global_transform`, ...)
do not count: they work the same with the node in GDScript or in Rust.

```
main ──────────────────┬──→ Settings (config_file)
  │ (has_signal, duck)  │
  ├──→ menu ────────────┤──→ Settings (config_file, apply_graphics_settings, GIType/GIQuality enums — ~100 refs)
  └──→ level ───────────┤──→ Settings (config_file, enums)
         ├──→ red_robot (signal exploded)
         │      ├──→ Player (is Player, add_camera_shake_trauma)
         │      ├──→ part (explode)
         │      │      └──→ part_disappear (base API only)
         │      └──→ blast (base API only)
         ├──→ Player (player_id)
         │      ├──→ PlayerInputSynchronizer (motion/aiming/shooting/jumping/shoot_target,
         │      │      get_aim_rotation, get_camera_rotation_basis, get_camera_base_quaternion, camera_camera)
         │      ├──→ camera_noise_shake_effect (add_trauma)
         │      └──→ bullet (base API only)
         │             ├──→ Settings (config_file)
         │             └──→ red_robot / Player (hit via has_method — duck, already dynamic in the original)
         └──→ debug (nothing)
door ──→ Player (is Player)
flying_forklift ──→ Settings (config_file)
```

Leaves (depend on nobody): `debug`, `part_disappear`, `blast`,
`camera_noise_shake_effect`, `player_input`, `part`, `settings`.

## Rule that defines the order

- **GDScript → Rust** works effortlessly: `is RustClass`, calling `#[func]`, reading `#[var]`,
  connecting `#[signal]`, `.rpc()` — all dynamic and native.
- **Rust → GDScript** requires dynamic calls (`.call("x", &[..])`, `.get("y")`, comparing
  `get_script()` instead of `is Player`). It works, but it is untyped code that would be thrown away.

Therefore the port is **bottom-up**: dependencies before dependents, so that the Rust code is born
typed and never needs to call custom GDScript API.

### Exception: the `Settings` autoload

`Settings` is a leaf, but it is an autoload with ~100 references in `menu.gd` and enum access
(`Settings.GIType.SDFGI`). Ported early, `menu.gd`/`level.gd` in GDScript break (a script enum
does not exist on a native node). Ported last, no GDScript depends on it anymore.

Cost: Rust consumers (forklift, bullet, level, menu, main) access `Settings` dynamically
(`get_node("/root/Settings").get("config_file")` → from there a typed `Gd<ConfigFile>`; enums
become local integer constants). Accepted in v1; **v2 backlog: typed access to Settings**.
In Rust, `Settings` becomes a `menu/settings.tscn` scene with a root of type `Settings`, registered
in the `project.godot` autoload.

## Constraints verified in the scenes

- `MultiplayerSynchronizer` replicates script properties — they must exist as
  `#[var]`/`#[export]` with the same name:
  - `InputSynchronizer`: `aiming`, `motion`, `shooting`, `shoot_target`
  - `Player`: `current_animation`, `player_id`
  - `red_robot`: `target_position`, `health`, `state`, `dead`
  - `part`: `fade_value`
  - `bullet`: transforms only (base API)
- `[connection]`s in the scenes (names preserved via `#[func]`): `door._on_door_body_entered`,
  `red_robot._on_area_body_entered` / `_on_area_body_exited`, 11 handlers in `menu.tscn`.
- Scripts attached to inner nodes of another scene (the type swap happens in the parent scene):
  `part.gd` on 3 nodes of `red_robot.tscn` (PartShield1, PartShield2, PartHead);
  `player_input.gd` and `camera_noise_shake_effect.gd` in `player.tscn`;
  `debug.gd` in `level.tscn`.

## (1) Scripts that can come first

No dependency on another script and called by others only via base or dynamic API.
Each is testable right after the port, with the rest still in GDScript.

| # | Script | Base | Lines | How to test | What it introduces |
|---|---|---|---|---|---|
| 1 | `level/debug.gd` | `Label` | 15 | run the level, F3 toggles the FPS overlay | `process`, `Input`, `Engine`/`OS` |
| 2 | `enemies/red_robot/parts/part_disappear_effect/part_disappear.gd` | `CPUParticles3D` | 9 | kill a robot, parts vanish with a puff | `await create_timer` → timer + signal |
| 3 | `enemies/red_robot/laser/impact_effect/blast.gd` | `Node3D` | 15 | robot shoots, impact animates and vanishes | `await animation_finished`, `get_camera_3d` |
| 4 | `player/camera_noise_shake_effect.gd` | `Camera3D` | 61 | shoot, camera shakes | `FastNoiseLite`, `#[func] add_trauma` called from GDScript |
| 5 | `player/player_input.gd` | `MultiplayerSynchronizer` | 142 | move the camera, aim (toggle/hold), jump | replicated `#[export]`s, `#[rpc(call_local)]`, `_input`, raycast, `OnReady` via `node_paths` |

## (2) Full migration order

| Order | Script | Base | Scene where the type is swapped | Depends on (already in Rust) | Notes |
|---|---|---|---|---|---|
| 1 | `level/debug.gd` | Label | `level.tscn` | — | |
| 2 | `part_disappear.gd` | CPUParticles3D | `part_disappear.tscn` | — | |
| 3 | `blast.gd` | Node3D | `impact_effect.tscn` | — | |
| 4 | `camera_noise_shake_effect.gd` | Camera3D | `player.tscn` (Camera3D) | — | |
| 5 | `player_input.gd` | MultiplayerSynchronizer | `player.tscn` (InputSynchronizer) | — | `class_name PlayerInputSynchronizer`; the GDScript Player calls it dynamically |
| 6 | `player/player.gd` | CharacterBody3D | `player.tscn` (root) | 5, 4 | `class_name Player`; `player_id` with setter; `#[rpc]` jump/land/shoot/hit; instantiates bullet (base) |
| 7 | `player/bullet/bullet.gd` | CharacterBody3D | `bullet.tscn` | Settings (dynamic) | dynamic `has_method("hit")` + `rpc("hit")`, like the original |
| 8 | `door/door.gd` | Area3D | `door.tscn` | 6 | `try_cast::<Player>()` |
| 9 | `enemies/red_robot/parts/part.gd` | RigidBody3D | `red_robot.tscn` (3 nodes) | 2 | `fade_value` setter touches a shader; the GDScript red_robot calls `explode()` dynamically |
| 10 | `enemies/red_robot/red_robot.gd` | CharacterBody3D | `red_robot.tscn` (root) | 6, 9, 3 | largest script (283 l); state machine, `#[signal] exploded`, `#[rpc] hit/play_shoot`, `await` → timers |
| 11 | `level/forklift/flying_forklift.gd` | Node3D | `flying_forklift.tscn` | Settings (dynamic) | trivial; fits at any point after 1 |
| 12 | `level/level.gd` | Node3D | `level.tscn` (root) | 10, 6, Settings (dynamic) | typed spawn `Gd<RedRobot>`/`Gd<Player>`; `#[signal] quit`; `_input`; multiplayer peer signals |
| 13 | `menu/menu.gd` | Node | `menu.tscn` | Settings (dynamic) | 460 l, mechanical: ~50 `OnReady` + 11 `#[func]` handlers; `#[signal] replace_main_scene`; `load_threaded` |
| 14 | `main/main.gd` | Node | `main.tscn` | 13, 12 (signals) | `has_signal` duck → `try_cast` on the two Rust types |
| 15 | `menu/settings.gd` | Node (autoload) | new `menu/settings.tscn` + `project.godot` | — | last: no GDScript remains; enums become `#[constant]` |

After 15 no `.gd` remains — v1 completion criterion.

## Suggested milestones (grouping for specs)

- **A** — 1 to 5: leaves and input.
- **B** — 6 to 8: complete player (player, bullet, door).
- **C** — 9 to 10: enemy (part, red_robot).
- **D** — 11 to 15: level, menu, main, settings — the whole game in Rust.

## v2 backlog identified in this analysis

Recorded in `docs/v2-backlog.md` (items 1–3).

- Typed access to the `Settings` autoload (replace `get_node("/root/Settings").get(...)`).
- Replace `bullet`'s dynamic `has_method("hit")` + rpc with a target trait/enum.
- Replace `main`'s `has_signal` duck typing with a scene manager.
