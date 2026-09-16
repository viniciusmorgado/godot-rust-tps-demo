# Tasks: Milestone A — leaves and player input (v1 raw port)

**Input**: Design documents from `/specs/001-v1-leaves-and-input/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all approved)

**Phase**: v1 — Raw Port (Principle I). No task may introduce abstraction, refactoring,
optimization, unit tests or infrastructure. If something like that seems necessary, it becomes an entry in
`docs/v2-backlog.md`, not a task.

**Tests**: there are no automated tests in this phase (plan.md "Testing"). The validation of each story
is the quickstart cycle (build → headless import → headless scene → mechanical checks →
user's visual validation).

**Organization**: one phase per user story, in the mandatory order US1 → US2 → US3 → US4 → US5
(spec FR-030). The stories are **not** parallelizable among themselves: each one ends with its own
commit on `main` and the next one starts from the clean tree; US4 and US5 edit the same file
(`player.tscn`) and port 4 shifts the lines of port 5.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task) — rare
  in this milestone, because the build depends on the module, the scene depends on the build, validation depends on the scene.
- **[Story]**: US1..US5 (spec.md)
- Paths relative to the repository root (`oxide-godot/` = Godot project;
  `oxide_godot_core/oxide_godot_lib/src/` = Rust crate).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` for each module (only that)
oxide_godot_core/oxide_godot_lib/src/<module>.rs   one class per script (research.md §"Map per script")
oxide-godot/<scene>.tscn                           `type` swap (plan.md "Scene edits")
oxide-godot/<script>.gd + .gd.uid                  deleted in the port's commit
docs/v2-backlog.md                                 one line per improvement noticed
../oxide_godot_origins/                            untouched reference of the original GDScript
```

Closed decisions (apply to all stories — instruction, not option):

- Methods used only internally stay **private, without `#[func]`**, with the same name as the GDScript
  (`rotate_camera`; `decay_trauma`, `apply_shake`, `get_noise_value`).
- RPC `jump`: `#[rpc(authority, call_local, unreliable)]` — **not** `reliable` (research D5).
- Overlay FPS: `Variant::from(fps).stringify()` to reproduce `str(float)` = `"60.0"` (D9).
- Raycast: `.exclude(&array![Rid::Invalid])`; do **not** exclude the player's body (D11, FR-017).
- Scene references: `#[export] Option<Gd<T>>` + `.as_ref().unwrap()`/`.as_mut().unwrap()`;
  do **not** use `OnEditor` (D3).
- gdext spelling: `CpuParticles3D`/`ICpuParticles3D`; `AnimationPlayer::play_ex().name("…").done()`.
- Types: `delta: f64`; everything that feeds `Vector2`/`Vector3`/`Color`/`rotate_y`/`get_noise_1d`
  is `f32` with `as f32` at the point of use; `time: f64` in the shake (D13).
- One commit per script on `main`, message in the format of quickstart step 8.

---

## Phase 1: Setup

**Purpose**: record the baseline against which each port is compared. No "create project
structure" or "configure lint": the crate already exists and no infrastructure may be added
(Principle I).

- [x] T001 Confirm preconditions: no Godot editor open on the `oxide-godot/` project; clean `git status` on `main`; note `git rev-parse --short HEAD` as the start of the milestone. The base commit `6b22de3` used by `quickstart.md`/T057 for `git diff -- '*.gd'` and `git log 6b22de3..HEAD` remains valid: the later commits (plan, tasks) touch no `.gd` and are not "Port …" commits
- [x] T002 Record the build baseline: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → expected `0`; note the obtained value in `specs/001-v1-leaves-and-input/quickstart.md` §"Baseline" if it differs
- [x] T003 Record the import baseline: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirm `grep -n 'Initialize godot-rust' /tmp/import.log` (line 1) and `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` empty (or only the 3 upstream errors from `CLAUDE.md`)
- [x] T004 [P] Confirm the generated bindings directory used as reference by research.md: `ls -d oxide_godot_core/target/debug/build/godot-core-*/out` → exactly one must exist; if the hash is not `aea5c50e7fda9d57`, update line 11 of `specs/001-v1-leaves-and-input/research.md` with the current hash (the cited lines remain valid — it is the same version 0.5.5)

**Checkpoint**: baseline known — 0 warnings, extension loads, 0 `ERROR` on import.

---

## Phase 2: Foundational

**Does not exist in this milestone.** The 5 scripts are independent leaves (`docs/port-order.md`); no
common module, helper, trait or shared constant may be created (Principle I — it would be
abstraction). Each story creates its own self-contained module and the only shared edit is the line
`mod <module>;` in `lib.rs`, done within the story itself.

---

## Phase 3: User Story 1 — Debug overlay (F3) ported (Priority: P1) 🎯 MVP

**Goal**: `level/debug.gd` (15 l.) → `DebugLabel: Label`; F3 toggles the overlay; text with FPS,
VSync, memory, online and (if online) ID, recomputed every frame — including while hidden (quirk).

**Independent Test**: `level/level.tscn` headless without `ERROR`; in the game, F3 toggles and the text shows
`FPS: 60.0` (with `.0`), `VSync: Enabled|Disabled`, `Memory: xx.xx MiB`, `Online: No`.

