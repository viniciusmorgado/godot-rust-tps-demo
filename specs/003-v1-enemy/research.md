# Research: Milestone C — part and red robot (gdext 0.5.5)

**Phase**: v1 — Raw Port. Exact gdext 0.5.5 signatures used by the two ports and translation
decisions, with discarded alternatives. Sources: crate `~/.cargo/registry/src/*/godot-core-0.5.5/`
and `godot-macros-0.5.5/`; bindings generated in
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (single
`godot-core-*/out` directory on 2026-09-15; if there is another, `ls -dt .../godot-core-*/out | head -1` applies).

**Confirmation method**: the whole mapping was written as a temporary module
(`zz_research.rs`, classes `ZzPart`/`ZzRedRobot`) inside the crate, compiled with `cargo build`
→ **0 errors, 0 warnings** after four corrections imposed by the compiler (D3, D4, D12, D13 — recorded
below because the command's input had them differently), and exercised headless via `-s` scripts
(§E). Module and temporary visibility change in `player.rs` reverted (`git status` clean).

## D1 — Structure and visibility

- **Decision**: `src/part.rs` (`Part: RigidBody3D`), `src/red_robot.rs` (`EnemyRobot: CharacterBody3D`);
  `mod part;` and `mod red_robot;` in `lib.rs`. In `part.rs`, `explode` is `#[func] pub(crate)` since
  port 1 (`#[func]` because `red_robot.gd` calls it by name until port 2; `pub(crate)` for the
  typed access of port 2 — declared already in port 1 so that the robot's commit does not touch
  `part.rs`). In `player.rs`, `add_camera_shake_trauma` becomes `pub(crate)` **in the robot's commit**
  (only the visibility keyword; Milestone B precedent, D1).
- **Names**: `Part` and `EnemyRobot` checked against `out/classes/` **and against the identifiers of the remaining `.gd` files** (`grep -hoE '^(const|class_name|var) [A-Za-z_]+'`). The originally planned name, `RedRobot`, was discarded in T024: `level.gd:6` declares `const RedRobot: PackedScene` and GDScript rejects the script when a native class has the same name (`Parse Error: The member "RedRobot" shadows a native class`) — the whole game stopped loading. Rule: the name of a registered class cannot coincide with any constant, `class_name` or script variable of the `.gd` files that still exist — there is no `part.rs` nor
  `red_robot.rs` in the bindings; no collision.
- **Discarded alternatives**: access to the part via `call("explode")` (would violate FR-018);
  `pub(crate)` in `player.rs` already in port 1 (would mix stories).

## D2 — `fade_value` with a setter that applies to the shader

- **Decision**:
  ```rust
  #[export] #[var(set = set_fade_value)] fade_value: f32,   // default 0.0
  _mat: Option<Gd<Material>>,
  // main #[godot_api] impl Part block:
  #[func]
  fn set_fade_value(&mut self, value: f32) {
      self.fade_value = value;
      if let Some(mat) = &self._mat {
          mat.get_next_pass().unwrap().cast::<ShaderMaterial>()
              .set_shader_parameter("emission_cutout", &value.to_variant());
      }
  }
  ```
  In `process`, the GDScript's `fade_value = pow(...)` (which triggers the setter) becomes
  `self.set_fade_value(fade)` — a direct call to the setter, not an assignment to the field.
- **Rationale**: `#[var(set = ...)]` in the main block (godot-macros `lib.rs:184-215, 1257`);
  `Material::get_next_pass() -> Option<Gd<Material>>` (`material.rs:159`);
  `ShaderMaterial::set_shader_parameter(param, &Variant)` (`shader_material.rs:169`). `_mat` is
  an own field and `get_next_pass()` returns a new `Gd` — no borrow conflict. `f32` types
  (GDScript `float`; feeds shader and `powi`).
- **Confirmed empirically (§E.1)**: `set("fade_value", 0.5)` and `set_indexed("fade_value", 0.25)`
  (the `MultiplayerSynchronizer` path, `red_robot.tscn:10419`) trigger the setter and the
  `emission_cutout` of the **duplicated** `next_pass` changes; the original material does not change.
- **Correction imposed by the compiler**: none here (the input was already right).

## D3 — `ready`: duplicate material and `next_pass` (only outside a dedicated server)

- **Decision**:
  ```rust
  self.base_mut().set_process(false);
  if !Os::singleton().has_feature("dedicated_server") {
      let mesh_inst = self.base().get_node_as::<Node>("Model").get_child(0).unwrap().cast::<MeshInstance3D>();
      let mut mesh = mesh_inst.get_mesh().unwrap();
      let mut mat: Gd<Material> = mesh.surface_get_material(0).unwrap().duplicate_resource();
      mesh.surface_set_material(0, &mat);
      let next_pass: Gd<Material> = mat.get_next_pass().unwrap().duplicate_resource();
      mat.set_next_pass(&next_pass);
      self._mat = Some(mat);
  }
  ```
- **Rationale**: `Os::has_feature(tag) -> bool` (`os.rs:890`); `Node::get_child(idx) -> Option<Gd<Node>>`
  (`node.rs:368`); `MeshInstance3D::get_mesh() -> Option<Gd<Mesh>>` (`mesh_instance_3d.rs:182`);
  `Mesh::surface_get_material(i) -> Option<Gd<Material>>` / `surface_set_material(i, &mat)`
  (`mesh.rs:245,235`); `Material::set_next_pass(&mat)` (`material.rs:150`).
- **Correction imposed by the compiler**: `Resource::duplicate()` (`resource.rs:336`, returns
  `Option<Gd<Resource>>`) is **deprecated** in 0.5.5 and generates a warning — use
  `Gd::<T>::duplicate_resource() -> Gd<T>` (`obj/gd_duplicate.rs:160`), which already returns the
  right type (no `cast`). Discarded alternative: `duplicate()` + `#[allow(deprecated)]` (a suppressed
  warning is not "0 warnings").
- **Confirmed empirically (§E.1)**: `material duplicated=true`, `next_pass duplicated=true`,
  `is_processing=false` after `ready`; `OS.has_feature("dedicated_server")` is **false** in
  `--headless` (the duplication branch runs in the headless validation).

## D4 — The part's `explode()`

- **Decision**:
  ```rust
  #[func]
  pub(crate) fn explode(&mut self) {
      self.base().get_node_as::<MultiplayerSynchronizer>("MultiplayerSynchronizer").set_visibility_public(true);
      self.base_mut().set_freeze_enabled(false);
      if !self.base().get_multiplayer().unwrap().is_server() { return; }
      self.base().get_node_as::<CollisionShape3D>("Col1").set_disabled(false);
      self.base().get_node_as::<CollisionShape3D>("Col2").set_disabled(false);
      self.base_mut().set_linear_velocity(3.0 * Vector3::UP);
      let angular = (Vector3::new(randf() as f32, randf() as f32, randf() as f32).normalized() * 2.0 - Vector3::ONE) * 10.0;
      self.base_mut().set_angular_velocity(angular);
      let wait = self.lifetime + self.lifetime_random * randf() as f32;
      self.base().get_tree().create_timer(wait as f64).signals().timeout()
          .connect_other(&*self, |this: &mut Part| this.base_mut().set_process(true));
  }
  ```
- **Rationale**: `public_visibility` → `set_visibility_public(bool)` (`multiplayer_synchronizer.rs:281`);
  `freeze` → `set_freeze_enabled(bool)` (`rigid_body_3d.rs:717`); `set_linear_velocity`/
  `set_angular_velocity` (`:276,:294`); `randf() -> f64` (`godot::global`, `utilities.rs:812`);
  `await create_timer(..).timeout` → `connect_other` with a *linked* callable (Milestone A D6 pattern:
  if the part is freed before, the callable is invalidated). `$Col1`/`$Col2`/`$MultiplayerSynchronizer`
  stay as `get_node_as` at the point of use (the original also uses `$` inline, not `@onready`).
- **`create_timer` returns `Gd<SceneTreeTimer>` directly** (`scene_tree.rs:316`, no `Option`) —
  do not use `.unwrap()`.

## D5 — `process` and `destroy` RPC

- **Decision**: `process(&mut self, delta: f64)`: `let fade = (self._disappearing_counter / self.disappearing_time).powi(2); self.set_fade_value(fade); self._disappearing_counter += delta as f32; if self._disappearing_counter >= self.disappearing_time - 0.2 { self.base_mut().rpc("destroy", &[]); self.base_mut().set_process(false); }`.
  `#[rpc(authority, call_local, unreliable)] fn destroy`: `let mut puff: Gd<CpuParticles3D> = load::<PackedScene>("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn").instantiate_as::<CpuParticles3D>(); self.base().get_parent().unwrap().add_child(&puff); let origin = self.base().get_global_transform().origin; puff.set_global_position(origin); create_timer(0.2).signals().timeout().connect_other(&*self, |this| this.base_mut().queue_free());`
- **Rationale**: `pow(x, 2.0)` → `powi(2)` (same value); the instance is typed as
  `CpuParticles3D` (base API), **never** `PartDisappear` — the original types `puff` as
  `CPUParticles3D` and only uses `global_transform.origin` (Milestone B D11). `add_child` without
  `force_readable_name` (the original passes only one argument). `preload` → `load` at the point of use.
- **Correction imposed by the compiler**: none.

## D6 — `exploded` signal and `#[godot_api]` blocks

- **Decision**: `#[signal] fn exploded();` inside the **single** `#[godot_api] impl EnemyRobot`
  (together with the `#[func]`s and `#[rpc]`s); emission `self.signals().exploded().emit();`.
- **Rationale**: `#[signal]` and `#[rpc]` are not supported in secondary blocks
  (godot-macros `lib.rs:1256`); `signals()` comes from `WithUserSignals` (`obj/traits.rs:606,639`,
  `&mut self`). The signal is registered with the method's name, so `robot.exploded.connect(...)`
  in `level.gd:99` resolves by name.
- **Confirmed empirically (§E.2)**: `has_signal("exploded") == true`; connection from
  GDScript and emission received.

## D7 — `State` enum and the robot's exports

- **Decision**: `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum State { Idle, Approach, Aim, Shooting }`
  (0..3 = `IDLE..SHOOTING`); `#[export] #[init(val = State::Idle)] state: State`;
  `#[export] test_shoot: bool`, `#[export] target_position: Vector3`, `#[export] #[init(val = 5)] health: i32`,
  `#[export] dead: bool`, `#[export] #[init(val = AIM_PREPARE_TIME)] aim_preparing: f32`.
  Internal: `#[init(val = SHOOT_WAIT)] shoot_countdown: f32`, `#[init(val = AIM_TIME)] aim_countdown: f32`,
  `player: Option<Gd<Node3D>>`, `orientation: Transform3D`.
- **Rationale**: Milestone B D3 precedent. `health: int` → `i32` (comparison `== 0`,
  replicated as `int`). `player: Node3D = null` → `Option<Gd<Node3D>>` (type of the reference
  preserved: it may be a "Target", not a `Player`).
- **Confirmed empirically (§E.2)**: the 6 exports registered with the right types and defaults
  `false / (0,0,0) / 5 / 0 / false / 0.5`; `hint_string` of `state` = `Idle:0,Approach:1,Aim:2,Shooting:3`
  (Rust labels in the inspector — cosmetic, same case as Milestone B).

## D8 — Constants and scene references

- `f32` consts: `PLAYER_AIM_TOLERANCE_DEGREES: f32 = 15.0_f32.to_radians()` (`to_radians` is a
  `const fn` — compiles), `SHOOT_WAIT = 6.0`, `AIM_TIME = 1.0`, `AIM_PREPARE_TIME = 0.5`,
  `BLEND_AIM_SPEED = 0.05`.
- `OnReady` (`#[init(node = ...)]`), full paths from the robot (the original's chained
  `x.get_node(^"…")` become composite paths, as in Milestone B D5):

  | Field | Type | Path |
  |---|---|---|
  | `animation_tree` | `AnimationTree` | `AnimationTree` |
  | `shoot_animation` | `AnimationPlayer` | `ShootAnimation` |
  | `model` | `Node3D` | `RedRobotModel` |
  | `ray_from` | `BoneAttachment3D` | `RedRobotModel/Armature/Skeleton3D/RayFrom` |
  | `ray_mesh` | `MeshInstance3D` | `RedRobotModel/Armature/Skeleton3D/RayFrom/RayMesh` |
  | `laser_raycast` | `RayCast3D` | `RedRobotModel/Armature/Skeleton3D/RayFrom/RayCast` |
  | `collision_shape` | `CollisionShape3D` | `CollisionShape3D` |
  | `explosion_sound` / `hit_sound` | `AudioStreamPlayer3D` | `SoundEffects/Explosion`, `SoundEffects/Hit` |
  | `death` | `Node3D` | `Death` |
  | `death_shield1` / `death_shield2` / `death_head` | **`Part`** (typed) | `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` |
  | `death_detach_spark1` / `2` | `CpuParticles3D` | `Death/DetachSpark1`, `Death/DetachSpark2` |

  `LaserEmber` is looked up inside `shoot()` via `get_node_as::<CpuParticles3D>("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber")`
  (the original uses `$...` inline, `red_robot.gd:124`). The original types the parts as
  `RigidBody3D`; the port types them as `Part` because port 1 already made them a Rust class and FR-018 requires
  a typed `explode()` (`self.death_shield1.bind_mut().explode()`).

## D9 — Inverse transformation (`Vector3 * Transform3D`)

- **Decision**: `let gt = self.base().get_global_transform(); let to_player_local = gt.basis.transposed() * (self.target_position - gt.origin);`
  and, for the cannon, `let mt = self.ray_mesh.get_global_transform(); let to_cannon_local = mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin);`.
- **Rationale**: in Godot, `Vector3 * Transform3D` is `Transform3D::xform_inv` =
  `basis.xform_inv(v - origin)` = **transposed** basis applied to `v − origin` — it only coincides
  with `affine_inverse() * v` when the basis is orthonormal (the `RayMesh` sits under an animated
  skeleton with scale). gdext: `Basis::transposed()` (`basis.rs:373`), `impl Mul<Vector3> for Basis`
  (`basis.rs:595`).
- **Confirmed empirically (§E.2)**, with a scaled basis: `v * t == transposed * (v − origin)`
  = `(-4.664, -2.577, 10.043)`; `affine_inverse() * v` = `(4.808, -11.914, 5.848)` — different.
- **Discarded alternative**: `gt.affine_inverse() * v` — would reproduce a different value whenever
  there is scale.

## D10 — Trigonometry and axes

- GDScript's `atan2(a, b)` → `a.atan2(b)` (`f32::atan2(self, other)` = atan2(self, other) —
  same argument order): `to.x.atan2(to.z)`, `to.x.atan2(-to.z)`, `to.y.atan2(-to.z)`.
