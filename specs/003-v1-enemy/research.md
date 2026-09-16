# Research: Marco C — peça e robô vermelho (gdext 0.5.5)

**Fase**: v1 — Raw Port. Assinaturas exatas do gdext 0.5.5 usadas pelos dois ports e decisões de
tradução, com alternativas descartadas. Fontes: crate `~/.cargo/registry/src/*/godot-core-0.5.5/`
e `godot-macros-0.5.5/`; bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (único diretório
`godot-core-*/out` em 2026-09-15; se houver outro, vale `ls -dt .../godot-core-*/out | head -1`).

**Método de confirmação**: todo o mapeamento foi escrito como módulo temporário
(`zz_research.rs`, classes `ZzPart`/`ZzRedRobot`) dentro do crate, compilado com `cargo build`
→ **0 erros, 0 warnings** após quatro correções que o compilador impôs (D3, D4, D12, D13 — registradas
abaixo porque o input do comando as tinha diferente), e exercitado em headless por scripts `-s`
(§E). Módulo e alteração temporária de visibilidade em `player.rs` revertidos (`git status` limpo).

## D1 — Estrutura e visibilidade

- **Decisão**: `src/part.rs` (`Part: RigidBody3D`), `src/red_robot.rs` (`RedRobot: CharacterBody3D`);
  `mod part;` e `mod red_robot;` em `lib.rs`. Em `part.rs`, `explode` é `#[func] pub(crate)` desde
  o port 1 (`#[func]` porque `red_robot.gd` chama por nome até o port 2; `pub(crate)` para o
  acesso tipado do port 2 — declarado já no port 1 para que o commit do robô não toque em
  `part.rs`). Em `player.rs`, `add_camera_shake_trauma` passa a `pub(crate)` **no commit do robô**
  (só a palavra de visibilidade; precedente do Marco B, D1).
- **Nomes**: `Part` e `RedRobot` conferidos contra `out/classes/` — não existem `part.rs` nem
  `red_robot.rs` nas bindings; sem colisão.
- **Alternativas descartadas**: acesso à peça via `call("explode")` (violaria FR-018);
  `pub(crate)` em `player.rs` já no port 1 (misturaria stories).

## D2 — `fade_value` com setter que aplica ao shader

- **Decisão**:
  ```rust
  #[export] #[var(set = set_fade_value)] fade_value: f32,   // default 0.0
  _mat: Option<Gd<Material>>,
  // bloco #[godot_api] impl Part principal:
  #[func]
  fn set_fade_value(&mut self, value: f32) {
      self.fade_value = value;
      if let Some(mat) = &self._mat {
          mat.get_next_pass().unwrap().cast::<ShaderMaterial>()
              .set_shader_parameter("emission_cutout", &value.to_variant());
      }
  }
  ```
  Em `process`, o `fade_value = pow(...)` do GDScript (que aciona o setter) vira
  `self.set_fade_value(fade)` — chamada direta ao setter, não atribuição ao campo.
- **Racional**: `#[var(set = ...)]` no bloco principal (godot-macros `lib.rs:184-215, 1257`);
  `Material::get_next_pass() -> Option<Gd<Material>>` (`material.rs:159`);
  `ShaderMaterial::set_shader_parameter(param, &Variant)` (`shader_material.rs:169`). `_mat` é
  campo próprio e `get_next_pass()` devolve um `Gd` novo — sem conflito de borrow. Tipos `f32`
  (`float` do GDScript; alimenta shader e `powi`).
- **Confirmado empiricamente (§E.1)**: `set("fade_value", 0.5)` e `set_indexed("fade_value", 0.25)`
  (o caminho do `MultiplayerSynchronizer`, `red_robot.tscn:10419`) acionam o setter e o
  `emission_cutout` do `next_pass` **duplicado** muda; o material original não muda.
- **Correção imposta pelo compilador**: nenhuma aqui (o input já estava certo).

## D3 — `ready`: duplicar material e `next_pass` (só fora de servidor dedicado)

