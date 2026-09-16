# Research: Milestone V2-B — Leaves and player input

Evidence gathered from gdext 0.5.5 sources (`~/.cargo/registry/src/index.crates.io-*/godot-core-0.5.5/`),
the generated bindings for the local Godot 4.7.2 build (`oxide_godot_core/target/debug/build/
godot-core-*/out/classes/`), the current working tree (`oxide_godot_lib/src/{player_input,
camera_noise_shake,debug_label,part_disappear,blast}.rs`, `player.rs`, `player.tscn`), and two
small headless GDScript probes run against the local Godot binary (float stringify, noise
determinism — scripts not committed, throwaway).

## R1 — `OnEditor<Gd<T>>` with `#[export]` and `node_paths`

**Decision**: all 6 node references (`camera_animation`, `crosshair`, `camera_base`, `camera_rot`,
`camera_camera`, `color_rect`) become `#[export] field: OnEditor<Gd<T>>`, same names, same
`#[export]` attribute. `player.tscn` needs ZERO edits: `node_paths=PackedStringArray(...)` and the
per-field `NodePath(...)` values are Godot's own node-reference export mechanism (the engine
resolves the stored `NodePath` into an object reference and calls the property setter once the
scene subtree is ready); it is agnostic to whether the Rust-side storage is `Option<Gd<T>>` or
`OnEditor<Gd<T>>` — both implement the `Export`/`Var` traits gdext's property glue needs to accept
a `Gd<T>` value being set. `player.rs:138`'s only touch point (`camera_camera`) changes from
`self.player_input.bind().camera_camera.clone().unwrap()` to
`self.player_input.bind().camera_camera.clone()` (`OnEditor<Gd<T>>` derefs straight to `Gd<T>`,
and `Gd<T>: Clone`).

**Evidence**: `godot-core-0.5.5/src/obj/on_editor.rs`'s doc comment shows the exact pattern as a
first-class, documented example:
```rust
#[derive(GodotClass)]
#[class(base = Node)]
struct NodeHolder {
    #[export]
    required_node: OnEditor<Gd<Node>>,
    base: Base<Node>
}
```
and states the failure mode precisely: *"When used inside a node class, `OnEditor` checks if a
value has been set before `ready()` is run, and panics otherwise. This validation is performed
for all `OnEditor` fields declared in a given `GodotClass`, regardless of whether they are
`#[var]`, `#[export]`, or neither."* This is a **louder, earlier** failure than `v1`'s per-frame
`.unwrap()` (which panics deep inside `process()` the first time a mis-wired scene runs a frame
that touches the field) — it panics once, before `ready()` starts, for every `OnEditor` field at
once, which is exactly backlog #8's motivation ("makes the editor flag a missing reference").

**Alternatives considered**: keeping `Option<Gd<T>>` and only removing the per-frame `.unwrap()`
by resolving once into a local `Gd<T>` at `ready` — rejected, since it still requires a fallible
unwrap somewhere and doesn't get the pre-`ready()` validation; `OnEditor` is strictly better for
mandatory references and is what backlog #8 itself names.

## R2 — Parent handle: resolved once via `OnReady::from_base_fn`

**Decision**: `player_input.rs` adds `parent: OnReady<Gd<CharacterBody3D>>` and
`parent_rid: OnReady<Rid>`, both initialized via `OnReady::from_base_fn(|base| ...)` — NOT a
hand-written `ready()` assignment — so they auto-resolve before `ready()` runs, in declaration
order, exactly like every other `OnReady` field in this codebase:
```rust
#[init(val = OnReady::from_base_fn(|base| base.get_parent().unwrap().cast::<CharacterBody3D>()))]
parent: OnReady<Gd<CharacterBody3D>>,
#[init(val = OnReady::from_base_fn(|base| base.get_parent().unwrap().cast::<CharacterBody3D>().get_rid()))]
parent_rid: OnReady<Rid>,
```
`get_world_3d()`/`get_direct_space_state()` are still called on `self.parent` fresh every frame
(they are cheap getters reflecting current engine state, not something to cache) — only the
*lookup* of the parent NODE and its RID is cached, matching Principle III's "resolve once per
frame/event" (the tree-walk-and-cast, not the physics-state reads that legitimately vary frame to
frame).

**Evidence**: `godot-core-0.5.5/src/obj/on_ready.rs:174-192` — `OnReady::new<F>(init_fn: F) where
F: FnOnce() -> T + 'static` is defined as `Self::from_base_fn(|_| init_fn())`; the more general
`from_base_fn<F>(init_fn: F) where F: FnOnce(&Gd<Node>) -> T + 'static` gives the closure the
node's own `&Gd<Node>` base — precisely what's needed to call `.get_parent()` without a
hand-written `ready()`. This is the same "automatic mode" already used for V2-A's
`settings: OnReady<Gd<Settings>>` (`OnReady::new(|| ...)`), just using the `Base`-aware variant
since resolving the parent needs a node reference the plain `new()` closure doesn't have.

