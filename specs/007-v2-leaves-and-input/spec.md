# Feature Specification: Milestone V2-B — Leaves and player input

**Feature Branch**: `v2` (work directly, per constitution 1.4.0 Principle II — these are leaf
modules with a single, not-yet-remodeled consumer (`Player`, milestone V2-C); no per-milestone
feature branch. Baseline: `dd763a2` (V2-A complete). Local commits only, never pushed.)

**Created**: 2026-09-16

**Status**: Draft

**Phase**: v2 — Idiomatic Rust (constitution 1.4.0, Principles I, II and III). Both pillars apply,
done together: (1) idiomatic Rust leaning on the type system (an `AimState` enum replacing three
booleans/counters that admit invalid combinations; `DebugStats`/`InputSnapshot`-style typed
snapshots; tuning structs instead of loose module consts) and (2) FFI reduction through the
interface/implementation separation of Principle III (per-frame `Gd<T>`/`Input` calls collapsed
into one snapshot, node references resolved once via `OnEditor`, one `async` pattern replacing
nested `connect_other` chains). This milestone additionally FIXES two patterns every later v2
milestone reuses verbatim: the snapshot → pure step → apply frame shape, and the one way
`await`-style timers/signals are written from here on (`godot::task::spawn` +
`TypedSignal::to_future`/`to_fallible_future`, verified present and unconditional in gdext 0.5.5
— see Assumptions).

- **Backlog items closed by this spec**: #4 (no debug-label text recomputation while hidden),
  #5 (nested `connect_other` chains → the one `async` timer pattern, in both `part_disappear.rs`
  and `blast.rs`), #7 (raycast excludes the player's own body via its real RID instead of the
  ineffective `[RID(0)]`), #8 (`OnEditor<Gd<T>>` for `player_input.rs`'s 6 mandatory node
  references), #9 (`jumping`'s `#[export]` removed — it is replicated nowhere and read only by
  `Player` through the typed field), #26 (VRAM line in the debug overlay, already authorized for
  v2 by the user on 2026-09-16).
- **Backlog items explicitly deferred**: #6 (recapture `start_rotation` when other code moves the
  camera) — deferred because it requires a design decision about who owns the camera's rest
  rotation across effects, which is a cross-cutting concern the constitution reserves for a
  dedicated spec, not something a leaf module's remodel should decide as a side effect.
- **Residual dynamic access left in the touched modules after this milestone**: `player_input.rs`
  `self.base_mut().rpc("jump", &[])` — **permanent**, gdext exposes RPC dispatch only by name
  (the engine-limitation residual case the constitution names explicitly). `player.rs` receives
  a one-line edit here (scenario 1 of US1); its own residual cases (`.rpc` of `jump`/`land`/
  `shoot`/`hit`) belong to the V2-C spec, not this one.

**Input**: User description: "Milestone V2-B — leaves and player input. Remodel the five leaf
modules of `docs/port-order.md` (`debug_label.rs`, `part_disappear.rs`, `blast.rs`,
`camera_noise_shake.rs`, `player_input.rs`) under constitution 1.4.0 Principles I–III: typed
state machines instead of loose booleans, pure snapshot→step→apply logic with unit tests, node
references resolved once, and the ONE async pattern for `await`-style timers that every later v2
milestone (`player.rs`/`bullet.rs` in V2-C, `part.rs`/`red_robot.rs` in V2-D) will reuse
verbatim. `player_input.rs` is the pattern exemplar and must keep every name/signature `player.rs`
(itself remodeled later, in V2-C) calls today, migrating the one call site that changes type in
the same commit. Behavioral parity with `v1` is mandatory, verified headless and with a parity
harness against a `v1` worktree; the backlog items above are the only sanctioned exceptions."

## Context

None of these five modules is consumed by typed Rust code other than `Player` (itself unremodeled
until V2-C), and `Player` already reaches all five **typed** — v1's bottom-up port order put
these leaves before `player.rs`, so there is no dynamic-access exception to lift here (unlike
`Settings` in V2-A). Confirmed by reading `player.rs`'s call sites:

| Module | Lines | Base | What `player.rs` calls today |
|---|---|---|---|
| `player_input.rs` | 234 | `MultiplayerSynchronizer` | reads `.motion`, `.aiming`, `.shooting`, `.jumping` (also **writes** `.jumping = false` at `player.rs:216`), `.shoot_target`, `.camera_camera` (then casts to `CameraNoiseShake`); calls `.get_aim_rotation()`, `.get_camera_rotation_basis()`, `.get_camera_base_quaternion()` |
| `camera_noise_shake.rs` | 83 | `Camera3D` | `.add_trauma(amount)`, called on the `Gd<CameraNoiseShake>` obtained by casting `player_input`'s `camera_camera` |
| `debug_label.rs` | 50 | `Label` | none — instantiated only via `level.tscn`'s `type="DebugLabel"` node, no Rust module references it |
| `part_disappear.rs` | 34 | `CpuParticles3D` | none — instantiated only via its own `.tscn`, no Rust module references it |
| `blast.rs` | 37 | `Node3D` | none — instantiated only via its own `.tscn`, no Rust module references it |

