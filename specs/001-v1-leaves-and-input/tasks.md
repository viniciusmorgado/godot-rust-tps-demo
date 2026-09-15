# Tasks: Marco A — folhas e input do jogador (v1 raw port)

**Input**: Design documents from `/specs/001-v1-leaves-and-input/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (todos aprovados)

**Fase**: v1 — Raw Port (Princípio I). Nenhuma task pode introduzir abstração, refatoração,
otimização, teste unitário ou infraestrutura. Se algo assim parecer necessário, vira entrada em
`docs/v2-backlog.md`, não task.

**Tests**: não há testes automatizados nesta fase (plan.md "Testing"). A validação de cada story
é o ciclo do quickstart (build → import headless → cena headless → verificações mecânicas →
validação visual do usuário).

**Organization**: uma phase por user story, na ordem obrigatória US1 → US2 → US3 → US4 → US5
(spec FR-030). As stories **não** são paralelizáveis entre si: cada uma termina com um commit
próprio na `main` e a seguinte começa da árvore limpa; US4 e US5 editam o mesmo arquivo
(`player.tscn`) e o port 4 desloca as linhas do port 5.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência de task incompleta) — raro
  neste marco, porque build depende do módulo, cena depende do build, validação depende da cena.
- **[Story]**: US1..US5 (spec.md)
- Caminhos relativos à raiz do repositório (`oxide-godot/` = projeto Godot;
  `oxide_godot_core/oxide_godot_lib/src/` = crate Rust).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` de cada módulo (só isso)
oxide_godot_core/oxide_godot_lib/src/<módulo>.rs   uma classe por script (research.md §"Mapa por script")
oxide-godot/<cena>.tscn                            troca de `type` (plan.md "Edição das cenas")
oxide-godot/<script>.gd + .gd.uid                  apagados no commit do port
docs/v2-backlog.md                                 uma linha por melhoria percebida
../oxide_godot_origins/                            referência intocada do GDScript original
```

Decisões fechadas (valem para todas as stories — instrução, não opção):

- Métodos usados só internamente ficam **privados, sem `#[func]`**, com o mesmo nome do GDScript
  (`rotate_camera`; `decay_trauma`, `apply_shake`, `get_noise_value`).
- RPC `jump`: `#[rpc(authority, call_local, unreliable)]` — **não** `reliable` (research D5).
- FPS do overlay: `Variant::from(fps).stringify()` para reproduzir `str(float)` = `"60.0"` (D9).
- Raycast: `.exclude(&array![Rid::Invalid])`; **não** excluir o corpo do jogador (D11, FR-017).
- Referências de cena: `#[export] Option<Gd<T>>` + `.as_ref().unwrap()`/`.as_mut().unwrap()`;
  **não** usar `OnEditor` (D3).
- Grafia gdext: `CpuParticles3D`/`ICpuParticles3D`; `AnimationPlayer::play_ex().name("…").done()`.
- Tipos: `delta: f64`; tudo que alimenta `Vector2`/`Vector3`/`Color`/`rotate_y`/`get_noise_1d`
  é `f32` com `as f32` no ponto de uso; `time: f64` no tremor (D13).
- Um commit por script na `main`, mensagem no formato do quickstart passo 8.

---

## Phase 1: Setup

**Purpose**: registrar a baseline contra a qual cada port é comparado. Nada de "criar estrutura
de projeto" ou "configurar lint": o crate já existe e nenhuma infraestrutura pode ser adicionada
(Princípio I).

- [ ] T001 Confirmar pré-condições: nenhum editor Godot aberto no projeto `oxide-godot/`; `git status` limpo na `main`; anotar `git rev-parse --short HEAD` como início do marco. O commit-base `6b22de3` usado por `quickstart.md`/T057 para `git diff -- '*.gd'` e `git log 6b22de3..HEAD` continua válido: os commits posteriores (plan, tasks) não tocam `.gd` nem são commits "Port …"
- [ ] T002 Registrar baseline do build: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → esperado `0`; anotar o valor obtido em `specs/001-v1-leaves-and-input/quickstart.md` §"Baseline" se diferir
- [ ] T003 Registrar baseline do import: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirmar `grep -n 'Initialize godot-rust' /tmp/import.log` (linha 1) e `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` vazio (ou só os 3 erros do upstream do `CLAUDE.md`)
- [ ] T004 [P] Confirmar o diretório de bindings geradas usado como referência pelo research.md: `ls -d oxide_godot_core/target/debug/build/godot-core-*/out` → deve existir exatamente um; se o hash não for `aea5c50e7fda9d57`, atualizar a linha 11 de `specs/001-v1-leaves-and-input/research.md` com o hash atual (as linhas citadas continuam válidas — é a mesma versão 0.5.5)

**Checkpoint**: baseline conhecida — 0 warnings, extensão carrega, 0 `ERROR` no import.

---

## Phase 2: Foundational

**Não existe neste marco.** Os 5 scripts são folhas independentes (`docs/port-order.md`); nenhum
módulo comum, helper, trait ou constante compartilhada pode ser criado (Princípio I — seria
abstração). Cada story cria seu próprio módulo autocontido e a única edição compartilhada é a linha
`mod <módulo>;` em `lib.rs`, feita dentro da própria story.

---

## Phase 3: User Story 1 — Overlay de debug (F3) portado (Priority: P1) 🎯 MVP

**Goal**: `level/debug.gd` (15 l.) → `DebugLabel: Label`; F3 alterna o overlay; texto com FPS,
VSync, memória, online e (se online) ID, recalculado a cada frame — inclusive oculto (quirk).

**Independent Test**: `level/level.tscn` headless sem `ERROR`; no jogo, F3 alterna e o texto mostra
`FPS: 60.0` (com `.0`), `VSync: Enabled|Disabled`, `Memory: xx.xx MiB`, `Online: No`.

