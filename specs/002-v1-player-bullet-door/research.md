# Research: Marco B — jogador, bala e porta (gdext 0.5.5)

**Fase**: v1 — Raw Port. Este documento fixa as assinaturas exatas do gdext 0.5.5 usadas pelos três
ports e as decisões de tradução, com alternativas descartadas. Fontes: crate
`~/.cargo/registry/src/*/godot-core-0.5.5/` e `godot-macros-0.5.5/`; bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-aea5c50e7fda9d57/out/` (único diretório
`godot-core-*/out` existente em 2026-09-15 — se surgir outro, vale `ls -dt .../godot-core-*/out | head -1`).
Linhas citadas como `arquivo.rs:N` referem-se a esses diretórios.

**Método de confirmação**: todo o mapeamento abaixo foi escrito como um módulo temporário
(`zz_research.rs`, três classes `ZzPlayer`/`ZzBullet`/`ZzDoor`) dentro do crate, compilado com
`cargo build` → **0 erros, 0 warnings**, e exercitado em headless por um script `-s` (§E). O módulo
e as alterações de visibilidade temporárias foram revertidos (`git status` limpo) — nada disso é
implementação; a implementação acontece nas tasks.

## D1 — Estrutura: três módulos, visibilidade `pub(crate)` no que o Player consome

- **Decisão**: `src/player.rs` (`Player: CharacterBody3D`), `src/bullet.rs`
  (`Bullet: CharacterBody3D`), `src/door.rs` (`Door: Area3D`); `mod` em `lib.rs`. Em
  `player_input.rs`, os campos `aiming`, `shoot_target`, `motion`, `shooting`, `jumping`,
  `camera_camera` e os métodos `get_aim_rotation`, `get_camera_base_quaternion`,
  `get_camera_rotation_basis` passam de privados a `pub(crate)`; em `camera_noise_shake.rs`,
  `add_trauma` idem. Só a palavra de visibilidade muda — nenhuma linha se move, nenhum corpo muda.
  A alteração entra no commit do port 1 (Player) e é citada na mensagem.
- **Racional**: FR-010/FR-011 exigem acesso **tipado** (`self.player_input.bind().motion`,
  `.bind_mut().jumping = false`, `camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(..)`).
  Em Rust, campos e métodos sem `pub` são privados ao módulo; `pub(crate)` é o mínimo que torna
  o acesso possível sem expor nada fora do crate. Não é abstração nem refatoração: nenhuma
  lógica é extraída ou movida (registrado em plan.md §Complexity Tracking como não-violação).
- **Alternativas descartadas**: (a) acesso via `Variant` (`player_input.get("motion")`) —
  viola FR-011 e o Princípio II (acesso dinâmico entre classes já em Rust); (b) mover
  `PlayerInputSynchronizer` para dentro de `player.rs` — mexeria em código já commitado sem
  necessidade; (c) `pub` irrestrito — desnecessário, o crate é `cdylib`.
- **Nomes**: `Player` obrigatório (FR-001). `Bullet` e `Door` conferidos contra a lista de
  classes das bindings (`out/classes/`): não existe `bullet.rs` nem `door.rs` — sem colisão.

## D2 — `player_id` com setter que roda fora da árvore

- **Decisão**:
  ```rust
  #[export]
  #[var(set = set_player_id)]
  #[init(val = 1)]
  player_id: i32,
  // ... no bloco #[godot_api] impl Player PRINCIPAL:
  #[func]
  fn set_player_id(&mut self, value: i32) {
      self.player_id = value;
      self.base()
          .get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")
          .set_multiplayer_authority(value);
  }
  ```
- **Racional**: `#[var(set = nome)]` é a forma documentada (godot-macros `lib.rs:184-215`);
  o setter DEVE ser `#[func]` e ficar no bloco `#[godot_api] impl` principal
  (`lib.rs:1257`: blocos secundários não podem ser referenciados por `get`/`set`). O setter
  roda **antes** de `ready` (`level.gd:119` atribui `player_id` com o node fora da árvore), por
  isso não usa o `OnReady` `player_input` (ainda não inicializado) e sim `get_node_as` com
  caminho relativo — que funciona fora da árvore porque os filhos da cena instanciada já
  existem. Tipo `i32`: `set_multiplayer_authority(id: i32)` (`node.rs:1341`) e
  `get_unique_id() -> i32`; a conversão `int` GDScript → `i32` é feita pelo gdext na fronteira.
