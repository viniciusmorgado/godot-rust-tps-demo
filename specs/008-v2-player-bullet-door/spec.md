# Feature Specification: Milestone V2-C — player, bullet, door

**Feature Branch**: `v2` (work directly, per constitution 1.4.0 Principle II — `player.rs` is the
hub these three modules revolve around; `bullet.rs` and `door.rs` are its immediate consumers.
No per-milestone feature branch. Baseline: `e0fffbf` (V2-B complete). Local commits only, never
pushed.)

**Created**: 2026-09-17

**Status**: Draft

**Phase**: v2 — Idiomatic Rust (constitution 1.4.0, Principles I, II and III). Both pillars apply,
done together: (1) idiomatic Rust leaning on the type system (`BulletState`/`DoorState` enums
replacing loose booleans, a typed `HitTarget` dispatch replacing `has_method("hit")` duck typing,
tuning structs instead of module consts) and (2) FFI reduction through Principle III (six-plus
`player_input.bind()` calls per physics frame collapsed into one `InputFrame` snapshot, per-shot
`get_node_as`/`load::<PackedScene>` resolved once via `OnReady`, `animate()`'s `if`/`else if`
chain replaced by `match` over a computed `AnimPlan`).

- **Backlog items closed by this spec**: #2 (`has_method("hit")` → typed `HitTarget` dispatch),
  #10 (`airborne_time` starts at `0.0`, not `100.0` — no spurious `land` RPC/sound on spawn),
  #11 (velocity zeroed on respawn below `-40`), #12 (the never-read `crosshair` field removed
  from `Player`), #13 (no second `explode` RPC when the bullet's lifetime expires and it
  collides in the same physics tick), #14 (closed in a MODIFIED form — see US3; the literal
  proposal of typing `_on_door_body_entered`'s parameter as `Gd<Player>` is REJECTED as a
  behavior change, not applied).
- **Backlog items explicitly deferred**: none new. #6 (camera `start_rotation` ownership)
  remains deferred from V2-B, untouched here.
- **Residual dynamic access left in the touched modules after this milestone**: `player.rs`
  `self.base_mut().rpc("jump"/"land"/"shoot", &[])` and the `add_camera_shake_trauma` RPC
  attribute's own dispatch path (unchanged, typed call from `red_robot.rs`, RPC name stays a
  wire concern only); `bullet.rs` `self.base_mut().rpc("explode", &[])` and
  `collider.rpc("hit", &[])` — **all permanent**, gdext exposes RPC dispatch only by name (the
  engine-limitation residual case the constitution names explicitly). `door.rs` has none.
- **#28 stays open**: the first-laser-shot hitch (recorded 2026-09-16) is NOT investigated or
  fixed by this milestone. US1 preloads `bullet.tscn` via `OnReady<Gd<PackedScene>>` (closing
  the per-shot `load::<PackedScene>` site named in #28's hypothesis for the PLAYER's bullets
  only — `red_robot.rs`'s `impact_effect.tscn` load is V2-D's concern), and the checkpoint asks
  the user to note, without acting on it, whether the hitch on the player's first shot changed.

