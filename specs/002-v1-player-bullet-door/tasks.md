# Tasks: Milestone B — player, bullet and door (v1 raw port)

**Input**: Design documents from `/specs/002-v1-player-bullet-door/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D16, §E), data-model.md, contracts/, quickstart.md (all approved)

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.0). No task may introduce
abstraction, refactoring, optimization, unit tests or infrastructure. If something like that seems
necessary, it becomes an entry in `docs/v2-backlog.md`, not a task. This milestone uses **a single time** the
conservative upstream bug fix clause (US3, door); no other fix.

**Tests**: there are no automated tests in this phase (plan.md "Testing"). The validation of each story
is the quickstart cycle (build → headless import → headless scene → mechanical checks →
contract → user visual validation).

**Organization**: one phase per user story, in the mandatory order US1 → US2 → US3 (spec FR-028;
`docs/port-order.md` items 6 → 7 → 8). The stories are **not** parallelizable with each other: each one
ends with its own commit on `main` and the next one starts from the clean tree; US3 depends on
`Player` in Rust (`is Player`) and US2 is instantiated by the Player (only via base API, but the game has
to be playable after each commit).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task) — rare
  in this milestone, because build depends on the module, scene depends on the build, validation depends on the scene.
- **[Story]**: US1..US3 (spec.md)
- Paths relative to the repository root (`oxide-godot/` = Godot project;
  `oxide_godot_core/oxide_godot_lib/src/` = Rust crate).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` of each module (only that)
oxide_godot_core/oxide_godot_lib/src/<module>.rs   one class per script (research.md §"Map per script")
oxide_godot_core/oxide_godot_lib/src/player_input.rs, camera_noise_shake.rs   Milestone A — only visibility changes (US1)
oxide-godot/<scene>.tscn                           `type` swap (plan.md "Scene editing")
oxide-godot/<script>.gd + .gd.uid                  deleted in the port commit
docs/v2-backlog.md                                 one line per noticed improvement (items 10–14)
docs/upstream-bugs.md                              NEW in US3: register of the door fix
../oxide_godot_origins/                            untouched reference of the original GDScript
```

Closed decisions (apply to all stories — instruction, not option; details in research.md):

- `Player` access to `PlayerInputSynchronizer`/`CameraNoiseShake` is **typed** (`bind()`/
  `bind_mut()`/`cast::<CameraNoiseShake>()`); for that, in the **same commit** as the Player, only the
  visibility of the items listed in T006 changes to `pub(crate)` — no other line of those
  files (D1).
- `player_id`: `#[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32`; setter
  `#[func]` in the **main** `#[godot_api] impl Player` block, using
  `self.base().get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")` — **not** the `OnReady`
  (runs before `ready`) (D2).
- `current_animation`: `enum Animations { JumpUp, JumpDown, Strafe, Walk }` with
  `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)]`;
  field `#[export] #[init(val = Animations::Walk)]` (D3). `motion`: `#[var] motion: Vector2` (D4).
- `animate` and `apply_input` **private, without `#[func]`**; `ShootParticle`/`MuzzleFlash` via
  `get_node_as::<CpuParticles3D>` inside `shoot`; sounds via `#[init(node = "SoundEffects/Jump")]`
  etc.; `crosshair` declared and unused (D5).
- Bullet instantiated as `Gd<CharacterBody3D>` (`load::<PackedScene>(..).instantiate_as::<CharacterBody3D>()`
  at the point of use; `add_child_ex(&bullet).force_readable_name(true).done()`;
  `add_collision_exception_with(&self.to_gd())`). **Never** `instantiate_as::<Bullet>` (D11).
- Axes: `basis.col_c()` = `basis.z`, `basis.col_a()` = `basis.x`; `Basis::looking_at(target)`;
  `Basis::from_quaternion(q)`; `q.slerp(q_to, w as f32)`; `Transform3D::new(basis, origin)`;
  `orientation * root_motion`; `player_model.set_global_basis(..)` (D9).
- RPCs `#[rpc(authority, call_local, unreliable)]`: `jump`, `land`, `shoot`, `hit`,
  `add_camera_shake_trauma` (Player); `explode` (Bullet). `shoot`/`hit` call
  `self.add_camera_shake_trauma(x)` **directly**; firing via `self.base_mut().rpc("name", &[])` (D7).
- Bullet: `has_method("hit")` + `rpc("hit", &[])` on the collider (D13); `Settings` via
  `get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>()` (D14);
  `#[func] destroy` (D12).
- Door: `#[init(node = "DoorModel2/AnimationPlayer")]` with the comment `// upstream bug fix: ...`
  on the line **immediately above**; `body.try_cast::<Player>().is_ok()` without `clone()`;
  `#[godot_api] impl IArea3D for Door {}` empty (D15).
- `docs/upstream-bugs.md`: created in US3; "commit" column = commit **subject**; **no**
  `git commit --amend`.
- Types: `delta: f64` in the virtuals; constants and `airborne_time`/`time_alive` in `f32`; `as f32`
  at the point of use; `amount: f64` in the RPCs (like `add_trauma`).
- One commit per script on `main`, message in the format of quickstart step 8; fix after checkpoint
  = commit `Fix port …`.

---

## Phase 1: Setup

**Purpose**: record the baseline against which each port is compared — including the **measurement of
the door error**, which must disappear in US3. No infrastructure (Principle I).