- **Confirmado empiricamente (§E.1)**: com o node fora da árvore, `p.player_id = 7` e
  `p.set("player_id", 9)` (a forma que `level.gd` usa, pois tipa `player` como
  `CharacterBody3D`) atualizam a autoridade do filho.
- **Alternativas descartadas**: `try_get_node_as` (esconderia a falta do filho; o original
  também erra nesse caso); guardar `player_id` sem efeito colateral e aplicar em `ready`
  (mudaria o momento da atribuição — melhoria/mudança de comportamento).
- **Efeito colateral inevitável**: `set_player_id` fica visível como método Godot
  (`has_method("set_player_id") == true`). O GDScript original expõe o setter inline de forma
  diferente; nenhum consumidor chama esse nome — não observável no jogo.

## D3 — `Animations` como enum exportado

- **Decisão**:
  ```rust
  #[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
  #[godot(via = i64)]
  pub enum Animations { JumpUp, JumpDown, Strafe, Walk }   // 0, 1, 2, 3
  // campo:
  #[export]
  #[init(val = Animations::Walk)]
  current_animation: Animations,
  ```
- **Racional**: `derive(GodotConvert)` + `#[godot(via = i64)]` gera discriminantes 0.. na
  ordem de declaração (`lib.rs:1494-1508`); `Var`/`Export` deriváveis para enums c-like
  (`lib.rs:1532,1540`). A propriedade é registrada com o nome do campo `current_animation`,
  tipo `int`, hint `ENUM` — é o que `player.tscn:29` (`.:current_animation`) replica.
- **Confirmado empiricamente (§E.1)**: default `3` (= WALK), `set`/`get` por nome funcionam,
  `hint_string = "JumpUp:0,JumpDown:1,Strafe:2,Walk:3"`.
- **Diferença cosmética aceita**: no inspector os rótulos são `JumpUp…` em vez de `JUMP_UP…`
  (o valor gravado/replicado é o inteiro; a cena não grava o default). Não é observável no jogo.
- **Alternativas descartadas**: `i32` cru com constantes — perderia o hint e a legibilidade sem
  ganho; variantes `JUMP_UP` com `#[allow(non_camel_case_types)]` — não é idiomático nem
  necessário.
- **`animate(anim: Animations, _delta: f64)`** privado (spec: método interno). A comparação
  `anim == Animations::JumpUp` exige `PartialEq` (derivado acima). Nos clientes,
  `physics_process` faz `let anim = self.current_animation; self.animate(anim, delta)` (cópia
  antes do `&mut self`; `Copy` derivado).

## D4 — `motion` replicado sem ser exportado → `#[var]`

- **Decisão**: `#[var] motion: Vector2`.
- **Racional**: `player.tscn:26` replica `.:motion`; o `MultiplayerSynchronizer` lê/escreve por
  `get_indexed`/`set_indexed`, que só enxergam propriedades **registradas**. `#[var]` registra
  com `PropertyUsageFlags::NONE` (`registry/info/property_info.rs:125-128`): acessível por nome,
  não aparece no inspector nem é gravado na cena — mesmo efeito do `var` de script do original.
- **Confirmado empiricamente (§E.1)**: `get("motion")`, `set("motion", ..)`,
  `set_indexed("motion", ..)`, `get_indexed("motion")` funcionam; `usage = 0`.
- **Alternativa descartada**: `#[export]` — apareceria no inspector e seria gravado na cena
  (o original não é `@export`).

## D5 — Referências de cena (`@onready`)

