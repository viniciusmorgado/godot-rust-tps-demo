# Research: mapeamento GDScript → gdext 0.5.5 (Marco A)

**Fase**: v1 raw port. **Data**: 2026-09-15.

Fontes conferidas (todas locais, nenhuma de memória):

- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/godot-macros-0.5.5/src/` (atributos
  `#[class]`, `#[init]`, `#[export]`, `#[rpc]`);
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/godot-core-0.5.5/src/` (`OnReady`,
  sinais tipados, `Callable`, `Variant`, `Basis`, `AsArg`);
- bindings geradas: `oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/`
  (`classes/*.rs`, `utilities.rs`) — o hash muda a cada rebuild limpo; usar `ls -d .../godot-core-*/out`;
- comportamento do engine verificado empiricamente em Godot 4.7.2 headless (§E).

Toolchain confirmada: rustc/cargo 1.98.1; `godot` 0.5.5 com API prebuilt padrão **4.6**
(`godot-bindings-0.5.5/src/import.rs:72` — `pub use gdextension_api::version_4_6 as prebuilt;`
quando nenhuma feature `api-4-x` está ativa). Não alterar.

## Decisões

### D1 — Esqueleto de classe

**Decision**: para cada script,

```rust
#[derive(GodotClass)]
#[class(init, base=<Base>)]
struct <Nome> { base: Base<<Base>>, /* campos */ }

#[godot_api]
impl I<Base> for <Nome> { fn ready(&mut self) {..} fn process(&mut self, delta: f64) {..} fn input(&mut self, event: Gd<InputEvent>) {..} }