- [x] T001 Confirm preconditions: no Godot editor open (`pgrep -a godot` empty — if there is one, warn the user and wait, never kill); `git status --short` empty on `main`; note `git rev-parse --short HEAD`. The base commit `108584e` used by `quickstart.md`/T040 for `git diff -- '*.gd'` and `git log 108584e..HEAD` remains valid: later commits (plan, tasks) touch no `.gd` nor are "Port …" commits; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 10
- [x] T002 Record the build baseline: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → expected `0`; note in `specs/002-v1-player-bullet-door/quickstart.md` §"Baseline" if it differs
- [x] T003 Record the import baseline: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirm `grep -n 'Initialize godot-rust' /tmp/import.log` (line 1) and `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` empty (or only the 3 upstream errors from `CLAUDE.md`)
- [x] T004 [P] Confirm the generated bindings directory used by research.md: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` → expected `.../godot-core-aea5c50e7fda9d57/out`; if the hash differs, update line 6 of `specs/002-v1-player-bullet-door/research.md` (the cited lines remain valid — same version 0.5.5)
- [x] T005 **Measure the door baseline** (quickstart §"Baseline"): `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn 2>&1 | tee /tmp/door_baseline.log`; expected exit 124, `grep -c 'Node not found' /tmp/door_baseline.log` = **1** (`ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`), and no other `ERROR` line. Also confirm `grep -n 'Node not found' CLAUDE.md` empty (the error was never in the catalog). If the count is not 1, stop and report — US3 depends on this measurement

**Checkpoint**: baseline known — 0 warnings, extension loads, 0 `ERROR` on import, door with exactly 1 `Node not found`.

---

## Phase 2: Foundational

**Does not exist in this milestone.** The three ports are strictly sequential (player → bullet → door) and
the only shared edit is the `mod <module>;` line in `lib.rs`, done inside each story.
The `pub(crate)` visibility change in `player_input.rs`/`camera_noise_shake.rs` **belongs to
US1** (it is the Player that requires it and it goes into the Player commit), not to a common phase. No
common module, helper, trait or shared constant may be created (Principle I).

---

## Phase 3: User Story 1 — Ported player (Priority: P1) 🎯 MVP

**Goal**: `player/player.gd` (211 l., `class_name Player`) → `Player: CharacterBody3D`; name
**mandatory** (`is Player` in `red_robot.gd:131,275,281` and `door.gd:10`); `player_id` with a setter
that runs outside the tree; `current_animation` enum and `motion` replicable by name; 5 RPCs;
typed access to `PlayerInputSynchronizer` and `CameraNoiseShake`; bullet instantiated via base API.

**Independent Test**: `player/player.tscn` and `level/level.tscn` headless with no `ERROR`; contract
`contracts/player.md` checked; in the game, move/jump/land/aim/shoot/shake/respawn
indistinguishable from the original; `red_robot.gd` and `level.gd` keep finding `Player`,
`player_id`, `add_camera_shake_trauma` with no edits.

- [x] T006 [US1] Change **only the visibility** in `oxide_godot_core/oxide_godot_lib/src/player_input.rs`: fields `aiming`, `shoot_target`, `motion`, `shooting`, `jumping`, `camera_camera` (l.33–54) and methods `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis` (l.184, 203, 213) receive `pub(crate)` (`pub(crate) aiming: bool`, `pub(crate) fn get_aim_rotation(&self) -> f64`, …); in `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs`, `fn add_trauma` (l.52) → `pub(crate) fn add_trauma`. `#[export]`/`#[func]` attributes stay where they are. Check: `git diff --stat` = `player_input.rs | 18 +++++++++---------`, `camera_noise_shake.rs | 2 +-` (10 −/+ pairs, nothing else) (research D1)
- [x] T007 [US1] Create `oxide_godot_core/oxide_godot_lib/src/player.rs` — part 1 (declarations), translating `oxide-godot/player/player.gd:1-43`: `use godot::classes::{AnimationTree, AudioStreamPlayer, CharacterBody3D, CpuParticles3D, ICharacterBody3D, Marker3D, MultiplayerSynchronizer, Node3D, PackedScene, TextureRect, Timer}; use godot::prelude::*; use crate::camera_noise_shake::CameraNoiseShake; use crate::player_input::PlayerInputSynchronizer;`; `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum Animations { JumpUp, JumpDown, Strafe, Walk }`; `f32` consts: `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`, `MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct Player { base: Base<CharacterBody3D>, #[init(val = 100.0)] airborne_time: f32, orientation: Transform3D, root_motion: Transform3D, #[var] motion: Vector2, initial_position: Vector3, #[init(node = "InputSynchronizer")] player_input: OnReady<Gd<PlayerInputSynchronizer>>, #[init(node = "AnimationTree")] animation_tree: OnReady<Gd<AnimationTree>>, #[init(node = "PlayerModel")] player_model: OnReady<Gd<Node3D>>, #[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom")] shoot_from: OnReady<Gd<Marker3D>>, #[init(node = "Crosshair")] crosshair: OnReady<Gd<TextureRect>>, #[init(node = "FireCooldown")] fire_cooldown: OnReady<Gd<Timer>>, #[init(node = "SoundEffects/Jump")] sound_effect_jump: OnReady<Gd<AudioStreamPlayer>>, #[init(node = "SoundEffects/Land")] sound_effect_land: OnReady<Gd<AudioStreamPlayer>>, #[init(node = "SoundEffects/Shoot")] sound_effect_shoot: OnReady<Gd<AudioStreamPlayer>>, #[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32, #[export] #[init(val = Animations::Walk)] current_animation: Animations }`. Keep the original's comments translated where they exist (research D2–D5)
- [x] T008 [US1] `player.rs` — part 2 (virtuals and private), translating `player.gd:46-176` line by line: `#[godot_api] impl ICharacterBody3D for Player` with `fn ready(&mut self)` (`self.initial_position = self.base().get_transform().origin; self.orientation = self.player_model.get_global_transform(); self.orientation.origin = Vector3::ZERO; if !self.base().get_multiplayer().unwrap().is_server() { self.base_mut().set_process(false); }`) and `fn physics_process(&mut self, delta: f64)` (`if is_server { self.apply_input(delta) } else { let anim = self.current_animation; self.animate(anim, delta) }`); `impl Player` block **without** `#[godot_api]` with `fn animate(&mut self, anim: Animations, _delta: f64)` (`self.current_animation = anim;` + the 4 branches with `self.animation_tree.set("parameters/state/transition_request", &"jump_up".to_variant())` etc.; STRAFE: `let aim = self.player_input.bind().get_aim_rotation(); set("parameters/aim/add_amount", &aim.to_variant()); set("parameters/strafe/blend_position", &Vector2::new(self.motion.x, -self.motion.y).to_variant())`; WALK: `set("parameters/aim/add_amount", &0.to_variant()); set(".../transition_request", &"walk".to_variant()); set("parameters/walk/blend_position", &Vector2::new(self.motion.length(), 0.0).to_variant())`) and `fn apply_input(&mut self, delta: f64)`: lerp of `motion` (`self.motion.lerp(self.player_input.bind().motion, MOTION_INTERPOLATE_SPEED * delta as f32)`); `camera_basis = self.player_input.bind().get_camera_rotation_basis()`, `camera_z = camera_basis.col_c()`, `camera_x = camera_basis.col_a()`, zero `.y` and `normalized()`; `airborne_time += delta as f32`; `is_on_floor()` → `if airborne_time > 0.5 { self.base_mut().rpc("land", &[]) }; airborne_time = 0.0`; `on_air = airborne_time > MIN_AIRBORNE_TIME`; jump (`if !on_air && self.player_input.bind().jumping { velocity.y = JUMP_SPEED via get/set_velocity; on_air = true; airborne_time = MIN_AIRBORNE_TIME; rpc("jump") }`); `self.player_input.bind_mut().jumping = false;`; air branch (`velocity.y > 0.0` → `animate(JumpUp)` otherwise `animate(JumpDown)`); aim branch (`q_from = self.orientation.basis.get_quaternion(); q_to = self.player_input.bind().get_camera_base_quaternion(); self.orientation.basis = Basis::from_quaternion(q_from.slerp(q_to, delta as f32 * ROTATION_INTERPOLATE_SPEED)); animate(Strafe); root_motion = Transform3D::new(Basis::from_quaternion(animation_tree.get_root_motion_rotation()), animation_tree.get_root_motion_position());` shot if `self.player_input.bind().shooting && self.fire_cooldown.get_time_left() == 0.0`: `shoot_origin = self.shoot_from.get_global_transform().origin; shoot_target = self.player_input.bind().shoot_target; shoot_dir = (shoot_target - shoot_origin).normalized(); let mut bullet: Gd<CharacterBody3D> = load::<PackedScene>("res://player/bullet/bullet.tscn").instantiate_as::<CharacterBody3D>(); self.base().get_parent().unwrap().add_child_ex(&bullet).force_readable_name(true).done(); bullet.set_global_position(shoot_origin); bullet.look_at(shoot_origin + shoot_dir); bullet.add_collision_exception_with(&self.to_gd()); self.base_mut().rpc("shoot", &[]);`); walk branch (`target = camera_x * motion.x + camera_z * motion.y; if target.length() > 0.001 { slerp toward Basis::looking_at(target).get_quaternion() }; animate(Walk); root_motion = ...`); afterwards: `self.orientation = self.orientation * self.root_motion; h_velocity = self.orientation.origin / delta as f32; velocity.x/z = h_velocity.x/z; velocity += self.base().get_gravity() * delta as f32; set_velocity; set_up_direction(Vector3::UP); move_and_slide(); self.orientation.origin = Vector3::ZERO; self.orientation = self.orientation.orthonormalized(); let basis = self.orientation.basis; self.player_model.set_global_basis(basis); if self.base().get_transform().origin.y < -40.0 { let mut t = self.base().get_transform(); t.origin = self.initial_position; self.base_mut().set_transform(t); }`. `bind()` guards always temporary (one expression) (research D6–D11)
- [x] T009 [US1] `player.rs` — part 3 (Godot block), translating `player.gd:38-41,179-211`: **single** `#[godot_api] impl Player` with `#[func] fn set_player_id(&mut self, value: i32) { self.player_id = value; self.base().get_node_as::<MultiplayerSynchronizer>("InputSynchronizer").set_multiplayer_authority(value); }`; `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.animate(Animations::JumpUp, 0.0); self.sound_effect_jump.play(); }`; `land` likewise with `JumpDown`/`sound_effect_land`; `shoot`: `let mut shoot_particle = self.base().get_node_as::<CpuParticles3D>("PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle"); shoot_particle.restart(); shoot_particle.set_emitting(true);` likewise `muzzle_particle` at `.../ShootFrom/MuzzleFlash`; `self.fire_cooldown.start(); self.sound_effect_shoot.play(); self.add_camera_shake_trauma(0.35);`; `hit`: `self.add_camera_shake_trauma(0.75);`; `add_camera_shake_trauma(&mut self, amount: f64)`: `let camera = self.player_input.bind().camera_camera.clone().unwrap(); camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);`. No other `#[func]` (research D2, D6, D7)
- [x] T010 [US1] Add `mod player;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod player_input;`; nothing else changes in lib.rs)
- [x] T011 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. In case of a compilation error, consult research.md D1–D11 and the bindings in `oxide_godot_core/target/debug/build/godot-core-*/out/classes/` before improvising; do not change `Cargo.toml`
- [x] T012 [US1] Edit `oxide-godot/player/player.tscn` as text (re-verify with `grep -n 'name="Player" type=\|^script = ExtResource("1")\|player.gd' oxide-godot/player/player.tscn`): on the root line (≈333) replace `type="CharacterBody3D"` with `type="Player"`; remove the line `script = ExtResource("1")` (≈336); remove the line `[ext_resource type="Script" uid="uid://ctlx3bqglonsx" path="res://player/player.gd" id="1"]` (l.3). **KEEP** the root's `collision_layer = 6`/`collision_mask = 7`, `ServerSynchronizer` with `replication_config`, `InputSynchronizer` (already `PlayerInputSynchronizer`, `node_paths`, 6 `NodePath`), `BulletCache` and the `[editable path=...]`. Nothing else changes (plan.md "Scene editing", port 1)
- [x] T013 [US1] Delete `oxide-godot/player/player.gd` and `oxide-godot/player/player.gd.uid` (`git rm`); verify `grep -rn 'uid://ctlx3bqglonsx' oxide-godot/` empty and `grep -rn 'player.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T014 [US1] Headless validation (quickstart §2–3; no editor open): import with `Initialize godot-rust` and no new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` and then `level/level.tscn` → exit 124 expected; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log` empty in both (only the 2 `WARNING`s from the baseline). The level spawns the Player with `name`/`player_id` outside the tree and `red_robot.gd`/`level.gd` resolve `Player`, `player_id`, `add_camera_shake_trauma` by name — any wrong name shows up here
- [x] T015 [US1] Mechanical checks and contract (quickstart §4–5; `contracts/player.md` §"Verification before the commit"): `grep -n 'type="Player"' oxide-godot/player/player.tscn` = 1 line; `grep -n 'ExtResource("1")' oxide-godot/player/player.tscn` empty; `grep -n 'properties/[0-9]/path' oxide-godot/player/player.tscn | head -5` lists `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`, `.:current_animation` — the three from the script exist in `player.rs` with the same name; `grep -rn 'is Player\|player_id\|add_camera_shake_trauma\|\.hit\.rpc\|has_method(&"hit")' --include=*.gd oxide-godot` returns only `red_robot.gd:131,133,275,281`, `level.gd:119`, `bullet.gd:31,32`, `door.gd:10`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `player/player.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player_input.rs oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs | grep '^[-+]' | grep -v '^[-+][-+]' | grep -v 'pub(crate)'` shows only the 10 original `-` lines (no other change); `ls oxide_godot_core/oxide_godot_lib/src/` = 7 files (lib.rs + 6 modules)
- [x] T016 [US1] Record in `docs/v2-backlog.md` lines no. 10, 11 and 12 (research.md §"Candidate v2 backlog"): 10 — `player/player.gd` (port 1): start `airborne_time` at 0 / ignore the first landing (with initial 100 the first contact triggers `land` and the sound on spawn); 11 — zero `velocity` on respawn below −40 (the teleport preserves the fall velocity); 12 — remove/use the never-read `crosshair` reference (`player.gd:30`). Do not duplicate the existing items 1–9 (9 already covers `jumping`)
- [x] T017 [US1] Single commit of port 1 on `main`, including `src/player.rs`, `src/lib.rs`, `src/player_input.rs`, `src/camera_noise_shake.rs`, `oxide-godot/player/player.tscn`, the deletions of `player.gd`/`player.gd.uid` and `docs/v2-backlog.md`. Message: `Port player.gd → Player (CharacterBody3D); player.tscn: node Player type="CharacterBody3D"→"Player"` + body with: notes (`player_id` setter outside the tree via `get_node_as`; `Animations` enum `via = i64`; `motion` `#[var]`; bullet instantiated as `CharacterBody3D` via base API; preserved quirks: `airborne_time = 100`, `jumping` zeroed per frame, `velocity` on respawn, `crosshair` unused); `- player_input.rs / camera_noise_shake.rs: only pub(crate) visibility on the fields/methods consumed by the Player (typed access, FR-010/FR-011); no logic moved`; `- v2 backlog: items 10, 11, 12`. Note the hash
- [x] T018 [US1] **User checkpoint (visual validation, plan.md "Visual validation" port 1)** — done by the user in the editor/game, comparing with `../oxide_godot_origins/`: enter the level → **immediate landing sound on spawn** (quirk); move in all directions with the model turning toward the camera; jump (Jump sound, rising/falling animation) and land after > 0.5 s in the air (Land sound); aim/strafe (aim follows the pitch); shoot (visible bullet — still GDScript —, muzzle flash, 0.4 s cooldown, light shake); robot hits → strong shake (13.0); fall below −40 → respawn at the initial point; F3 and everything from Milestone A still work. When opening `player.tscn`: root of type `Player`, no script; `ServerSynchronizer`/`InputSynchronizer`/`BulletCache` intact. Divergence → commit `Fix port player.gd …` in the same story. Only proceed to US2 with the explicit OK