- [x] T005 [US1] Create `oxide_godot_core/oxide_godot_lib/src/debug_label.rs`: `#[derive(GodotClass)] #[class(init, base=Label)] struct DebugLabel { base: Base<Label> }`; `#[godot_api] impl ILabel for DebugLabel` with `fn process(&mut self, _delta: f64)` translating `oxide-godot/level/debug.gd` line by line: (1) `if Input::singleton().is_action_just_pressed("toggle_debug") { let v = self.base().is_visible(); self.base_mut().set_visible(!v); }`; (2) build a `String` with `"FPS: " + Variant::from(Engine::singleton().get_frames_per_second()).stringify()`; (3) `"\nVSync: " + ("Enabled" if DisplayServer::singleton().window_get_vsync_mode() != VSyncMode::DISABLED else "Disabled")`; (4) `"\nMemory: " + format!("{:3.2}", Os::singleton().get_static_memory_usage() as f64 / 1048576.0) + " MiB"`; (5) `online = !(get_multiplayer().unwrap().get_multiplayer_peer().map(|p| p.try_cast::<OfflineMultiplayerPeer>().is_ok()) == Some(true))`; `"\nOnline: Yes|No"`; (6) if online, `"\nMultiplayer ID: " + get_unique_id()`; (7) `self.base_mut().set_text(&text)`. No `#[func]`, no fields besides `base`. Signatures in research.md D9
- [x] T006 [US1] Add `mod debug_label;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (below `use godot::prelude::*;`, before the `struct OxideGodot`; nothing else changes in lib.rs)
- [x] T007 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`, `grep -c '^warning'` = 0. Fix any warning (unused imports etc.) before proceeding
- [x] T008 [US1] Edit `oxide-godot/level/level.tscn` (re-check lines with `grep -n 'name="Debug"\|ExtResource("9")\|debug.gd' oxide-godot/level/level.tscn`): on the line of the `Debug` node (≈148) swap `type="Label"` for `type="DebugLabel"`; remove the line `script = ExtResource("9")` (≈156); remove the line `[ext_resource type="Script" uid="uid://6ec6m14rhsxi" path="res://level/debug.gd" id="9"]` (≈7). Touch nothing else
- [x] T009 [US1] Delete `oxide-godot/level/debug.gd` and `oxide-godot/level/debug.gd.uid` (`git rm`); verify `grep -rn 'uid://6ec6m14rhsxi' oxide-godot/` empty and `grep -rn 'debug.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T010 [US1] Headless validation (quickstart steps 2–3): `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log` → contains `Initialize godot-rust`, no new `ERROR`; `timeout 20 /usr/bin/godot.x86_64 --headless --path . level/level.tscn 2>&1 | tee /tmp/run.log` → exit 124 expected; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked' /tmp/run.log` empty (only the 2 baseline `WARNING`s)
- [x] T011 [US1] Mechanical checks (quickstart step 4): `grep -n 'type="DebugLabel"' oxide-godot/level/level.tscn` = 1 line; `grep -n 'ExtResource("9")' oxide-godot/level/level.tscn` empty; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `level/debug.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs`; check that the node's block in the `.tscn` defines no script property (only base class properties) — Principle II, name preservation
- [x] T012 [US1] Record in `docs/v2-backlog.md` line no. 4: origin `level/debug.gd` (port 1) — "Do not recompute the text while the overlay is hidden" — motivation "unnecessary per-frame work" (research.md §"Candidate v2 backlog"). Do not duplicate the already existing items 1–3
- [x] T013 [US1] Commit on `main` with: `oxide_godot_core/oxide_godot_lib/src/debug_label.rs`, `lib.rs`, `oxide-godot/level/level.tscn`, deletion of `debug.gd` + `.uid`, `docs/v2-backlog.md`. Message (quickstart step 8): `Port debug.gd → DebugLabel (Label); level.tscn: node Debug type="Label"→"DebugLabel"` + body with notes (`str(float)` via `Variant::stringify`; text recomputed while hidden, as in the original) and `v2 backlog: item 4`
- [x] T014 [US1] **User checkpoint (SC-004)**: (open `oxide-godot/project.godot` in the Godot 4.7.2 editor and run with F5) open the project in the editor, run (F5), menu → level; check F3 toggles the overlay; lines `FPS: 60.0`, `VSync: …`, `Memory: xx.xx MiB`, `Online: No` (no ID line); values change every frame; game playable end to end. Compare with `../oxide_godot_origins/`. Only move on to US2 after the user's OK

**Checkpoint**: 14 `.gd` remaining; `level.tscn` uses `DebugLabel`; game playable.

---

## Phase 4: User Story 2 — Part disappear effect ported (Priority: P2)

**Goal**: `part_disappear.gd` (9 l.) → `PartDisappear: CpuParticles3D`; `MiniBlasts` emits
immediately, the emitter itself after 0.2 s, `queue_free` after 2×`lifetime` — two chained `await`s
become two connections to `SceneTreeTimer.timeout`.

**Independent Test**: `part_disappear.tscn` headless without `ERROR` (exit 124 — `queue_free` of the root
does not end the SceneTree); in the game, when killing a robot, each part disappears with mini-blasts + puff.

