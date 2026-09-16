# Feature Specification: Milestone A — leaves and player input (v1 raw port)

**Feature Branch**: `001-v1-leaves-and-input`

**Created**: 2026-09-15

**Status**: Draft

**Phase**: v1 — Raw Port (Principle I of the constitution). No abstraction, refactoring or optimization is allowed in this feature; improvements noticed go to `docs/v2-backlog.md`.

**Input**: User description: "Milestone A of the port — port to Rust the 5 leaf scripts and the player input of the Godot TPS Demo, keeping the game playable and with behavior identical to the original at each ported script. Input: docs/port-order.md, items 1 to 5 of the migration order."

## Context

The project is the migration of the Godot TPS Demo (15 scripts) to native code, script by script, keeping the game playable at the end of each step. `docs/port-order.md` defines the bottom-up order; this milestone covers items 1 to 5 — the scripts that depend on no other and that can be swapped individually while the 10 remaining ones stay in the original. Each of the five is delivered as an independent step, verifiable on its own, with the game playable before and after.

The five scripts, in delivery order:

| # | Original script | Affected node/scene | Lines |
|---|---|---|---|
| 1 | `level/debug.gd` | `Debug` (Label) in `level/level.tscn` | 15 |
| 2 | `enemies/red_robot/parts/part_disappear_effect/part_disappear.gd` | root of `part_disappear.tscn` (CPUParticles3D) | 9 |
| 3 | `enemies/red_robot/laser/impact_effect/blast.gd` | root of `impact_effect.tscn` (Node3D) | 15 |
| 4 | `player/camera_noise_shake_effect.gd` | `Camera3D` in `player/player.tscn` | 61 |
| 5 | `player/player_input.gd` (`class_name PlayerInputSynchronizer`) | `InputSynchronizer` (MultiplayerSynchronizer) in `player/player.tscn` | 142 |

