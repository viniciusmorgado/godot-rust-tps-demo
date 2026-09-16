# Research: Milestone V2-A — Typed `Settings` and its consumers

Evidence gathered from gdext 0.5.5 sources (`~/.cargo/registry/src/index.crates.io-*/godot-core-0.5.5/`)
and the generated bindings for the local Godot 4.7.2 build
(`oxide_godot_core/target/debug/build/godot-core-*/out/classes/`), plus the current working tree
(`oxide_godot_lib/src/settings.rs` and the 5 consumers) and a `cargo clippy` run against it.

## R1 — Engine enums and parsing

**Decision**: parse every wire int for an engine-backed field with `EngineEnum::try_from_ord`,
never `from_ord`. A field's "known set" is the SAME set the menu actually writes (i.e. the
*real* enumerators of that field, one project enum variant per real value) — not the engine's
raw ordinal range, which can include a `MAX` sentinel that carries no meaning of its own. An
ordinal outside that set (including a bare `MAX` if it were ever written) is malformed → default
+ `godot_warn!` (FR-009).

**Evidence**:
- `EngineEnum::from_ord` panics on an unmapped ordinal; `try_from_ord` returns `Option<Self>`
  (`godot-core-0.5.5/src/obj/traits.rs:201-204`).
- Every engine enum used here (`Mode`/`WindowMode` in `window.rs:2379`, `VSyncMode` in
  `display_server.rs:8267`, `Msaa` in `viewport.rs:1679`, `ScreenSpaceAa` in `viewport.rs:1920`,
  `EnvironmentSsaoQuality`/`EnvironmentSsilQuality` in `rendering_server.rs:13934,14055`) is a
  `#[repr(transparent)] #[derive(Copy, Clone, Eq, PartialEq, Hash)] struct { ord: i32 }` generated
  from the class's real Godot constants, confirmed by reading the generated `out/classes/*.rs`.
  None of these types need a live engine or `unsafe` to construct or compare — `try_from_ord` is a
  plain integer `match`, `ord()` reads the field. They are exercisable in a plain `#[test]` with no
  Godot binary, exactly like the math builtins Principle III already names.
- Confirms `try_from_ord`'s accepted range is exactly the type's own declared constants,
  including any `MAX` — e.g. `Msaa::try_from_ord` accepts `0..=4` because `Msaa::MAX = 4`
  (`viewport.rs:1682`); `Scaling3DMode` accepts `0..=5` because `Scaling3DMode::MAX = 5`
  (`viewport.rs:1563`, see R2).

**Clarification applied to the constitution's Principle III boundary**: the principle lists
`Gd<T>`, engine singletons and Variant-family builtins as forbidden in pure logic, and names
gdext's math builtins as an allowed example because "they never cross the FFI and need no
engine." Engine enums (`WindowMode`, `Msaa`, ...) satisfy that exact same test — verified above —
so `ApplyPlan` (the pure decision output, R4) is allowed to carry them. This is a plan-time
reading of an implicit gap, not a deviation from a stated rule; flagged here for review.

**Alternatives considered**: treating the engine's full ordinal range (including `MAX`) as
"known" — rejected, because it would silently accept a sentinel that has no real meaning in `v1`
and was never a value the menu could produce, which is a worse notion of "malformed" than the one
the deviation (FR-009) is meant to catch.

## R2 — The `Scaling3DMode::NEAREST` API gap

**Decision**: `ScaleFilter` is a 6-variant project enum (`Nearest, Bilinear, Fsr1,
MetalFxSpatial, Fsr2, MetalFxTemporal`) with wire codes `0..=5` matching Godot's real
`SCALING_3D_MODE_*` constants in 4.7 (`NEAREST` did not exist as a separate mode before 4.7; the
menu already writes `5` for it — see `menu.rs:533`, `SCALING_3D_MODE_NEAREST = 5`). `ScaleFilter`
owns the gap at one spot: `to_engine(self) -> Scaling3DMode` maps `Nearest` to
`Scaling3DMode::from_ord(5)` with the `// api-gap(godot-4.7): ...` comment immediately above that
one line; `from_wire(code: i64) -> Option<ScaleFilter>` matches `0..=5` directly (it does not go
through the engine enum at all for parsing, so 4.6-vs-4.7 drift is invisible to callers).