- [ ] T005 [US1] Criar `oxide_godot_core/oxide_godot_lib/src/debug_label.rs`: `#[derive(GodotClass)] #[class(init, base=Label)] struct DebugLabel { base: Base<Label> }`; `#[godot_api] impl ILabel for DebugLabel` com `fn process(&mut self, _delta: f64)` traduzindo linha a linha `oxide-godot/level/debug.gd`: (1) `if Input::singleton().is_action_just_pressed("toggle_debug") { let v = self.base().is_visible(); self.base_mut().set_visible(!v); }`; (2) montar `String` com `"FPS: " + Variant::from(Engine::singleton().get_frames_per_second()).stringify()`; (3) `"\nVSync: " + ("Enabled" se DisplayServer::singleton().window_get_vsync_mode() != VSyncMode::DISABLED senão "Disabled")`; (4) `"\nMemory: " + format!("{:3.2}", Os::singleton().get_static_memory_usage() as f64 / 1048576.0) + " MiB"`; (5) `online = !(get_multiplayer().unwrap().get_multiplayer_peer().map(|p| p.try_cast::<OfflineMultiplayerPeer>().is_ok()) == Some(true))`; `"\nOnline: Yes|No"`; (6) se online, `"\nMultiplayer ID: " + get_unique_id()`; (7) `self.base_mut().set_text(&text)`. Sem `#[func]`, sem campos além de `base`. Assinaturas em research.md D9
- [ ] T006 [US1] Adicionar `mod debug_label;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (abaixo de `use godot::prelude::*;`, antes do `struct OxideGodot`; nada mais muda em lib.rs)
- [ ] T007 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`, `grep -c '^warning'` = 0. Corrigir qualquer warning (imports não usados etc.) antes de seguir
- [ ] T008 [US1] Editar `oxide-godot/level/level.tscn` (reconferir linhas com `grep -n 'name="Debug"\|ExtResource("9")\|debug.gd' oxide-godot/level/level.tscn`): na linha do node `Debug` (≈148) trocar `type="Label"` por `type="DebugLabel"`; remover a linha `script = ExtResource("9")` (≈156); remover a linha `[ext_resource type="Script" uid="uid://6ec6m14rhsxi" path="res://level/debug.gd" id="9"]` (≈7). Não tocar em mais nada
- [ ] T009 [US1] Apagar `oxide-godot/level/debug.gd` e `oxide-godot/level/debug.gd.uid` (`git rm`); verificar `grep -rn 'uid://6ec6m14rhsxi' oxide-godot/` vazio e `grep -rn 'debug.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T010 [US1] Validação headless (quickstart passos 2–3): `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log` → contém `Initialize godot-rust`, sem `ERROR` novo; `timeout 20 /usr/bin/godot.x86_64 --headless --path . level/level.tscn 2>&1 | tee /tmp/run.log` → exit 124 esperado; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked' /tmp/run.log` vazio (só os 2 `WARNING` da baseline)
- [ ] T011 [US1] Verificações mecânicas (quickstart passo 4): `grep -n 'type="DebugLabel"' oxide-godot/level/level.tscn` = 1 linha; `grep -n 'ExtResource("9")' oxide-godot/level/level.tscn` vazio; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `level/debug.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs`; conferir que o bloco do node na `.tscn` não define nenhuma propriedade do script (só propriedades da classe base) — Princípio II, preservação de nomes
- [ ] T012 [US1] Registrar em `docs/v2-backlog.md` a linha nº 4: origem `level/debug.gd` (port 1) — "Não recalcular o texto enquanto o overlay está oculto" — motivação "trabalho por frame desnecessário" (research.md §"Backlog v2 candidato"). Não duplicar os itens 1–3 já existentes
- [ ] T013 [US1] Commit na `main` com: `oxide_godot_core/oxide_godot_lib/src/debug_label.rs`, `lib.rs`, `oxide-godot/level/level.tscn`, deleção de `debug.gd` + `.uid`, `docs/v2-backlog.md`. Mensagem (quickstart passo 8): `Port debug.gd → DebugLabel (Label); level.tscn: node Debug type="Label"→"DebugLabel"` + corpo com notas (`str(float)` via `Variant::stringify`; texto recalculado oculto, como no original) e `backlog v2: item 4`
- [ ] T014 [US1] **Checkpoint do usuário (SC-004)**: (abrir `oxide-godot/project.godot` no editor Godot 4.7.2 e rodar com F5) abrir o projeto no editor, rodar (F5), menu → level; conferir F3 alterna o overlay; linhas `FPS: 60.0`, `VSync: …`, `Memory: xx.xx MiB`, `Online: No` (sem linha de ID); valores mudam a cada frame; jogo jogável de ponta a ponta. Comparar com `../oxide_godot_origins/`. Só avançar para a US2 após o OK do usuário

**Checkpoint**: 14 `.gd` restantes; `level.tscn` usa `DebugLabel`; jogo jogável.

---

## Phase 4: User Story 2 — Efeito de desaparecimento de peça portado (Priority: P2)

**Goal**: `part_disappear.gd` (9 l.) → `PartDisappear: CpuParticles3D`; `MiniBlasts` emite na
hora, o próprio emissor após 0,2 s, `queue_free` após 2×`lifetime` — dois `await` encadeados
viram duas conexões a `SceneTreeTimer.timeout`.

**Independent Test**: `part_disappear.tscn` headless sem `ERROR` (exit 124 — `queue_free` da raiz
não encerra a SceneTree); no jogo, ao matar um robô, cada peça some com mini-blasts + puff.