**Input**: User description: "Milestone V2-C — player, bullet, door. Apply the V2-B leaf
patterns (`InputFrame` snapshot analogous to `player_input`'s `InputSnapshot`, `OnReady`
preloaded resources, pure step/apply, tuning structs) to `player.rs`, the hub every other
gameplay module reaches through, `bullet.rs` (introducing the typed `Hittable`/`HitTarget`
dispatch V2-D's `EnemyRobot` will also adopt), and `door.rs`. Preserve every name/signature
`level.rs` and `red_robot.rs` call today. Behavioral parity with `v1` is mandatory except the
five closed backlog items above, verified headless and with a parity harness against a `v1`
worktree."

## Context

`Player` is consumed **typed** by three modules already: `level.rs:203-206` instantiates it
(`load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>()`) and calls
`player.bind_mut().set_player_id(id)` (`id: i32`); `door.rs` (in scope this milestone, US3)
`try_cast::<Player>()`s the body that enters its `Area3D`; `red_robot.rs` `try_cast::<Player>()`s
twice (`_on_area_body_entered`/`_on_area_body_exited`) and calls
`player.clone().bind_mut().add_camera_shake_trauma(13.0)` after a hit. `bullet.rs` is the one
remaining **dynamic** consumer: `collider.has_method("hit")` + `collider.rpc("hit", &[])`,
because a bullet's collider is either a `Player` or an `EnemyRobot` (still v1-shaped, V2-D) and
nothing today lets `bullet.rs` express "either of these two typed things" without duck typing.

Confirmed by reading the three files in full (`player.rs` 309 lines, `bullet.rs` 87,
`door.rs` 31) and their consumers:

| Module | Base | What consumers call today |
|---|---|---|
| `player.rs` | `CharacterBody3D` | `level.rs`: `instantiate_as::<Player>()`, `set_player_id(i32)`. `red_robot.rs`: `try_cast::<Player>()` (×3), `add_camera_shake_trauma(f64)` (typed, via `bind_mut()`). `door.rs`: `try_cast::<Player>()`. RPCs `jump`/`land`/`shoot`/`hit` are fired by `player.rs` on itself via `self.base_mut().rpc(...)`, never by another module. |
| `bullet.rs` | `CharacterBody3D` | `player.rs`'s `apply_input` instantiates it (`load::<PackedScene>("bullet.tscn").instantiate_as::<CharacterBody3D>()`, untyped on purpose — the spawn site only needs `CharacterBody3D`-level calls: `set_global_position`, `look_at`, `add_collision_exception_with`); `bullet.tscn`'s own method track calls `destroy` by name (`#[func]`, must stay). |
| `door.rs` | `Area3D` | `door.tscn:35`'s `[connection] body_entered → _on_door_body_entered` (kept, see US3); no Rust module calls into `Door`. |

`player_id`'s replicated surface (`player.tscn`'s `SceneReplicationConfig`,
`sub_resource SceneReplicationConfig_o4rt5`): `.:transform` (mode 1), `.:player_id` (mode 0,
i.e. spawn-only, not continuously replicated — it is set once via `set_player_id` at spawn),
`PlayerModel:transform` (mode 1), `.:motion` (mode 1), `.:current_animation` (mode 1) — all five
properties belong to `Player` itself or its `PlayerModel` child, none to `player_input`. This
confirms `self.motion: Vector2` (the AUTHORITY's lerped value) is what actually replicates to
remote peers — remote peers never read `player_input`'s raw motion.

`bullet.tscn`'s root node is already `type="Bullet"` (v1-shaped Rust class, confirmed at
`bullet.tscn:480`); its `AnimationPlayer`'s `explode` animation has a method-call track invoking
`destroy` by name (`bullet.tscn:103`, `"method": &"destroy"`) — `destroy` MUST keep `#[func]`.

`door.tscn:35`'s connection is `[connection signal="body_entered" from="." to="."
method="_on_door_body_entered"]` — a same-node connection with no parameter type declared in the
`.tscn` (Godot resolves the handler's declared Rust parameter type at connect time); today it is
`Gd<Node3D>`.

Baseline gates at the start of this milestone (inherited from V2-B, unchanged): `cargo build` /
`cargo clippy` / `cargo test` all clean, 41 tests. Behavior baseline for parity is branch `v1`
(a separate worktree, built and run independently, as in V2-A/V2-B).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `player.rs` reads one `InputFrame`, computes one `AnimPlan` (Priority: P1)

Movement, jumping, landing, aiming/strafing, shooting and respawning all look, sound and feel
exactly as before (minus the three closed spawn/respawn/dead-field quirks below), but the frame's
`player_input` reads happen once, the movement/animation decisions are pure and unit-tested, and
the two per-shot resource lookups are resolved once instead of on every shot.

**Why this priority**: `Player` is the hub every other gameplay module reaches through; getting
its shape right is this milestone's actual deliverable, and V2-D's `EnemyRobot` will mirror it.

**Independent Test**: `cargo test` passes for the new pure module with no Godot binary; headless
`main.tscn`/`level.tscn` show no new errors; in the running game, movement, strafing while
aiming, jumping, landing, shooting and falling below the map all look and feel identical to `v1`
except: no `land` sound/RPC on spawn (#10), no residual fall velocity after a respawn teleport
(#11).

**Acceptance Scenarios**:

1. **Given** `player.tscn`'s replication config, **When** `player.rs` is remodeled, **Then**
   `player_id: i32` (with its `#[func] set_player_id(&mut self, value: i32)` setter — kept
   exactly, since replication and `level.rs:206`'s `bind_mut().set_player_id(id)` both need it),
   `motion: Vector2` (`#[var]`) and `current_animation: Animations` (`#[export]`, wire codes
   unchanged) keep their exact names, types and attributes; a `PeerId` newtype for `player_id`
   was considered and DEFERRED (see Assumptions) since it protects against no concrete
   confusion `player_id` is exposed to in this module's own scope. RPCs `jump`, `land`, `shoot`,
   `hit`, `add_camera_shake_trauma` keep their exact names, `#[rpc(authority, call_local,
   unreliable)]` attributes and bodies' observable effect; `self.base_mut().rpc("jump"/"land"/
   "shoot", &[])` (dispatch by name) stays exactly as today — gdext has no typed alternative
   (the permanent residual case, listed in the top block).
2. **Given** the two `get_node_as::<CpuParticles3D>` calls inside `shoot` (`ShootParticle`,
   `MuzzleFlash`, resolved by string path on every shot) and the `load::<PackedScene>
   ("bullet.tscn")` call inside `apply_input` (resolved on every shot fired), **When**
   remodeled, **Then** all three become fields resolved once: the two particle emitters as
   `OnReady<Gd<CpuParticles3D>>` (`#[init(node = "...")]`, same paths), and the bullet scene as
   `#[init(node = ...)]`-incompatible (it is a resource, not a child node) so it becomes an
   `OnReady<Gd<PackedScene>>` initialized via `OnReady::new(|| load("res://player/bullet/
   bullet.tscn"))` — resolved once at `ready`, instantiated fresh per shot exactly as before
   (only the SCENE RESOURCE lookup is cached, not the instance).