- **Decisão**:
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
- **Racional**: `Os::has_feature(tag) -> bool` (`os.rs:890`); `Node::get_child(idx) -> Option<Gd<Node>>`
  (`node.rs:368`); `MeshInstance3D::get_mesh() -> Option<Gd<Mesh>>` (`mesh_instance_3d.rs:182`);
  `Mesh::surface_get_material(i) -> Option<Gd<Material>>` / `surface_set_material(i, &mat)`
  (`mesh.rs:245,235`); `Material::set_next_pass(&mat)` (`material.rs:150`).
- **Correção imposta pelo compilador**: `Resource::duplicate()` (`resource.rs:336`, retorna
  `Option<Gd<Resource>>`) está **deprecado** em 0.5.5 e gera warning — usar
  `Gd::<T>::duplicate_resource() -> Gd<T>` (`obj/gd_duplicate.rs:160`), que já devolve o tipo
  certo (sem `cast`). Alternativa descartada: `duplicate()` + `#[allow(deprecated)]` (warning
  suprimido não é "0 warnings").
- **Confirmado empiricamente (§E.1)**: `material duplicated=true`, `next_pass duplicated=true`,
  `is_processing=false` após `ready`; `OS.has_feature("dedicated_server")` é **false** em
  `--headless` (o ramo de duplicação roda na validação headless).

## D4 — `explode()` da peça

- **Decisão**:
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
- **Racional**: `public_visibility` → `set_visibility_public(bool)` (`multiplayer_synchronizer.rs:281`);
  `freeze` → `set_freeze_enabled(bool)` (`rigid_body_3d.rs:717`); `set_linear_velocity`/
  `set_angular_velocity` (`:276,:294`); `randf() -> f64` (`godot::global`, `utilities.rs:812`);
  `await create_timer(..).timeout` → `connect_other` com callable *linked* (padrão Marco A D6:
  se a peça for liberada antes, o callable é invalidado). `$Col1`/`$Col2`/`$MultiplayerSynchronizer`
  ficam como `get_node_as` no ponto de uso (o original também usa `$` inline, não `@onready`).
- **`create_timer` retorna `Gd<SceneTreeTimer>` direto** (`scene_tree.rs:316`, sem `Option`) —
  não usar `.unwrap()`.

## D5 — `process` e RPC `destroy`

- **Decisão**: `process(&mut self, delta: f64)`: `let fade = (self._disappearing_counter / self.disappearing_time).powi(2); self.set_fade_value(fade); self._disappearing_counter += delta as f32; if self._disappearing_counter >= self.disappearing_time - 0.2 { self.base_mut().rpc("destroy", &[]); self.base_mut().set_process(false); }`.
  `#[rpc(authority, call_local, unreliable)] fn destroy`: `let mut puff: Gd<CpuParticles3D> = load::<PackedScene>("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn").instantiate_as::<CpuParticles3D>(); self.base().get_parent().unwrap().add_child(&puff); let origin = self.base().get_global_transform().origin; puff.set_global_position(origin); create_timer(0.2).signals().timeout().connect_other(&*self, |this| this.base_mut().queue_free());`
- **Racional**: `pow(x, 2.0)` → `powi(2)` (mesmo valor); a instância é tipada como
  `CpuParticles3D` (API base), **nunca** `PartDisappear` — o original tipa `puff` como
  `CPUParticles3D` e só usa `global_transform.origin` (D11 do Marco B). `add_child` sem
  `force_readable_name` (o original passa só um argumento). `preload` → `load` no ponto de uso.
- **Correção imposta pelo compilador**: nenhuma.

## D6 — Sinal `exploded` e blocos `#[godot_api]`

- **Decisão**: `#[signal] fn exploded();` dentro do **único** `#[godot_api] impl RedRobot`
  (junto com `#[func]`s e `#[rpc]`s); emissão `self.signals().exploded().emit();`.
- **Racional**: `#[signal]` e `#[rpc]` não são suportados em blocos secundários
  (godot-macros `lib.rs:1256`); `signals()` vem de `WithUserSignals` (`obj/traits.rs:606,639`,
  `&mut self`). O sinal é registrado com o nome do método, então `robot.exploded.connect(...)`
  em `level.gd:99` resolve por nome.