**Checkpoint**: 9 `.gd` remaining; `player.tscn` with `Player`; game playable.

---

## Phase 4: User Story 2 — Ported bullet (Priority: P2)

**Goal**: `player/bullet/bullet.gd` (51 l.) → `Bullet: CharacterBody3D`; flies at 20 u/s, explodes on
collision (calling `hit` via duck typing) or at 5 s; `explode` RPC reads `Settings.config_file`
(first use of the exception); `destroy()` via the method track; `BulletCache` in `player.tscn` now
instantiates the Rust class.

**Independent Test**: `player/bullet/bullet.tscn` headless (expires at 5 s → `explode` →
`Settings` → `destroy` within the 20 s), `player/player.tscn` (`BulletCache`) and
`level/level.tscn` with no `ERROR`; contract `contracts/bullet.md`; in the game, the visible bullet explodes on
walls, on the robot (which reacts) and on expiry.

- [x] T019 [US2] Create `oxide_godot_core/oxide_godot_lib/src/bullet.rs` translating `oxide-godot/player/bullet/bullet.gd` line by line: `use godot::classes::{AnimationPlayer, CharacterBody3D, CollisionShape3D, ConfigFile, ICharacterBody3D, KinematicCollision3D, Node, Node3D, OmniLight3D}; use godot::prelude::*;`; `const BULLET_VELOCITY: f32 = 20.0;`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct Bullet { base: Base<CharacterBody3D>, #[init(val = 5.0)] time_alive: f32, hit: bool, #[init(node = "AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>, #[init(node = "CollisionShape3D")] collision_shape: OnReady<Gd<CollisionShape3D>>, #[init(node = "OmniLight3D")] omni_light: OnReady<Gd<OmniLight3D>> }`; `#[godot_api] impl ICharacterBody3D for Bullet`: `ready` (`if !is_server { self.base_mut().set_physics_process(false); self.collision_shape.set_disabled(true); }`), `physics_process(&mut self, delta: f64)` (`if self.hit { return; } self.time_alive -= delta as f32; if self.time_alive < 0.0 { self.hit = true; self.base_mut().rpc("explode", &[]); } let displacement = -(delta as f32) * BULLET_VELOCITY * self.base().get_transform().basis.col_c(); let col: Option<Gd<KinematicCollision3D>> = self.base_mut().move_and_collide(displacement); if let Some(col) = col { let collider: Option<Gd<Node3D>> = col.get_collider().and_then(|c| c.try_cast::<Node3D>().ok()); if let Some(mut collider) = collider { if collider.has_method("hit") { collider.rpc("hit", &[]); } } self.collision_shape.set_disabled(true); self.base_mut().rpc("explode", &[]); self.hit = true; }`); `#[godot_api] impl Bullet`: `#[rpc(authority, call_local, unreliable)] fn explode(&mut self)` (`self.animation_player.play_ex().name("explode").done();` + the original's comment + `let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>(); if config_file.get_value("rendering", "shadow_mapping").to::<bool>() { self.omni_light.set_shadow(true); }`) and `#[func] fn destroy(&mut self)` (`if !is_server { return; } self.base_mut().queue_free();`). No other `#[func]` (research D10, D12–D14)
- [x] T020 [US2] Add `mod bullet;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod player;`)
- [x] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [x] T022 [US2] Edit `oxide-godot/player/bullet/bullet.tscn` as text (re-verify with `grep -n 'name="Bullet" type=\|^script = ExtResource("1")\|bullet.gd' oxide-godot/player/bullet/bullet.tscn`): on the root line (≈481) replace `type="CharacterBody3D"` with `type="Bullet"`; remove `script = ExtResource("1")` (≈485); remove `[ext_resource type="Script" uid="uid://iybteh2g0be4" path="res://player/bullet/bullet.gd" id="1"]` (l.3). **KEEP** the root's `transform`/`collision_layer = 0`/`collision_mask = 3`, `MultiplayerSynchronizer` with `replication_config` (`.:global_transform`) and the method track `destroy` (`tracks/1/…`, ≈l.93–105). Nothing else changes (plan.md "Scene editing", port 2)
- [x] T023 [US2] Delete `oxide-godot/player/bullet/bullet.gd` and `oxide-godot/player/bullet/bullet.gd.uid` (`git rm`); verify `grep -rn 'uid://iybteh2g0be4' oxide-godot/` empty and `grep -rn 'bullet.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T024 [US2] Headless validation (no editor open): import with `Initialize godot-rust` and no new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . player/bullet/bullet.tscn 2>&1 | tee /tmp/run.log` → exit 124 or 0; regression grep empty — the bullet expires at 5 s, `explode` runs (reads `/root/Settings`, which **is** loaded in `--path`) and `destroy` at 1.5 s of the animation does `queue_free` with no error; repeat for `player/player.tscn` (`BulletCache` instantiates `Bullet`) and `level/level.tscn`; regression grep empty in all three (only the `WARNING`s from the baseline)
- [x] T025 [US2] Mechanical checks and contract (`contracts/bullet.md` §"Verification before the commit"): `grep -n 'type="Bullet"' oxide-godot/player/bullet/bullet.tscn` = 1 line; `grep -n 'ExtResource("1")' oxide-godot/player/bullet/bullet.tscn` empty; `grep -n '"method": &"destroy"\|tracks/1/type' oxide-godot/player/bullet/bullet.tscn` = 2 lines (name `destroy` identical to the `#[func]`); `grep -n 'properties/0/path' oxide-godot/player/bullet/bullet.tscn` = `.:global_transform`; `grep -rn 'explode\|destroy' --include=*.gd oxide-godot` empty; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `bullet.gd`; `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs` empty (the Player does not change in port 2 — it types the bullet as `CharacterBody3D`); `ls src/` = 8 files
- [x] T026 [US2] Record in `docs/v2-backlog.md` line no. 13: `player/bullet/bullet.gd` (port 2) — avoid the double `explode` when `time_alive` expires and there is a collision in the same frame (two `explode` RPCs restart the animation). Do not duplicate items 1 (typed Settings) and 2 (`Hittable`), which already cover the other points of this bullet
- [x] T027 [US2] Single commit of port 2 on `main`, including `src/bullet.rs`, `src/lib.rs`, `oxide-godot/player/bullet/bullet.tscn`, the deletions of `bullet.gd`/`bullet.gd.uid` and `docs/v2-backlog.md`. Message: `Port bullet.gd → Bullet (CharacterBody3D); bullet.tscn: node Bullet type="CharacterBody3D"→"Bullet"` + body: notes (duck typing `has_method("hit")` + `rpc("hit")` preserved; first use of the `Settings` exception via `/root/Settings` → `Gd<ConfigFile>`; `destroy` via the method track; `BulletCache` of `player.tscn` now instantiates the Rust class; preserved quirk: double `explode` possible); `- v2 backlog: item 13`. Note the hash
- [x] T028 [US2] **User checkpoint (visual validation, plan.md port 2)** — done by the user in the game, comparing with the original: visible blue bullet leaves the barrel and flies straight; explodes on walls/floor (animation + light) and on hitting the robot (robot reacts to `hit`); a stray bullet explodes on its own after 5 s; disappears after the explosion; with `Shadow mapping` enabled in the settings, the explosion light casts a shadow; console with no error at spawn (`BulletCache`). Divergence → commit `Fix port bullet.gd …`. Only proceed to US3 with the explicit OK