3. **Given** `apply_input`'s six-plus `self.player_input.bind()`/`bind_mut()` calls per physics
   frame (`motion`, `get_camera_rotation_basis()`, `jumping` read, `jumping` write, `aiming`,
   `get_camera_base_quaternion()`, `get_aim_rotation()` inside the `Strafe` branch of
   `animate()`, `shooting`, `shoot_target`), **When** remodeled, **Then** `apply_input` acquires
   `player_input` exactly ONCE per frame (one `bind_mut()` call, since the frame also needs to
   clear `jumping` back to `false`) and builds one `InputFrame { motion, aiming, shooting,
   jumping, shoot_target, camera_rotation_basis, camera_base_quaternion, aim_rotation }` from
   that single guard before dropping it; nothing after that point in the frame touches
   `player_input` again. On the NON-authority (remote) path, `physics_process`'s `else` branch
   (which calls `self.animate(anim, delta)` with the replicated `current_animation`) also
   acquires `player_input` at most once per frame, for the one value `animate` still needs
   externally-sourced (`get_aim_rotation()`, only read when transitioning into `Strafe` — the
   replicated `self.motion` is read locally, not from `player_input`, since it is `Player`'s own
   replicated field per the Context table).
4. **Given** the jump/land/airborne logic (`airborne_time` accumulation, the `> 0.5` land
   threshold, `MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`), **When** the pure
   `airborne_step(airborne_time: f32, dt: f32, is_on_floor: bool, jump_pressed: bool, tuning:
   &PlayerTuning) -> AirborneOutcome` function runs (no `Gd`, no engine call), **Then** it
   reproduces `player.rs:196-214` exactly, in the same order (accumulate first; if on floor,
   check the land threshold THEN reset to zero; recompute `on_air`; only then check the jump
   condition against the POST-reset `on_air`), returning the new `airborne_time`, whether the
   player is airborne this frame, and TWO independent event flags `land: bool` and `jump: bool`
   — they are NOT mutually exclusive in `v1`: landing from a fall (`is_on_floor` with
   `airborne_time > 0.5`) fires `land`, resets `airborne_time` to `0`, which makes `on_air`
   false, and if `jumping` is held that same frame the jump branch ALSO fires — both RPCs in one
   frame. A unit test MUST pin "land + jump on the same frame" so the outcome type cannot be
   narrowed to a single event.