- `rad_to_deg(x)` → `x.to_degrees()`; `clamp(x, 0, 1)`/`clampf` → `f32::clamp`.
- `gt.basis.y` → `gt.basis.col_b()` (`basis.rs:487`); `absf` → `.abs()`;
  `a.distance_to(b)` → `Vector3::distance_to` (exists in `vector_macros.rs`); `(a - b).length()`.

## D11 — `AnimationTree`

- `animation_tree["parameters/state/transition_request"] = "turn_left"` →
  `self.animation_tree.set("parameters/state/transition_request", &"turn_left".to_variant())`.
- `animation_tree[param] = 1` with `param = "parameters/hit" + str(randi() % 3 + 1) + "/request"` →
  `let param = format!("parameters/hit{}/request", randi() % 3 + 1); self.animation_tree.set(&param, &1.to_variant());`
  (`randi() -> i64`; `1` = `AnimationNodeOneShot.ONE_SHOT_REQUEST_FIRE`, integer as in the original).
- `animation_tree.get("parameters/aim/blend_position")` → `.get(..).to::<Vector2>()`;
  `set("parameters/aiming/blend_amount", &x.to_variant())` with `x: f32`;
  `$AnimationTree.active = true` → `self.animation_tree.set_active(true)` (`animation_mixer.rs:114`).