Only **one** call site needs a same-commit edit: `camera_camera` moves from
`Option<Gd<Camera3D>>` to `OnEditor<Gd<Camera3D>>` (backlog #8), so `player.rs:138`'s
`self.player_input.bind().camera_camera.clone().unwrap()` loses its `.unwrap()` (`OnEditor<Gd<T>>`
derefs straight to `Gd<T>`). Every other name/signature `player.rs` uses (`motion`, `aiming`,
`shooting`, `jumping`, `shoot_target`, `get_aim_rotation`, `get_camera_rotation_basis`,
`get_camera_base_quaternion`, `add_trauma`) keeps its exact name, type and arity.

Confirmed by reading `player.tscn`: the `SceneReplicationConfig` for `InputSynchronizer` lists
exactly `shoot_target`, `motion`, `shooting`, `aiming` (lines 39–50) — `jumping` is **not** there
and has no stored value anywhere in the file, confirming backlog #9's premise. The
`node_paths=PackedStringArray("camera_animation", "crosshair", "camera_base", "camera_rot",
"camera_camera", "color_rect")` at `player.tscn:339` is the exact 6-field list backlog #8 names.
No `.tscn` among the five modules' host scenes (`player.tscn`, `level.tscn`,
`impact_effect.tscn`, `part_disappear.tscn`) has a `[connection]` block for any of them.

Baseline gates at the start of this milestone (inherited from V2-A, unchanged): `cargo build` /
`cargo clippy` / `cargo test` all clean, 17 tests. Behavior baseline for parity is branch `v1`
(a separate worktree, built and run independently, as in V2-A).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `player_input.rs` becomes the snapshot→step→apply pattern exemplar (Priority: P1)

Movement, camera look, aim toggle/hold, jump, shoot-target raycasting and the fall-to-black fade
all behave exactly as before, but the frame's engine reads happen once, the aim/camera/fade
decisions are pure and unit-tested, and the six mandatory node references are resolved once
instead of `unwrap()`-checked every frame.

**Why this priority**: every other module in this milestone (and V2-C/D after it) copies this
module's shape; getting the pattern right here is the actual deliverable.

**Independent Test**: `cargo test` passes for the new pure module with no Godot binary; headless
`main.tscn`/`menu.tscn`/`level.tscn` show no new errors; in the running game, movement, aiming
(hold and short-tap-toggle), jumping, shooting and falling off the map all look and feel
identical to `v1`.

**Acceptance Scenarios**:

1. **Given** `player.tscn`'s replication config and node-path exports, **When** `player_input.rs`
   is remodeled, **Then** `aiming: bool`, `shoot_target: Vector3`, `motion: Vector2`,
   `shooting: bool` keep their exact names and types as the replicated projection of the internal
   model (`SceneReplicationConfig` unedited); `jumping: bool` keeps its name/type/read-write
   access from `player.rs` but loses its `#[export]` (backlog #9 — confirmed by the Context
   table that nothing replicates or stores it); the 6 node-path fields
   (`camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect`)
   move from `Option<Gd<T>>` to `OnEditor<Gd<T>>`, same names (backlog #8); `player.rs:138`'s
   `.camera_camera.clone().unwrap()` is edited to `.camera_camera.clone()` in the SAME commit.
2. **Given** the three fields `aiming` / `toggled_aim` / `aiming_timer` (which today admit
   combinations `v1`'s own logic never produces, e.g. `toggled_aim = true` while `aiming_timer`
   is still counting), **When** remodeled, **Then** they are replaced by one internal
   `enum AimState { Idle, Held { seconds: f32 }, Toggled }`, with the replicated `aiming: bool`
   field kept as a pure projection (`true` for `Held`/`Toggled`, `false` for `Idle`) recomputed
   whenever the state transitions.
3. **Given** a scripted sequence of aim-button press/hold/release events and a `dt`, **When** the
   pure `step_aim(state: AimState, just_pressed: bool, pressed: bool, just_released: bool, dt: f32) -> (AimState, Option<CameraCue>)`
   function runs (no `Input`, no `Gd`), **Then** it reproduces `v1`'s exact rule: releasing aim
   before `AIM_HOLD_THRESHOLD` (0.4s) of holding it flips into `Toggled`; a short tap while
   `Toggled` clears it; holding past the threshold and releasing returns to `Idle`; a transition
   into or out of "aiming" yields `Some(CameraCue::Shoot)` / `Some(CameraCue::Far)` (driving the
   `camera_animation` play call), otherwise `None`.
4. **Given** camera-look input (gamepad axis or mouse delta) and the current aim state, **When**
   the pure rotation-speed scaling and `CAMERA_X_ROT_MIN`/`MAX` clamp run, **Then** they reproduce
   `v1` exactly: controller look scaled by `dt × CAMERA_CONTROLLER_ROTATION_SPEED`, halved while
   aiming; mouse look scaled by `CAMERA_MOUSE_ROTATION_SPEED`, `×0.75` while aiming; the resulting
   pitch is clamped to `(-89.9°, 70°)` before being written back to `camera_rot`.
5. **Given** the player's world-Y position, the previous fade alpha and `dt`, **When** the pure
   `alpha_for_height(y: f32, prev_alpha: f32, dt: f32) -> f32` function runs, **Then** it
   reproduces `v1` exactly: below `y = -17`, alpha ramps linearly to `1.0` over the next `15`
   units of fall; at or above `-17`, alpha decays by a factor of `(1 - 4·dt)` per frame (never
   recomputed from the transform when the player isn't falling out of bounds — it decays from
   whatever it already was).
6. **Given** the current `camera_rot` pitch, **When** the pure `aim_rotation(camera_x_rot: f32) -> f64`
   function runs, **Then** it reproduces `v1`'s `get_aim_rotation` math exactly (aim-up branch
   `-x / CAMERA_X_ROT_MAX`, aim-down branch `x / CAMERA_X_ROT_MIN`), and `get_aim_rotation` itself
   becomes a thin wrapper: read `camera_rot`'s current pitch once, call the pure function.