- [x] T015 [US2] Create `oxide_godot_core/oxide_godot_lib/src/part_disappear.rs`: `#[class(init, base=CpuParticles3D)] struct PartDisappear { base: Base<CpuParticles3D>, #[init(node = "MiniBlasts")] mini_blasts: OnReady<Gd<CpuParticles3D>> }`; `#[godot_api] impl ICpuParticles3D for PartDisappear` with `fn ready(&mut self)` translating `oxide-godot/enemies/red_robot/parts/part_disappear_effect/part_disappear.gd`: (1) `self.mini_blasts.set_emitting(true);` (2) `let timer = self.base().get_tree().create_timer(0.2); timer.signals().timeout().connect_other(&*self, |this: &mut PartDisappear| { this.base_mut().set_emitting(true); let t2 = this.base().get_tree().create_timer(this.base().get_lifetime() * 2.0); t2.signals().timeout().connect_other(&*this, |this2: &mut PartDisappear| this2.base_mut().queue_free()); });`. No `#[func]`. Signatures and justification (linked callable, invalidated if the node is freed) in research.md D6
- [x] T016 [US2] Add `mod part_disappear;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [x] T017 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings. If the borrow checker refuses `&*self`/`&*this` inside the nested closure, use `let gd = this.to_gd();` and `connect_other(&gd, ...)` — same semantics (research D6 accepts `&Gd<T>`)
- [x] T018 [US2] Edit `oxide-godot/enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn` (re-check with `grep -n 'PartDisappearPuff\|ExtResource("3")\|part_disappear.gd'`): root node `PartDisappearPuff` (≈43) `type="CPUParticles3D"` → `type="PartDisappear"`; remove `script = ExtResource("3")` (≈59); remove the `[ext_resource type="Script" uid="uid://dxd6xoeg627y6" ... id="3"]` (≈5)
- [x] T019 [US2] Delete `part_disappear.gd` and `part_disappear.gd.uid` in `oxide-godot/enemies/red_robot/parts/part_disappear_effect/` (`git rm`); `grep -rn 'uid://dxd6xoeg627y6' oxide-godot/` empty; `grep -rn 'part_disappear.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty (the `preload` in `part.gd:4` references the `.tscn`, not the `.gd` — it must remain)
- [x] T020 [US2] Headless validation: import (`Initialize godot-rust`, no new `ERROR`) and `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn 2>&1 | tee /tmp/run.log` → `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked' /tmp/run.log` empty; also run `level/level.tscn` (same grep empty)
- [x] T021 [US2] Mechanical checks: `grep -n 'type="PartDisappear"' .../part_disappear.tscn` = 1; `grep -n 'ExtResource("3")' .../part_disappear.tscn` empty; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` only the deletion of `part_disappear.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs`; check that the node's block in the `.tscn` defines no script property (only base class properties) — Principle II, name preservation
- [x] T022 [US2] Record in `docs/v2-backlog.md` line no. 5: origin `part_disappear.gd` / `blast.gd` (ports 2–3) — "Timers/signals as `async` (`godot::task`) when the API stabilizes" — motivation "closure chaining reproduces `await` in a less readable way". (US3 does not add another line: this item covers both scripts)
- [x] T023 [US2] Commit on `main`: `part_disappear.rs`, `lib.rs`, `part_disappear.tscn`, deletion `.gd` + `.uid`, `docs/v2-backlog.md`. Message: `Port part_disappear.gd → PartDisappear (CpuParticles3D); part_disappear.tscn: node PartDisappearPuff type="CPUParticles3D"→"PartDisappear"` + notes (`await create_timer` → `connect_other` with linked callable) + `v2 backlog: item 5`
- [x] T024 [US2] **User checkpoint (SC-004)**: (open `oxide-godot/project.godot` in the Godot 4.7.2 editor and run with F5) in the game, kill a robot; each part when disappearing fires mini-blasts immediately and the smoke puff ~0.2 s later; the effect goes away on its own (~3.2 s); no error in the editor console. Game playable. Compare with `../oxide_godot_origins/`. Only move on after OK

**Checkpoint**: 13 `.gd` remaining; game playable.

---

## Phase 5: User Story 3 — Laser impact ported (Priority: P3)

**Goal**: `blast.gd` (15 l.) → `Blast: Node3D`; captures the active camera in `ready`, orients
`LightRays` toward it every frame while valid, `queue_free` on `animation_finished`.

**Independent Test**: `impact_effect.tscn` headless without `ERROR`; in the game, the robot's laser
impact animates with the rays facing the camera and goes away at the end of the animation.

- [x] T025 [US3] Create `oxide_godot_core/oxide_godot_lib/src/blast.rs`: `#[class(init, base=Node3D)] struct Blast { base: Base<Node3D>, #[init(node = "LightRays")] light_rays: OnReady<Gd<CpuParticles3D>>, #[init(node = "AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>, camera: Option<Gd<Camera3D>> }`; `#[godot_api] impl INode3D for Blast`: `fn ready(&mut self)` = (1) `self.camera = self.base().get_tree().get_root().unwrap().get_camera_3d();` (translation of the `@onready var camera`, before the rest); (2) `self.animation_player.signals().animation_finished().connect_other(&*self, |this: &mut Blast, _anim_name: StringName| this.base_mut().queue_free());`; `fn process(&mut self, _delta: f64)` = `if let Some(cam) = &self.camera { if cam.is_instance_valid() { let origin = cam.get_global_transform().origin; self.light_rays.look_at(origin); } }`. No `#[func]`. Signatures in research.md D7–D8
- [x] T026 [US3] Add `mod blast;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [x] T027 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings
- [x] T028 [US3] Edit `oxide-godot/enemies/red_robot/laser/impact_effect/impact_effect.tscn` (re-check with `grep -n 'name="Blast" type="Node3D"\|ExtResource("5")\|blast.gd'`): root node `Blast` (≈170) `type="Node3D"` → `type="Blast"`; remove `script = ExtResource("5")` (≈171); remove the `[ext_resource type="Script" uid="uid://bk20efkdq4v3m" ... id="5"]` (≈7). Attention: there is a child node also named `Blast` (`type="CPUParticles3D"`, ≈177) — do not touch it
- [x] T029 [US3] Delete `blast.gd` and `blast.gd.uid` in `oxide-godot/enemies/red_robot/laser/impact_effect/` (`git rm`); `grep -rn 'uid://bk20efkdq4v3m' oxide-godot/` empty; `grep -rn 'blast.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty (the `preload` in `red_robot.gd:35` references the `.tscn` — it remains)
- [x] T030 [US3] Headless validation: import (`Initialize godot-rust`, no new `ERROR`) and `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/laser/impact_effect/impact_effect.tscn 2>&1 | tee /tmp/run.log` → regression grep empty; also run `level/level.tscn` (grep empty)
- [x] T031 [US3] Mechanical checks: `grep -n 'type="Blast"' .../impact_effect.tscn` = 1 (the root); `grep -n 'ExtResource("5")' .../impact_effect.tscn` empty; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` only the deletion of `blast.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs`; check that the node's block in the `.tscn` defines no script property (only base class properties) — Principle II, name preservation
- [x] T032 [US3] Backlog: no new line in `docs/v2-backlog.md` (item 5, added in US2, already covers `blast.gd`); confirm that line 5 mentions `blast.gd` — if it does not, edit line 5 to include it
- [x] T033 [US3] Commit on `main`: `blast.rs`, `lib.rs`, `impact_effect.tscn`, deletion `.gd` + `.uid` (+ `docs/v2-backlog.md` if T032 edited it). Message: `Port blast.gd → Blast (Node3D); impact_effect.tscn: node Blast (raiz) type="Node3D"→"Blast"` + notes (`await animation_finished` → `connect_other`; camera captured in `ready` and checked with `is_instance_valid`) + `v2 backlog: none (covered by item 5)`
- [x] T034 [US3] **User checkpoint (SC-004)**: (open `oxide-godot/project.godot` in the Godot 4.7.2 editor and run with F5) in the game, let the robot shoot at the player or at a wall; the impact animates, the light rays stay facing the camera when moving around, the effect goes away at the end of the animation; no errors in the console. Game playable. Compare with the original. Only move on after OK

**Checkpoint**: 12 `.gd` remaining; game playable.

---

## Phase 6: User Story 4 — Camera shake ported (Priority: P4)

**Goal**: `camera_noise_shake_effect.gd` (61 l.) → `CameraNoiseShake: Camera3D`; exposes
`add_trauma(amount)` (called by `player.gd:211`, still in GDScript); trauma ≤ 1.2, decays
1.5/s, rotation = `start_rotation` + noise × trauma².

**Independent Test**: `player.tscn` and `level.tscn` headless without `ERROR`; contract
`contracts/camera-noise-shake.md` checked; in the game, shooting/being hit/being hit by the robot
shakes the camera with the same intensity and it returns exactly to rest.

- [x] T035 [US4] Create `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs`: constants `const SPEED: f32 = 1.0; const DECAY_RATE: f32 = 1.5; const MAX_YAW: f32 = 0.05; const MAX_PITCH: f32 = 0.05; const MAX_ROLL: f32 = 0.1; const MAX_TRAUMA: f32 = 1.2;`; `#[class(init, base=Camera3D)] struct CameraNoiseShake { base: Base<Camera3D>, start_rotation: Vector3, trauma: f32, time: f64, #[init(val = FastNoiseLite::new_gd())] noise: Gd<FastNoiseLite>, #[init(val = (randi() as i32))] noise_seed: i32 }`; `#[godot_api] impl ICamera3D`: `fn ready` = `self.noise.set_seed(self.noise_seed); self.noise.set_fractal_octaves(1); self.noise.set_fractal_lacunarity(1.0); self.start_rotation = self.base().get_rotation();`; `fn process(&mut self, delta: f64)` = `if self.trauma > 0.0 { self.decay_trauma(delta); self.apply_shake(delta); }`. `#[godot_api] impl CameraNoiseShake`: `#[func] fn add_trauma(&mut self, amount: f64) { self.trauma = (self.trauma + amount as f32).min(MAX_TRAUMA); }`. **Private** methods, **without `#[func]`**, in a plain `impl CameraNoiseShake`: `fn decay_trauma(&mut self, delta: f64) { let change = DECAY_RATE * delta as f32; self.trauma = (self.trauma - change).max(0.0); }`; `fn apply_shake(&mut self, delta: f64) { self.time += delta * SPEED as f64 * 5000.0; let shake = self.trauma * self.trauma; let yaw = MAX_YAW * shake * self.get_noise_value(self.noise_seed, self.time); let pitch = MAX_PITCH * shake * self.get_noise_value(self.noise_seed.wrapping_add(1), self.time); let roll = MAX_ROLL * shake * self.get_noise_value(self.noise_seed.wrapping_add(2), self.time); let r = self.start_rotation + Vector3::new(pitch, yaw, roll); self.base_mut().set_rotation(r); }`; `fn get_noise_value(&mut self, seed_value: i32, pos: f64) -> f32 { self.noise.set_seed(seed_value); self.noise.get_noise_1d(pos as f32) }`. Signatures in research.md D10, D13
- [x] T036 [US4] Add `mod camera_noise_shake;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [x] T037 [US4] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings (mind `randi` — import `godot::global::randi`)
- [x] T038 [US4] Edit `oxide-godot/player/player.tscn` (re-check with `grep -n 'name="Camera3D"\|ExtResource("8")\|camera_noise_shake_effect.gd' oxide-godot/player/player.tscn`): node `Camera3D` (≈630, `parent="CameraBase/CameraRot/SpringArm3D"`) `type="Camera3D"` → `type="CameraNoiseShake"`; remove `script = ExtResource("8")` (≈633); remove the `[ext_resource type="Script" uid="uid://byrvr71jmaisi" path="res://player/camera_noise_shake_effect.gd" id="8"]` (≈11). Do not touch the `InputSynchronizer` nor `ExtResource("2_g11dy")` (that is US5)
- [x] T039 [US4] Delete `oxide-godot/player/camera_noise_shake_effect.gd` and `.gd.uid` (`git rm`); `grep -rn 'uid://byrvr71jmaisi' oxide-godot/` empty; `grep -rn 'camera_noise_shake_effect.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T040 [US4] Headless validation: import (`Initialize godot-rust`, no new `ERROR`); `timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` → regression grep empty; same for `level/level.tscn`. In particular, no line `Invalid call. Nonexistent function 'add_trauma'` (it would only appear under interaction, but headless loads `player.gd` and resolves the type)
- [x] T041 [US4] Mechanical checks and contract: `grep -n 'type="CameraNoiseShake"' oxide-godot/player/player.tscn` = 1; `grep -n 'ExtResource("8")' oxide-godot/player/player.tscn` empty; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` only the deletion of `camera_noise_shake_effect.gd`; `grep -rn 'add_trauma\|add_camera_shake_trauma' --include=*.gd oxide-godot/` returns exactly `player.gd:201,206,210,211` and `red_robot.gd:133` (contracts/camera-noise-shake.md); `add_trauma` exists as `#[func]` in `camera_noise_shake.rs`
- [x] T042 [US4] Record in `docs/v2-backlog.md` line no. 6: origin `player/camera_noise_shake_effect.gd` (port 4) — "Recapture `start_rotation` when animations/scripts move the camera" — motivation "the original's comment acknowledges the problem; the shake adds to the rotation captured a single time"
- [x] T043 [US4] Commit on `main`: `camera_noise_shake.rs`, `lib.rs`, `player.tscn`, deletion `.gd` + `.uid`, `docs/v2-backlog.md`. Message: `Port camera_noise_shake_effect.gd → CameraNoiseShake (Camera3D); player.tscn: node Camera3D type="Camera3D"→"CameraNoiseShake"` + notes (`decay_trauma`/`apply_shake`/`get_noise_value` private without `#[func]`; `time: f64`; `wrapping_add` on the seeds) + `v2 backlog: item 6`
- [x] T044 [US4] **User checkpoint (SC-004)**: (open `oxide-godot/project.godot` in the Godot 4.7.2 editor and run with F5) in the game, shoot (light shake, 0.35) and get hit by the robot's laser (strong, 13.0 saturated at 1.2); the medium level (0.75, the player's RPC `hit`) only occurs in multiplayer when ANOTHER player's bullet hits — not verifiable single-player; the camera returns exactly to the rest position; intensity and duration equal to the original (noise pattern may differ — random seed). No errors in the console. Only move on after OK