- **Confirmado empiricamente (§E.2)**: `has_signal("exploded") == true`; conexão a partir de
  GDScript e emissão recebida.

## D7 — Enum `State` e exports do robô

- **Decisão**: `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum State { Idle, Approach, Aim, Shooting }`
  (0..3 = `IDLE..SHOOTING`); `#[export] #[init(val = State::Idle)] state: State`;
  `#[export] test_shoot: bool`, `#[export] target_position: Vector3`, `#[export] #[init(val = 5)] health: i32`,
  `#[export] dead: bool`, `#[export] #[init(val = AIM_PREPARE_TIME)] aim_preparing: f32`.
  Internos: `#[init(val = SHOOT_WAIT)] shoot_countdown: f32`, `#[init(val = AIM_TIME)] aim_countdown: f32`,
  `player: Option<Gd<Node3D>>`, `orientation: Transform3D`.
- **Racional**: precedente D3 do Marco B. `health: int` → `i32` (comparação `== 0`,
  replicado como `int`). `player: Node3D = null` → `Option<Gd<Node3D>>` (tipo da referência
  preservado: pode ser um "Target", não um `Player`).
- **Confirmado empiricamente (§E.2)**: os 6 exports registrados com os tipos certos e defaults
  `false / (0,0,0) / 5 / 0 / false / 0.5`; `hint_string` de `state` = `Idle:0,Approach:1,Aim:2,Shooting:3`
  (rótulos Rust no inspector — cosmético, mesmo caso do Marco B).

## D8 — Constantes e referências de cena

- Consts `f32`: `PLAYER_AIM_TOLERANCE_DEGREES: f32 = 15.0_f32.to_radians()` (`to_radians` é
  `const fn` — compila), `SHOOT_WAIT = 6.0`, `AIM_TIME = 1.0`, `AIM_PREPARE_TIME = 0.5`,
  `BLEND_AIM_SPEED = 0.05`.
- `OnReady` (`#[init(node = ...)]`), caminhos completos a partir do robô (os `x.get_node(^"…")`
  encadeados do original viram caminhos compostos, como no Marco B D5):

  | Campo | Tipo | Caminho |
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
  | `death_shield1` / `death_shield2` / `death_head` | **`Part`** (tipado) | `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` |
  | `death_detach_spark1` / `2` | `CpuParticles3D` | `Death/DetachSpark1`, `Death/DetachSpark2` |

  `LaserEmber` é buscado dentro de `shoot()` por `get_node_as::<CpuParticles3D>("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber")`
  (o original usa `$...` inline, `red_robot.gd:124`). O original tipa as peças como
  `RigidBody3D`; o port tipa como `Part` porque o port 1 já as tornou classe Rust e FR-018 exige
  `explode()` tipado (`self.death_shield1.bind_mut().explode()`).

## D9 — Transformação inversa (`Vector3 * Transform3D`)

- **Decisão**: `let gt = self.base().get_global_transform(); let to_player_local = gt.basis.transposed() * (self.target_position - gt.origin);`
  e, para o canhão, `let mt = self.ray_mesh.get_global_transform(); let to_cannon_local = mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin);`.
- **Racional**: no Godot, `Vector3 * Transform3D` é `Transform3D::xform_inv` =
  `basis.xform_inv(v - origin)` = **transposta** da base aplicada a `v − origin` — só coincide
  com `affine_inverse() * v` quando a base é ortonormal (o `RayMesh` fica sob um esqueleto
  animado com escala). gdext: `Basis::transposed()` (`basis.rs:373`), `impl Mul<Vector3> for Basis`
  (`basis.rs:595`).
- **Confirmado empiricamente (§E.2)**, com base escalada: `v * t == transposed * (v − origin)`
  = `(-4.664, -2.577, 10.043)`; `affine_inverse() * v` = `(4.808, -11.914, 5.848)` — diferente.
- **Alternativa descartada**: `gt.affine_inverse() * v` — reproduziria outro valor sempre que
  houver escala.

## D10 — Trigonometria e eixos

- `atan2(a, b)` do GDScript → `a.atan2(b)` (`f32::atan2(self, other)` = atan2(self, other) —
  mesma ordem de argumentos): `to.x.atan2(to.z)`, `to.x.atan2(-to.z)`, `to.y.atan2(-to.z)`.
