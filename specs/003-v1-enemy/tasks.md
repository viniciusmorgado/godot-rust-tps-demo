# Tasks: Milestone C — enemy: part and red robot (v1 raw port)

**Input**: Design documents from `/specs/003-v1-enemy/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D15, §E), data-model.md, contracts/, quickstart.md (all approved, commit `6cedcb4`)

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.0). No task may introduce
abstraction, refactoring, optimization, unit tests or infrastructure. If something like that seems
necessary, it becomes an entry in `docs/v2-backlog.md`, not a task. **No bug fix** is
planned: if an objective upstream defect appears during the port, STOP and report — the clause
requires a declaration in the spec before any commit; the part received a declared fix (FR-028–FR-032, Phase 3b): `docs/upstream-bugs.md` ends with 2 entries.

**Tests**: there are no automated tests in this phase (plan.md "Testing"). The validation of each story
is the quickstart cycle (build → headless import → headless scene → mechanical checks →
contract → user's visual validation).

**Organization**: one phase per user story, in the mandatory order US1 → US2 (spec FR-026;
`docs/port-order.md` items 9 → 10). The stories are **not** parallelizable with each other: each one
ends with its own commit on `main` and the next one starts from the clean tree; US2 depends on
`Part` in Rust (typed access to `explode`) and both edit the same `red_robot.tscn` (US1
shifts US2's lines).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task) — rare
  in this milestone, because the build depends on the module, the scene depends on the build, validation depends on the scene.
- **[Story]**: US1, US2 (spec.md)
- Paths relative to the repository root (`oxide-godot/` = Godot project;
  `oxide_godot_core/oxide_godot_lib/src/` = Rust crate).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` of each module (only that)
oxide_godot_core/oxide_godot_lib/src/<module>.rs   one class per script (research.md §"Map per script")
oxide_godot_core/oxide_godot_lib/src/player.rs     Milestone B — only visibility changes (US2)
oxide-godot/enemies/red_robot/red_robot.tscn       11,053 lines; edited in BOTH stories (plan.md "Scene edits")
oxide-godot/enemies/red_robot/parts/part.gd + .uid       deleted in the US1 commit
oxide-godot/enemies/red_robot/red_robot.gd + .uid        deleted in the US2 commit
docs/v2-backlog.md                                 one line per perceived improvement (items 15–18)
docs/upstream-bugs.md                              entry #2 in Phase 3b (part fix)
../oxide_godot_origins/                            untouched reference of the original GDScript
```

Closed decisions (apply to both stories — instruction, not option; details in research.md):

- **Part**: `#[export] #[var(set = set_fade_value)] fade_value: f32` with `#[func] fn set_fade_value`
  in the main `#[godot_api] impl Part` block; in `process`, **call `self.set_fade_value(..)`**,
  never assign to the field (D2). `ready` duplicates the surface 0 material and the copy's `next_pass`
  with `Gd::duplicate_resource()` (the generated `duplicate()` is deprecated and generates a warning), only if
  `!Os::singleton().has_feature("dedicated_server")` (D3). `#[func] pub(crate) fn explode`
  (`#[func]` because `red_robot.gd` calls it by name until US2; `pub(crate)` for the typed access of
  US2) (D1, D4). Timers: `self.base().get_tree().create_timer(t).signals().timeout().connect_other(&*self, |this| ...)`
  — `create_timer` returns `Gd` directly, **without `unwrap`** (D4). `destroy` instantiates
  `part_disappear.tscn` as `instantiate_as::<CpuParticles3D>` — **never** `PartDisappear` (D5).
- **EnemyRobot**: enum `State { Idle, Approach, Aim, Shooting }` with
  `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)]` (D7);
  `#[signal] fn exploded();` in the **single** `#[godot_api] impl EnemyRobot` block, emission
  `self.signals().exploded().emit()` (D6); RPCs `#[rpc(authority, call_local, unreliable)]` `hit` and
  `play_shoot`; `shoot`, `animate`, `_clip_ray` **private without `#[func]`**; `player: Option<Gd<Node3D>>`.
- **Inverse transformation** (`Vector3 * Transform3D`): `gt.basis.transposed() * (v - gt.origin)` —
  **never** `affine_inverse()` (D9, confirmed numerically).
- **Raycasts**: 3 times **inline** (no helper — D12); `PhysicsRayQueryParameters3D::create_ex(from, to).collision_mask(0xFFFFFFFF).exclude(&array![rid]).done()`
  with `let rid = self.base().get_rid()` — **effective** exclusion, NOT `Rid::Invalid`; `intersect_ray`
  returns `VarDictionary`; `col.collider == player` → compare `instance_id()`.
- Y axis: `gt.basis.col_b()`; `atan2(a, b)` → `a.atan2(b)`; degrees via `.to_degrees()`;
  `format!("parameters/hit{}/request", randi() % 3 + 1)` with value `1.to_variant()` (D10–D11).
- **Blast**: `instantiate_as::<Node3D>` (never `Blast`), `add_child` on the tree root,
  `set_global_position`; shake 13.0 after a 0.1 s timer with a `move` closure capturing `Gd<Player>`
  and `player.clone().bind_mut().add_camera_shake_trauma(13.0)` (D13).
- `body.get_name() == StringName::from("Target")` (D13 — `"Target".into()` is ambiguous).
- **US2 opens visibility** in `player.rs`: `pub(crate) fn add_camera_shake_trauma` — only the keyword,
  its own task before the build, `git diff --stat` = `1 +-`, cited in the commit message (D1).
- One commit per script on `main`, message in the format of quickstart §8; fix after checkpoint =
  `Fix port …` commit.

---

## Phase 1: Setup

**Purpose**: record the baseline against which each port is compared. No infrastructure
(Principle I).