5. **Given** the orientation update (slerp toward the camera-base quaternion while aiming, or
   toward the flattened movement direction while walking, both at
   `ROTATION_INTERPOLATE_SPEED = 10.0`, guarded by `target.length() > 0.001` for the walking
   case), **When** pure functions run (`Basis`/`Quaternion`/`Vector3` only), **Then** they
   reproduce `player.rs:224-234` (aiming) and `player.rs:262-272` (walking) exactly: a shared
   slerp-toward-target-basis step, fed either the camera-base quaternion (aiming) or
   `Basis::looking_at(camera_x·motion.x + camera_z·motion.y)` when that target's length exceeds
   `0.001` (walking; below the threshold, orientation is left unchanged this frame, exactly as
   `v1`'s `if target.length() > 0.001` guard does).
6. **Given** the root-motion integration (`orientation = orientation * root_motion`; horizontal
   velocity from `orientation.origin / delta`; gravity added; origin cleared and the basis
   orthonormalized afterward), **When** a pure function runs on `Transform3D`/`Vector3` inputs
   (root motion's rotation/position are READ from the `AnimationTree` once per frame by glue,
   the integration itself is pure), **Then** it reproduces `player.rs:283-297` exactly,
   including that `self.root_motion` is a field only reassigned in the aiming/walking branches
   (glue's responsibility) — while airborne, integration uses whatever `root_motion` was
   captured on the last non-airborne frame, unchanged from `v1`.
7. **Given** the animation decision (`Animations::{JumpUp,JumpDown,Strafe,Walk}` plus the
   `AnimationTree` parameter values `animate()` sets: `parameters/state/transition_request`,
   `parameters/aim/add_amount`, `parameters/strafe/blend_position`,
   `parameters/walk/blend_position`), **When** remodeled, **Then** the four `if anim ==`
   branches become a `match` over a computed `AnimPlan` enum (`JumpUp`, `JumpDown`, `Strafe {
   aim_rotation: f64, blend_position: Vector2 }`, `Walk { blend_position: Vector2 }`) whose
   payload is exactly `player.rs:153-176`'s values (`Strafe`'s blend position is
   `Vector2::new(motion.x, -motion.y)` — the animation's forward/backward axis is reversed, kept
   verbatim; `Walk`'s is `Vector2::new(motion.length(), 0.0)`); the four `AnimationTree`
   parameter path strings become named constants (no behavior change, only where the literals
   live); `jump`/`land`'s direct `self.animate(Animations::JumpUp/JumpDown, 0.0)` calls apply
   the trivial `AnimPlan::JumpUp`/`JumpDown` variants through the same one apply-step function.
8. **Given** backlog #10 (`airborne_time` starts at `100.0`, so the very first floor contact
   after spawn exceeds the `0.5` s land threshold and fires a spurious `land` RPC + landing
   sound), **When** closed, **Then** `airborne_time` starts at `0.0`; the first physics frame on
   the ground never exceeds the threshold, so no `land` fires on spawn. This is a behavior
   change, excluded from the parity harness's spawn-frame comparison (documented in Edge Cases).
9. **Given** backlog #11 (the respawn teleport at `player.rs:303-307` resets `transform.origin`
   but not velocity, so a long fall's terminal velocity carries into the next frame after
   teleporting back to `initial_position`), **When** closed, **Then** the respawn branch also
   zeroes `velocity` (both components — the vertical one that caused the fall and any residual
   horizontal one). Excluded from the parity harness's post-respawn-frame comparison.
10. **Given** backlog #12 (the `crosshair: OnReady<Gd<TextureRect>>` field, declared, never
    read anywhere in `player.rs`), **When** closed, **Then** the field is removed entirely; no
    behavior change (nothing ever consumed it).

---

### User Story 2 - `bullet.rs`: typed `Hittable` dispatch, no double `explode` (Priority: P2)

A bullet's lifetime, its collision explosion, and what happens when it hits the player or a
robot are all unchanged, except a bullet that expires and collides in the very same physics tick
now explodes once instead of twice.

**Why this priority**: introduces the ONE typed-dispatch pattern (`HitTarget`) that V2-D's
`EnemyRobot` will also use for its own laser-hit logic — worth landing before that milestone, but
independent of, and smaller than, US1.

**Independent Test**: `cargo test` passes for the new pure state machine; in the running game, a
bullet still flies for 5 seconds before self-destructing, still explodes on hitting a wall or a
robot, robots still take damage, and rapid-fire bullets that expire mid-collision no longer
double-restart the explosion animation (observable only as a subtle visual glitch removed, not a
new behavior to look for).

**Acceptance Scenarios**:

1. **Given** `hit: bool` and `time_alive: f32` (an invalid-state-admitting pair: nothing stops
   `time_alive` from continuing to count down after `hit` is already `true`, it is simply
   ignored by the `if self.hit { return; }` guard), **When** remodeled, **Then** they become one
   `enum BulletState { Flying { time_alive: f32 }, Exploded }`, with a pure `step(state:
   BulletState, dt: f32) -> (BulletState, bool)` (the `bool` is "explode now due to expiry")
   reproducing `bullet.rs:43-47` exactly: decrement `time_alive`; if it drops below `0.0`,
   transition to `Exploded` and report "explode now"; a bullet that is ALREADY `Exploded` at
   the start of a frame is a no-op (mirrors the early `if self.hit { return; }` guard — no
   engine calls happen on later frames). On the EXPIRY frame itself, `v1` does NOT return after
   firing `explode`: it still runs `move_and_collide`, and a collision on that same frame still
   RPCs `hit` on the collider and disables the collision shape — v2 keeps all of that; the only
   thing #13 suppresses is the DUPLICATE `explode` RPC (scenario 2).
2. **Given** the collision handling (`move_and_collide`, then `explode` RPC'd unconditionally on
   any collision), **When** remodeled, **Then** a collision AFTER `step()` already produced
   "explode now" (both triggered by the same physics tick — lifetime expired AND a collider was
   hit) does NOT fire a second `explode` RPC (backlog #13, the one behavior change of this
   story): the glue checks the state is still `Flying` (i.e., `step()` did not already explode
   it this frame) before RPC'ing `explode` for a collision.
