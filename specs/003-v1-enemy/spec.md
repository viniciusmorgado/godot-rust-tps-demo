# Feature Specification: Milestone C — enemy: part and red robot (v1 raw port)

**Feature Branch**: `003-v1-enemy` (work on `main`, as in Milestones A and B)

**Created**: 2026-09-15

**Status**: Draft

**Phase**: v1 — Raw Port (Principle I of constitution v1.3.0). No abstraction, refactoring or optimization; perceived improvements go to `docs/v2-backlog.md`. One objective upstream defect was found during the review of port 1 (part: shield material shared — see "Conservative upstream bug fix — part") and is fixed under the conservative-fix clause (declare in spec, isolate with a comment, mention in the commit, record in `docs/upstream-bugs.md`). No other defect is known.

**Input**: User description: "Port the Godot TPS Demo enemy to Rust — the detachable part (part.gd) and the red robot (red_robot.gd) — keeping the game playable and identical to the original at each script. Milestone C of docs/port-order.md (items 9 and 10)."

## Context

Continuation of the script-by-script port. Milestones A and B (`specs/001`, `specs/002`, commits up to `dcf3816`) left 7 scripts in the original and delivered `Player`, `PlayerInputSynchronizer`, `CameraNoiseShake`, `Blast` and `PartDisappear` as native classes. This milestone ports the complete enemy: the part that detaches on the robot's death and the robot itself. The 5 remaining scripts (`flying_forklift`, `level`, `menu`, `main`, `settings`) stay byte-for-byte intact and keep consuming the ported API by the original names.

| # | Original script | Base | Affected node/scene | Lines | Consumers that remain in the original |
|---|---|---|---|---|---|
| 1 | `enemies/red_robot/parts/part.gd` | RigidBody3D | **no scene of its own** — attached to 3 nodes of `enemies/red_robot/red_robot.tscn`: `Death/PartShield1` (l.10833), `Death/PartShield2` (l.10885), `Death/PartHead` (l.10936); `ext_resource id="24"` (l.26) | 57 | `red_robot.gd:96-98` (`explode()` on the 3 parts — GDScript until port 2) |
| 2 | `enemies/red_robot/red_robot.gd` | CharacterBody3D | root of `enemies/red_robot/red_robot.tscn` (l.10584); `ext_resource id="1"` (l.3) | 283 | `level.gd:97-100` (instantiates, `transform`, `exploded.connect`, `add_child(robot, true)`); bullet (`has_method("hit")` → `rpc("hit")`, already a native class) |

