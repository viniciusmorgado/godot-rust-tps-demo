# Research: Milestone V2-E — level, menu, main: the scene manager and the end of v2

## R1 — Typed deferred call (FR-002/FR-014)

Surveyed gdext 0.5.5 for a typed replacement of `self.base_mut().call_deferred("name",
&[args])` (by-name dispatch, `main_scene.rs:52-53`; `menu.rs:212`).

**Decision**: `Callable::from_fn(name, closure).call_deferred(&[])`.

- `Callable::from_fn<R, F, S>(name: S, rust_function: F) -> Self` (hand-written,
  `godot-core-0.5.5/src/builtin/callable.rs:152-158`, from the crates.io source) builds a
  `Callable` from a `'static FnMut(&[&Variant]) -> R` Rust closure — no by-name method lookup
  at all, the closure IS the typed logic.
- `Callable::call_deferred(&self, varargs: &[Variant])` — a GENERATED engine method (`target/
  debug/build/godot-core-*/out/builtin_classes/callable.rs:198-206`), confirmed via its own doc
  comment: "Calls the method represented by this Callable in deferred mode, i.e. at the end of
  the current frame... See also `call_deferred` [`crate::classes::Object::call_deferred`]." —
  this is the SAME underlying engine deferred-call queue `Object::call_deferred` uses
  internally (`Object::call_deferred` constructs a `Callable` from the object+method-name and
  defers THAT). Using `Callable::from_fn(...).call_deferred(&[])` therefore has **exactly** the
  same timing as today's `self.base_mut().call_deferred("name", ...)` — no delta to measure,
  no harness exclusion needed for R1 specifically (unlike the `godot::task::spawn` +
  `process_frame`-await alternative also surveyed, which resumes at the START of the next
  frame rather than at end-of-current-frame idle time, and would have introduced a real,
  measurable one-frame-boundary difference).
- Both `main_scene.rs`'s `replace_main_scene` and `menu.rs`'s headless-auto-host call use this
  SAME mechanism: `Callable::from_fn("change_scene_to_packed", move |_args| { this.bind_mut()
  .change_scene_to_packed(resource.clone()); Variant::nil() }).call_deferred(&[])` (capturing
  `Gd<Main>` + the packed-scene resource by move — both `'static`, matching every other
  captured-closure precedent in this codebase); `menu.rs`'s is the zero-argument form
  (`Callable::from_fn("_on_host_pressed", move |_| { this.bind_mut()._on_host_pressed();
  Variant::nil() }).call_deferred(&[])`).
- No `Gd::apply_deferred`/`run_deferred`-style API exists in gdext 0.5.5 (grepped `obj/gd.rs`,
  no match); `Callable::from_fn` + `.call_deferred(&[])` is the complete, idiomatic answer.

## R2 — Scene manager shape

`main.tscn` has ZERO `[connection]` blocks (grep-confirmed, spec Context); no `.rs`/`.tscn` file
anywhere in the project references `go_to_main_menu`, `replace_main_scene`, or
`change_scene_to_packed` by string (grepped). All three lose `#[func]` (FR-003).

`change_scene_to_packed`'s new body:

```rust
fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) {
    let node = resource.instantiate().unwrap();
    for mut child in self.base().get_children().iter_shared() {
        self.base_mut().remove_child(&child);
        child.queue_free();
    }
    let this = self.to_gd();
    if let Ok(level) = node.clone().try_cast::<Level>() {
        level.signals().quit().connect_other(&this, |main: &mut Main| main.go_to_main_menu());
    } else if let Ok(menu) = node.clone().try_cast::<Menu>() {
        menu.signals()
            .replace_main_scene()
            .connect_other(&this, |main: &mut Main, scene: Gd<PackedScene>| {
                main.replace_main_scene(scene)
            });
    }
    self.base_mut().add_child(&node);
}
```

**Decision**: NO owned `Scene` enum field. Nothing in the current code ever reads back "which
scene is active" after `change_scene_to_packed` returns (verified: `go_to_main_menu` and
`replace_main_scene` both simply call `change_scene_to_packed` again with a fresh resource,
never inspecting `Main`'s prior state) — a purely local `try_cast` inside the one function that
needs it is simpler and equally typed. This is the plan's answer to the spec's Assumptions
deferral.