**Alternatives considered**: `OnReady::manual()` + explicit `self.parent.init(...)` inside a
hand-written `ready()` — works, but reintroduces a hand-written `ready()` body purely for this
one line, when `from_base_fn` does it declaratively at the field, matching the project's existing
style (no module in this codebase hand-writes `ready()` only to resolve an `OnReady` field).

## R3 — `InputSnapshot` and the pure API

**Decision**: one snapshot struct, read once at the top of `process`:
```rust
struct InputSnapshot {
    motion: Vector2,       // move_right-left, move_back-forward
    camera_move: Vector2,  // view_right-left, view_up-down
    aim_just_pressed: bool,
    aim_pressed: bool,
    aim_just_released: bool,
    jump_just_pressed: bool,
    shoot_pressed: bool,
}
```
built from the 8 `get_action_strength` calls + 5 boolean action checks, all against
`Input::singleton()`, in `process`'s first lines. The `input()` callback (mouse motion) is
**glue**, not folded into the same snapshot — it fires on a different lifecycle event
(`InputEventMouseMotion`, not every frame) with its own single read (`get_screen_relative()`) —
but it calls the SAME pure rotation functions as `process`, so there is exactly one pure
implementation of "how a look-delta becomes a new clamped pitch/yaw", not two.

Pure functions (all engine-free — `Vector2`/`Vector3` are gdext math builtins, never cross FFI):
- `step_aim(state: AimState, snapshot: &InputSnapshot, dt: f32, tuning: &PlayerInputTuning) -> (AimState, Option<CameraCue>)`
- `scaled_look(raw: Vector2, aiming: bool, dt: f32, tuning: &PlayerInputTuning) -> Vector2` (controller: `raw * dt * speed`, halved while aiming)
- `scaled_mouse_look(raw: Vector2, aiming: bool, tuning: &PlayerInputTuning) -> Vector2` (mouse: `raw * speed`, `×0.75` while aiming, no `dt`)
- `clamp_pitch(current: f32, delta_y: f32, tuning: &PlayerInputTuning) -> f32`
- `alpha_for_height(y: f32, prev_alpha: f32, dt: f32) -> f32`
- `aim_rotation(camera_x_rot: f32, tuning: &PlayerInputTuning) -> f64`

Apply step (glue, in `process`/`input`): write `self.motion`; call `scaled_look`/`scaled_mouse_look`
then `clamp_pitch`, then `camera_base.rotate_y`/`orthonormalize`/`camera_rot.set_rotation` (the
only engine writes for rotation); call `step_aim`, update the `AimState` field and the projected
`aiming: bool`, play the animation cue if `Some`; `if jump_just_pressed { rpc("jump") }`; if
shooting, raycast via the cached `parent`/`parent_rid` and write `shoot_target`; compute
`alpha_for_height` from the cached parent's current transform and write `color_rect`'s modulate.

## R4 — `AimState` ↔ `aiming: bool` projection

**Decision**: the projection is written inside the apply step, immediately after `step_aim`
returns the new state — `self.aiming = !matches!(new_state, AimState::Idle)` — every frame the
state is recomputed (i.e. every frame, since `step_aim` runs every frame regardless of whether it
changes anything), not only on transition edges. This guarantees the replicated field is never
stale, at the cost of a trivial redundant write on frames where nothing changed (`self.aiming =
self.aiming` in practice) — cheaper and simpler than tracking "did it change" separately, and
`MultiplayerSynchronizer` replication already only sends deltas at the network layer regardless of
how often the Rust field is *assigned* locally.

## R5 — Camera shake: 3 `FastNoiseLite` instances, bit-identical confirmed empirically

**Decision**: three `FastNoiseLite` instances, each seeded exactly once at `ready`
(`seed`, `seed.wrapping_add(1)`, `seed.wrapping_add(2)`), each with `fractal_octaves = 1` and
`fractal_lacunarity = 1.0` (matching `v1`'s settings, applied to all three now instead of one
object reconfigured on the fly — the three calls in `v1`'s `ready()` set these on the SAME shared
object once, so applying them to each of the 3 new objects once is the direct equivalent). This
removes all 3 per-frame `set_seed` calls entirely (0 FFI calls/frame for seeding, down from 3).

