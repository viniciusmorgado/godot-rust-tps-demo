# Feature Specification: Milestone B — player, bullet and door (v1 raw port)

**Feature Branch**: `002-v1-player-bullet-door` (work on `main`, as in Milestone A)

**Created**: 2026-09-15

**Status**: Draft

**Phase**: v1 — Raw Port (Principle I of constitution v1.3.0). No abstraction, refactoring or optimization; noticed improvements go to `docs/v2-backlog.md`. This feature uses, for the first time, the **conservative upstream bug fix** clause (a single fix, in the door — see US3 and FR-030–FR-035).

**Input**: User description: "Port to Rust the player, the bullet and the door of the Godot TPS Demo, keeping the game playable and identical to the original at each script — with a single conservative upstream bug fix (door). Milestone B of docs/port-order.md (items 6, 7, 8)."

## Context

Continuation of the script-by-script port. Milestone A (`specs/001-v1-leaves-and-input`, commits `6b12ebf`..`0e71a49`) left 10 scripts in the original and delivered `PlayerInputSynchronizer` and `CameraNoiseShake` as native classes — which is why the player can now be born with **typed** access to them, with no dynamic calls. This milestone ports the three scripts of the "complete player"; the remaining 7 (`part`, `red_robot`, `flying_forklift`, `level`, `menu`, `main`, `settings`) stay byte-for-byte intact and keep consuming the ported API by the original names.

| # | Original script | Base | Affected node/scene | Lines | Consumers that remain in the original |
|---|---|---|---|---|---|
| 1 | `player/player.gd` (`class_name Player`) | CharacterBody3D | root of `player/player.tscn` | 211 | `red_robot.gd:131,133,275,281` (`is Player`, `add_camera_shake_trauma`), `level.gd:118-119` (`name`, `player_id`), `bullet.gd:31-32` (`hit` via `has_method`) |
| 2 | `player/bullet/bullet.gd` | CharacterBody3D | root of `player/bullet/bullet.tscn` (also instantiated as `BulletCache` in `player.tscn:679`) | 51 | `player.gd` (instantiates the scene — becomes a native class in this milestone) |
| 3 | `door/door.gd` | Area3D | root of `door/door.tscn` | 12 | none (`door.tscn` is not instantiated by any scene — orphan asset) |