3. **Given** `collider.has_method("hit")` (duck typing: works today because both `Player` and
   `EnemyRobot` happen to expose a same-named, no-argument `#[rpc] fn hit(&mut self)`), **When**
   remodeled, **Then** a crate-level `HitTarget` (`enum HitTarget { Player(Gd<Player>),
   Robot(Gd<EnemyRobot>) }`, resolved from the collider via two `try_cast`s) replaces the
   `has_method` check: `bullet.rs` only RPCs `hit` on a collider that resolves to a `HitTarget`
   variant. The RPC dispatch itself is UNCHANGED (`.rpc("hit", &[])` by name — gdext has no
   typed RPC dispatch, the permanent residual named in the top block), so the observable effect
   for an actual `Player`/`EnemyRobot` collider is identical to `v1`; what changes is that the
   DECISION of whether to fire it is now a type check, not a string-named method probe. This
   abstraction lives in a new crate-level module (not private to `bullet.rs`) since V2-D's
   `EnemyRobot`'s own laser-hit logic (`red_robot.rs::shoot`, currently a direct
   `player.try_cast::<Player>()`) will reuse it.
4. **Given** `BULLET_VELOCITY = 20.0`, **When** remodeled, **Then** it becomes an associated
   constant or a `BulletTuning` field (`Default` matching the literal) per Principle III.
5. **Given** `explode`'s typed `Settings` read (`self.settings.bind().graphics().shadow_mapping`,
   already typed since V2-A) and `destroy`'s `#[func]` (called by `bullet.tscn`'s own animation
   method track), **When** remodeled, **Then** both stay exactly as they are — no change
   required, confirmed already compliant.

---

### User Story 3 - `door.rs`: typed state, boundary `try_cast` kept as today's shape (Priority: P3)

The door still opens only when the player walks into it, plays the same animation, and ignores
every other body (robots, bullets, props) exactly as before.

**Why this priority**: smallest, self-contained, lowest risk; independent of US1/US2.

**Independent Test**: `cargo test` passes for the pure decision function; in the running game,
walking into the door opens it once (repeated entry does nothing further, matching `v1`'s
one-way `open` flag), and a robot or any non-player body walking through it does not open it and
prints no engine error.

**Acceptance Scenarios**:

1. **Given** `open: bool` (a one-way flag: once `true`, never reset), **When** remodeled,
   **Then** it becomes `enum DoorState { Closed, Open }` (not `Opening` — the animation is
   fire-and-forget, there is no distinct "in the middle of opening, don't re-trigger" state
   beyond what `open: bool` already captured; the enum only makes "closed" and "already
   triggered" mutually exclusive by construction, same as the bool did, but names the states).
2. **Given** backlog #14's literal proposal (`_on_door_body_entered(body: Gd<Player>)`, typing
   the SIGNAL PARAMETER itself so gdext/Godot only calls the handler for actual `Player` bodies),
   **When** evaluated, **Then** it is REJECTED: `[connection] body_entered → 
   _on_door_body_entered` (`door.tscn:35`) has NO per-connection type filter — Godot invokes the
   handler for EVERY body that enters the `Area3D` and attempts to convert the argument to the
   handler's declared parameter type; if that type were `Gd<Player>`, every non-player body
   (a robot, a bullet, a flying forklift) entering the door's area would produce an engine
   conversion warning/error on every entry, which `v1` does not produce — an observable behavior
   change, not a pure typing improvement. `_on_door_body_entered` THEREFORE KEEPS its
   `Gd<Node3D>` parameter and the `.tscn` connection is UNCHANGED.
