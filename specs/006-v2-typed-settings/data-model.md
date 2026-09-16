# Data Model: Milestone V2-A — Typed `Settings`

All types below except `Settings` itself live in `oxide_godot_lib/src/settings/graphics.rs`, are
plain Rust (`derive(Copy, Clone, ...)`, no `Gd<T>`/`Variant`/engine singleton), and are exercised
by `cargo test` with no Godot binary. Wire codes are cross-checked against the generated gdext
bindings in `research.md` (R1–R3).

## `GraphicsSettings`

The 15 graphics options, one field per option, `Copy + Clone + PartialEq + Debug`.

| Field | Type | Wire section/key | Wire type | Wire code(s) |
|---|---|---|---|---|
| `display_mode` | `WindowMode` (engine enum, `godot::classes::window::Mode`) | `video/display_mode` | int | engine ord, `0..=4` |
| `vsync` | `VSyncMode` (engine enum) | `video/vsync` | int | engine ord, `0..=3` |
| `max_fps` | `i32` | `video/max_fps` | int | raw; `0` = unlimited |
| `resolution_scale` | `f64` | `video/resolution_scale` | real | raw — kept `f64` end to end: the menu writes `1.0/3.0`, `1.0/1.7`, `1.0/1.3` as doubles and SC-003 requires the saved text to be byte-identical to `v1`; an `f32` field would change the double on the round trip. Converted with `as f32` only inside `apply()` for `set_scaling_3d_scale` |
| `scale_filter` | `ScaleFilter` (project enum) | `video/scale_filter` | int | `0..=5` (owns the NEAREST api-gap, R2) |
| `taa` | `bool` | `rendering/taa` | bool | — |
| `msaa` | `Msaa` (engine enum) | `rendering/msaa` | int | engine ord, `0..=4` |
| `screen_space_aa` | `ScreenSpaceAa` (engine enum) | `rendering/screen_space_aa` | int | engine ord, `0..=3` |
| `shadow_mapping` | `bool` | `rendering/shadow_mapping` | bool | — |
| `gi_type` | `GiType` (project enum) | `rendering/gi_type` | int | `Sdfgi=0, VoxelGi=1, LightmapGi=2` |
| `gi_quality` | `GiQuality` (project enum) | `rendering/gi_quality` | int | `Disabled=0, Low=1, High=2` |
| `ssao_quality` | `SsaoQuality` (project enum) | `rendering/ssao_quality` | int | `Disabled=-1, Medium=2, High=3` |
| `ssil_quality` | `SsilQuality` (project enum) | `rendering/ssil_quality` | int | `Disabled=-1, Medium=2, High=3` |
| `bloom` | `bool` | `rendering/bloom` | bool | — |
| `volumetric_fog` | `bool` | `rendering/volumetric_fog` | bool | — |

Field/key order above is the ORDER preserved on disk (`v1`-identical, per SC-003) — `to_wire`/
`from_wire` iterate in exactly this order.

### `GraphicsSettings::default_for(metalfx_supported: bool) -> Self`

Same values as `v1`'s defaults: `display_mode = EXCLUSIVE_FULLSCREEN`, `vsync = ENABLED`,
`max_fps = 0`, `resolution_scale = 1.0`, `scale_filter = MetalFxTemporal if metalfx_supported else
Fsr2`, `taa = false`, `msaa = DISABLED`, `screen_space_aa = DISABLED`, `shadow_mapping = true`,
`gi_type = VoxelGi`, `gi_quality = Low`, `ssao_quality = Medium`, `ssil_quality = Disabled`,
`bloom = true`, `volumetric_fog = true`.

### `GraphicsSettings::to_wire(&self) -> [WireValue; 15]`

Pure projection to the wire order/types above.

### `GraphicsSettings::from_wire(present: [Option<WireValue>; 15], metalfx_supported: bool) -> (Self, Vec<&'static str>)`

For each of the 15 slots: `None` (key absent from the file) → that field takes its default,
silently (matches `v1`'s merge). `Some(value)` → attempt to interpret `value` as the field's type;
success → typed field; failure (wrong `WireValue` variant, or an int outside the field's known
enum codes) → default + the field's name appended to the returned `Vec` (the caller emits one
`godot_warn!` per name — the one documented deviation, FR-009).

## Project enums

All `#[derive(Copy, Clone, Eq, PartialEq, Debug)]`, no engine derive needed (they are consumed by
`GodotConvert`/`Var` only at the `Settings` node's residual `#[var]`/`#[export]` boundary, if any
— none is currently required by a scene, see `research.md` R8).

- **`GiType`**: `Sdfgi, VoxelGi, LightmapGi` — replaces `v1`'s `Settings::GI_TYPE_*` constants.
- **`GiQuality`**: `Disabled, Low, High` — replaces `v1`'s `Settings::GI_QUALITY_*` constants.
- **`SsaoQuality`**: `Disabled, Medium, High` — kept as its own type (not merged with
  `SsilQuality`) so backlog #25 can later diverge SSAO's behavior from SSIL's without a breaking
  rename; the naming-vs-behavior quirk (`Medium` → engine `HIGH`, `High` → engine `MEDIUM`) is
  reproduced in `plan()` (R4), not in this enum's own definition.