**Checkpoint**: 11 `.gd` remaining; `player.tscn` with `CameraNoiseShake`; game playable.

---

## Phase 7: User Story 5 — Player input synchronizer ported (Priority: P5)

**Goal**: `player_input.gd` (142 l.) → `PlayerInputSynchronizer: MultiplayerSynchronizer` (name
**mandatory**: `player.gd:26` types `$InputSynchronizer` as `PlayerInputSynchronizer`); 5
exported properties, 6 references via `node_paths`, 3 `#[func]`, RPC `jump`; `player.gd`
keeps consuming everything without change.

**Independent Test**: `player.tscn` and `level.tscn` headless without `ERROR`; contract
`contracts/player-input-synchronizer.md` checked name by name; in the game, move, look (mouse and
analog stick, slower while aiming), pitch locked, aim toggle/hold with shoot/far animations, jump,
shoot at the point under the crosshair, fade when falling off the map.

- [x] T045 [US5] Create `oxide_godot_core/oxide_godot_lib/src/player_input.rs` — **struct and fields**: constants `const CAMERA_CONTROLLER_ROTATION_SPEED: f32 = 3.0; const CAMERA_MOUSE_ROTATION_SPEED: f32 = 0.001; const CAMERA_X_ROT_MIN: f32 = (-89.9_f32).to_radians(); const CAMERA_X_ROT_MAX: f32 = 70.0_f32.to_radians(); const AIM_HOLD_THRESHOLD: f32 = 0.4;`; `#[class(init, base=MultiplayerSynchronizer)] struct PlayerInputSynchronizer { base: Base<MultiplayerSynchronizer>, toggled_aim: bool, aiming_timer: f32, #[export] aiming: bool, #[export] shoot_target: Vector3, #[export] motion: Vector2, #[export] shooting: bool, #[export] jumping: bool, #[export] camera_animation: Option<Gd<AnimationPlayer>>, #[export] crosshair: Option<Gd<TextureRect>>, #[export] camera_base: Option<Gd<Node3D>>, #[export] camera_rot: Option<Gd<Node3D>>, #[export] camera_camera: Option<Gd<Camera3D>>, #[export] color_rect: Option<Gd<ColorRect>> }` — names exactly these (data-model.md §1; replicated in `player.tscn:42–53`, `node_paths` in `:343,346–351`)
- [x] T046 [US5] In `player_input.rs` — **`impl IMultiplayerSynchronizer`**: `fn ready` = `if self.base().get_multiplayer_authority() == self.base().get_multiplayer().unwrap().get_unique_id() { self.camera_camera.as_mut().unwrap().make_current(); Input::singleton().set_mouse_mode(MouseMode::CAPTURED); } else { self.base_mut().set_process(false); self.base_mut().set_process_input(false); self.color_rect.as_mut().unwrap().hide(); }`. `fn process(&mut self, delta: f64)` translating `_process` of `oxide-godot/player/player_input.gd` line by line: (1) `motion` = `Vector2::new(strength("move_right") - strength("move_left"), strength("move_back") - strength("move_forward"))` with `Input::singleton().get_action_strength(..)`; (2) `camera_move` likewise with `view_right/left/up/down`; `camera_speed_this_frame = delta as f32 * CAMERA_CONTROLLER_ROTATION_SPEED`, `*= 0.5` if `self.aiming`; `self.rotate_camera(camera_move * camera_speed_this_frame)`; (3) aim state machine (data-model.md §1): `current_aim`, `toggled_aim`, `aiming_timer` exactly as in the GDScript (`is_action_just_released("aim") && aiming_timer <= AIM_HOLD_THRESHOLD` → `current_aim = true; toggled_aim = true`; otherwise `current_aim = toggled_aim || is_action_pressed("aim")` and `if is_action_just_pressed("aim") { toggled_aim = false }`); `aiming_timer += delta as f32` if `current_aim` otherwise `= 0.0`; if `aiming != current_aim`: `aiming = current_aim` and `camera_animation.as_mut().unwrap().play_ex().name(if aiming {"shoot"} else {"far"}).done()`; (4) `if is_action_just_pressed("jump") { self.base_mut().rpc("jump", &[]); }`; (5) `shooting = is_action_pressed("shoot")`; if `shooting`: `ch_pos = crosshair.get_position() + crosshair.get_size() * 0.5`; `ray_from = camera_camera.project_ray_origin(ch_pos)`; `ray_dir = camera_camera.project_ray_normal(ch_pos)`; `params = PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0).collision_mask(0b11).exclude(&array![Rid::Invalid]).done().unwrap()`; `col = self.base().get_parent().unwrap().cast::<Node3D>().get_world_3d().unwrap().get_direct_space_state().unwrap().intersect_ray(&params)`; `shoot_target = if col.is_empty() { ray_from + ray_dir * 1000.0 } else { col.get("position").unwrap().to::<Vector3>() }` — do **not** exclude the player's body (FR-017, research D11); (6) fade: `y = get_parent().cast::<Node3D>().get_global_transform().origin.y`; `let mut m = color_rect.get_modulate(); if y < -17.0 { m.a = ((-17.0 - y) / 15.0).min(1.0) } else { m.a *= 1.0 - delta as f32 * 4.0 }; color_rect.set_modulate(m)`. `fn input(&mut self, event: Gd<InputEvent>)` = `if let Ok(mm) = event.try_cast::<InputEventMouseMotion>() { let mut speed = CAMERA_MOUSE_ROTATION_SPEED; if self.aiming { speed *= 0.75; } self.rotate_camera(mm.get_screen_relative() * speed); }`. Signatures in research.md D9, D11, D12
- [x] T047 [US5] In `player_input.rs` — **public API and RPC** in `#[godot_api] impl PlayerInputSynchronizer`: `#[func] fn get_aim_rotation(&self) -> f64 { let x = self.camera_rot.as_ref().unwrap().get_rotation().x.clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); if x >= 0.0 { (-x / CAMERA_X_ROT_MAX) as f64 } else { (x / CAMERA_X_ROT_MIN) as f64 } }`; `#[func] fn get_camera_base_quaternion(&self) -> Quaternion { self.camera_base.as_ref().unwrap().get_global_transform().basis.get_quaternion() }`; `#[func] fn get_camera_rotation_basis(&self) -> Basis { self.camera_rot.as_ref().unwrap().get_global_transform().basis }`; `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.jumping = true; }` (**not** `reliable` — research D5). **Private** method, **without `#[func]`**, in a plain `impl PlayerInputSynchronizer`: `fn rotate_camera(&mut self, mv: Vector2) { let cb = self.camera_base.as_mut().unwrap(); cb.rotate_y(-mv.x); cb.orthonormalize(); let cr = self.camera_rot.as_mut().unwrap(); let mut r = cr.get_rotation(); r.x = (r.x + mv.y).clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); cr.set_rotation(r); }`
- [x] T048 [US5] Add `mod player_input;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [x] T049 [US5] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings. Expected imports: `godot::prelude::*`, `godot::classes::{Input, InputEvent, InputEventMouseMotion, MultiplayerSynchronizer, IMultiplayerSynchronizer, AnimationPlayer, TextureRect, ColorRect, Camera3D, Node3D, PhysicsRayQueryParameters3D}`, `godot::classes::input::MouseMode`. If `#[rpc]` complains about a missing `Base<T>`, check that the field is named `base` and is `Base<MultiplayerSynchronizer>`
- [x] T050 [US5] Edit `oxide-godot/player/player.tscn` (**re-check lines** — port 4 removed one `ext_resource` line, shifting everything by −1: `grep -n 'name="InputSynchronizer"\|ExtResource("2_g11dy")\|player_input.gd' oxide-godot/player/player.tscn`): on the line of the `InputSynchronizer` node (≈342) swap `type="MultiplayerSynchronizer"` for `type="PlayerInputSynchronizer"` **keeping** on the same line `parent="."`, `unique_id=…` and `node_paths=PackedStringArray("camera_animation", "crosshair", "camera_base", "camera_rot", "camera_camera", "color_rect")`; remove only the line `script = ExtResource("2_g11dy")` (≈344); **keep** `replication_config = SubResource("SceneReplicationConfig_8yuxf")` and the 6 lines `camera_animation = NodePath(...)` … `color_rect = NodePath(...)`; remove the `[ext_resource type="Script" uid="uid://m1xn31x0lssx" path="res://player/player_input.gd" id="2_g11dy"]` (≈5). Do not touch the `SceneReplicationConfig_8yuxf` (lines ≈34–52 after port 4)
- [x] T051 [US5] Delete `oxide-godot/player/player_input.gd` and `.gd.uid` (`git rm`); `grep -rn 'uid://m1xn31x0lssx' oxide-godot/` empty; `grep -rn 'player_input.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty; `grep -rn 'PlayerInputSynchronizer' oxide-godot/ --include='*.gd'` returns only `player/player.gd:26` (the type annotation, now resolved to the native class)
- [x] T052 [US5] Headless validation: import (`Initialize godot-rust`, no new `ERROR` — in particular no parse error in `player.gd` about the type `PlayerInputSynchronizer`); `timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` → regression grep empty (`ready` runs the authority branch in headless: `make_current` + mouse captured, no error); same for `level/level.tscn` (spawns the Player; `player.gd` reads `player_input.motion` etc. every frame — any wrong name would show up here as `Invalid get index`)
- [x] T053 [US5] Mechanical checks and contract: `grep -n 'type="PlayerInputSynchronizer"' oxide-godot/player/player.tscn` = 1 and the same line contains `node_paths=`; `grep -c 'NodePath("../' oxide-godot/player/player.tscn` includes the 6 references; `grep -n 'ExtResource("2_g11dy")' oxide-godot/player/player.tscn` empty; `grep -n 'InputSynchronizer:' oxide-godot/player/player.tscn` shows `shoot_target`, `motion`, `shooting`, `aiming`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` only the deletion of `player_input.gd`; run `grep -n 'player_input\.\|PlayerInputSynchronizer\|\$InputSynchronizer' oxide-godot/player/player.gd` and check each name (`get_aim_rotation`, `motion`, `get_camera_rotation_basis`, `jumping`, `aiming`, `get_camera_base_quaternion`, `shooting`, `shoot_target`, `camera_camera`) against `player_input.rs` (contracts/player-input-synchronizer.md)
- [x] T054 [US5] Record in `docs/v2-backlog.md` lines no. 7, 8 and 9, origin `player/player_input.gd` (port 5): (7) "Actually exclude the player's body in the raycast (`exclude` with the RID of the parent `CharacterBody3D`)" — "the original passes `[RID(0)]`; ineffective exclusion"; (8) "`OnEditor<Gd<T>>` instead of `Option<Gd<T>>` for the 6 mandatory references" — "eliminates `unwrap()` per frame and makes the editor flag a missing reference"; (9) "Replicate `jumping` or remove the `@export`" — "exported but outside replication; only makes sense via RPC"
- [x] T055 [US5] Commit on `main`: `player_input.rs`, `lib.rs`, `player.tscn`, deletion `.gd` + `.uid`, `docs/v2-backlog.md`. Message: `Port player_input.gd → PlayerInputSynchronizer (MultiplayerSynchronizer); player.tscn: node InputSynchronizer type="MultiplayerSynchronizer"→"PlayerInputSynchronizer"` + notes (`#[rpc(authority, call_local, unreliable)]` = `@rpc("call_local")`; raycast with `exclude([RID(0)])` preserved; `rotate_camera` private; `Option<Gd<T>>` for `node_paths`) + `v2 backlog: items 7–9`
- [x] T056 [US5] **User checkpoint (SC-004)**: (open `oxide-godot/project.godot` in the Godot 4.7.2 editor and run with F5) in the game — move (WASD/analog stick); look with mouse and analog stick (slower while aiming); pitch locks at −89.9°/70°; aim by short tap stays on and turns off on the next tap; aim by hold (> 0.4 s) releases on release; shoot/far camera animations; jump; shooting hits the point under the crosshair; falling through the hole in the map darkens the screen (black at y ≤ −32) and on returning fades out; F3 still works. No errors in the console. Compare with the original. Only move on after OK