Behavior reference: the untouched original project in `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Debug overlay (F3) ported (Priority: P1)

The player is inside the level and presses F3: a text panel appears (or disappears, if it was already visible) showing FPS, VSync state, static memory in MiB, whether the session is online and — only when online — the multiplayer ID. The values change every frame. All exactly as in the original, but without the original script in the project.

**Why this priority**: It is the smallest script, with no dependencies and no state, and introduces the basic mechanics that all the other steps reuse (per-frame processing, input reading, node type swap in an existing scene, script removal). It serves as proof of the complete port cycle with minimal risk.

**Independent Test**: Open the game, enter the level, press F3 repeatedly and compare the panel with the original's side by side. Headless validation: project import confirming the extension loaded and run of `level/level.tscn` with no new errors.

**Acceptance Scenarios**:

1. **Given** the player is in the level with the overlay in the scene's initial state, **When** F3 is pressed, **Then** the overlay's visibility inverts; pressing again, it returns to the previous state.
2. **Given** the overlay is visible, **When** a frame passes, **Then** the text shows, on separate lines: `FPS: <n>`, `VSync: Enabled|Disabled`, `Memory: <x.xx> MiB` (two decimal places) and `Online: Yes|No`.
3. **Given** the session is offline (single-player), **When** the overlay is displayed, **Then** it shows `Online: No` and does not show the multiplayer ID line.
4. **Given** the session is online, **When** the overlay is displayed, **Then** it shows `Online: Yes` followed by `Multiplayer ID: <id>`.
5. **Given** the overlay is hidden, **When** frames pass, **Then** the text keeps being updated (as in the original), so that when it reappears it already shows current values.

---

### User Story 2 - Part disappear effect ported (Priority: P2)

When a robot is destroyed, its parts fall and, when they disappear, each one fires a particle effect: an immediate burst of "mini blasts", followed by a puff of smoke that starts 0.2 s later, and the effect removes itself from the scene after 2× the particles' own lifetime.

**Why this priority**: Nine lines, no dependencies, and introduces the first time-based behavior (chained waits) and self-removal from the scene — a capability needed for steps 3 and 4.

**Independent Test**: Destroy a robot in the level and watch each part disappear with the puff, comparing with the original. Headless validation: run of `part_disappear.tscn` with no new errors.

**Acceptance Scenarios**:

1. **Given** the effect is instantiated and enters the scene, **When** the first frame runs, **Then** the child `MiniBlasts` starts emitting immediately and the main emitter does not emit yet.
2. **Given** the effect has been in the scene for 0.2 s, **When** that instant is reached, **Then** the main emitter starts emitting.
3. **Given** the main emitter has `lifetime` L, **When** 0.2 s + 2×L elapse since entering the scene, **Then** the effect node is removed from the scene.
4. **Given** the effect is in progress, **When** observed next to the original under the same conditions, **Then** the timings and appearance are indistinguishable.

---

### User Story 3 - Laser impact ported (Priority: P3)

When the robot's laser hits something, an animated impact effect appears: every frame, the effect's "light rays" orient toward the active camera (if it still exists), and the effect removes itself from the scene as soon as its animation ends.

**Why this priority**: Fifteen lines, no dependencies. Introduces the response to a signal (end of animation), references to child nodes and querying the active camera — capabilities used by steps 4 and 5.

**Independent Test**: Let the robot shoot at the player or at a wall and watch the impact oriented toward the camera and disappearing at the end of the animation, comparing with the original. Headless validation: run of `impact_effect.tscn` with no new errors.

**Acceptance Scenarios**:

1. **Given** there is an active camera when the effect enters the scene, **When** each frame runs, **Then** the child `LightRays` stays oriented toward the camera's global position.
2. **Given** the camera captured on entry has been destroyed, **When** subsequent frames run, **Then** the effect continues without orienting the rays and without producing errors.
3. **Given** the effect is in the scene, **When** the `AnimationPlayer`'s animation ends, **Then** the effect node is removed from the scene.
4. **Given** there is no active camera at the moment the effect enters the scene, **When** the effect runs, **Then** it animates and disappears normally, without orienting the rays and without errors.

---

### User Story 4 - Camera shake ported (Priority: P4)

When shooting, when hit by another player's bullet or when hit by the robot's laser, the player's camera shakes: a "trauma" accumulates (capped at 1.2), decays at 1.5 per second, and while there is trauma the camera receives a yaw/pitch/roll rotation from noise proportional to the square of the trauma, added to its initial rotation. The rest of the game (still in the original) keeps calling `add_trauma(...)` with 0.35 when shooting, 0.75 when hit (RPC `hit`, triggered by `bullet.gd` — only reachable in multiplayer, since the player's own bullets have a collision exception with them) and 13.0 when the robot's laser hits the player, without any change.

**Why this priority**: First script whose API is called by code that remains in the original (`player.gd`, `red_robot.gd`). It proves that the public interface (method name) survives the node type swap. It also introduces persistent internal state and noise generation.

**Independent Test**: Shoot and get hit by the robot's laser in the level (the 0.75 level is verified by code reading — requires multiplayer); the camera must shake with the same intensity and duration as in the original and return exactly to the initial rotation at the end. Headless validation: run of `player/player.tscn` with no new errors.

**Acceptance Scenarios**:

1. **Given** trauma = 0, **When** `add_trauma(0.35)` is called, **Then** trauma = 0.35 and the camera starts shaking on the next frame.
2. **Given** trauma = 1.0, **When** `add_trauma(13.0)` is called, **Then** trauma = 1.2 (cap), no more.
3. **Given** trauma > 0, **When** a frame of duration `delta` runs, **Then** trauma decreases by 1.5×`delta` (without going negative) and the camera rotation = initial rotation + (pitch, yaw, roll) where each component = limit × trauma² × noise in [-1, 1], with limits 0.05 (yaw), 0.05 (pitch) and 0.1 (roll).
4. **Given** trauma has just reached 0 in a frame, **When** that frame ends, **Then** the camera rotation is exactly the initial rotation (noise × 0) and on the following frames the camera is no longer changed.
5. **Given** the script `player.gd` and `red_robot.gd` remain in the original, **When** the player shoots, is hit by another player's bullet (multiplayer) or is hit by the robot's laser, **Then** the existing calls to `add_camera_shake_trauma`/`add_trauma` work without any edit to those scripts.
6. **Given** two runs of the game, **When** the shake happens, **Then** the noise pattern may differ between runs (random seed, as in the original), but the intensity and duration are the same.

---

### User Story 5 - Player input synchronizer ported (Priority: P5)

All player input — move, look (analog stick and mouse), aim (toggle by short tap or hold), jump, shoot with raycast target — and the fade to black when falling off the map are now produced by the ported node. `player.gd`, which remains in the original, reads the same properties (`aiming`, `shoot_target`, `motion`, `shooting`, `jumping`) and calls the same methods (`get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`) without any change. The first four properties keep being replicated by the synchronization configuration already existing in the scene.

**Why this priority**: It is the largest and richest of the five scripts (RPC, replicated properties, node references filled in by the scene, raycast, continuous and event-based input) and depends on all the capabilities proven in the previous steps. At the same time it is the most valuable: it is half of the gameplay experience, and it is a direct prerequisite of Milestone B (`player.gd`).

**Independent Test**: Enter the level in single-player and exercise each input, comparing with the original: camera turn speed with analog stick and mouse (and the reductions while aiming), pitch limit, aim toggle and hold with the "shoot"/"far" animations, jump, shot hitting the point under the crosshair, falling through the hole in the map with fade to black and return with fade-out. Headless validation: run of `player/player.tscn` with no new errors.

**Acceptance Scenarios**:

1. **Given** the node is the multiplayer authority (single-player case), **When** it enters the scene, **Then** its camera becomes the active camera and the mouse is captured.
2. **Given** the node is NOT the multiplayer authority, **When** it enters the scene, **Then** it stops processing frames and input, and the fade rectangle is hidden.
3. **Given** the player holds `move_right`, **When** a frame runs, **Then** `motion` = (1, 0); with `move_forward`, `motion` = (0, -1); combinations and analog intensities produce the corresponding vector (right−left, back−forward).
4. **Given** the player deflects the look stick with intensity 1 for 1 s without aiming, **When** the frames run, **Then** the camera turns 3.0 rad in yaw; while aiming, 1.5 rad.
5. **Given** the mouse moves N pixels, **When** the event is received, **Then** the camera turns 0.001×N rad without aiming and 0.00075×N rad while aiming.
6. **Given** the camera is looking up or down, **When** the player keeps turning vertically, **Then** the pitch is limited to the range [-89.9°, 70°].
7. **Given** the player is not aiming, **When** `aim` is pressed and released in less than 0.4 s, **Then** aim stays on (toggle) and the "shoot" camera animation plays; when `aim` is pressed again, aim turns off and "far" plays.
8. **Given** the player is not aiming, **When** `aim` is held for more than 0.4 s and released, **Then** aim stays on while holding and turns off on release ("shoot" when turning on, "far" when turning off).
9. **Given** the player presses `jump`, **When** the frame runs, **Then** a local RPC `jump` is fired and `jumping` becomes `true` (the original `player.gd` consumes and resets it, without change).
10. **Given** the player holds `shoot`, **When** the frame runs, **Then** `shooting` = true and `shoot_target` = collision point of the ray cast from the center of the crosshair (range 1000, layer mask 0b11, no effective exclusion — quirk of the original, see FR-017); if nothing is hit, `shoot_target` = origin + direction × 1000.
11. **Given** the player falls and their height goes below −17, **When** the frame runs, **Then** the opacity of the black rectangle = min((−17 − y)/15, 1) — fully black at y ≤ −32.
12. **Given** the player has been teleported back (y ≥ −17) with the rectangle still opaque, **When** frames run, **Then** the opacity is multiplied by (1 − 4×`delta`) every frame until it gradually disappears.
13. **Given** `player.gd` remains in the original, **When** it reads `aiming`, `shoot_target`, `motion`, `shooting`, `jumping` and calls `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`, **Then** everything works without any edit to `player.gd`.
14. **Given** the scene `player.tscn` fills in `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect` via `node_paths`, **When** the scene is loaded, **Then** all six references are filled in on the ported node (same names and compatible types).
15. **Given** `get_aim_rotation()` is called, **When** the camera pitch is ≥ 0, **Then** it returns −pitch/70°; when < 0, it returns pitch/(−89.9°) — that is, a value in [−1, 1] normalized by the limits.

---

### Edge Cases

- **Hidden overlay**: the overlay text keeps being recomputed every frame even while invisible (behavior of the original; do not optimize).
- **F3 outside the level**: the `toggle_debug` action only has an effect when the `Debug` node exists in the current scene (the menu has no overlay).
- **Part effect removed before the timers**: if the scene is unloaded before 0.2 s or before 2×lifetime, the effect must not produce new errors beyond those the original would produce.
- **Camera destroyed during the impact** (e.g. player switched scene): the rays stop orienting without errors.
- **Trauma accumulated above the cap**: any sum above 1.2 is truncated to 1.2.
- **Camera initial rotation**: it is captured when the camera enters the scene; if other animations move the camera afterwards, the shake keeps adding to the captured initial rotation (quirk of the original, preserved).
- **Non-authority peer**: no input is processed, no mouse event is handled, and the fade remains hidden.
- **Aim: pressing `aim` while already toggled**: turns the toggle off (the tap that turns it off does not turn it back on).
- **Aim: tap of exactly 0.4 s**: counts as a short tap (≤ 0.4 s turns the toggle on).
- **Raycast without collision**: the target becomes the point 1000 units away in the look direction.
- **Raycast hitting the player themselves**: the original's exclusion list resolves to an invalid RID (no effective exclusion), and the player's body is in mask 0b11; if the ray hits the player's own body, `shoot_target` is that point — behavior of the original, preserved (do not fix).
- **Pitch at the limits**: continuing to turn beyond −89.9° or 70° does not change the pitch.
- **Fade without falling**: with y ≥ −17 and opacity already 0, the multiplication by (1 − 4×delta) keeps 0.

## Requirements *(mandatory)*

### Functional Requirements

**Behavior — debug overlay (US1)**

- **FR-001**: The debug overlay MUST toggle visibility when the `toggle_debug` action (F3) is pressed, once per press.
- **FR-002**: The overlay MUST update, every frame, a text with FPS, VSync (`Enabled`/`Disabled`), static memory in MiB with two decimal places and online state (`Yes`/`No`), appending the multiplayer ID only when online. "Online" means the current multiplayer peer is not the default offline peer.

**Behavior — part disappear effect (US2)**

- **FR-003**: On entering the scene, the effect MUST immediately turn on the emission of the child `MiniBlasts`.
- **FR-004**: The effect MUST turn on its own emission 0.2 s after entering the scene.
- **FR-005**: The effect MUST remove itself from the scene 2× its own `lifetime` after turning on its own emission.

**Behavior — laser impact (US3)**

- **FR-006**: On entering the scene, the effect MUST capture the active camera of that moment and, every frame, orient the child `LightRays` toward that camera's global position while it exists.
- **FR-007**: The effect MUST remove itself from the scene when the child `AnimationPlayer`'s animation ends.

**Behavior — camera shake (US4)**

- **FR-008**: The camera MUST expose the method `add_trauma(amount)`, which accumulates trauma capped at 1.2.
- **FR-009**: While trauma > 0, every frame the camera MUST reduce the trauma by 1.5 × delta (without going negative) and then apply rotation = initial rotation + (pitch, yaw, roll), with pitch = 0.05 × trauma² × noise, yaw = 0.05 × trauma² × noise, roll = 0.1 × trauma² × noise, each noise in [−1, 1] obtained from a 1D noise generator with distinct seeds (seed, seed+1, seed+2) and a time position accumulated at 5000 × delta per frame.
- **FR-010**: The noise seed MUST be random per instance, and the initial rotation MUST be captured when the camera enters the scene.

**Behavior — input synchronizer (US5)**

- **FR-011**: On entering the scene, if the node is the multiplayer authority, it MUST make `camera_camera` the active camera and capture the mouse; otherwise it MUST turn off its per-frame and input processing and hide `color_rect`.
- **FR-012**: Every frame it MUST compute `motion` = (strength(`move_right`) − strength(`move_left`), strength(`move_back`) − strength(`move_forward`)).
- **FR-013**: Every frame it MUST turn the camera by the look stick (`view_right`−`view_left`, `view_up`−`view_down`) at 3.0 rad/s, halved while aiming; and, per mouse motion event, at 0.001 rad/pixel, reduced to 0.75× while aiming.
- **FR-014**: Turning the camera MUST apply yaw on `camera_base` (rotation in Y by −move.x, followed by re-orthonormalization) and pitch on `camera_rot` (X rotation plus move.y and limited to [−89.9°, 70°]).
- **FR-015**: Aiming MUST follow the toggle/hold logic: releasing `aim` after ≤ 0.4 s pressed turns the toggle on; pressing `aim` turns the toggle off; aim is active if the toggle is on or `aim` is pressed; the pressed-time counter accumulates while aim is active and resets to zero when it is not. On state change, it MUST play the "shoot" (turned on) or "far" (turned off) animation on `camera_animation`.
- **FR-016**: On pressing `jump`, it MUST fire the RPC `jump` (`call_local` mode), whose effect is `jumping = true`.
- **FR-017**: Every frame `shooting` MUST reflect whether `shoot` is pressed; when pressed, `shoot_target` MUST be the collision point of a ray cast from the center of `crosshair` through `camera_camera`, with range 1000, collision mask 0b11 and an exclusion list containing only the RID of the synchronizer itself — which, not being a physics body, resolves to an invalid RID (`RID(0)`, verified in Godot 4.7.2); the practical effect is **no exclusion**, and this behavior MUST be preserved (do NOT exclude the player's body: that would be a bug fix, forbidden in v1 by Principle I). Without collision, `shoot_target` = origin + direction × 1000.
- **FR-018**: Every frame, if the global height of the parent (player) is < −17, the opacity of `color_rect` MUST be min((−17 − y)/15, 1); otherwise it MUST be multiplied by (1 − 4 × delta).
- **FR-019**: The node MUST expose `get_aim_rotation()` (limited pitch, normalized: ≥ 0 → −pitch/70°; < 0 → pitch/(−89.9°)), `get_camera_base_quaternion()` (rotation quaternion of the global basis of `camera_base`) and `get_camera_rotation_basis()` (global basis of `camera_rot`).
- **FR-020**: The node MUST expose under the original names the properties `aiming`, `shoot_target`, `motion`, `shooting`, `jumping` and the references `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect`, so that the replication configuration and the `node_paths` already existing in `player.tscn` remain valid without editing.

**Port cycle — common to the five (Principle II)**

- **FR-021**: Each script MUST become exactly one class registered by the native extension, with the same base class as the original script (Label, CPUParticles3D, Node3D, Camera3D, MultiplayerSynchronizer). Item 5 MUST keep the class name `PlayerInputSynchronizer`.
- **FR-022**: The binding to the scene MUST be done by swapping the node's `type` in the `.tscn` and removing the `script` line and the `ext_resource` of the orphaned `.gd`; no `.gd` may remain attached as a bridge.
- **FR-023**: The corresponding `.gd` and `.gd.uid` MUST be deleted in the same commit in which the node starts using the ported class.
- **FR-024**: Names of exposed methods and of exported/replicated properties MUST be identical to those of the GDScript (checked against the affected `.tscn` before concluding each port).
- **FR-025**: Each code change MUST be followed by a successful debug build with no new warnings, before validating or committing.
- **FR-026**: Each port MUST be validated in headless mode: (a) project import confirming the extension loaded and (b) run of the affected scene with no new errors beyond the three catalogued in `CLAUDE.md` (`Cannon_Charge already exists`, missing `doorsimple_d.png`, `surfaces.is_empty()`).
- **FR-027**: Each port MUST be its own commit whose message states the ported script and the scene(s) with the node type swapped.
- **FR-028**: Every improvement noticed during the port MUST be recorded in `docs/v2-backlog.md` (origin + motivation) in the same commit, and NEVER applied in the code.
- **FR-029**: The 10 scripts outside this milestone (`player.gd`, `bullet.gd`, `door.gd`, `part.gd`, `red_robot.gd`, `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`) MUST remain byte-for-byte intact.
- **FR-030**: Delivery MUST follow the order 1 → 5; at the end of each step the game MUST be playable end to end (menu → level → play).

### Key Entities

- **Input synchronizer contract**: observable state read by the original player — `motion` (2D vector), `aiming` (bool), `shooting` (bool), `shoot_target` (3D point), `jumping` (bool, consumed by the player after the jump). The first four are replicated by the scene's synchronization configuration; `jumping` is propagated by RPC. Methods: `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`, `jump` (RPC).
- **Synchronizer scene references**: `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect` — filled in by the scene, never looked up by path in code.
- **Camera trauma**: scalar in [0, 1.2], fed by `add_trauma(amount)`, decayed at 1.5/s; the shake intensity is trauma².
- **Debug overlay**: multiline text recomputed per frame; visibility toggled by `toggle_debug`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the milestone, the project contains exactly 10 `.gd` files (the 5 of this milestone no longer exist, nor their `.gd.uid`), and the 10 remaining ones are identical to the state before the milestone.
- **SC-002**: The game opens through the menu, enters the level and, in a single-player session, moving, looking (analog stick and mouse), aiming (toggle and hold), jumping, shooting, camera shake, F3, laser impact and the robot's parts disappearing are indistinguishable from the original in `../oxide_godot_origins/` in a side-by-side comparison done by the user.
- **SC-003**: For each of the 5 ports, the headless validation (import + run of the affected scene) reports zero new errors beyond the 3 catalogued in `CLAUDE.md`, and the import shows the extension loading line.
- **SC-004**: After each of the 5 commits, the game is playable end to end — no intermediate state breaks the menu, the level or the player.
- **SC-005**: The milestone's history contains exactly 5 port commits (one per script), each naming the ported script and the changed scene; no commit touches the 10 out-of-scope scripts.
- **SC-006**: The debug build finishes with no new warning relative to the state before the milestone, in all 5 commits.
- **SC-007**: No line of ported code introduces abstraction, refactoring or optimization; every improvement noticed appears as an entry in `docs/v2-backlog.md` in the commit in which it was noticed (conformance review against Principle I).
- **SC-008**: `player.gd`, `red_robot.gd` and the scenes `player.tscn`/`level.tscn` keep finding all methods and properties by their original names — no warning of a nonexistent method/property appears in the headless logs or in the editor.

## Assumptions

- The phase is v1 (raw port); the constraints of Principle I apply in full. Direct translation, "Rust that looks like GDScript" is the expected result.
- Functional validation (SC-002) is visual, done by the user in the editor/game comparing with `../oxide_godot_origins/`; automated validation is exclusively the headless one (SC-003).
- Validation is single-player/offline: the local peer is the multiplayer authority, so the "non-authority" branch of the synchronizer (FR-011) is verified by code reading and by headless, not by a session with two peers. Real multiplayer tests with two peers are out of scope.
- Out of scope: `player.gd`, `bullet.gd`, `door.gd` and all scripts of Milestones B, C and D; typed access to the `Settings` autoload (none of the 5 scripts uses it); any behavior improvement, even trivial.
- The "non-idiomatic" behaviors of the original are preserved on purpose: overlay text recomputed while hidden; camera initial rotation captured a single time; `jumping` exported but not replicated (propagated by RPC); raycast redone every frame while shooting; raycast exclusion list with no practical effect (invalid RID of the synchronizer).
- The order 1 → 5 is the delivery order; each step is independent, but steps 4 and 5 touch the same scene (`player.tscn`) and are therefore separate commits on the same scene.
- The shake noise seed is random per instance (as in the original); the comparison criterion is intensity/duration, not the exact noise pattern.
- The three pre-existing errors of the upstream demo catalogued in `CLAUDE.md` are the baseline; any other error in headless counts as a regression.
- `docs/port-order.md` and `docs/v2-backlog.md` already exist and are the source of the order and the destination of the improvements, respectively.