`Menu::replace_main_scene`'s existing signal declaration already carries its parameter
(`menu.rs:274-275`, `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);`) — confirmed,
no change needed there; only `Main`'s SIDE of the connection changes.

## R3 — `GiPlan` transcription

Re-reading `level.rs:94-162` line by line, cross-tabulated per `(gi_type, gi_quality)` — 9
cells, plus the `has_lightmap` axis where `gi_type == LightmapGi`:

| gi_type | gi_quality | sdfgi_enabled | voxel_visible | probes_visible | free_lightmap | create_lightmap | lightmap_visible |
|---|---|---|---|---|---|---|---|
| Sdfgi | High | true | false | false | true | false | — |
| Sdfgi | Low | true | false | false | true | false | — |
| Sdfgi | Disabled | false | false | false | true | false | — |
| VoxelGi | High | false | true | false | true | false | — |
| VoxelGi | Low | false | true | false | true | false | — |
| VoxelGi | Disabled | false | false | false | true | false | — |
| LightmapGi | High | false | false | true | false | `!has_lightmap` | `None` (no explicit write) |
| LightmapGi | Low | false | false | true | false | `!has_lightmap` | `None` (no explicit write) |
| LightmapGi | Disabled | false | false | false¹ | false | `!has_lightmap` | `Some(false)` |

¹ `setup_lightmapgi` unconditionally shows `ReflectionProbes` at the top (`:147`) THEN, only if
`gi_quality == Disabled`, hides it again (`:160`) — net effect: visible unless Disabled, exactly
like the table shows. `sdfgi_rays`/`voxel_quality` are `Some(COUNT_96/32)`/`Some(HIGH/LOW)` only
on the Sdfgi/VoxelGi × High/Low cells respectively, `None` everywhere else (nothing writes them).

**The `lightmap_visible` asymmetry (spec Acceptance Scenario 2b), confirmed precisely**: on
`High`/`Low`, NOTHING in `setup_lightmapgi` explicitly sets the (possibly pre-existing)
`LightmapGi` node's visibility — a FRESH node (`create_lightmap == true`) is visible by
Godot's own default for a newly-allocated `LightmapGi` (`new_alloc()`, never hidden), so it
reads as visible in practice; but if a PRE-EXISTING, previously-hidden `LightmapGi` were re-
entered with `High`/`Low` (never happens today — `ready()` calls this once), nothing would
re-show it. This is why `lightmap_visible: Option<bool>` (`Some(false)` only on the `Disabled`
row, `None` elsewhere = "don't touch") is the faithful pure signature, NOT a plain `bool`
that would incorrectly imply an explicit "show" write exists on the other two rows.

**Decision — pure signature**:

```rust
pub struct GiPlan {
    pub sdfgi_enabled: bool,
    pub voxel_visible: bool,
    pub probes_visible: bool,
    pub free_lightmap: bool,
    pub create_lightmap: bool,
    pub lightmap_visible: Option<bool>,
    pub sdfgi_rays: Option<EnvironmentSdfgiRayCount>,
    pub voxel_quality: Option<VoxelGiQuality>,
}

pub fn gi_plan(gi_type: GiType, gi_quality: GiQuality, has_lightmap: bool) -> GiPlan
```

`EnvironmentSdfgiRayCount`/`VoxelGiQuality` are plain `#[derive(...)]` newtype-style generated
enums over an `i32` ordinal (confirmed by their use as plain `match` targets in the CURRENT
code, `level.rs:106-113,132-137` — no `Gd<T>`, no engine call to construct or compare them,
satisfying the 1.4.1 purity rule trivially since they carry no behavior, only a discriminant).