- [ ] T015 [US2] Criar `oxide_godot_core/oxide_godot_lib/src/part_disappear.rs`: `#[class(init, base=CpuParticles3D)] struct PartDisappear { base: Base<CpuParticles3D>, #[init(node = "MiniBlasts")] mini_blasts: OnReady<Gd<CpuParticles3D>> }`; `#[godot_api] impl ICpuParticles3D for PartDisappear` com `fn ready(&mut self)` traduzindo `oxide-godot/enemies/red_robot/parts/part_disappear_effect/part_disappear.gd`: (1) `self.mini_blasts.set_emitting(true);` (2) `let timer = self.base().get_tree().create_timer(0.2); timer.signals().timeout().connect_other(&*self, |this: &mut PartDisappear| { this.base_mut().set_emitting(true); let t2 = this.base().get_tree().create_timer(this.base().get_lifetime() * 2.0); t2.signals().timeout().connect_other(&*this, |this2: &mut PartDisappear| this2.base_mut().queue_free()); });`. Sem `#[func]`. Assinaturas e justificativa (callable linked, invalidada se o node for liberado) em research.md D6
- [ ] T016 [US2] Adicionar `mod part_disappear;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [ ] T017 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings. Se o borrow checker recusar `&*self`/`&*this` dentro do closure aninhado, usar `let gd = this.to_gd();` e `connect_other(&gd, ...)` — mesma semântica (research D6 aceita `&Gd<T>`)
- [ ] T018 [US2] Editar `oxide-godot/enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn` (reconferir com `grep -n 'PartDisappearPuff\|ExtResource("3")\|part_disappear.gd'`): node raiz `PartDisappearPuff` (≈43) `type="CPUParticles3D"` → `type="PartDisappear"`; remover `script = ExtResource("3")` (≈59); remover o `[ext_resource type="Script" uid="uid://dxd6xoeg627y6" ... id="3"]` (≈5)
- [ ] T019 [US2] Apagar `part_disappear.gd` e `part_disappear.gd.uid` em `oxide-godot/enemies/red_robot/parts/part_disappear_effect/` (`git rm`); `grep -rn 'uid://dxd6xoeg627y6' oxide-godot/` vazio; `grep -rn 'part_disappear.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio (o `preload` em `part.gd:4` referencia a `.tscn`, não o `.gd` — deve permanecer)
- [ ] T020 [US2] Validação headless: import (`Initialize godot-rust`, sem `ERROR` novo) e `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn 2>&1 | tee /tmp/run.log` → `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked' /tmp/run.log` vazio; rodar também `level/level.tscn` (mesmo grep vazio)
- [ ] T021 [US2] Verificações mecânicas: `grep -n 'type="PartDisappear"' .../part_disappear.tscn` = 1; `grep -n 'ExtResource("3")' .../part_disappear.tscn` vazio; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` só a deleção de `part_disappear.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs`; conferir que o bloco do node na `.tscn` não define nenhuma propriedade do script (só propriedades da classe base) — Princípio II, preservação de nomes
- [ ] T022 [US2] Registrar em `docs/v2-backlog.md` a linha nº 5: origem `part_disappear.gd` / `blast.gd` (ports 2–3) — "Timers/sinais como `async` (`godot::task`) quando a API estabilizar" — motivação "encadeamento de closures reproduz `await` de forma menos legível". (A US3 não adiciona outra linha: este item cobre os dois scripts)
- [ ] T023 [US2] Commit na `main`: `part_disappear.rs`, `lib.rs`, `part_disappear.tscn`, deleção `.gd` + `.uid`, `docs/v2-backlog.md`. Mensagem: `Port part_disappear.gd → PartDisappear (CpuParticles3D); part_disappear.tscn: node PartDisappearPuff type="CPUParticles3D"→"PartDisappear"` + notas (`await create_timer` → `connect_other` com callable linked) + `backlog v2: item 5`
- [ ] T024 [US2] **Checkpoint do usuário (SC-004)**: (abrir `oxide-godot/project.godot` no editor Godot 4.7.2 e rodar com F5) no jogo, matar um robô; cada peça ao sumir dispara mini-blasts imediatamente e o puff de fumaça ~0,2 s depois; o efeito some sozinho (~3,2 s); nenhum erro no console do editor. Jogo jogável. Comparar com `../oxide_godot_origins/`. Só avançar após OK

**Checkpoint**: 13 `.gd` restantes; jogo jogável.

---

## Phase 5: User Story 3 — Impacto do laser portado (Priority: P3)

**Goal**: `blast.gd` (15 l.) → `Blast: Node3D`; captura a câmera ativa no `ready`, orienta
`LightRays` para ela a cada frame enquanto válida, `queue_free` em `animation_finished`.

**Independent Test**: `impact_effect.tscn` headless sem `ERROR`; no jogo, o impacto do laser do
robô anima com os raios voltados para a câmera e some ao fim da animação.

- [ ] T025 [US3] Criar `oxide_godot_core/oxide_godot_lib/src/blast.rs`: `#[class(init, base=Node3D)] struct Blast { base: Base<Node3D>, #[init(node = "LightRays")] light_rays: OnReady<Gd<CpuParticles3D>>, #[init(node = "AnimationPlayer")] animation_player: OnReady<Gd<AnimationPlayer>>, camera: Option<Gd<Camera3D>> }`; `#[godot_api] impl INode3D for Blast`: `fn ready(&mut self)` = (1) `self.camera = self.base().get_tree().get_root().unwrap().get_camera_3d();` (tradução do `@onready var camera`, antes do resto); (2) `self.animation_player.signals().animation_finished().connect_other(&*self, |this: &mut Blast, _anim_name: StringName| this.base_mut().queue_free());`; `fn process(&mut self, _delta: f64)` = `if let Some(cam) = &self.camera { if cam.is_instance_valid() { let origin = cam.get_global_transform().origin; self.light_rays.look_at(origin); } }`. Sem `#[func]`. Assinaturas em research.md D7–D8
- [ ] T026 [US3] Adicionar `mod blast;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [ ] T027 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings
- [ ] T028 [US3] Editar `oxide-godot/enemies/red_robot/laser/impact_effect/impact_effect.tscn` (reconferir com `grep -n 'name="Blast" type="Node3D"\|ExtResource("5")\|blast.gd'`): node raiz `Blast` (≈170) `type="Node3D"` → `type="Blast"`; remover `script = ExtResource("5")` (≈171); remover o `[ext_resource type="Script" uid="uid://bk20efkdq4v3m" ... id="5"]` (≈7). Atenção: existe um node filho também chamado `Blast` (`type="CPUParticles3D"`, ≈177) — não tocar nele
- [ ] T029 [US3] Apagar `blast.gd` e `blast.gd.uid` em `oxide-godot/enemies/red_robot/laser/impact_effect/` (`git rm`); `grep -rn 'uid://bk20efkdq4v3m' oxide-godot/` vazio; `grep -rn 'blast.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio (o `preload` em `red_robot.gd:35` referencia a `.tscn` — permanece)
- [ ] T030 [US3] Validação headless: import (`Initialize godot-rust`, sem `ERROR` novo) e `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/laser/impact_effect/impact_effect.tscn 2>&1 | tee /tmp/run.log` → grep de regressão vazio; rodar também `level/level.tscn` (grep vazio)
- [ ] T031 [US3] Verificações mecânicas: `grep -n 'type="Blast"' .../impact_effect.tscn` = 1 (a raiz); `grep -n 'ExtResource("5")' .../impact_effect.tscn` vazio; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` só a deleção de `blast.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs`; conferir que o bloco do node na `.tscn` não define nenhuma propriedade do script (só propriedades da classe base) — Princípio II, preservação de nomes
- [ ] T032 [US3] Backlog: nenhuma linha nova em `docs/v2-backlog.md` (o item 5, adicionado na US2, já cobre `blast.gd`); confirmar que a linha 5 cita `blast.gd` — se não citar, editar a linha 5 para incluir
- [ ] T033 [US3] Commit na `main`: `blast.rs`, `lib.rs`, `impact_effect.tscn`, deleção `.gd` + `.uid` (+ `docs/v2-backlog.md` se T032 editou). Mensagem: `Port blast.gd → Blast (Node3D); impact_effect.tscn: node Blast (raiz) type="Node3D"→"Blast"` + notas (`await animation_finished` → `connect_other`; câmera capturada no `ready` e checada com `is_instance_valid`) + `backlog v2: nenhum (coberto pelo item 5)`
- [ ] T034 [US3] **Checkpoint do usuário (SC-004)**: (abrir `oxide-godot/project.godot` no editor Godot 4.7.2 e rodar com F5) no jogo, deixar o robô atirar no jogador ou numa parede; o impacto anima, os raios de luz ficam voltados para a câmera ao mover-se ao redor, o efeito some ao fim da animação; sem erros no console. Jogo jogável. Comparar com o original. Só avançar após OK

**Checkpoint**: 12 `.gd` restantes; jogo jogável.

---

## Phase 6: User Story 4 — Tremor de câmera portado (Priority: P4)

**Goal**: `camera_noise_shake_effect.gd` (61 l.) → `CameraNoiseShake: Camera3D`; expõe
`add_trauma(amount)` (chamado por `player.gd:211`, ainda em GDScript); trauma ≤ 1,2, decai
1,5/s, rotação = `start_rotation` + ruído × trauma².

**Independent Test**: `player.tscn` e `level.tscn` headless sem `ERROR`; contrato
`contracts/camera-noise-shake.md` conferido; no jogo, atirar/ser atingido/ser acertado pelo robô
treme a câmera com a mesma intensidade e ela volta exatamente ao repouso.

- [ ] T035 [US4] Criar `oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs`: constantes `const SPEED: f32 = 1.0; const DECAY_RATE: f32 = 1.5; const MAX_YAW: f32 = 0.05; const MAX_PITCH: f32 = 0.05; const MAX_ROLL: f32 = 0.1; const MAX_TRAUMA: f32 = 1.2;`; `#[class(init, base=Camera3D)] struct CameraNoiseShake { base: Base<Camera3D>, start_rotation: Vector3, trauma: f32, time: f64, #[init(val = FastNoiseLite::new_gd())] noise: Gd<FastNoiseLite>, #[init(val = (randi() as i32))] noise_seed: i32 }`; `#[godot_api] impl ICamera3D`: `fn ready` = `self.noise.set_seed(self.noise_seed); self.noise.set_fractal_octaves(1); self.noise.set_fractal_lacunarity(1.0); self.start_rotation = self.base().get_rotation();`; `fn process(&mut self, delta: f64)` = `if self.trauma > 0.0 { self.decay_trauma(delta); self.apply_shake(delta); }`. `#[godot_api] impl CameraNoiseShake`: `#[func] fn add_trauma(&mut self, amount: f64) { self.trauma = (self.trauma + amount as f32).min(MAX_TRAUMA); }`. Métodos **privados, sem `#[func]`**, em `impl CameraNoiseShake` comum: `fn decay_trauma(&mut self, delta: f64) { let change = DECAY_RATE * delta as f32; self.trauma = (self.trauma - change).max(0.0); }`; `fn apply_shake(&mut self, delta: f64) { self.time += delta * SPEED as f64 * 5000.0; let shake = self.trauma * self.trauma; let yaw = MAX_YAW * shake * self.get_noise_value(self.noise_seed, self.time); let pitch = MAX_PITCH * shake * self.get_noise_value(self.noise_seed.wrapping_add(1), self.time); let roll = MAX_ROLL * shake * self.get_noise_value(self.noise_seed.wrapping_add(2), self.time); let r = self.start_rotation + Vector3::new(pitch, yaw, roll); self.base_mut().set_rotation(r); }`; `fn get_noise_value(&mut self, seed_value: i32, pos: f64) -> f32 { self.noise.set_seed(seed_value); self.noise.get_noise_1d(pos as f32) }`. Assinaturas em research.md D10, D13
- [ ] T036 [US4] Adicionar `mod camera_noise_shake;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [ ] T037 [US4] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings (atenção a `randi` — importar `godot::global::randi`)
- [ ] T038 [US4] Editar `oxide-godot/player/player.tscn` (reconferir com `grep -n 'name="Camera3D"\|ExtResource("8")\|camera_noise_shake_effect.gd' oxide-godot/player/player.tscn`): node `Camera3D` (≈630, `parent="CameraBase/CameraRot/SpringArm3D"`) `type="Camera3D"` → `type="CameraNoiseShake"`; remover `script = ExtResource("8")` (≈633); remover o `[ext_resource type="Script" uid="uid://byrvr71jmaisi" path="res://player/camera_noise_shake_effect.gd" id="8"]` (≈11). Não tocar no `InputSynchronizer` nem em `ExtResource("2_g11dy")` (é a US5)
- [ ] T039 [US4] Apagar `oxide-godot/player/camera_noise_shake_effect.gd` e `.gd.uid` (`git rm`); `grep -rn 'uid://byrvr71jmaisi' oxide-godot/` vazio; `grep -rn 'camera_noise_shake_effect.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T040 [US4] Validação headless: import (`Initialize godot-rust`, sem `ERROR` novo); `timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` → grep de regressão vazio; idem `level/level.tscn`. Em particular, nenhuma linha `Invalid call. Nonexistent function 'add_trauma'` (só apareceria sob interação, mas o headless carrega `player.gd` e resolve o tipo)
- [ ] T041 [US4] Verificações mecânicas e contrato: `grep -n 'type="CameraNoiseShake"' oxide-godot/player/player.tscn` = 1; `grep -n 'ExtResource("8")' oxide-godot/player/player.tscn` vazio; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` só a deleção de `camera_noise_shake_effect.gd`; `grep -rn 'add_trauma\|add_camera_shake_trauma' --include=*.gd oxide-godot/` retorna exatamente `player.gd:201,206,210,211` e `red_robot.gd:133` (contracts/camera-noise-shake.md); `add_trauma` existe como `#[func]` em `camera_noise_shake.rs`
- [ ] T042 [US4] Registrar em `docs/v2-backlog.md` a linha nº 6: origem `player/camera_noise_shake_effect.gd` (port 4) — "Recapturar `start_rotation` quando animações/scripts movem a câmera" — motivação "comentário do original admite o problema; o tremor soma à rotação capturada uma única vez"
- [ ] T043 [US4] Commit na `main`: `camera_noise_shake.rs`, `lib.rs`, `player.tscn`, deleção `.gd` + `.uid`, `docs/v2-backlog.md`. Mensagem: `Port camera_noise_shake_effect.gd → CameraNoiseShake (Camera3D); player.tscn: node Camera3D type="Camera3D"→"CameraNoiseShake"` + notas (`decay_trauma`/`apply_shake`/`get_noise_value` privados sem `#[func]`; `time: f64`; `wrapping_add` nas sementes) + `backlog v2: item 6`
- [ ] T044 [US4] **Checkpoint do usuário (SC-004)**: (abrir `oxide-godot/project.godot` no editor Godot 4.7.2 e rodar com F5) no jogo, atirar (tremor leve, 0,35) e ser acertado pelo laser do robô (forte, 13,0 saturado em 1,2); o nível médio (0,75, RPC `hit` do player) só ocorre em multiplayer quando a bala de OUTRO jogador acerta — não é verificável single-player; a câmera volta exatamente à posição de repouso; intensidade e duração iguais ao original (padrão de ruído pode diferir — semente aleatória). Sem erros no console. Só avançar após OK