- **Decisão**: `#[init(node = "...")] OnReady<Gd<T>>`, caminhos relativos ao node:

  | Campo | Tipo | Caminho |
  |---|---|---|
  | `player_input` | `PlayerInputSynchronizer` | `InputSynchronizer` |
  | `animation_tree` | `AnimationTree` | `AnimationTree` |
  | `player_model` | `Node3D` | `PlayerModel` |
  | `shoot_from` | `Marker3D` | `PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom` |
  | `crosshair` | `TextureRect` | `Crosshair` (declarado e nunca usado — como no original, `player.gd:30`) |
  | `fire_cooldown` | `Timer` | `FireCooldown` |
  | `sound_effect_jump` / `_land` / `_shoot` | `AudioStreamPlayer` | `SoundEffects/Jump`, `SoundEffects/Land`, `SoundEffects/Shoot` |

  `initial_position: Vector3` é capturada na primeira linha de `ready`
  (`self.base().get_transform().origin`) — equivalente ao `@onready var initial_position =
  transform.origin` (`player.gd:24`).
- **Racional**: `OnReady` exige `Base` + classe `Node` (`obj/on_ready.rs:63-64`) e é
  inicializado antes de `ready()` na ordem de declaração. `OnReady<Gd<PlayerInputSynchronizer>>`
  funciona porque a classe já é Rust (Marco A) — é o acesso tipado exigido.
- **Caminhos compostos**: o original chega a `ShootFrom` e aos sons por dois passos
  (`player_model.get_node(^"Robot_Skeleton/...")`, `sound_effects.get_node(^"Jump")`). Um
  `#[init(node)]` não pode partir de outro `OnReady`, então usa-se o caminho completo a partir do
  Player — mesma resolução de node, mesmo resultado. O intermediário `sound_effects: Node`
  (`player.gd:33`) não é mantido porque só existia para encadear o `get_node`.
- **Campo não usado sem warning**: `crosshair` não é lido em lugar nenhum, mas o derive
  `GodotClass` referencia o campo, e o build confirmou 0 warnings. Mantido por fidelidade.
- **`ShootParticle`/`MuzzleFlash`**: buscados dentro de `shoot()` por
  `self.base().get_node_as::<CpuParticles3D>("PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle")`
  (e `.../MuzzleFlash`), como o `$...` do original (`player.gd:193,196`). Grafia gdext:
  `CpuParticles3D` (`cpu_particles_3d.rs`).

## D6 — Acesso tipado e disciplina de `bind()`/`bind_mut()`

- **Decisão**: leituras por guard temporário — `self.player_input.bind().motion`,
  `self.player_input.bind().aiming`, `self.player_input.bind().get_camera_rotation_basis()`;
  escrita `self.player_input.bind_mut().jumping = false`. Em `add_camera_shake_trauma`:
  ```rust
  let camera = self.player_input.bind().camera_camera.clone().unwrap(); // guard solto aqui
  camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);
  ```
- **Racional**: `Gd<T>::bind()`/`bind_mut()` (`obj/gd.rs`) emprestam o objeto **outro**
  (`PlayerInputSynchronizer`, `CameraNoiseShake`) — não o `Player`, que já está em `&mut self`.
  Objetos distintos não conflitam. Mesmo assim, os guards são sempre temporários (uma expressão)
  para nunca coexistirem com um `bind_mut()` do mesmo objeto. `camera_camera` é
  `Option<Gd<Camera3D>>` (Marco A, D3); `.clone()` do `Gd` é barato (ref-count) e `cast::<CameraNoiseShake>()`
  (`obj/gd.rs:550`) é válido porque `player.tscn:627` já tem `type="CameraNoiseShake"` — o
  `cast` faz panic se o tipo não bater, equivalente ao erro de tipo que o GDScript daria.
- **Confirmado**: compila sem warnings; padrão idêntico ao usado em `blast.rs`/`part_disappear.rs`.
- **Alternativa descartada**: guardar `Gd<CameraNoiseShake>` num campo em `ready` — mudaria o
  momento da resolução (o original resolve a cada chamada).