- `rad_to_deg(x)` → `x.to_degrees()`; `clamp(x, 0, 1)`/`clampf` → `f32::clamp`.
- `gt.basis.y` → `gt.basis.col_b()` (`basis.rs:487`); `absf` → `.abs()`;
  `a.distance_to(b)` → `Vector3::distance_to` (existe em `vector_macros.rs`); `(a - b).length()`.

## D11 — `AnimationTree`

- `animation_tree["parameters/state/transition_request"] = "turn_left"` →
  `self.animation_tree.set("parameters/state/transition_request", &"turn_left".to_variant())`.
- `animation_tree[param] = 1` com `param = "parameters/hit" + str(randi() % 3 + 1) + "/request"` →
  `let param = format!("parameters/hit{}/request", randi() % 3 + 1); self.animation_tree.set(&param, &1.to_variant());`
  (`randi() -> i64`; `1` = `AnimationNodeOneShot.ONE_SHOT_REQUEST_FIRE`, inteiro como no original).
- `animation_tree.get("parameters/aim/blend_position")` → `.get(..).to::<Vector2>()`;
  `set("parameters/aiming/blend_amount", &x.to_variant())` com `x: f32`;
  `$AnimationTree.active = true` → `self.animation_tree.set_active(true)` (`animation_mixer.rs:114`).
- Root motion e o bloco final de física: idênticos ao Player (Marco B D8–D10), exceto que a base
  final vai para **o próprio robô**: `self.base_mut().set_global_basis(basis)`.

## D12 — Raycasts com exclusão efetiva

- **Decisão**: cada um dos três raycasts do original (`red_robot.gd:115,209,233`) é traduzido
  **inline**, no ponto de uso, sem helper:
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
- **Racional**: `[self]` no original é o RID do próprio `CharacterBody3D` — `CollisionObject3D::get_rid()`
  (`collision_object_3d.rs:173`); a exclusão **é** efetiva (diferente do quirk do
  `player_input.gd`, onde o `self` era um `MultiplayerSynchronizer` sem RID). `col.collider == player`
  compara identidade de objeto → `instance_id()` (`obj/gd.rs:301`); `Gd<Object>` e `Gd<Node3D>`
  são tipos diferentes, por isso não se usa `==` direto. `col.position` → `col.get("position").unwrap().to::<Vector3>()`.
  `create_ex(..).done()` devolve `Option<Gd<PhysicsRayQueryParameters3D>>` (static, como no
  Marco A D11).
- **Correção imposta pelo compilador**: em 0.5.5 `Dictionary` é genérico (`Dictionary<K, V>`);
  `intersect_ray` retorna **`VarDictionary`** (`= Dictionary<Variant, Variant>`,
  `physics_direct_space_state_3d.rs:46`, `dictionary.rs:127`) — usar esse nome.
- **Alternativa descartada**: helpers privados `intersect_ray(from, to)` / `collider_is(col, node)`
  (compilados no rascunho) — funcionam, mas seriam extração de código repetido, ou seja,
  refatoração (Princípio I). Ficam para a v2.

## D13 — `shoot()`: laser, `LaserEmber`, `Blast`, tremor com atraso

- `ray_dir = gt.basis.col_b()`; `max_dist: f32 = 1000.0`; `ray_origin.distance_to(position)`.
- `_clip_ray(length)`: `if !Os::singleton().has_feature("dedicated_server") { self.ray_mesh.get_surface_override_material(0).unwrap().cast::<ShaderMaterial>().set_shader_parameter("clip", &(length + mesh_offset).to_variant()); }`
  com `mesh_offset = self.ray_mesh.get_position().z` (`mesh_instance_3d.rs:258`).
- `LaserEmber`: `set_position(Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset))`;
  `let mut e = get_emission_box_extents(); e.z = (max_dist - mesh_offset.abs()) / 2.0; set_emission_box_extents(e)`
  (`cpu_particles_3d.rs:753,744` — o original escreve `.z` de uma propriedade `Vector3`, que é
  ler/modificar/gravar).