**Checkpoint**: 8 `.gd` remaining; `bullet.tscn` with `Bullet`; game playable.

---

## Phase 5: User Story 3 — Ported door, with conservative upstream bug fix (Priority: P3)

**Goal**: `door/door.gd` (12 l.) → `Door: Area3D`; opens once (`doorsimple_opening`) when a
`Player` enters. **The milestone's only bug fix**: reference `DoorModel2/AnimationPlayer` instead
of the non-existent `DoorModel/AnimationPlayer` (FR-030–FR-035), with the 4 requirements of the clause:
(a) spec ✓, (b) comment `// upstream bug fix` at the exact spot, (c) commit message, (d)
`docs/upstream-bugs.md`.

**Independent Test**: `door/door.tscn` headless with `grep -c 'Node not found'` = **0** (baseline
T005 = 1) and regression grep empty; `level/level.tscn` unchanged; contract `contracts/door.md`;
`docs/upstream-bugs.md` with 1 entry; `CLAUDE.md` untouched.

- [x] T029 [US3] Create `oxide_godot_core/oxide_godot_lib/src/door.rs` translating `oxide-godot/door/door.gd`: `use godot::classes::{AnimationPlayer, Area3D, IArea3D, Node3D}; use godot::prelude::*; use crate::player::Player;`; `#[derive(GodotClass)] #[class(init, base=Area3D)] pub struct Door { base: Base<Area3D>, open: bool, ` + the two comment lines **immediately above** the attribute: `// upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (a node that does not exist);` / `// the scene node is "DoorModel2" — the door never opened and Godot printed "Node not found".` + `#[init(node = "DoorModel2/AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>> }`; `#[godot_api] impl IArea3D for Door {}` (empty); `#[godot_api] impl Door { #[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>) { if !self.open && body.try_cast::<Player>().is_ok() { self.animation_player.play_ex().name("doorsimple_opening").done(); self.open = true; } } }`. Nothing beyond that: do not rename anything, do not "improve" the `open` logic (FR-031, research D15)
- [x] T030 [US3] Add `mod door;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod bullet;`)
- [x] T031 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [x] T032 [US3] Edit `oxide-godot/door/door.tscn` as text (re-verify with `grep -n 'name="Door" type=\|^script = ExtResource("1")\|door.gd\|DoorModel2\|_on_door_body_entered' oxide-godot/door/door.tscn`): on the root line (l.10) replace `type="Area3D"` with `type="Door"`; remove `script = ExtResource("1")` (l.11); remove `[ext_resource type="Script" uid="uid://7v3r683kok5s" path="res://door/door.gd" id="1"]` (l.3). **KEEP** `[node name="DoorModel2" ...]` (l.13 — **do NOT rename**), `AnimationPlayer` (l.25, `autoplay = &"doorsimple_closed"`), `sound`, `CollisionShape3D`, the `[connection signal="body_entered" from="." to="." method="_on_door_body_entered"]` (l.37) and `[editable path="DoorModel2"]`. Nothing else changes (plan.md "Scene editing", port 3)
- [x] T033 [US3] Delete `oxide-godot/door/door.gd` and `oxide-godot/door/door.gd.uid` (`git rm`); verify `grep -rn 'uid://7v3r683kok5s' oxide-godot/` empty and `grep -rn 'door.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T034 [US3] Headless validation (no editor open): import with `Initialize godot-rust` and no new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn 2>&1 | tee /tmp/run.log` → exit 124; regression grep empty **and** `grep -c 'Node not found' /tmp/run.log` = **0** (baseline T005 = 1 — the upstream error disappeared); `level/level.tscn` with regression grep empty. If `Node not found` is still 1, stop: the fix was not applied at the right spot
- [x] T035 [US3] Mechanical checks and contract (`contracts/door.md` §"Verification before the commit"): `grep -n 'type="Door"' oxide-godot/door/door.tscn` = 1 line; `grep -n 'ExtResource("1")' oxide-godot/door/door.tscn` empty; `grep -n 'method="_on_door_body_entered"' oxide-godot/door/door.tscn` = 1 (name identical to the `#[func]`); `grep -n 'name="DoorModel2"\|name="AnimationPlayer" parent="DoorModel2"' oxide-godot/door/door.tscn` = 2 untouched lines; `grep -c 'doorsimple_opening' oxide-godot/door/model/door.dae` = 1; `grep -n 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/door.rs` = 1 line, immediately above `#[init(node = "DoorModel2/AnimationPlayer")]`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `door.gd`; `git diff --stat HEAD -- CLAUDE.md` empty; `ls src/` = 9 files
- [x] T036 [US3] Create `docs/upstream-bugs.md` (requirement (d), FR-034): title `# Upstream bugs fixed in v1`, paragraph explaining the purpose (record required by constitution v1.3.0, Principle I — reference for v2 and for an eventual upstream contribution; each entry = conservative fix declared in the feature spec) and table `| # | Defect | Script / scene | Fix applied | Commit |` with entry `1`: defect `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` — `door.gd:6` referenced a non-existent node; the door never opened; script/scene `door/door.gd:6` vs `door/door.tscn:13` (`DoorModel2`), spec `specs/002-v1-player-bullet-door` US3/FR-030–FR-035; fix `src/door.rs`: `#[init(node = "DoorModel2/AnimationPlayer")]` with comment `// upstream bug fix` (nothing renamed in the scene; `open` logic intact); commit = subject `Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"` (no hash — no `--amend`)
- [x] T037 [US3] Record in `docs/v2-backlog.md` line no. 14 **reduced**: `door/door.gd` (port 3) — type `_on_door_body_entered` with `Gd<Player>` already at the boundary (`try_cast` at the signal) instead of `Gd<Node3D>` + `try_cast` in the body. Do **not** include "consider closing the door" (feature idea, not a noticed improvement)
- [x] T038 [US3] Single commit of port 3 on `main`, including `src/door.rs`, `src/lib.rs`, `oxide-godot/door/door.tscn`, the deletions of `door.gd`/`door.gd.uid`, `docs/upstream-bugs.md` (new) and `docs/v2-backlog.md`. **Mandatory** message (requirement (c), quickstart §8): subject `Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"`; body with the lines `- upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (non-existent node); the port references "DoorModel2/AnimationPlayer" — the door now opens and the "ERROR: Node not found" disappears (baseline 1 → 0). Minimal fix; scene node not renamed; open logic intact.`, `- docs/upstream-bugs.md created with entry #1.`, `- CLAUDE.md: catalog unchanged (the door error was never in it — only the 3 import errors).`, `- v2 backlog: item 14`. Note the hash
- [x] T039 [US3] **User checkpoint (plan.md port 3)** — nothing changes in the game (the door is an orphan asset). The user opens `door/door.tscn` in the editor: root of type `Door`, no script, `DoorModel2` untouched, the `body_entered → _on_door_body_entered` connection preserved in the signals panel, Output with no `Node not found`. Optional (SC-009): **uncommitted** test scene (scratchpad or temporary `.tscn` removed before any commit) with `door.tscn` + `player.tscn` — when walking up to the door, `doorsimple_opening` plays once. This checkpoint may be given based on headless (T034) + editor. Divergence → commit `Fix port door.gd …`. Only proceed to Polish with the explicit OK Optional (spec US3 scenario 4): in the same isolated scene, a body that is not a `Player` (bullet/robot) entering the area does not trigger the animation.