Behavior reference: the untouched original project in `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ported player (Priority: P1)

The player continues to be exactly the same character: walks, runs, jumps, lands with sound, aims and shoots; when falling off the map it reappears at the initial point. All of this is now produced by the native `Player` class, while the level (which instantiates it and gives it a `player_id`), the robot (which tests `is Player` and calls `add_camera_shake_trauma`) and the bullet (which calls `hit`) remain in the original, with no edits, finding everything by the usual names.

**Why this priority**: It is the largest script of the milestone, the only one with dependents outside it (`red_robot`, `level`, `bullet`) and a direct prerequisite of the bullet (which it instantiates) and of the door (which tests `is Player`). It is also the first ported script that **consumes** already-ported classes (`PlayerInputSynchronizer`, `CameraNoiseShake`) — it proves the typed integration between native classes.

**Independent Test**: `player.tscn` and `level.tscn` headless with no new errors; in the game, move/jump/aim/shoot/landing/respawn indistinguishable from the original; the robot still hits the player (shake 13.0) and the level still spawns it with the correct name and id.

**Acceptance Scenarios**:

1. **Given** the level instantiates the player, sets `name` and `player_id` **before** adding it to the tree, **When** the node enters the scene, **Then** the child `InputSynchronizer` has multiplayer authority equal to `player_id` (the setter runs outside the tree, as in the original) and the replication of `player_id` (spawn), `motion` and `current_animation` (per frame) remains valid by the scene's names.
2. **Given** the player is on the ground with no input, **When** physics frames pass, **Then** it animates WALK with `blend_position = (0, 0)` and does not move.
3. **Given** the player holds `move_forward`, **When** physics frames pass, **Then** `motion` interpolates linearly toward the input at 10×delta, the model orientation rotates (slerp at 10×delta) toward the camera direction flattened on Y, the WALK animation receives `blend_position = (|motion|, 0)` and the displacement comes from the animation's root motion (horizontal velocity = displacement/delta), with gravity applied and `move_and_slide` with up = +Y.
4. **Given** the player is on the ground and the input synchronizer signals `jumping`, **When** the physics frame runs, **Then** `velocity.y = 5`, the player enters `on_air`, the `jump` RPC plays the JUMP_UP animation and the Jump sound, and `jumping` is zeroed in the synchronizer.
5. **Given** the player is in the air, **When** `velocity.y > 0`, **Then** it animates JUMP_UP; **When** `velocity.y ≤ 0`, **Then** it animates JUMP_DOWN.
6. **Given** the player has been in the air for more than 0.5 s, **When** it touches the ground, **Then** the `land` RPC plays JUMP_DOWN and the Land sound; `airborne_time` goes back to 0.
7. **Given** the player has just entered the scene (initial `airborne_time` = 100), **When** it touches the ground for the first time, **Then** the `land` RPC fires once (quirk of the original, preserved).
8. **Given** the player is aiming, **When** physics frames pass, **Then** the orientation slerps toward the camera base quaternion, it animates STRAFE with `aim/add_amount = get_aim_rotation()` and `strafe/blend_position = (motion.x, −motion.y)`, and the AnimationTree root motion is applied.
9. **Given** the player is aiming, `shooting` active and the 0.4 s cooldown at zero, **When** the physics frame runs, **Then** a bullet is instantiated as a child of the player's **parent** (readable name), positioned at the global origin of `ShootFrom`, oriented toward `shoot_target`, with a collision exception against the player; the `shoot` RPC restarts and enables `ShootParticle` and `MuzzleFlash`, restarts the cooldown, plays the Shoot sound and calls `add_camera_shake_trauma(0.35)`.
10. **Given** the cooldown has not reached zero yet, **When** `shooting` remains active, **Then** no new bullet is created.
11. **Given** the robot hits the player with the laser, **When** `red_robot.gd:133` calls `player.add_camera_shake_trauma(13.0)`, **Then** the camera (`CameraNoiseShake`, via `player_input.camera_camera`) receives `add_trauma(13.0)` — typed call, without `.call()`.
12. **Given** a bullet from **another** player hits this player (only in multiplayer — the player's own bullets have a collision exception with it, `player.gd:142`), **When** `bullet` calls `hit.rpc()` on the collider, **Then** the player receives `add_camera_shake_trauma(0.75)`. Verified by code reading; not observable single-player.
13. **Given** the player falls below y = −40, **When** the physics frame runs, **Then** it is teleported to the initial position captured when entering the scene.
14. **Given** the local peer is not the server, **When** the node enters the scene, **Then** per-frame processing is turned off and, on each physics frame, only `current_animation` (replicated) is animated — branch verified by code reading and headless (validation is single-player, where the peer is server and authority).
15. **Given** `red_robot.gd`, `level.gd` and `door.gd` (until port 3) remain in the original, **When** they use `is Player`, `player_id`, `name`, `add_camera_shake_trauma`, `hit`, **Then** everything resolves with no edits to those scripts.

---

### User Story 2 - Ported bullet (Priority: P2)

When shooting, the visible bullet leaves the barrel, flies in a straight line at 20 units/s, explodes on hitting something (calling `hit` on the target if it has that method — player or robot) or after 5 s, and disappears at the end of the explosion animation. If the shadows option is enabled in the settings, the explosion light casts a shadow.

**Why this priority**: Depends on the player (which instantiates it) and is the first consumer of the `Settings` autoload exception (dynamic access to `config_file`). Introduces duck typing preserved from the original (`has_method("hit")` + `rpc`) and an animation method track (`destroy`) — two contract names.

**Independent Test**: `bullet.tscn` isolated and `player.tscn` (which loads the `BulletCache`) headless with no new errors; in the game, the bullet is visible, explodes on collision and on expiry, the robot reacts to `hit`.

**Acceptance Scenarios**:

1. **Given** the bullet is instantiated by the player on the server, **When** physics frames pass, **Then** it moves `−delta × 20 × basis.z` per frame with `move_and_collide`.
2. **Given** the bullet collides with a body that has a `hit` method (robot; or another player, only in multiplayer), **When** the collision occurs, **Then** `hit` is called via RPC on the collider (duck typing of the original), the collision is disabled, the `explode` RPC fires and the internal `hit` becomes `true`.
3. **Given** the bullet collides with a body **without** a `hit` method (wall), **When** the collision occurs, **Then** it only disables the collision and explodes.
4. **Given** the bullet has been flying for 5 s without colliding, **When** `time_alive` becomes negative, **Then** it marks `hit = true` and explodes; in the following frames it does nothing else.
5. **Given** the `explode` RPC fires, **When** it runs, **Then** it plays the "explode" animation and, if `Settings.config_file` has `rendering/shadow_mapping` true, enables `shadow_enabled` on the light.
6. **Given** the "explode" animation reaches the `destroy` method track, **When** the peer is the server, **Then** the bullet is removed from the scene; **When** it is not the server, **Then** nothing happens (the server replicates the removal).
7. **Given** the peer is not the server, **When** the bullet enters the scene, **Then** physics processing is turned off and the collision disabled (branch by code reading/headless).
8. **Given** `player.tscn` loads, **When** the `BulletCache` (instance of `bullet.tscn`) is created, **Then** the ported class is instantiated there with no error and no visible effect (pre-warming, as in the original).
9. **Given** the scene replication (`global_transform`), **When** the bullet moves, **Then** the synchronization configuration remains valid with no edits.

---

### User Story 3 - Ported door, with conservative upstream bug fix (Priority: P3)

The door opens ("doorsimple_opening" animation) when the player enters its area, a single time. In the original this **never happens**: the script looks for `DoorModel/AnimationPlayer`, but the scene node is named `DoorModel2`; the reference stays null, Godot prints `Node not found` on instantiation and the door does not open. The intent of the code is unambiguous (open when a `Player` enters) and the result contradicts it — it is a **bug**, not an improvement (constitution v1.3.0, Principle I).

**Declared minimal fix (requirement (a) of the clause)**: the ported class references `DoorModel2/AnimationPlayer`. Nothing else changes: the scene node is not renamed, the `open` logic is not touched, `door.tscn` only receives the type swap and the script removal. The `doorsimple_opening` animation exists in that `AnimationPlayer` (verified in the model `door/model/door.dae`).

**Why this priority**: Smallest script, with no consumers (orphan asset — no scene instantiates `door.tscn`), and the only one with a bug fix: it goes last so that the fix is an isolated, auditable commit. Depends on US1 (`is Player`).

**Independent Test**: `door.tscn` headless instantiates **without** the `ERROR: Node not found` that the original produces; in an isolated test scene (outside the repo or temporary, uncommitted) a `Player` entering the area makes the animation play once.

**Acceptance Scenarios**:

1. **Given** `door.tscn` is instantiated in isolation, **When** it enters the scene, **Then** no error is printed (in the original: `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`).
2. **Given** the door is closed (`open = false`), **When** a body that is a `Player` enters the area (`body_entered` signal connected in the scene to `_on_door_body_entered`), **Then** the "doorsimple_opening" animation plays and `open = true`.
3. **Given** the door is already open, **When** a `Player` enters again, **Then** nothing happens.
4. **Given** a body that is **not** a `Player` (bullet, robot, part) enters the area, **When** the signal fires, **Then** nothing happens.
5. **Given** the fix has been applied, **When** the compliance review runs, **Then** it finds the four requirements: declaration in this spec, comment `// upstream bug fix: ...` at the exact spot of the reference, mention in the commit message, and entry in `docs/upstream-bugs.md` (defect, script/scene, fix, commit).
6. **Given** the game runs end to end, **When** compared to the original, **Then** nothing visible changes (the door is not in any scene of the game).