## D7 — RPCs e re-entrância

- **Decisão**: `#[rpc(authority, call_local, unreliable)]` em `jump`, `land`, `shoot`, `hit`,
  `add_camera_shake_trauma(amount: f64)` (Player) e `explode` (Bullet). Disparo:
  `self.base_mut().rpc("land", &[])` (`node.rs:1480`, retorna `Error`, ignorado como no original).
  `shoot()` chama `self.add_camera_shake_trauma(0.35)` **diretamente** (chamada Rust, sem RPC —
  como `player.gd:201`); `hit()` idem com `0.75`.
- **Racional**: `@rpc("call_local")` = modo `authority` + `unreliable` (defaults), confirmado em
  Marco A (research 001 D5). `call_local` faz o Godot invocar o próprio `#[func]` no mesmo
  objeto **enquanto** `apply_input(&mut self)` está ativo; isso é seguro porque a chamada passa
  por `self.base_mut()`, cujo guard permite re-entrância no objeto (`obj/base.rs:163`,
  `storage/instance_storage.rs:73`) — exatamente o caminho já validado no jogo em Marco A
  (`player_input.rs:217`, RPC `jump` disparado de dentro de `process`).
- **Bala → jogador**: `collider.rpc("hit", &[])` é chamado de `Bullet::physics_process` sobre o
  `Gd<Node3D>` do jogador, que não está emprestado nesse momento — sem re-entrância.

## D8 — `AnimationTree`: propriedades dinâmicas e root motion

- **Decisão**: `self.animation_tree.set("parameters/state/transition_request", &"strafe".to_variant())`
  (`Object::set(&mut self, property: impl AsArg<StringName>, value: &Variant)`, `object.rs:175`);
  `set("parameters/aim/add_amount", &aim.to_variant())` com `aim: f64`;
  `set("parameters/aim/add_amount", &0.to_variant())` (inteiro `0`, como `player.gd:79`);
  `set("parameters/strafe/blend_position", &Vector2::new(self.motion.x, -self.motion.y).to_variant())`;
  `set("parameters/walk/blend_position", &Vector2::new(self.motion.length(), 0.0).to_variant())`.
  Root motion tipado: `get_root_motion_rotation() -> Quaternion` (`animation_mixer.rs:287`),
  `get_root_motion_position() -> Vector3` (`:277`).
- **Racional**: `animation_tree["parameters/..."] = x` é `Object.set` — API base (FR-025 permite).
  O `0` inteiro é preservado porque o Godot converte `int → float` ao gravar no parâmetro; usar
  `0.0` daria o mesmo resultado, mas o literal do original é `0`.

## D9 — Matemática de builtins

| GDScript | gdext 0.5.5 | Fonte |
|---|---|---|
| `Transform3D(quat, pos)` | `Transform3D::new(Basis::from_quaternion(q), pos)` | `transform3d.rs:90`; `basis.rs:128` |
| `orientation *= root_motion` | `self.orientation = self.orientation * self.root_motion` | `impl Mul for Transform3D`, `transform3d.rs:291` |
| `orientation.orthonormalized()` | `self.orientation.orthonormalized()` | `transform3d.rs:177` |
| `basis.get_rotation_quaternion()` | `basis.get_quaternion()` | `basis.rs:191` |
| `q_from.slerp(q_to, w)` | `q_from.slerp(q_to, w as f32)` (`weight: real`) | `quaternion.rs:203` |
| `Basis(q)` | `Basis::from_quaternion(q)` | `basis.rs:128` |
| `Basis.looking_at(target)` | `Basis::looking_at(target)` (defaults `up = UP`, `use_model_front = false`) | gerada, `builtin_classes/basis.rs:231` |
| `basis.x` / `basis.z` | `basis.col_a()` / `basis.col_c()` | `basis.rs:471,503` — o gdext guarda `rows`; as colunas são os eixos |
| `orientation.basis = ...` / `.origin = ...` | campos públicos `basis`/`origin` | `transform3d.rs:58,61` |
| `motion.lerp(to, w)` / `.length()` / `.normalized()` | idem (`weight: real`) | `vector_macros.rs:776,453,802` |
| `player_model.global_transform.basis = b` | `self.player_model.set_global_basis(b)` | `node_3d.rs:410` |
| `transform.origin` (leitura) | `self.base().get_transform().origin` | `node_3d.rs:211` |
| `transform.origin = initial_position` | `let mut t = get_transform(); t.origin = ...; set_transform(t)` (preserva a base) | `node_3d.rs:202` |

