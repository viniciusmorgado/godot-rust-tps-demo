# Research: Milestone B — player, bullet and door (gdext 0.5.5)

**Phase**: v1 — Raw Port. This document pins the exact gdext 0.5.5 signatures used by the three
ports and the translation decisions, with discarded alternatives. Sources: crate
`~/.cargo/registry/src/*/godot-core-0.5.5/` and `godot-macros-0.5.5/`; bindings generated in
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (the only
`godot-core-*/out` directory existing on 2026-09-15 — if another appears, `ls -dt .../godot-core-*/out | head -1` applies).
Lines cited as `file.rs:N` refer to those directories.

**Confirmation method**: the whole mapping below was written as a temporary module
(`zz_research.rs`, three classes `ZzPlayer`/`ZzBullet`/`ZzDoor`) inside the crate, compiled with
`cargo build` → **0 errors, 0 warnings**, and exercised headless by a `-s` script (§E). The module
and the temporary visibility changes were reverted (`git status` clean) — none of this is
implementation; the implementation happens in the tasks.

## D1 — Structure: three modules, `pub(crate)` visibility on what the Player consumes

- **Decision**: `src/player.rs` (`Player: CharacterBody3D`), `src/bullet.rs`
  (`Bullet: CharacterBody3D`), `src/door.rs` (`Door: Area3D`); `mod` in `lib.rs`. In
  `player_input.rs`, the fields `aiming`, `shoot_target`, `motion`, `shooting`, `jumping`,
  `camera_camera` and the methods `get_aim_rotation`, `get_camera_base_quaternion`,
  `get_camera_rotation_basis` go from private to `pub(crate)`; in `camera_noise_shake.rs`,
  `add_trauma` likewise. Only the visibility keyword changes — no line moves, no body changes.
  The change goes into the port 1 (Player) commit and is cited in the message.
- **Rationale**: FR-010/FR-011 require **typed** access (`self.player_input.bind().motion`,
  `.bind_mut().jumping = false`, `camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(..)`).
  In Rust, fields and methods without `pub` are private to the module; `pub(crate)` is the minimum that makes
  the access possible without exposing anything outside the crate. It is neither abstraction nor refactoring: no
  logic is extracted or moved (recorded in plan.md §Complexity Tracking as a non-violation).
- **Discarded alternatives**: (a) access via `Variant` (`player_input.get("motion")`) —
  violates FR-011 and Principle II (dynamic access between classes already in Rust); (b) moving
  `PlayerInputSynchronizer` into `player.rs` — would touch already committed code without
  need; (c) unrestricted `pub` — unnecessary, the crate is `cdylib`.
- **Names**: `Player` mandatory (FR-001). `Bullet` and `Door` checked against the class list
  of the bindings (`out/classes/`): neither `bullet.rs` nor `door.rs` exists — no collision.

## D2 — `player_id` with a setter that runs outside the tree

- **Decision**:
  ```rust
  #[export]
  #[var(set = set_player_id)]
  #[init(val = 1)]
  player_id: i32,
  // ... in the MAIN #[godot_api] impl Player block:
  #[func]
  fn set_player_id(&mut self, value: i32) {
      self.player_id = value;
      self.base()
          .get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")
          .set_multiplayer_authority(value);
  }
  ```
- **Rationale**: `#[var(set = name)]` is the documented form (godot-macros `lib.rs:184-215`);
  the setter MUST be `#[func]` and live in the main `#[godot_api] impl` block
  (`lib.rs:1257`: secondary blocks cannot be referenced by `get`/`set`). The setter
  runs **before** `ready` (`level.gd:119` assigns `player_id` with the node outside the tree), which is
  why it does not use the `OnReady` `player_input` (not yet initialized) but rather `get_node_as` with a
  relative path — which works outside the tree because the children of the instantiated scene already
  exist. Type `i32`: `set_multiplayer_authority(id: i32)` (`node.rs:1341`) and
  `get_unique_id() -> i32`; the GDScript `int` → `i32` conversion is done by gdext at the boundary.
- **Confirmed empirically (§E.1)**: with the node outside the tree, `p.player_id = 7` and
  `p.set("player_id", 9)` (the form `level.gd` uses, since it types `player` as
  `CharacterBody3D`) update the child's authority.
- **Discarded alternatives**: `try_get_node_as` (would hide the missing child; the original
  also errors in that case); storing `player_id` without side effect and applying it in `ready`
  (would change the moment of the assignment — improvement/behavior change).