- Root motion and the final physics block: identical to the Player (Milestone B D8–D10), except that the final
  basis goes to **the robot itself**: `self.base_mut().set_global_basis(basis)`.

## D12 — Raycasts with effective exclusion

- **Decision**: each of the original's three raycasts (`red_robot.gd:115,209,233`) is translated
  **inline**, at the point of use, without a helper:
  ```rust
  let rid = self.base().get_rid();
  let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, ray_to)
      .collision_mask(0xFFFFFFFF)
      .exclude(&array![rid])
      .done();
  let col: VarDictionary = self.base().get_world_3d().unwrap()
      .get_direct_space_state().unwrap()
      .intersect_ray(&params.unwrap());
  // `not col.is_empty() and col.collider == player`:
  let hit_player = !col.is_empty()
      && col.get("collider").and_then(|v| v.try_to::<Gd<Object>>().ok())
          .map(|c| c.instance_id() == player.instance_id()).unwrap_or(false);
  ```
- **Rationale**: `[self]` in the original is the RID of the `CharacterBody3D` itself — `CollisionObject3D::get_rid()`
  (`collision_object_3d.rs:173`); the exclusion **is** effective (unlike the `player_input.gd`
  quirk, where `self` was a `MultiplayerSynchronizer` without a RID). `col.collider == player`
  compares object identity → `instance_id()` (`obj/gd.rs:301`); `Gd<Object>` and `Gd<Node3D>`
  are different types, which is why `==` is not used directly. `col.position` → `col.get("position").unwrap().to::<Vector3>()`.
  `create_ex(..).done()` returns `Option<Gd<PhysicsRayQueryParameters3D>>` (static, as in
  Milestone A D11).