7. **Given** the per-frame work, **When** `process`/`input` run, **Then** all 8 action-strength
   reads (`move_*`×4, `view_*`×4) and the 5 boolean action checks (`aim` just-pressed/pressed/
   just-released, `jump` just-pressed, `shoot` pressed) are read into ONE snapshot at the top of
   the callback; `self.base().get_parent().unwrap().cast::<Node3D>()` (today called twice per
   frame, once for the raycast's space state and once for the fall-height transform) is resolved
   ONCE at `ready` into a `Gd<CharacterBody3D>` handle (`OnReady`; the parent of a
   `MultiplayerSynchronizer` never changes) — reading `get_global_transform()` on that handle
   every frame still yields the current transform, so nothing is frozen. The parent's `get_rid()`
   for the raycast exclusion (scenario 8) is likewise captured once at `ready`.
8. **Given** the shoot raycast's exclude list (`array![Rid::Invalid]`, which excludes nothing
   real, since the `MultiplayerSynchronizer` itself has no physics RID), **When** remodeled,
   **Then** it excludes the parent `CharacterBody3D`'s actual RID instead (backlog #7, closed),
   so the player's own hitbox can no longer self-occlude the shoot-target ray; this is the ONE
   behavior change of this story beyond the backlog items already listed as closed, and it is
   covered by Edge Cases below (the parity harness's raycast scenarios must not place the target
   behind the player's own body, where `v1` and this branch would now legitimately differ).
9. **Given** `#[rpc(authority, call_local, unreliable)] fn jump(&mut self)`, **When** remodeled,
   **Then** its name, attributes and body (`self.jumping = true`) are unchanged, and
   `self.base_mut().rpc("jump", &[])` (dynamic dispatch by name) stays exactly as it is — gdext
   has no typed alternative for RPC dispatch (the permanent residual case, listed in the top
   block).

---

### User Story 2 - `camera_noise_shake.rs`: pure trauma/shake math, node references resolved once (Priority: P2)

Camera shake on taking or dealing damage looks and feels identical to `v1`.

**Why this priority**: small, self-contained, and a second worked example of the pattern (a
`Camera3D`-derived leaf with a tiny public API) before the milestone moves to the two modules
that need the new async pattern.

**Independent Test**: `cargo test` passes for the new pure functions; in the running game,
getting hit or firing produces the same shake magnitude and feel as `v1`.

**Acceptance Scenarios**:

1. **Given** the 6 module constants (`SPEED`, `DECAY_RATE`, `MAX_YAW`, `MAX_PITCH`, `MAX_ROLL`,
   `MAX_TRAUMA`), **When** remodeled, **Then** they become a `CameraShakeTuning` struct with a
   `Default` matching `v1`'s literals exactly (constitution Principle III's tuning-struct rule).
2. **Given** `trauma`, `dt`, **When** the pure `decay(trauma: f32, dt: f32, tuning: &CameraShakeTuning) -> f32`
   function runs, **Then** it reproduces `v1`'s `trauma - DECAY_RATE·dt`, floored at `0.0`.
3. **Given** `trauma`, **When** the pure `shake(trauma: f32) -> f32` function runs, **Then** it
   returns `trauma²` exactly as `v1`'s `apply_shake`.