- **Atenção `col_c`**: o GDScript `transform.basis.z` é a **coluna** z (terceiro eixo). Em gdext
  `Basis { rows: [Vector3; 3] }` — `rows[2]` seria a linha, errado; o acessor correto é
  `col_c()`. Idem `basis.x` → `col_a()`.

## D10 — `CharacterBody3D` / física

- `is_on_floor() -> bool` (`character_body_3d.rs:473`), `get_velocity()/set_velocity(Vector3)`
  (`:211,:202`), `set_up_direction(Vector3::UP)` (`:427`), `move_and_slide() -> bool` (`:183`,
  retorno ignorado como no original), `get_gravity() -> Vector3` (`physics_body_3d.rs:67`).
- `velocity.y = JUMP_SPEED` → `let mut v = get_velocity(); v.y = JUMP_SPEED; set_velocity(v)`.
  A sequência final do original (`velocity.x/z = h; velocity += gravity*delta; set_velocity(velocity)`)
  vira uma única leitura/modificação/escrita — mesmo valor final.
- Tipos: constantes `f32` (`MOTION_INTERPOLATE_SPEED`, `ROTATION_INTERPOLATE_SPEED` = 10.0;
  `MIN_AIRBORNE_TIME` = 0.1; `JUMP_SPEED` = 5.0); `airborne_time: f32` com `#[init(val = 100.0)]`;
  `delta: f64` nos virtuais, `delta as f32` no ponto de uso (regra do Marco A).
- Bala: `move_and_collide(motion: Vector3) -> Option<Gd<KinematicCollision3D>>`
  (`physics_body_3d.rs:37`); `KinematicCollision3D::get_collider() -> Option<Gd<Object>>`
  (`kinematic_collision_3d.rs:259`); deslocamento
  `-(delta as f32) * BULLET_VELOCITY * self.base().get_transform().basis.col_c()`.

## D11 — Instanciar a bala

- **Decisão**:
  ```rust
  let mut bullet: Gd<CharacterBody3D> = load::<PackedScene>("res://player/bullet/bullet.tscn")
      .instantiate_as::<CharacterBody3D>();
  self.base().get_parent().unwrap().add_child_ex(&bullet).force_readable_name(true).done();
  bullet.set_global_position(shoot_origin);
  bullet.look_at(shoot_origin + shoot_dir);
  bullet.add_collision_exception_with(&self.to_gd());
  self.base_mut().rpc("shoot", &[]);
  ```
- **Racional**: `load::<T>` (`tools/save_load.rs:30`) no ponto de uso substitui `preload`; após o
  primeiro carregamento o `ResourceLoader` serve do cache, e `bullet.tscn` já está carregada
  antes do primeiro tiro porque é `ext_resource` de `player.tscn` (`BulletCache`, l.679) — sem
  diferença observável. `instantiate_as::<CharacterBody3D>` (`manual_extensions.rs:65`) tipa
  como **API base**, não como `Bullet`: no commit do port 1 a bala ainda é GDScript e, depois do
  port 2, só API base continua sendo usada (`set_global_position`, `look_at`,
  `add_collision_exception_with`) — o Player nunca depende da classe `Bullet` (Princípio II,
  "API base não é dependência"). `add_child_ex(..).force_readable_name(true)` = `add_child(bullet, true)`
  (`node.rs:279`). `set_global_position` (`node_3d.rs:392`) = `global_transform.origin = ...`.
  `look_at(target)` (`node_3d.rs:851`, `up` default). `add_collision_exception_with(body: impl AsArg<Gd<Node>>)`
  (`physics_body_3d.rs:107`) aceita `&self.to_gd()`.