**Glue `apply_gi_plan` order** (matches `v1`'s own per-branch order, union of all three
`setup_*` bodies): (1) `world_environment.get_environment().set_sdfgi_enabled(plan
.sdfgi_enabled)`; (2) `voxel_gi.set_visible(plan.voxel_visible)`; (3)
`reflection_probes.set_visible(plan.probes_visible)`; (4) if `plan.free_lightmap`, free the
existing `self.lightmap_gi` if `Some`; (5) if `plan.create_lightmap`, allocate + load
`.lmbake` + name + store + `add_child` (the ONE remaining per-ready `load()`, unconditionally
run at most once per `Level` instance — spec's Assumptions already excludes this from FR-005's
per-spawn scope, since it is not per-spawn/per-event, it is once-per-level-instance, matching
`v1`'s own comment "If no LightmapGI node, create one"); (6) if `plan.lightmap_visible ==
Some(v)`, `self.lightmap_gi.set_visible(v)`; (7) `sdfgi_rays`/`voxel_quality`, if `Some`, write
via `RenderingServer::singleton()`.

## R4 — `level.rs` glue

- `robot_scene: Gd<PackedScene>` / `player_scene: Gd<PackedScene>` via `#[init(val =
  load(...))]` bare fields (V2-C `bullet_scene` precedent).
- `voxel_gi: OnReady<Gd<Node3D>>` (`#[init(node = "VoxelGI")]`), `reflection_probes:
  OnReady<Gd<Node3D>>` (`#[init(node = "ReflectionProbes")]`) — replacing the 5 `get_node_as`
  call sites across the three `setup_*` functions (`:96-97,122-123,139,146-147,160`).
- `pick_spawn(r: i64, count: i64) -> i64 { r.rem_euclid(count) }` — `r` an already-sampled
  `randi()` value from glue; `rem_euclid` matches `%`'s behavior for the non-negative `randi()`
  range exactly (both are equivalent for non-negative operands; `rem_euclid` is defensive
  against a hypothetical negative input, `%`'s v1 behavior for a negative left operand would
  differ from Godot's own `%` — moot here since `randi()` is always non-negative, stated for
  clarity in data-model.md, not a behavior change).
- Both spawns: `self.spawned_nodes.add_child_ex(&node).force_readable_name(true).done()`
  (spec Acceptance Scenario 5 — verified no-op for the player, whose `set_name` already runs
  first).
- `Level::ready`'s own `randomize()` (`:57`) REMOVED (spec Acceptance Scenario 6).
- `_respawn_robot`: `godot::task::spawn` capturing `Gd<Level>` (`self.to_gd()`) and the spawn
  point (`Gd<Node3D>`, cloned), `create_timer(15.0).signals().timeout().to_future().await`
  (`SceneTree`-owned timer, `to_future()`, no fallible variant — established convention), then
  `is_instance_valid()` on the captured `Gd<Level>` before calling `spawn_robot` again.
- `peer_connected`/`peer_disconnected`: UNCHANGED (`level.rs:68-75`, already typed
  `connect_other`, confirmed by re-reading).
- `add_player`/`del_player`: stay plain methods (never were `#[func]`, confirmed by re-reading
  — backlog #21's literal premise does not hold against the current code, spec top block);
  `add_player`'s inline `randi() % count as i64` (`:199`) becomes `pick_spawn(randi() as i64,
  count as i64)`.
- `Marker3D` cast (`spawn_points.pop_front().map(|n| n.cast::<Marker3D>())`) and
  `spawn_points.shuffle()` on the `Array<Gd<Node>>` returned by `get_children()`
  (`level.rs:58-59`): UNCHANGED — both already typed/engine-appropriate, no dynamic access,
  outside this milestone's dynamic-access scope (shuffle is an engine RNG call on an Array, not
  a by-name dispatch).

## R5 — `flying_forklift.rs`

`pick_model(r: f64, count: usize) -> usize { (r * count as f64).floor() as usize }` — reproduces
`flying_forklift.rs:30` exactly, fed an already-sampled `randf()`. The per-instance
`randomize()` (`:27`) is removed (spec Acceptance Scenario 7). The children-visibility toggle
loop (`:31-33`) stays glue (touches `Gd<Node3D>` children), driven by the pure function's index.

## R6 — Menu option table design

**Critical finding**: `godot::global::is_equal_approx(a: f64, b: f64) -> bool` — the function
`menu.rs:330-342` currently calls for `resolution_scale` — is ENGINE-BACKED, NOT pure. Confirmed
by reading its generated source (`target/debug/build/godot-core-*/out/utilities.rs:411-420`):
it dispatches through `sys::utility_function_table().is_equal_approx`, an FFI call to the
engine's utility-function table (the exact same category as V2-C's `Basis::looking_at`
discovery — a `#[test]` calling it panics with "Godot engine not available"). It MUST NOT be
called from the pure option-table mapping.