**Evidence (empirical, headless probe against the local Godot 4.7.2 binary)**: created 3
`FastNoiseLite` instances seeded once (`12345`, `12346`, `12347`) vs. one shared instance
re-seeded 3 times immediately before each `get_noise_1d(1234.5)` call — **identical output to the
full printed precision** in both arrangements (`-0.47050881385803`, `-0.6902214884758`,
`0.42612898349762` in both cases). Confirms `get_noise_1d`'s output depends only on
`(seed, fractal_octaves, fractal_lacunarity, position)`, not on any other mutable state the
object accumulates — the 3-instance form is bit-identical to `v1`, not merely "close enough".
The fallback named in the spec (re-seed one shared instance) is therefore NOT needed.

**`v1`'s exact axis mapping** (confirmed by reading `camera_noise_shake.rs`'s `apply_shake`):
`time += delta * SPEED * 5000.0` first (the "magic number" comment in v1 — part of the formula,
reproduced by a pure `advance_time`), then `yaw` reads `noise_seed` (unmodified), `pitch` reads
`noise_seed.wrapping_add(1)`, `roll` reads `noise_seed.wrapping_add(2)`; the final rotation is
`start_rotation + Vector3(pitch, yaw, roll)`
— i.e. `Vector3.x = pitch (seed+1)`, `.y = yaw (seed)`, `.z = roll (seed+2)`. `offsets()` (R8)
takes `samples: [f32; 3]` indexed by seed-offset (`samples[0]` = seed, `samples[1]` = seed+1,
`samples[2]` = seed+2) and returns `Vector3(MAX_PITCH·shake·samples[1], MAX_YAW·shake·samples[0],
MAX_ROLL·shake·samples[2])`, preserving this exact (unconventional) mapping.

## R6 — Debug label: `compose` format, pinned by two empirical checks

**Decision**: `DebugStats { fps: f64, vsync_enabled: bool, ram_bytes: u64, vram_bytes: u64,
multiplayer_id: Option<i64> }`. Two of `v1`'s four value lines are ALREADY plain Rust formatting
(no Godot-stringify quirk to reproduce) and two need care:
- **Memory / VRAM lines**: `v1` already computes these with Rust's own `format!("{:3.2}", bytes as
  f64 / 1048576.0)` (confirmed by reading `debug_label.rs:30-34`) — `compose` reproduces this
  `format!` call verbatim for both `Memory:` (existing) and the new `VRAM:` line (backlog #26),
  no special algorithm needed.
- **FPS line**: `v1` goes through `Variant::from(fps).stringify()`, which is Godot's own
  float-to-string conversion, NOT a Rust `format!`. Empirically probed (headless GDScript,
  `str()` is GDScript's equivalent of `Variant::stringify()`): `str(60.0)` → `"60.0"`,
  `str(59.94)` → `"59.94"`. Compared against Rust's own `f64` `Display` (`{}`) on the same
  values: `60.0` → `"60"`, `59.94` → `"59.94"`. The two agree on the FRACTIONAL representation
  (both use a shortest-round-trip algorithm) and differ ONLY in that Godot always keeps at least
  one decimal digit for a float, while Rust drops the decimal point entirely for a whole number.
  **`compose`'s FPS formatting is therefore**: `let s = format!("{fps}"); if s.contains('.') { s }
  else { format!("{s}.0") }` — this matches Godot's `stringify()` for whole numbers and short
  fractions, which covers `get_frames_per_second()` (always integer-valued). It is NOT a general
  reimplementation of Godot's float printing (Godot renders `0.1 + 0.2` as `0.3`, Rust as
  `0.30000000000000004`); the parity harness confirms the actual bytes.
- **VRAM source**: `RenderingServer::singleton().get_rendering_info(RenderingInfo::VIDEO_MEM_USED)
  -> u64` — confirmed present in the generated bindings
  (`out/classes/rendering_server.rs:16946`, `:5341`), same shape as `Os::get_static_memory_usage()
  -> u64` already used for RAM.
- **Online / Multiplayer ID**: `multiplayer_id: Option<i64>` — `None` prints `v1`'s "Online: No"
  (with no ID line); `Some(id)` prints "Online: Yes" + "Multiplayer ID: {id}" — same text, typed
  so the "online but no id" / "offline with an id" combinations don't type-check (FR-017).

**Hidden-frame skip / same-frame refresh (backlog #4)**: `process` checks the toggle action
first; if the label is hidden AFTER that check, return immediately (no `DebugStats`, no
`compose`, no `set_text`). If the toggle flips visibility ON this frame, fall through to the
normal visible-frame path in the SAME `process` call — so the very first visible frame already
shows fresh stats, never a stale string from before the overlay was hidden.

## R7 — The one `async` timer pattern

**Decision**, verified against the gdext 0.5.5 sources directly (not just the API's existence,
already confirmed in the spec's Assumptions):

- **No `Send` bound**: `godot::task::spawn(future: impl Future<Output = ()> + 'static) ->
  TaskHandle` requires only `'static`, NOT `Send` (`task/async_runtime.rs:148`). The doc comment
  explains why: "We can not accept `Sync + Send` futures since all object references (i.e.
  `Gd<T>`) are not thread-safe. So a future has to remain on the same thread it was created on."
  `TaskHandle` itself is `!Send + !Sync` (`_no_send_sync: PhantomData<*const ()>`). This confirms
  capturing a `Gd<Self>` inside the `async move` block is exactly the intended usage, not a
  workaround.
