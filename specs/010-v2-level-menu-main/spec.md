# Feature Specification: Milestone V2-E — level, menu, main: the scene manager and the end of v2

**Feature Branch**: `v2` (work directly, per constitution 1.4.1 Principle II — these four modules
are the last v1-shaped ones; after this milestone every module follows Principles I–III and v2
is complete). No per-milestone feature branch. Baseline: `dcf948a` (V2-D complete). Local
commits only, never pushed.

**Created**: 2026-09-18

**Status**: Draft

**Phase**: v2 — Idiomatic Rust (constitution 1.4.1, Principles I, II and III). Both pillars
apply, done together: (1) idiomatic Rust leaning on the type system (a typed scene-manager
enum/dispatch replacing `has_signal` + `Callable::from_object_method`, a declarative
option↔button table replacing ~250 lines of `if`/`else if`, a pure `GiPlan` replacing three
near-identical `setup_*` functions) and (2) FFI reduction through Principle III (per-spawn
`load::<PackedScene>` resolved once via preloaded fields, repeated `get_node_as` resolved via
`OnReady`, the crate's last two by-name dynamic calls — `call_deferred("change_scene_to_packed",
...)` and `call_deferred("_on_host_pressed", ...)` — replaced by a typed deferred mechanism).

- **Backlog items closed by this spec**: #3 (`has_signal("quit")`/`has_signal(
  "replace_main_scene")` replaced by typed `try_cast` + `connect_other`, closing the
  constitution's own named goal — "a centralized signal-driven scene manager"), #19
  (`flying_forklift.rs`'s per-instance `randomize()` removed — `Main` already seeds once at
  boot; **found during this milestone's investigation: `level.rs`'s OWN pre-shuffle
  `randomize()` call is the SAME class of redundancy, not previously catalogued as its own
  item — closed together with #19 in the same commit, as an extension of its scope**), #20
  (`add_child_ex(...).force_readable_name(true)` applied uniformly to both the robot and the
  player spawn — a no-op for the player, who already gets an explicit unique name via
  `set_name`, before `force_readable_name` could ever matter), #21 (the default-parameter
  hazard the item names does not actually exist in the current code — `add_player`/`del_player`
  are already plain methods reached through an explicit, already-typed closure connection,
  `peer_connected`/`peer_disconnected`, that already passes `None` explicitly where v1's default
  parameter would have applied; **this milestone's actual remaining work is extracting the
  player-spawn-point-picking arithmetic into a pure, tested function**, closing the item's
  Principle III spirit even though its literal premise had already been resolved), #22
  (`_on_settings_pressed`/`_on_apply_pressed`'s ~250 lines of `if`/`else if` become one
  declarative option↔button table, walked twice — once to read, once to write), #23
  (`replace_main_scene`'s signal declaration already carries its parameter in the current code;
  the `quit`/`replace_main_scene` connections in `Main` become typed), #24
  (`call_deferred("change_scene_to_packed", ...)` by name replaced by a typed deferred
  mechanism — the same one closes the analogous `call_deferred("_on_host_pressed", ...)` in
  `menu.rs`, discovered during this milestone's own reading as the SAME class of by-name
  deferred call, not separately catalogued).
- **Backlog items conditionally closed**: #32 (the robot-respawn FPS drop) — this spec preloads
  `red_robot.tscn` in `Level` (closing the `load::<PackedScene>` half of the item's own
  hypothesis); the checkpoint after US2 asks the user to observe whether the respawn hitch
  changed. If confirmed resolved, #32 closes citing this milestone's commit; if not, it stays
  open with the observation appended (the same "record, don't chase further" pattern #28/#29
  used in V2-C/V2-D).
- **Backlog items explicitly deferred, unchanged**: #6 (camera `start_rotation` ownership), #25
  (SSAO/SSIL naming ambiguity, parity preserved verbatim by design), #29 (occasional FPS drop,
  unconfirmed relation, needs reproduction), #30 (`door.tscn`'s collision-mask fix, deferred by
  user request), #31 (the robot state's flat-vs-enum counters, deferred alongside a genuine `v1`
  behavior change). All five get a one-line "post-v2 review" note in the milestone's final docs
  commit — they are NOT re-litigated here, only acknowledged as still open when v2 closes.
- **Residual dynamic access left in the crate after this milestone**: every `.rpc("name", &[])`
  site across every v2 module (`player.rs`: `jump`/`land`/`shoot`; `bullet.rs`: `explode`;
  `hittable.rs`: `hit`; `red_robot.rs`: `play_shoot`; `part.rs`: `destroy`) — **all permanent**,
  gdext exposes RPC dispatch only by name, the engine-limitation residual case the constitution
  names explicitly. After this milestone, `.rpc(` sites are the ONLY by-name dynamic access
  anywhere in the crate: zero `has_signal`, zero `has_method`, zero `.call(`, zero `.get("`,
  zero `call_deferred("..."`, zero `Callable::from_object_method`, crate-wide (SC-007, grepped).

**Input**: User description: "Milestone V2-E — level, menu, main. Apply the V2-A through V2-D
patterns (typed autoload access, snapshot→step→apply, `OnReady`/preloaded resources, the
`godot::task` async pattern, declarative pure mappings) to the last four v1-shaped modules:
`main_scene.rs` (the scene manager the constitution names as a v2 goal), `level.rs` +
`flying_forklift.rs` (spawning, GI setup, the respawn hitch), and `menu.rs` (the largest
remaining `if`/`else if` surface, ~250 lines mapping 15 settings rows to buttons). After this
milestone, v2 is complete: every module follows Principles I–III, and the only by-name dynamic
access left in the crate is `.rpc(\"name\")`, gdext's permanent RPC-dispatch limitation."

## Context

`Main` is consumed by nothing — it is the crate's own bootstrap root (`main.tscn`'s single root
node, `type="Main"`, confirmed by grep: **zero** `[connection]` blocks in `main.tscn`, and no
other file references `go_to_main_menu`/`replace_main_scene`/`change_scene_to_packed` by string
anywhere in the project). This means all three of `Main`'s current `#[func]`s lose that
attribute once the signal connections that reach them today (`Callable::from_object_method`
targets) become typed `connect_other` closures instead — nothing will call them by name
afterward.

`Menu` is consumed **dynamically** by `Main` today (the thing this milestone fixes) and exposes
its 10 `[connection]`s FROM `menu.tscn` (confirmed by grep, `menu.tscn:834-843`): 8 button
`pressed` signals (`Play`, `PlayOnline`, `Settings`, `Quit`, `Host`, `Connect`, `Back`×2 reused
for both `Online/Back` and `Settings/Actions/Cancel`, `Settings/Actions/Apply`) plus
`Loading/DoneTimer`'s `timeout` — ALL ten reach `#[func]`s by name via `.tscn`-declared
connections, so all ten methods MUST keep both their exact name and `#[func]` attribute; this is
unaffected by the scene-manager typing (that only changes how `Main` reaches `Menu`'s
`replace_main_scene` SIGNAL, not how `menu.tscn` reaches `Menu`'s own methods).

`Level` is consumed **typed** already: `main_scene.rs` will `try_cast::<Level>()` the
instantiated node and connect its `#[signal] quit()` typed (today it goes through
`has_signal`/`Callable::from_object_method`, the thing this milestone fixes); `Level`'s own
`peer_connected`/`peer_disconnected` connections to `MultiplayerApi` are ALREADY typed
(`connect_other`, confirmed by reading `level.rs:68-75` — no work needed there).
`flying_forklift.rs` is consumed only by `flying_forklift.tscn`'s own root-type swap (no other
module references it); its `settings` autoload read is already typed (V2-A).

Confirmed by reading all four files in full (`main_scene.rs` 74 lines, `level.rs` 210,
`menu.rs` 593, `flying_forklift.rs` 37):

| Module | Base | What consumers call today |
|---|---|---|
| `main_scene.rs` | `Node` | Nothing calls `Main`'s `#[func]`s by name (grep-confirmed); `main.tscn` has zero `[connection]`s. `Main` itself dynamically reaches whatever scene it just instantiated (`has_signal`/`Callable::from_object_method`) — this dynamic direction is what closes. |
| `level.rs` | `Node3D` | `main_scene.rs` will `try_cast::<Level>()` + connect `quit()` typed (closes this milestone). `level.tscn`'s own children (`RobotSpawnpoints`, `PlayerSpawnpoints`, `SpawnedNodes`, `VoxelGI`, `ReflectionProbes`, `WorldEnvironment`) are read by name/type, all `OnReady`-eligible or already `OnReady`. |
| `menu.rs` | `Node` | `menu.tscn`'s 10 `[connection]`s (unchanged, see above). `main_scene.rs` will `try_cast::<Menu>()` + connect `replace_main_scene(scene)` typed (closes this milestone). |
| `flying_forklift.rs` | `CharacterBody3D` | `flying_forklift.tscn`'s own root-type swap only; no Rust module references it. |

`GraphicsSettings` (already typed since V2-A, `settings/graphics.rs:176-192`) is the struct
`menu.rs`'s option table reads from and writes to — 15 fields: `display_mode: WindowMode`,
`vsync: VSyncMode`, `max_fps: i32`, `resolution_scale: f64`, `scale_filter: ScaleFilter`,
`taa: bool`, `msaa: Msaa`, `screen_space_aa: ScreenSpaceAa`, `shadow_mapping: bool`,
`gi_type: GiType`, `gi_quality: GiQuality`, `ssao_quality: SsaoQuality`,
`ssil_quality: SsilQuality`, `bloom: bool`, `volumetric_fog: bool` — unchanged by this milestone
(only how `menu.rs` reads/writes it changes, not the struct itself).

Baseline gates at the start of this milestone (inherited from V2-D, unchanged): `cargo build` /
`cargo clippy` / `cargo test` all clean, 92 tests. Behavior baseline for parity is branch `v1`
(a separate worktree, built and run independently, as in V2-A through V2-D).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `main_scene.rs`: the scene manager (Priority: P1)

The game still boots into the main menu, playing still loads the level, pressing Esc in the
level still returns to the menu, and the settings menu's Apply still works — but `Main` reaches
whichever scene it just loaded through typed signal connections instead of probing it with
`has_signal` and dispatching through a by-name `Callable`.

**Why this priority**: this is the constitution's own named v2 goal ("a centralized
signal-driven scene manager", Principle I); every other user story's checkpoint depends on the
game actually booting through `Main`.

**Independent Test**: `cargo test` passes (if any pure logic is extracted — see Acceptance
Scenario 3); headless `main/main.tscn` boots to the menu with no new errors; in the running
game, Play → level loads and plays; Esc in the level → back to the menu; the settings menu's
Apply button still returns to the menu.

**Acceptance Scenarios**:

1. **Given** `change_scene_to_packed`'s current generic handling of ANY instantiated node
   (`if node.has_signal("quit") { ... }`, `if node.has_signal("replace_main_scene") { ... }`,
   `main_scene.rs:64-72`), **When** remodeled, **Then** the instantiated node is `try_cast`
   against the two concrete types it can actually be (`Menu`, `Level` — the only two scenes
   `Main` ever loads, confirmed: `go_to_main_menu` loads `menu.tscn`, `Menu::
   _on_loading_done_timer_timeout` emits `replace_main_scene` carrying `level.tscn`'s packed
   scene); on a successful cast, the corresponding signal is connected TYPED
   (`level.signals().quit().connect_other(&*self, |this: &mut Main| this.go_to_main_menu())`;
   `menu.signals().replace_main_scene().connect_other(&*self, |this: &mut Main, scene:
   Gd<PackedScene>| this.replace_main_scene(scene))`) — the exact Rust encoding (an owned
   `enum Scene` field vs. purely local `try_cast`s inside `change_scene_to_packed` with no
   persisted state, since nothing today ever reads back "what scene is currently active") is a
   plan-time decision; behaviorally, EITHER shape must produce identical connections to today's
   dynamic ones for both concrete scene types.
2. **Given** `replace_main_scene`'s `self.base_mut().call_deferred("change_scene_to_packed",
   &[resource.to_variant()])` (`main_scene.rs:52-53`) and `menu.rs`'s analogous
   `self.base_mut().call_deferred("_on_host_pressed", &[])` for headless auto-host
   (`menu.rs:212`) — the crate's last two by-name deferred calls, discovered to be the SAME
   class of issue during this milestone's own reading (only `main_scene.rs`'s was originally
   catalogued as backlog #24) — **When** remodeled, **Then** BOTH become a typed deferred
   mechanism with identical timing (still deferred to the next idle frame, not called
   synchronously) — the exact API (`Callable::from_local_fn` + `call_deferred`, a
   `godot::task::spawn` block awaiting one `process_frame` then calling the typed method
   directly, or an equivalent typed-deferred primitive if gdext 0.5.5 exposes one) is a
   plan-time decision with the gdext source cited as evidence, per research.md.
3. **Given** `Main`'s three `#[func]`s (`go_to_main_menu`, `replace_main_scene`,
   `change_scene_to_packed`), **When** Scenario 1's typed connections and Scenario 2's typed
   deferred call land, **Then** NONE of the three is reached by name anymore (confirmed:
   `main.tscn` has zero `[connection]`s; no other file references any of the three by string,
   grep-confirmed in Context) — all three lose `#[func]` and become plain `pub(crate)` methods.
