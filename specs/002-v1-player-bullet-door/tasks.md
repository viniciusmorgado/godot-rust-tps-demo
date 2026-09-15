# Tasks: Marco B — jogador, bala e porta (v1 raw port)

**Input**: Design documents from `/specs/002-v1-player-bullet-door/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D16, §E), data-model.md, contracts/, quickstart.md (todos aprovados)

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.0). Nenhuma task pode introduzir
abstração, refatoração, otimização, teste unitário ou infraestrutura. Se algo assim parecer
necessário, vira entrada em `docs/v2-backlog.md`, não task. Este marco usa **uma única vez** a
cláusula de correção conservadora de bug do upstream (US3, porta); nenhuma outra correção.

**Tests**: não há testes automatizados nesta fase (plan.md "Testing"). A validação de cada story
é o ciclo do quickstart (build → import headless → cena headless → verificações mecânicas →
contrato → validação visual do usuário).

**Organization**: uma phase por user story, na ordem obrigatória US1 → US2 → US3 (spec FR-028;
`docs/port-order.md` itens 6 → 7 → 8). As stories **não** são paralelizáveis entre si: cada uma
termina com um commit próprio na `main` e a seguinte começa da árvore limpa; a US3 depende de
`Player` em Rust (`is Player`) e a US2 é instanciada pelo Player (só por API base, mas o jogo tem
de estar jogável após cada commit).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência de task incompleta) — raro
  neste marco, porque build depende do módulo, cena depende do build, validação depende da cena.
- **[Story]**: US1..US3 (spec.md)
- Caminhos relativos à raiz do repositório (`oxide-godot/` = projeto Godot;
  `oxide_godot_core/oxide_godot_lib/src/` = crate Rust).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` de cada módulo (só isso)
oxide_godot_core/oxide_godot_lib/src/<módulo>.rs   uma classe por script (research.md §"Mapa por script")
oxide_godot_core/oxide_godot_lib/src/player_input.rs, camera_noise_shake.rs   Marco A — só visibilidade muda (US1)
oxide-godot/<cena>.tscn                            troca de `type` (plan.md "Edição das cenas")
oxide-godot/<script>.gd + .gd.uid                  apagados no commit do port
docs/v2-backlog.md                                 uma linha por melhoria percebida (itens 10–14)
docs/upstream-bugs.md                              NOVO na US3: registro da correção da porta
../oxide_godot_origins/                            referência intocada do GDScript original
```

Decisões fechadas (valem para todas as stories — instrução, não opção; detalhes em research.md):

- Acesso do `Player` a `PlayerInputSynchronizer`/`CameraNoiseShake` é **tipado** (`bind()`/
  `bind_mut()`/`cast::<CameraNoiseShake>()`); para isso, no **mesmo commit** do Player, só a
  visibilidade dos itens listados em T006 muda para `pub(crate)` — nenhuma outra linha desses
  arquivos (D1).
- `player_id`: `#[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32`; setter
  `#[func]` no bloco `#[godot_api] impl Player` **principal**, usando
  `self.base().get_node_as::<MultiplayerSynchronizer>("InputSynchronizer")` — **não** o `OnReady`
  (roda antes de `ready`) (D2).
- `current_animation`: `enum Animations { JumpUp, JumpDown, Strafe, Walk }` com
  `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)]`;
  campo `#[export] #[init(val = Animations::Walk)]` (D3). `motion`: `#[var] motion: Vector2` (D4).
- `animate` e `apply_input` **privados, sem `#[func]`**; `ShootParticle`/`MuzzleFlash` por
  `get_node_as::<CpuParticles3D>` dentro de `shoot`; sons por `#[init(node = "SoundEffects/Jump")]`
  etc.; `crosshair` declarado e não usado (D5).
- Bala instanciada como `Gd<CharacterBody3D>` (`load::<PackedScene>(..).instantiate_as::<CharacterBody3D>()`
  no ponto de uso; `add_child_ex(&bullet).force_readable_name(true).done()`;
  `add_collision_exception_with(&self.to_gd())`). **Nunca** `instantiate_as::<Bullet>` (D11).
- Eixos: `basis.col_c()` = `basis.z`, `basis.col_a()` = `basis.x`; `Basis::looking_at(target)`;
  `Basis::from_quaternion(q)`; `q.slerp(q_to, w as f32)`; `Transform3D::new(basis, origin)`;
  `orientation * root_motion`; `player_model.set_global_basis(..)` (D9).
- RPCs `#[rpc(authority, call_local, unreliable)]`: `jump`, `land`, `shoot`, `hit`,
  `add_camera_shake_trauma` (Player); `explode` (Bullet). `shoot`/`hit` chamam
  `self.add_camera_shake_trauma(x)` **diretamente**; disparo por `self.base_mut().rpc("nome", &[])` (D7).
- Bullet: `has_method("hit")` + `rpc("hit", &[])` no collider (D13); `Settings` por
  `get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>()` (D14);
  `#[func] destroy` (D12).
- Door: `#[init(node = "DoorModel2/AnimationPlayer")]` com o comentário `// upstream bug fix: ...`
  na linha **imediatamente acima**; `body.try_cast::<Player>().is_ok()` sem `clone()`;
  `#[godot_api] impl IArea3D for Door {}` vazio (D15).
- `docs/upstream-bugs.md`: criado na US3; coluna "commit" = **assunto** do commit; **sem**
  `git commit --amend`.
- Tipos: `delta: f64` nos virtuais; constantes e `airborne_time`/`time_alive` em `f32`; `as f32`
  no ponto de uso; `amount: f64` nos RPCs (como `add_trauma`).
- Um commit por script na `main`, mensagem no formato do quickstart passo 8; fix após checkpoint
  = commit `Fix port …`.

---

## Phase 1: Setup

**Purpose**: registrar a baseline contra a qual cada port é comparado — incluindo a **medição do
erro da porta**, que deve desaparecer na US3. Nada de infraestrutura (Princípio I).