Behavior reference: the untouched original project at `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Robot part ported (Priority: P1)

When a robot dies, its three parts (two shields and the head) come loose, fly with random rotation, fall with physics, stay on the ground for a few seconds, vanish in a fade and end with a puff of smoke — exactly as today. All of this is now produced by the part's native class, while the robot (still in the original) keeps calling `explode()` on it by name.

**Why this priority**: The part is a leaf (it only consumes `PartDisappear`, already a native class, through base API) and is a dependency of the robot (`explode()` called by name; in port 2 the access becomes typed). Without it ported, the robot cannot be ported with typed access (Principle II, bottom-up order).

**Independent Test**: `red_robot.tscn` and `level.tscn` headless with no new errors; in the game, killing a robot produces the same spectacle of parts (flight, fall, fade, puff) as the original.

**Acceptance Scenarios**:

1. **Given** `red_robot.tscn` is instantiated, **When** each of the 3 parts enters the scene, **Then** per-frame processing is off and, outside a dedicated server, the part now has its own copy of the material of surface 0 of its model (and of that copy's `next_pass`), so that one part's fade affects neither the others nor the original model.
2. **Given** the part is frozen in the scene (`freeze`), **When** the robot dies and calls `explode()`, **Then** the part's synchronization becomes public, freezing is turned off and, on the server, both collisions are enabled, the linear velocity is 3 units/s upward and the angular one is `(normalized(rand, rand, rand) × 2 − ONE) × 10` (three randoms in [0, 1], normalized, then × 2 − 1 on each component — the exact formula of FR-003, not a uniform random in [−1, 1]).
3. **Given** the part exploded on the server, **When** `lifetime + lifetime_random × random` seconds elapse (3 to 6 s), **Then** per-frame processing is turned on and the fade begins.
4. **Given** the fade is active, **When** frames pass, **Then** `fade_value = (counter / disappearing_time)²` (0 → 1 in 0.5 s, with exponent 2) and the value is applied to the `emission_cutout` shader parameter of the part material's `next_pass`; when the counter reaches `disappearing_time − 0.2`, the `destroy` RPC is fired and processing is turned off.
5. **Given** the `destroy` RPC runs, **When** it executes, **Then** a `part_disappear.tscn` effect (native class `PartDisappear`) is instantiated as a child of the part's **parent**, at the part's global position; 0.2 s later the part is removed from the scene.
6. **Given** the peer is not the server, **When** `explode()` is called, **Then** only the public visibility of the synchronization and the unfreezing happen; collisions, velocities and the fade timer do not (branch by code reading).
7. **Given** the replication configured in the scene (`fade_value`, `position`, `rotation`, `linear_velocity`, `angular_velocity` — `red_robot.tscn:10419-10431`), **When** the part is synchronized, **Then** `fade_value` remains accessible by name with the setter applying the value to the shader; the others are base properties.
8. **Given** `red_robot.gd` stays in the original until port 2, **When** it calls `death_shield1.explode()` etc. (`red_robot.gd:96-98`), **Then** it resolves by name without any edit.

---

### User Story 2 - Red robot ported (Priority: P2)

The robot remains the same enemy: it stays still until it detects the player, turns and walks until it is facing them, prepares its aim (visible laser, clipped against the scenery), shoots (impact at the hit point and strong shake on the player's camera if it hits them), reacts to each shot received with a damage animation and sound, and dies on the fifth shot — parts flying, sparks, explosion sound — being removed 10 s later; the level keeps receiving the `exploded` signal and respawning another robot 15 s later.

**Why this priority**: Largest script of the milestone and the last one with dependents outside it (`level.gd` via `exploded`; bullet via `hit`). Consumes `Player` (typed), `Part` (typed, port 1) and `Blast` (base API). Closes the whole enemy as native classes.

**Independent Test**: `red_robot.tscn` and `level.tscn` headless with no new errors; contract checked (`hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered/_exited`, `exploded`, replicated properties); in the game, the cycle detect → approach → aim → shoot → get hit → die → respawn indistinguishable from the original.

**Acceptance Scenarios**:

1. **Given** the level instantiates the robot (`level.gd:97-100`: `transform` assigned, `exploded` connected, `add_child(robot, true)`), **When** the node enters the scene, **Then** `orientation` = global transform with zeroed origin, the `AnimationTree` becomes active, `shoot_countdown` = 0 if `test_shoot`, and if `dead` the model becomes invisible, the collision disabled and the `AnimationTree` inactive; `animate(0)` runs once.
2. **Given** the robot is IDLE and the player enters the `PlayerDetectionArea`, **When** `body_entered` fires `_on_area_body_entered(body)`, **Then** if `body` is `Player` **or** `body.name == "Target"`, `player = body` and `state = APPROACH`; on exit, if `body` is `Player`, `player = null` and `state = IDLE`.
3. **Given** the robot is in APPROACH with the player outside the ±15° tolerance, **When** physics frames pass, **Then** the animation is `turn_left` (angle > 15°) or `turn_right` (< −15°); within the tolerance, `walk`; with no target (`target_position` zero), `idle`; outside APPROACH, always `idle`. The angle is `atan2(x, z)` of the target in the robot's local space (front = +Z).
4. **Given** the robot is facing the player in APPROACH, **When** `shoot_countdown` (6 s, decremented only while facing) becomes negative, **Then** a ray from the origin of `RayFrom` to the player's origin + UP (mask 0xFFFFFFFF, excluding the robot itself) is cast; if it hits the player: `state = AIM`, `aim_countdown = 1 s`, `aim_preparing = 0`; otherwise `shoot_countdown = 6 s`.
5. **Given** the robot is in AIM or SHOOTING, **When** frames pass, **Then** the laser is clipped at the `RayCast` distance (or 1000), `aim_preparing` rises up to 0.5 s, `aim_countdown` decrements; when it becomes negative in AIM, the same ray is cast: if it hits the player, `state = SHOOTING`, `shoot_countdown = 6 s` and the `play_shoot` RPC plays the "shoot" animation; otherwise `resume_approach()` (APPROACH, `aim_preparing = 0.5`, `shoot_countdown = 6`).
6. **Given** the "shoot" animation is playing, **When** its method tracks fire (`shoot_check` at 2.25 s, `resume_approach` at 3 s — `red_robot.tscn:10296-10299`), **Then** `shoot_check` sets `test_shoot = true`, the next physics frame calls `shoot()` and clears `test_shoot`; `resume_approach` returns to APPROACH.
7. **Given** `shoot()` runs, **When** the ray from the origin of `RayFrom` in the direction of the Y axis of its global basis (range 1000, mask 0xFFFFFFFF, excluding the robot) collides, **Then** `max_dist` = distance to the point; the laser is clipped at `max_dist`; `LaserEmber` is positioned at `(0, 0, −max_dist/2 − offset_z)` with `emission_box_extents.z = (max_dist − |offset_z|)/2`; an `impact_effect.tscn` (native class `Blast`) is instantiated as a child of the tree root at the impact position; if the collider is the `player` and it is `Player`, 0.1 s later `add_camera_shake_trauma(13.0)` is called on it with typed access.
8. **Given** the target exists (`target_position ≠ 0`), **When** `animate` runs, **Then** `parameters/aiming/blend_amount = clamp(aim_preparing / 0.5, 0, 1)` and `parameters/aim/blend_position` is adjusted incrementally (`0.05 × delta × −h_angle` in x, `0.05 × delta × v_angle` in y, both in degrees, clamped to [−1, 1]) from the angles of the target + UP in the local space of the `RayMesh`.
9. **Given** the robot is hit by a bullet (`hit` via RPC), **When** it runs, **Then** if `dead` it returns; otherwise one of the three damage animations (`parameters/hit{1|2|3}/request = 1`, random draw) fires, the Hit sound plays and `health` decrements.
10. **Given** `health` reaches 0, **When** `hit` runs, **Then** `dead = true`, `AnimationTree` inactive, model invisible, `Death` visible, collision disabled, `DetachSpark1/2` emitting, `explode()` on the 3 parts with **typed** access (`Part`), Explosion sound, `exploded` signal emitted; on the server, 10 s later the robot is removed from the scene.
11. **Given** `level.gd:99` connected `exploded` to `_respawn_robot`, **When** the signal is emitted, **Then** the level waits 15 s and spawns another robot at the same point — with no edit to `level.gd`.
12. **Given** there is no detected player, **When** physics frames pass on the server, **Then** `target_position = 0`, `animate`, velocity = gravity × delta, `move_and_slide` with up = +Y, and the rest is skipped.
13. **Given** the peer is not the server, **When** physics frames pass, **Then** only `animate(delta)` runs (branch by code reading); if `dead`, nothing runs.
14. **Given** the configured replication (`red_robot.tscn:30-42`: `global_transform`, `health`, `state`, `target_position`, `dead`), **When** the robot is synchronized, **Then** `health`, `state`, `target_position`, `dead` remain accessible by name; `aim_preparing` and `test_shoot` are exported but not replicated (as in the original).
15. **Given** `level.gd` and the bullet stay as they are, **When** they use `exploded`, `hit`, `transform`, **Then** everything resolves without any edit.

---

### Edge Cases

- **Part: `fade_value` before the material exists**: the setter only applies to the shader if the material was already duplicated in `ready`; values assigned before that (replication at spawn, inspector) are only stored — behavior of the original.
- **Part: dedicated server**: no material duplication (`dedicated_server` feature); the fade has no visual effect. Branch by code reading.
- **Part: `explode()` before `ready`**: does not occur (the robot only calls it on death, long after the spawn).
- **Part: puff instantiated on the part's parent** (`Death`, child of the robot): the robot is removed 10 s after death and the parts' fade ends in ≤ 6.5 s (+ 0.2 s of the puff), so the effect is born and gone before that; if the timings crossed, the puff would leave together with the robot. Behavior of the original, preserved.
- **Robot: `body.name == "Target"`**: no scene in the project has a node named `Target` (empty grep); the branch is preserved as dead code of the original.
- **Robot: `player` typed as `Node3D`**: it may be a `Target` (not `Player`); that is why `shoot()` tests `player is Player` before `add_camera_shake_trauma`. The typing of the reference remains `Node3D`; only the call is typed after the test.
- **Robot: `hit` during the 10 s `await`**: `dead` is already `true`, it returns. Bullets that still collide call `hit` with no effect.
- **Robot: `shoot()` with `col.collider == player`**: the original's `pass # Kill.` does nothing — preserved (no "implementing the player's death").
- **Robot: exclusion in the raycast**: `[self]` excludes the body itself (valid RID of the `CharacterBody3D`) — here the exclusion **is** effective, unlike the `player_input.gd` quirk; preserve the semantics (exclude the robot).
- **Robot: `orientation` vs `global_transform`**: the robot writes `global_transform.basis = orientation.basis` on the body itself (not on a child model, as the player does).
- **Robot dead at spawn (`dead = true` replicated)**: model invisible and without collision since `ready`; `_physics_process` always returns.
- **Robot: `hit` with `health` already negative**: only `== 0` triggers death; if a frame with two bullets takes `health` to −1, the second `hit` already sees `dead` — original.