---

### Edge Cases

- **Order `player_id` → tree**: the level sets `player_id` before `add_child`; the setter needs to reach the child `InputSynchronizer` outside the tree (the child already exists as part of the instantiated scene). Quirk preserved.
- **First landing**: `airborne_time` starts at 100 → the first contact with the ground triggers `land` (landing sound on spawn), as in the original.
- **`jumping` zeroed by the player**: on each physics frame the player writes `player_input.jumping = false`, even without jumping — contract with `PlayerInputSynchronizer.jumping` (typed write).
- **Shooting with cooldown**: `FireCooldown` is `autostart` of 0.4 s — in the first 0.4 s after spawn it is not possible to shoot (original).
- **Bullet born inside a collider**: `add_collision_exception_with(self)` avoids hitting the player itself; any other body at the origin explodes on the first frame (original).
- **Bullet expires and collides in the same frame**: `time_alive < 0` marks `hit` and explodes; the `move_and_collide` of the same frame still runs and may explode again (two `explode` RPCs) — behavior of the original, preserved.
- **`Settings` in isolated run**: `explode` reads `/root/Settings` dynamically. In the planned validation (headless run of `bullet.tscn`/`player.tscn` via `--path`) the autoload **is** loaded (verified), so the access works as in the game. The autoload is only missing in a `SceneTree` harness (`-s`) outside the repo — the reviewer's scenario, not the implementer's; in that case the error is the same as the original's and does not count.
- **Door: body that leaves and comes back**: only the first `Player` opens; there is no closing (original).
- **Door: upstream error eliminated**: `Node not found: "DoorModel/AnimationPlayer"` ceases to exist — it is **not** in the `CLAUDE.md` catalog (which lists only the 3 import errors), therefore the catalog does not change.
- **Respawn**: below −40 the player is teleported, but `velocity` is not zeroed (original); the `ColorRect` fade (Milestone A) already covers the fall.