- **Fallible resolution on a freed emitter**: `FallibleSignalFuture`'s `poll` returns
  `Poll::Ready(Err(FallibleSignalFutureError))` when the signal's state is `Dead` (`task/
  futures.rs:259`) — confirmed this is a clean `Result::Err`, not a panic; the plain (non-fallible)
  `SignalFuture` instead panics in the equivalent situation unless the engine itself is exiting
  (`task/futures.rs:68-71`). This is exactly why FR-020 mandates the fallible variant for any
  signal whose *emitter* isn't guaranteed to outlive the wait.
- **Emitter lifetimes for the two usages**: a `SceneTreeTimer` (`create_timer(...)`) is owned and
  kept alive by the `SceneTree` itself until it fires, independent of the awaiting node's own
  lifetime — `to_future()` is fine for `part_disappear.rs`'s two timer waits. A child
  `AnimationPlayer` is owned by (and freed together with) the `Blast` node that awaits it — if
  something else frees `Blast` early, the `AnimationPlayer` and its `animation_finished` signal
  go with it, so `blast.rs` uses `to_fallible_future()`.
- **Guarding the awaiting object itself**: after EVERY `await`, the pattern re-checks
  `godot_self.is_instance_valid()` on the captured `Gd<Self>` before calling `bind_mut()` again —
  this is what protects `part_disappear.rs` (whose own emitters, `SceneTreeTimer`s, resolve
  normally even if `part_disappear` itself were freed meanwhile) and complements the fallible
  check for `blast.rs`. `Gd<T>::bind_mut()` on a freed object panics; the `is_instance_valid()`
  guard turns that into a clean early return instead.

**The paragraph for `CLAUDE.md`'s "Port conventions (v2)"** (FR-021, verbatim to add):

> **Await-style sequences**: replace nested `connect_other` signal chains with one `async` block
> spawned from the lifecycle callback via `godot::task::spawn`. Capture `Gd<Self>` (never
> `&mut self`) at spawn time; after every `.await`, check `is_instance_valid()` on that handle and
> return early if the node was freed, before calling `bind_mut()` again. Use
> `TypedSignal::to_future()` when the signal's emitter is guaranteed to outlive the wait (e.g. a
> `SceneTreeTimer`, owned by the `SceneTree`); use `to_fallible_future()` — treating `Err` as the
> same early return — whenever the emitter's own lifetime isn't otherwise guaranteed (e.g. a
> child node that can be freed independently). No feature flag is required
> (`godot::task`/`TypedSignal::to_future`/`to_fallible_future` are unconditional in gdext 0.5.5).

**Alternatives considered**: a `Timer` node added per effect — rejected now that the `async` form
is confirmed to work exactly as needed; kept as a documented fallback only if a future gdext
version regresses this (not expected).

## R8 — Module layout

**Decision**:
- `player_input.rs` (glue) + `player_input/model.rs` (pure: `AimState`, `CameraCue`,
  `PlayerInputTuning`, `InputSnapshot`, `step_aim`, `scaled_look`, `scaled_mouse_look`,
  `clamp_pitch`, `alpha_for_height`, `aim_rotation`, `#[cfg(test)] mod tests`) — mirrors
  `settings.rs`/`settings/graphics.rs` from V2-A.
- `camera_noise_shake.rs` (glue) + `camera_noise_shake/model.rs` (pure: `CameraShakeTuning`,
  `decay`, `shake`, `offsets`, tests) — same reason, the pure part is more than a few lines.
  Naming these `<module>/model.rs` throughout (not `graphics.rs`, `input.rs`, etc.) keeps a single
  convention name across the crate for "the pure sibling of a glue file", now that a second
  precedent exists beyond V2-A's `settings/graphics.rs` (that one keeps its established name;
  new modules from here on use `model.rs`).