## Requirements *(mandatory)*

### Functional Requirements

**Behavior — part (US1)**

- **FR-001**: The part MUST have base `RigidBody3D` and export `lifetime` (3.0), `lifetime_random` (3.0), `disappearing_time` (0.5) and `fade_value` (0.0); the `fade_value` setter MUST store the value and, if the material already exists, set the `emission_cutout` shader parameter of the material's `next_pass`.
- **FR-002**: On entering the scene it MUST turn off per-frame processing and, outside a dedicated server, duplicate the material of surface 0 of the mesh of the first child of `Model`, assign it to surface 0 and duplicate the copy's `next_pass`.
- **FR-003**: `explode()` MUST exist with that name (called by `red_robot.gd:96-98`): make the child `MultiplayerSynchronizer`'s synchronization public, unfreeze; if not server, return; enable `Col1`/`Col2`; `linear_velocity = 3 × UP`; `angular_velocity = (normalized random × 2 − ONE) × 10`; wait `lifetime + lifetime_random × random` s and turn processing on.
- **FR-004**: Per frame it MUST do `fade_value = (counter / disappearing_time)²`, increment the counter by `delta` and, on reaching `disappearing_time − 0.2`, fire the `destroy` RPC and turn processing off.
- **FR-005**: The `destroy` RPC (`authority`, `call_local`, `unreliable`) MUST instantiate `part_disappear.tscn` as a child of the part's parent at the part's global position, wait 0.2 s and remove the part. Only base API of `CpuParticles3D`/`Node3D` is used on the effect.
- **FR-006**: The loading of `part_disappear.tscn` MUST happen via `load` at the point of use (Milestone B pattern for `preload`).