**Evidence**: the generated `Scaling3DMode` (`viewport.rs:1512-1546`) has real variants
`BILINEAR=0, FSR=1, FSR2=2, METALFX_SPATIAL=3, METALFX_TEMPORAL=4` and a `MAX=5` sentinel;
`try_from_ord` (`viewport.rs:1561-1567`) accepts `0..=5` inclusive — `MAX` is accepted as a
"valid" ordinal by the generated code, which is exactly why `v1`'s `Scaling3DMode::from_ord(5)`
does not panic today even though Godot 4.7 assigns ordinal `5` a real meaning (`NEAREST`) that
gdext 0.5.5 (built against the 4.6 prebuilt API) has no name for. `values()`
(`viewport.rs:1578-1579`) — the engine's own "meaningful enumerators" list — already excludes
`MAX`, confirming that treating `5` as semantically empty is gdext's own reading too; `NEAREST`
existing in the running engine and not in the binding is precisely the gap.

**`docs/api-gaps.md` entry** (to be created in US3, content pinned here):

| Symbol | Introducing version | Workaround | Location |
|---|---|---|---|
| `Scaling3DMode::NEAREST` | Godot 4.7 (absent from gdext 0.5.5's prebuilt API 4.6) | `ScaleFilter::Nearest` maps to `Scaling3DMode::from_ord(5)`, reusing the `MAX` ordinal slot the 4.6 binding leaves unnamed | `oxide_godot_lib/src/settings/graphics.rs`, `ScaleFilter::to_engine()` |

**Alternatives considered**: switching the crate to `api-custom` (build bindings from the local
4.7.2 binary, which would give a real `NEAREST` constant) — rejected as an implementer default per
constitution Principle I ("changing the crate's API feature set ... is a user decision"); noted as
available if the user wants it later.

## R3 — Engine-free wire representation

**Decision**: introduce `WireValue { Int(i64), Real(f64), Bool(bool) }` and a fixed-order array of
15 `(section: &'static str, key: &'static str)` slots matching `v1`'s insertion order exactly:

`video`: `display_mode, vsync, max_fps, resolution_scale, scale_filter`
`rendering`: `taa, msaa, screen_space_aa, shadow_mapping, gi_type, gi_quality, ssao_quality, ssil_quality, bloom, volumetric_fog`

`GraphicsSettings::to_wire(&self) -> [WireValue; 15]` and
`GraphicsSettings::from_wire(present: [Option<WireValue>; 15], metalfx_supported: bool) -> (Self, Vec<&'static str>)`
(the `Vec` names which fields fell back to default because the *present* value was malformed —
as opposed to simply absent, which defaults silently per `v1`) are pure Rust, unit-tested. The
glue in `settings.rs` is the only code that touches `ConfigFile`: it reads each of the 15
`(section, key)` pairs with `has_section_key`/`get_value` into `Option<WireValue>` (by matching
the `Variant`'s type), calls `from_wire`, and — for saving — calls `to_wire` and writes each
`WireValue` back with `set_value`, preserving each key's original Variant type.

**Wire codes pinned** (cross-checked against the generated bindings, not just `v1`'s source):

| Field | Wire type | Codes |
|---|---|---|
| `display_mode` | int | `WindowMode` ord (`window.rs:2379`, `0..=4`) |
| `vsync` | int | `VSyncMode` ord (`display_server.rs:8267`, `0..=3`) |
| `max_fps` | int | raw `i64`, `0` = unlimited |
| `resolution_scale` | real | raw `f64` — stored as `f64` in the model (NOT `f32`): the menu writes `1.0/3.0`, `1.0/2.0`, `1.0/1.7`, `1.0/1.5`, `1.0/1.3`, `1.0` as doubles (`menu.rs:519-529`) and an `f32` round trip would alter the saved text for 5 of the 6 presets, breaking SC-003; `as f32` only at `set_scaling_3d_scale` in `apply()` |
| `scale_filter` | int | `ScaleFilter` wire code `0..=5` (R2) |
| `taa` | bool | — |
| `msaa` | int | `Msaa` ord (`viewport.rs:1679`, `0..=4`) |
| `screen_space_aa` | int | `ScreenSpaceAa` ord (`viewport.rs:1920`, `0..=3`) |
| `shadow_mapping` | bool | — |
| `gi_type` | int | project `GiType`: `Sdfgi=0, VoxelGi=1, LightmapGi=2` |
| `gi_quality` | int | project `GiQuality`: `Disabled=0, Low=1, High=2` |
| `ssao_quality` | int | project `SsaoQuality`: `Disabled=-1, Medium=2, High=3` — `2`/`3` cross-checked as `EnvironmentSsaoQuality::MEDIUM`/`HIGH` ords (`rendering_server.rs:13891-13934`) |
| `ssil_quality` | int | project `SsilQuality`: `Disabled=-1, Medium=2, High=3` — same cross-check against `EnvironmentSsilQuality` (`rendering_server.rs:14012-14055`) |
| `bloom` | bool | — |
| `volumetric_fog` | bool | — |

**Alternatives considered**: passing `ConfigFile` (or a `HashMap<String, Variant>` mirror of it)
into the pure layer — rejected, `ConfigFile`/`Variant` are engine-backed and forbidden in pure
code by Principle III; a `HashMap` mirror would also lose the fixed key ORDER that SC-003 requires
to survive a round trip.

## R4 — Apply decomposition

**Decision**: adopt the shape already specified in the spec's FR-005 verbatim: a pure
`fn plan(settings: &GraphicsSettings) -> ApplyPlan` and a glue
`fn apply(plan: &ApplyPlan, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)`.
`ApplyPlan` carries: `window_mode: WindowMode`, `vsync_mode: VSyncMode`, `max_fps: i32`,
`scaling_3d_scale: f32`, `scaling_3d_mode: Scaling3DMode` (via `ScaleFilter::to_engine`),
`use_taa: bool`, `msaa_3d: Msaa`, `screen_space_aa: ScreenSpaceAa`, `disable_shadows: bool`,
`ssao: AoDecision<EnvironmentSsaoQuality>`, `ssil: AoDecision<EnvironmentSsilQuality>`,
`glow_enabled: bool`, `volumetric_fog_enabled: bool`, where
`struct AoDecision<Q> { enabled: bool, quality: Q, half_size: bool }` (generic — SSAO and SSIL
share the shape but not the type, since `RenderingServer`'s two setters take distinct engine
enums). `plan()` reproduces `v1` exactly: SSAO `Disabled → { enabled: false, .. }` (kept `else
if`, upstream-bug-fix comment moves with it), `Medium → { enabled: true, quality: HIGH,
half_size: false }`, `High → { enabled: true, quality: MEDIUM, half_size: true }` (quirk,
backlog #25, untouched); SSIL `Disabled → off`, `Medium → { MEDIUM, false }`, `High → { HIGH,
true }` (correct, `elif` already in `v1`). `0.5, 2, 50.0, 300.0` stay literal call arguments in
`apply()` (constitution Principle III: "tuning parameters... belong to the type" — these four are
`RenderingServer` call arguments, not tunable by this milestone's scope, so they are passed
through unchanged rather than promoted to a struct nobody varies yet).

**Rationale**: this is the direct execution of Principle III's snapshot→step→apply shape for the
one method in the crate that most needed it (12-line-deep `if`/`else if` chains reading
`ConfigFile` inline). Unit tests on `plan()` cover all 3×3 SSAO/SSIL states plus the boundary
values, with no engine involved.

## R5 — Shadow-disabling call

**Decision**: replace the dynamic `scene_root.propagate_call_ex("set").args(&varray!
["shadow_enabled", false])` with a typed recursive walk from `scene_root` that calls
`Light3D::set_shadow(false)` on every `Light3D`-castable descendant (`try_cast::<Light3D>` at
each node, recursing into `get_children_ex().include_internal(true)` regardless of cast result —
`Node::propagate_call` iterates `data.children`, which includes internal children, so the walk
must include them too for strict equivalence). This is `apply()`'s only
loop; it runs once per `apply_graphics_settings` call (menu open, Apply, level `ready`), not
per-frame, so Principle III's "resolve once" is satisfied trivially.

**Evidence for equivalence**:
- The generated `Light3D` binding exposes `set_shadow(&mut self, enabled: bool)` /
  `has_shadow(&self) -> bool` for the property Godot calls `shadow_enabled`
  (`out/classes/light_3d.rs:62-77`) — `v1`'s `propagate_call("set", ["shadow_enabled", false])`
  is exactly this setter, called dynamically on every node (a no-op on anything that isn't a
  `Light3D`, since `Object::set` on an unknown property is silently ignored).
- `level.rs`/`menu.rs` pass `self.to_gd()` (the `Level`/`Menu` node itself, not the tree root) as
  `scene_root` (`level.rs:45`, `menu.rs:607`), so the walk starts from the same node
  `propagate_call` already started from and reaches the same subtree, including `Light3D`s that
  live inside instanced sub-scenes (`red_robot.tscn`, `flying_forklift.tscn`, `bullet.tscn`),
  since those are ordinary scene children once instanced, not "internal" engine-only children.
- Direct light nodes were found by grepping the project's `.tscn` files: `menu.tscn` has two
  `SpotLight3D` at its own top level (`menu.tscn:123,127`); `level.tscn` has none directly (its
  lights come from the instanced enemy/vehicle/bullet sub-scenes), which the recursive walk
  reaches identically to `propagate_call`.
- The parity harness (R10) additionally dumps `shadow_enabled`/`has_shadow()` for every `Light3D`
  found in the loaded scene after `apply_graphics_settings(shadow_mapping = false)` on both
  branches, as a belt-and-suspenders check beyond this reasoning.

**Alternatives considered**: keeping `propagate_call` as a listed residual case — rejected, since
the typed walk is straightforward, provably equivalent, and removes one more by-name dynamic call;
no justification for keeping it would survive review.

## R6 — Consumer access to `Settings`

**Decision**: each of the 5 consumers stores `settings: OnReady<Gd<Settings>>` initialized via the
field attribute `#[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]`
(all 5 use the derived `#[class(init, ...)]`, so there is no hand-written `init()` to put it in), and
reads it only from `ready()` onward (never in `init()`, since the tree — and therefore the
autoload — is not available yet). `bullet.rs` resolves it in its own `ready()`, not inside
`explode()`, even though today's dynamic access happens inside `explode`.

**Evidence**: gdext 0.5.5 ships a purpose-built helper for exactly this,
`godot::tools::get_autoload_by_name::<T>(name) -> Gd<T>` (and a fallible
`try_get_autoload_by_name`), in `godot-core-0.5.5/src/tools/autoload.rs:46-130`. It resolves
`/root/{name}` via the scene tree, `try_cast`s to `T` with a descriptive error on mismatch, and
caches the result in a thread-local map keyed by name so repeat calls (e.g. from 5 different
consumers) are O(1) after the first. `OnReady::new(closure)` (its "automatic mode") is
auto-initialized by the generated `init()`/`ready()` glue *before* `ready()` runs, in declaration
order (`godot-core-0.5.5/src/obj/on_ready.rs:27-31`), so accessing `self.settings` anywhere from
`ready()` onward — including inside a later `explode()` call — is safe and requires no `unwrap()`.
This is strictly better than a hand-rolled `#[init(node = "/root/Settings")] OnReady<Gd<Settings>>`
(which also works, since `OnReady::from_node` accepts an absolute `NodePath`) because it names the
autoload the same way `project.godot`'s `[autoload]` key does, rather than duplicating the
`/root/...` path string in 5 places, and shares gdext's own cache instead of each consumer's
`OnReady` doing a fresh tree lookup.

**Read/write API** on `Settings` (final shape, after US2's last commit — see R7 for the
transitional shape during US1):
- `fn graphics(&self) -> GraphicsSettings` (the model is `Copy`; consumers that only read take a
  cheap copy instead of holding a borrow across frames).
- `fn set_graphics(&mut self, graphics: GraphicsSettings)` (used by `menu.rs`'s Apply handler,
  which builds a full copy from the button states and hands it back).
- `fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)`
  and `fn save_settings(&mut self)` keep their names (called by name from GDScript nowhere after
  this milestone, but kept as plain typed methods — no `#[func]` needed once US2's last commit
  lands, per FR-007/FR-014).

**Alternatives considered**: a free function `Settings::from_tree(&Gd<SceneTree>) -> Gd<Settings>`
duplicating what `get_autoload_by_name` already does — rejected, redundant with a library-provided
helper that is more idiomatic and already cached.

## R7 — Transitional contract mechanics

Restating precisely what the spec's top-of-file contract requires, so tasks can carry it without
re-deriving it:

**During US1** (one commit, `settings.rs` only):
- `GraphicsSettings`, its enums, `WireValue`/wire conversion, `plan()`/`ApplyPlan` and their unit
  tests land in `settings/graphics.rs`.
- `#[var] config_file` and `#[func]` on `load_settings`, `save_settings`, `apply_graphics_settings`
  ALL REMAIN on `Settings` exactly as today — the 5 consumers have not moved yet and still call
  them dynamically by name.
- `ready()` still calls `load_settings()`, which still merges defaults into `config_file` in
  memory (unchanged behavior) — the typed model is NOT yet the single source of truth.
- `apply_graphics_settings` and `save_settings` internally do: read `config_file` → build a wire
  array → `GraphicsSettings::from_wire(...)` → `plan()` → `apply()` (for
  `apply_graphics_settings`); `save_settings` still just saves `config_file` as today (nothing to
  change there yet, since nothing new is writing anywhere else). This "parse at the boundary,
  every call" is deliberately less efficient than the final shape — it is the explicitly-allowed
  intermediate step of Principle I ("an intermediate step inside a user story may refactor with
  the logic still in the glue code").
- `#[constant] GI_TYPE_*`/`GI_QUALITY_*` and `#[var] metalfx_supported` ARE removed in this commit
  (confirmed unreferenced by any `.tscn`/`.gd` — see R8).

**During US2** (one commit per consumer, migrating reads/writes to `graphics()`/`set_graphics()`
and to the still-present typed `apply_graphics_settings`/`save_settings` methods — called
directly now, no longer through `.call("...", ...)`):
- `flying_forklift.rs`, `bullet.rs`, `main_scene.rs`: read `settings.bind().graphics()` field(s)
  instead of `config_file.get_value(...)`.
- `level.rs`: same, plus the GI `match`.
- `menu.rs`: reads populate the settings-menu rows from `graphics()`; the Apply handler builds a
  `GraphicsSettings` from the button states and calls `set_graphics(...)`, then
  `apply_graphics_settings(...)` and `save_settings()` — all typed method calls now, no
  `.call("name", ...)`.

**Last commit of US2** (after all 5 consumers are migrated):
- `ready()` changes to parse the model exactly once (`GraphicsSettings::from_wire` from the
  loaded `ConfigFile`, stored in a field) and stops calling the old `load_settings`.
- `#[var] config_file`, and `#[func]` on all three methods, are removed — nothing calls them by
  name any more (grep confirms — FR-014).
- `save_settings`/`apply_graphics_settings` become plain methods operating on the stored model.
  The `ConfigFile` stays as a PRIVATE field (no `#[var]`) used only as the I/O buffer: loaded once
  at `ready`, and `save_settings` writes the 15 wire values into that same loaded object with
  `set_value` before saving — exactly `v1`'s mechanics. A `ConfigFile` created fresh at save time
  would drop unknown sections/keys and the key order of a hand-edited file, an undocumented
  deviation; keeping the loaded object preserves them as `v1` does.

## R8 — Module layout and the `.tscn`/`.gd` grep

**Decision**: `oxide_godot_lib/src/settings.rs` becomes the glue file (`Settings` struct,
`impl INode for Settings`, `#[godot_api] impl Settings` for the residual API) with `mod graphics;`
at the top; `oxide_godot_lib/src/settings/graphics.rs` holds `GraphicsSettings`, the five project
enums, `WireValue`, `to_wire`/`from_wire`, `ApplyPlan`, `plan()`, and `#[cfg(test)] mod tests`
beside them. `CONFIG_FILE_PATH` stays a `const` in `settings.rs` (a path, per Principle III's
carve-out).

**Grep result** (recorded per FR-007, run against the current working tree):
`grep -rn "GI_TYPE_\|GI_QUALITY_\|metalfx_supported\|config_file" --include='*.tscn' --include='*.gd' oxide-godot/` → **zero matches**. No `.tscn` or `.gd` in the project references
`#[constant] GI_TYPE_*`/`GI_QUALITY_*` or `#[var] metalfx_supported`/`config_file` by name, so
removing them (US1 for the constants/`metalfx_supported`; last commit of US2 for `config_file`,
once the 5 consumers no longer call it dynamically) breaks nothing at the scene level.

**`cargo test` link check**: `cargo test` in `oxide_godot_core` already links and runs today (0
tests, exit 0) against the `cdylib` crate type — confirmed by running it — so no Cargo.toml
change is needed to add a `#[cfg(test)]` module; `godot`'s test-support feature is not required
because the new tests touch no engine type.

## R9 — Clippy: the 9 current warnings

All 9 confirmed by running `cargo clippy --quiet` against the current working tree; listed with
lint, fix, and a behavior-preservation note. **Ordering**: Principle III makes `cargo clippy`
with no warnings a gate before EVERY commit, so this fix is the FIRST commit of the milestone
(Setup phase), not the last — the 9 warnings are pre-existing and mechanical, and fixing them
first lets every subsequent commit satisfy the gate literally. Note that US2's `menu.rs` commit
would by itself remove the `menu.rs:379,381` site (with a typed `ScaleFilter` the
`else if self.metalfx_supported` fallback becomes dead), so if the clippy commit ran later it
would only fix "whatever remains"; running it first makes the table below exact.

| Location | Lint | Fix | Behavior change? |
|---|---|---|---|
| `blast.rs:30` | `collapsible_if` | merge into `if let Some(camera) = &self.camera && camera.is_instance_valid()` (stable let-chains, edition 2024) | None — mechanical, clippy's own suggested rewrite |
| `bullet.rs:50` | `collapsible_if` | merge into `if let Some(mut collider) = collider && collider.has_method("hit")` | None — mechanical |
| `red_robot.rs:325` | `cmp_owned` | `body.get_name() == StringName::from("Target")` → `body.get_name() == "Target"` (`StringName: PartialEq<&str>`) | None — same comparison, avoids an allocation |
| `red_robot.rs:391` | `collapsible_if` | merge into `if hit_player && let Ok(player) = player.try_cast::<Player>()` | None — mechanical; do NOT touch anything else in this branch (the caution in the milestone brief: this is a 1:1 syntax rewrite, not a restructuring of the robot's state machine) |
| `menu.rs:28` | `cmp_owned` | `get_current_rendering_driver_name() == GString::from("metal")` → `== "metal"` | None |
| `menu.rs:213` | `cmp_owned` | `get_name() == GString::from("headless")` → `== "headless"` | None |
| `menu.rs:379,381` | `if_same_then_else` | merge `else if scale_filter == METALFX_TEMPORAL.ord() as i64 { X } else if self.metalfx_supported { X }` into one `else if (...) || self.metalfx_supported { X }` | None — both conditions are side-effect-free reads; the truth table of the merged condition is identical, only the redundant duplicate action is removed |
| `main_scene.rs:23` | `cmp_owned` | `get_name() == GString::from("headless")` → `== "headless"` | None |
| `settings.rs:31` | `cmp_owned` | `get_current_rendering_driver_name() == GString::from("metal")` → `== "metal"` | None (this line moves into `graphics.rs`'s default-computation glue anyway, per R8) |

No `#[allow]` is needed anywhere; every fix above is either clippy's own mechanical suggestion or
a provably truth-table-preserving merge.

## R10 — Parity harness design

**Decision**: a scratch GDScript scene (`zz_settings_parity.tscn` + `.gd`, never committed to
either branch's normal scene set — lives only in the harness run, deleted afterward or kept under
a `zz_` prefix that both branches' `.gitignore`/review conventions treat as scratch) is run twice:
once against a `v1` worktree (`git worktree add ../oxide-godot-v1 v1`, built with its own
`cargo build`) and once against this branch, headless, with output diffed. Both checkouts share
the project name (`config/name="Third-Person Shooter Demo"`), hence the SAME `user://` directory
(`~/.local/share/godot/app_userdata/Third-Person Shooter Demo/`); the runs would overwrite each
other's `settings.ini` and dump. Each run therefore gets its own `XDG_DATA_HOME` (Godot resolves
`user://` under it on Linux): `XDG_DATA_HOME=/tmp/parity-v1` for the worktree run,
`XDG_DATA_HOME=/tmp/parity-v2` for this branch — the dumps and ini files are then diffed across
those two trees. Three cases:

**(a) Boot / first Apply**: delete `user://settings.ini` if present, run `main.tscn` headless,
assert the file does not exist; drive the menu's Apply once with no changes from defaults; copy
the resulting file from each branch's user dir and diff — must be byte-identical.

**(b) Every menu option**: for each of the 15 rows, for each button, the harness scene drives the
SAME path a player uses — `pressed = true` on the target button node, then invoke
`_on_apply_pressed()` — on BOTH branches (not `config_file.set_value` directly on `v2`, since
`#[var] config_file` is gone after US2's last commit; driving the actual menu buttons is also
closer to a real regression test and works unchanged on `v1`). After each Apply, dump: the ini
bytes; `get_window().mode`; `DisplayServer.window_get_vsync_mode()`; `Engine.max_fps`; the
viewport's `scaling_3d_scale`/`scaling_3d_mode`; `use_taa`; `msaa_3d`; `screen_space_aa`;
`environment.ssao_enabled`/`ssil_enabled`/`glow_enabled`/`volumetric_fog_enabled`; and
`shadow_enabled` (via `has_shadow()`) of every `Light3D` found under the level root. Diff all of
it across branches — must match exactly.

**Coverage gap, stated explicitly**: the SSAO/SSIL *quality* constant (`MEDIUM`/`HIGH`) and
`half_size` passed to `RenderingServer.environment_set_ssao_quality`/`environment_set_ssil_quality`
have no getter on `RenderingServer` — Godot only lets you set them, not read them back. Their
parity is therefore NOT covered by the runtime harness; it is covered by (1) the `plan()` unit
tests (R4) asserting the exact `AoDecision` produced for each of the 3 SSAO/3 SSIL states, and (2)
a manual side-by-side code review of the constants against `v1`'s literals. This gap is
acceptable and should be stated in the plan/tasks, not silently assumed away.

**(c) Malformed value**: hand-edit `gi_type = 7` into `user://settings.ini` on the `v2` side only,
boot, assert the console log contains the `godot_warn!` line and that the effective GI setup is
the DEFAULT (`VoxelGI`, per `v1`'s default); on `v1`, the same file makes `gi_type` fall into
whichever `else` branch the original's chain resolves to — recorded as a known difference, not
diffed against `v2` (this is the one documented deviation, not a parity requirement).

Headless validation (`CLAUDE.md`'s existing recipe: import, then run each affected `.tscn`) runs
after every commit of every user story, independent of the harness.