- **Correction imposed by the compiler**: in 0.5.5 `Dictionary` is generic (`Dictionary<K, V>`);
  `intersect_ray` returns **`VarDictionary`** (`= Dictionary<Variant, Variant>`,
  `physics_direct_space_state_3d.rs:46`, `dictionary.rs:127`) — use that name.
- **Discarded alternative**: private helpers `intersect_ray(from, to)` / `collider_is(col, node)`
  (compiled in the draft) — they work, but they would be an extraction of repeated code, that is,
  refactoring (Principle I). Left for v2.

## D13 — `shoot()`: laser, `LaserEmber`, `Blast`, delayed shake

- `ray_dir = gt.basis.col_b()`; `max_dist: f32 = 1000.0`; `ray_origin.distance_to(position)`.
- `_clip_ray(length)`: `if !Os::singleton().has_feature("dedicated_server") { self.ray_mesh.get_surface_override_material(0).unwrap().cast::<ShaderMaterial>().set_shader_parameter("clip", &(length + mesh_offset).to_variant()); }`
  with `mesh_offset = self.ray_mesh.get_position().z` (`mesh_instance_3d.rs:258`).
- `LaserEmber`: `set_position(Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset))`;
  `let mut e = get_emission_box_extents(); e.z = (max_dist - mesh_offset.abs()) / 2.0; set_emission_box_extents(e)`
  (`cpu_particles_3d.rs:753,744` — the original writes `.z` of a `Vector3` property, which is
  read/modify/write).