## Requirements *(mandatory)*

### Functional Requirements

**Behavior — player (US1)**

- **FR-001**: The player class MUST be named `Player` (mandatory: `red_robot.gd:131,275,281` and `door.gd:10` do `is Player`) and have base `CharacterBody3D`.
- **FR-002**: `player_id` (integer, default 1) MUST be exported, replicated on spawn, and its setter MUST store the value and call `set_multiplayer_authority(value)` on the child `InputSynchronizer` — working when called before the node enters the tree (`level.gd:119`).
- **FR-003**: `current_animation` MUST be exported as enumeration `Animations {JUMP_UP=0, JUMP_DOWN=1, STRAFE=2, WALK=3}`, default WALK, replicated per frame; `motion` (2D vector) MUST remain accessible by name for replication (`player.tscn:26`).
- **FR-004**: On entering the scene, MUST capture `orientation` = global transform of `PlayerModel` with zeroed origin and the initial position; if not server, MUST turn off per-frame processing.
- **FR-005**: On each physics frame, on the server MUST execute `apply_input(delta)`; on the other peers MUST only animate `current_animation`.
- **FR-006**: `animate(anim)` MUST store `current_animation` and configure the `AnimationTree` through the dynamic properties `parameters/state/transition_request` ("jump_up" | "jump_down" | "strafe" | "walk"), `parameters/aim/add_amount` (`get_aim_rotation()` in STRAFE, 0 in WALK), `parameters/strafe/blend_position` (`(motion.x, −motion.y)`) and `parameters/walk/blend_position` (`(|motion|, 0)`).
- **FR-007**: `apply_input(delta)` MUST reproduce, in order, the logic of `player.gd:86-176`: interpolation of `motion` (10×delta); X/Z axes of the camera basis flattened and normalized; `airborne_time += delta`; on the ground, `land` if `airborne_time > 0.5` and zero it; `on_air = airborne_time > 0.1`; jump (`velocity.y = 5`, `airborne_time = 0.1`, `jump` RPC) if not `on_air` and `jumping`; `jumping = false` always; air / aim (slerp toward the camera basis, STRAFE, root motion, shooting with cooldown) / walk (slerp toward `Basis.looking_at(target)` if |target| > 0.001, WALK, root motion) branches; `orientation *= root_motion`; horizontal velocity = `orientation.origin / delta`; gravity; `move_and_slide` with up +Y; zero the origin and orthonormalize `orientation`; global basis of the model = `orientation.basis`; respawn if y < −40.
- **FR-008**: Shooting MUST instantiate `player/bullet/bullet.tscn` as a child of the player's parent (`add_child` with readable name), position it at the global origin of `ShootFrom`, orient it with `look_at` toward `shoot_target`, add a collision exception with the player and fire the `shoot` RPC.
- **FR-009**: The RPCs `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount)` MUST exist with these names, mode `authority`, `call_local`, transfer `unreliable` (defaults of `@rpc("call_local")`), with the effects of `player.gd:179-211`.
- **FR-010**: `add_camera_shake_trauma` MUST call `add_trauma(amount)` on the camera referenced by `player_input.camera_camera` with **typed** access (`CameraNoiseShake`), never via dynamic call.
- **FR-011**: Access to the `InputSynchronizer` MUST be typed (`PlayerInputSynchronizer`): reading of `motion`, `aiming`, `shooting`, `shoot_target`, `jumping`, writing of `jumping`, and calls to `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis`.

**Behavior — bullet (US2)**