**Behavior — robot (US2)**

- **FR-007**: The robot MUST have base `CharacterBody3D`, signal `exploded` (no arguments), enum `State {IDLE=0, APPROACH=1, AIM=2, SHOOTING=3}`, constants `PLAYER_AIM_TOLERANCE_DEGREES` = 15° in radians, `SHOOT_WAIT` 6.0, `AIM_TIME` 1.0, `AIM_PREPARE_TIME` 0.5, `BLEND_AIM_SPEED` 0.05.
- **FR-008**: It MUST export `test_shoot` (false), `target_position` (zero), `health` (5), `state` (IDLE), `dead` (false), `aim_preparing` (0.5); `target_position`, `health`, `state`, `dead` MUST remain accessible by name for replication (`red_robot.tscn:30-42`).
- **FR-009**: Internal state: `shoot_countdown` (6.0), `aim_countdown` (1.0), `player` (reference to `Node3D`, null), `orientation`; scene references `AnimationTree`, `ShootAnimation`, `RedRobotModel`, `RedRobotModel/Armature/Skeleton3D/RayFrom` (+ `RayMesh`, `RayCast`, `LaserEmber`), `CollisionShape3D`, `SoundEffects/Explosion`, `SoundEffects/Hit`, `Death` (+ `PartShield1`, `PartShield2`, `PartHead`, `DetachSpark1`, `DetachSpark2`).
- **FR-010**: On entering the scene it MUST reproduce `red_robot.gd:57-69` (orientation, `AnimationTree` active, `test_shoot` → `shoot_countdown = 0`, `dead` branch, `animate(0)`).
- **FR-011**: `resume_approach()` and `shoot_check()` MUST exist with those names (method tracks of the "shoot" animation, `red_robot.tscn:10296-10299`) and the effects of `red_robot.gd:72-75,264-265`.
- **FR-012**: The `hit` RPC (`authority`, `call_local`, `unreliable`) MUST reproduce `red_robot.gd:78-105`: return if `dead`; randomly drawn damage animation (`parameters/hit{1|2|3}/request = 1`); Hit sound; `health −= 1`; death at `health == 0` (flags, visibility, collision, sparks, **typed** `explode()` on the 3 parts, Explosion sound, `exploded` signal); on the server, removal after 10 s.
- **FR-013**: `shoot()` MUST reproduce `red_robot.gd:108-133`: ray from `RayFrom` in the direction of the Y axis of its global basis, range 1000, mask 0xFFFFFFFF, excluding the robot itself (effective exclusion); laser clip; position and extents of `LaserEmber`; instantiation of `impact_effect.tscn` (class `Blast`, base API) as a child of the tree root at the impact point; if `col.collider == player` and `player` is `Player`, after 0.1 s `add_camera_shake_trauma(13.0)` with typed access. The `pass # Kill.` MUST remain without effect.
- **FR-014**: `animate(delta)` MUST reproduce `red_robot.gd:136-168`, including the inverse transformation (GDScript's `target_position * global_transform` = point in the robot's local space; likewise for the `RayMesh`), `atan2(x, z)` for the angle to the player, `atan2(x, −z)`/`atan2(y, −z)` in degrees for the aim, and the read/write of `parameters/aim/blend_position`.
- **FR-015**: The physics frame MUST reproduce `red_robot.gd:171-256` in order: `dead` → return; not server → `animate` and return; `test_shoot` → `shoot()`; no `player` → zero target, `animate`, gravity, `move_and_slide`, return; APPROACH (decrement of `aim_preparing`, tolerance, `shoot_countdown`, raycast, AIM transition); AIM/SHOOTING (clip, `aim_preparing`, `aim_countdown`, raycast, SHOOTING transition + `play_shoot` RPC or `resume_approach`); `animate`; root motion; velocity; gravity; `move_and_slide`; orthonormalization; `global_transform.basis = orientation.basis`.
- **FR-016**: The `play_shoot` RPC (`authority`, `call_local`, `unreliable`) MUST play "shoot" on the `ShootAnimation`; `_clip_ray(length)` MUST, outside a dedicated server, set the `clip` parameter of the surface 0 override material of the `RayMesh` to `length + offset_z`.
- **FR-017**: `_on_area_body_entered(body)` and `_on_area_body_exited(body)` MUST exist with those names (connections `red_robot.tscn:11050-11051`) and the logic of `red_robot.gd:274-283` (`is Player` or `name == "Target"`; `is Player`).
- **FR-018**: Access to the `Player` (`add_camera_shake_trauma`) and to the part (`explode`) MUST be typed; for that, and only for that, the visibility of `add_camera_shake_trauma` on the ported player and of `explode` on the part may be opened internally (never to GDScript beyond what is already exposed) in the robot's commit (Milestone B precedent: visibility only, cited in the message). The ported code MUST NOT call custom GDScript API (the robot consumes no remaining script; `Settings` is not used). The instantiated `blast`/`puff` MUST be typed by the base (`Node3D`/`CpuParticles3D`), never by `Blast`/`PartDisappear`.
- **FR-019**: `impact_effect.tscn` MUST be loaded via `load` at the point of use (`preload` → `load` pattern).