- [ ] T001 Confirmar pré-condições: nenhum editor Godot aberto (`pgrep -a godot` vazio — se houver, avisar o usuário e aguardar, nunca matar); `git status --short` vazio na `main`; anotar `git rev-parse --short HEAD`. O commit-base `108584e` usado por `quickstart.md`/T040 para `git diff -- '*.gd'` e `git log 108584e..HEAD` continua válido: commits posteriores (plan, tasks) não tocam `.gd` nem são commits "Port …"; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 10
- [ ] T002 Registrar baseline do build: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → esperado `0`; anotar em `specs/002-v1-player-bullet-door/quickstart.md` §"Baseline" se diferir
- [ ] T003 Registrar baseline do import: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirmar `grep -n 'Initialize godot-rust' /tmp/import.log` (linha 1) e `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` vazio (ou só os 3 erros do upstream do `CLAUDE.md`)
- [ ] T004 [P] Confirmar o diretório de bindings geradas usado pelo research.md: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` → esperado `.../godot-core-aea5c50e7fda9d57/out`; se o hash diferir, atualizar a linha 6 de `specs/002-v1-player-bullet-door/research.md` (as linhas citadas continuam válidas — mesma versão 0.5.5)
- [ ] T005 **Medir a baseline da porta** (quickstart §"Baseline"): `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn 2>&1 | tee /tmp/door_baseline.log`; esperado exit 124, `grep -c 'Node not found' /tmp/door_baseline.log` = **1** (`ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`), e nenhuma outra linha `ERROR`. Confirmar também `grep -n 'Node not found' CLAUDE.md` vazio (o erro nunca esteve no catálogo). Se a contagem não for 1, parar e reportar — a US3 depende desta medição

**Checkpoint**: baseline conhecida — 0 warnings, extensão carrega, 0 `ERROR` no import, porta com exatamente 1 `Node not found`.

---

## Phase 2: Foundational

**Não existe neste marco.** Os três ports são estritamente sequenciais (player → bullet → door) e
a única edição compartilhada é a linha `mod <módulo>;` em `lib.rs`, feita dentro de cada story.
A mudança de visibilidade `pub(crate)` em `player_input.rs`/`camera_noise_shake.rs` **pertence à
US1** (é o Player quem a exige e ela entra no commit do Player), não a uma fase comum. Nenhum
módulo comum, helper, trait ou constante compartilhada pode ser criado (Princípio I).

---

## Phase 3: User Story 1 — Jogador portado (Priority: P1) 🎯 MVP

**Goal**: `player/player.gd` (211 l., `class_name Player`) → `Player: CharacterBody3D`; nome
**obrigatório** (`is Player` em `red_robot.gd:131,275,281` e `door.gd:10`); `player_id` com setter
que roda fora da árvore; `current_animation` enum e `motion` replicáveis por nome; 5 RPCs;
acesso tipado a `PlayerInputSynchronizer` e `CameraNoiseShake`; bala instanciada por API base.

**Independent Test**: `player/player.tscn` e `level/level.tscn` headless sem `ERROR`; contrato
`contracts/player.md` conferido; no jogo, mover/pular/pousar/mirar/atirar/tremor/respawn
indistinguíveis do original; `red_robot.gd` e `level.gd` seguem encontrando `Player`,
`player_id`, `add_camera_shake_trauma` sem edição.

- [ ] T006 [US1] Alterar **só a visibilidade** em `oxide_godot_core/oxide_godot_lib/src/player_input.rs`: campos `aiming`, `shoot_target`, `motion`, `shooting`, `jumping`, `camera_camera` (l.33–54) e métodos `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis` (l.184, 203, 213) recebem `pub(crate)` (`pub(crate) aiming: bool`, `pub(crate) fn get_aim_rotation(&self) -> f64`, …); em `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs`, `fn add_trauma` (l.52) → `pub(crate) fn add_trauma`. Atributos `#[export]`/`#[func]` ficam onde estão. Conferir: `git diff --stat` = `player_input.rs | 18 +++++++++---------`, `camera_noise_shake.rs | 2 +-` (10 pares −/+, nada mais) (research D1)
- [ ] T007 [US1] Criar `oxide_godot_core/oxide_godot_lib/src/player.rs` — parte 1 (declarações), traduzindo `oxide-godot/player/player.gd:1-43`: `use godot::classes::{AnimationTree, AudioStreamPlayer, CharacterBody3D, CpuParticles3D, ICharacterBody3D, Marker3D, MultiplayerSynchronizer, Node3D, PackedScene, TextureRect, Timer}; use godot::prelude::*; use crate::camera_noise_shake::CameraNoiseShake; use crate::player_input::PlayerInputSynchronizer;`; `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum Animations { JumpUp, JumpDown, Strafe, Walk }`; consts `f32`: `MOTION_INTERPOLATE_SPEED = 10.0`, `ROTATION_INTERPOLATE_SPEED = 10.0`, `MIN_AIRBORNE_TIME = 0.1`, `JUMP_SPEED = 5.0`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct Player { base: Base<CharacterBody3D>, #[init(val = 100.0)] airborne_time: f32, orientation: Transform3D, root_motion: Transform3D, #[var] motion: Vector2, initial_position: Vector3, #[init(node = "InputSynchronizer")] player_input: OnReady<Gd<PlayerInputSynchronizer>>, #[init(node = "AnimationTree")] animation_tree: OnReady<Gd<AnimationTree>>, #[init(node = "PlayerModel")] player_model: OnReady<Gd<Node3D>>, #[init(node = "PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom")] shoot_from: OnReady<Gd<Marker3D>>, #[init(node = "Crosshair")] crosshair: OnReady<Gd<TextureRect>>, #[init(node = "FireCooldown")] fire_cooldown: OnReady<Gd<Timer>>, #[init(node = "SoundEffects/Jump")] sound_effect_jump: OnReady<Gd<AudioStreamPlayer>>, #[init(node = "SoundEffects/Land")] sound_effect_land: OnReady<Gd<AudioStreamPlayer>>, #[init(node = "SoundEffects/Shoot")] sound_effect_shoot: OnReady<Gd<AudioStreamPlayer>>, #[export] #[var(set = set_player_id)] #[init(val = 1)] player_id: i32, #[export] #[init(val = Animations::Walk)] current_animation: Animations }`. Manter os comentários do original traduzidos onde existem (research D2–D5)
- [ ] T008 [US1] `player.rs` — parte 2 (virtuais e privados), traduzindo `player.gd:46-176` linha a linha: `#[godot_api] impl ICharacterBody3D for Player` com `fn ready(&mut self)` (`self.initial_position = self.base().get_transform().origin; self.orientation = self.player_model.get_global_transform(); self.orientation.origin = Vector3::ZERO; if !self.base().get_multiplayer().unwrap().is_server() { self.base_mut().set_process(false); }`) e `fn physics_process(&mut self, delta: f64)` (`if is_server { self.apply_input(delta) } else { let anim = self.current_animation; self.animate(anim, delta) }`); bloco `impl Player` **sem** `#[godot_api]` com `fn animate(&mut self, anim: Animations, _delta: f64)` (`self.current_animation = anim;` + os 4 ramos com `self.animation_tree.set("parameters/state/transition_request", &"jump_up".to_variant())` etc.; STRAFE: `let aim = self.player_input.bind().get_aim_rotation(); set("parameters/aim/add_amount", &aim.to_variant()); set("parameters/strafe/blend_position", &Vector2::new(self.motion.x, -self.motion.y).to_variant())`; WALK: `set("parameters/aim/add_amount", &0.to_variant()); set(".../transition_request", &"walk".to_variant()); set("parameters/walk/blend_position", &Vector2::new(self.motion.length(), 0.0).to_variant())`) e `fn apply_input(&mut self, delta: f64)`: lerp de `motion` (`self.motion.lerp(self.player_input.bind().motion, MOTION_INTERPOLATE_SPEED * delta as f32)`); `camera_basis = self.player_input.bind().get_camera_rotation_basis()`, `camera_z = camera_basis.col_c()`, `camera_x = camera_basis.col_a()`, zerar `.y` e `normalized()`; `airborne_time += delta as f32`; `is_on_floor()` → `if airborne_time > 0.5 { self.base_mut().rpc("land", &[]) }; airborne_time = 0.0`; `on_air = airborne_time > MIN_AIRBORNE_TIME`; pulo (`if !on_air && self.player_input.bind().jumping { velocity.y = JUMP_SPEED via get/set_velocity; on_air = true; airborne_time = MIN_AIRBORNE_TIME; rpc("jump") }`); `self.player_input.bind_mut().jumping = false;`; ramo ar (`velocity.y > 0.0` → `animate(JumpUp)` senão `animate(JumpDown)`); ramo mira (`q_from = self.orientation.basis.get_quaternion(); q_to = self.player_input.bind().get_camera_base_quaternion(); self.orientation.basis = Basis::from_quaternion(q_from.slerp(q_to, delta as f32 * ROTATION_INTERPOLATE_SPEED)); animate(Strafe); root_motion = Transform3D::new(Basis::from_quaternion(animation_tree.get_root_motion_rotation()), animation_tree.get_root_motion_position());` tiro se `self.player_input.bind().shooting && self.fire_cooldown.get_time_left() == 0.0`: `shoot_origin = self.shoot_from.get_global_transform().origin; shoot_dir = (shoot_target - shoot_origin).normalized(); let mut bullet: Gd<CharacterBody3D> = load::<PackedScene>("res://player/bullet/bullet.tscn").instantiate_as::<CharacterBody3D>(); self.base().get_parent().unwrap().add_child_ex(&bullet).force_readable_name(true).done(); bullet.set_global_position(shoot_origin); bullet.look_at(shoot_origin + shoot_dir); bullet.add_collision_exception_with(&self.to_gd()); self.base_mut().rpc("shoot", &[]);`); ramo andar (`target = camera_x * motion.x + camera_z * motion.y; if target.length() > 0.001 { slerp para Basis::looking_at(target).get_quaternion() }; animate(Walk); root_motion = ...`); depois: `self.orientation = self.orientation * self.root_motion; h_velocity = self.orientation.origin / delta as f32; velocity.x/z = h_velocity.x/z; velocity += self.base().get_gravity() * delta as f32; set_velocity; set_up_direction(Vector3::UP); move_and_slide(); self.orientation.origin = Vector3::ZERO; self.orientation = self.orientation.orthonormalized(); let basis = self.orientation.basis; self.player_model.set_global_basis(basis); if self.base().get_transform().origin.y < -40.0 { let mut t = self.base().get_transform(); t.origin = self.initial_position; self.base_mut().set_transform(t); }`. Guards `bind()` sempre temporários (uma expressão) (research D6–D11)
- [ ] T009 [US1] `player.rs` — parte 3 (bloco Godot), traduzindo `player.gd:38-41,179-211`: `#[godot_api] impl Player` **único** com `#[func] fn set_player_id(&mut self, value: i32) { self.player_id = value; self.base().get_node_as::<MultiplayerSynchronizer>("InputSynchronizer").set_multiplayer_authority(value); }`; `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.animate(Animations::JumpUp, 0.0); self.sound_effect_jump.play(); }`; `land` idem com `JumpDown`/`sound_effect_land`; `shoot`: `let mut shoot_particle = self.base().get_node_as::<CpuParticles3D>("PlayerModel/Robot_Skeleton/Skeleton3D/GunBone/ShootFrom/ShootParticle"); shoot_particle.restart(); shoot_particle.set_emitting(true);` idem `muzzle_particle` em `.../ShootFrom/MuzzleFlash`; `self.fire_cooldown.start(); self.sound_effect_shoot.play(); self.add_camera_shake_trauma(0.35);`; `hit`: `self.add_camera_shake_trauma(0.75);`; `add_camera_shake_trauma(&mut self, amount: f64)`: `let camera = self.player_input.bind().camera_camera.clone().unwrap(); camera.cast::<CameraNoiseShake>().bind_mut().add_trauma(amount);`. Nenhum outro `#[func]` (research D2, D6, D7)
- [ ] T010 [US1] Adicionar `mod player;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod player_input;`; nada mais muda em lib.rs)
- [ ] T011 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. Em caso de erro de compilação, consultar research.md D1–D11 e as bindings em `oxide_godot_core/target/debug/build/godot-core-*/out/classes/` antes de improvisar; não alterar `Cargo.toml`
- [ ] T012 [US1] Editar `oxide-godot/player/player.tscn` como texto (reconferir com `grep -n 'name="Player" type=\|^script = ExtResource("1")\|player.gd' oxide-godot/player/player.tscn`): na linha da raiz (≈333) trocar `type="CharacterBody3D"` por `type="Player"`; remover a linha `script = ExtResource("1")` (≈336); remover a linha `[ext_resource type="Script" uid="uid://ctlx3bqglonsx" path="res://player/player.gd" id="1"]` (l.3). **MANTER** `collision_layer = 6`/`collision_mask = 7` da raiz, `ServerSynchronizer` com `replication_config`, `InputSynchronizer` (já `PlayerInputSynchronizer`, `node_paths`, 6 `NodePath`), `BulletCache` e os `[editable path=...]`. Nada mais muda (plan.md "Edição das cenas", port 1)
- [ ] T013 [US1] Apagar `oxide-godot/player/player.gd` e `oxide-godot/player/player.gd.uid` (`git rm`); verificar `grep -rn 'uid://ctlx3bqglonsx' oxide-godot/` vazio e `grep -rn 'player.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T014 [US1] Validação headless (quickstart §2–3; sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` e depois `level/level.tscn` → exit 124 esperado; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log` vazio em ambas (só os 2 `WARNING` da baseline). O level spawna o Player com `name`/`player_id` fora da árvore e `red_robot.gd`/`level.gd` resolvem `Player`, `player_id`, `add_camera_shake_trauma` por nome — qualquer nome errado aparece aqui
- [ ] T015 [US1] Verificações mecânicas e contrato (quickstart §4–5; `contracts/player.md` §"Verificação antes do commit"): `grep -n 'type="Player"' oxide-godot/player/player.tscn` = 1 linha; `grep -n 'ExtResource("1")' oxide-godot/player/player.tscn` vazio; `grep -n 'properties/[0-9]/path' oxide-godot/player/player.tscn | head -5` lista `.:transform`, `.:player_id`, `PlayerModel:transform`, `.:motion`, `.:current_animation` — os três do script existem em `player.rs` com o mesmo nome; `grep -rn 'is Player\|player_id\|add_camera_shake_trauma\|\.hit\.rpc\|has_method(&"hit")' --include=*.gd oxide-godot` retorna só `red_robot.gd:131,133,275,281`, `level.gd:119`, `bullet.gd:31,32`, `door.gd:10`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `player/player.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player_input.rs oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs | grep '^[-+]' | grep -v '^[-+][-+]' | grep -v 'pub(crate)'` mostra apenas as 10 linhas `-` originais (nenhuma outra alteração); `ls oxide_godot_core/oxide_godot_lib/src/` = 7 arquivos (lib.rs + 6 módulos)
- [ ] T016 [US1] Registrar em `docs/v2-backlog.md` as linhas nº 10, 11 e 12 (research.md §"Backlog v2 candidato"): 10 — `player/player.gd` (port 1): iniciar `airborne_time` em 0 / ignorar o primeiro pouso (com 100 inicial o primeiro contato dispara `land` e o som ao spawnar); 11 — zerar `velocity` no respawn abaixo de −40 (o teleporte preserva a velocidade de queda); 12 — remover/usar a referência `crosshair` nunca lida (`player.gd:30`). Não duplicar os itens 1–9 existentes (o 9 já cobre `jumping`)
- [ ] T017 [US1] Commit único do port 1 na `main` (autor the repository author), incluindo `src/player.rs`, `src/lib.rs`, `src/player_input.rs`, `src/camera_noise_shake.rs`, `oxide-godot/player/player.tscn`, as deleções de `player.gd`/`player.gd.uid` e `docs/v2-backlog.md`. Mensagem: `Port player.gd → Player (CharacterBody3D); player.tscn: node Player type="CharacterBody3D"→"Player"` + corpo com: notas (setter `player_id` fora da árvore via `get_node_as`; `Animations` enum `via = i64`; `motion` `#[var]`; bala instanciada como `CharacterBody3D` por API base; quirks preservados: `airborne_time = 100`, `jumping` zerado por frame, `velocity` no respawn, `crosshair` não usado); `- player_input.rs / camera_noise_shake.rs: só visibilidade pub(crate) nos campos/métodos consumidos pelo Player (acesso tipado, FR-010/FR-011); nenhuma lógica movida`; `- backlog v2: itens 10, 11, 12`. Anotar o hash
- [ ] T018 [US1] **Checkpoint do usuário (validação visual, plan.md "Validação visual" port 1)** — feito pelo usuário no editor/jogo, comparando com `../oxide_godot_origins/`: entrar no level → **som de pouso imediato ao spawnar** (quirk); mover em todas as direções com o modelo virando para a câmera; pular (som Jump, animação subindo/descendo) e pousar após > 0,5 s no ar (som Land); mirar/strafe (mira acompanha o pitch); atirar (bala visível — ainda GDScript —, muzzle flash, cooldown 0,4 s, tremor leve); robô acerta → tremor forte (13,0); cair abaixo de −40 → respawn no ponto inicial; F3 e tudo do Marco A continuam. Ao abrir `player.tscn`: raiz tipo `Player`, sem script; `ServerSynchronizer`/`InputSynchronizer`/`BulletCache` intactos. Divergência → commit `Fix port player.gd …` na mesma story. Só seguir para a US2 com o OK explícito

