# Research: GDScript → gdext 0.5.5 mapping (Milestone A)

**Phase**: v1 raw port. **Date**: 2026-09-15.

Sources checked (all local, none from memory):

- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/godot-macros-0.5.5/src/` (attributes
  `#[class]`, `#[init]`, `#[export]`, `#[rpc]`);
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/godot-core-0.5.5/src/` (`OnReady`,
  typed signals, `Callable`, `Variant`, `Basis`, `AsArg`);
- generated bindings: `oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/`
  (`classes/*.rs`, `utilities.rs`) — the hash changes on every clean rebuild; use `ls -d .../godot-core-*/out`;
- engine behavior verified empirically in Godot 4.7.2 headless (§E).

Toolchain confirmed: rustc/cargo 1.98.1; `godot` 0.5.5 with default prebuilt API **4.6**
(`godot-bindings-0.5.5/src/import.rs:72` — `pub use gdextension_api::version_4_6 as prebuilt;`
when no `api-4-x` feature is active). Do not change.

## Decisions

### D1 — Class skeleton

**Decision**: for each script,

```rust
#[derive(GodotClass)]
#[class(init, base=<Base>)]
struct <Name> { base: Base<<Base>>, /* fields */ }

#[godot_api]
impl I<Base> for <Name> { fn ready(&mut self) {..} fn process(&mut self, delta: f64) {..} fn input(&mut self, event: Gd<InputEvent>) {..} }