- **Unavoidable side effect**: `set_player_id` becomes visible as a Godot method
  (`has_method("set_player_id") == true`). The original GDScript exposes the inline setter in a
  different way; no consumer calls that name — not observable in the game.

## D3 — `Animations` as an exported enum

- **Decision**:
  ```rust
  #[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
  #[godot(via = i64)]
  pub enum Animations { JumpUp, JumpDown, Strafe, Walk }   // 0, 1, 2, 3
  // field:
  #[export]
  #[init(val = Animations::Walk)]
  current_animation: Animations,
  ```
- **Rationale**: `derive(GodotConvert)` + `#[godot(via = i64)]` generates discriminants 0.. in
  declaration order (`lib.rs:1494-1508`); `Var`/`Export` derivable for c-like enums
  (`lib.rs:1532,1540`). The property is registered with the field name `current_animation`,
  type `int`, hint `ENUM` — which is what `player.tscn:29` (`.:current_animation`) replicates.
- **Confirmed empirically (§E.1)**: default `3` (= WALK), `set`/`get` by name work,
  `hint_string = "JumpUp:0,JumpDown:1,Strafe:2,Walk:3"`.
- **Accepted cosmetic difference**: in the inspector the labels are `JumpUp…` instead of `JUMP_UP…`
  (the stored/replicated value is the integer; the scene does not store the default). Not observable in the game.
- **Discarded alternatives**: raw `i32` with constants — would lose the hint and the readability with no
  gain; `JUMP_UP` variants with `#[allow(non_camel_case_types)]` — neither idiomatic nor
  necessary.
- **`animate(anim: Animations, _delta: f64)`** private (spec: internal method). The comparison
  `anim == Animations::JumpUp` requires `PartialEq` (derived above). On clients,
  `physics_process` does `let anim = self.current_animation; self.animate(anim, delta)` (copy
  before the `&mut self`; `Copy` derived).

## D4 — `motion` replicated without being exported → `#[var]`

- **Decision**: `#[var] motion: Vector2`.
- **Rationale**: `player.tscn:26` replicates `.:motion`; the `MultiplayerSynchronizer` reads/writes via
  `get_indexed`/`set_indexed`, which only see **registered** properties. `#[var]` registers
  with `PropertyUsageFlags::NONE` (`registry/info/property_info.rs:125-128`): accessible by name,
  does not appear in the inspector nor is stored in the scene — same effect as the original's script `var`.
- **Confirmed empirically (§E.1)**: `get("motion")`, `set("motion", ..)`,
  `set_indexed("motion", ..)`, `get_indexed("motion")` work; `usage = 0`.
- **Discarded alternative**: `#[export]` — would appear in the inspector and be stored in the scene
  (the original is not `@export`).

## D5 — Scene references (`@onready`)

- **Decision**: `#[init(node = "...")] OnReady<Gd<T>>`, paths relative to the node:

  | Field | Type | Path |
  |---|---|---|
  | `player_input` | `PlayerInputSynchronizer` | `InputSynchronizer` |
  | `animation_tree` | `AnimationTree` | `AnimationTree` |
  | `player_model` | `Node3D` | `PlayerModel` |
  | `shoot_from` | `Marker3D` | `PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom` |
  | `crosshair` | `TextureRect` | `Crosshair` (declared and never used — as in the original, `player.gd:30`) |
  | `fire_cooldown` | `Timer` | `FireCooldown` |
  | `sound_effect_jump` / `_land` / `_shoot` | `AudioStreamPlayer` | `SoundEffects/Jump`, `SoundEffects/Land`, `SoundEffects/Shoot` |

  `initial_position: Vector3` is captured on the first line of `ready`
  (`self.base().get_transform().origin`) — equivalent to `@onready var initial_position =
  transform.origin` (`player.gd:24`).
- **Rationale**: `OnReady` requires `Base` + a `Node` class (`obj/on_ready.rs:63-64`) and is
  initialized before `ready()` in declaration order. `OnReady<Gd<PlayerInputSynchronizer>>`
  works because the class is already Rust (Milestone A) — it is the required typed access.