- **Alternativas descartadas**: campo `#[init(val = load(..))]` (carregaria na construção do
  Player, antes do `ready` — funciona, mas desloca o momento do load sem ganho);
  `instantiate_as::<Bullet>` (criaria dependência Rust→Rust desnecessária e quebraria o commit 1).

## D12 — `Timer`, `AudioStreamPlayer`, `CpuParticles3D`, `CollisionShape3D`, `Light3D`

| GDScript | gdext 0.5.5 | Fonte |
|---|---|---|
| `fire_cooldown.time_left == 0` | `self.fire_cooldown.get_time_left() == 0.0` (`f64`) | `timer.rs:301` |
| `fire_cooldown.start()` | `self.fire_cooldown.start()` (sem args = `time_sec` default −1) | `timer.rs:237` |
| `sound.play()` | `.play()` | `audio_stream_player.rs:255` |
| `particle.restart()` / `.emitting = true` | `.restart()` / `.set_emitting(true)` | `cpu_particles_3d.rs:492,173` |
| `collision_shape.disabled = true` | `.set_disabled(true)` | `collision_shape_3d.rs:198` |
| `omni_light.shadow_enabled = true` | `.set_shadow(true)` (setter da propriedade `shadow_enabled`) | `light_3d.rs:62` |
| `animation_player.play(&"explode")` | `.play_ex().name("explode").done()` | `animation_player.rs:322` (padrão Marco A) |
| `queue_free()` / `set_physics_process(false)` / `set_process(false)` | idem em `base_mut()` | `node.rs:1299,739,779` |
| `multiplayer.is_server()` | `self.base().get_multiplayer().unwrap().is_server()` | `node.rs:1369`; `multiplayer_api.rs:63` |

## D13 — Duck typing preservado na bala (`has_method("hit")` + `rpc`)