**Checkpoint**: 10 `.gd` remaining; `player.tscn` with `CameraNoiseShake` + `PlayerInputSynchronizer`; game playable.

---

## Phase 8: Polish — final verification of the milestone

**Purpose**: only the final verification of the quickstart and the complete visual validation. No
extra documentation, no README, no refactoring.

- [x] T057 Mechanical verification of the milestone (quickstart §"Final verification"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 10; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 10; `git diff --stat 6b22de3 -- 'oxide-godot/**/*.gd'` shows exactly 5 deletions (`debug.gd`, `part_disappear.gd`, `blast.gd`, `camera_noise_shake_effect.gd`, `player_input.gd`) and no modification; `git log --oneline 6b22de3..HEAD | grep -c '^[0-9a-f]* Port '` = 5; (`Fix port …` commits, if any, do not count); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs camera_noise_shake.rs player_input.rs`; `grep -c '^| ' docs/v2-backlog.md` = 10 (header + 9 items)
- [x] T058 Final headless validation: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; headless import with `Initialize godot-rust` and no `ERROR`; `level/level.tscn`, `player/player.tscn`, `part_disappear.tscn`, `impact_effect.tscn` headless with regression grep empty
- [x] T059 **User's complete visual validation (SC-002)**: menu → level; move, look, aim (toggle and hold), jump, shoot, camera shake, F3, laser impact, robot parts disappearing — all indistinguishable from `../oxide_godot_origins/` in a side-by-side session. Open `level.tscn`, `player.tscn`, `part_disappear.tscn`, `impact_effect.tscn` in the editor: nodes with the Rust type, no script attached, `InputSynchronizer` with the 6 references and `replication_config` in the inspector. Milestone A concluded with the user's OK

---

## Dependencies & Execution Order

### Mandatory order

```
Phase 1 (Setup: T001–T004)
  → Phase 3 US1 (T005–T014)  → commit 1
  → Phase 4 US2 (T015–T024)  → commit 2
  → Phase 5 US3 (T025–T034)  → commit 3
  → Phase 6 US4 (T035–T044)  → commit 4   (edits player.tscn)
  → Phase 7 US5 (T045–T056)  → commit 5   (edits player.tscn — lines shifted by commit 4)
  → Phase 8 Polish (T057–T059)
```

- **Phase 2 (Foundational)**: does not exist — nothing blocks the stories besides Setup.
- **Stories are not parallelizable among themselves**: each one ends with a commit on `main` and the
  next starts from the clean tree (FR-030, SC-004: the game must be playable after each commit).
  US4 and US5 touch the same `player.tscn`.
- **Internal order of each story** (strict dependencies): `.rs` module → `mod` in `lib.rs` →
  `cargo build` → edit `.tscn` → delete `.gd`/`.uid` → headless → mechanical checks →
  backlog → commit → user checkpoint. The `.tscn` is only edited after the build because the
  class needs to exist in the loaded lib for the `type` to resolve on headless import.
- **User checkpoint** (T014, T024, T034, T044, T056, T059) is blocking: the next story
  only starts with the OK.

### Parallel Opportunities

Practically none, by construction:

- Setup: T004 is [P] relative to T002/T003 (it only reads the bindings directory).
- Within each story, no task is [P]: each step consumes the result of the previous one
  (build depends on the module; scene depends on the build; validation depends on the scene; commit depends on
  everything). T045→T046→T047 write the same file `player_input.rs` in sequence.
- Backlog (T012, T022, T042, T054) could be written at any time before the story's
  commit, but it edits `docs/v2-backlog.md`, shared between stories — keep it sequential.

### Parallel Example

```bash
# The only truly independent pair (Phase 1):
Task: "T002 cargo build → count warnings"
Task: "T004 ls -d oxide_godot_core/target/debug/build/godot-core-*/out"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T004) — baseline recorded.
2. Phase 3: US1 (T005–T014) — `debug.gd` ported, commit 1, user's OK.
3. **STOP AND VALIDATE**: the complete Principle II cycle (build → import → scene → commit) is
   proven on the simplest script; any toolchain/scene/hot-reload problem shows up here.