**Checkpoint**: 7 `.gd` remaining; `door.tscn` with `Door`; `docs/upstream-bugs.md` exists; game playable.

---

## Phase 6: Polish — final verification of the milestone

**Purpose**: only the quickstart's final verification and the complete visual validation. No
extra documentation, no README, no refactoring.

- [x] T040 Mechanical verification of the milestone (quickstart §"Final verification"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 7; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 7; `git diff --stat 108584e -- 'oxide-godot/**/*.gd'` shows exactly 3 deletions (`player.gd`, `bullet.gd`, `door.gd`) and no modification (the 7 remaining byte-for-byte equal — SC-001); `git log --oneline 108584e..HEAD | grep -c '^[0-9a-f]* Port '` = 3 (`Fix port …` commits, if any, do not count); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs camera_noise_shake.rs player_input.rs player.rs bullet.rs door.rs`; `test -f docs/upstream-bugs.md && grep -c '^| 1 ' docs/upstream-bugs.md` = 1; `grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/` = 1 line (`door.rs`); `git diff --stat 108584e -- CLAUDE.md` empty; `grep -c '^| [0-9]' docs/v2-backlog.md` = 14; FR-025 (Rust does not call custom GDScript API): `grep -nE '\.call\(|\.call_deferred\(' oxide_godot_core/oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/bullet.rs oxide_godot_core/oxide_godot_lib/src/door.rs` → empty; `grep -nE '\.get\("' ` in those 3 files → only `get("config_file")` in bullet.rs; `grep -nE '\.set\("' ` → only `set("parameters/` in player.rs
- [x] T041 Final headless validation (no editor open): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; headless import with `Initialize godot-rust` and no `ERROR`; `level/level.tscn`, `player/player.tscn`, `player/bullet/bullet.tscn`, `door/door.tscn` headless with regression grep empty and `grep -c 'Node not found'` = 0 on the door
- [x] T042 **Complete user visual validation (SC-002, SC-009)**: menu → level; move, jump (sound), land (sound), aim, shoot with a visible bullet that explodes and hits robots, camera shake, respawn when falling below −40, F3, laser impact, parts disappearing — all indistinguishable from `../oxide_godot_origins/` in a side-by-side session; `player.tscn`, `bullet.tscn`, `door.tscn` in the editor with the Rust type and no script. Milestone B completed with the user's OK; then mark T001–T042 `[x]` and commit only `tasks.md` (`Tasks 002: milestone B completed`)

---

## Dependencies & Execution Order

### Mandatory order

```
Phase 1 (Setup: T001–T005)          — includes the measurement of the door's "Node not found" (= 1)
  → Phase 3 US1 (T006–T018)  → commit 1  (Player; + pub(crate) in player_input.rs/camera_noise_shake.rs)
  → Phase 4 US2 (T019–T028)  → commit 2  (Bullet; BulletCache becomes Rust)
  → Phase 5 US3 (T029–T039)  → commit 3  (Door + docs/upstream-bugs.md; "Node not found" = 0)
  → Phase 6 Polish (T040–T042)
```

- **Phase 2 (Foundational)**: does not exist — nothing blocks the stories besides the Setup.
- **Stories are not parallelizable with each other**: each one ends with a commit on `main` and the
  next one starts from the clean tree (FR-028, SC-004). US3 requires `Player` in Rust (`is Player`); US2 is
  instantiated by the Player via base API, and the Player types the bullet as `CharacterBody3D` precisely
  so that commit 1 does not depend on commit 2.
- **Internal order of each story** (strict dependencies): [US1: `pub(crate)` visibility] →
  `.rs` module → `mod` in `lib.rs` → `cargo build` → edit `.tscn` → delete `.gd`/`.uid` →
  headless → mechanical checks + contract → [US3: `docs/upstream-bugs.md`] → backlog →
  commit → user checkpoint. The `.tscn` is only edited after the build because the class needs to
  exist in the loaded lib for the `type` to resolve on the headless import.
- **User checkpoint** (T018, T028, T039, T042) is blocking: the next story only starts
  with the explicit OK.

### Parallel Opportunities

Practically none, by construction:

- Setup: T004 is [P] relative to T002/T003/T005 (it only reads the bindings directory).
- Within each story, no task is [P]: each step consumes the result of the previous one.
  T006→T007→T008→T009 write files in sequence (T007–T009 the same `player.rs`).
- Backlog (T016, T026, T037) and `docs/upstream-bugs.md` (T036) could be written at any
  time before the story's commit, but they edit shared files — keep sequential.

### Parallel Example

```bash
# The only truly independent pair (Phase 1):
Task: "T002 cargo build → count warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline recorded, including the door error.
2. Phase 3: US1 (T006–T018) — `player.gd` ported, commit 1, user's OK.
3. **STOP AND VALIDATE**: it is the largest script of the project and the first to consume Rust classes in a
   typed way and to expose a contract to three GDScript consumers; any problem with setter outside
   the tree, replication by name or RPC re-entrancy shows up here.

### Incremental Delivery

Each story is a complete port and the game remains playable after each commit:

1. US1 → 9 `.gd` remaining → playable (bullet still GDScript, instantiated via base API)
2. US2 → 8 → playable (`BulletCache` and shots in Rust)
3. US3 → 7 → playable (nothing visible changes; door starts working in an isolated test)
4. Polish → milestone B completed

### If something fails in the middle of a story

Do not commit partially. Either the whole port (module + scene + deletion [+ visibility / +
`upstream-bugs.md`]) goes into the commit, or nothing:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<module>.rs docs/upstream-bugs.md`
returns to the clean tree of the previous story, which is always playable. If the implementation requires something not
foreseen in the tasks (another file, another fix, another visibility), stop and report.

---

## Notes

- No task creates a helper, trait, common module, test or new log. If it seems necessary, it is an
  entry in `docs/v2-backlog.md` — not a task.
- **A single bug fix** (US3). Any other noticed defect (including the quirks of
  research D16) stays as it is and goes to the backlog; "when in doubt, it is an improvement".
- Every `.tscn` is edited as text; re-verify lines with `grep -n` before each edit (the
  cited lines are those of 2026-09-15; removing the `ext_resource` on l.3 shifts the others by −1).
- Names of `#[func]`, RPCs and `#[export]`/`#[var]` properties are contract (FR-024) — copy from
  GDScript, never "translate" (`add_camera_shake_trauma`, `_on_door_body_entered`, `destroy`,
  `current_animation`, `motion`).
- The headless regression grep is `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked`;
  the two `WARNING`s from the baseline (`HDR output`, `Physics interpolation`) do not count. On the door,
  `grep -c 'Node not found'` = 0 also applies.
- No Godot editor open during headless validations — warn the user beforehand; never kill their
  process.
- The commit (T017, T027, T038) comes **before** the visual checkpoint; a divergence found by the
  user is fixed in a `Fix port …` commit in the same story (never `Port …`, so that
  `git log | grep -c '^[0-9a-f]* Port '` remains = 3).
- Files that **never** change in this milestone: `Cargo.toml`, `.gdextension`, `project.godot`,
  `CLAUDE.md`, the 7 remaining `.gd`, `specs/` (except the `[x]` in `tasks.md` at the end) and, in the
  Milestone A modules, anything beyond the `pub(crate)` keyword listed in T006.