- [x] T001 Confirm preconditions: no Godot editor open (`pgrep -a godot` empty — if there is one, warn the user and wait, never kill); `git status --short` empty on `main`; note `git rev-parse --short HEAD` (expected `6cedcb4` or later without "Port …" commits). The base commit `4bb8f7f` used by `quickstart.md`/T029 for `git diff -- '*.gd'` and `git log 4bb8f7f..HEAD` remains valid (plan/tasks do not touch `.gd`); `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 7
- [x] T002 Record the build baseline: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → expected `0`; note in `specs/003-v1-enemy/quickstart.md` §"Baseline" if it differs
- [x] T003 Record the import baseline: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirm `grep -n 'Initialize godot-rust' /tmp/import.log` (line 1) and `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` empty (or only the 3 upstream errors from `CLAUDE.md`)
- [x] T004 [P] Confirm the generated bindings directory used by research.md: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` → expected `.../godot-core-aea5c50e7fda9d57/out`; if the hash differs, update line 6 of `specs/003-v1-enemy/research.md`
- [x] T005 Measure the baseline of the milestone's scenes (quickstart §"Baseline"): `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/base_robot.log` and likewise `level/level.tscn` → `/tmp/base_level.log`; expected exit 124 on both, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked'` empty, WARNINGs = 1 (`HDR`) on the robot and 2 (`HDR`, `Physics interpolation`) on the level. Also confirm `grep -c '^| 1 ' docs/upstream-bugs.md` = 1 and `grep -c '^| [0-9]' docs/v2-backlog.md` = 14

**Checkpoint**: baseline known — 0 warnings, extension loads, 0 `ERROR` on import and on both scenes.

---

## Phase 2: Foundational

**Does not exist in this milestone.** The two ports are strictly sequential (part → red_robot) and the
only shared edit is the `mod <module>;` line in `lib.rs`, made inside each story. The
`pub(crate)` visibility opening in `player.rs` **belongs to US2** (it is the robot that requires it and
it goes into the robot's commit); that of `Part::explode` is born already `pub(crate)` in US1. No common
module, helper, trait or shared constant may be created (Principle I) — the robot's three raycasts
stay inline (research D12).

---

## Phase 3: User Story 1 — Robot part ported (Priority: P1) 🎯 MVP

**Goal**: `enemies/red_robot/parts/part.gd` (57 l.) → `Part: RigidBody3D`; no scene of its own — the
port is the `type` swap on the 3 nodes `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` of
`red_robot.tscn`. `fade_value` with setter on the shader, replicated by name; `explode()` exposed by
name (the robot, still GDScript, calls `death_shield1.explode()`); fade → `destroy` RPC → puff.

**Independent Test**: `red_robot.tscn` and `level.tscn` headless without `ERROR` (the 3 parts run
`ready` and duplicate materials); contract `contracts/part.md`; in the game, killing a robot produces the same
flight/fall/fade/puff of the parts.

- [x] T006 [US1] Create `oxide_godot_core/oxide_godot_lib/src/part.rs` translating line by line `oxide-godot/enemies/red_robot/parts/part.gd`: `use godot::classes::{CollisionShape3D, CpuParticles3D, IRigidBody3D, Material, MeshInstance3D, MultiplayerSynchronizer, Node, Os, PackedScene, RigidBody3D, ShaderMaterial}; use godot::global::randf; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=RigidBody3D)] pub struct Part { base: Base<RigidBody3D>, _mat: Option<Gd<Material>>, #[export] #[init(val = 3.0)] lifetime: f32, #[export] #[init(val = 3.0)] lifetime_random: f32, #[export] #[init(val = 0.5)] disappearing_time: f32, #[export] #[var(set = set_fade_value)] fade_value: f32, _disappearing_counter: f32 }`; `#[godot_api] impl IRigidBody3D for Part`: `ready` (`self.base_mut().set_process(false); if !Os::singleton().has_feature("dedicated_server") { let mesh_inst = self.base().get_node_as::<Node>("Model").get_child(0).unwrap().cast::<MeshInstance3D>(); let mut mesh = mesh_inst.get_mesh().unwrap(); let mut mat: Gd<Material> = mesh.surface_get_material(0).unwrap().duplicate_resource(); mesh.surface_set_material(0, &mat); let next_pass: Gd<Material> = mat.get_next_pass().unwrap().duplicate_resource(); mat.set_next_pass(&next_pass); self._mat = Some(mat); }`) and `process(&mut self, delta: f64)` (`let fade = (self._disappearing_counter / self.disappearing_time).powi(2); self.set_fade_value(fade); self._disappearing_counter += delta as f32; if self._disappearing_counter >= self.disappearing_time - 0.2 { self.base_mut().rpc("destroy", &[]); self.base_mut().set_process(false); }`); `#[godot_api] impl Part` (single block): `#[func] fn set_fade_value(&mut self, value: f32) { self.fade_value = value; if let Some(mat) = &self._mat { mat.get_next_pass().unwrap().cast::<ShaderMaterial>().set_shader_parameter("emission_cutout", &value.to_variant()); } }`; `#[func] pub(crate) fn explode(&mut self)` (`// Start synching.` + `self.base().get_node_as::<MultiplayerSynchronizer>("MultiplayerSynchronizer").set_visibility_public(true); self.base_mut().set_freeze_enabled(false); if !self.base().get_multiplayer().unwrap().is_server() { return; } self.base().get_node_as::<CollisionShape3D>("Col1").set_disabled(false); ...("Col2")...; self.base_mut().set_linear_velocity(3.0 * Vector3::UP); let angular = (Vector3::new(randf() as f32, randf() as f32, randf() as f32).normalized() * 2.0 - Vector3::ONE) * 10.0; self.base_mut().set_angular_velocity(angular); let wait = self.lifetime + self.lifetime_random * randf() as f32; self.base().get_tree().create_timer(wait as f64).signals().timeout().connect_other(&*self, |this: &mut Part| this.base_mut().set_process(true));`); `#[rpc(authority, call_local, unreliable)] fn destroy(&mut self)` (`let mut puff: Gd<CpuParticles3D> = load::<PackedScene>("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn").instantiate_as::<CpuParticles3D>(); self.base().get_parent().unwrap().add_child(&puff); let origin = self.base().get_global_transform().origin; puff.set_global_position(origin); self.base().get_tree().create_timer(0.2).signals().timeout().connect_other(&*self, |this: &mut Part| this.base_mut().queue_free());`). No other `#[func]` (research D1–D5)
- [x] T007 [US1] Add `mod part;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod door;`; nothing else changes in lib.rs)
- [x] T008 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0 (caution: `duplicate()` generates a deprecation warning — use `duplicate_resource()`). In case of error, consult research.md D2–D5 and the bindings in `oxide_godot_core/target/debug/build/godot-core-*/out/classes/` before improvising; do not change `Cargo.toml`
- [x] T009 [US1] Edit `oxide-godot/enemies/red_robot/red_robot.tscn` as text (11,053 lines — re-check with `grep -n 'name="PartShield1"\|name="PartShield2"\|name="PartHead"\|^script = ExtResource("24")\|parts/part.gd' oxide-godot/enemies/red_robot/red_robot.tscn` → expected l.10833/10885/10936, l.10841/10892/10944, l.26): on the 3 node lines swap `type="RigidBody3D"` for `type="Part"`; remove the 3 `script = ExtResource("24")` lines; remove the line `[ext_resource type="Script" uid="uid://c3vo80hyj6w6c" path="res://enemies/red_robot/parts/part.gd" id="24"]` (l.26). Edit bottom-up (10944 → 10892 → 10841 → nodes → l.26) or by text pattern, so the numbers do not shift during the edit. **KEEP** on each part `transform`, `collision_layer = 3`, `collision_mask = 3`, `mass = 2000.0`, `physics_material_override`, `freeze = true`, `angular_damp = 0.3` and the children `MultiplayerSynchronizer` (`replication_config`, `public_visibility = false`), `Model`, `Col1`, `Col2`. Check beforehand that none of the 4 exports (`lifetime`, `lifetime_random`, `disappearing_time`, `fade_value`) is written in the blocks (expected: none). Expected diff: `4 deletions, 3 changed lines` (`git diff --stat` = `7 +++----`)
- [x] T010 [US1] Delete `oxide-godot/enemies/red_robot/parts/part.gd` and `oxide-godot/enemies/red_robot/parts/part.gd.uid` (`git rm`); verify `grep -rn 'uid://c3vo80hyj6w6c' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'parts/part.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T011 [US1] Headless validation (quickstart §2–3; no editor open): import with `Initialize godot-rust` and no new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/run.log` and then `level/level.tscn` → exit 124; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log` empty on both (only the WARNINGs of the T005 baseline). The 3 parts instantiate the Rust class: `ready` runs `get_node("Model").get_child(0)`, duplicates material and `next_pass` — any wrong path shows up here
- [x] T012 [US1] Mechanical checks and contract (quickstart §4–5; `contracts/part.md` §"Verification before the commit"): `grep -c 'type="Part"' oxide-godot/enemies/red_robot/red_robot.tscn` = 3; `grep -c 'ExtResource("24")' ...red_robot.tscn` = 0; `grep -c 'public_visibility = false' ...red_robot.tscn` = 3; `grep -n 'properties/0/path = NodePath(".:fade_value")' ...red_robot.tscn` = 1 line (≈l.10418) — the name exists in `part.rs` as `#[export] #[var(set)]`; `grep -n 'explode()' oxide-godot/enemies/red_robot/red_robot.gd` = l.96, 97, 98 (the GDScript robot keeps calling by name — `explode` is `#[func]`); `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `parts/part.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = 10 files (lib.rs + 9 modules); `grep -n '#\[func\]\|#\[rpc' -A1 oxide_godot_core/oxide_godot_lib/src/part.rs | grep 'fn '` = `set_fade_value`, `explode`, `destroy`
- [x] T013 [US1] Record in `docs/v2-backlog.md` line no. 15 (research.md §"Candidate v2 backlog"): `enemies/red_robot/parts/part.gd` (port 1) — instantiate the puff on the robot's parent (or on the root) instead of the part's parent (`Death`); motivation: the puff is born under the robot, removed 10 s after death — the timings do not cross today, but the dependency is fragile. Do not duplicate items 1–14
- [x] T014 [US1] Single commit of port 1 on `main`, including `src/part.rs`, `src/lib.rs`, `oxide-godot/enemies/red_robot/red_robot.tscn`, the deletions of `parts/part.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port part.gd → Part (RigidBody3D); red_robot.tscn: nodes Death/PartShield1, Death/PartShield2, Death/PartHead type="RigidBody3D"→"Part"` + body: notes (`fade_value` with setter on the shader, also triggered by replication; material and `next_pass` duplicated via `duplicate_resource()` outside a dedicated server; `explode` exposed by name for the GDScript robot and `pub(crate)` for port 2; `await` → timers with `connect_other`; puff instantiated as `CpuParticles3D` through base API; quirk preserved: puff on the part's parent); `- v2 backlog: item 15`. Note the hash
- [x] T015 [US1] **User checkpoint (visual validation, plan.md "Visual validation" port 1)** — done by the user in the game, comparing with `../oxide_godot_origins/`: kill a robot (5 shots) → the two shields and the head come loose upward with random rotation, fall and bounce with physics, stay 3–6 s on the ground, vanish in a fade (~0.3 s, `emission_cutout` glow) and end with the puff; each part vanishes at its own time, without affecting the others or neighboring robots; console without error. The robot is still GDScript calling `explode()` by name. In the editor, `red_robot.tscn`: the 3 nodes with type `Part`, no script, `freeze` checked, child `MultiplayerSynchronizer` with `public_visibility` unchecked. Divergence → `Fix port part.gd …` commit. Only proceed to US2 with the explicit OK

**Checkpoint**: 6 `.gd` remaining; `red_robot.tscn` with 3 `Part` nodes; game playable.

---

## Phase 3b: User Story 1 — conservative fix of the upstream bug in the part (FR-028–FR-032)

**Goal**: amendment discovered during the review of port 1 (parity harness): the duplicated material was installed on the `Mesh` shared by the two shields; fix with a per-instance override. Own commit `Fix port part.gd …`; `9aee2b8` is not rewritten.

- [x] T032 [US1] Edit `oxide_godot_core/oxide_godot_lib/src/part.rs`, only in `ready()`: replace the line `mesh.surface_set_material(0, &mat);` with `mesh_inst.set_surface_override_material(0, &mat);` (`MeshInstance3D::set_surface_override_material(surface: i32, material: impl AsArg<Option<Gd<Material>>>)`, bindings `mesh_instance_3d.rs`; `mesh_inst` needs to be `mut`), with the two comment lines **immediately above**: `// upstream bug fix: part.gd installed the duplicated material on the Mesh resource shared by both shields` / `// (surface_set_material), so the last shield to enter "won" and the other one's fade was never rendered; per-instance override.` The read `mesh.surface_get_material(0)` (source of the copy), the `next_pass` duplication, `self._mat = Some(mat)` and all the rest of the file stay the same. If `mesh` no longer needs `mut`, remove the `mut` (0 warnings)
- [x] T033 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings
- [x] T034 [US1] Headless validation: import (`Initialize godot-rust`, no new `ERROR`); `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn` and `level/level.tscn` → regression grep empty
- [x] T035 [US1] Verification of the result (FR-032), with a disposable `SceneTree` script outside the repo (do not commit): instantiate `red_robot.tscn`, get `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead`; for each, `mi = get_node("Model").get_child(0)`: `mi.get_surface_override_material(0)` non-null and distinct among the 3 parts; `mi.mesh.surface_get_material(0)` equal between the two shields (shared mesh untouched) and without a `next_pass` duplicated by them; `set_indexed("fade_value", 0.7)` on `PartShield1` changes `get_surface_override_material(0).next_pass.get_shader_parameter("emission_cutout")` of `PartShield1` to 0.7 and that of `PartShield2` remains 0.0. Report the outputs
- [x] T036 [US1] Mechanical checks: `git diff --stat` = only `part.rs` (≈ 3 +, 1 −) and `docs/upstream-bugs.md`; `grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/` = 2 files (door.rs, part.rs); `grep -n 'surface_set_material' part.rs` empty; no other file touched
- [x] T037 [US1] Record in `docs/upstream-bugs.md` entry `2`: defect "shield material shared — `part.gd:23-26` installed the copy with `mesh.surface_set_material` on the `Mesh` shared by `PartShield1`/`PartShield2`; the last shield won, the other's fade was not rendered (verified headless on the original)"; script/scene `enemies/red_robot/parts/part.gd:23-26` / `red_robot.tscn` (`Death/PartShield1`, `Death/PartShield2`, model ext_resource id="12"); spec `specs/003-v1-enemy` FR-028–FR-032; fix `src/part.rs ready()`: `set_surface_override_material(0, copy)` on the `MeshInstance3D` with comment `// upstream bug fix`; commit = subject of the T038 commit
- [x] T038 [US1] Single commit on `main`: `part.rs` + `docs/upstream-bugs.md`. Subject: `Fix port part.gd → Part: surface override material per instance (upstream bug fix)`; body with the line `- upstream bug fix: part.gd installed the material copy on the Mesh shared by the shields (surface_set_material); now per-instance override (set_surface_override_material). Minimal fix; scene and models untouched; declared in the spec (FR-028–FR-032).` and `- docs/upstream-bugs.md: entry #2.` Prefix `Fix port` (does not count as a `Port` commit)
- [x] T039 [US1] **User checkpoint**: in the game, kill a robot and watch the **two shields** fading each at its own time (before, one vanished without fade); head as before. Only advance to US2 after OK

**Checkpoint**: `docs/upstream-bugs.md` with 2 entries; `part.rs` with the milestone's only `upstream bug fix`.

---

## Phase 4: User Story 2 — Red robot ported (Priority: P2)

**Goal**: `enemies/red_robot/red_robot.gd` (283 l.) → `EnemyRobot: CharacterBody3D`; signal
`exploded` (consumed by `level.gd:99`), RPCs `hit`/`play_shoot`, method tracks
`shoot_check`/`resume_approach`, handlers `_on_area_body_entered/_exited`; state machine
IDLE→APPROACH→AIM→SHOOTING; typed access to `Part` (`explode`) and `Player` (`add_camera_shake_trauma`);
`Blast` through base API.

**Independent Test**: `red_robot.tscn` (robot IDLE + gravity) and `level.tscn` (spawn by
`level.gd`, `exploded` connected by name) headless without `ERROR`; contract `contracts/red-robot.md`;
in the game, the cycle detect → approach → aim → shoot → get hit → die → respawn
indistinguishable from the original.

- [x] T016 [US2] Change **only the visibility** in `oxide_godot_core/oxide_godot_lib/src/player.rs`: `fn add_camera_shake_trauma(&mut self, amount: f64)` (inside the `#[godot_api] impl Player`, the `#[rpc(...)]` attribute stays where it is) → `pub(crate) fn add_camera_shake_trauma(&mut self, amount: f64)`. Check: `git diff --stat` = `player.rs | 2 +-` (1 −/+ pair, nothing else) (research D1)
- [x] T017 [US2] Create `oxide_godot_core/oxide_godot_lib/src/red_robot.rs` — part 1 (declarations), translating `oxide-godot/enemies/red_robot/red_robot.gd:1-54`: `use godot::classes::{AnimationPlayer, AnimationTree, AudioStreamPlayer3D, BoneAttachment3D, CharacterBody3D, CollisionShape3D, CpuParticles3D, ICharacterBody3D, MeshInstance3D, Node3D, Os, PackedScene, PhysicsRayQueryParameters3D, RayCast3D, ShaderMaterial}; use godot::global::randi; use godot::prelude::*; use crate::part::Part; use crate::player::Player;`; `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum State { Idle, Approach, Aim, Shooting }`; `f32` consts: `PLAYER_AIM_TOLERANCE_DEGREES = 15.0_f32.to_radians()`, `SHOOT_WAIT = 6.0`, `AIM_TIME = 1.0`, `AIM_PREPARE_TIME = 0.5`, `BLEND_AIM_SPEED = 0.05`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct EnemyRobot { base: Base<CharacterBody3D>, #[export] test_shoot: bool, #[export] target_position: Vector3, #[export] #[init(val = 5)] health: i32, #[export] #[init(val = State::Idle)] state: State, #[export] dead: bool, #[export] #[init(val = AIM_PREPARE_TIME)] aim_preparing: f32, #[init(val = SHOOT_WAIT)] shoot_countdown: f32, #[init(val = AIM_TIME)] aim_countdown: f32, player: Option<Gd<Node3D>>, orientation: Transform3D,` + 15 `OnReady` (research D8, full paths): `animation_tree: AnimationTree` ("AnimationTree"), `shoot_animation: AnimationPlayer` ("ShootAnimation"), `model: Node3D` ("RedRobotModel"), `ray_from: BoneAttachment3D` ("RedRobotModel/Armature/Skeleton3D/RayFrom"), `ray_mesh: MeshInstance3D` (".../RayFrom/RayMesh"), `laser_raycast: RayCast3D` (".../RayFrom/RayCast"), `collision_shape: CollisionShape3D` ("CollisionShape3D"), `explosion_sound`/`hit_sound: AudioStreamPlayer3D` ("SoundEffects/Explosion", "SoundEffects/Hit"), `death: Node3D` ("Death"), `death_shield1`/`death_shield2`/`death_head: Part` ("Death/PartShield1", "Death/PartShield2", "Death/PartHead"), `death_detach_spark1`/`2: CpuParticles3D` ("Death/DetachSpark1", "Death/DetachSpark2") `}`. `blast_scene` (preload) does not become a field: `load` at the point of use in `shoot` (D13). Keep the original's comments where they exist
- [x] T018 [US2] `red_robot.rs` — part 2 (virtuals and private), translating `red_robot.gd:57-69,108-168,171-256,268-271`: `#[godot_api] impl ICharacterBody3D for EnemyRobot` with `ready` (`self.orientation = self.base().get_global_transform(); self.orientation.origin = Vector3::ZERO; self.animation_tree.set_active(true); if self.test_shoot { self.shoot_countdown = 0.0; } if self.dead { self.model.set_visible(false); self.collision_shape.set_disabled(true); self.animation_tree.set_active(false); } self.animate(0.0);`) and `physics_process(&mut self, delta: f64)`: `if self.dead { return; }`; `if !is_server { self.animate(delta); return; }`; `if self.test_shoot { self.shoot(); self.test_shoot = false; }`; `let Some(player) = self.player.clone() else { self.target_position = Vector3::ZERO; self.animate(delta); let g = self.base().get_gravity() * delta as f32; self.base_mut().set_velocity(g); self.base_mut().set_up_direction(Vector3::UP); self.base_mut().move_and_slide(); return; };`; `self.target_position = player.get_global_transform().origin;`; APPROACH: decrement of `aim_preparing` down to 0; `let gt = self.base().get_global_transform(); let to_player_local = gt.basis.transposed() * (self.target_position - gt.origin); let angle_to_player = to_player_local.x.atan2(to_player_local.z);` if `> -TOL && < TOL`: `shoot_countdown -= delta as f32; if < 0.0 {` **inline** raycast (D12): `let rid = self.base().get_rid(); let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, player.get_global_transform().origin + Vector3::UP).collision_mask(0xFFFFFFFF).exclude(&array![rid]).done(); let col: VarDictionary = self.base().get_world_3d().unwrap().get_direct_space_state().unwrap().intersect_ray(&params.unwrap());` and `!col.is_empty() && col.get("collider").and_then(|v| v.try_to::<Gd<Object>>().ok()).map(|c| c.instance_id() == player.instance_id()).unwrap_or(false)` → `state = Aim; aim_countdown = AIM_TIME; aim_preparing = 0.0` otherwise `shoot_countdown = SHOOT_WAIT }`; AIM/SHOOTING: `max_dist = 1000.0; if self.laser_raycast.is_colliding() { max_dist = (self.ray_from.get_global_transform().origin - self.laser_raycast.get_collision_point()).length(); } self._clip_ray(max_dist);` `aim_preparing` rises up to `AIM_PREPARE_TIME`; `aim_countdown -= delta as f32; if < 0.0 && state == Aim {` same inline raycast to `self.target_position + Vector3::UP`; hits → `state = Shooting; shoot_countdown = SHOOT_WAIT; self.base_mut().rpc("play_shoot", &[]);` otherwise `self.resume_approach(); }`; then `self.animate(delta); self.orientation = self.orientation * Transform3D::new(Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()), self.animation_tree.get_root_motion_position());` velocity/gravity/`set_up_direction(UP)`/`move_and_slide` as in the Player; `orientation.origin = ZERO; orientation = orientation.orthonormalized(); let basis = self.orientation.basis; self.base_mut().set_global_basis(basis);`. `impl EnemyRobot` block **without** `#[godot_api]` with: `fn shoot(&mut self)` (`let gt = self.ray_from.get_global_transform(); let ray_origin = gt.origin; let ray_dir = gt.basis.col_b(); let mut max_dist: f32 = 1000.0;` inline raycast from `ray_origin` to `ray_origin + ray_dir * max_dist`; `if !col.is_empty() { let position = col.get("position").unwrap().to::<Vector3>(); max_dist = ray_origin.distance_to(position); }` (the `if col.collider == player: pass # Kill.` generates no code); `self._clip_ray(max_dist); let mesh_offset = self.ray_mesh.get_position().z; let mut laser_ember = self.base().get_node_as::<CpuParticles3D>("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber"); laser_ember.set_position(Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset)); let mut e = laser_ember.get_emission_box_extents(); e.z = (max_dist - mesh_offset.abs()) / 2.0; laser_ember.set_emission_box_extents(e); if !col.is_empty() { let position = ...; let mut blast: Gd<Node3D> = load::<PackedScene>("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn").instantiate_as::<Node3D>(); self.base().get_tree().get_root().unwrap().add_child(&blast); blast.set_global_position(position); if let Some(player) = self.player.clone() { if <collider == player by instance_id> { if let Ok(player) = player.try_cast::<Player>() { self.base().get_tree().create_timer(0.1).signals().timeout().connect_other(&*self, move |_this: &mut EnemyRobot| { player.clone().bind_mut().add_camera_shake_trauma(13.0); }); } } } }`); `fn animate(&mut self, delta: f64)` (APPROACH: `to_player_local` via the transpose, `angle_to_player`, `set("parameters/state/transition_request", &"turn_left"/"turn_right"/"idle"/"walk".to_variant())` on the 4 conditions in the original's order; outside APPROACH `"idle"`; `if self.target_position != Vector3::ZERO { set("parameters/aiming/blend_amount", &(self.aim_preparing / AIM_PREPARE_TIME).clamp(0.0, 1.0).to_variant()); let mt = self.ray_mesh.get_global_transform(); let to_cannon_local = mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin); let h_angle = to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees(); let v_angle = to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees(); let mut blend_pos = self.animation_tree.get("parameters/aim/blend_position").to::<Vector2>(); blend_pos.x += BLEND_AIM_SPEED * delta as f32 * -h_angle; blend_pos.x = blend_pos.x.clamp(-1.0, 1.0); blend_pos.y += BLEND_AIM_SPEED * delta as f32 * v_angle; blend_pos.y = blend_pos.y.clamp(-1.0, 1.0); set("parameters/aim/blend_position", &blend_pos.to_variant()); }`); `fn _clip_ray(&mut self, length: f32)` (`let mesh_offset = self.ray_mesh.get_position().z; if !Os::singleton().has_feature("dedicated_server") { self.ray_mesh.get_surface_override_material(0).unwrap().cast::<ShaderMaterial>().set_shader_parameter("clip", &(length + mesh_offset).to_variant()); }`) (research D9–D14)
- [x] T019 [US2] `red_robot.rs` — part 3 (Godot block), translating `red_robot.gd:4,72-105,259-265,274-283`: **single** `#[godot_api] impl EnemyRobot` with `#[signal] fn exploded();`; `#[func] fn resume_approach(&mut self) { self.state = State::Approach; self.aim_preparing = AIM_PREPARE_TIME; self.shoot_countdown = SHOOT_WAIT; }`; `#[rpc(authority, call_local, unreliable)] fn hit(&mut self)` (`if self.dead { return; } let param = format!("parameters/hit{}/request", randi() % 3 + 1); self.animation_tree.set(&param, &1.to_variant()); self.hit_sound.play(); self.health -= 1; if self.health == 0 { self.dead = true; self.animation_tree.set_active(false); self.model.set_visible(false); self.death.set_visible(true); self.collision_shape.set_disabled(true); self.death_detach_spark1.set_emitting(true); self.death_detach_spark2.set_emitting(true); self.death_shield1.bind_mut().explode(); self.death_shield2.bind_mut().explode(); self.death_head.bind_mut().explode(); self.explosion_sound.play(); self.signals().exploded().emit(); if self.base().get_multiplayer().unwrap().is_server() { self.base().get_tree().create_timer(10.0).signals().timeout().connect_other(&*self, |this: &mut EnemyRobot| this.base_mut().queue_free()); } }`); `#[rpc(authority, call_local, unreliable)] fn play_shoot(&mut self) { self.shoot_animation.play_ex().name("shoot").done(); }`; `#[func] fn shoot_check(&mut self) { self.test_shoot = true; }`; `#[func] fn _on_area_body_entered(&mut self, body: Gd<Node3D>) { if body.clone().try_cast::<Player>().is_ok() || body.get_name() == StringName::from("Target") { self.player = Some(body); self.state = State::Approach; } }`; `#[func] fn _on_area_body_exited(&mut self, body: Gd<Node3D>) { if body.try_cast::<Player>().is_ok() { self.player = None; self.state = State::Idle; } }`. No other `#[func]`/`#[signal]`/`#[rpc]` (research D6, D14)
- [x] T020 [US2] Add `mod red_robot;` in `oxide_godot_core/oxide_godot_lib/src/lib.rs` (after `mod part;`)
- [x] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. In case of error, research.md D6–D14 and the bindings before improvising
- [x] T022 [US2] Edit `oxide-godot/enemies/red_robot/red_robot.tscn` as text (re-check with `grep -n 'name="RedRobot" type=\|^script = ExtResource("1")\|red_robot.gd' oxide-godot/enemies/red_robot/red_robot.tscn` → expected ≈l.10583 (root) and ≈l.10586 (`script`), l.3 — the root shifted only −1 after US1: the `ext_resource` l.26; the parts' 3 `script` lines are below it): on the root line swap `type="CharacterBody3D"` for `type="EnemyRobot"`; remove `script = ExtResource("1")`; remove the line `[ext_resource type="Script" uid="uid://bf14mo0lrrvjl" path="res://enemies/red_robot/red_robot.gd" id="1"]` (l.3). **KEEP** the root's `collision_layer = 3`/`collision_mask = 3`, `MultiplayerSynchronizer` with `replication_config`, `AnimationTree`, `ShootAnimation` (method tracks `shoot_check`/`resume_approach`), `PlayerDetectionArea`, the 3 `Part` nodes, and the 2 `[connection …]` at the end of the file. Expected diff: `4 +---` (1 swap + 2 removals)
- [x] T023 [US2] Delete `oxide-godot/enemies/red_robot/red_robot.gd` and `oxide-godot/enemies/red_robot/red_robot.gd.uid` (`git rm`); verify `grep -rn 'uid://bf14mo0lrrvjl' oxide-godot/ | grep -v '/.godot/'` empty and `grep -rn 'red_robot.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` empty
- [x] T024 [US2] Headless validation (no editor open): import with `Initialize godot-rust` and no new `ERROR`; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/run.log` → exit 124, regression grep empty (isolated robot: 15 `OnReady` resolve, `ready` activates the `AnimationTree` and calls `animate(0)`, `physics_process` runs the no-player branch — IDLE + gravity); `level/level.tscn` → exit 124, grep empty (`level.gd:97-100` instantiates, assigns `transform`, connects `exploded` **by name**, `add_child(robot, true)`; if the detection area reaches the spawned Player, APPROACH/AIM run too). Only the baseline WARNINGs
- [x] T025 [US2] Mechanical checks and contract (`contracts/red-robot.md` §"Verification before the commit"): `grep -c 'type="EnemyRobot"' oxide-godot/enemies/red_robot/red_robot.tscn` = 1; `grep -c 'ExtResource("1")' ...` = 0; `grep -n '"method": &"shoot_check"\|"method": &"resume_approach"' ...` = 2 lines; `grep -n 'method="_on_area_body_entered"\|method="_on_area_body_exited"' ...` = 2 lines; `grep -n 'properties/[0-9]/path' ... | head -5` = `.:global_transform`, `.:health`, `.:state`, `.:target_position`, `.:dead` — the script's 4 exist in `red_robot.rs` as `#[export]`; `grep -n 'exploded\|EnemyRobot' oxide-godot/level/level.gd` = l.6, 97, 99; `grep -n 'has_method("hit")' oxide_godot_core/oxide_godot_lib/src/bullet.rs` = 1; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` shows only the deletion of `red_robot.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs | grep '^[-+]' | grep -v '^[-+][-+]'` = exactly 2 lines (`-    fn add_camera_shake_trauma(` / `+    pub(crate) fn add_camera_shake_trauma(`); `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/part.rs` empty; `ls src/` = 11 files; `grep -n '#\[func\]\|#\[rpc\|#\[signal\]' -A1 oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep 'fn '` = `exploded`, `resume_approach`, `hit`, `play_shoot`, `shoot_check`, `_on_area_body_entered`, `_on_area_body_exited` (7, no other)
- [x] T026 [US2] Record in `docs/v2-backlog.md` lines no. 16, 17 and 18 (research.md §"Candidate v2 backlog"): 16 — `enemies/red_robot/red_robot.gd` (port 2): remove the dead branch `body.name == "Target"` and type `player` as `Gd<Player>` (no scene has a `Target` node; the generic reference forces a `try_cast` on each use); 17 — replace the 10 s `await` inside the `hit` RPC with a timer/signal outside the RPC (removal coupled to the damage handler); 18 — replicate `aim_preparing` (or not export it) and take `test_shoot` out of the inspector (exported outside the `SceneReplicationConfig`; `test_shoot` is an internal trigger of the method track). Do not duplicate items 1–15
- [x] T027 [US2] Single commit of port 2 on `main`, including `src/red_robot.rs`, `src/lib.rs`, `src/player.rs`, `oxide-godot/enemies/red_robot/red_robot.tscn`, the deletions of `red_robot.gd`/`.uid` and `docs/v2-backlog.md`. Message: `Port red_robot.gd → EnemyRobot (CharacterBody3D); red_robot.tscn: node EnemyRobot type="CharacterBody3D"→"EnemyRobot"` + body: notes (`exploded` signal registered in the main block and connected by name by `level.gd`; `State` enum `via = i64`; `Vector3 * Transform3D` translated as `basis.transposed() * (v - origin)`; raycasts with effective exclusion by the robot's RID and `collider == player` by `instance_id`; typed access to `Part::explode` and `Player::add_camera_shake_trauma` (after `try_cast`, with a 0.1 s delay); `Blast` through base API; `await`s → timers with `connect_other`; quirks preserved: `body.name == "Target"`, `pass # Kill.`, `player: Node3D`, 10 s inside `hit`, `aim_preparing`/`test_shoot` not replicated); `- player.rs: pub(crate) visibility only on add_camera_shake_trauma (typed access from the robot, FR-018); no logic moved`; `- v2 backlog: items 16, 17, 18`. Note the hash
- [x] T028 [US2] **User checkpoint (visual validation, plan.md port 2)** — done by the user in the game, comparing with `../oxide_godot_origins/`: robot standing still far away (IDLE); when approaching, it turns (`turn_left/right`) and walks until facing; ~6 s facing → aims (red laser clipped against the scenery/player, `LaserEmber` along the ray, aim animation follows); ~1 s → shoots ("shoot" animation, impact at the point, **strong shake** if it hits) and goes back to approaching; each shot received → one of the 3 damage animations + sound; 5th shot → death (parts, sparks, explosion sound); robot disappears 10 s later; 15 s after death another robot spawns at the same point (level, `exploded` signal); leaving the detection area → back to IDLE. In the editor, `red_robot.tscn`: root `RedRobot`, no script, `MultiplayerSynchronizer` with `replication_config`, connections of the `PlayerDetectionArea` in the signals panel. Divergence → `Fix port red_robot.gd …` commit. Only proceed to Polish with the explicit OK

**Checkpoint**: 5 `.gd` remaining; `red_robot.tscn` entirely in Rust; game playable.

---

## Phase 5: Polish — final verification of the milestone

**Purpose**: only the quickstart's final verification and the complete visual validation. No
extra documentation, no README, no refactoring.

- [x] T029 Mechanical verification of the milestone (quickstart §"Final verification"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 5; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 5; `git diff --stat 4bb8f7f -- 'oxide-godot/**/*.gd'` shows exactly 2 deletions (`parts/part.gd`, `red_robot.gd`) and no modification (the 5 remaining byte-for-byte equal — SC-001); `git log --oneline 4bb8f7f..HEAD | grep -c '^[0-9a-f]* Port '` = 2 (`Fix port …` do not count); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 10 modules (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot`); `grep -c '^| 1 ' docs/upstream-bugs.md` = 1 and `git diff --stat 4bb8f7f -- docs/upstream-bugs.md CLAUDE.md` empty; `grep -c '^| [0-9]' docs/v2-backlog.md` = 18; FR-018: `grep -nE '\.call\(|\.call_deferred\(|get_script' oxide_godot_core/oxide_godot_lib/src/part.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs` empty; `grep -nE '\.get\("' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → only `"parameters/aim/blend_position"`, `"position"`, `"collider"`; `grep -nE '\.set\(' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → only `"parameters/…"` and the `set(&param, …)` of `hit`; `grep -n 'affine_inverse\|Rid::Invalid' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` empty
- [x] T030 Final headless validation (no editor open): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; headless import with `Initialize godot-rust` and no `ERROR`; `enemies/red_robot/red_robot.tscn`, `level/level.tscn`, `player/player.tscn`, `player/bullet/bullet.tscn`, `door/door.tscn` headless with regression grep empty
- [x] T031 **Complete visual validation by the user (SC-002)**: menu → level; robots patrol/turn, aim with laser clipped, shoot (impact + shake 13.0 on hit), react to shots (animation + sound), die on the 5th shot (parts fly, sparks, sound, fade + puff between 3 and 6.5 s), respawn 15 s later; player (Milestone B) and effects (Milestone A) remain the same — everything indistinguishable from `../oxide_godot_origins/` in a side-by-side session; `red_robot.tscn` in the editor with root `RedRobot` and 3 `Part` nodes, no scripts. Milestone C completed with the user's OK; then mark T001–T031 `[x]` and commit only `tasks.md` (`Tasks 003: marco C concluído`)

---

## Dependencies & Execution Order

### Mandatory order

```
Phase 1 (Setup: T001–T005)          — baseline of red_robot.tscn and level.tscn
  → Phase 3 US1 (T006–T015)  → commit 1  (Part; red_robot.tscn: 3 nodes + ext_resource id="24")
  → Phase 4 US2 (T016–T028)  → commit 2  (EnemyRobot; red_robot.tscn: root + ext_resource id="1"; player.rs pub(crate))
  → Phase 5 Polish (T029–T031)
```

- **Phase 2 (Foundational)**: does not exist — nothing blocks the stories besides the Setup.
- **Stories are not parallelizable with each other**: each one ends with a commit on `main` and the
  next one starts from the clean tree (FR-026, SC-004). US2 requires `Part` in Rust (`OnReady<Gd<Part>>`,
  `bind_mut().explode()`), and both edit `red_robot.tscn` — US1 shifts US2's lines
  by −1 (only the `ext_resource` l.26; the parts' 3 `script` lines, l.10841+, are below the root and do not shift it — they shift by −4 only the lines ≥ 10841, such as the connections).
- **Internal order of each story** (strict dependencies): [US2: `pub(crate)` visibility in
  `player.rs`] → `.rs` module → `mod` in `lib.rs` → `cargo build` → edit `.tscn` → delete
  `.gd`/`.uid` → headless → mechanical checks + contract → backlog → commit → user
  checkpoint. The `.tscn` is only edited after the build because the class needs to exist in the loaded
  lib for the `type` to resolve in the headless import.
- **User checkpoint** (T015, T028, T031) is blocking: the next story only starts with the
  explicit OK.

### Parallel Opportunities

Practically none, by construction:

- Setup: T004 is [P] with respect to T002/T003/T005 (it only reads the bindings directory).
- Within each story, no task is [P]: each step consumes the result of the previous one.
  T017→T018→T019 write the same `red_robot.rs` in sequence.
- Backlog (T013, T026) could be written at any time before the story's commit, but
  it edits `docs/v2-backlog.md`, shared — keep sequential.

### Parallel Example

```bash
# Only truly independent pair (Phase 1):
Task: "T002 cargo build → count warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline recorded.
2. Phase 3: US1 (T006–T015) — `part.gd` ported, commit 1, user's OK.
3. **STOP AND VALIDATE**: first class without a scene of its own (`type` swap on 3 nodes of the same
   scene), first setter that writes to a shader and first `duplicate_resource()`; any
   node path (`Model` → child 0), material or replication problem shows up here.

### Incremental Delivery

Each story is a complete port and the game stays playable after each commit:

1. US1 → 6 `.gd` remaining → playable (GDScript robot calls `explode()` by name on the Rust class)
2. US2 → 5 → playable (whole enemy in Rust; `level.gd` receives `exploded` by name)
3. Polish → milestone C completed

### If something fails in the middle of a story

Do not commit partially. Either the whole port (module + scene + deletion [+ visibility]) goes into the
commit, or nothing: `git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<module>.rs`
returns to the previous story's clean tree, which is always playable. If the implementation requires something not
foreseen in the tasks (another file, another visibility, a fix), stop and report.

---

## Notes

- No task creates a helper, trait, common module, test or new log. If it seems necessary, it is
  an entry in `docs/v2-backlog.md` — not a task. The robot's three raycasts stay inline (D12).
- **No bug fix** in this milestone. Any objective defect found → stop and
  report (the clause requires the spec first); anything else is an improvement → backlog. In case
  of doubt, it is an improvement.
- `red_robot.tscn` (11,053 lines) is edited as text in both stories; re-check lines with
  `grep -n` immediately before each `sed` (the cited lines are those of 2026-09-15; US1
  shifts US2's).
- Names of `#[func]`, RPCs, signal and `#[export]`/`#[var]` properties are contract (FR-023) —
  copy from the GDScript, never "translate" (`_on_area_body_entered`, `shoot_check`, `fade_value`,
  `exploded`).
- The headless regression grep is `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked`;
  the baseline WARNINGs (`HDR output`, `Physics interpolation`) do not count.
- No Godot editor open during headless validations — warn the user beforehand; never kill their
  process.
- The commit (T014, T027) comes **before** the visual checkpoint; a divergence found by the user
  is fixed in a `Fix port …` commit in the same story (never `Port …`, so that
  `git log | grep -c '^[0-9a-f]* Port '` remains = 2).
- Files that **never** change in this milestone: `Cargo.toml`, `.gdextension`, `project.godot`,
  `CLAUDE.md`, `docs/upstream-bugs.md` (except entry #2 of Phase 3b), the 5 remaining `.gd`, `specs/` (except the `[x]` in
  `tasks.md` at the end) and, in `player.rs`, anything beyond the `pub(crate)` keyword of T016.