- `Blast`: `let mut blast: Gd<Node3D> = load::<PackedScene>("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn").instantiate_as::<Node3D>(); self.base().get_tree().get_root().unwrap().add_child(&blast); blast.set_global_position(position);`
  — API base, nunca `Blast`.
- `if col.collider == player and player is Player: await 0.1 s; player.add_camera_shake_trauma(13.0)` →
  ```rust
  if let Some(player) = self.player.clone() {
      if hit_player {   // col.collider == player (D12, inline)
          if let Ok(player) = player.try_cast::<Player>() {
              self.base().get_tree().create_timer(0.1).signals().timeout()
                  .connect_other(&*self, move |_this: &mut RedRobot| {
                      player.clone().bind_mut().add_camera_shake_trauma(13.0);
                  });
          }
      }
  }
  ```
  O closure captura `Gd<Player>` por `move`; `Gd::clone()` é necessário porque o callable
  pode ser chamado por `&Fn`. `add_camera_shake_trauma` precisa de `pub(crate)` em `player.rs`
  (D1). `pass # Kill.` fica sem efeito (nenhum código).
- **Correção imposta pelo compilador**: `body.get_name() == "Target".into()` é ambíguo
  (`StringName: PartialEq<_>` tem várias impls) — usar `body.get_name() == StringName::from("Target")`.

## D14 — Handlers de área e `hit`

- `_on_area_body_entered(body: Gd<Node3D>)`: `if body.clone().try_cast::<Player>().is_ok() || body.get_name() == StringName::from("Target") { self.player = Some(body); self.state = State::Approach; }`
  (`clone()` necessário porque `try_cast` consome e `body` ainda é guardado).
  `_on_area_body_exited(body)`: `if body.try_cast::<Player>().is_ok() { self.player = None; self.state = State::Idle; }`.
- `hit` (`#[rpc(authority, call_local, unreliable)]`): tradução de `red_robot.gd:78-105`; os
  10 s → `create_timer(10.0).signals().timeout().connect_other(&*self, |this| this.base_mut().queue_free())`
  só se `is_server()`; `self.model.set_visible(false)` / `self.death.set_visible(true)`
  (`node_3d.rs:620`); peças por `self.death_shield1.bind_mut().explode()`.
- `physics_process`: `let Some(player) = self.player.clone() else { ...; return; };` reproduz
  `if not player: ... return` e dá um `Gd<Node3D>` para o resto do frame. Sem jogador:
  `set_velocity(get_gravity() * delta)` **substitui** a velocidade (o original também).

## D15 — O que NÃO muda (quirks preservados, FR-027)

`body.name == "Target"` (nenhum node com esse nome existe); `pass # Kill.`; `player: Node3D`
(não `Player`); `aim_preparing`/`test_shoot` exportados sem replicação; `await` de 10 s dentro do
RPC `hit`; puff instanciado no pai da peça (`Death`); `angular_velocity` com `randf()` por eixo
(distribuição do original); `explode()` das peças chamado mesmo em peers não-servidor (retorna
cedo). Candidatos ao backlog em §"Backlog v2 candidato".

## §E — Verificação empírica (2026-09-15, headless, sem editor aberto)

### E.1 Peça (`ZzPart` programática: `Model/MeshInstance3D` com `BoxMesh` + `StandardMaterial3D` cujo `next_pass` é um `ShaderMaterial` com `uniform float emission_cutout`)

```
is_node_ready=true material duplicated=true next_pass duplicated=true is_processing=false
set(): fade_value=0.5 emission_cutout(dup)=0.5 (original)=<null>      ← setter via Object.set
set_indexed(): emission_cutout(dup)=0.25                                ← caminho do MultiplayerSynchronizer
has explode=true destroy=true set_fade_value=true
props lifetime/lifetime_random/disappearing_time/fade_value: type=3 (FLOAT) usage=6 (DEFAULT), defaults 3.0/3.0/0.5
OS.has_feature("dedicated_server")=false em --headless
```

(Observação de método: os checks precisam rodar em `_process` do `SceneTree`, não em `_init` —
`_ready` dos nodes só dispara depois que a árvore começa a processar.)