#[godot_api]
impl <Name> { #[func] fn ... }
```

Virtual signatures (bindings `classes/node.rs:46,54,62`): `fn ready(&mut self)`,
`fn process(&mut self, delta: f64)`, `fn input(&mut self, event: Gd<InputEvent>)`.

Existing interface traits (`classes/*.rs:27`): `ILabel`, `ICpuParticles3D`, `INode3D`,
`ICamera3D`, `IMultiplayerSynchronizer`. **Mind the gdext spelling**: the class is `CpuParticles3D`
(not `CPUParticles3D`) — `base=CpuParticles3D`, `impl ICpuParticles3D`, `Gd<CpuParticles3D>`.

Name registered in Godot = struct name (no `rename`). `player.gd:26` requires exactly
`PlayerInputSynchronizer`.

**Rationale**: it is the only supported skeleton; `#[class(init)]` generates the constructor with
`Default`/`#[init(val)]`/`OnReady`, exactly what `var x = ...` does in GDScript.
**Alternatives**: `#[class(no_init)]` + manual `fn init(base)` — more code with no gain; rejected.

### D2 — `@onready var x = $Path` → `OnReady`

**Decision**: `#[init(node = "Path")] x: OnReady<Gd<T>>`. Confirmed in
`godot-macros-0.5.5/src/class/derive_godot_class.rs:765` (`OnReady::from_node(path)`);
`OnReady::from_node(path: impl AsArg<NodePath>)` in `godot-core/src/obj/on_ready.rs:146`.
Resolved automatically before `ready()`. Access via `Deref`/`DerefMut`
(`on_ready.rs:263,290`): `self.light_rays.look_at(..)`.

Uses: `part_disappear` → `mini_blasts: OnReady<Gd<CpuParticles3D>>` (`"MiniBlasts"`);
`blast` → `light_rays: OnReady<Gd<CpuParticles3D>>` (`"LightRays"`),
`animation_player: OnReady<Gd<AnimationPlayer>>` (`"AnimationPlayer"`).

Exception: `blast.gd`'s `@onready var camera = get_tree().get_root().get_camera_3d()` is not a
`$Path` → field `camera: Option<Gd<Camera3D>>` filled in at the start of `ready()` (D8).

**Alternatives**: `get_node_as::<T>("Path")` inside `ready()` — works, but `OnReady` is the
direct translation of `@onready`; rejected for being less literal.

### D3 — `@export var x: <NodeType>` filled in by `node_paths` → `Option<Gd<T>>`

**Decision**: `#[export] camera_base: Option<Gd<Node3D>>` etc. `impl<T> Export for Option<Gd<T>>`
in `godot-core/src/obj/gd.rs:1249` (node class hint via `as_node_class()`), therefore Godot
writes/reads it as NodePath and resolves the node when instantiating the scene — the `node_paths` of
`player.tscn:343,346–351` remain valid. Access: `self.camera_base.as_mut().unwrap()` /
`.as_ref().unwrap()` — in the original, a `null` here is also a runtime error; the `unwrap` reproduces
that (panic caught by gdext and printed as an error, without bringing down the game).

The six fields: `camera_animation: Option<Gd<AnimationPlayer>>`, `crosshair: Option<Gd<TextureRect>>`,
`camera_base: Option<Gd<Node3D>>`, `camera_rot: Option<Gd<Node3D>>`, `camera_camera: Option<Gd<Camera3D>>`,
`color_rect: Option<Gd<ColorRect>>`.

**Alternatives**: `OnEditor<Gd<T>>` (`gd.rs:1303`) — exports as "required" and panics if not
filled in; semantically close, but the command input decided on `Option`, and `Option` is the
1:1 representation of a GDScript reference that can be `null`. Rejected. Candidate item for the
v2 backlog (§Backlog).

### D4 — `@export var aiming: bool` etc. → value `#[export]`

**Decision**: `#[export] aiming: bool`, `#[export] shoot_target: Vector3`, `#[export] motion: Vector2`,
`#[export] shooting: bool`, `#[export] jumping: bool`. `#[export]` generates getter+setter, so
`player.gd:114` (`player_input.jumping = false`) keeps working, and the `MultiplayerSynchronizer`
replicates `aiming/motion/shooting/shoot_target` by name (`player.tscn:42–53`). `jumping` remains
exported but outside replication, as in the original (comment "handled via RPC").

Internal fields without annotation: `toggled_aim: bool`, `aiming_timer: f32` (D13 for the type).

**Alternatives**: `#[var]` (without export) — replication would work the same, but it would change the visibility in the
inspector relative to the original (`@export`). Rejected.

### D5 — `@rpc("call_local") func jump()` → `#[rpc]`

**Decision**: `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.jumping = true; }`
(inside `#[godot_api] impl PlayerInputSynchronizer`; `#[rpc]` implies `#[func]` —
`godot-macros/src/lib.rs:1055`). Accepted keys (`inherent_impl.rs:702–716`): `any_peer|authority`,
`reliable|unreliable|unreliable_ordered`, `call_local|call_remote`, `channel = N`.
Trigger: `self.base_mut().rpc("jump", &[])` — `Node::rpc(&mut self, method: impl AsArg<StringName>, varargs: &[Variant]) -> Error`
(`classes/node.rs:1480`).

**Correction relative to the command input**: the input suggested `reliable`. The default of
`@rpc` in GDScript is `mode="authority", sync="call_remote", transfer_mode="unreliable"`, and
`@rpc("call_local")` only changes `sync`. The gdext `#[rpc]` default is the same (table in
`godot-macros/src/lib.rs:1046–1051`: `authority`, `call_remote`, `unreliable`). Using `reliable`
would change the network behavior relative to the original — forbidden in v1. Declaring the three keys
explicitly (`authority, call_local, unreliable`) documents the intent and is identical to
`#[rpc(call_local)]`.

**Alternatives**: `#[rpc(config = CONST)]` — equivalent, more verbose; rejected.
`reliable` — changes behavior; rejected.

### D6 — `await get_tree().create_timer(t).timeout` → typed signal

**Decision**:

```rust
let timer = self.base().get_tree().create_timer(0.2);          // Gd<SceneTreeTimer>
timer.signals().timeout().connect_other(&*self, |this: &mut PartDisappear| { ... });
```

- `Node::get_tree(&self) -> Gd<SceneTree>` (type-safe replacement,
  `godot-core/src/classes/type_safe_replacements.rs:84`);
- `SceneTree::create_timer(&mut self, time_sec: f64) -> Gd<SceneTreeTimer>` (`classes/scene_tree.rs:316`);
- `Gd::signals()` (`obj/gd.rs:1055`) → `timeout()` (`classes/scene_tree_timer.rs:118`), parameter
  tuple `()`;
- `TypedSignal::connect_other(&self, object: &impl ObjectToOwned<OtherC>, method: F)`
  (`godot-core/src/signal/typed_signal.rs:316`) accepts `&Gd<OtherC>` **or** `&OtherC` when
  `OtherC` is a user class with `Base` — inside `ready(&mut self)` pass `&*self`.
- The connection uses a callable **linked** to the receiving object
  (`inner_connect_godot_fn` → `bound.linked_callable(..)`, `typed_signal.rs:205–213`;
  `Callable::from_linked_fn` doc, `builtin/callable.rs:161–166`: "automatically invalidated by
  Godot when a linked object is freed"). Hence, if the effect is freed before the timer fires,
  the callable is invalidated by Godot and the closure does not run — no panic. Equivalent to the edge
  case in the spec ("must not produce new errors").

Chaining in `part_disappear` (two sequential `await`s): inside the first closure,
`this.base_mut().set_emitting(true)`, then create the second timer with
`this.base().get_tree().create_timer(this.base().get_lifetime() * 2.0)` and
`connect_other(&*this, |this2| this2.base_mut().queue_free())`. `CpuParticles3D::get_lifetime(&self) -> f64`
(`classes/cpu_particles_3d.rs:308`); `set_emitting(&mut self, bool)` (`:173`); `Node::queue_free`
(`classes/node.rs:1299`).

The first statement of `_ready` (`$MiniBlasts.emitting = true`) runs before any timer:
`self.mini_blasts.set_emitting(true)`.

**Alternatives**: (a) `Callable::from_object_method(&gd, "_on_timeout")` + `#[func]` — same
invalidation semantics, but adds method names that do not exist in the original; rejected.
(b) `connect_self` — only for the object's own signals; N/A. (c) `godot::task::spawn` +
`.to_future()` — experimental async API; rejected (less predictable, no gain).
(d) `builder().flags(ONE_SHOT)` — `SceneTreeTimer` is one-shot by nature; unnecessary.

### D7 — `await $AnimationPlayer.animation_finished` → typed signal

**Decision**: in `ready()`:
`self.animation_player.signals().animation_finished().connect_other(&*self, |this: &mut Blast, _anim_name: StringName| this.base_mut().queue_free());`
Signal inherited from `AnimationMixer` (`classes/animation_mixer.rs:863`, tuple `(StringName,)`
— `:935`). `impact_effect.tscn` has `autoplay = "blast"`, the signal arrives once. Linked callable
(D6) → safe if the effect is freed earlier.

**Alternatives**: same as D6.

### D8 — Orient toward the camera every frame (`blast`)

**Decision**: field `camera: Option<Gd<Camera3D>>`; in `ready()` (first line, before connecting
the signal, respecting the order `@onready` → `_ready`):
`self.camera = self.base().get_tree().get_root().unwrap().get_camera_3d();`
— `SceneTree::get_root(&self) -> Option<Gd<Window>>` (`classes/scene_tree.rs:138`);
`Viewport::get_camera_3d(&self) -> Option<Gd<Camera3D>>` (`classes/viewport.rs:1047`).
In `process()`: `if let Some(cam) = &self.camera { if cam.is_instance_valid() { let origin = cam.get_global_transform().origin; self.light_rays.look_at(origin); } }`
— `Gd::is_instance_valid(&self) -> bool` (`obj/gd.rs:332`); `Node3D::look_at(&mut self, target: Vector3)`
(`classes/node_3d.rs:851`); `Transform3D { basis, origin }` public fields
(`builtin/matrices/transform3d.rs:58,61`).

`get_root().unwrap()`: the original also assumes the root is present; N/A outside the tree.

### D9 — Singletons and utilities (`debug`)

All via the `Singleton` trait (`classes/engine.rs:465`: `fn singleton() -> Gd<Self>`):

| GDScript | gdext 0.5.5 | Source |
|---|---|---|
| `Input.is_action_just_pressed(&"toggle_debug")` | `Input::singleton().is_action_just_pressed("toggle_debug")` — `&str: AsArg<StringName>` (`meta/args/as_arg.rs:523–539`) | `classes/input.rs:118` |
| `Input.get_action_strength(&"x")` → `f32` | `get_action_strength(&self, action) -> f32` | `input.rs:202` |
| `Input.is_action_pressed` / `is_action_just_released` | same, `-> bool` | `input.rs:97,139` |
| `Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)` | `Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED)` | `input.rs:565,1240` |
| `Engine.get_frames_per_second()` | `Engine::singleton().get_frames_per_second() -> f64` | `classes/engine.rs:135` |
| `DisplayServer.window_get_vsync_mode()` (truthy = ≠ 0) | `DisplayServer::singleton().window_get_vsync_mode() != godot::classes::display_server::VSyncMode::DISABLED` | `display_server.rs:2041,8235` |
| `OS.get_static_memory_usage()` | `Os::singleton().get_static_memory_usage() -> u64` | `classes/os.rs:699` |
| `visible = not visible` | `let v = self.base().is_visible(); self.base_mut().set_visible(!v);` | `canvas_item.rs:66,75` |
| `text = ...` | `self.base_mut().set_text(&s)` (`impl AsArg<GString>`; `&String` accepted) | `label.rs:254` |
| `multiplayer` | `self.base().get_multiplayer() -> Option<Gd<MultiplayerApi>>` | `node.rs:1369` |
| `multiplayer.get_unique_id()` | `-> i32` | `multiplayer_api.rs:53` |
| `multiplayer.multiplayer_peer is OfflineMultiplayerPeer` | `get_multiplayer_peer() -> Option<Gd<MultiplayerPeer>>` → `.map(\|p\| p.try_cast::<OfflineMultiplayerPeer>().is_ok())`; `online = !matches!(.., Some(true))` (`null` peer ⇒ `is` false ⇒ online, as in the original) | `multiplayer_api.rs:34`; `obj/gd.rs:538` |
| `get_multiplayer_authority()` | `self.base().get_multiplayer_authority() -> i32` | `node.rs:1350` |
| `set_process(false)` / `set_process_input(false)` | same | `node.rs:779,835` |
| `color_rect.hide()` | `CanvasItem::hide` | `canvas_item.rs:105` |

**Text formatting (verified empirically, §E)**:
- `str(Engine.get_frames_per_second())` → `"60.0"` (Godot appends `.0` to integer floats).
  Faithful translation: `Variant::from(fps).stringify()` (`builtin/variant/mod.rs:302`) — delegates to
  the engine's own `str()`, identical by construction. The alternative `format!("{:.1}", fps)` is only
  equal because `_fps` is always an integer; rejected for not being `str()`.
- `"%3.2f" % x` → `format!("{:3.2}", x)` — identical output (`117.74`, `0.50`).
- `str(int)` → `format!("{}", id)` — identical for integers.
- Concatenation: build a Rust `String` and `set_text(&text)` once per frame (the original does
  `text +=` four times; the final result is the same — a single assignment is not an optimization, it is
  the natural way to build the string in Rust without a helper).

### D10 — Noise (`camera_noise_shake`)

**Decision**: `noise: Gd<FastNoiseLite>` with `#[init(val = FastNoiseLite::new_gd())]`;
`noise_seed: i32` with `#[init(val = (randi() as i32))]` — `godot::global::randi() -> i64`
(`utilities.rs:802`); `FastNoiseLite::set_seed(&mut self, seed: i32)` (`fast_noise_lite.rs:156`),
so the `as i32` reproduces the `int → int32` conversion that Godot does when assigning to `seed`.
`set_fractal_octaves(&mut self, i32)` (`:228`), `set_fractal_lacunarity(&mut self, f32)` (`:246`),
`Noise::get_noise_1d(&self, x: f32) -> f32` (inherited, `classes/noise.rs:25`).
`noise_seed + 1` / `+ 2`: `wrapping_add` is not necessary (GDScript overflow semantics is
wrap in i64; here `i32` + 2 only overflows for `randi()` ≥ 2³¹−2, ≈ 1 in 10⁹ — using `wrapping_add`
anyway is free and avoids a panic in debug; **decision**: use `wrapping_add`).

`start_rotation: Vector3` captured in `ready()` (`self.base().get_rotation()`,
`node_3d.rs:247`); the GDScript initializer `var start_rotation = rotation` runs before the
node is in the tree and is overwritten in `_ready` — the value that matters is the one from `_ready`; a field with
`Default` (zero) and assignment in `ready()` reproduces the result.

Constants: `const SPEED: f32 = 1.0; DECAY_RATE: f32 = 1.5; MAX_YAW: f32 = 0.05; MAX_PITCH: f32 = 0.05; MAX_ROLL: f32 = 0.1; MAX_TRAUMA: f32 = 1.2;` — `f32` because they feed `Vector3`/`real` (D13).

`add_trauma`: `#[func] fn add_trauma(&mut self, amount: f64)` — GDScript `float` parameters
arrive as `f64`; `self.trauma = (self.trauma + amount as f32).min(MAX_TRAUMA)`.
GDScript `minf`/`maxf`/`clampf` → `f32::min`/`max`/`clamp` (the Godot utilities exist in
`utilities.rs:702,732,762` but in `f64`; using the native methods avoids conversions — it is not
abstraction).

`rotation = start_rotation + Vector3(pitch, yaw, roll)` → `self.base_mut().set_rotation(self.start_rotation + Vector3::new(pitch, yaw, roll))` (`node_3d.rs:238`).

### D11 — Raycast (`player_input`) — quirk FR-017 preserved

**Decision**:

```rust
let params = PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0)
    .collision_mask(0b11)
    .exclude(&array![Rid::Invalid])
    .done()
    .unwrap();
let col = space_state.intersect_ray(&params);   // VarDictionary
if col.is_empty() { self.shoot_target = ray_from + ray_dir * 1000.0; }
else { self.shoot_target = col.get("position").unwrap().to::<Vector3>(); }
```

- `create_ex(from, to) -> ExCreate` with `.collision_mask(u32)`, `.exclude(&Array<Rid>)`, `.done() -> Option<Gd<..>>`
  (`physics_ray_query_parameters_3d.rs:140,378,384,390`);
- `Rid::Invalid` (`builtin/rid.rs:44`); `array![]` (`builtin/collections/array.rs:1545`);
- `Node3D::get_world_3d() -> Option<Gd<World3D>>` (`node_3d.rs:523`) on the **parent** (`get_parent().unwrap().cast::<Node3D>()`);
  `World3D::get_direct_space_state() -> Option<Gd<PhysicsDirectSpaceState3D>>` (`world_3d.rs:219`);
  `intersect_ray(&mut self, params) -> VarDictionary` (`physics_direct_space_state_3d.rs:46`;
  `VarDictionary = Dictionary<Variant, Variant>`, `dictionary.rs:127`; `is_empty` `:274`, `get` `:183`).
- `camera_camera.project_ray_origin(Vector2) -> Vector3` / `project_ray_normal` (`camera_3d.rs:191,171`);
  `crosshair.get_position() + crosshair.get_size() * 0.5` (`control.rs:582,591`).

**Verified in 4.7.2 (§E)**: `Array([self], TYPE_RID, "", null)` with `self` being a
`MultiplayerSynchronizer` produces `[RID(0)]` — the original **excludes nothing** in fact. Excluding the
player's body would be a bug fix (v1 forbids it). `exclude(&array![Rid::Invalid])` is the
exact translation. Candidate for the v2 backlog (§Backlog).

### D12 — Mouse events and camera rotation

- `fn input(&mut self, event: Gd<InputEvent>)`: `if let Ok(motion) = event.try_cast::<InputEventMouseMotion>() { let rel = motion.get_screen_relative(); .. }`
  (`input_event_mouse_motion.rs:219` → `Vector2`).
- `rotate_camera(move)`: **private** Rust method (`fn rotate_camera(&mut self, mv: Vector2)`,
  without `#[func]` — in the original it is only called internally; no GDScript uses it; keeping it without
  `#[func]` does not change the contract, and the name stays the same). Body:
  `camera_base.rotate_y(-mv.x)` (`node_3d.rs:789`, `f32`), `camera_base.orthonormalize()` (`:819`),
  `let mut r = camera_rot.get_rotation(); r.x = (r.x + mv.y).clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); camera_rot.set_rotation(r);`.
- `CAMERA_X_ROT_MIN/MAX`: `const CAMERA_X_ROT_MIN: f32 = (-89.9_f32).to_radians();`
  `const CAMERA_X_ROT_MAX: f32 = 70.0_f32.to_radians();` — `f32::to_radians` is a `const fn`
  (compiled and verified with rustc 1.98.1, edition 2024: `-1.569051 1.2217305`). The alternative
  `godot::global::deg_to_rad(f64) -> f64` (`utilities.rs:612`) is not `const`; rejected for
  constants.
- `get_aim_rotation() -> f64` (`float` in GDScript; returning `f64` keeps the Godot type `float`):
  clamp of `camera_rot.get_rotation().x` and the two divisions, converting `as f64` on return.
- `get_camera_base_quaternion() -> Quaternion`: `camera_base.get_global_transform().basis.get_quaternion()`
  — `Basis::get_quaternion` is the documented equivalent of `get_rotation_quaternion()`
  (`builtin/matrices/basis.rs:187–192`, `#[doc(alias = "get_rotation_quaternion")]`).
- `get_camera_rotation_basis() -> Basis`: `camera_rot.get_global_transform().basis`.
- `camera_animation.play("shoot")`: `play()` generated without arguments (`animation_player.rs:317`);
  use `play_ex().name("shoot").done()` (`:322,912,936`).
- Fade: `let mut m = color_rect.get_modulate(); m.a = ...; color_rect.set_modulate(m);`
  (`canvas_item.rs:170,179`; `Color.a: f32`, `builtin/color.rs:46`).
- Parent height: `self.base().get_parent().unwrap().cast::<Node3D>().get_global_transform().origin.y`.
- `_ready` authority: `if self.base().get_multiplayer_authority() == self.base().get_multiplayer().unwrap().get_unique_id() { camera_camera.make_current(); Input::singleton().set_mouse_mode(MouseMode::CAPTURED); } else { set_process(false); set_process_input(false); color_rect.hide(); }`
  — `Camera3D::make_current` (`camera_3d.rs:261`).

### D13 — Numeric types

**Decision**: `delta: f64` (virtual); everything that feeds `Vector2/3`, `Color`, `rotate_y`,
`get_noise_1d` is `f32` (`real = f32`, `builtin/real.rs:43`; the `double-precision` feature is **not**
active). Convert at the point of use with `as f32` / `as f64`, explicitly, without a helper. State
fields that only interact with `real` (`trauma`, `time`, `aiming_timer`) stay `f32`; return values of
`#[func]` that are `float` in GDScript stay `f64` (Godot type `float`).

Note: in GDScript everything is 64-bit `float` and `Vector3` is 32-bit — the implicit conversion
happens on assignment to the vector; doing it at the same point in Rust preserves the original's precision.
`time += delta * SPEED * 5000.0` accumulates in `f32` in Rust vs `f64` in GDScript, but the value is
consumed by `get_noise_1d(x: f32)` anyway; after minutes of continuous shaking the `f32`
loses resolution before the `f64` — imperceptible effect (noise), and the shake lasts ~0.8 s per
event. **Decision**: `time: f64` (faithful to the GDScript `float`), converted `as f32` at the call.

### D14 — Initialization order and hot-reload

- `#[class(init)]` initializes fields → `OnReady` resolved → `ready()`. Same as
  `var` initializers → `@onready` → `_ready`.
- `reloadable = true`: after hot-reload, `OnReady` goes back to the uninitialized state
  (`godot-macros/src/lib.rs:591–596`); irrelevant for the game scenes (re-instantiated), but
  expected when editing in the editor with the game stopped. Does not affect headless validation.

## Map per script (fields, virtuals, methods) — reference for tasks.md

| # | File | `#[class(init, base=…)]` | Fields | Virtuals | `#[func]`/`#[rpc]` |
|---|---|---|---|---|---|
| 1 | `debug_label.rs` | `DebugLabel`, `Label` | `base` | `process` | — |
| 2 | `part_disappear.rs` | `PartDisappear`, `CpuParticles3D` | `base`; `#[init(node="MiniBlasts")] mini_blasts: OnReady<Gd<CpuParticles3D>>` | `ready` | — |
| 3 | `blast.rs` | `Blast`, `Node3D` | `base`; `#[init(node="LightRays")] light_rays: OnReady<Gd<CpuParticles3D>>`; `#[init(node="AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>`; `camera: Option<Gd<Camera3D>>` | `ready`, `process` | — |
| 4 | `camera_noise_shake.rs` | `CameraNoiseShake`, `Camera3D` | `base`; `start_rotation: Vector3`; `trauma: f32`; `time: f64`; `#[init(val = FastNoiseLite::new_gd())] noise: Gd<FastNoiseLite>`; `#[init(val = (randi() as i32))] noise_seed: i32`; 6 `const` | `ready`, `process` | `#[func] add_trauma(&mut self, amount: f64)`; private `decay_trauma(delta)`, `apply_shake(delta)`, `get_noise_value(seed, pos)` (without `#[func]`: in the original they are not called from outside) |
| 5 | `player_input.rs` | `PlayerInputSynchronizer`, `MultiplayerSynchronizer` | `base`; `toggled_aim: bool`; `aiming_timer: f32`; `#[export]` ×5 values (D4); `#[export]` ×6 `Option<Gd<..>>` (D3); 5 `const` | `ready`, `process`, `input` | `#[func] get_aim_rotation(&self) -> f64`; `#[func] get_camera_base_quaternion(&self) -> Quaternion`; `#[func] get_camera_rotation_basis(&self) -> Basis`; `#[rpc(authority, call_local, unreliable)] jump(&mut self)`; private `rotate_camera(&mut self, Vector2)` |

On private methods (`decay_trauma`, `apply_shake`, `get_noise_value`, `rotate_camera`): in
GDScript every `func` is public, but no other script calls them (checked:
`grep -rn 'decay_trauma\|apply_shake\|get_noise_value\|rotate_camera' oxide-godot/ --include=*.gd`
only finds the files themselves). Without `#[func]` they are not visible to Godot; **this does not change
observable behavior**. If the review prefers literal fidelity, adding `#[func]` to them
is neutral — decision for the implementer; the spec (FR-024) only requires preserving the names that are
consumed. Record the choice in the commit message.

## §E — Empirical checks (Godot 4.7.2 headless, `--script`)

```
A|60.0|59.5|1.0|      str(60.0), str(59.5), str(Engine.get_frames_per_second()) in headless
B|117.74|0.50|        "%3.2f" % (123456789/1048576.0), "%3.2f" % 0.5
C|1|                  DisplayServer.window_get_vsync_mode() in headless (ENABLED=1)
D|[RID(0)]|1|         Array([self], TYPE_RID, "", null)  ← FR-017: ineffective exclusion
E|-1|0.936|           str(int(-1)); 1.0 - 0.016*4.0
```

Script used: `scratchpad/fmt_test.gd` (`extends SceneTree`, `_init` prints and `quit()`).

## Name checks against the remaining GDScript

| Consumer | Line | Name used | Exists in the contract |
|---|---|---|---|
| `player.gd` | 26 | type `PlayerInputSynchronizer` | ✅ struct name |
| `player.gd` | 41 | `$InputSynchronizer.set_multiplayer_authority()` | Node base API |
| `player.gd` | 73 | `player_input.get_aim_rotation()` | ✅ |
| `player.gd` | 87 | `player_input.motion` | ✅ |
| `player.gd` | 89 | `player_input.get_camera_rotation_basis()` | ✅ |
| `player.gd` | 107, 114 | `player_input.jumping` (read and write) | ✅ |
| `player.gd` | 121 | `player_input.aiming` | ✅ |
| `player.gd` | 124 | `player_input.get_camera_base_quaternion()` | ✅ |
| `player.gd` | 133, 135 | `player_input.shooting`, `player_input.shoot_target` | ✅ |
| `player.gd` | 211 | `player_input.camera_camera.add_trauma(amount)` | ✅ (`camera_camera` + `add_trauma`) |
| `red_robot.gd` | 133 | `player.add_camera_shake_trauma(13.0)` → `player.gd:210` → `add_trauma` | ✅ |
| `player.tscn` | 42–53 | replication of `shoot_target`, `motion`, `shooting`, `aiming` | ✅ |
| `player.tscn` | 343–351 | `node_paths` ×6 | ✅ |

Static analysis note: `player_input.camera_camera` is typed `Camera3D` in Godot; calling
`.add_trauma()` on it generates at most the GDScript warning `UNSAFE_METHOD_ACCESS` — exactly
as today (the script `camera_noise_shake_effect.gd` has no `class_name`). No change.

## Candidate v2 backlog (record in `docs/v2-backlog.md` in the commit of the corresponding script)

| Origin | Improvement | Motivation |
|---|---|---|
| `player_input.gd` (port 5) | Actually exclude the player's body in the raycast (`exclude` with the RID of the parent `CharacterBody3D`) | The original passes `[RID(0)]` — ineffective exclusion; the shot can hit the player's own body at extreme angles |
| `player_input.gd` (port 5) | `OnEditor<Gd<T>>` instead of `Option<Gd<T>>` for the 6 mandatory references | Eliminates `unwrap()` per frame and makes the editor flag a missing reference |
| `player_input.gd` (port 5) | Replicate `jumping` or remove the `@export` | Exported but outside replication; today it only makes sense via RPC |
| `camera_noise_shake_effect.gd` (port 4) | Recapture `start_rotation` when animations/scripts move the camera | The original's comment acknowledges the problem; shake adds to the rotation captured a single time |
| `debug.gd` (port 1) | Do not recompute the text while the overlay is hidden | Unnecessary per-frame work |
| `part_disappear.gd` / `blast.gd` (ports 2–3) | Timers/signals as `async` (`godot::task`) when the API stabilizes | Closure chaining reproduces `await` in a less readable way |