- **Composite paths**: the original reaches `ShootFrom` and the sounds in two steps
  (`player_model.get_node(^"Robot_Skeleton/...")`, `sound_effects.get_node(^"Jump")`). A
  `#[init(node)]` cannot start from another `OnReady`, so the full path from the
  Player is used — same node resolution, same result. The intermediate `sound_effects: Node`
  (`player.gd:33`) is not kept because it only existed to chain the `get_node`.
- **Unused field without warning**: `crosshair` is not read anywhere, but the derive
  `GodotClass` references the field, and the build confirmed 0 warnings. Kept for fidelity.
- **`ShootParticle`/`MuzzleFlash`**: looked up inside `shoot()` via
  `self.base().get_node_as::<CpuParticles3D>("PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle")`
  (and `.../MuzzleFlash`), like the original's `$...` (`player.gd:193,196`). gdext spelling:
  `CpuParticles3D` (`cpu_particles_3d.rs`).

## D6 — Typed access and `bind()`/`bind_mut()` discipline

- **Decision**: reads via temporary guard — `self.player_input.bind().motion`,
  `self.player_input.bind().aiming`, `self.player_input.bind().get_camera_rotation_basis()`;
  write `self.player_input.bind_mut().jumping = false`. In `add_camera_shake_trauma`:
  ```rust
  let camera = self.player_input.bind().camera_camera.clone().unwrap(); // guard released here
  camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);
  ```
- **Rationale**: `Gd<T>::bind()`/`bind_mut()` (`obj/gd.rs`) borrow the **other** object
  (`PlayerInputSynchronizer`, `CameraNoiseShake`) — not the `Player`, which is already in `&mut self`.
  Distinct objects do not conflict. Even so, the guards are always temporary (one expression)
  so they never coexist with a `bind_mut()` of the same object. `camera_camera` is
  `Option<Gd<Camera3D>>` (Milestone A, D3); `.clone()` of the `Gd` is cheap (ref-count) and `cast::<CameraNoiseShake>()`
  (`obj/gd.rs:550`) is valid because `player.tscn:627` already has `type="CameraNoiseShake"` — the
  `cast` panics if the type does not match, equivalent to the type error GDScript would give.
- **Confirmed**: compiles without warnings; pattern identical to the one used in `blast.rs`/`part_disappear.rs`.
- **Discarded alternative**: storing `Gd<CameraNoiseShake>` in a field in `ready` — would change the
  moment of the resolution (the original resolves on every call).

## D7 — RPCs and re-entrancy

- **Decision**: `#[rpc(authority, call_local, unreliable)]` on `jump`, `land`, `shoot`, `hit`,
  `add_camera_shake_trauma(amount: f64)` (Player) and `explode` (Bullet). Firing:
  `self.base_mut().rpc("land", &[])` (`node.rs:1480`, returns `Error`, ignored as in the original).
  `shoot()` calls `self.add_camera_shake_trauma(0.35)` **directly** (Rust call, no RPC —
  like `player.gd:201`); `hit()` likewise with `0.75`.
- **Rationale**: `@rpc("call_local")` = mode `authority` + `unreliable` (defaults), confirmed in
  Milestone A (research 001 D5). `call_local` makes Godot invoke the `#[func]` itself on the same
  object **while** `apply_input(&mut self)` is active; this is safe because the call goes
  through `self.base_mut()`, whose guard allows re-entrancy on the object (`obj/base.rs:163`,
  `storage/instance_storage.rs:73`) — exactly the path already validated in the game in Milestone A
  (`player_input.rs:217`, `jump` RPC fired from inside `process`).
- **Bullet → player**: `collider.rpc("hit", &[])` is called from `Bullet::physics_process` on the
  player's `Gd<Node3D>`, which is not borrowed at that moment — no re-entrancy.

## D8 — `AnimationTree`: dynamic properties and root motion

- **Decision**: `self.animation_tree.set("parameters/state/transition_request", &"strafe".to_variant())`
  (`Object::set(&mut self, property: impl AsArg<StringName>, value: &Variant)`, `object.rs:175`);
  `set("parameters/aim/add_amount", &aim.to_variant())` with `aim: f64`;
  `set("parameters/aim/add_amount", &0.to_variant())` (integer `0`, like `player.gd:79`);
  `set("parameters/strafe/blend_position", &Vector2::new(self.motion.x, -self.motion.y).to_variant())`;
  `set("parameters/walk/blend_position", &Vector2::new(self.motion.length(), 0.0).to_variant())`.
  Typed root motion: `get_root_motion_rotation() -> Quaternion` (`animation_mixer.rs:287`),
  `get_root_motion_position() -> Vector3` (`:277`).