#[godot_api]
impl <Nome> { #[func] fn ... }
```

Assinaturas dos virtuais (bindings `classes/node.rs:46,54,62`): `fn ready(&mut self)`,
`fn process(&mut self, delta: f64)`, `fn input(&mut self, event: Gd<InputEvent>)`.

Traits de interface existentes (`classes/*.rs:27`): `ILabel`, `ICpuParticles3D`, `INode3D`,
`ICamera3D`, `IMultiplayerSynchronizer`. **Atenção à grafia gdext**: a classe é `CpuParticles3D`
(não `CPUParticles3D`) — `base=CpuParticles3D`, `impl ICpuParticles3D`, `Gd<CpuParticles3D>`.

Nome registrado no Godot = nome do struct (sem `rename`). `player.gd:26` exige exatamente
`PlayerInputSynchronizer`.

**Rationale**: é o único esqueleto suportado; `#[class(init)]` gera o construtor com
`Default`/`#[init(val)]`/`OnReady`, exatamente o que `var x = ...` faz no GDScript.
**Alternatives**: `#[class(no_init)]` + `fn init(base)` manual — mais código sem ganho; rejeitado.

### D2 — `@onready var x = $Path` → `OnReady`

**Decision**: `#[init(node = "Path")] x: OnReady<Gd<T>>`. Confirmado em
`godot-macros-0.5.5/src/class/derive_godot_class.rs:765` (`OnReady::from_node(path)`);
`OnReady::from_node(path: impl AsArg<NodePath>)` em `godot-core/src/obj/on_ready.rs:146`.
Resolvido automaticamente antes de `ready()`. Acesso via `Deref`/`DerefMut`
(`on_ready.rs:263,290`): `self.light_rays.look_at(..)`.

Usos: `part_disappear` → `mini_blasts: OnReady<Gd<CpuParticles3D>>` (`"MiniBlasts"`);
`blast` → `light_rays: OnReady<Gd<CpuParticles3D>>` (`"LightRays"`),
`animation_player: OnReady<Gd<AnimationPlayer>>` (`"AnimationPlayer"`).

Exceção: `blast.gd`'s `@onready var camera = get_tree().get_root().get_camera_3d()` não é um
`$Path` → campo `camera: Option<Gd<Camera3D>>` preenchido no início de `ready()` (D9).

**Alternatives**: `get_node_as::<T>("Path")` dentro de `ready()` — funciona, mas `OnReady` é a
tradução direta de `@onready`; rejeitado por ser menos literal.

### D3 — `@export var x: <NodeType>` preenchido por `node_paths` → `Option<Gd<T>>`

**Decision**: `#[export] camera_base: Option<Gd<Node3D>>` etc. `impl<T> Export for Option<Gd<T>>`
em `godot-core/src/obj/gd.rs:1249` (hint de node class via `as_node_class()`), portanto o Godot
grava/lê como NodePath e resolve o node ao instanciar a cena — os `node_paths` de
`player.tscn:343,346–351` continuam válidos. Acesso: `self.camera_base.as_mut().unwrap()` /
`.as_ref().unwrap()` — no original, um `null` aqui também é erro em runtime; o `unwrap` reproduz
isso (panic capturado pelo gdext e impresso como erro, sem derrubar o jogo).

Os seis campos: `camera_animation: Option<Gd<AnimationPlayer>>`, `crosshair: Option<Gd<TextureRect>>`,
`camera_base: Option<Gd<Node3D>>`, `camera_rot: Option<Gd<Node3D>>`, `camera_camera: Option<Gd<Camera3D>>`,
`color_rect: Option<Gd<ColorRect>>`.

**Alternatives**: `OnEditor<Gd<T>>` (`gd.rs:1303`) — exporta como "obrigatório" e panica se não
preenchido; semanticamente próximo, mas o input do comando decidiu `Option`, e `Option` é a
representação 1:1 de uma referência GDScript que pode ser `null`. Rejeitado. Item candidato ao
backlog v2 (§Backlog).

### D4 — `@export var aiming: bool` etc. → `#[export]` de valor

**Decision**: `#[export] aiming: bool`, `#[export] shoot_target: Vector3`, `#[export] motion: Vector2`,
`#[export] shooting: bool`, `#[export] jumping: bool`. `#[export]` gera getter+setter, então
`player.gd:114` (`player_input.jumping = false`) continua funcionando, e o `MultiplayerSynchronizer`
replica `aiming/motion/shooting/shoot_target` por nome (`player.tscn:42–53`). `jumping` continua
exportado mas fora da replicação, como no original (comentário "handled via RPC").

Campos internos sem anotação: `toggled_aim: bool`, `aiming_timer: f32` (D13 para o tipo).

**Alternatives**: `#[var]` (sem export) — replicação funcionaria igual, mas mudaria a visibilidade no
inspector em relação ao original (`@export`). Rejeitado.

### D5 — `@rpc("call_local") func jump()` → `#[rpc]`

**Decision**: `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.jumping = true; }`
(dentro de `#[godot_api] impl PlayerInputSynchronizer`; `#[rpc]` implica `#[func]` —
`godot-macros/src/lib.rs:1055`). Chaves aceitas (`inherent_impl.rs:702–716`): `any_peer|authority`,
`reliable|unreliable|unreliable_ordered`, `call_local|call_remote`, `channel = N`.
Disparo: `self.base_mut().rpc("jump", &[])` — `Node::rpc(&mut self, method: impl AsArg<StringName>, varargs: &[Variant]) -> Error`
(`classes/node.rs:1480`).

**Correção em relação ao input do comando**: o input sugeria `reliable`. O default de
`@rpc` no GDScript é `mode="authority", sync="call_remote", transfer_mode="unreliable"`, e
`@rpc("call_local")` só altera `sync`. O default do `#[rpc]` gdext é o mesmo (tabela em
`godot-macros/src/lib.rs:1046–1051`: `authority`, `call_remote`, `unreliable`). Usar `reliable`
mudaria o comportamento de rede em relação ao original — proibido na v1. Declarar as três chaves
explicitamente (`authority, call_local, unreliable`) documenta a intenção e é idêntico a
`#[rpc(call_local)]`.

**Alternatives**: `#[rpc(config = CONST)]` — equivalente, mais verboso; rejeitado.
`reliable` — altera comportamento; rejeitado.

### D6 — `await get_tree().create_timer(t).timeout` → sinal tipado

**Decision**:

```rust
let timer = self.base().get_tree().create_timer(0.2);          // Gd<SceneTreeTimer>
timer.signals().timeout().connect_other(&*self, |this: &mut PartDisappear| { ... });
```

- `Node::get_tree(&self) -> Gd<SceneTree>` (substituição type-safe,
  `godot-core/src/classes/type_safe_replacements.rs:84`);
- `SceneTree::create_timer(&mut self, time_sec: f64) -> Gd<SceneTreeTimer>` (`classes/scene_tree.rs:316`);
- `Gd::signals()` (`obj/gd.rs:1055`) → `timeout()` (`classes/scene_tree_timer.rs:118`), tupla de
  parâmetros `()`;
- `TypedSignal::connect_other(&self, object: &impl ObjectToOwned<OtherC>, method: F)`
  (`godot-core/src/signal/typed_signal.rs:316`) aceita `&Gd<OtherC>` **ou** `&OtherC` quando
  `OtherC` é classe de usuário com `Base` — dentro de `ready(&mut self)` passar `&*self`.
- A conexão usa uma callable **linked** ao objeto receptor
  (`inner_connect_godot_fn` → `bound.linked_callable(..)`, `typed_signal.rs:205–213`;
  `Callable::from_linked_fn` doc, `builtin/callable.rs:161–166`: "automatically invalidated by
  Godot when a linked object is freed"). Logo, se o efeito for liberado antes do timer disparar,
  a callable é invalidada pelo Godot e o closure não roda — sem panic. Equivale ao caso de borda
  da spec ("não deve produzir erros novos").

Encadeamento em `part_disappear` (dois `await` sequenciais): dentro do primeiro closure,
`this.base_mut().set_emitting(true)`, depois criar o segundo timer com
`this.base().get_tree().create_timer(this.base().get_lifetime() * 2.0)` e
`connect_other(&*this, |this2| this2.base_mut().queue_free())`. `CpuParticles3D::get_lifetime(&self) -> f64`
(`classes/cpu_particles_3d.rs:308`); `set_emitting(&mut self, bool)` (`:173`); `Node::queue_free`
(`classes/node.rs:1299`).

O primeiro comando do `_ready` (`$MiniBlasts.emitting = true`) roda antes de qualquer timer:
`self.mini_blasts.set_emitting(true)`.

**Alternatives**: (a) `Callable::from_object_method(&gd, "_on_timeout")` + `#[func]` — mesma
semântica de invalidação, mas adiciona nomes de método que não existem no original; rejeitado.
(b) `connect_self` — só para sinais do próprio objeto; N/A. (c) `godot::task::spawn` +
`.to_future()` — API assíncrona experimental; rejeitado (menos previsível, sem ganho).
(d) `builder().flags(ONE_SHOT)` — `SceneTreeTimer` é one-shot por natureza; desnecessário.

### D7 — `await $AnimationPlayer.animation_finished` → sinal tipado

**Decision**: em `ready()`:
`self.animation_player.signals().animation_finished().connect_other(&*self, |this: &mut Blast, _anim_name: StringName| this.base_mut().queue_free());`
Sinal herdado de `AnimationMixer` (`classes/animation_mixer.rs:863`, tupla `(StringName,)`
— `:935`). `impact_effect.tscn` tem `autoplay = "blast"`, o sinal chega uma vez. Callable linked
(D6) → seguro se o efeito for liberado antes.

**Alternatives**: iguais a D6.

### D8 — Orientar para a câmera a cada frame (`blast`)

**Decision**: campo `camera: Option<Gd<Camera3D>>`; em `ready()` (primeira linha, antes de conectar
o sinal, respeitando a ordem `@onready` → `_ready`):
`self.camera = self.base().get_tree().get_root().unwrap().get_camera_3d();`
— `SceneTree::get_root(&self) -> Option<Gd<Window>>` (`classes/scene_tree.rs:138`);
`Viewport::get_camera_3d(&self) -> Option<Gd<Camera3D>>` (`classes/viewport.rs:1047`).
Em `process()`: `if let Some(cam) = &self.camera { if cam.is_instance_valid() { let origin = cam.get_global_transform().origin; self.light_rays.look_at(origin); } }`
— `Gd::is_instance_valid(&self) -> bool` (`obj/gd.rs:332`); `Node3D::look_at(&mut self, target: Vector3)`
(`classes/node_3d.rs:851`); `Transform3D { basis, origin }` campos públicos
(`builtin/matrices/transform3d.rs:58,61`).

`get_root().unwrap()`: o original também assume raiz presente; N/A fora da árvore.

### D9 — Singletons e utilitários (`debug`)

Todos via trait `Singleton` (`classes/engine.rs:465`: `fn singleton() -> Gd<Self>`):

| GDScript | gdext 0.5.5 | Fonte |
|---|---|---|
| `Input.is_action_just_pressed(&"toggle_debug")` | `Input::singleton().is_action_just_pressed("toggle_debug")` — `&str: AsArg<StringName>` (`meta/args/as_arg.rs:523–539`) | `classes/input.rs:118` |
| `Input.get_action_strength(&"x")` → `f32` | `get_action_strength(&self, action) -> f32` | `input.rs:202` |
| `Input.is_action_pressed` / `is_action_just_released` | idem, `-> bool` | `input.rs:97,139` |
| `Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)` | `Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED)` | `input.rs:565,1240` |
| `Engine.get_frames_per_second()` | `Engine::singleton().get_frames_per_second() -> f64` | `classes/engine.rs:135` |
| `DisplayServer.window_get_vsync_mode()` (truthy = ≠ 0) | `DisplayServer::singleton().window_get_vsync_mode() != godot::classes::display_server::VSyncMode::DISABLED` | `display_server.rs:2041,8235` |
| `OS.get_static_memory_usage()` | `Os::singleton().get_static_memory_usage() -> u64` | `classes/os.rs:699` |
| `visible = not visible` | `let v = self.base().is_visible(); self.base_mut().set_visible(!v);` | `canvas_item.rs:66,75` |
| `text = ...` | `self.base_mut().set_text(&s)` (`impl AsArg<GString>`; `&String` aceito) | `label.rs:254` |
| `multiplayer` | `self.base().get_multiplayer() -> Option<Gd<MultiplayerApi>>` | `node.rs:1369` |
| `multiplayer.get_unique_id()` | `-> i32` | `multiplayer_api.rs:53` |
| `multiplayer.multiplayer_peer is OfflineMultiplayerPeer` | `get_multiplayer_peer() -> Option<Gd<MultiplayerPeer>>` → `.map(\|p\| p.try_cast::<OfflineMultiplayerPeer>().is_ok())`; `online = !matches!(.., Some(true))` (peer `null` ⇒ `is` falso ⇒ online, como no original) | `multiplayer_api.rs:34`; `obj/gd.rs:538` |
| `get_multiplayer_authority()` | `self.base().get_multiplayer_authority() -> i32` | `node.rs:1350` |
| `set_process(false)` / `set_process_input(false)` | idem | `node.rs:779,835` |
| `color_rect.hide()` | `CanvasItem::hide` | `canvas_item.rs:105` |

**Formatação do texto (verificado empiricamente, §E)**:
- `str(Engine.get_frames_per_second())` → `"60.0"` (Godot acrescenta `.0` a floats inteiros).
  Tradução fiel: `Variant::from(fps).stringify()` (`builtin/variant/mod.rs:302`) — delega ao
  `str()` do próprio engine, idêntico por construção. Alternativa `format!("{:.1}", fps)` só é
  igual porque `_fps` é sempre inteiro; rejeitada por não ser `str()`.
- `"%3.2f" % x` → `format!("{:3.2}", x)` — saída idêntica (`117.74`, `0.50`).
- `str(int)` → `format!("{}", id)` — idêntico para inteiros.
- Concatenação: montar uma `String` Rust e `set_text(&text)` uma vez por frame (o original faz
  `text +=` quatro vezes; o resultado final é o mesmo — uma única atribuição não é otimização, é
  o modo natural de construir a string em Rust sem helper).

### D10 — Ruído (`camera_noise_shake`)

**Decision**: `noise: Gd<FastNoiseLite>` com `#[init(val = FastNoiseLite::new_gd())]`;
`noise_seed: i32` com `#[init(val = (randi() as i32))]` — `godot::global::randi() -> i64`
(`utilities.rs:802`); `FastNoiseLite::set_seed(&mut self, seed: i32)` (`fast_noise_lite.rs:156`),
então o `as i32` reproduz a conversão `int → int32` que o Godot faz ao atribuir a `seed`.
`set_fractal_octaves(&mut self, i32)` (`:228`), `set_fractal_lacunarity(&mut self, f32)` (`:246`),
`Noise::get_noise_1d(&self, x: f32) -> f32` (herdado, `classes/noise.rs:25`).
`noise_seed + 1` / `+ 2`: `wrapping_add` não é necessário (semântica de overflow do GDScript é
wrap em i64; aqui `i32` + 2 só estoura para `randi()` ≥ 2³¹−2, ≈ 1 em 10⁹ — usar `wrapping_add`
mesmo assim é gratuito e evita panic em debug; **decisão**: usar `wrapping_add`).

`start_rotation: Vector3` capturada em `ready()` (`self.base().get_rotation()`,
`node_3d.rs:247`); o inicializador `var start_rotation = rotation` do GDScript roda antes de o
node estar na árvore e é sobrescrito em `_ready` — o valor que importa é o do `_ready`; campo com
`Default` (zero) e atribuição em `ready()` reproduz o resultado.

Constantes: `const SPEED: f32 = 1.0; DECAY_RATE: f32 = 1.5; MAX_YAW: f32 = 0.05; MAX_PITCH: f32 = 0.05; MAX_ROLL: f32 = 0.1; MAX_TRAUMA: f32 = 1.2;` — `f32` porque alimentam `Vector3`/`real` (D13).

`add_trauma`: `#[func] fn add_trauma(&mut self, amount: f64)` — parâmetros `float` do GDScript
chegam como `f64`; `self.trauma = (self.trauma + amount as f32).min(MAX_TRAUMA)`.
`minf`/`maxf`/`clampf` do GDScript → `f32::min`/`max`/`clamp` (os utilitários Godot existem em
`utilities.rs:702,732,762` mas em `f64`; usar os métodos nativos evita conversões — não é
abstração).

`rotation = start_rotation + Vector3(pitch, yaw, roll)` → `self.base_mut().set_rotation(self.start_rotation + Vector3::new(pitch, yaw, roll))` (`node_3d.rs:238`).

### D11 — Raycast (`player_input`) — quirk FR-017 preservado

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

- `create_ex(from, to) -> ExCreate` com `.collision_mask(u32)`, `.exclude(&Array<Rid>)`, `.done() -> Option<Gd<..>>`
  (`physics_ray_query_parameters_3d.rs:140,378,384,390`);
- `Rid::Invalid` (`builtin/rid.rs:44`); `array![]` (`builtin/collections/array.rs:1545`);
- `Node3D::get_world_3d() -> Option<Gd<World3D>>` (`node_3d.rs:523`) no **pai** (`get_parent().unwrap().cast::<Node3D>()`);
  `World3D::get_direct_space_state() -> Option<Gd<PhysicsDirectSpaceState3D>>` (`world_3d.rs:219`);
  `intersect_ray(&mut self, params) -> VarDictionary` (`physics_direct_space_state_3d.rs:46`;
  `VarDictionary = Dictionary<Variant, Variant>`, `dictionary.rs:127`; `is_empty` `:274`, `get` `:183`).
- `camera_camera.project_ray_origin(Vector2) -> Vector3` / `project_ray_normal` (`camera_3d.rs:191,171`);
  `crosshair.get_position() + crosshair.get_size() * 0.5` (`control.rs:582,591`).

**Verificado em 4.7.2 (§E)**: `Array([self], TYPE_RID, "", null)` com `self` sendo um
`MultiplayerSynchronizer` produz `[RID(0)]` — o original **não exclui nada** de fato. Excluir o
corpo do jogador seria correção de bug (v1 proíbe). `exclude(&array![Rid::Invalid])` é a
tradução exata. Candidato ao backlog v2 (§Backlog).

### D12 — Eventos de mouse e rotação da câmera

- `fn input(&mut self, event: Gd<InputEvent>)`: `if let Ok(motion) = event.try_cast::<InputEventMouseMotion>() { let rel = motion.get_screen_relative(); .. }`
  (`input_event_mouse_motion.rs:219` → `Vector2`).
- `rotate_camera(move)`: método Rust **privado** (`fn rotate_camera(&mut self, mv: Vector2)`,
  sem `#[func]` — no original é chamado só internamente; nenhum GDScript o usa; manter sem
  `#[func]` não altera contrato, e o nome fica o mesmo). Corpo:
  `camera_base.rotate_y(-mv.x)` (`node_3d.rs:789`, `f32`), `camera_base.orthonormalize()` (`:819`),
  `let mut r = camera_rot.get_rotation(); r.x = (r.x + mv.y).clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); camera_rot.set_rotation(r);`.
- `CAMERA_X_ROT_MIN/MAX`: `const CAMERA_X_ROT_MIN: f32 = (-89.9_f32).to_radians();`
  `const CAMERA_X_ROT_MAX: f32 = 70.0_f32.to_radians();` — `f32::to_radians` é `const fn`
  (compilado e verificado com rustc 1.98.1, edition 2024: `-1.569051 1.2217305`). Alternativa
  `godot::global::deg_to_rad(f64) -> f64` (`utilities.rs:612`) não é `const`; rejeitada para
  constantes.
- `get_aim_rotation() -> f64` (`float` no GDScript; retornar `f64` mantém o tipo Godot `float`):
  clamp de `camera_rot.get_rotation().x` e as duas divisões, convertendo `as f64` no retorno.
- `get_camera_base_quaternion() -> Quaternion`: `camera_base.get_global_transform().basis.get_quaternion()`
  — `Basis::get_quaternion` é o equivalente documentado de `get_rotation_quaternion()`
  (`builtin/matrices/basis.rs:187–192`, `#[doc(alias = "get_rotation_quaternion")]`).
- `get_camera_rotation_basis() -> Basis`: `camera_rot.get_global_transform().basis`.
- `camera_animation.play("shoot")`: `play()` gerado sem argumentos (`animation_player.rs:317`);
  usar `play_ex().name("shoot").done()` (`:322,912,936`).
- Fade: `let mut m = color_rect.get_modulate(); m.a = ...; color_rect.set_modulate(m);`
  (`canvas_item.rs:170,179`; `Color.a: f32`, `builtin/color.rs:46`).
- Altura do pai: `self.base().get_parent().unwrap().cast::<Node3D>().get_global_transform().origin.y`.
- `_ready` autoridade: `if self.base().get_multiplayer_authority() == self.base().get_multiplayer().unwrap().get_unique_id() { camera_camera.make_current(); Input::singleton().set_mouse_mode(MouseMode::CAPTURED); } else { set_process(false); set_process_input(false); color_rect.hide(); }`
  — `Camera3D::make_current` (`camera_3d.rs:261`).

### D13 — Tipos numéricos

**Decision**: `delta: f64` (virtual); tudo que alimenta `Vector2/3`, `Color`, `rotate_y`,
`get_noise_1d` é `f32` (`real = f32`, `builtin/real.rs:43`; feature `double-precision` **não** está
ativa). Converter no ponto de uso com `as f32` / `as f64`, explicitamente, sem helper. Campos de
estado que só interagem com `real` (`trauma`, `time`, `aiming_timer`) ficam `f32`; retornos de
`#[func]` que no GDScript são `float` ficam `f64` (tipo Godot `float`).

Observação: no GDScript tudo é `float` 64-bit e `Vector3` é 32-bit — a conversão implícita
acontece na atribuição ao vetor; fazê-la no mesmo ponto em Rust preserva a precisão do original.
`time += delta * SPEED * 5000.0` acumula em `f32` no Rust vs `f64` no GDScript, mas o valor é
consumido por `get_noise_1d(x: f32)` de qualquer modo; após minutos de tremor contínuo o `f32`
perde resolução antes do `f64` — efeito imperceptível (ruído), e o tremor dura ~0,8 s por
evento. **Decisão**: `time: f64` (fiel ao `float` do GDScript), convertido `as f32` na chamada.

### D14 — Ordem de inicialização e hot-reload

- `#[class(init)]` inicializa campos → `OnReady` resolvidos → `ready()`. Igual a
  inicializadores de `var` → `@onready` → `_ready`.
- `reloadable = true`: após hot-reload, `OnReady` volta ao estado não-inicializado
  (`godot-macros/src/lib.rs:591–596`); irrelevante para as cenas do jogo (re-instanciadas), mas
  ao editar no editor com o jogo parado é esperado. Não afeta a validação headless.

## Mapa por script (campos, virtuais, métodos) — referência para tasks.md

| # | Arquivo | `#[class(init, base=…)]` | Campos | Virtuais | `#[func]`/`#[rpc]` |
|---|---|---|---|---|---|
| 1 | `debug_label.rs` | `DebugLabel`, `Label` | `base` | `process` | — |
| 2 | `part_disappear.rs` | `PartDisappear`, `CpuParticles3D` | `base`; `#[init(node="MiniBlasts")] mini_blasts: OnReady<Gd<CpuParticles3D>>` | `ready` | — |
| 3 | `blast.rs` | `Blast`, `Node3D` | `base`; `#[init(node="LightRays")] light_rays: OnReady<Gd<CpuParticles3D>>`; `#[init(node="AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>`; `camera: Option<Gd<Camera3D>>` | `ready`, `process` | — |
| 4 | `camera_noise_shake.rs` | `CameraNoiseShake`, `Camera3D` | `base`; `start_rotation: Vector3`; `trauma: f32`; `time: f64`; `#[init(val = FastNoiseLite::new_gd())] noise: Gd<FastNoiseLite>`; `#[init(val = (randi() as i32))] noise_seed: i32`; 6 `const` | `ready`, `process` | `#[func] add_trauma(&mut self, amount: f64)`; privados `decay_trauma(delta)`, `apply_shake(delta)`, `get_noise_value(seed, pos)` (sem `#[func]`: no original não são chamados de fora) |
| 5 | `player_input.rs` | `PlayerInputSynchronizer`, `MultiplayerSynchronizer` | `base`; `toggled_aim: bool`; `aiming_timer: f32`; `#[export]` ×5 valores (D4); `#[export]` ×6 `Option<Gd<..>>` (D3); 5 `const` | `ready`, `process`, `input` | `#[func] get_aim_rotation(&self) -> f64`; `#[func] get_camera_base_quaternion(&self) -> Quaternion`; `#[func] get_camera_rotation_basis(&self) -> Basis`; `#[rpc(authority, call_local, unreliable)] jump(&mut self)`; privado `rotate_camera(&mut self, Vector2)` |

Sobre métodos privados (`decay_trauma`, `apply_shake`, `get_noise_value`, `rotate_camera`): no
GDScript toda `func` é pública, mas nenhum outro script as chama (conferido:
`grep -rn 'decay_trauma\|apply_shake\|get_noise_value\|rotate_camera' oxide-godot/ --include=*.gd`
só encontra os próprios arquivos). Sem `#[func]` eles não aparecem para o Godot; **isso não muda
comportamento observável**. Se a revisão preferir fidelidade literal, adicionar `#[func]` a eles
é neutro — decisão para o implementador; a spec (FR-024) só exige preservar os nomes que são
consumidos. Registrar a escolha na mensagem do commit.

## §E — Verificações empíricas (Godot 4.7.2 headless, `--script`)

```
A|60.0|59.5|1.0|      str(60.0), str(59.5), str(Engine.get_frames_per_second()) em headless
B|117.74|0.50|        "%3.2f" % (123456789/1048576.0), "%3.2f" % 0.5
C|1|                  DisplayServer.window_get_vsync_mode() em headless (ENABLED=1)
D|[RID(0)]|1|         Array([self], TYPE_RID, "", null)  ← FR-017: exclusão inefetiva
E|-1|0.936|           str(int(-1)); 1.0 - 0.016*4.0
```

Script usado: `scratchpad/fmt_test.gd` (`extends SceneTree`, `_init` imprime e `quit()`).

## Verificações de nomes contra o GDScript restante

| Consumidor | Linha | Nome usado | Existe no contrato |
|---|---|---|---|
| `player.gd` | 26 | tipo `PlayerInputSynchronizer` | ✅ nome do struct |
| `player.gd` | 41 | `$InputSynchronizer.set_multiplayer_authority()` | API base de Node |
| `player.gd` | 73 | `player_input.get_aim_rotation()` | ✅ |
| `player.gd` | 87 | `player_input.motion` | ✅ |
| `player.gd` | 89 | `player_input.get_camera_rotation_basis()` | ✅ |
| `player.gd` | 107, 114 | `player_input.jumping` (leitura e escrita) | ✅ |
| `player.gd` | 121 | `player_input.aiming` | ✅ |
| `player.gd` | 124 | `player_input.get_camera_base_quaternion()` | ✅ |
| `player.gd` | 133, 135 | `player_input.shooting`, `player_input.shoot_target` | ✅ |
| `player.gd` | 211 | `player_input.camera_camera.add_trauma(amount)` | ✅ (`camera_camera` + `add_trauma`) |
| `red_robot.gd` | 133 | `player.add_camera_shake_trauma(13.0)` → `player.gd:210` → `add_trauma` | ✅ |
| `player.tscn` | 42–53 | replicação `shoot_target`, `motion`, `shooting`, `aiming` | ✅ |
| `player.tscn` | 343–351 | `node_paths` ×6 | ✅ |

Nota de análise estática: `player_input.camera_camera` é tipado `Camera3D` no Godot; chamar
`.add_trauma()` nele gera no máximo o warning `UNSAFE_METHOD_ACCESS` do GDScript — exatamente
como hoje (o script `camera_noise_shake_effect.gd` não tem `class_name`). Nenhuma mudança.

## Backlog v2 candidato (registrar em `docs/v2-backlog.md` no commit do script correspondente)

| Origem | Melhoria | Motivação |
|---|---|---|
| `player_input.gd` (port 5) | Excluir de fato o corpo do jogador no raycast (`exclude` com o RID do `CharacterBody3D` pai) | O original passa `[RID(0)]` — exclusão inefetiva; o tiro pode acertar o próprio corpo em ângulos extremos |
| `player_input.gd` (port 5) | `OnEditor<Gd<T>>` em vez de `Option<Gd<T>>` para as 6 referências obrigatórias | Elimina `unwrap()` por frame e faz o editor sinalizar referência ausente |
| `player_input.gd` (port 5) | Replicar `jumping` ou remover o `@export` | Exportado mas fora da replicação; hoje só faz sentido via RPC |
| `camera_noise_shake_effect.gd` (port 4) | Recapturar `start_rotation` quando animações/scripts movem a câmera | Comentário do original admite o problema; tremor soma à rotação capturada uma única vez |
| `debug.gd` (port 1) | Não recalcular o texto enquanto o overlay está oculto | Trabalho por frame desnecessário |
| `part_disappear.gd` / `blast.gd` (ports 2–3) | Timers/sinais como `async` (`godot::task`) quando a API estabilizar | Encadeamento de closures reproduz `await` de forma menos legível |