4. **Given** a `shake` value and 3 noise samples (one per axis), **When** the pure
   `offsets(shake: f32, samples: [f32; 3], tuning: &CameraShakeTuning) -> Vector3` function runs,
   **Then** it returns `Vector3(MAX_PITCH·shake·samples[1], MAX_YAW·shake·samples[0],
   MAX_ROLL·shake·samples[2])` where `samples[0]` is the noise at `seed`, `samples[1]` at
   `seed+1`, `samples[2]` at `seed+2` — i.e. exactly `v1`'s `apply_shake`: `yaw` uses `seed`,
   `pitch` uses `seed+1`, `roll` uses `seed+2`, and the rotation is `start_rotation +
   Vector3(pitch, yaw, roll)` (x = pitch, y = yaw, z = roll). The unit test pins this mapping
   with three distinct sample values so a swapped axis fails.
5. **Given** an `add_trauma(amount: f64)` call, **When** remodeled, **Then** it still clamps the
   result to `MAX_TRAUMA` and keeps its exact name and `&mut self, f64` signature (`player.rs`'s
   only call site into this module); it loses `#[func]` (confirmed by grep: no `.tscn` calls it
   by name, `player.rs` already calls it typed) and becomes a plain `pub(crate) fn`.
6. **Given** the per-frame `noise.set_seed()` call repeated 3× (once per axis, since `v1` reuses
   one `FastNoiseLite` object across all three axis queries), **When** remodeled, **Then** three
   separate `FastNoiseLite` instances are created and each seeded exactly ONCE at `ready`
   (`seed`, `seed+1`, `seed+2`, matching `v1`'s `wrapping_add`), removing all 3 per-frame
   `set_seed` calls; this MUST produce bit-identical noise values to `v1` for the same seed and
   position, verified by the parity harness (Edge Cases) — if it does not, the spec's fallback is
   to keep re-seeding one shared instance (still only at the point of use, not as 3 separate
   redundant objects) and record why.
7. **Given** backlog #6 (recapturing `start_rotation` when other code moves the camera), **When**
   this story is scoped, **Then** it is explicitly DEFERRED (see top block) — `camera_noise_shake.rs`
   keeps `v1`'s exact behavior (the comment's caveat and all).

---

### User Story 3 - `debug_label.rs`: pure `compose`, VRAM line, no recompute while hidden (Priority: P2)

The F3 debug overlay shows the same information as `v1` plus one new VRAM line, and stops
rebuilding its text every frame while hidden.

**Why this priority**: closes two backlog items in one small, low-risk module; independent of
US1/US2/US4.

**Independent Test**: `cargo test` passes for the pure `compose` function; in the running game,
pressing the debug toggle shows FPS/VSync/Memory/VRAM/Online/Multiplayer-ID exactly as before
plus the new VRAM line, and toggling it off stops the (otherwise invisible) per-frame text
rebuild — observable only via the pure function's test, not visually.

**Acceptance Scenarios**:

1. **Given** `fps: f64` (Godot's `Engine.get_frames_per_second()` is a float; `v1` prints it
   through `Variant::stringify`, which renders an integer-valued float as `60.0` — `compose` MUST
   reproduce that text, e.g. `format!("{:.1}")`, since the value is always integer-valued; the
   parity harness confirms the bytes), `vsync_enabled: bool`, `ram_bytes: u64`, `vram_bytes: u64`, and
   `multiplayer_id: Option<i64>` (`None` when offline, `Some(id)` when online — replacing `v1`'s
   separate `online: bool` + conditionally-meaningful id, an invalid-state-admitting pair, with
   one type that cannot represent "online but no id" or "offline but has an id"), **When** the
   pure `compose(stats: &DebugStats) -> String` function runs, **Then** it produces text
   byte-identical to `v1` for the FPS/VSync/Memory/Online/Multiplayer-ID lines, plus one new line
   `VRAM: {vram_bytes as MiB, 2 decimals} MiB` immediately below the `Memory:` line (backlog #26).
2. **Given** the overlay is hidden, **When** `process` runs, **Then** it only checks the toggle
   action and (if just toggled) flips visibility — it does NOT read `Engine`/`DisplayServer`/`Os`/
   `RenderingServer`/`multiplayer`, build a `DebugStats`, or call `compose` (backlog #4); the
   snapshot/compose/`set_text` sequence runs only in the frame(s) the overlay is visible.
3. **Given** the overlay is visible, **When** `process` runs, **Then** it reads ONE `DebugStats`
   snapshot from the engine (`Engine::get_frames_per_second`, `DisplayServer::window_get_vsync_mode`,
   `Os::get_static_memory_usage`, `RenderingServer::get_rendering_info(VIDEO_MEM_USED)`,
   `get_multiplayer()`'s peer-is-offline check and unique id), calls `compose`, and sets the
   label's text — once per visible frame, matching `v1`'s cadence exactly except for the new
   VRAM line and the hidden-frame skip.

---

### User Story 4 - `part_disappear.rs` and `blast.rs`: the one `async` timer pattern (Priority: P3)

The red robot's part-disappear puff and the laser impact blast look, sound and time out exactly
as before; the code no longer nests `connect_other` closures to emulate `await`.

**Why this priority**: lowest visual risk (pure timing, no math to get subtly wrong), but it is
where backlog #5 closes and the ONE pattern this milestone fixes gets its two real usages before
V2-C/D lean on it.

**Independent Test**: headless validation shows no new errors; in the running game, a robot's
death parts still puff and vanish on the same schedule, and a laser impact's blast still
disappears when its animation finishes.

**Acceptance Scenarios**:

1. **Given** `part_disappear.rs`'s two sequential waits (`await get_tree().create_timer(0.2).timeout`
   then `await get_tree().create_timer(lifetime * 2.0).timeout`), **When** remodeled, **Then**
   they become one `async` block spawned once from `ready` via `godot::task::spawn`, awaiting
   `create_timer(0.2).signals().timeout().to_future::<()>()` (or the fallible variant — see rule
   below) then `create_timer(lifetime * 2.0).signals().timeout().to_future::<()>()` in sequence,
   with the exact same two delays and the same two engine actions in between (`set_emitting(true)`
   after the first wait, `queue_free()` after the second) — observable timing and outcome
   unchanged.
2. **Given** `blast.rs`'s single wait (`await $AnimationPlayer.animation_finished`), **When**
   remodeled, **Then** it becomes one `async` block spawned from `ready`, awaiting
   `animation_player.signals().animation_finished().to_future::<StringName>()` (or fallible),
   then calling `queue_free()` — same outcome, same trigger.
3. **Given** the async block must operate on `self` after the object may have entered a
   "possibly freed" window (the engine could free it for reasons unrelated to this code path),
   **When** the pattern is written, **Then** the rule stated once here and reused verbatim by
   V2-C/D is: capture `Gd<Self>` (not `&mut self`) at spawn time; after EVERY `await`, check
   `this.is_instance_valid()` and return (drop the rest of the sequence) if the node was freed
   meanwhile — this is what protects against `queue_free` during a wait, since `bind_mut()` on a
   freed `Gd` panics; only then re-`bind_mut()`; and use `to_fallible_future` (returning a
   `Result`) rather than the plain `to_future` wherever the awaited signal's EMITTER is not
   guaranteed to outlive the wait (a `SceneTreeTimer` is kept alive by the tree until it fires,
   so `to_future` is fine there; a child `AnimationPlayer` dies with its parent, so its future
   is fallible), treating `Err` as the same no-op — verified present and unconditional (no
   feature flag) in gdext 0.5.5 (`godot::task::spawn`, `TypedSignal::to_future`/
   `to_fallible_future`; see Assumptions for the exact source locations checked).
4. **Given** `blast.rs`'s `camera: Option<Gd<Camera3D>>` (resolved once at `ready` from
   `get_tree().get_root().get_camera_3d()`, already not re-fetched per frame in the current code),
   **When** remodeled, **Then** it is unchanged — already compliant; the per-frame `look_at` call
   in `process` stays, since it is an unavoidable per-frame engine call with no pure logic to
   extract beyond the existing `is_instance_valid()` guard.
5. **Given** neither module has any domain logic beyond timing and a trivial validity guard,
   **When** Principle III's layout is evaluated, **Then** both stay single-file glue (`x.rs`
   only, no pure submodule) — Principle III requires SEPARATING domain logic into pure Rust, it
   does not require inventing pure logic where a leaf genuinely has none.
6. **Given** the pattern is meant to be reused verbatim by V2-C (`bullet.rs`) and V2-D
   (`part.rs`/`red_robot.rs`), **When** this story closes, **Then** the rule from scenario 3 is
   written down once in `CLAUDE.md`'s "Port conventions (v2)" section, not only in this spec —
   so later milestones cite `CLAUDE.md`, not this spec.

### Edge Cases

- **Self-hit raycast (US1, backlog #7)**: `v1` excludes nothing (`[RID(0)]` is not a real
  body), so when the player's own `CharacterBody3D` collider lies on the camera→crosshair ray the
  ray stops on it and `shoot_target` lands on the player's own body; this branch excludes that
  RID, so the ray passes through and hits whatever is behind. The two branches match everywhere
  else. The parity harness's scripted raycast scenarios MUST NOT place the player's own collider
  in the ray path, and this is recorded as the one excluded case rather than silently assumed
  away.
- **Aim state impossibility**: the `AimState` enum makes "toggled AND still counting the hold
  timer" unrepresentable — `v1`'s three loose fields could (harmlessly, since nothing reads the
  combination) enter that shape; this is not a behavior change since `v1` never observably acts
  on it, only a type-level tightening.
- **FastNoiseLite bit-identical assumption (US2)**: three separately-seeded instances are assumed
  to produce the exact same `get_noise_1d` output as one instance re-seeded between each call,
  because both isolate the seed as the noise function's only varying parameter. This is a
  plan-time-verified assumption (parity harness with a fixed `noise_seed`), not a bare guess left
  unchecked.