- `Blast`: `let mut blast: Gd<Node3D> = load::<PackedScene>("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn").instantiate_as::<Node3D>(); self.base().get_tree().get_root().unwrap().add_child(&blast); blast.set_global_position(position);`
  — base API, never `Blast`.
- `if col.collider == player and player is Player: await 0.1 s; player.add_camera_shake_trauma(13.0)` →
  ```rust
  if let Some(player) = self.player.clone() {
      if hit_player {   // col.collider == player (D12, inline)
          if let Ok(player) = player.try_cast::<Player>() {
              self.base().get_tree().create_timer(0.1).signals().timeout()
                  .connect_other(&*self, move |_this: &mut EnemyRobot| {
                      player.clone().bind_mut().add_camera_shake_trauma(13.0);
                  });
          }
      }
  }
  ```
  The closure captures `Gd<Player>` by `move`; `Gd::clone()` is necessary because the callable
  may be called via `&Fn`. `add_camera_shake_trauma` needs `pub(crate)` in `player.rs`
  (D1). `pass # Kill.` stays without effect (no code).
- **Correction imposed by the compiler**: `body.get_name() == "Target".into()` is ambiguous
  (`StringName: PartialEq<_>` has several impls) — use `body.get_name() == StringName::from("Target")`.

## D14 — Area handlers and `hit`