4. **Given** the boot sequence (`ready()`: disable server relay, set headless max FPS, seed the
   global RNG once, apply the typed `Settings`' window mode, call `go_to_main_menu`) and the
   offline-peer reset inside `go_to_main_menu` (close the current peer, install a fresh
   `OfflineMultiplayerPeer`), **When** remodeled, **Then** both stay EXACTLY as they are today —
   pure glue, no domain decision to extract; Principle III requires pure separation only where
   domain logic exists, and none does here beyond the type-based dispatch Scenario 1 already
   covers.

---

### User Story 2 - `level.rs` + `flying_forklift.rs`: spawning, GI setup, the respawn hitch (Priority: P1)

The level still boots with the correct global-illumination technique and quality applied,
robots and players still spawn where they should, a defeated robot still comes back 15 seconds
later, and forklifts still show a random one of their three models — but the per-spawn resource
loads are resolved once, the GI setup's three near-identical functions share one pure decision,
and the two redundant `randomize()` calls (the forklift's own, and `level.rs`'s own before the
player-spawn shuffle) are gone, letting a seeded parity harness reproduce every RNG-driven
choice.

**Why this priority**: independent of US1 (its own harness can run standalone), but shares P1
because `Level` is where the milestone's own `#[init(val = load(...))]` preload closes half of
backlog #32's hypothesis, and the checkpoint asks the user to specifically evaluate the respawn
hitch — the highest-value observation of this milestone.

**Independent Test**: `cargo test` passes for the new pure `gi_plan`/`pick_spawn`/`pick_model`
functions; headless `level/level.tscn` and the auto-hosted `main/main.tscn` show no new errors;
in the running game, a level boots with the GI technique its settings specify, robots and
players appear at their spawn points, killing a robot brings a fresh one back ~15 seconds
later, and flying forklifts show one of three models at random.

**Acceptance Scenarios**:

1. **Given** `spawn_robot`'s per-spawn `load::<PackedScene>("res://enemies/red_robot/
   red_robot.tscn")` and `add_player`'s per-spawn `load::<PackedScene>("res://player/
   player.tscn")` (`level.rs:165-166,203-204`), **When** remodeled, **Then** both become
   `#[init(val = load(...))]` bare fields (V2-C's `bullet_scene` precedent — no tree
   dependency), resolved once; a fresh instance is still created per spawn via
   `.instantiate_as::<T>()`, unchanged.
2. **Given** the three near-identical `setup_sdfgi`/`setup_voxelgi`/`setup_lightmapgi`
   functions (`level.rs:94-162`, each: unconditionally set one GI technique's headline flag,
   hide/show the other techniques' nodes, free an existing `LightmapGi` if switching away from
   it, then a `match gi_quality` writing the technique-specific detail), **When** decomposed
   into a pure `gi_plan`, **Then** it reproduces every line's effect exactly, INCLUDING two
   quirks the literal transcription must preserve: (a) `create_lightmap` is only meaningful
   given whether a `LightmapGi` node ALREADY exists (`level.rs:149`, `if self.lightmap_gi
   .is_none()`) — a THIRD input beyond `(gi_type, gi_quality)` the pure function needs to decide
   it correctly, even though in `ready()`'s single call site a lightmap never pre-exists
   (`lightmap_gi` starts `None`); (b) `LightmapGi`'s visibility is ONLY ever explicitly set to
   hidden (`gi_quality == Disabled`, `:158-161`) — unlike `ReflectionProbes`, which is
   unconditionally shown at `setup_lightmapgi`'s top (`:147`) THEN conditionally hidden, nothing
   in the current code explicitly RE-SHOWS an already-hidden `LightmapGi`; a fully faithful
   pure signature makes this asymmetry explicit (e.g. an `Option<bool>` "set visibility to
   this, only if `Some`" field for the lightmap specifically, vs. a plain `bool` for
   `ReflectionProbes`) rather than collapsing it to a uniform shape. The exact field types are a
   plan-time transcription decision; the BEHAVIOR above is what data-model.md must reproduce,
   verified per (gi_type × gi_quality) combination — 9 cells — plus the `has_lightmap` axis
   where it matters.
3. **Given** the 15 s robot respawn (`_respawn_robot`, `level.rs:178-185`,
   `create_timer(15.0).signals().timeout().connect_other(...)`), **When** closed via the
   established async pattern, **Then** it becomes a `godot::task::spawn` block capturing
   `Gd<Level>` and the spawn point, awaiting the `SceneTreeTimer` via `to_future()` (the
   `SceneTree`-owned shape, no fallible variant needed — same reasoning as every prior
   milestone's `create_timer` waits), checking `is_instance_valid()` on the captured `Gd<Level>`
   before calling `spawn_robot` again — same 15 s delay, same effect.
4. **Given** `add_player`'s inline random-spawn-point pick (`level.rs:196-202`,
   `randi() % count`) when no explicit spawn point is passed, **When** extracted, **Then** a
   pure `pick_spawn(r: i64, count: i64) -> i64` (or the plan's exact numeric types) reproduces
   `% count` exactly, fed an already-sampled `randi()` value from glue (RNG stays in glue,
   established convention since `part.rs`).
5. **Given** the two spawn call sites' different `add_child` forms (`spawn_robot`'s
   `add_child_ex(&robot).force_readable_name(true).done()` vs. `add_player`'s plain
   `add_child(&player)`, `level.rs:172-175,208`), **When** unified, **Then** BOTH use
   `add_child_ex(...).force_readable_name(true).done()` — chosen because it is OBSERVABLY A
   NO-OP for the player (whose name is already set explicitly via `set_name(&id.to_string())`,
   `level.rs:205`, BEFORE `add_child` — `force_readable_name` only affects nodes Godot would
   otherwise need to auto-name/disambiguate, which an explicitly-named, already-unique player
   node never is) while leaving the robot's existing display-name behavior completely unchanged.
6. **Given** `Level::ready`'s own `randomize()` call immediately before the player-spawn-point
   shuffle (`level.rs:57`) — a SEPARATE redundant re-seed from `Main`'s own boot-time
   `randomize()` (`main_scene.rs:29`, which ALWAYS runs first, since `Main` is the crate's own
   bootstrap root and every scene, including `Level`, loads after it) — **When** removed
   (closing this alongside backlog #19, which only catalogued the forklift's own instance of
   the identical redundancy), **Then** the spawn-point shuffle draws from whatever RNG state
   `Main`'s single boot-time seed left it in — observably indistinguishable in normal play
   (still effectively random), but now reproducible under a harness `seed()` call made once,
   after `Main`'s boot, instead of needing to account for a SECOND reseed inside `Level`.
7. **Given** `flying_forklift.rs`'s own per-instance `randomize()` (`flying_forklift.rs:27`,
   backlog #19's originally-catalogued instance) and its model-pick arithmetic (`(randf() *
   child_count as f64).floor() as usize`, `:30`), **When** remodeled, **Then** the
   `randomize()` call is removed (same reasoning as Scenario 6) and a pure `pick_model(r: f64,
   count: usize) -> usize` reproduces the floor-multiply formula exactly, fed an
   already-sampled `randf()` value from glue.
8. **Given** `flying_forklift.rs`'s shadow-mapping read (already typed `Settings`, unchanged)
   and its children-visibility toggle (`:28-33`, hide all but the picked index), **When**
   remodeled, **Then** both stay functionally identical — the visibility toggle loop itself is
   glue (it touches `Gd<Node3D>` children), only the INDEX it toggles on comes from the pure
   function.

### Edge Cases

- **`create_lightmap`'s hidden state dependency (Scenario 2)**: the parity harness's GI-plan
  case must exercise `ready()`'s actual single-call pattern (a level that has never had a
  `LightmapGi` before) — it is NOT expected to exercise the theoretical "already has one"
  branch, since nothing in the current codebase ever re-enters `setup_lightmapgi` a second
  time; the pure function's THIRD input exists for correctness/testability, not because the
  harness can observe a second call.
- **The redundant `randomize()` removals (Scenarios 6–7) are the ONLY behavior deltas of US2**:
  in an UNSEEDED run (normal play), removing a redundant re-seed changes nothing observable —
  the RNG was already random either way. Under a SEEDED harness run, the exact sequence of
  values `randi()`/`randf()` produce from that point on DOES change (one fewer reseed call
  consumes/resets the stream differently) — this is the expected, intentional effect of closing
  #19 and its `level.rs` extension, not a bug; the harness must `seed()` its own run AFTER
  instantiating `Main` (or wherever the boot-time `randomize()` now lives) for a reproducible
  spawn/model draw, exactly as the milestone brief anticipates.
- **Respawn hitch observation (backlog #32, informational)**: the checkpoint after US2 asks the
  user to note, without any code change in response, whether preloading `red_robot.tscn`
  changed the previously-observed hitch on a defeated robot's respawn. This observation is
  recorded in `docs/v2-backlog.md`'s existing #32 row — closed if confirmed resolved, annotated
  and left open otherwise.
- **`menu.tscn`'s 10 `[connection]`s are entirely unaffected by US1**: `Main` reaching `Menu`
  typed changes how `Main` listens to `Menu`'s OWN `replace_main_scene` signal; it does not
  touch how `menu.tscn`'s buttons reach `Menu`'s own `#[func]`s, which is a completely separate
  wiring path (scene-file connections, not `Main`'s dynamic-signal-probing code).

---

### User Story 3 - `menu.rs`: declarative option table (Priority: P2)

Opening the settings menu still shows exactly the currently-saved choice pre-selected in every
row, and pressing Apply after changing any combination of rows still applies and saves exactly
those choices — but the ~250 lines of repetitive `if`/`else if` chains become one declarative
table walked twice (once to read, once to write), so adding a future settings row touches one
place instead of two.

**Why this priority**: independent of US1/US2 (its own harness — V2-A's settings harness,
rerun as-is — proves it); smaller in behavioral risk than US1/US2 despite being the largest
line-count change, since every value mapping already exists, typed and tested, in
`GraphicsSettings`/`settings/graphics.rs`.

**Independent Test**: `cargo test` passes for the new pure option-table mapping; V2-A's
settings harness (menu buttons → `settings.ini` bytes + applied engine state) produces
identical output on both trees; in the running game, every settings row round-trips: open
settings, change a row, Apply, reopen — the changed row shows the new selection.

**Acceptance Scenarios**:

1. **Given** `_on_settings_pressed`'s 15 read-and-`set_pressed` blocks and `_on_apply_pressed`'s
   15 read-`is_pressed`-and-write blocks (`menu.rs:299-415`, `:423-560`), **When** replaced by
   one declarative table (one entry per settings row, each pairing a `GraphicsSettings` field
   with its group of buttons and the value each button corresponds to), **Then** the
   value↔button mapping for each row is a PURE, unit-tested function, reproducing every
   quirk the current code has, verbatim:
   - `display_mode`: `WINDOWED` OR `MAXIMIZED` both show "Windowed" pressed; `FULLSCREEN` shows
     "Fullscreen"; anything else (today, only `EXCLUSIVE_FULLSCREEN`) shows "Exclusive
     Fullscreen" (`:306-310`). On Apply, pressing "Windowed" ALWAYS writes back `WINDOWED`
     specifically — a saved `MAXIMIZED` that was never touched collapses to `WINDOWED` the next
     time Apply is pressed (`:430-436`) — THIS COLLAPSE MUST BE PRESERVED, not "fixed".
   - `resolution_scale: f64`: shown via `is_equal_approx` against six literal ratios in a fixed
     order (`1/3, 1/2, 1/1.7, 1/1.5, 1/1.3`, else "Native" — `:330-342`); an unlisted custom
     value (e.g. a hand-edited `settings.ini`) falls through to "Native" being shown pressed
     EVEN THOUGH the actual value isn't `1.0` — preserve this fallback-to-last, not "fix" it
     into a "nothing selected" or "closest match" behavior.
   - `msaa`/`screen_space_aa`: on show, an UNLISTED enum value (there is no explicit catch-all,
     `:371-384`) leaves NO button in the group pressed at all — a DIFFERENT quirk from
     `resolution_scale`'s fallback (preserve both, verbatim, as they are today).
   - Every OTHER row (`vsync`, `max_fps`, `scale_filter`, `gi_type`, `gi_quality`, `ssao_quality`,
     `ssil_quality`, and the four plain booleans `taa`/`shadow_mapping`/`bloom`/
     `volumetric_fog`) has an exhaustive show-side mapping (a named button for every value the
     typed enum/bool admits) and an apply-side mapping with NO trailing catch-all (if — in
     practice never, since these are `ButtonGroup`s guaranteeing exactly one selection — no
     button in a group were pressed, the field is left unchanged, not reset to a default); the
     table-driven form MAY collapse this into a total/exhaustive match instead of literally
     replicating "silently do nothing", since `ButtonGroup`'s own invariant makes that branch
     UNREACHABLE in practice — state this simplification explicitly if taken.
2. **Given** `!self.metalfx_supported`'s effect (hides the two MetalFX `scale_filter` buttons,
   `:217-220`, evaluated ONCE in `ready()`, not re-evaluated per settings-menu open), **When**
   remodeled, **Then** this stays exactly as it is — a one-time gate, unaffected by the
   table-driven refactor of the READ/WRITE logic itself.