- **FR-012**: The bullet MUST have base `CharacterBody3D`, velocity 20, initial `time_alive` 5 s, initial `hit` false, and references to `AnimationPlayer`, `CollisionShape3D`, `OmniLight3D`.
- **FR-013**: On entering the scene on a non-server peer, MUST turn off physics processing and disable the collision.
- **FR-014**: On each physics frame MUST reproduce `bullet.gd:20-35`: return if `hit`; decrement `time_alive` and explode (marking `hit`) if negative; move `−delta × 20 × basis.z` with `move_and_collide`; on collision, call `hit` via RPC on the collider **if it has that method** (`has_method` — duck typing of the original, preserved), disable collision, explode, mark `hit`.
- **FR-015**: The `explode` RPC (`authority`, `call_local`, `unreliable`) MUST play the "explode" animation and enable `shadow_enabled` on the light when `Settings.config_file` has `rendering/shadow_mapping` true.
- **FR-016**: Access to `Settings` MUST follow the Principle II exception: obtain the autoload dynamically at `/root/Settings`, read `config_file` and, from there, use the typed `ConfigFile` API. v2 backlog item 1 already covers this — do not duplicate.
- **FR-017**: `destroy()` MUST exist with that name (method track of the "explode" animation, `bullet.tscn:104`), return if not server, otherwise remove the bullet from the scene.
- **FR-018**: The replication of `global_transform` (`bullet.tscn:12`) MUST remain valid with no edits.

**Behavior — door (US3)**

- **FR-019**: The door MUST have base `Area3D`, initial `open` false, and the handler `_on_door_body_entered(body)` with that name (connection in `door.tscn:37`).
- **FR-020**: When a body enters, if not `open` and the body is a `Player`, MUST play "doorsimple_opening" on the model's `AnimationPlayer` and mark `open = true`; otherwise it does nothing.

**Port cycle — common to the three (Principle II)**

- **FR-021**: Each script MUST become exactly one class registered by the native extension, with the same base (`CharacterBody3D`, `CharacterBody3D`, `Area3D`).
- **FR-022**: The binding MUST be by `type` swap in the `.tscn` with removal of `script` and of the orphan `ext_resource`; no bridge `.gd`.
- **FR-023**: `.gd` and `.gd.uid` MUST be deleted in the same commit as the port.
- **FR-024**: Names of exposed methods, RPCs and exported/replicated properties MUST be identical to GDScript, verified against `player.tscn` (`ServerSynchronizer`: `transform`, `player_id`, `PlayerModel:transform`, `motion`, `current_animation`), `bullet.tscn` (`global_transform`; method track `destroy`) and `door.tscn` (`_on_door_body_entered` connection).
- **FR-025**: The ported code MUST NOT call custom GDScript API. The only allowed dynamic calls are the ones the original already makes dynamically (`has_method("hit")` + `rpc("hit")` in the bullet; dynamic properties of the `AnimationTree`, which are base API) and the `Settings` exception.
- **FR-026**: Each code change MUST be followed by a debug build with no new warnings.
- **FR-027**: Each port MUST be validated headless: import with the extension loading + run of the affected scene(s) with no new errors beyond the baseline (the 3 from `CLAUDE.md`). For the door, the baseline **excludes** the original's `Node not found` — it must disappear.
- **FR-028**: One commit per script, message with the ported script and the changed scene; the delivery order MUST be 1 → 2 → 3 and the game MUST remain playable after each commit.
- **FR-029**: The 7 scripts outside the milestone MUST remain byte-for-byte intact; noticed improvements MUST go to `docs/v2-backlog.md` in the same commit.

**Conservative upstream bug fix — door (Principle I, v1.3.0)**

- **FR-030**: The defect: `door.gd:6` references `DoorModel/AnimationPlayer`, but the node in `door.tscn:13` is named `DoorModel2`; result: `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` and the door never opens. Classification: **bug** (unambiguous intent contradicted by the result).
- **FR-031**: The fix MUST be minimal: reference `DoorModel2/AnimationPlayer`. It is FORBIDDEN to rename the scene node, change the `open` logic, or touch `door.tscn` beyond the `type` swap and script removal.
- **FR-032**: The fix MUST be isolated and identifiable in the code, with comment `// upstream bug fix: ...` at the exact spot of the reference.
- **FR-033**: The commit message of the door port MUST mention the fix.
- **FR-034**: `docs/upstream-bugs.md` MUST be created in this milestone (header + first entry: defect, script/scene, fix applied, commit).
- **FR-035**: No other fix MUST be applied in this milestone; any other noticed defect or improvement goes to `docs/v2-backlog.md` (or, if it is an objective bug, is declared in a future spec).