**Checkpoint**: 9 `.gd` restantes; `player.tscn` com `Player`; jogo jogável.

---

## Phase 4: User Story 2 — Bala portada (Priority: P2)

**Goal**: `player/bullet/bullet.gd` (51 l.) → `Bullet: CharacterBody3D`; voa a 20 u/s, explode ao
colidir (chamando `hit` por duck typing) ou aos 5 s; RPC `explode` lê `Settings.config_file`
(primeiro uso da exceção); `destroy()` pelo method track; `BulletCache` em `player.tscn` passa a
instanciar a classe Rust.

**Independent Test**: `player/bullet/bullet.tscn` headless (expira aos 5 s → `explode` →
`Settings` → `destroy` dentro dos 20 s), `player/player.tscn` (`BulletCache`) e
`level/level.tscn` sem `ERROR`; contrato `contracts/bullet.md`; no jogo, bala visível explode em
paredes, no robô (que reage) e ao expirar.

- [ ] T019 [US2] Criar `oxide_godot_core/oxide_godot_lib/src/bullet.rs` traduzindo linha a linha `oxide-godot/player/bullet/bullet.gd`: `use godot::classes::{AnimationPlayer, CharacterBody3D, CollisionShape3D, ConfigFile, ICharacterBody3D, KinematicCollision3D, Node, Node3D, OmniLight3D}; use godot::prelude::*;`; `const BULLET_VELOCITY: f32 = 20.0;`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct Bullet { base: Base<CharacterBody3D>, #[init(val = 5.0)] time_alive: f32, hit: bool, #[init(node = "AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>, #[init(node = "CollisionShape3D")] collision_shape: OnReady<Gd<CollisionShape3D>>, #[init(node = "OmniLight3D")] omni_light: OnReady<Gd<OmniLight3D>> }`; `#[godot_api] impl ICharacterBody3D for Bullet`: `ready` (`if !is_server { self.base_mut().set_physics_process(false); self.collision_shape.set_disabled(true); }`), `physics_process(&mut self, delta: f64)` (`if self.hit { return; } self.time_alive -= delta as f32; if self.time_alive < 0.0 { self.hit = true; self.base_mut().rpc("explode", &[]); } let displacement = -(delta as f32) * BULLET_VELOCITY * self.base().get_transform().basis.col_c(); let col: Option<Gd<KinematicCollision3D>> = self.base_mut().move_and_collide(displacement); if let Some(col) = col { let collider: Option<Gd<Node3D>> = col.get_collider().and_then(|c| c.try_cast::<Node3D>().ok()); if let Some(mut collider) = collider { if collider.has_method("hit") { collider.rpc("hit", &[]); } } self.collision_shape.set_disabled(true); self.base_mut().rpc("explode", &[]); self.hit = true; }`); `#[godot_api] impl Bullet`: `#[rpc(authority, call_local, unreliable)] fn explode(&mut self)` (`self.animation_player.play_ex().name("explode").done();` + comentário do original + `let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>(); if config_file.get_value("rendering", "shadow_mapping").to::<bool>() { self.omni_light.set_shadow(true); }`) e `#[func] fn destroy(&mut self)` (`if !is_server { return; } self.base_mut().queue_free();`). Nenhum outro `#[func]` (research D10, D12–D14)
- [ ] T020 [US2] Adicionar `mod bullet;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod player;`)
- [ ] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [ ] T022 [US2] Editar `oxide-godot/player/bullet/bullet.tscn` como texto (reconferir com `grep -n 'name="Bullet" type=\|^script = ExtResource("1")\|bullet.gd' oxide-godot/player/bullet/bullet.tscn`): na linha da raiz (≈481) trocar `type="CharacterBody3D"` por `type="Bullet"`; remover `script = ExtResource("1")` (≈485); remover `[ext_resource type="Script" uid="uid://iybteh2g0be4" path="res://player/bullet/bullet.gd" id="1"]` (l.3). **MANTER** `transform`/`collision_layer = 0`/`collision_mask = 3` da raiz, `MultiplayerSynchronizer` com `replication_config` (`.:global_transform`) e o method track `destroy` (`tracks/1/…`, ≈l.93–105). Nada mais muda (plan.md "Edição das cenas", port 2)
- [ ] T023 [US2] Apagar `oxide-godot/player/bullet/bullet.gd` e `oxide-godot/player/bullet/bullet.gd.uid` (`git rm`); verificar `grep -rn 'uid://iybteh2g0be4' oxide-godot/` vazio e `grep -rn 'bullet.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T024 [US2] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . player/bullet/bullet.tscn 2>&1 | tee /tmp/run.log` → exit 124 ou 0; grep de regressão vazio — a bala expira aos 5 s, `explode` roda (lê `/root/Settings`, que **está** carregado em `--path`) e `destroy` aos 1,5 s da animação faz `queue_free` sem erro; repetir para `player/player.tscn` (`BulletCache` instancia `Bullet`) e `level/level.tscn`; grep de regressão vazio nas três (só os `WARNING` da baseline)
- [ ] T025 [US2] Verificações mecânicas e contrato (`contracts/bullet.md` §"Verificação antes do commit"): `grep -n 'type="Bullet"' oxide-godot/player/bullet/bullet.tscn` = 1 linha; `grep -n 'ExtResource("1")' oxide-godot/player/bullet/bullet.tscn` vazio; `grep -n '"method": &"destroy"\|tracks/1/type' oxide-godot/player/bullet/bullet.tscn` = 2 linhas (nome `destroy` idêntico ao `#[func]`); `grep -n 'properties/0/path' oxide-godot/player/bullet/bullet.tscn` = `.:global_transform`; `grep -rn 'explode\|destroy' --include=*.gd oxide-godot` vazio; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `bullet.gd`; `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs` vazio (o Player não muda no port 2 — tipa a bala como `CharacterBody3D`); `ls src/` = 8 arquivos
- [ ] T026 [US2] Registrar em `docs/v2-backlog.md` a linha nº 13: `player/bullet/bullet.gd` (port 2) — evitar o `explode` duplo quando `time_alive` expira e há colisão no mesmo frame (dois RPCs `explode` reiniciam a animação). Não duplicar os itens 1 (Settings tipado) e 2 (`Hittable`), que já cobrem os outros pontos desta bala
- [ ] T027 [US2] Commit único do port 2 na `main` (autor the repository author), incluindo `src/bullet.rs`, `src/lib.rs`, `oxide-godot/player/bullet/bullet.tscn`, as deleções de `bullet.gd`/`bullet.gd.uid` e `docs/v2-backlog.md`. Mensagem: `Port bullet.gd → Bullet (CharacterBody3D); bullet.tscn: node Bullet type="CharacterBody3D"→"Bullet"` + corpo: notas (duck typing `has_method("hit")` + `rpc("hit")` preservado; primeiro uso da exceção `Settings` via `/root/Settings` → `Gd<ConfigFile>`; `destroy` pelo method track; `BulletCache` de `player.tscn` passa a instanciar a classe Rust; quirk preservado: `explode` duplo possível); `- backlog v2: item 13`. Anotar o hash
- [ ] T028 [US2] **Checkpoint do usuário (validação visual, plan.md port 2)** — feito pelo usuário no jogo, comparando com o original: bala azul visível sai do cano e voa reta; explode em paredes/chão (animação + luz) e ao acertar o robô (robô reage ao `hit`); bala perdida explode sozinha após 5 s; some após a explosão; com `Shadow mapping` ligado nas configurações, a luz da explosão projeta sombra; console sem erro no spawn (`BulletCache`). Divergência → commit `Fix port bullet.gd …`. Só seguir para a US3 com o OK explícito