- **`SsilQuality`**: `Disabled, Medium, High` — same shape, correct mapping (`Medium` → `MEDIUM`,
  `High` → `HIGH`), kept separate for the same reason.
- **`ScaleFilter`**: `Nearest, Bilinear, Fsr1, MetalFxSpatial, Fsr2, MetalFxTemporal` — owns the
  `Scaling3DMode::NEAREST` API gap; `to_engine(self) -> Scaling3DMode`, `from_wire(code: i64) ->
  Option<Self>` (matches `0..=5` directly, independent of the engine binding's own range).

## `ApplyPlan` (pure output of `plan(&GraphicsSettings) -> ApplyPlan`)

| Field | Type |
|---|---|
| `window_mode` | `WindowMode` |
| `vsync_mode` | `VSyncMode` |
| `max_fps` | `i32` |
| `scaling_3d_scale` | `f32` |
| `scaling_3d_mode` | `Scaling3DMode` (via `ScaleFilter::to_engine`) |
| `use_taa` | `bool` |
| `msaa_3d` | `Msaa` |
| `screen_space_aa` | `ScreenSpaceAa` |
| `disable_shadows` | `bool` (`!shadow_mapping`; `true` never re-enables — `v1`'s `FIXME` limitation, preserved) |
| `ssao` | `AoDecision<EnvironmentSsaoQuality>` |
| `ssil` | `AoDecision<EnvironmentSsilQuality>` |
| `glow_enabled` | `bool` (= `bloom`) |
| `volumetric_fog_enabled` | `bool` (= `volumetric_fog`) |

`struct AoDecision<Q> { enabled: bool, quality: Q, half_size: bool }`, generic over the engine's
two distinct quality enum types. `plan()`'s SSAO/SSIL branches (values pinned in `research.md`
R4) are the only place the naming quirk (backlog #25) and the upstream bug fix (SSAO `-1` using
`else if`) live; both are reproduced verbatim from `v1`.

`apply(plan: &ApplyPlan, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)`
(glue, in `settings.rs`) pushes every field above to the engine and additionally walks
`scene_root`'s descendants calling `Light3D::set_shadow(false)` when `disable_shadows` is true
(R5) — the `0.5, 2, 50.0, 300.0` SSAO/SSIL call arguments stay literal in this function (see
`research.md` R4 for why they are not promoted to a struct).

## `Settings` (glue node, `/root/Settings`, unchanged base `Node`)

**Transitional shape (US1, and consumers not yet migrated in US2)** — see `research.md` R7 for
the full commit-by-commit breakdown:
- `#[var] config_file: Gd<ConfigFile>` — KEPT.
- `#[func] load_settings`, `#[func] save_settings`, `#[func] apply_graphics_settings` — KEPT,
  their bodies re-parse `GraphicsSettings` from `config_file` at the boundary.
- `#[var]` exposure of `metalfx_supported` and the `GI_TYPE_*`/`GI_QUALITY_*` `#[constant]`s —
  REMOVED in US1 (no scene/script references them, confirmed by grep, R8). The
  `metalfx_supported` field itself stays, private: `default_for`/`from_wire` need it.
- `fn graphics(&self) -> GraphicsSettings` / `fn set_graphics(&mut self, GraphicsSettings)` —
  PRESENT already in US1, in transitional form (read = parse from `config_file` at each call;
  write = `to_wire()` into `config_file` via `set_value`), so the consumer commits of US2 compile
  and behave one by one before the last commit turns both into plain field access.

**Final shape (from US2's last commit on)**:
- `graphics: GraphicsSettings` — the single parsed-once model, no longer wrapped in `#[var]`.
- `fn graphics(&self) -> GraphicsSettings`, `fn set_graphics(&mut self, graphics: GraphicsSettings)`
  — plain methods, no `#[func]` (nothing calls them by name).
- `fn apply_graphics_settings(&mut self, window: Gd<Window>, environment: Gd<Environment>, scene_root: Gd<Node>)`,
  `fn save_settings(&mut self)` — kept as plain typed methods (called directly by the 5
  consumers via `Gd<Settings>`, no longer `#[func]`).
- `config_file: Gd<ConfigFile>` — PRIVATE field (no `#[var]`), the I/O buffer only: loaded once at
  `ready`, never read by consumers; `save_settings` writes the 15 wire values into this SAME loaded
  object with `set_value` and saves it, exactly as `v1` did. This preserves unknown sections/keys
  and the key order of a hand-edited file (a fresh `ConfigFile` would drop them — an undocumented
  deviation). The typed `graphics` model remains the single in-memory source of truth.
- `impl INode for Settings`: `init` builds `metalfx_supported` and the default `GraphicsSettings`;
  `ready` loads `user://settings.ini` once into `graphics`; `input` keeps the unchanged
  `toggle_fullscreen` handling.
- `CONFIG_FILE_PATH` stays a module `const` (path).

## `user://settings.ini` (wire file)

Unchanged format: sections `video`/`rendering`, 15 keys, same order and Variant types as `v1`.
Interchangeable in both directions except the one documented deviation (an out-of-range enum code
on an existing key resolves to that key's default + a logged warning on this branch; `v1` falls
into whichever `else` branch happened to match).