**Decision**: reimplement the comparison in pure Rust using Godot's documented `is_equal_approx`
semantics (per its own doc comment, `utilities.rs:411`: "within a small internal epsilon...
which scales with the magnitude of the numbers") — Godot's well-known engine formula (stable
across versions, part of its public `Math::` API contract, `CMP_EPSILON = 1e-5`): `let
tolerance = (CMP_EPSILON * a.abs()).max(CMP_EPSILON); (a - b).abs() < tolerance`. This formula
is NOT locally grep-able (it lives in Godot's C++ engine source, not the gdext Rust bindings),
so it is cited from documented/known Godot engine behavior rather than a local file — **the
implementation task MUST empirically cross-check the pure reimplementation against the REAL
`godot::global::is_equal_approx` for the six preset ratios (and a couple of near-boundary
values) from a headless context where the engine is available** (not `cargo test`), and STOP
if they disagree, per this project's established discipline (V2-C's "STOP if a gdext API
doesn't behave as research assumed"). Given the six presets are well-separated (≥0.06 apart)
and `resolution_scale` is only ever SET by this same code to one of the six exact literals (a
hand-edited config is the only way to reach a near-boundary value), the practical risk is low,
but the check must still happen.

**Decision — option table shape**: one pure function PER ROW (not a single generic data-driven
table), matching the existing code's own per-row grouping and keeping each row's quirk
(catch-all vs. none, the `MAXIMIZED` collapse) local and easy to unit-test in isolation, rather
than forcing every row through one generic `(enum_value) -> button_index` shape that doesn't
fit the boolean rows or the two irregular rows (`resolution_scale`'s epsilon compare,
`msaa`/`screen_space_aa`'s no-catch-all). Each row gets:

```rust
// Rows with an exhaustive show-side + a "leave unchanged if none pressed" apply-side collapse
// to a total match, since ButtonGroup guarantees exactly one selection (spec FR-012's stated
// simplification) -- e.g.:
pub fn display_mode_button(mode: WindowMode) -> DisplayModeButton  // show
pub fn display_mode_value(button: DisplayModeButton) -> WindowMode  // apply

// The two irregular rows keep their exact quirk:
pub fn resolution_scale_button(scale: f64) -> ResolutionScaleButton  // fallback-to-Native
pub fn msaa_button(msaa: Msaa) -> Option<MsaaButton>  // None = nothing pressed (v1's own gap)
```

15 rows total: `display_mode` (3 buttons, MAXIMIZED-collapses-to-Windowed), `vsync` (4),
`max_fps` (8), `resolution_scale` (6, epsilon fallback), `scale_filter` (6, MetalFX
visibility gate untouched), `gi_type` (3), `gi_quality` (3), `taa`/`msaa`(4, no-catch-all-show)/
`screen_space_aa`(3, no-catch-all-show)/`shadow_mapping`/`ssao_quality`(3)/`ssil_quality`(3)/
`bloom`/`volumetric_fog` (4 plain booleans). Glue's `_on_settings_pressed`/`_on_apply_pressed`
each become 15 short calls (one per row: read the field, call the row's `_button` function,
`set_pressed` on the matching `OnReady` field; or read `is_pressed()` off each of the row's
`OnReady` fields, call the row's `_value` function, write the field) — target ≤ 40 lines each
(SC-002), down from ~115/~135.

## R7 — `loading_step` + typed deferred host call

```rust
pub enum LoadingCmd {
    UpdateProgress(f64),
    Finished,
    Failed,
}

pub fn loading_step(status: ThreadLoadStatus, progress: f64) -> LoadingCmd
// v1: menu.rs:246-256's three-way branch, transcribed exactly (progress *100.0 on IN_PROGRESS,
// Finished on LOADED, Failed otherwise incl. the ERROR/logged case).
```

Glue: reads `load_threaded_get_status_ex(...).progress(&progress).done()`, extracts
`progress.at(0).to::<f64>()`, calls `loading_step`, matches the `LoadingCmd` to the three
existing effects (unchanged: set progress bar value, or stop polling + start the done timer, or
log + show main + hide loading). The headless-auto-host `call_deferred("_on_host_pressed", ...)`
uses R1's `Callable::from_fn(...).call_deferred(&[])` mechanism.

## R8 — Parity harness `zz_end_parity.tscn`/`.gd`

Four cases, both trees, `XDG_DATA_HOME` + `--fixed-fps 60`:

- **(a)** rerun `specs/006-v2-typed-settings`'s existing settings harness UNCHANGED (it already
  drives menu buttons → `settings.ini` bytes + applied engine state; the table-driven rewrite
  must reproduce it exactly — this is the harness's OWN regression test for US3, no new script
  needed, just re-run the existing one from `specs/006/contracts/`).
- **(b)** GI plan per the 9 cells (R3's table): on `v2`, set `Settings`' typed `graphics()` /
  `set_graphics()` directly before instancing `level.tscn`; on `v1`, write the equivalent
  `config_file.set_value("rendering", "gi_type"/"gi_quality", <wire code>)` before instancing
  (V2-A's wire codes, unchanged); dump `WorldEnvironment`'s `sdfgi_enabled`, `VoxelGI`/
  `ReflectionProbes` visibility, whether a `LightmapGI` child exists and its visibility.
- **(c)** spawn dump: instance `level.tscn` standalone (offline, authority), `seed()` called
  ONCE right after (matching where `Main`'s boot-time `randomize()` would have run — Edge
  Cases), dump `SpawnedNodes`' children (count, names — the robot's per FR-009 unified
  `add_child_ex(...).force_readable_name(true)` name, the player's explicit `id`-string name),
  a robot's respawn frame count after `exploded` (~900 @60fps for 15s), and the forklift's
  picked model index (documented #19 exclusion: compared for "exactly one visible", not the
  specific index, since `v1`'s own forklift call is unseeded and cannot be reproduced
  value-for-value even with the harness's OWN seed).
- **(d)** `main.tscn` end-to-end, headless auto-host (menu's own `DisplayServer... == "headless"`
  branch already does this): dump the child node's TYPE under `Main` at boot (`Menu`), after
  the auto-host completes loading (`Level`), and confirm `Level::quit()`'s effect returns to
  `Menu` — plus the exact FRAME each transition's new child appears, to empirically confirm
  R1's zero-timing-delta claim (expected: identical frame on both trees, since `Callable
  ::call_deferred` and `Object::call_deferred` share the same engine queue).

## R9 — Module layout and commit plan

- `main_scene.rs`: stays glue-only, no pure submodule (spec's own Acceptance Scenario 4 — no
  domain decision exists beyond the type-based dispatch R2 already covers).
- `level.rs` + `level/model.rs` (pure: `GiPlan`, `gi_plan`, `pick_spawn`) — mirrors every prior
  milestone's `<module>.rs` + `<module>/model.rs` split for a module whose pure surface
  warrants its own file.
- `flying_forklift.rs` + inline `mod pure` (one function, `pick_model` — matches `bullet.rs`'s/
  `door.rs`'s V2-C precedent for a small pure surface).
- `menu.rs` + `menu/model.rs` (pure: the 15 row functions, `LoadingCmd`, `loading_step`) — the
  largest pure surface in v2 so far, its own file.

| # | Commit | Files | Gate + validation |
|---|--------|-------|--------------------|
| 1 | `main_scene: typed try_cast + connect_other scene dispatch, Callable::call_deferred instead of call_deferred("name"); closes backlog #3, #23, #24` | `main_scene.rs` | gates + headless (`main.tscn`) |
| 2 | `level: extract GiPlan, pick_spawn into level/model.rs; preloaded scenes, OnReady GI nodes, uniform add_child, async respawn, redundant randomize() removed; closes backlog #19 (level half), #20, #21, #32 (load half)` | `level/model.rs` (new), `level.rs` | gates + headless (`level.tscn`) |
| 3 | `flying_forklift: pick_model, redundant randomize() removed; closes backlog #19 (forklift half)` | `flying_forklift.rs` | gates + headless |
| 4 | `menu: declarative option table + loading_step in menu/model.rs, Callable::call_deferred for headless auto-host; closes backlog #22` | `menu/model.rs` (new), `menu.rs` | gates + headless (`menu.tscn`, auto-hosted `main.tscn`) |
| 5 | `docs: close backlog #3, #19, #20, #21, #22, #23, #24 citing this milestone's commits; #32 per the checkpoint observation; #6/#25/#29/#30/#31 post-v2 review notes; header "v2 complete on <date>"; README.md v2 Status -> Complete + summary` | `docs/v2-backlog.md`, `README.md` | none (docs-only) |

User visual checkpoints: after commit 3 (US1+US2 combined: boot → menu → play → level → Esc →
menu → play again; robot respawn; forklift model; note backlog #32's hitch observation), after
commit 4 (US3: every settings row round-trips; F11 fullscreen toggle).