- **Debug label while hidden, first toggle**: the very frame the overlay becomes visible again
  must still show fresh stats (not a stale, never-updated string) — `process` must recompute on
  the same frame visibility flips on, not one frame later.
- **Async task lifetime across scene teardown**: if `queue_free()` runs on a `part_disappear`/
  `blast` node WHILE its own spawned task is mid-await (e.g., the level unloads early), the task
  must stop cleanly, not panic: the `is_instance_valid()` check on the captured `Gd<Self>` after
  the `await` returns early (a `SceneTreeTimer` still fires — the tree owns it — so the future
  resolves normally and the guard is what matters), and for the `AnimationPlayer` future the
  fallible variant additionally yields `Err`. This is exactly why scenario 3's rule has both
  parts.
- **Headless async execution**: `godot::task::spawn`'s tasks are driven by the engine's own frame
  processing, which continues under `--headless`; the parity harness must keep the process alive
  long enough (via `await get_tree().process_frame` loops, as used in the V2-A harness) for the
  awaited timers to actually fire before reading results.

## Requirements *(mandatory)*

### Functional Requirements

**`player_input.rs` (US1)**

- **FR-001**: The replicated fields `aiming`, `shoot_target`, `motion`, `shooting` MUST keep
  their exact names and types; `SceneReplicationConfig` in `player.tscn` MUST NOT change.
- **FR-002**: `jumping` MUST keep its name, type (`bool`) and `pub(crate)` read/write access from
  `player.rs`, but MUST lose `#[export]` (backlog #9; confirmed by grep that no `.tscn` stores or
  replicates it).