**Checkpoint**: 8 `.gd` restantes; `bullet.tscn` com `Bullet`; jogo jogável.

---

## Phase 5: User Story 3 — Porta portada, com correção conservadora do bug do upstream (Priority: P3)

**Goal**: `door/door.gd` (12 l.) → `Door: Area3D`; abre uma vez (`doorsimple_opening`) quando um
`Player` entra. **Única correção de bug do marco**: referência `DoorModel2/AnimationPlayer` em vez
do inexistente `DoorModel/AnimationPlayer` (FR-030–FR-035), com os 4 requisitos da cláusula:
(a) spec ✓, (b) comentário `// upstream bug fix` no ponto exato, (c) mensagem do commit, (d)
`docs/upstream-bugs.md`.

**Independent Test**: `door/door.tscn` headless com `grep -c 'Node not found'` = **0** (baseline
T005 = 1) e grep de regressão vazio; `level/level.tscn` inalterado; contrato `contracts/door.md`;
`docs/upstream-bugs.md` com 1 entrada; `CLAUDE.md` intocado.

- [ ] T029 [US3] Criar `oxide_godot_core/oxide_godot_lib/src/door.rs` traduzindo `oxide-godot/door/door.gd`: `use godot::classes::{AnimationPlayer, Area3D, IArea3D, Node3D}; use godot::prelude::*; use crate::player::Player;`; `#[derive(GodotClass)] #[class(init, base=Area3D)] pub struct Door { base: Base<Area3D>, open: bool, ` + as duas linhas de comentário **imediatamente acima** do atributo: `// upstream bug fix: door.gd referenciava "DoorModel/AnimationPlayer" (node inexistente);` / `// o node da cena é "DoorModel2" — a porta nunca abria e o Godot imprimia "Node not found".` + `#[init(node = "DoorModel2/AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>> }`; `#[godot_api] impl IArea3D for Door {}` (vazio); `#[godot_api] impl Door { #[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>) { if !self.open && body.try_cast::<Player>().is_ok() { self.animation_player.play_ex().name("doorsimple_opening").done(); self.open = true; } } }`. Nada além disso: não renomear nada, não "melhorar" a lógica de `open` (FR-031, research D15)
- [ ] T030 [US3] Adicionar `mod door;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod bullet;`)
- [ ] T031 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [ ] T032 [US3] Editar `oxide-godot/door/door.tscn` como texto (reconferir com `grep -n 'name="Door" type=\|^script = ExtResource("1")\|door.gd\|DoorModel2\|_on_door_body_entered' oxide-godot/door/door.tscn`): na linha da raiz (l.10) trocar `type="Area3D"` por `type="Door"`; remover `script = ExtResource("1")` (l.11); remover `[ext_resource type="Script" uid="uid://7v3r683kok5s" path="res://door/door.gd" id="1"]` (l.3). **MANTER** `[node name="DoorModel2" ...]` (l.13 — **NÃO renomear**), `AnimationPlayer` (l.25, `autoplay = &"doorsimple_closed"`), `sound`, `CollisionShape3D`, a `[connection signal="body_entered" from="." to="." method="_on_door_body_entered"]` (l.37) e `[editable path="DoorModel2"]`. Nada mais muda (plan.md "Edição das cenas", port 3)
- [ ] T033 [US3] Apagar `oxide-godot/door/door.gd` e `oxide-godot/door/door.gd.uid` (`git rm`); verificar `grep -rn 'uid://7v3r683kok5s' oxide-godot/` vazio e `grep -rn 'door.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T034 [US3] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . door/door.tscn 2>&1 | tee /tmp/run.log` → exit 124; grep de regressão vazio **e** `grep -c 'Node not found' /tmp/run.log` = **0** (baseline T005 = 1 — o erro do upstream desapareceu); `level/level.tscn` com grep de regressão vazio. Se `Node not found` ainda for 1, parar: a correção não foi aplicada no ponto certo
- [ ] T035 [US3] Verificações mecânicas e contrato (`contracts/door.md` §"Verificação antes do commit"): `grep -n 'type="Door"' oxide-godot/door/door.tscn` = 1 linha; `grep -n 'ExtResource("1")' oxide-godot/door/door.tscn` vazio; `grep -n 'method="_on_door_body_entered"' oxide-godot/door/door.tscn` = 1 (nome idêntico ao `#[func]`); `grep -n 'name="DoorModel2"\|name="AnimationPlayer" parent="DoorModel2"' oxide-godot/door/door.tscn` = 2 linhas intocadas; `grep -c 'doorsimple_opening' oxide-godot/door/model/door.dae` = 1; `grep -n 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/door.rs` = 1 linha, imediatamente acima de `#[init(node = "DoorModel2/AnimationPlayer")]`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `door.gd`; `git diff --stat HEAD -- CLAUDE.md` vazio; `ls src/` = 9 arquivos
- [ ] T036 [US3] Criar `docs/upstream-bugs.md` (requisito (d), FR-034): título `# Bugs do upstream corrigidos na v1`, parágrafo explicando o propósito (registro exigido pela constituição v1.3.0, Princípio I — referência para a v2 e para eventual contribuição upstream; cada entrada = correção conservadora declarada na spec da feature) e tabela `| # | Defeito | Script / cena | Correção aplicada | Commit |` com a entrada `1`: defeito `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` — `door.gd:6` referenciava um node inexistente; a porta nunca abria; script/cena `door/door.gd:6` vs `door/door.tscn:13` (`DoorModel2`), spec `specs/002-v1-player-bullet-door` US3/FR-030–FR-035; correção `src/door.rs`: `#[init(node = "DoorModel2/AnimationPlayer")]` com comentário `// upstream bug fix` (nada renomeado na cena; lógica de `open` intacta); commit = assunto `Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"` (sem hash — sem `--amend`)
- [ ] T037 [US3] Registrar em `docs/v2-backlog.md` a linha nº 14 **reduzida**: `door/door.gd` (port 3) — tipar `_on_door_body_entered` com `Gd<Player>` já na fronteira (`try_cast` no sinal) em vez de `Gd<Node3D>` + `try_cast` no corpo. **Não** incluir "considerar fechar a porta" (ideia de feature, não melhoria percebida)
- [ ] T038 [US3] Commit único do port 3 na `main` (autor the repository author), incluindo `src/door.rs`, `src/lib.rs`, `oxide-godot/door/door.tscn`, as deleções de `door.gd`/`door.gd.uid`, `docs/upstream-bugs.md` (novo) e `docs/v2-backlog.md`. Mensagem **obrigatória** (requisito (c), quickstart §8): assunto `Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"`; corpo com as linhas `- upstream bug fix: door.gd referenciava "DoorModel/AnimationPlayer" (node inexistente); o port referencia "DoorModel2/AnimationPlayer" — a porta passa a abrir e o "ERROR: Node not found" desaparece (baseline 1 → 0). Correção mínima; node da cena não renomeado; lógica de open intacta.`, `- docs/upstream-bugs.md criado com a entrada #1.`, `- CLAUDE.md: catálogo inalterado (o erro da porta nunca constou dele — só os 3 erros de import).`, `- backlog v2: item 14`. Anotar o hash
- [ ] T039 [US3] **Checkpoint do usuário (plan.md port 3)** — nada muda no jogo (a porta é asset órfão). O usuário abre `door/door.tscn` no editor: raiz tipo `Door`, sem script, `DoorModel2` intocado, a conexão `body_entered → _on_door_body_entered` preservada no painel de sinais, Output sem `Node not found`. Opcional (SC-009): cena de teste **não commitada** (scratchpad ou `.tscn` temporária removida antes de qualquer commit) com `door.tscn` + `player.tscn` — ao andar até a porta, `doorsimple_opening` toca uma vez. Este checkpoint pode ser dado com base no headless (T034) + editor. Divergência → commit `Fix port door.gd …`. Só seguir para o Polish com o OK explícito