- `debug_label.rs`: pure `compose`/`DebugStats` land in an INLINE `mod pure { ... }` block with
  its own `#[cfg(test)] mod tests` nested inside, not a separate file — the pure surface is one
  struct and one function, small enough that a submodule file would be pure ceremony; Principle
  III requires the separation (a distinct `mod`, no engine types inside it), not a distinct file.
- `part_disappear.rs`, `blast.rs`: unchanged, single-file — confirmed in the spec (US4 scenario 5)
  that neither has domain logic to separate.
- `PlayerInputTuning`/`CameraShakeTuning` are plain structs with a `Default` impl matching `v1`'s
  literals, passed by `&reference` into the pure functions that need them (constitution's
  "tuning struct with `Default` that the pure functions receive as input").

## R9 — Parity harness design

**Decision**: `zz_leaves_parity.tscn`/`.gd`, same shape as V2-A's harness (root `Node`, copied
unchanged to a `v1` worktree, run headless with a separate `XDG_DATA_HOME` per tree,
`OS.get_cmdline_user_args()` selects which case to run if a case needs its own process — none do
here, since nothing in this milestone tears down the compatibility surface `config_file`-style;
one run covers all four cases).

- **(a) Input state machine + fall-to-black**: instantiate `player.tscn` as an offline,
  authority-1 player (matches how `main.tscn`'s headless auto-host already runs a local game, so
  this is a realistic setup, not a contrived one). Drive `Input.action_press("aim")` /
  `action_release("aim")` / `action_press("jump")` / `action_press("move_forward")` etc. across a
  scripted sequence of `await get_tree().process_frame` steps (one frame = one `process()` tick;
  `is_action_just_pressed`/`just_released` are edge-triggered and clear at the next frame
  boundary, so each scripted press/release needs its own frame before the next input change).
  Dump per scripted step: `aiming`, `motion`, `shoot_target`, `camera_rot`'s pitch, `ColorRect`'s
  modulate alpha. For the fall-to-black case, teleport the player's `CharacterBody3D` below
  `y = -17` for a few frames, then back above it, and dump alpha across that window. The
  shoot-target scenario positions the crosshair target so the ray does NOT pass through the
  player's own collider first (the one documented excluded case, backlog #7).
- **(b) Camera shake**: fix the global RNG with `seed(N)` BEFORE instantiating `player.tscn` (so
  `CameraNoiseShake::init`'s `randi()` — unchanged by this milestone — produces the same
  `noise_seed` on both branches), then trigger shake via `player.rpc("add_camera_shake_trauma",
  amount)` — this method is `#[rpc(authority, call_local, unreliable)] pub(crate) fn
  add_camera_shake_trauma(&mut self, amount: f64)` on `Player` ITSELF (confirmed by reading
  `player.rs:135-139`), unchanged by this milestone (`player.rs` is V2-C), and already reachable
  by name from GDScript on both branches — no need to keep `CameraNoiseShake::add_trauma` itself
  `#[func]` (FR-013 stands). Dump the camera's `rotation` per frame after triggering.
- **(c) Debug label**: force the `DebugLabel` node visible (`set_visible(true)` from the harness,
  bypassing the toggle action), wait one frame, dump `text`, with the `VRAM:` line stripped
  before comparing (present on `v2` only, by design — not a parity requirement, SC-004 covers its
  presence separately).
- **(d) Part-disappear / blast**: instantiate `part_disappear.tscn` / `impact_effect.tscn`
  directly, count `process_frame` awaits until `is_instance_valid()` on the captured handle
  becomes `false` (queued-free), dump the frame count (bounded by each effect's own known total
  duration from `v1`'s code: `0.2 + lifetime*2.0` seconds for the part puff; one `AnimationPlayer`
  cycle for the blast).

**Unit-test-only** (not covered by the harness, stated once so it isn't silently assumed):
the SSAO/SSIL-style "no getter" problem does not recur here, but the FastNoiseLite bit-identical
claim (R5) IS covered by the harness's shake dump (both branches use the real engine's own
`FastNoiseLite`, so if `v1` and `v2` ever diverged it would show up in the rotation dump) — no
additional gap to name for this milestone.

## R10 — Commit plan

One commit per module, `player_input.rs` split into two (pure model, then glue — matching V2-A's
`settings`/`settings/graphics` split order), the `player.rs:138` edit riding inside the
`player_input` GLUE commit (not the pure-model commit, since that's the commit that actually
changes the type consumers see), the `CLAUDE.md` paragraph riding inside the US4 (async pattern)
commit, and `docs/v2-backlog.md` bookkeeping in a final commit — full detail in `plan.md`'s
Commit Plan.