3. **Given** the scenario above, **When** #14 is closed in its revised form, **Then** the
   handler's FIRST statement is the `try_cast::<Player>()` (immediate, at the boundary, exactly
   where it is today) feeding a pure `fn on_body(state: DoorState, is_player: bool) ->
   (DoorState, bool)` (the trailing `bool` is "should play the open animation now") that
   reproduces `door.rs:26-29` exactly: only a `Player` body on a currently-`Closed` door opens
   it; anything else (wrong body type, or a door that's already `Open`) is a no-op.

### Edge Cases

- **Spawn-frame land/sound (backlog #10)**: excluded from the parity harness's very first
  physics frame comparison — `v1` fires `land` there, this branch does not, by design.
- **Post-respawn velocity (backlog #11)**: excluded from the parity harness's frame immediately
  following a below-`-40` teleport — `v1` preserves fall velocity there, this branch zeroes it,
  by design.
- **Same-tick bullet expiry + collision (backlog #13)**: the parity harness's bullet scenarios
  must include one deliberately-timed case where a bullet's `time_alive` crosses zero on the
  SAME physics tick it collides, and assert exactly one `explode` (a `v1`-vs-`v2` DIVERGENCE
  point recorded as intentional, not a parity failure — `v1` doubles the RPC there).
  Non-coincident expiry and collision cases remain full parity checks.
- **Non-player body at the door**: a robot or bullet entering the door's `Area3D` must produce
  no state change and no engine warning on EITHER branch — the harness's door scenario includes
  a non-player entry as a control case.
- **Airborne root-motion persistence**: while the player is airborne across several frames, the
  harness must NOT expect `root_motion` (and hence horizontal velocity) to freeze at zero — it
  keeps integrating whatever the last grounded frame captured, matching `v1`'s field-persistence
  behavior (Scenario 6).
- **First-shot hitch (backlog #28, informational only)**: the checkpoint after US1 asks the user
  to note, without any code change in response, whether preloading `bullet.tscn` (Scenario 2)
  changed the previously-observed ~10-frame hitch on the player's first shot of a sequence. This
  observation is recorded in `docs/v2-backlog.md`'s existing #28 row, which stays open.

## Requirements *(mandatory)*

### Functional Requirements

**`player.rs` (US1)**

- **FR-001**: `player_id: i32`, `motion: Vector2`, `current_animation: Animations` MUST keep
  their exact names, types and export/var attributes; `set_player_id(&mut self, value: i32)`
  MUST keep its exact signature (`level.rs:206`'s call site) and `#[func]` attribute (needed for
  the `#[var(set = set_player_id)]` property setter path replication uses).
- **FR-002**: RPCs `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma` MUST keep their
  exact names, `#[rpc]` attributes, and observable effect; `self.base_mut().rpc(...)` dynamic
  dispatch by name MUST remain (permanent residual, no typed alternative exists).
- **FR-003**: The two per-shot `get_node_as::<CpuParticles3D>` calls in `shoot` MUST become
  `OnReady<Gd<CpuParticles3D>>` fields resolved once; the per-shot
  `load::<PackedScene>("bullet.tscn")` in `apply_input` MUST become an `OnReady<Gd<PackedScene>>`
  field resolved once (the scene resource lookup only — a fresh instance is still created per
  shot via `.instantiate_as::<CharacterBody3D>()`).
- **FR-004**: `apply_input` MUST acquire `self.player_input` (`bind`/`bind_mut`) AT MOST ONCE
  per physics frame, building one `InputFrame` snapshot (`motion`, `aiming`, `shooting`,
  `jumping`, `shoot_target`, `camera_rotation_basis`, `camera_base_quaternion`, `aim_rotation`)
  from that single acquisition; the non-authority `physics_process` branch MUST likewise acquire
  `player_input` at most once per frame (only for `get_aim_rotation()`, needed solely when
  entering the `Strafe` animation state).
- **FR-005**: A pure `airborne_step` function (no `Gd`, no engine call) MUST reproduce `v1`'s
  jump/land/airborne logic exactly, unit-tested for: landing after exceeding the `0.5` s
  threshold, NOT landing at or under it, jumping while grounded, NOT double-jumping while
  already airborne, landing AND jumping in the same frame (both events true), and the exact
  order of accumulate-then-check-then-reset.
- **FR-006**: Pure functions (no `Gd`, `Basis`/`Quaternion`/`Vector3`/`Transform3D` only) MUST
  reproduce, unit-tested: the orientation slerp toward a target basis at
  `ROTATION_INTERPOLATE_SPEED`; the walking-target computation with its `> 0.001` length guard;
  the root-motion integration (apply, extract horizontal velocity, clear origin, orthonormalize).
- **FR-007**: Tunable constants (`MOTION_INTERPOLATE_SPEED`, `ROTATION_INTERPOLATE_SPEED`,
  `MIN_AIRBORNE_TIME`, `JUMP_SPEED`, the `0.5` s land threshold, the `-40.0` respawn threshold)
  MUST become a `PlayerTuning` struct with a `Default` matching `v1`'s literals.
- **FR-008**: `animate()`'s four `if anim ==`/`else if` branches MUST become a `match` over a
  computed `AnimPlan` enum (`JumpUp`, `JumpDown`, `Strafe { aim_rotation, blend_position }`,
  `Walk { blend_position }`); the four `AnimationTree` parameter path strings MUST become named
  constants; `jump`/`land`'s direct animation calls MUST go through the same apply-step
  function via `AnimPlan`'s trivial variants.
- **FR-009**: Backlog #10 MUST be closed: `airborne_time` starts at `0.0`.
- **FR-010**: Backlog #11 MUST be closed: the respawn branch zeroes `velocity`.
- **FR-011**: Backlog #12 MUST be closed: the never-read `crosshair` field is removed.
- **FR-012**: A `PeerId` newtype for `player_id` was considered and is NOT applied — recorded in
  Assumptions with the reason (no concrete confusion it would prevent within this module).

**`bullet.rs` (US2)**

- **FR-013**: `hit: bool` + `time_alive: f32` MUST be replaced by `enum BulletState { Flying {
  time_alive: f32 }, Exploded }` with a pure `step(state, dt) -> (BulletState, bool)` function
  reproducing `v1`'s countdown/expiry exactly.
- **FR-014**: Backlog #13 MUST be closed: a collision on the same physics tick `step()` already
  produced "explode now" (lifetime expiry) MUST NOT fire a second `explode` RPC — everything
  else on that tick (the `move_and_collide`, the collider's `hit` RPC, the collision-shape
  disable) MUST still happen exactly as in `v1`.
- **FR-015**: `has_method("hit")` MUST be replaced by a crate-level typed `HitTarget` (`enum
  HitTarget { Player(Gd<Player>), Robot(Gd<EnemyRobot>) }`) resolved via `try_cast`; the RPC
  dispatch (`.rpc("hit", &[])`) MUST remain by name (permanent residual). This abstraction MUST
  live in a shared, crate-visible location (not private to `bullet.rs`), since V2-D's
  `EnemyRobot` reuses it.
- **FR-016**: `BULLET_VELOCITY` MUST become an associated constant or a `BulletTuning` field
  with a `Default` matching the literal.

**`door.rs` (US3)**

- **FR-017**: `open: bool` MUST become `enum DoorState { Closed, Open }`.
- **FR-018**: `_on_door_body_entered` MUST KEEP its `Gd<Node3D>` parameter and `door.tscn:35`'s
  `[connection]` MUST NOT change — backlog #14's literal proposal (typing the parameter itself
  as `Gd<Player>`) is REJECTED as a behavior change (every non-player body entering would
  produce an engine conversion error `v1` does not produce).
- **FR-019**: The handler's immediate `try_cast::<Player>()` MUST feed a pure `fn on_body(state:
  DoorState, is_player: bool) -> (DoorState, bool)` reproducing `v1`'s logic exactly (open only
  a `Closed` door for a `Player` body; no-op otherwise).

**Cross-cutting (all three modules)**

- **FR-020**: Each module's `#[godot_api] impl I<Base>`/`impl X` blocks MUST contain only
  lifecycle callbacks and exposed API, delegating to plain `impl`/free functions for domain
  logic (Principle III); `player.rs` gains a pure `player/model.rs` submodule; `bullet.rs` gains
  either an inline `mod pure` or a `bullet/model.rs` submodule (plan-time choice, per research.md
  R8's precedent: pick based on the pure surface's size); `door.rs`'s pure surface (one small
  function) stays an inline `mod pure` or bare free function — no separate file needed.
- **FR-021**: `cargo build`/`cargo clippy`/`cargo test` MUST be clean before every commit;
  headless validation (`CLAUDE.md`'s recipe) MUST show no new errors after every commit.
- **FR-022**: A parity harness run against a `v1` worktree (separate `XDG_DATA_HOME` per tree,
  `--fixed-fps 60` for determinism, as in V2-B) MUST cover: scripted movement/strafe/jump/shoot
  input driving an offline authority-1 `Player` (transform, velocity, `current_animation`
  dumped per frame); a bullet's lifetime-to-explode frame count against a wall and against a
  robot (robot `health` before/after); the same-tick expiry+collision case (documented
  divergence, not asserted equal); a door-open scenario with a player and a non-player body
  (documented no-op for the latter). The spawn-frame and post-respawn-frame cases (#10/#11) are
  excluded from equality assertions per Edge Cases.
- **FR-023**: One local commit per module (US1 may be two: pure model, then glue), each message
  naming what was remodeled; never pushed.

### Key Entities

- **`InputFrame`** (`player.rs`): one physics frame's snapshot of `player_input` — `motion:
  Vector2`, `aiming: bool`, `shooting: bool`, `jumping: bool`, `shoot_target: Vector3`,
  `camera_rotation_basis: Basis`, `camera_base_quaternion: Quaternion`, `aim_rotation: f64`.
- **`AirborneOutcome`** (`player.rs`): the pure result of `airborne_step` — new `airborne_time`,
  whether the player is airborne this frame, and two independent event flags `land`/`jump`
  (both may be true in the same frame, as in `v1`).
- **`AnimPlan`** (`player.rs`): `JumpUp | JumpDown | Strafe { aim_rotation: f64, blend_position:
  Vector2 } | Walk { blend_position: Vector2 }` — the pure animation decision, applied once.
- **`PlayerTuning`** (`player.rs`): the 6 tunable constants named in FR-007.
- **`BulletState`** (`bullet.rs`): `Flying { time_alive: f32 } | Exploded` — replaces `hit` +
  `time_alive`.
- **`HitTarget`** (crate-level, introduced by `bullet.rs`, reused by V2-D): `Player(Gd<Player>)
  | Robot(Gd<EnemyRobot>)` — replaces `has_method("hit")` duck typing.
- **`BulletTuning`** (`bullet.rs`): `BULLET_VELOCITY`, per FR-016.
- **`DoorState`** (`door.rs`): `Closed | Open` — replaces `open: bool`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build`/`cargo clippy`/`cargo test` clean, with at least 15 new unit tests
  across the pure functions/modules introduced by this milestone (`airborne_step`, orientation
  slerp, walking-target guard, root-motion integration, `AnimPlan` selection per state, motion
  lerp, `BulletState::step`, `DoorState::on_body`) — total test count at or above 56 (41
  inherited from V2-B + 15).
- **SC-002**: Zero `get_node_as`/`load::<PackedScene>` calls remain inside `player.rs`'s
  `physics_process`/`shoot`/`apply_input` bodies (grep + code review) — all such lookups moved
  to `OnReady` fields resolved once.
- **SC-003**: At most one `self.player_input.bind()`/`bind_mut()` call per physics frame, on
  both the authority and non-authority code paths (code review — no automated grep can express
  "at most one call", since the pattern `player_input.bind` may legitimately appear once).
- **SC-004**: Zero `has_method` calls remain in `bullet.rs` (grep).
- **SC-005**: Headless validation (import + `main.tscn`/`level.tscn`) shows zero new errors
  relative to the documented baseline, after every commit.
- **SC-006**: The parity harness's non-excluded cases (movement/strafe/jump/shoot trace; bullet
  lifetime and explode-on-wall/robot frame counts; door open/no-op) produce identical dumps
  between `v1` and this branch; the spawn-frame, post-respawn, and same-tick-expiry+collision
  cases are documented divergences (backlog #10/#11/#13), not equality assertions.
- **SC-007**: `docs/v2-backlog.md` items #2, #10, #11, #12, #13, #14 are marked done citing this
  milestone's closing commits; #28 remains open, its existing row annotated with the user's
  checkpoint observation (hitch changed / unchanged / not tested).

## Assumptions

- Phase v2 (constitution 1.4.0). The behavior deviations from `v1` beyond the closed backlog
  items are exactly the three named in the top block (#10, #11, #13) plus #14's REVISED closure
  (no signal-parameter type change) — all pre-authorized, all already listed.
- **`PeerId` newtype deferred**: `player_id` is read/written in exactly two places within
  `Player` (the field itself and the `set_multiplayer_authority(value)` call inside its own
  setter) and is never mixed with another bare `i32` in the same scope (unlike, say, a case
  where a health value and an id value could be accidentally swapped). A newtype here would be
  type-system decoration without a concrete bug it rules out, unlike `AimState`/`BulletState`/
  `DoorState`, which each eliminate a real invalid-state combination. `docs/v2-catalog.md`'s
  suggestion of `PeerId (#[godot(transparent)])` is noted as a possibility for a future
  milestone if `player_id` ever needs to be passed alongside other bare integers.
- **`HitTarget`'s exact shape** (an `enum` over the two concrete types vs. a trait object) is a
  plan-time decision — the constitution requires the type-safety outcome (no more
  `has_method("hit")`), not a specific Rust encoding. Since `Gd<T>` appears in its variants, this
  abstraction is GLUE, not pure domain logic — Principle III's "no `Gd<T>` in pure modules" rule
  does not apply to it, and it is not expected to carry unit tests beyond what `bullet.rs`'s own
  `step` function already covers (there is no pure decision inside the type resolution itself).
- **Module layout for `bullet.rs`/`door.rs`** (inline `mod pure` vs. a separate `<module>/
  model.rs` file) is a plan-time decision per research.md R8's established precedent (file
  boundary follows the pure surface's size, not a fixed rule) — not fixed by this spec.
- The parity harness reuses V2-B's `v1`-worktree + `XDG_DATA_HOME` + `--fixed-fps 60` pattern;
  building the bullet-vs-robot and door scenarios (spawning a target `EnemyRobot`/`Door` scene
  alongside the scripted `Player`) is new work for this milestone, not a reuse of V2-B's
  single-scene harness structure.
- Out of scope: `red_robot.rs`/`part.rs` (V2-D, only benefits from `HitTarget` being
  crate-visible — no `red_robot.rs` edit happens in this milestone); `main.rs`/`level.rs`/
  `menu.rs` (unaffected, only consumers, verified unchanged call sites); backlog #6 (still
  deferred, V2-B); backlog #15-#25 (out of scope, different modules); any change to `player.tscn`
  /`bullet.tscn`/`door.tscn` beyond what a name-preserving Rust remodel requires (none is
  expected — `door.tscn`'s `[connection]` is explicitly KEPT, not removed, per US3/FR-018).