3. **Given** `call_deferred("_on_host_pressed", &[])` for headless auto-host (`menu.rs:212`),
   **When** remodeled, **Then** it uses the SAME typed deferred mechanism US1's Acceptance
   Scenario 2 establishes for `main_scene.rs`'s analogous call — one mechanism, two call sites.
4. **Given** `process`'s load-status polling (`menu.rs:239-258`: `IN_PROGRESS` → update the
   progress bar; `LOADED` → set 100%, stop polling, start the done-timer; anything else → log
   the error, show the main menu again, hide the loading bar), **When** decomposed, **Then** a
   pure `loading_step(status: ThreadLoadStatus, progress: f64) -> LoadingCmd` (an enum:
   `UpdateProgress(f64)`, `Finished`, `Failed`) reproduces the three-way branch exactly; glue
   performs the engine reads (`load_threaded_get_status_ex`) and applies whichever `LoadingCmd`
   comes back.
5. **Given** `_make_button_group` (glue, wires each settings row's buttons into one
   `ButtonGroup` so exactly one is ever pressed) and the ~50 `OnReady<Gd<Button>>`/`OnReady<
   Gd<HBoxContainer>>` fields (scene structure, one per button/row container), **When**
   reviewed, **Then** both stay exactly as they are — no `.tscn` edit, no field removed; the
   table-driven mapping reads/writes THROUGH these existing `OnReady` handles, it does not
   replace them.