**Checkpoint**: 11 `.gd` restantes; `player.tscn` com `CameraNoiseShake`; jogo jogável.

---

## Phase 7: User Story 5 — Sincronizador de input do jogador portado (Priority: P5)

**Goal**: `player_input.gd` (142 l.) → `PlayerInputSynchronizer: MultiplayerSynchronizer` (nome
**obrigatório**: `player.gd:26` tipa `$InputSynchronizer` como `PlayerInputSynchronizer`); 5
propriedades exportadas, 6 referências via `node_paths`, 3 `#[func]`, RPC `jump`; `player.gd`
continua consumindo tudo sem alteração.

**Independent Test**: `player.tscn` e `level.tscn` headless sem `ERROR`; contrato
`contracts/player-input-synchronizer.md` conferido nome a nome; no jogo, mover, olhar (mouse e
analógico, mais lento ao mirar), pitch travado, mira toggle/hold com animações shoot/far, pular,
atirar no ponto sob o crosshair, fade ao cair do mapa.

- [ ] T045 [US5] Criar `oxide_godot_core/oxide_godot_lib/src/player_input.rs` — **struct e campos**: constantes `const CAMERA_CONTROLLER_ROTATION_SPEED: f32 = 3.0; const CAMERA_MOUSE_ROTATION_SPEED: f32 = 0.001; const CAMERA_X_ROT_MIN: f32 = (-89.9_f32).to_radians(); const CAMERA_X_ROT_MAX: f32 = 70.0_f32.to_radians(); const AIM_HOLD_THRESHOLD: f32 = 0.4;`; `#[class(init, base=MultiplayerSynchronizer)] struct PlayerInputSynchronizer { base: Base<MultiplayerSynchronizer>, toggled_aim: bool, aiming_timer: f32, #[export] aiming: bool, #[export] shoot_target: Vector3, #[export] motion: Vector2, #[export] shooting: bool, #[export] jumping: bool, #[export] camera_animation: Option<Gd<AnimationPlayer>>, #[export] crosshair: Option<Gd<TextureRect>>, #[export] camera_base: Option<Gd<Node3D>>, #[export] camera_rot: Option<Gd<Node3D>>, #[export] camera_camera: Option<Gd<Camera3D>>, #[export] color_rect: Option<Gd<ColorRect>> }` — nomes exatamente estes (data-model.md §1; replicados em `player.tscn:42–53`, `node_paths` em `:343,346–351`)
- [ ] T046 [US5] Em `player_input.rs` — **`impl IMultiplayerSynchronizer`**: `fn ready` = `if self.base().get_multiplayer_authority() == self.base().get_multiplayer().unwrap().get_unique_id() { self.camera_camera.as_mut().unwrap().make_current(); Input::singleton().set_mouse_mode(MouseMode::CAPTURED); } else { self.base_mut().set_process(false); self.base_mut().set_process_input(false); self.color_rect.as_mut().unwrap().hide(); }`. `fn process(&mut self, delta: f64)` traduzindo `_process` de `oxide-godot/player/player_input.gd` linha a linha: (1) `motion` = `Vector2::new(strength("move_right") - strength("move_left"), strength("move_back") - strength("move_forward"))` com `Input::singleton().get_action_strength(..)`; (2) `camera_move` idem com `view_right/left/up/down`; `camera_speed_this_frame = delta as f32 * CAMERA_CONTROLLER_ROTATION_SPEED`, `*= 0.5` se `self.aiming`; `self.rotate_camera(camera_move * camera_speed_this_frame)`; (3) máquina de mira (data-model.md §1): `current_aim`, `toggled_aim`, `aiming_timer` exatamente como no GDScript (`is_action_just_released("aim") && aiming_timer <= AIM_HOLD_THRESHOLD` → `current_aim = true; toggled_aim = true`; senão `current_aim = toggled_aim || is_action_pressed("aim")` e `if is_action_just_pressed("aim") { toggled_aim = false }`); `aiming_timer += delta as f32` se `current_aim` senão `= 0.0`; se `aiming != current_aim`: `aiming = current_aim` e `camera_animation.as_mut().unwrap().play_ex().name(if aiming {"shoot"} else {"far"}).done()`; (4) `if is_action_just_pressed("jump") { self.base_mut().rpc("jump", &[]); }`; (5) `shooting = is_action_pressed("shoot")`; se `shooting`: `ch_pos = crosshair.get_position() + crosshair.get_size() * 0.5`; `ray_from = camera_camera.project_ray_origin(ch_pos)`; `ray_dir = camera_camera.project_ray_normal(ch_pos)`; `params = PhysicsRayQueryParameters3D::create_ex(ray_from, ray_from + ray_dir * 1000.0).collision_mask(0b11).exclude(&array![Rid::Invalid]).done().unwrap()`; `col = self.base().get_parent().unwrap().cast::<Node3D>().get_world_3d().unwrap().get_direct_space_state().unwrap().intersect_ray(&params)`; `shoot_target = if col.is_empty() { ray_from + ray_dir * 1000.0 } else { col.get("position").unwrap().to::<Vector3>() }` — **não** excluir o corpo do jogador (FR-017, research D11); (6) fade: `y = get_parent().cast::<Node3D>().get_global_transform().origin.y`; `let mut m = color_rect.get_modulate(); if y < -17.0 { m.a = ((-17.0 - y) / 15.0).min(1.0) } else { m.a *= 1.0 - delta as f32 * 4.0 }; color_rect.set_modulate(m)`. `fn input(&mut self, event: Gd<InputEvent>)` = `if let Ok(mm) = event.try_cast::<InputEventMouseMotion>() { let mut speed = CAMERA_MOUSE_ROTATION_SPEED; if self.aiming { speed *= 0.75; } self.rotate_camera(mm.get_screen_relative() * speed); }`. Assinaturas em research.md D9, D11, D12
- [ ] T047 [US5] Em `player_input.rs` — **API pública e RPC** em `#[godot_api] impl PlayerInputSynchronizer`: `#[func] fn get_aim_rotation(&self) -> f64 { let x = self.camera_rot.as_ref().unwrap().get_rotation().x.clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); if x >= 0.0 { (-x / CAMERA_X_ROT_MAX) as f64 } else { (x / CAMERA_X_ROT_MIN) as f64 } }`; `#[func] fn get_camera_base_quaternion(&self) -> Quaternion { self.camera_base.as_ref().unwrap().get_global_transform().basis.get_quaternion() }`; `#[func] fn get_camera_rotation_basis(&self) -> Basis { self.camera_rot.as_ref().unwrap().get_global_transform().basis }`; `#[rpc(authority, call_local, unreliable)] fn jump(&mut self) { self.jumping = true; }` (**não** `reliable` — research D5). Método **privado, sem `#[func]`**, em `impl PlayerInputSynchronizer` comum: `fn rotate_camera(&mut self, mv: Vector2) { let cb = self.camera_base.as_mut().unwrap(); cb.rotate_y(-mv.x); cb.orthonormalize(); let cr = self.camera_rot.as_mut().unwrap(); let mut r = cr.get_rotation(); r.x = (r.x + mv.y).clamp(CAMERA_X_ROT_MIN, CAMERA_X_ROT_MAX); cr.set_rotation(r); }`
- [ ] T048 [US5] Adicionar `mod player_input;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs`
- [ ] T049 [US5] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings. Imports esperados: `godot::prelude::*`, `godot::classes::{Input, InputEvent, InputEventMouseMotion, MultiplayerSynchronizer, IMultiplayerSynchronizer, AnimationPlayer, TextureRect, ColorRect, Camera3D, Node3D, PhysicsRayQueryParameters3D}`, `godot::classes::input::MouseMode`. Se `#[rpc]` reclamar de `Base<T>` ausente, conferir que o campo se chama `base` e é `Base<MultiplayerSynchronizer>`
- [ ] T050 [US5] Editar `oxide-godot/player/player.tscn` (**reconferir linhas** — o port 4 removeu uma linha de `ext_resource`, deslocando tudo em −1: `grep -n 'name="InputSynchronizer"\|ExtResource("2_g11dy")\|player_input.gd' oxide-godot/player/player.tscn`): na linha do node `InputSynchronizer` (≈342) trocar `type="MultiplayerSynchronizer"` por `type="PlayerInputSynchronizer"` **mantendo** na mesma linha `parent="."`, `unique_id=…` e `node_paths=PackedStringArray("camera_animation", "crosshair", "camera_base", "camera_rot", "camera_camera", "color_rect")`; remover só a linha `script = ExtResource("2_g11dy")` (≈344); **manter** `replication_config = SubResource("SceneReplicationConfig_8yuxf")` e as 6 linhas `camera_animation = NodePath(...)` … `color_rect = NodePath(...)`; remover o `[ext_resource type="Script" uid="uid://m1xn31x0lssx" path="res://player/player_input.gd" id="2_g11dy"]` (≈5). Não tocar no `SceneReplicationConfig_8yuxf` (linhas ≈34–52 após o port 4)
- [ ] T051 [US5] Apagar `oxide-godot/player/player_input.gd` e `.gd.uid` (`git rm`); `grep -rn 'uid://m1xn31x0lssx' oxide-godot/` vazio; `grep -rn 'player_input.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio; `grep -rn 'PlayerInputSynchronizer' oxide-godot/ --include='*.gd'` retorna só `player/player.gd:26` (a anotação de tipo, agora resolvida na classe nativa)
- [ ] T052 [US5] Validação headless: import (`Initialize godot-rust`, sem `ERROR` novo — em particular nenhum erro de parse em `player.gd` sobre o tipo `PlayerInputSynchronizer`); `timeout 20 /usr/bin/godot.x86_64 --headless --path . player/player.tscn 2>&1 | tee /tmp/run.log` → grep de regressão vazio (o `ready` roda o ramo autoridade em headless: `make_current` + mouse capturado, sem erro); idem `level/level.tscn` (spawna o Player; `player.gd` lê `player_input.motion` etc. a cada frame — qualquer nome errado apareceria aqui como `Invalid get index`)
- [ ] T053 [US5] Verificações mecânicas e contrato: `grep -n 'type="PlayerInputSynchronizer"' oxide-godot/player/player.tscn` = 1 e a mesma linha contém `node_paths=`; `grep -c 'NodePath("../' oxide-godot/player/player.tscn` inclui as 6 referências; `grep -n 'ExtResource("2_g11dy")' oxide-godot/player/player.tscn` vazio; `grep -n 'InputSynchronizer:' oxide-godot/player/player.tscn` mostra `shoot_target`, `motion`, `shooting`, `aiming`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` só a deleção de `player_input.gd`; rodar `grep -n 'player_input\.\|PlayerInputSynchronizer\|\$InputSynchronizer' oxide-godot/player/player.gd` e conferir cada nome (`get_aim_rotation`, `motion`, `get_camera_rotation_basis`, `jumping`, `aiming`, `get_camera_base_quaternion`, `shooting`, `shoot_target`, `camera_camera`) contra `player_input.rs` (contracts/player-input-synchronizer.md)
- [ ] T054 [US5] Registrar em `docs/v2-backlog.md` as linhas nº 7, 8 e 9, origem `player/player_input.gd` (port 5): (7) "Excluir de fato o corpo do jogador no raycast (`exclude` com o RID do `CharacterBody3D` pai)" — "o original passa `[RID(0)]`; exclusão inefetiva"; (8) "`OnEditor<Gd<T>>` em vez de `Option<Gd<T>>` para as 6 referências obrigatórias" — "elimina `unwrap()` por frame e faz o editor sinalizar referência ausente"; (9) "Replicar `jumping` ou remover o `@export`" — "exportado mas fora da replicação; só faz sentido via RPC"
- [ ] T055 [US5] Commit na `main`: `player_input.rs`, `lib.rs`, `player.tscn`, deleção `.gd` + `.uid`, `docs/v2-backlog.md`. Mensagem: `Port player_input.gd → PlayerInputSynchronizer (MultiplayerSynchronizer); player.tscn: node InputSynchronizer type="MultiplayerSynchronizer"→"PlayerInputSynchronizer"` + notas (`#[rpc(authority, call_local, unreliable)]` = `@rpc("call_local")`; raycast com `exclude([RID(0)])` preservado; `rotate_camera` privado; `Option<Gd<T>>` para `node_paths`) + `backlog v2: itens 7–9`
- [ ] T056 [US5] **Checkpoint do usuário (SC-004)**: (abrir `oxide-godot/project.godot` no editor Godot 4.7.2 e rodar com F5) no jogo — mover (WASD/analógico); olhar com mouse e analógico (mais lento ao mirar); pitch trava em −89,9°/70°; mira por toque curto fica ligada e desliga no toque seguinte; mira por hold (> 0,4 s) solta ao soltar; animações de câmera shoot/far; pular; atirar acerta o ponto sob o crosshair; cair pelo buraco do mapa escurece a tela (preta em y ≤ −32) e ao voltar faz fade-out; F3 ainda funciona. Sem erros no console. Comparar com o original. Só avançar após OK