- **Rationale**: `animation_tree["parameters/..."] = x` is `Object.set` — base API (FR-025 allows it).
  The integer `0` is preserved because Godot converts `int → float` when writing to the parameter; using
  `0.0` would give the same result, but the original's literal is `0`.

## D9 — Builtin math

| GDScript | gdext 0.5.5 | Source |
|---|---|---|
| `Transform3D(quat, pos)` | `Transform3D::new(Basis::from_quaternion(q), pos)` | `transform3d.rs:90`; `basis.rs:128` |
| `orientation *= root_motion` | `self.orientation = self.orientation * self.root_motion` | `impl Mul for Transform3D`, `transform3d.rs:291` |
| `orientation.orthonormalized()` | `self.orientation.orthonormalized()` | `transform3d.rs:177` |
| `basis.get_rotation_quaternion()` | `basis.get_quaternion()` | `basis.rs:191` |
| `q_from.slerp(q_to, w)` | `q_from.slerp(q_to, w as f32)` (`weight: real`) | `quaternion.rs:203` |
| `Basis(q)` | `Basis::from_quaternion(q)` | `basis.rs:128` |
| `Basis.looking_at(target)` | `Basis::looking_at(target)` (defaults `up = UP`, `use_model_front = false`) | generated, `builtin_classes/basis.rs:231` |
| `basis.x` / `basis.z` | `basis.col_a()` / `basis.col_c()` | `basis.rs:471,503` — gdext stores `rows`; the columns are the axes |
| `orientation.basis = ...` / `.origin = ...` | public fields `basis`/`origin` | `transform3d.rs:58,61` |
| `motion.lerp(to, w)` / `.length()` / `.normalized()` | same (`weight: real`) | `vector_macros.rs:776,453,802` |
| `player_model.global_transform.basis = b` | `self.player_model.set_global_basis(b)` | `node_3d.rs:410` |
| `transform.origin` (read) | `self.base().get_transform().origin` | `node_3d.rs:211` |
| `transform.origin = initial_position` | `let mut t = get_transform(); t.origin = ...; set_transform(t)` (preserves the basis) | `node_3d.rs:202` |

- **Attention `col_c`**: the GDScript `transform.basis.z` is the z **column** (third axis). In gdext
  `Basis { rows: [Vector3; 3] }` — `rows[2]` would be the row, wrong; the correct accessor is
  `col_c()`. Likewise `basis.x` → `col_a()`.

## D10 — `CharacterBody3D` / physics

- `is_on_floor() -> bool` (`character_body_3d.rs:473`), `get_velocity()/set_velocity(Vector3)`
  (`:211,:202`), `set_up_direction(Vector3::UP)` (`:427`), `move_and_slide() -> bool` (`:183`,
  return ignored as in the original), `get_gravity() -> Vector3` (`physics_body_3d.rs:67`).
- `velocity.y = JUMP_SPEED` → `let mut v = get_velocity(); v.y = JUMP_SPEED; set_velocity(v)`.
  The original's final sequence (`velocity.x/z = h; velocity += gravity*delta; set_velocity(velocity)`)
  becomes a single read/modify/write — same final value.
- Types: `f32` constants (`MOTION_INTERPOLATE_SPEED`, `ROTATION_INTERPOLATE_SPEED` = 10.0;
  `MIN_AIRBORNE_TIME` = 0.1; `JUMP_SPEED` = 5.0); `airborne_time: f32` with `#[init(val = 100.0)]`;
  `delta: f64` in the virtuals, `delta as f32` at the point of use (Milestone A rule).
- Bullet: `move_and_collide(motion: Vector3) -> Option<Gd<KinematicCollision3D>>`
  (`physics_body_3d.rs:37`); `KinematicCollision3D::get_collider() -> Option<Gd<Object>>`
  (`kinematic_collision_3d.rs:259`); displacement
  `-(delta as f32) * BULLET_VELOCITY * self.base().get_transform().basis.col_c()`.

## D11 — Instantiating the bullet

- **Decision**:
  ```rust
  let mut bullet: Gd<CharacterBody3D> = load::<PackedScene>("res://player/bullet/bullet.tscn")
      .instantiate_as::<CharacterBody3D>();
  self.base().get_parent().unwrap().add_child_ex(&bullet).force_readable_name(true).done();
  bullet.set_global_position(shoot_origin);
  bullet.look_at(shoot_origin + shoot_dir);
  bullet.add_collision_exception_with(&self.to_gd());
  self.base_mut().rpc("shoot", &[]);
  ```