## Requirements *(mandatory)*

### Functional Requirements

**`main_scene.rs` (US1)**

- **FR-001**: `change_scene_to_packed` MUST `try_cast` the freshly instantiated node against
  `Menu` and `Level` (the only two scenes `Main` ever loads) and, on a successful cast, connect
  that scene's own signal (`quit()` for `Level`, `replace_main_scene(scene)` for `Menu`) TYPED
  via `connect_other`; `has_signal`/`Callable::from_object_method` MUST NOT remain anywhere in
  `main_scene.rs`.
- **FR-002**: `replace_main_scene`'s `call_deferred("change_scene_to_packed", ...)` and
  `menu.rs`'s `call_deferred("_on_host_pressed", ...)` MUST both use the SAME typed deferred
  mechanism (plan-time choice, evidenced against gdext 0.5.5's source), preserving the deferred
  (next-idle-frame) timing of both call sites exactly.
- **FR-003**: `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` MUST lose
  `#[func]` once FR-001/FR-002 land (confirmed by grep: nothing calls any of the three by name
  after this milestone).
- **FR-004**: The boot sequence (server relay off, headless max FPS, one `randomize()`, window
  mode from `Settings`, `go_to_main_menu`) and the offline-peer reset inside `go_to_main_menu`
  MUST be unchanged.

**`level.rs` + `flying_forklift.rs` (US2)**

- **FR-005**: The per-spawn `load::<PackedScene>` calls for `red_robot.tscn` (in `spawn_robot`)
  and `player.tscn` (in `add_player`) MUST become `#[init(val = load(...))]` bare fields,
  resolved once (backlog #32's `load` half).
- **FR-006**: The three `setup_sdfgi`/`setup_voxelgi`/`setup_lightmapgi` functions MUST be
  replaced by one pure `gi_plan`-shaped decision (exact signature a plan-time transcription
  decision — see Acceptance Scenario 2's two quirks it MUST preserve) applied once in `ready`;
  `VoxelGI`/`ReflectionProbes` node lookups MUST become `OnReady` fields (were `get_node_as`
  per call).
- **FR-007**: The 15 s robot respawn (`_respawn_robot`) MUST use the established
  `godot::task::spawn` + `SceneTreeTimer::to_future()` + `is_instance_valid()` async pattern,
  producing the same 15 s delay and the same effect (a fresh `spawn_robot` call) as today.
- **FR-008**: `add_player`'s random-spawn-point pick (when no explicit spawn point is given)
  MUST become a pure function fed an already-sampled `randi()` value from glue.
- **FR-009**: Both `Level` spawn call sites (robot, player) MUST use the SAME `add_child`
  form (`add_child_ex(...).force_readable_name(true).done()`), verified to have no observable
  effect on the player's node name (already set explicitly before `add_child`).
- **FR-010**: `Level::ready`'s own pre-shuffle `randomize()` call and
  `flying_forklift.rs`'s own per-instance `randomize()` call MUST both be removed (backlog #19
  and its `level.rs` extension); `flying_forklift.rs`'s model-pick arithmetic MUST become a
  pure function fed an already-sampled `randf()` value from glue.
- **FR-011**: `peer_connected`/`peer_disconnected`'s existing typed `connect_other` connections
  MUST be unchanged (already compliant, confirmed by reading the current code).

**`menu.rs` (US3)**

- **FR-012**: `_on_settings_pressed` and `_on_apply_pressed` MUST be replaced by one declarative
  option↔button table walked twice (read: settings → button state; write: button state →
  settings), preserving EVERY quirk named in Acceptance Scenario 1 verbatim (the
  `MAXIMIZED`→`Windowed` collapse on Apply, `resolution_scale`'s fallback-to-Native on an
  unlisted value, `msaa`/`screen_space_aa`'s "nothing selected" on an unlisted value); the
  value↔button mapping for each row MUST be pure and unit-tested.
- **FR-013**: `!self.metalfx_supported`'s one-time button-hiding gate in `ready()` MUST be
  unchanged.
- **FR-014**: `menu.rs`'s `call_deferred("_on_host_pressed", ...)` MUST use the SAME typed
  deferred mechanism FR-002 establishes.
- **FR-015**: `process`'s load-status polling MUST be decomposed into a pure `loading_step`
  function (three-way: progress update, finished, failed) driving the SAME three observable
  effects (progress bar value, done-timer start, or the error-log+show-menu fallback).
- **FR-016**: `_make_button_group` and every existing `OnReady` field MUST be unchanged; no
  `.tscn` edit in this milestone.

**Cross-cutting (all four modules)**

- **FR-017**: Each module's `#[godot_api] impl I<Base>`/`impl X` blocks MUST contain only
  lifecycle callbacks and exposed API, delegating to plain `impl`/free functions or a pure
  submodule for domain logic (Principle III); `level.rs` gains a pure `level/model.rs`
  submodule (`gi_plan`, `pick_spawn`); `menu.rs` gains a pure `menu/model.rs` submodule (the
  option table, `loading_step`); `main_scene.rs` and `flying_forklift.rs` stay glue-only or gain
  a small inline `mod pure` (per plan-time file-size precedent) — `flying_forklift.rs`'s only
  pure surface is `pick_model`, one function.
- **FR-018**: `cargo build`/`cargo clippy`/`cargo test` MUST be clean before every commit;
  headless validation (`CLAUDE.md`'s recipe, extended to `menu/menu.tscn` and the auto-hosted
  end-to-end `main/main.tscn` boot) MUST show no new errors after every commit.
- **FR-019**: A parity harness run against a `v1` worktree (separate `XDG_DATA_HOME`,
  `--fixed-fps 60`, `seed()` called once after the boot-time `randomize()` per Edge Cases) MUST
  cover: (a) V2-A's existing settings harness, rerun as-is; (b) the GI plan's 9
  `(gi_type × gi_quality)` cells, dumping `sdfgi_enabled`, `VoxelGI`/`ReflectionProbes`
  visibility, `LightmapGI` presence; (c) spawn: robot count/positions under `SpawnedNodes`,
  player presence, a robot's respawn timing after `exploded` (frame count to the new robot
  appearing), the forklift's picked model index under a fixed seed (documented #19 divergence:
  `v1` is unseeded, so its own model pick cannot be compared value-for-value — only that A
  model, not zero and not more than one, is visible); (d) `main`: menu → play → level → quit →
  menu, confirming the node type under `Main` at each step.
- **FR-020**: One local commit per module (US2 may be two: `level.rs`, then
  `flying_forklift.rs`, or combined — plan-time choice), each message naming what was
  remodeled; never pushed. The milestone's final commit updates `README.md`'s Versions table
  (v2's `Status` cell: "In progress" → "**Complete**", with a short summary naming the modules
  and pattern count) and `docs/v2-backlog.md` (closing this milestone's items; a one-line
  "v2 complete on `<date>`; open items are post-v2 candidates" header note; the stale "26
  items" count in `README.md`'s "Upstream behavior" row updated to the current total).

### Key Entities

- **Typed scene dispatch** (`main_scene.rs`): the `try_cast::<Menu>`/`try_cast::<Level>` +
  `connect_other` pattern replacing `has_signal` + `Callable::from_object_method`; exact Rust
  encoding (owned enum vs. local casts) deferred to plan-time.
- **Typed deferred call** (`main_scene.rs`, `menu.rs`): the mechanism FR-002/FR-014 share,
  replacing `call_deferred("name", ...)`; exact API deferred to plan-time.
- **`GiPlan`** (`level.rs`): the pure decision `gi_plan` produces — the observable engine
  effects of choosing a GI technique and quality, replacing the three `setup_*` functions'
  duplicated logic; exact field shape (including the `has_lightmap` input and the lightmap
  visibility `Option<bool>` asymmetry) deferred to plan-time, behavior pinned by Acceptance
  Scenario 2.
- **`pick_spawn`** (`level.rs`): pure `(r, count) -> index` for the random player spawn point.
- **`pick_model`** (`flying_forklift.rs`): pure `(r, count) -> index` for the random forklift
  model, the same shape as `pick_spawn` but a separate function (different modules, no shared
  abstraction forced between them).
- **Option table** (`menu.rs`): the declarative description of all 15 settings rows —
  per-row value↔button mapping, pure and unit-tested; exact Rust encoding (a `match`-based
  function per row vs. a data-driven table literal) deferred to plan-time.
- **`LoadingCmd`** (`menu.rs`): `UpdateProgress(f64) | Finished | Failed` — the pure result of
  `loading_step`, replacing `process`'s three-way branch.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build`/`cargo clippy`/`cargo test` clean, with at least 20 new unit tests
  across the pure functions/modules introduced by this milestone (`gi_plan` — 9+ cells,
  `pick_spawn`, `pick_model`, the option table's per-row mappings, `loading_step`) — total test
  count at or above 112 (92 inherited from V2-D + 20).
- **SC-002**: `menu.rs`'s two settings handlers (`_on_settings_pressed`, `_on_apply_pressed`)
  each shrink to a table-walk of no more than 40 lines (down from ~115 and ~135 respectively).
- **SC-003**: Zero `load(`/`get_node_as` calls remain inside `level.rs`'s `spawn_robot`/
  `add_player`/`ready` bodies (grep + code review) — moved to `OnReady`/preloaded fields.
- **SC-004**: Zero `connect_other`-replaceable `create_timer(...).connect_other` chains remain
  in `level.rs` (grep for the removed pattern) — replaced by the `godot::task::spawn` pattern.
- **SC-005**: Crate-wide grep (all `.rs` files under `oxide_godot_lib/src/`) returns ZERO
  matches for `has_signal`, `has_method`, `\.call\(`, `\.get\("`, `call_deferred\(".*"`,
  `Callable::from_object_method` — the ONLY by-name dynamic access anywhere in the crate is
  `.rpc("name"...)`, every site listed in the spec's top block.
- **SC-006**: Headless validation (import + `main.tscn` end-to-end boot-to-menu-to-level +
  `menu.tscn` standalone + `level.tscn` standalone) shows zero new errors relative to the
  documented baseline, after every commit.
- **SC-007**: The parity harness's non-excluded cases (settings round-trip, GI plan per cell,
  spawn counts/positions, robot respawn frame count, `main`'s node-type-per-step trace) produce
  identical dumps between `v1` and this branch; the two `randomize()`-removal effects (#19 and
  its `level.rs` extension) are documented divergences under a seeded harness, not equality
  assertions — the forklift's model INDEX specifically cannot be compared value-for-value
  against unseeded `v1`, only that exactly one model is visible on both.
- **SC-008**: `docs/v2-backlog.md` items #3, #19, #20, #21, #22, #23, #24 are marked done citing
  this milestone's closing commits; #32 is marked done (if the checkpoint confirms the hitch is
  resolved) or annotated and left open (if not); #6, #25, #29, #30, #31 each gain a one-line
  "post-v2 review" note; `README.md`'s Versions table marks v2 `Status` as Complete.

## Assumptions

- Phase v2 (constitution 1.4.1). The behavior deviations from `v1` beyond the closed backlog
  items are exactly the two named in the top block/Edge Cases (the redundant `randomize()`
  removals, observable only under a seeded harness, not in normal unseeded play) — all
  pre-authorized, all already listed.
- **The scene-manager's exact Rust encoding** (an owned `Scene` enum field vs. purely local
  `try_cast`s with no persisted state) is a plan-time decision — the constitution requires
  typed dispatch, not a specific encoding; nothing today reads back "which scene is active"
  from `Main`, so either shape is behaviorally equivalent.
- **The typed deferred-call mechanism** (`Callable::from_local_fn` + `call_deferred`, a
  `godot::task::spawn` awaiting one `process_frame`, or an equivalent gdext 0.5.5 primitive) is
  a plan-time decision with source evidence required, per research.md's established precedent
  (V2-C's purity audits, V2-D's `from_base_fn` citation).
- **`gi_plan`'s exact field shape** (whether `has_lightmap` is a third pure input, whether
  `LightmapGi` visibility is `Option<bool>` vs. `bool`) is a plan-time transcription decision;
  Acceptance Scenario 2 pins the BEHAVIOR the plan must reproduce, not the Rust types.
  `docs/v2-catalog.md`'s own sketch (`GiPlan { sdfgi, voxel, probes, rays, ... }`) is a starting
  point, not a fixed contract.
- **The option table's exact Rust encoding** (per-row pure functions vs. a single data-driven
  table literal walked generically) is a plan-time decision; Acceptance Scenario 1's quirks are
  the behavioral contract, not a specific implementation shape.
- Out of scope: `player.rs`/`bullet.rs`/`door.rs`/`hittable.rs` (V2-C), `red_robot.rs`/`part.rs`
  (V2-D) — all untouched, read-only references (what `Level` spawns, what `Main`'s scene-manager
  pattern will look like once `main_scene.rs` sets the precedent); `settings.rs`/`settings/
  graphics.rs` (V2-A, untouched — this milestone only changes how `menu.rs` READS/WRITES the
  already-typed `GraphicsSettings`, never the struct or its `apply_graphics_settings`/
  `save_settings` methods); backlog #6, #25, #29, #30, #31 (explicitly deferred, see top block);
  any change to `main.tscn`/`menu.tscn`/`level.tscn`/`flying_forklift.tscn` beyond what a
  name-preserving Rust remodel requires (none is expected — every `[connection]` and node name
  stays exactly as it is today).