- **FR-003**: The 6 node-path fields (`camera_animation`, `crosshair`, `camera_base`,
  `camera_rot`, `camera_camera`, `color_rect`) MUST become `OnEditor<Gd<T>>`, same names
  (backlog #8); `player.rs`'s sole external touch point (`camera_camera`) MUST be updated in the
  SAME commit (drop the now-unnecessary `.unwrap()` at `player.rs:138`).
- **FR-004**: `aiming`/`toggled_aim`/`aiming_timer` MUST be replaced by one internal `AimState`
  enum (`Idle`, `Held { seconds }`, `Toggled}` or equivalent), with `aiming: bool` recomputed as
  its pure projection on every transition.
- **FR-005**: A pure `step_aim` function (no `Input`, no `Gd`) MUST reproduce `v1`'s
  `AIM_HOLD_THRESHOLD` (0.4s) hold/toggle rule exactly, unit-tested for: short-tap toggle-on,
  toggle-off by a second short tap, hold-past-threshold-then-release returns to idle, and the
  camera-cue transitions (`Shoot`/`Far`) fired exactly on aim-state changes.
- **FR-006**: Pure functions (no `Input`, no `Gd`) MUST reproduce, unit-tested: the camera
  rotation-speed scaling (controller `×0.5`, mouse `×0.75`, while aiming) and the
  `CAMERA_X_ROT_MIN`/`MAX` pitch clamp; the fall-to-black `alpha_for_height` rule (ramp below
  `y = -17` over 15 units, else decay `×(1 - 4·dt)`); the `aim_rotation` (`get_aim_rotation`)
  up/down branches.
- **FR-007**: Tunable constants (`CAMERA_CONTROLLER_ROTATION_SPEED`, `CAMERA_MOUSE_ROTATION_SPEED`,
  `CAMERA_X_ROT_MIN`/`MAX`, `AIM_HOLD_THRESHOLD`) MUST become a `PlayerInputTuning` struct (or
  associated consts) with a `Default` matching `v1`'s literals.
- **FR-008**: `process`/`input` MUST read ONE input snapshot (8 action strengths + the 5 boolean
  aim/jump/shoot action checks) per frame; the parent `CharacterBody3D` (today
  `get_parent().unwrap().cast::<Node3D>()` twice per frame) and its RID MUST be resolved once at
  `ready` (`OnReady` handle), never looked up per frame.
- **FR-009**: The shoot raycast's exclude list MUST use the parent `CharacterBody3D`'s real RID
  instead of `[RID(0)]` (backlog #7, closed) — the one behavior change of this story beyond the
  backlog items already closed, documented in Edge Cases.
- **FR-010**: `#[rpc(authority, call_local, unreliable)] fn jump` MUST keep its exact name,
  attributes and body; the dynamic `.rpc("jump", &[])` dispatch by name MUST remain (permanent
  residual, no typed alternative exists).

**`camera_noise_shake.rs` (US2)**

- **FR-011**: The 6 module constants MUST become a `CameraShakeTuning` struct with a `Default`
  matching `v1`'s literals.
- **FR-012**: Pure, unit-tested functions MUST reproduce `v1` exactly: `decay(trauma, dt, tuning)`,
  `shake(trauma) = trauma²`, `offsets(shake, samples, tuning) -> Vector3` with `v1`'s exact
  mapping: `x = MAX_PITCH·shake·noise(seed+1)`, `y = MAX_YAW·shake·noise(seed)`,
  `z = MAX_ROLL·shake·noise(seed+2)` (US2 scenario 4).
- **FR-013**: `add_trauma(&mut self, amount: f64)` MUST keep its exact name and signature and its
  `MAX_TRAUMA` clamp; it MUST lose `#[func]` (confirmed unused by name anywhere) and become a
  plain `pub(crate)` method.
- **FR-014**: The 3 per-frame `noise.set_seed()` calls MUST become 3 `FastNoiseLite` instances
  seeded exactly once at `ready` (seed, seed+1, seed+2) — verified bit-identical to `v1` by the
  parity harness; if verification fails, the fallback (re-seed a single shared instance, still
  only where used) MUST be recorded with the reason.
- **FR-015**: Backlog #6 MUST be left deferred, with `camera_noise_shake.rs`'s current behavior
  (including the documented `start_rotation` limitation) otherwise unchanged.

**`debug_label.rs` (US3)**

- **FR-016**: A pure `compose(stats: &DebugStats) -> String` function MUST produce
  byte-identical FPS/VSync/Memory/Online/Multiplayer-ID lines to `v1` (the FPS value is an `f64`
  rendered as Godot renders a float, `60.0`, not `60`), plus one new `VRAM:` line
  immediately below `Memory:` (backlog #26), formatted the same way (MiB, 2 decimals).
- **FR-017**: `DebugStats` MUST represent "online-ness" and the multiplayer id as one
  `multiplayer_id: Option<i64>` field (not a separate bool + conditionally-valid id), ruling out
  the invalid "online without an id" / "offline with an id" combinations at the type level.
- **FR-018**: `process` MUST NOT read any engine state or call `compose`/`set_text` while the
  overlay is hidden (backlog #4); it MUST do so exactly once per frame while visible, on the same
  frame visibility turns on.

**`part_disappear.rs` and `blast.rs` (US4)**

- **FR-019**: Both modules' nested `connect_other` timer/signal chains MUST be replaced by one
  `async` block spawned from `ready` via `godot::task::spawn`, awaiting
  `TypedSignal::to_future`/`to_fallible_future` in sequence, with identical delays and identical
  engine actions between/after waits (backlog #5, closed for both files in the SAME pattern).
- **FR-020**: The async pattern MUST capture `Gd<Self>` (not `&mut self`) at spawn time; after
  each `await` it MUST check `is_instance_valid()` on that handle and stop cleanly if the node was
  freed, and only then re-bind; it MUST use `to_fallible_future` (handling `Err` as the same clean
  no-op) wherever the awaited signal's emitter may be freed before firing.
- **FR-021**: This rule MUST be written once into `CLAUDE.md`'s "Port conventions (v2)" section
  in the commit that closes US4, so V2-C/D cite the operational guide, not this spec.
- **FR-022**: `blast.rs`'s `camera` field and per-frame `look_at` stay as they are (already
  resolved once; no pure logic to extract beyond the existing validity guard).

**Cross-cutting (all five modules)**

- **FR-023**: Each module's `#[godot_api] impl I<Base>`/`impl X` blocks MUST contain only
  lifecycle callbacks and exposed API, delegating to plain `impl`/free functions for any domain
  logic (Principle III); `player_input.rs` and `camera_noise_shake.rs` (and `debug_label.rs`, if
  its pure `compose` is placed in a submodule rather than an inline `mod`) gain a pure
  submodule/`mod` with `#[cfg(test)]`; `part_disappear.rs`/`blast.rs` stay single-file (FR-022 —
  no domain logic to separate).
- **FR-024**: `cargo build`/`cargo clippy`/`cargo test` MUST be clean before every commit; headless
  validation (`CLAUDE.md`'s recipe) MUST show no new errors after every commit.
- **FR-025**: A parity harness run against a `v1` worktree (separate `XDG_DATA_HOME` per tree, as
  in V2-A) MUST cover every deterministic case: a scripted input sequence for the aim state
  machine and fall-to-black fade (driven via `Input.action_press`/`release`, headless); the debug
  label's composed text for a fixed stats snapshot; the shake camera's rotation for a fixed
  `noise_seed` (private on both branches — the harness calls the global `seed(N)` before
  instantiating `player.tscn`, so `randi()` in `init` yields the same value on both) and trauma
  sequence (bounded by the FastNoiseLite bit-identical assumption in Edge Cases); the part-disappear/blast timing (elapsed frames to `queue_free`). Cases NOT covered by
  the harness (because they need real engine noise-generator internals, not scriptable inputs)
  MUST be named explicitly and covered by unit tests instead.
- **FR-026**: One local commit per module (US1 may be two: pure model, then glue), each message
  naming what was remodeled; never pushed.

### Key Entities

- **`AimState`** (`player_input.rs`): `Idle | Held { seconds: f32 } | Toggled` — replaces
  `aiming`/`toggled_aim`/`aiming_timer`; `aiming: bool` is its replicated projection.
- **`CameraCue`** (`player_input.rs`): `Shoot | Far` — the pure aim-transition output driving
  `camera_animation`'s play calls.
- **`PlayerInputTuning`** (`player_input.rs`): rotation speeds, pitch clamp, aim-hold threshold.
- **`CameraShakeTuning`** (`camera_noise_shake.rs`): the 6 shake constants.
- **`DebugStats`** (`debug_label.rs`): `fps: f64`, `vsync_enabled: bool`, `ram_bytes: u64`,
  `vram_bytes: u64`, `multiplayer_id: Option<i64>`.
- **The one async timer pattern** (`part_disappear.rs`, `blast.rs`, and `CLAUDE.md`'s "Port
  conventions (v2)"): `godot::task::spawn` + `Gd<Self>` capture + `is_instance_valid()` guard after every await +
  `to_future`/`to_fallible_future` (fallible when the emitter may die) + re-bind.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build`/`cargo clippy`/`cargo test` clean, with at least 12 new unit tests
  across the pure functions/modules introduced by this milestone (aim state machine, camera
  rotation/scaling, fall-to-black alpha, aim rotation, camera-shake decay/shake/offsets, debug
  label `compose`) — total test count at or above 29 (17 inherited from V2-A + 12).
- **SC-002**: Zero `get_parent().cast()` or `unwrap()` calls remain on any of `player_input.rs`'s
  6 mandatory node references, checked per frame (grep + code review).
- **SC-003**: Zero nested `connect_other` chains remain in `part_disappear.rs`/`blast.rs` (grep
  for `connect_other` inside a `connect_other` closure returns nothing).
- **SC-004**: The debug overlay shows a `VRAM:` line directly below `Memory:` when visible.
- **SC-005**: Headless validation (import + `main.tscn`/`menu.tscn`/`level.tscn`) shows zero new
  errors relative to the documented baseline, after every commit.
- **SC-006**: The parity harness's deterministic cases (scripted input → aim state/fall alpha;
  fixed stats → debug text; fixed seed/trauma → shake rotation, within the noted assumption;
  timer elapsed-frame counts for part-disappear/blast) produce identical dumps between `v1` and
  this branch, with the one documented self-hit-raycast exclusion (backlog #7) as the sole
  excluded case.
- **SC-007**: `docs/v2-backlog.md` items #4, #5, #7, #8, #9, #26 are marked done citing this
  milestone's closing commits; #6 remains open with the deferral reason recorded.

## Assumptions

- Phase v2 (constitution 1.4.0). The two behavior deviations from `v1` beyond the closed backlog
  items are: (a) the shoot raycast's self-exclusion (backlog #7, FR-009) and (b) the debug
  overlay's new VRAM line (backlog #26, FR-016) — both explicitly authorized, both already listed
  in the top block.
- `godot::task::spawn` and `TypedSignal::to_future`/`to_fallible_future` were confirmed present
  and NOT behind any Cargo feature in the installed gdext 0.5.5 sources (`godot-core-0.5.5/src/
  lib.rs:32` `pub mod task;`, unconditional; `task/async_runtime.rs:148` `pub fn spawn`; `task/
  futures.rs:356-385` `to_fallible_future`/`to_future` on both `Signal` and `TypedSignal`) — this
  milestone commits to the `async` pattern rather than a `Timer`-node fallback; if plan-time work
  discovers a robustness gap specific to this crate's usage, that is a plan-level finding to
  surface, not a silent reversion.
- Exact Rust module boundaries (whether `debug_label.rs`'s pure `compose` lives in an inline
  `mod tests`-adjacent block or a separate `debug_label/model.rs` file, and the precise
  `InputSnapshot`/`ApplyPlan`-style struct shapes for `player_input.rs`) are plan-time decisions,
  not fixed by this spec — the constitution requires the separation, not a specific file layout,
  when the pure part is small.
- The parity harness reuses V2-A's `v1`-worktree + `XDG_DATA_HOME`-split pattern; building a
  scripted-input driver (via `Input.parse_input_event` or `action_press`/`release` from a scratch
  scene) is new work for this milestone, not a reuse of V2-A's menu-button-driving harness.
- Out of scope: `player.rs` itself (V2-C, only the one `camera_camera` call-site edit happens
  here); `bullet.rs`'s `Hittable` dispatch (V2-C); `part.rs`/`red_robot.rs`'s state machine
  (V2-D); backlog #6 (deferred, see top block); any change to the five modules' host `.tscn`
  files beyond what a name-preserving Rust remodel requires (none is expected).