### Key Entities

- **Player (contract consumed by GDScript)**: class name `Player`; properties `player_id` (int, setter with side effect), `current_animation` (enum), `motion` (2D vector, replicated); RPCs `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount)`; internal method `animate(anim)` (not consumed by any external script — called only from within the class itself and from the `jump`/`land` RPCs; stays private, like the internal methods of Milestone A); consumers `red_robot.gd`, `level.gd`, `bullet` (via `has_method`), `door`.
- **Player internal state**: `airborne_time` (initial 100), `orientation` and `root_motion` (transforms), `initial_position`; scene references `InputSynchronizer`, `AnimationTree`, `PlayerModel`, `ShootFrom` (+ `ShootParticle`, `MuzzleFlash`), `Crosshair`, `FireCooldown`, `SoundEffects/Jump|Land|Shoot`.
- **Bullet**: `time_alive`, `hit`; RPC `explode`; method `destroy` (method track); replication `global_transform`; dynamic dependency on `Settings.config_file`.
- **Door**: `open`; handler `_on_door_body_entered`; corrected reference `DoorModel2/AnimationPlayer`.
- **Upstream bug register** (`docs/upstream-bugs.md`): entry per fix — defect, script/scene, fix applied, commit.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the milestone the project contains exactly **7** `.gd` files (and 7 `.uid`); the 7 are byte-for-byte identical to the state before the milestone.
- **SC-002**: The game opens from the menu, enters the level and — move, jump (with sound), land (with sound), aim, shoot with a visible bullet that explodes and hits robots, camera shake, respawn when falling below −40 — is indistinguishable from the original in `../oxide_godot_origins/` in a side-by-side comparison done by the user.
- **SC-003**: For each of the 3 ports, the headless validation (import + affected scenes) reports zero new errors beyond the 3 cataloged; for the door, the original's `ERROR: Node not found` **no longer** appears.
- **SC-004**: After each of the 3 commits the game is playable end to end.
- **SC-005**: The milestone history has exactly 3 `Port …` commits (one per script) and none touches the 7 out-of-scope scripts; the door commit mentions the bug fix.
- **SC-006**: Debug build with no new warning in all commits.
- **SC-007**: No ported line introduces abstraction, refactoring or optimization; the only behavior difference from the original is the door opening, and it is isolated by a comment, declared here, in the commit and in `docs/upstream-bugs.md` (4/4 requirements of the clause).
- **SC-008**: `red_robot.gd`, `level.gd` and the bullet keep finding `Player`, `player_id`, `name`, `hit` and `add_camera_shake_trauma` by the original names — no warning of non-existent type/method/property in the headless logs nor in the editor.
- **SC-009**: `door.tscn` instantiates with no error and, in an isolated test, opens exactly once for a `Player`.

## Assumptions

- Phase v1 (constitution v1.3.0); Principle I with the conservative fix clause used a single time (door).
- Visual validation (SC-002, SC-009) by the user; automated validation exclusively headless. The reviewer may run a parity harness outside the repository; none of that goes into the code.
- Single-player validation: the local peer is **server and authority**. The "client" branches (FR-004, FR-005, FR-013, FR-017) are verified by code reading and headless; real multiplayer with two peers is out of scope.
- Out of scope: `part.gd`, `red_robot.gd`, `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`; typed access to `Settings` (v2 backlog item 1); any other fix or improvement.
- `door.tscn` is an orphan asset: the fix changes nothing visible in the game; its functional validation is in an isolated, uncommitted test scene.
- The door's `Node not found` error is not in the `CLAUDE.md` catalog; the Principle II rule about updating the catalog in the same commit is satisfied with no change (nothing to remove) — record this finding in the door commit message.
- Quirks of the original deliberately preserved: initial `airborne_time = 100`; `player_id` setter outside the tree; `jumping` zeroed by the player every frame; possible double `explode` when the bullet expires and collides in the same frame; `velocity` not zeroed on respawn. Each one is a v2 backlog candidate, not a fix.
- `BulletCache` in `player.tscn` now instantiates the bullet's native class; since it is not server-dependent (it is only shader/resource pre-warming), there is no observable effect.