**Port cycle — common to both (Principle II)**

- **FR-020**: Each script MUST become exactly one native class with the same base (`RigidBody3D`, `CharacterBody3D`).
- **FR-021**: The binding MUST be by swapping `type` in the `.tscn`: in port 1, on the 3 nodes `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` (removal of the 3 `script = ExtResource("24")` lines and of the `ext_resource id="24"`); in port 2, on the root `RedRobot` (removal of `script = ExtResource("1")` and of the `ext_resource id="1"`). No bridge `.gd`. Base properties on the 3 part nodes (`transform`, `collision_layer/mask`, `mass`, `physics_material_override`, `freeze = true`, `angular_damp`) and their children (`MultiplayerSynchronizer` with `public_visibility = false`, `Model`, `Col1`, `Col2`) MUST stay intact.
- **FR-022**: `.gd` and `.gd.uid` MUST be deleted in the same commit as the port.
- **FR-023**: Names of exposed methods, RPCs, signal and exported/replicated properties MUST be identical to the GDScript: part — `explode`, `destroy`, `lifetime`, `lifetime_random`, `disappearing_time`, `fade_value`; robot — `hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered`, `_on_area_body_exited`, `exploded`, `test_shoot`, `target_position`, `health`, `state`, `dead`, `aim_preparing`. Checked against `red_robot.tscn` (replication l.30-42 and l.10419-10431; method tracks l.10296-10299; connections l.11050-11051) and `level.gd:99`.
- **FR-024**: Each code change MUST be followed by a debug build with no new warnings.
- **FR-025**: Each port MUST be validated headless: import with extension loading + `red_robot.tscn` and `level.tscn` with no new errors beyond the baseline (the 3 from `CLAUDE.md`).
- **FR-026**: One commit per script, in order 1 → 2; the game MUST remain playable after each commit.
- **FR-027**: The 5 scripts outside the milestone MUST remain byte-for-byte intact; perceived improvements MUST go to `docs/v2-backlog.md` in the same commit; no bug fix is planned — if an objective defect appears, it MUST follow the 4 requirements of the clause (spec, comment, commit, `docs/upstream-bugs.md`) or, in doubt, go to the backlog as an improvement.