### Incremental Delivery

Each story is a complete port and the game stays playable after each commit:

1. US1 → 14 `.gd` remaining → playable
2. US2 → 13 → playable
3. US3 → 12 → playable
4. US4 → 11 → playable (`player.tscn` edited once)
5. US5 → 10 → playable (`player.tscn` edited again; `player.gd` contract preserved)
6. Polish → Milestone A concluded

### If something fails in the middle of a story

Do not commit partially. Either the whole port (module + scene + deletion) goes into the commit, or nothing:
`git checkout -- oxide-godot/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<module>.rs`
returns to the clean tree of the previous story, which is always playable.

---

## Notes

- No task creates a helper, trait, common module, test or new log. If it seems necessary, it is an
  entry in `docs/v2-backlog.md` — not a task.
- Every `.tscn` is edited as text; re-check lines with `grep -n` before each edit (the
  cited lines are those of 2026-09-15 and US4 shifts those of US5).
- Names of `#[func]` and of `#[export]` properties are contract (FR-024) — copy from the GDScript,
  never "translate" (`get_camera_base_quaternion`, not `camera_base_quaternion`).
- The headless regression grep is `ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked`; the two
  baseline `WARNING`s (`HDR output`, `Physics interpolation`) do not count.
- Commit only when the user gives the OK at the checkpoint? **No**: the commit (T013 etc.) comes before the
  visual checkpoint; if the user finds a divergence, it is fixed in a fix commit in the same
  story with the prefix `Fix port …` (never `Port …`, so that `git log | grep -c '^[0-9a-f]* Port '` remains = 5)
  story before starting the next one.