- **Rationale**: `load::<T>` (`tools/save_load.rs:30`) at the point of use replaces `preload`; after the
  first load the `ResourceLoader` serves from the cache, and `bullet.tscn` is already loaded
  before the first shot because it is an `ext_resource` of `player.tscn` (`BulletCache`, l.679) — no
  observable difference. `instantiate_as::<CharacterBody3D>` (`manual_extensions.rs:65`) types
  it as **base API**, not as `Bullet`: in the port 1 commit the bullet is still GDScript and, after
  port 2, only base API keeps being used (`set_global_position`, `look_at`,
  `add_collision_exception_with`) — the Player never depends on the `Bullet` class (Principle II,
  "base API is not a dependency"). `add_child_ex(..).force_readable_name(true)` = `add_child(bullet, true)`
  (`node.rs:279`). `set_global_position` (`node_3d.rs:392`) = `global_transform.origin = ...`.
  `look_at(target)` (`node_3d.rs:851`, `up` default). `add_collision_exception_with(body: impl AsArg<Gd<Node>>)`
  (`physics_body_3d.rs:107`) accepts `&self.to_gd()`.
- **Discarded alternatives**: field `#[init(val = load(..))]` (would load at Player construction,
  before `ready` — works, but shifts the moment of the load with no gain);
  `instantiate_as::<Bullet>` (would create an unnecessary Rust→Rust dependency and break commit 1).

## D12 — `Timer`, `AudioStreamPlayer`, `CpuParticles3D`, `CollisionShape3D`, `Light3D`

| GDScript | gdext 0.5.5 | Source |
|---|---|---|
| `fire_cooldown.time_left == 0` | `self.fire_cooldown.get_time_left() == 0.0` (`f64`) | `timer.rs:301` |
| `fire_cooldown.start()` | `self.fire_cooldown.start()` (no args = `time_sec` default −1) | `timer.rs:237` |
| `sound.play()` | `.play()` | `audio_stream_player.rs:255` |
| `particle.restart()` / `.emitting = true` | `.restart()` / `.set_emitting(true)` | `cpu_particles_3d.rs:492,173` |
| `collision_shape.disabled = true` | `.set_disabled(true)` | `collision_shape_3d.rs:198` |
| `omni_light.shadow_enabled = true` | `.set_shadow(true)` (setter of the `shadow_enabled` property) | `light_3d.rs:62` |
| `animation_player.play(&"explode")` | `.play_ex().name("explode").done()` | `animation_player.rs:322` (Milestone A pattern) |
| `queue_free()` / `set_physics_process(false)` / `set_process(false)` | same on `base_mut()` | `node.rs:1299,739,779` |
| `multiplayer.is_server()` | `self.base().get_multiplayer().unwrap().is_server()` | `node.rs:1369`; `multiplayer_api.rs:63` |

## D13 — Duck typing preserved in the bullet (`has_method("hit")` + `rpc`)

- **Decision**:
  ```rust
  if let Some(col) = self.base_mut().move_and_collide(displacement) {
      let collider: Option<Gd<Node3D>> = col.get_collider().and_then(|c| c.try_cast::<Node3D>().ok());
      if let Some(mut collider) = collider {
          if collider.has_method("hit") { collider.rpc("hit", &[]); }
      }
      self.collision_shape.set_disabled(true);
      self.base_mut().rpc("explode", &[]);
      self.hit = true;
  }
  ```