**Conservative upstream bug fix — part (Principle I, v1.3.0)**

- **FR-028**: The defect: `part.gd:23-26` duplicates the material of surface 0 and installs it with `mesh.mesh.surface_set_material(0, _mat)` — on the **`Mesh` resource**, which is shared by `Death/PartShield1` and `Death/PartShield2` (same model scene, `red_robot.tscn` ext_resource id="12"). Result (verified on the original in headless): the second shield to enter duplicates the material already installed by the first and reinstalls it on the same `Mesh`; the mesh now renders shield 2's material for **both**; shield 1's `fade_value` goes to a material nobody renders (shield 1 vanishes without fade) and shield 2's fades both. The head (`PartHead`) has its own mesh and is not affected. Unambiguous intent of the code (own copy of the material per part, so the fade is independent) contradicted by the result → **bug**.
- **FR-029**: The fix MUST be minimal: install the copy as a surface *override* on the instance (`MeshInstance3D.set_surface_override_material(0, copy)`) instead of mutating the shared `Mesh`. Nothing else changes: the duplication of the material and of the `next_pass`, the `fade_value` setter, `explode`, `process`, `destroy` and the scene stay as they are. It is FORBIDDEN to make the mesh `resource_local_to_scene`, to edit `red_robot.tscn` or the models, or to restructure the part.
- **FR-030**: The fix MUST be isolated and identifiable, with a `// upstream bug fix: ...` comment on the line immediately above the replaced call, in its own commit `Fix port part.gd …` (do not rewrite commit `9aee2b8`), whose message mentions the fix.
- **FR-031**: `docs/upstream-bugs.md` MUST receive entry #2 (defect, script/scene, fix applied, commit by subject) in the same commit.
- **FR-032**: Expected result after the fix: the three `MeshInstance3D` of the parts render distinct materials (per-instance override; `get_surface_override_material(0)` distinct between the parts and distinct from the `Mesh`'s material, which stays untouched), and the rendered `emission_cutout` of each part follows its own `fade_value`.

### Key Entities

- **Part (contract consumed by GDScript until port 2 and by the scene)**: methods `explode()` (called by name by `red_robot.gd`), RPC `destroy()`; exported properties `lifetime`, `lifetime_random`, `disappearing_time`, `fade_value` (setter with effect on the shader; replicated); internal state `_mat` (duplicated material), `_disappearing_counter`; no scene of its own — 3 instances in `red_robot.tscn`.
- **EnemyRobot (contract consumed by `level.gd`, by the bullet and by the scene)**: signal `exploded`; RPCs `hit`, `play_shoot`; methods `shoot_check`, `resume_approach` (method tracks), `_on_area_body_entered`, `_on_area_body_exited` (connections); exported properties `test_shoot`, `target_position`, `health`, `state` (enum), `dead`, `aim_preparing`; internal methods `shoot`, `animate`, `_clip_ray` (not consumed externally — private, as in the previous milestones).
- **Robot internal state**: `shoot_countdown`, `aim_countdown`, `player: Node3D`, `orientation`; 15 scene references listed in FR-009.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the milestone the project contains exactly **5** `.gd` files (and 5 `.uid`); the 5 are byte-for-byte identical to the state before the milestone.
- **SC-002**: In the game, the robot's cycle — still → detects the player → turns and walks → aims with laser clipped against the scenery → shoots (impact at the point, shake 13.0 if it hits) → receives shots (damage animation + sound) → dies on the 5th shot (parts fly with rotation, sparks, sound; parts vanish with fade and puff between 3 and 6.5 s) → respawn 15 s later — is indistinguishable from the original in `../oxide_godot_origins/` in a side-by-side comparison made by the user.
- **SC-003**: For each of the 2 ports, headless validation (import + `red_robot.tscn` + `level.tscn`) reports zero new errors beyond the 3 catalogued ones.
- **SC-004**: After each of the 2 commits the game is playable end to end.
- **SC-005**: The milestone's history has exactly 2 `Port …` commits and none touches the 5 out-of-scope scripts.
- **SC-006**: Debug build with no new warning on all commits.
- **SC-007**: No ported line introduces abstraction, refactoring or optimization; the only fix is the part's (FR-028–FR-032), with the 4 requirements of the clause met; `docs/upstream-bugs.md` now has 2 entries.
- **SC-008**: `level.gd` keeps receiving `exploded` and respawning; the bullet keeps hitting the robot via `hit`; `red_robot.gd` (until port 2) keeps calling `explode()` on the parts — no warning of a nonexistent method/property in the headless logs or in the editor.

## Assumptions

- Phase v1 (constitution v1.3.0); one conservative bug fix (part, FR-028–FR-032), discovered during the review of port 1 and applied in a separate `Fix port …` commit.
- Visual validation (SC-002) by the user; automated validation exclusively headless (`red_robot.tscn` in isolation instantiates the robot without a player: IDLE + gravity; `level.tscn` exercises spawn and the `exploded` connection). The reviewer may run a parity harness outside the repository.
- Single-player validation: the local peer is server and authority; "client" branches (FR-003, FR-005, FR-012, FR-015) and "dedicated server" branches (FR-002, FR-016) are verified by code reading.
- Out of scope: `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`; any improvement (v2 backlog); real multiplayer with two peers.
- `part.gd` has no scene of its own: the part's "port" is the `type` swap on the 3 nodes of `red_robot.tscn`; the exported properties with default value are not written in the scene (check before editing — if any is, keep it).
- `red_robot.tscn` has 11,053 lines; the edits are text edits on lines checked by `grep` before each change.
- Quirks of the original preserved on purpose (candidates for the v2 backlog, not for a fix): `body.name == "Target"` (dead code); `pass # Kill.` in `shoot()`; `player` typed as `Node3D`; `aim_preparing`/`test_shoot` exported without replication; 10 s `await` inside the `hit` RPC; puff effect instantiated on the part's parent (`Death`), which goes away with the robot.
- The opening of internal visibility for typed access (`add_camera_shake_trauma` on the player; `explode` on the part) follows the Milestone B precedent: visibility only, in the commit that requires it, cited in the message.