**Checkpoint**: 7 `.gd` restantes; `door.tscn` com `Door`; `docs/upstream-bugs.md` existe; jogo jogável.

---

## Phase 6: Polish — verificação final do marco

**Purpose**: apenas a verificação final do quickstart e a validação visual completa. Sem
documentação extra, sem README, sem refatoração.

- [ ] T040 Verificação mecânica do marco (quickstart §"Verificação final"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 7; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 7; `git diff --stat 108584e -- 'oxide-godot/**/*.gd'` mostra exatamente 3 deleções (`player.gd`, `bullet.gd`, `door.gd`) e nenhuma modificação (os 7 restantes byte a byte iguais — SC-001); `git log --oneline 108584e..HEAD | grep -c '^[0-9a-f]* Port '` = 3 (commits `Fix port …`, se houver, não contam); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs camera_noise_shake.rs player_input.rs player.rs bullet.rs door.rs`; `test -f docs/upstream-bugs.md && grep -c '^| 1 ' docs/upstream-bugs.md` = 1; `grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/` = 1 linha (`door.rs`); `git diff --stat 108584e -- CLAUDE.md` vazio; `grep -c '^| [0-9]' docs/v2-backlog.md` = 14
- [ ] T041 Validação headless final (sem editor aberto): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; import headless com `Initialize godot-rust` e sem `ERROR`; `level/level.tscn`, `player/player.tscn`, `player/bullet/bullet.tscn`, `door/door.tscn` headless com grep de regressão vazio e `grep -c 'Node not found'` = 0 na porta
- [ ] T042 **Validação visual completa do usuário (SC-002, SC-009)**: menu → level; mover, pular (som), pousar (som), mirar, atirar com bala visível que explode e acerta robôs, tremor de câmera, respawn ao cair abaixo de −40, F3, impacto de laser, peças sumindo — tudo indistinguível de `../oxide_godot_origins/` em sessão lado a lado; `player.tscn`, `bullet.tscn`, `door.tscn` no editor com o tipo Rust e sem script. Marco B concluído com o OK do usuário; então marcar T001–T042 `[x]` e commitar só `tasks.md` (`Tasks 002: marco B concluído`)

---

## Dependencies & Execution Order

### Ordem obrigatória

```
Phase 1 (Setup: T001–T005)          — inclui a medição do "Node not found" da porta (= 1)
  → Phase 3 US1 (T006–T018)  → commit 1  (Player; + pub(crate) em player_input.rs/camera_noise_shake.rs)
  → Phase 4 US2 (T019–T028)  → commit 2  (Bullet; BulletCache passa a ser Rust)
  → Phase 5 US3 (T029–T039)  → commit 3  (Door + docs/upstream-bugs.md; "Node not found" = 0)
  → Phase 6 Polish (T040–T042)
```

- **Phase 2 (Foundational)**: não existe — nada bloqueia as stories além do Setup.
- **Stories não são paralelizáveis entre si**: cada uma termina com um commit na `main` e a
  próxima parte da árvore limpa (FR-028, SC-004). US3 exige `Player` em Rust (`is Player`); US2 é
  instanciada pelo Player por API base, e o Player tipa a bala como `CharacterBody3D` justamente
  para que o commit 1 não dependa do commit 2.
- **Ordem interna de cada story** (dependências estritas): [US1: visibilidade `pub(crate)`] →
  módulo `.rs` → `mod` em `lib.rs` → `cargo build` → editar `.tscn` → apagar `.gd`/`.uid` →
  headless → verificações mecânicas + contrato → [US3: `docs/upstream-bugs.md`] → backlog →
  commit → checkpoint do usuário. A `.tscn` só é editada depois do build porque a classe precisa
  existir na lib carregada para o `type` resolver no import headless.
- **Checkpoint do usuário** (T018, T028, T039, T042) é bloqueante: a story seguinte só começa
  com o OK explícito.

### Parallel Opportunities

Praticamente nenhuma, por construção:

- Setup: T004 é [P] em relação a T002/T003/T005 (só lê o diretório de bindings).
- Dentro de cada story, nenhuma task é [P]: cada passo consome o resultado do anterior.
  T006→T007→T008→T009 escrevem arquivos em sequência (T007–T009 o mesmo `player.rs`).
- Backlog (T016, T026, T037) e `docs/upstream-bugs.md` (T036) poderiam ser escritos a qualquer
  momento antes do commit da story, mas editam arquivos compartilhados — manter sequencial.

### Parallel Example

```bash
# Único par realmente independente (Phase 1):
Task: "T002 cargo build → contar warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline registrada, inclusive o erro da porta.
2. Phase 3: US1 (T006–T018) — `player.gd` portado, commit 1, OK do usuário.
3. **PARAR E VALIDAR**: é o maior script do projeto e o primeiro a consumir classes Rust de
   forma tipada e a expor contrato a três consumidores GDScript; qualquer problema de setter fora
   da árvore, replicação por nome ou re-entrância de RPC aparece aqui.

### Incremental Delivery

Cada story é um port completo e o jogo fica jogável após cada commit:

1. US1 → 9 `.gd` restantes → jogável (bala ainda GDScript, instanciada por API base)
2. US2 → 8 → jogável (`BulletCache` e tiros em Rust)
3. US3 → 7 → jogável (nada visível muda; porta passa a funcionar em teste isolado)
4. Polish → marco B concluído

### Se algo falhar no meio de uma story

Não commitar parcial. Ou o port inteiro (módulo + cena + deleção [+ visibilidade / +
`upstream-bugs.md`]) entra no commit, ou nada:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<módulo>.rs docs/upstream-bugs.md`
volta à árvore limpa da story anterior, que é sempre jogável. Se a implementação exigir algo não
previsto nas tasks (outro arquivo, outra correção, outra visibilidade), parar e reportar.

---

## Notes

- Nenhuma task cria helper, trait, módulo comum, teste ou log novo. Se parecer necessário, é
  entrada em `docs/v2-backlog.md` — não task.
- **Uma única correção de bug** (US3). Qualquer outro defeito percebido (inclusive os quirks de
  research D16) fica como está e vai para o backlog; "em caso de dúvida, é melhoria".
- Toda `.tscn` é editada como texto; reconferir linhas com `grep -n` antes de cada edição (as
  linhas citadas são as de 2026-09-15; remover o `ext_resource` da l.3 desloca as demais em −1).
- Nomes de `#[func]`, RPCs e propriedades `#[export]`/`#[var]` são contrato (FR-024) — copiar do
  GDScript, nunca "traduzir" (`add_camera_shake_trauma`, `_on_door_body_entered`, `destroy`,
  `current_animation`, `motion`).
- O grep de regressão headless é `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked`;
  os dois `WARNING` da baseline (`HDR output`, `Physics interpolation`) não contam. Na porta,
  vale também `grep -c 'Node not found'` = 0.
- Nenhum editor Godot aberto durante validações headless — avisar o usuário antes; nunca matar o
  processo dele.
- O commit (T017, T027, T038) vem **antes** do checkpoint visual; divergência encontrada pelo
  usuário é corrigida em commit `Fix port …` na mesma story (nunca `Port …`, para
  `git log | grep -c '^[0-9a-f]* Port '` continuar = 3).
- Arquivos que **nunca** mudam neste marco: `Cargo.toml`, `.gdextension`, `project.godot`,
  `CLAUDE.md`, os 7 `.gd` restantes, `specs/` (exceto o `[x]` em `tasks.md` ao final) e, nos
  módulos do Marco A, qualquer coisa além da palavra `pub(crate)` listada em T006.