- `_on_area_body_entered(body: Gd<Node3D>)`: `if body.clone().try_cast::<Player>().is_ok() || body.get_name() == StringName::from("Target") { self.player = Some(body); self.state = State::Approach; }`
  (`clone()` necessary because `try_cast` consumes and `body` is still stored).
  `_on_area_body_exited(body)`: `if body.try_cast::<Player>().is_ok() { self.player = None; self.state = State::Idle; }`.
- `hit` (`#[rpc(authority, call_local, unreliable)]`): translation of `red_robot.gd:78-105`; the
  10 s → `create_timer(10.0).signals().timeout().connect_other(&*self, |this| this.base_mut().queue_free())`
  only if `is_server()`; `self.model.set_visible(false)` / `self.death.set_visible(true)`
  (`node_3d.rs:620`); parts via `self.death_shield1.bind_mut().explode()`.
- `physics_process`: `let Some(player) = self.player.clone() else { ...; return; };` reproduces
  `if not player: ... return` and yields a `Gd<Node3D>` for the rest of the frame. Without a player:
  `set_velocity(get_gravity() * delta)` **replaces** the velocity (the original does too).

## D15 — What does NOT change (quirks preserved, FR-027)

`body.name == "Target"` (no node with that name exists); `pass # Kill.`; `player: Node3D`
(not `Player`); `aim_preparing`/`test_shoot` exported without replication; 10 s `await` inside the
`hit` RPC; puff instantiated on the part's parent (`Death`); `angular_velocity` with `randf()` per axis
(the original's distribution); the parts' `explode()` called even on non-server peers (returns
early). Backlog candidates in §"Candidate v2 backlog".

## §E — Empirical verification (2026-09-15, headless, no editor open)

### E.1 Part (programmatic `ZzPart`: `Model/MeshInstance3D` with `BoxMesh` + `StandardMaterial3D` whose `next_pass` is a `ShaderMaterial` with `uniform float emission_cutout`)

```
is_node_ready=true material duplicated=true next_pass duplicated=true is_processing=false
set(): fade_value=0.5 emission_cutout(dup)=0.5 (original)=<null>      ← setter via Object.set
set_indexed(): emission_cutout(dup)=0.25                                ← MultiplayerSynchronizer path
has explode=true destroy=true set_fade_value=true
props lifetime/lifetime_random/disappearing_time/fade_value: type=3 (FLOAT) usage=6 (DEFAULT), defaults 3.0/3.0/0.5
OS.has_feature("dedicated_server")=false in --headless
```

(Method note: the checks need to run in the `SceneTree`'s `_process`, not in `_init` —
the nodes' `_ready` only fires after the tree starts processing.)

### E.2 Robot (`ZzRedRobot.new()`)

```
has_signal exploded=true; GDScript connection by name + emit_signal → received
has_method: hit, play_shoot, shoot_check, resume_approach, _on_area_body_entered, _on_area_body_exited = true;
            shoot, animate, _clip_ray = false (private)
props: test_shoot BOOL false; target_position VECTOR3 (0,0,0); health INT 5; state INT 0 hint 'Idle:0,Approach:1,Aim:2,Shooting:3'; dead BOOL false; aim_preparing FLOAT 0.5
set("state", 2) → 2
xform_inv (basis with scale): v*t = transposed*(v−origin) = (-4.664, -2.577, 10.043); affine_inverse*v = (4.808, -11.914, 5.848)
```

### E.3 Baseline (commit `4bb8f7f`, before any port of this milestone)

`cargo build` 0 warnings; import with `Initialize godot-rust` and 0 `ERROR`;
`enemies/red_robot/red_robot.tscn` exit 124, regression grep empty, 1 WARNING (HDR);
`level/level.tscn` exit 124, grep empty, 2 WARNINGs (HDR, Physics interpolation).

## Map per script (summary for the tasks)

### 1. `part.gd` → `src/part.rs` — `Part: RigidBody3D`

- Fields: `_mat: Option<Gd<Material>>`; `#[export] #[init(val = 3.0)] lifetime: f32`;
  `#[export] #[init(val = 3.0)] lifetime_random: f32`; `#[export] #[init(val = 0.5)] disappearing_time: f32`;
  `#[export] #[var(set = set_fade_value)] fade_value: f32`; `_disappearing_counter: f32`.
- `IRigidBody3D`: `ready` (D3), `process` (D5).
- `#[godot_api] impl Part`: `#[func] set_fade_value` (D2), `#[func] pub(crate) explode` (D4),
  `#[rpc(authority, call_local, unreliable)] destroy` (D5).

### 2. `red_robot.gd` → `src/red_robot.rs` — `EnemyRobot: CharacterBody3D`

- `State` enum, consts (D7–D8), 6 exports, 4 internal, 15 `OnReady` (D8).
- `ICharacterBody3D`: `ready`, `physics_process` (D14, D11).
- Single `#[godot_api] impl EnemyRobot`: `#[signal] exploded`, `#[func] resume_approach`,
  `#[rpc] hit`, `#[rpc] play_shoot`, `#[func] shoot_check`, `#[func] _on_area_body_entered`,
  `#[func] _on_area_body_exited`.
- Private `impl EnemyRobot`: `shoot` (D12–D13), `animate` (D9–D11), `_clip_ray` (D13); the three
  inline raycasts (D12).
- `player.rs`: `add_camera_shake_trauma` → `pub(crate)` (D1), in the robot's commit.

## Candidate v2 backlog (record in `docs/v2-backlog.md` in the corresponding script's commit)

Items 1–14 already exist. New:

| # | Origin | Improvement | Motivation |
|---|---|---|---|
| 15 | `enemies/red_robot/parts/part.gd` (port 1) | Instantiate the puff on the **robot's** parent (or on the root) instead of the part's parent (`Death`) | The puff is born under the robot, which is removed 10 s after death; today the timings do not cross, but the dependency is fragile |
| 16 | `enemies/red_robot/red_robot.gd` (port 2) | Remove the dead branch `body.name == "Target"` and type `player` as `Gd<Player>` | No scene has a `Target` node; the generic reference forces a `try_cast` on each use |
| 17 | `enemies/red_robot/red_robot.gd` (port 2) | Replace the 10 s `await` inside the `hit` RPC with a timer/signal outside the RPC | Removal logic coupled to the damage handler |
| 18 | `enemies/red_robot/red_robot.gd` (port 2) | Replicate `aim_preparing` (or not export it) and remove `test_shoot` from the inspector | Exported but outside the `SceneReplicationConfig`; `test_shoot` is an internal trigger of the method track |