### E.2 Robô (`ZzRedRobot.new()`)

```
has_signal exploded=true; conexão GDScript por nome + emit_signal → recebido
has_method: hit, play_shoot, shoot_check, resume_approach, _on_area_body_entered, _on_area_body_exited = true;
            shoot, animate, _clip_ray = false (privados)
props: test_shoot BOOL false; target_position VECTOR3 (0,0,0); health INT 5; state INT 0 hint 'Idle:0,Approach:1,Aim:2,Shooting:3'; dead BOOL false; aim_preparing FLOAT 0.5
set("state", 2) → 2
xform_inv (base com escala): v*t = transposed*(v−origin) = (-4.664, -2.577, 10.043); affine_inverse*v = (4.808, -11.914, 5.848)
```

### E.3 Baseline (commit `4bb8f7f`, antes de qualquer port deste marco)

`cargo build` 0 warnings; import com `Initialize godot-rust` e 0 `ERROR`;
`enemies/red_robot/red_robot.tscn` exit 124, grep de regressão vazio, 1 WARNING (HDR);
`level/level.tscn` exit 124, grep vazio, 2 WARNINGs (HDR, Physics interpolation).

## Mapa por script (resumo para as tasks)

### 1. `part.gd` → `src/part.rs` — `Part: RigidBody3D`

- Campos: `_mat: Option<Gd<Material>>`; `#[export] #[init(val = 3.0)] lifetime: f32`;
  `#[export] #[init(val = 3.0)] lifetime_random: f32`; `#[export] #[init(val = 0.5)] disappearing_time: f32`;
  `#[export] #[var(set = set_fade_value)] fade_value: f32`; `_disappearing_counter: f32`.
- `IRigidBody3D`: `ready` (D3), `process` (D5).
- `#[godot_api] impl Part`: `#[func] set_fade_value` (D2), `#[func] pub(crate) explode` (D4),
  `#[rpc(authority, call_local, unreliable)] destroy` (D5).

### 2. `red_robot.gd` → `src/red_robot.rs` — `RedRobot: CharacterBody3D`

- Enum `State`, consts (D7–D8), 6 exports, 4 internos, 15 `OnReady` (D8).
- `ICharacterBody3D`: `ready`, `physics_process` (D14, D11).
- `#[godot_api] impl RedRobot` único: `#[signal] exploded`, `#[func] resume_approach`,
  `#[rpc] hit`, `#[rpc] play_shoot`, `#[func] shoot_check`, `#[func] _on_area_body_entered`,
  `#[func] _on_area_body_exited`.
- `impl RedRobot` privado: `shoot` (D12–D13), `animate` (D9–D11), `_clip_ray` (D13); os três
  raycasts inline (D12).
- `player.rs`: `add_camera_shake_trauma` → `pub(crate)` (D1), no commit do robô.

## Backlog v2 candidato (registrar em `docs/v2-backlog.md` no commit do script correspondente)

Itens 1–14 já existem. Novos:

| # | Origem | Melhoria | Motivação |
|---|---|---|---|
| 15 | `enemies/red_robot/parts/part.gd` (port 1) | Instanciar o puff no pai do **robô** (ou na raiz) em vez do pai da peça (`Death`) | O puff nasce sob o robô, que é removido 10 s após a morte; hoje os tempos não se cruzam, mas a dependência é frágil |
| 16 | `enemies/red_robot/red_robot.gd` (port 2) | Remover o ramo morto `body.name == "Target"` e tipar `player` como `Gd<Player>` | Nenhuma cena tem node `Target`; a referência genérica obriga `try_cast` em cada uso |
| 17 | `enemies/red_robot/red_robot.gd` (port 2) | Substituir o `await` de 10 s dentro do RPC `hit` por timer/sinal fora do RPC | Lógica de remoção acoplada ao handler de dano |
| 18 | `enemies/red_robot/red_robot.gd` (port 2) | Replicar `aim_preparing` (ou não exportá-lo) e remover `test_shoot` do inspector | Exportados mas fora da `SceneReplicationConfig`; `test_shoot` é gatilho interno do method track |