**Checkpoint**: 10 `.gd` restantes; `player.tscn` com `CameraNoiseShake` + `PlayerInputSynchronizer`; jogo jogável.

---

## Phase 8: Polish — verificação final do marco

**Purpose**: apenas a verificação final do quickstart e a validação visual completa. Sem
documentação extra, sem README, sem refatoração.

- [ ] T057 Verificação mecânica do marco (quickstart §"Verificação final"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 10; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 10; `git diff --stat 6b22de3 -- 'oxide-godot/**/*.gd'` mostra exatamente 5 deleções (`debug.gd`, `part_disappear.gd`, `blast.gd`, `camera_noise_shake_effect.gd`, `player_input.gd`) e nenhuma modificação; `git log --oneline 6b22de3..HEAD | grep -c '^[0-9a-f]* Port '` = 5; (commits `Fix port …`, se houver, não contam); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs debug_label.rs part_disappear.rs blast.rs camera_noise_shake.rs player_input.rs`; `grep -c '^| ' docs/v2-backlog.md` = 10 (cabeçalho + 9 itens)
- [ ] T058 Validação headless final: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; import headless com `Initialize godot-rust` e sem `ERROR`; `level/level.tscn`, `player/player.tscn`, `part_disappear.tscn`, `impact_effect.tscn` headless com grep de regressão vazio
- [ ] T059 **Validação visual completa do usuário (SC-002)**: menu → level; mover, olhar, mirar (toggle e hold), pular, atirar, tremor de câmera, F3, impacto de laser, desaparecimento das peças do robô — todos indistinguíveis de `../oxide_godot_origins/` em sessão lado a lado. Abrir `level.tscn`, `player.tscn`, `part_disappear.tscn`, `impact_effect.tscn` no editor: nodes com o tipo Rust, sem script anexado, `InputSynchronizer` com as 6 referências e `replication_config` no inspector. Marco A concluído com o OK do usuário

---

## Dependencies & Execution Order

### Ordem obrigatória

```
Phase 1 (Setup: T001–T004)
  → Phase 3 US1 (T005–T014)  → commit 1
  → Phase 4 US2 (T015–T024)  → commit 2
  → Phase 5 US3 (T025–T034)  → commit 3
  → Phase 6 US4 (T035–T044)  → commit 4   (edita player.tscn)
  → Phase 7 US5 (T045–T056)  → commit 5   (edita player.tscn — linhas deslocadas pelo commit 4)
  → Phase 8 Polish (T057–T059)
```

- **Phase 2 (Foundational)**: não existe — nada bloqueia as stories além do Setup.
- **Stories não são paralelizáveis entre si**: cada uma termina com um commit na `main` e a
  próxima parte da árvore limpa (FR-030, SC-004: o jogo tem de estar jogável após cada commit).
  US4 e US5 tocam o mesmo `player.tscn`.
- **Ordem interna de cada story** (dependências estritas): módulo `.rs` → `mod` em `lib.rs` →
  `cargo build` → editar `.tscn` → apagar `.gd`/`.uid` → headless → verificações mecânicas →
  backlog → commit → checkpoint do usuário. A `.tscn` só é editada depois do build porque a
  classe precisa existir na lib carregada para o `type` resolver no import headless.
- **Checkpoint do usuário** (T014, T024, T034, T044, T056, T059) é bloqueante: a story seguinte
  só começa com o OK.

### Parallel Opportunities

Praticamente nenhuma, por construção:

- Setup: T004 é [P] em relação a T002/T003 (só lê o diretório de bindings).
- Dentro de cada story, nenhuma task é [P]: cada passo consome o resultado do anterior
  (build depende do módulo; cena depende do build; validação depende da cena; commit depende de
  tudo). T045→T046→T047 escrevem o mesmo arquivo `player_input.rs` em sequência.
- Backlog (T012, T022, T042, T054) poderia ser escrito a qualquer momento antes do commit da
  story, mas edita `docs/v2-backlog.md`, compartilhado entre stories — manter sequencial.

### Parallel Example

```bash
# Único par realmente independente (Phase 1):
Task: "T002 cargo build → contar warnings"
Task: "T004 ls -d oxide_godot_core/target/debug/build/godot-core-*/out"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T004) — baseline registrada.
2. Phase 3: US1 (T005–T014) — `debug.gd` portado, commit 1, OK do usuário.
3. **PARAR E VALIDAR**: o ciclo completo do Princípio II (build → import → cena → commit) está
   provado no script mais simples; qualquer problema de toolchain/cena/hot-reload aparece aqui.