- **Decisão**:
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
- **Racional**: `col.get_collider() as Node3D` → `null` se não for `Node3D`, e o `if collider and
  ...` do original cobre isso — `and_then(try_cast(..).ok())` reproduz os dois casos.
  `has_method` (`object.rs:474`) e `rpc` (`node.rs:1480`) por nome são exatamente o que o
  original já faz dinamicamente — permitido por FR-025 e pelo Princípio II ("duck typing que já
  existe no original"). Backlog v2 item 2 (`Hittable`) já cobre a melhoria — não duplicar.

## D14 — Exceção `Settings` (primeiro uso)

- **Decisão**:
  ```rust
  let config_file = self.base().get_node_as::<Node>("/root/Settings")
      .get("config_file").to::<Gd<ConfigFile>>();
  if config_file.get_value("rendering", "shadow_mapping").to::<bool>() {
      self.omni_light.set_shadow(true);
  }
  ```
- **Racional**: é a exceção única do Princípio II — o autoload é obtido dinamicamente por
  caminho absoluto (válido porque `explode` roda com a bala na árvore), `config_file` lido por
  `Object::get` (`object.rs:185`) e, daí em diante, API tipada de `ConfigFile`
  (`get_value(section, key) -> Variant`, `config_file.rs:145`). `settings.gd` grava `bool` nessa
  chave, então `.to::<bool>()` é a conversão exata. Backlog v2 item 1 já registra o acesso
  tipado — não duplicar.
- **Headless**: o autoload **é** carregado em `--path . cena.tscn` (verificado na spec, Edge
  Cases); só falta em harness `-s`. A validação do port 2 roda `bullet.tscn` por 20 s: a bala
  expira aos 5 s → `explode` → `Settings` lido → `destroy` aos 1,5 s da animação — tudo dentro
  do `timeout 20`.
- **Alternativa descartada**: `try_get_node_as` com fallback silencioso — esconderia a ausência
  do autoload; o original também erra nesse caso.

## D15 — Porta: `try_cast` e a correção do bug

- **Decisão**:
  ```rust
  #[derive(GodotClass)]
  #[class(init, base=Area3D)]
  pub struct Door {
      base: Base<Area3D>,
      open: bool,
      // upstream bug fix: door.gd referenciava "DoorModel/AnimationPlayer" (node inexistente);
      // o node da cena é "DoorModel2" — a porta nunca abria e o Godot imprimia "Node not found".
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
- **Racional**: `body is Player` ⇔ `Gd::try_cast::<Player>()` (`obj/gd.rs:538`) bem-sucedido;
  `try_cast` consome `body`, e como o parâmetro é próprio (`Gd<Node3D>` por valor) e não é usado
  depois, não precisa de `clone()`. `impl IArea3D for Door {}` vazio: não há virtuais, mas o
  bloco é mantido por uniformidade com os demais ports (o `OnReady` é inicializado pelo hook
  gerado pelo `GodotClass`, independentemente do bloco). O comentário `// upstream bug fix: ...`
  fica **na linha imediatamente acima** do `#[init(node = ...)]` — o ponto exato da referência
  (requisito (b), FR-032).
- **Confirmado empiricamente (§E.1/E.2)**: `has_method("_on_door_body_entered")`; `p is ZzPlayer`
  verdadeiro para o Player e falso para a bala; baseline de `door.tscn` no original = exatamente
  **1** `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`.
- **Alternativas descartadas**: `body.is_class("Player")` (compara string, não tipo — e não é o
  que `is` faz com classes nativas); `body.clone().try_cast()` (clone desnecessário).
- **Catálogo do `CLAUDE.md`**: o `Node not found` da porta **nunca esteve** no catálogo
  (`CLAUDE.md:39-40` lista só `Cannon_Charge already exists`, `doorsimple_d.png`,
  `surfaces.is_empty()`, todos de import). A regra "remover do catálogo no mesmo commit" fica
  satisfeita sem alteração; a constatação vai na mensagem do commit do port 3.

## D16 — O que NÃO muda (quirks preservados, FR-035)

`airborne_time` inicia em 100 (primeiro pouso dispara `land`); `player_input.jumping = false` a
cada frame de física; possível `explode` duplo quando a bala expira e colide no mesmo frame;
`velocity` não zerada no respawn; `crosshair` referenciado e nunca usado; `FireCooldown`
`autostart` (0,4 s sem tiro após spawn). Todos ficam como estão. Candidatos ao backlog v2 estão
em §"Backlog v2 candidato".

## §E — Verificação empírica (2026-09-15, headless, sem editor aberto)

### E.1 Probe `-s` com as três classes de rascunho (`ZzPlayer`/`ZzBullet`/`ZzDoor`)

Script `SceneTree` construindo `ZzPlayer` + filho `MultiplayerSynchronizer` "InputSynchronizer"
**fora da árvore**:

```
in_tree=false
player_id=7 authority(InputSynchronizer)=7           ← setter por atribuição, fora da árvore
via set(): player_id=9 authority=9                   ← setter por Object.set (forma do level.gd)
motion default=(0.0, 0.0) type=5                     ← #[var] visível por nome (type 5 = VECTOR2)
motion after set=(1.0, 2.0) / after set_indexed=(3.0, 4.0)
current_animation default=3 (WALK=3 esperado) / after set=2
prop motion            type=5 hint=0 hint_string='' usage=0
prop player_id         type=2 hint=0 usage=6         ← DEFAULT (editor + storage)
prop current_animation type=2 hint=2 hint_string='JumpUp:0,JumpDown:1,Strafe:2,Walk:3' usage=6
has_method hit=true add_camera_shake_trauma=true jump=true animate=false apply_input=false set_player_id=true
bullet has destroy=true explode=true
door has _on_door_body_entered=true
is Player: true (Player) false (Bullet)
```

`cargo build` do rascunho: 0 erros, **0 warnings**. Rascunho e alterações de visibilidade
revertidos em seguida (`git status` limpo).

### E.2 Baseline da porta (commit `108584e`, `door.tscn` original)

```
timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn   → exit 124
ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door").   (linha 6 do log)
grep -c 'Node not found' → 1        WARNING: 1 (HDR, benigno)
```

Este é o erro que **DEVE desaparecer** após o port 3 (FR-027, SC-003).

## Mapa por script (resumo para as tasks)

### 1. `player.gd` → `src/player.rs` — `Player: CharacterBody3D`

- Consts (`f32`): `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`,
  `MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`.
- `enum Animations` (D3). Campos: `airborne_time: f32` (init 100.0), `orientation: Transform3D`,
  `root_motion: Transform3D`, `#[var] motion: Vector2`, `initial_position: Vector3`, os 10
  `OnReady` de D5, `#[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32`,
  `#[export] #[init(val = Animations::Walk)] current_animation: Animations`.
- `ICharacterBody3D`: `ready` (initial_position; orientation = global do modelo com origem zero;
  `set_process(false)` se não servidor), `physics_process(delta)` (servidor → `apply_input`;
  senão `animate(current_animation)`).
- `#[godot_api] impl Player` (bloco único): `#[func] set_player_id`; `#[rpc(authority, call_local, unreliable)]`
  `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount: f64)`.
- `impl Player` privado: `animate(anim, _delta)`, `apply_input(delta)` — linha a linha de
  `player.gd:61-176` com D6–D11.
- Edição em `player_input.rs`/`camera_noise_shake.rs`: só `pub(crate)` (D1).

### 2. `bullet.gd` → `src/bullet.rs` — `Bullet: CharacterBody3D`

- `const BULLET_VELOCITY: f32 = 20.0`; `time_alive: f32` (init 5.0); `hit: bool`; `OnReady`
  `animation_player` ("AnimationPlayer"), `collision_shape` ("CollisionShape3D"), `omni_light`
  ("OmniLight3D").
- `ready`: não servidor → `set_physics_process(false)` + `collision_shape.set_disabled(true)`.
- `physics_process`: D10/D13. `#[rpc(authority, call_local, unreliable)] explode` (D12/D14);
  `#[func] destroy` (não servidor → return; `queue_free`).

### 3. `door.gd` → `src/door.rs` — `Door: Area3D` (D15)

## Backlog v2 candidato (registrar em `docs/v2-backlog.md` no commit do script correspondente)

Itens 1 (Settings tipado) e 2 (`Hittable`) **já existem** — não duplicar. Novos candidatos:

| # | Origem | Melhoria | Motivação |
|---|---|---|---|
| 10 | `player/player.gd` (port 1) | Iniciar `airborne_time` em 0 (ou ignorar o primeiro pouso) | Com 100 inicial, o primeiro contato com o chão dispara `land` e o som de pouso ao spawnar |
| 11 | `player/player.gd` (port 1) | Zerar `velocity` no respawn abaixo de −40 | O teleporte preserva a velocidade de queda acumulada |
| 12 | `player/player.gd` (port 1) | Remover a referência `crosshair` nunca usada (ou usá-la) | Declarada em `player.gd:30` e não lida; na v1 é mantida por fidelidade |
| 13 | `player/bullet/bullet.gd` (port 2) | Evitar o `explode` duplo quando `time_alive` expira e há colisão no mesmo frame | Dois RPCs `explode` no mesmo frame reiniciam a animação |
| 14 | `door/door.gd` (port 3) | Tipar `_on_door_body_entered` com `Gd<Player>` via `try_cast` já no sinal / considerar fechar a porta | Só o primeiro `Player` abre e nunca fecha; a v1 só corrige a referência do node |

O item 9 existente (`jumping` replicado/`@export`) já cobre o quirk "jogador zera `jumping`";
não duplicar.