- **Rationale**: `col.get_collider() as Node3D` → `null` if it is not a `Node3D`, and the original's `if collider and
  ...` covers that — `and_then(try_cast(..).ok())` reproduces both cases.
  `has_method` (`object.rs:474`) and `rpc` (`node.rs:1480`) by name are exactly what the
  original already does dynamically — allowed by FR-025 and by Principle II ("duck typing that already
  exists in the original"). v2 backlog item 2 (`Hittable`) already covers the improvement — do not duplicate.

## D14 — `Settings` exception (first use)

- **Decision**:
  ```rust
  let config_file = self.base().get_node_as::<Node>("/root/Settings")
      .get("config_file").to::<Gd<ConfigFile>>();
  if config_file.get_value("rendering", "shadow_mapping").to::<bool>() {
      self.omni_light.set_shadow(true);
  }
  ```
- **Rationale**: it is the single exception of Principle II — the autoload is obtained dynamically by
  absolute path (valid because `explode` runs with the bullet in the tree), `config_file` read via
  `Object::get` (`object.rs:185`) and, from then on, typed `ConfigFile` API
  (`get_value(section, key) -> Variant`, `config_file.rs:145`). `settings.gd` stores a `bool` in that
  key, so `.to::<bool>()` is the exact conversion. v2 backlog item 1 already records the typed
  access — do not duplicate.
- **Headless**: the autoload **is** loaded in `--path . scene.tscn` (verified in the spec, Edge
  Cases); it is only missing in a `-s` harness. The port 2 validation runs `bullet.tscn` for 20 s: the bullet
  expires at 5 s → `explode` → `Settings` read → `destroy` at 1.5 s of the animation — all within
  the `timeout 20`.
- **Discarded alternative**: `try_get_node_as` with silent fallback — would hide the absence
  of the autoload; the original also errors in that case.

## D15 — Door: `try_cast` and the bug fix

- **Decision**:
  ```rust
  #[derive(GodotClass)]
  #[class(init, base=Area3D)]
  pub struct Door {
      base: Base<Area3D>,
      open: bool,
      // upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (a node that does not exist);
      // the scene node is "DoorModel2" — the door never opened and Godot printed "Node not found".
      #[init(node = "DoorModel2/AnimationPlayer")]
      animation_player: OnReady<Gd<AnimationPlayer>>,
  }
  #[godot_api] impl IArea3D for Door {}
  #[godot_api] impl Door {
      #[func]
      fn _on_door_body_entered(&mut self, body: Gd<Node3D>) {
          if !self.open && body.try_cast::<Player>().is_ok() {
              self.animation_player.play_ex().name("doorsimple_opening").done();
              self.open = true;
          }
      }
  }
  ```
- **Rationale**: `body is Player` ⇔ successful `Gd::try_cast::<Player>()` (`obj/gd.rs:538`);
  `try_cast` consumes `body`, and since the parameter is owned (`Gd<Node3D>` by value) and is not used
  afterwards, it does not need `clone()`. Empty `impl IArea3D for Door {}`: there are no virtuals, but the
  block is kept for uniformity with the other ports (the `OnReady` is initialized by the hook
  generated by `GodotClass`, regardless of the block). The comment `// upstream bug fix: ...`
  goes **on the line immediately above** the `#[init(node = ...)]` — the exact spot of the reference
  (requirement (b), FR-032).
- **Confirmed empirically (§E.1/E.2)**: `has_method("_on_door_body_entered")`; `p is ZzPlayer`
  true for the Player and false for the bullet; baseline of `door.tscn` in the original = exactly
  **1** `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`.
- **Discarded alternatives**: `body.is_class("Player")` (compares string, not type — and is not what
  `is` does with native classes); `body.clone().try_cast()` (unnecessary clone).
- **`CLAUDE.md` catalog**: the door's `Node not found` **was never** in the catalog
  (`CLAUDE.md:39-40` lists only `Cannon_Charge already exists`, `doorsimple_d.png`,
  `surfaces.is_empty()`, all from import). The rule "remove from the catalog in the same commit" is
  satisfied with no change; the finding goes in the port 3 commit message.

## D16 — What does NOT change (preserved quirks, FR-035)

`airborne_time` starts at 100 (first landing triggers `land`); `player_input.jumping = false` on
every physics frame; possible double `explode` when the bullet expires and collides in the same frame;
`velocity` not zeroed on respawn; `crosshair` referenced and never used; `FireCooldown`
`autostart` (0.4 s with no shot after spawn). All stay as they are. v2 backlog candidates are
in §"Candidate v2 backlog".

## §E — Empirical verification (2026-09-15, headless, no editor open)

### E.1 `-s` probe with the three draft classes (`ZzPlayer`/`ZzBullet`/`ZzDoor`)

`SceneTree` script building `ZzPlayer` + child `MultiplayerSynchronizer` "InputSynchronizer"
**outside the tree**:

```
in_tree=false
player_id=7 authority(InputSynchronizer)=7           ← setter by assignment, outside the tree
via set(): player_id=9 authority=9                   ← setter via Object.set (level.gd's form)
motion default=(0.0, 0.0) type=5                     ← #[var] visible by name (type 5 = VECTOR2)
motion after set=(1.0, 2.0) / after set_indexed=(3.0, 4.0)
current_animation default=3 (WALK=3 expected) / after set=2
prop motion            type=5 hint=0 hint_string='' usage=0
prop player_id         type=2 hint=0 usage=6         ← DEFAULT (editor + storage)
prop current_animation type=2 hint=2 hint_string='JumpUp:0,JumpDown:1,Strafe:2,Walk:3' usage=6
has_method hit=true add_camera_shake_trauma=true jump=true animate=false apply_input=false set_player_id=true
bullet has destroy=true explode=true
door has _on_door_body_entered=true
is Player: true (Player) false (Bullet)
```

`cargo build` of the draft: 0 errors, **0 warnings**. Draft and visibility changes
reverted right after (`git status` clean).

### E.2 Door baseline (commit `108584e`, original `door.tscn`)

```
timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn   → exit 124
ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door").   (line 6 of the log)
grep -c 'Node not found' → 1        WARNING: 1 (HDR, benign)
```

This is the error that **MUST disappear** after port 3 (FR-027, SC-003).

## Map per script (summary for the tasks)

### 1. `player.gd` → `src/player.rs` — `Player: CharacterBody3D`

- Consts (`f32`): `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`,
  `MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`.
- `enum Animations` (D3). Fields: `airborne_time: f32` (init 100.0), `orientation: Transform3D`,
  `root_motion: Transform3D`, `#[var] motion: Vector2`, `initial_position: Vector3`, the 10
  `OnReady` of D5, `#[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32`,
  `#[export] #[init(val = Animations::Walk)] current_animation: Animations`.
- `ICharacterBody3D`: `ready` (initial_position; orientation = global of the model with zero origin;
  `set_process(false)` if not server), `physics_process(delta)` (server → `apply_input`;
  otherwise `animate(current_animation)`).
- `#[godot_api] impl Player` (single block): `#[func] set_player_id`; `#[rpc(authority, call_local, unreliable)]`
  `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount: f64)`.
- Private `impl Player`: `animate(anim, _delta)`, `apply_input(delta)` — line by line from
  `player.gd:61-176` with D6–D11.
- Edit in `player_input.rs`/`camera_noise_shake.rs`: only `pub(crate)` (D1).

### 2. `bullet.gd` → `src/bullet.rs` — `Bullet: CharacterBody3D`

- `const BULLET_VELOCITY: f32 = 20.0`; `time_alive: f32` (init 5.0); `hit: bool`; `OnReady`
  `animation_player` ("AnimationPlayer"), `collision_shape` ("CollisionShape3D"), `omni_light`
  ("OmniLight3D").
- `ready`: not server → `set_physics_process(false)` + `collision_shape.set_disabled(true)`.
- `physics_process`: D10/D13. `#[rpc(authority, call_local, unreliable)] explode` (D12/D14);
  `#[func] destroy` (not server → return; `queue_free`).

### 3. `door.gd` → `src/door.rs` — `Door: Area3D` (D15)

## Candidate v2 backlog (record in `docs/v2-backlog.md` in the commit of the corresponding script)

Items 1 (typed Settings) and 2 (`Hittable`) **already exist** — do not duplicate. New candidates:

| # | Origin | Improvement | Motivation |
|---|---|---|---|
| 10 | `player/player.gd` (port 1) | Start `airborne_time` at 0 (or ignore the first landing) | With initial 100, the first contact with the ground triggers `land` and the landing sound on spawn |
| 11 | `player/player.gd` (port 1) | Zero `velocity` on respawn below −40 | The teleport preserves the accumulated fall velocity |
| 12 | `player/player.gd` (port 1) | Remove the never-used `crosshair` reference (or use it) | Declared in `player.gd:30` and not read; in v1 it is kept for fidelity |
| 13 | `player/bullet/bullet.gd` (port 2) | Avoid the double `explode` when `time_alive` expires and there is a collision in the same frame | Two `explode` RPCs in the same frame restart the animation |
| 14 | `door/door.gd` (port 3) | Type `_on_door_body_entered` with `Gd<Player>` via `try_cast` already at the signal | Only the first `Player` opens and it never closes; v1 only fixes the node reference |

The existing item 9 (`jumping` replicated/`@export`) already covers the quirk "player zeroes `jumping`";
do not duplicate.