### Incremental Delivery

Cada story é um port completo e o jogo fica jogável após cada commit:

1. US1 → 14 `.gd` restantes → jogável
2. US2 → 13 → jogável
3. US3 → 12 → jogável
4. US4 → 11 → jogável (`player.tscn` editada uma vez)
5. US5 → 10 → jogável (`player.tscn` editada de novo; contrato de `player.gd` preservado)
6. Polish → marco A concluído

### Se algo falhar no meio de uma story

Não commitar parcial. Ou o port inteiro (módulo + cena + deleção) entra no commit, ou nada:
`git checkout -- oxide-godot/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<módulo>.rs`
volta à árvore limpa da story anterior, que é sempre jogável.

---

## Notes

- Nenhuma task cria helper, trait, módulo comum, teste ou log novo. Se parecer necessário, é
  entrada em `docs/v2-backlog.md` — não task.
- Toda `.tscn` é editada como texto; reconferir linhas com `grep -n` antes de cada edição (as
  linhas citadas são as de 2026-09-15 e a US4 desloca as da US5).
- Nomes de `#[func]` e de propriedades `#[export]` são contrato (FR-024) — copiar do GDScript,
  nunca "traduzir" (`get_camera_base_quaternion`, não `camera_base_quaternion`).
- O grep de regressão headless é `ERROR|SCRIPT ERROR|Invalid call|Nonexistent|panicked`; os dois
  `WARNING` da baseline (`HDR output`, `Physics interpolation`) não contam.
- Commit só quando o usuário der OK no checkpoint? **Não**: o commit (T013 etc.) vem antes do
  checkpoint visual; se o usuário encontrar divergência, corrige-se em commit de fix na mesma
  story com prefixo `Fix port …` (nunca `Port …`, para `git log | grep -c '^[0-9a-f]* Port '` continuar = 5)
  story antes de começar a próxima.
