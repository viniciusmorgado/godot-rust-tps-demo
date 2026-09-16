# Tasks: Marco D — empilhadeira, level, menu e main (v1 raw port)

**Input**: Design documents from `/specs/004-v1-level-menu-main/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D11, §E), data-model.md, contracts/, quickstart.md (todos aprovados, commit `fe4975f`)

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.1). Nenhuma task pode introduzir
abstração, refatoração, otimização, teste unitário ou infraestrutura. Se algo assim parecer
necessário, vira entrada em `docs/v2-backlog.md`, não task. **Nenhuma correção de bug** está
prevista: defeito objetivo do upstream → PARAR e reportar (a cláusula exige spec antes de qualquer
commit); `docs/upstream-bugs.md` fica com 2 entradas. **Uma mudança de configuração** do crate,
decidida pelo usuário: a feature `experimental-threads` do gdext, na US3 (research D1).

**Tests**: não há testes automatizados nesta fase. A validação de cada story é o ciclo do
quickstart (build → import headless → cena headless → verificações mecânicas → contrato →
validação visual do usuário).

**Organization**: uma phase por user story, na ordem obrigatória US1 → US2 → US3 → US4 (spec
FR-028; `docs/port-order.md` itens 11 → 14). As stories **não** são paralelizáveis entre si: cada
uma termina com um commit próprio na `main` e a seguinte começa da árvore limpa; a US4 (`Main`)
conecta por nome os sinais que as US2/US3 registram; `main.gd` original continua fazendo isso até
o port 4, então o jogo fica jogável após cada commit.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência de task incompleta) — raro
  neste marco, porque build depende do módulo, cena depende do build, validação depende da cena.
- **[Story]**: US1..US4 (spec.md)
- Caminhos relativos à raiz do repositório (`oxide-godot/` = projeto Godot;
  `oxide_godot_core/oxide_godot_lib/src/` = crate Rust).

## Path Conventions

```
oxide_godot_core/Cargo.toml                        US3: godot = { version = "0.5.5", features = ["experimental-threads"] } (única mudança)
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` de cada módulo (só isso)
oxide_godot_core/oxide_godot_lib/src/<módulo>.rs   uma classe por script (research.md §"Mapa por script")
oxide_godot_core/oxide_godot_lib/src/player.rs, red_robot.rs   Marcos B/C — só visibilidade muda (US2)
oxide-godot/level/forklift/flying_forklift.tscn (+ .gd/.uid)   US1
oxide-godot/level/level.tscn (+ .gd/.uid)                      US2
oxide-godot/menu/menu.tscn (+ .gd/.uid)                        US3
oxide-godot/main/main.tscn (+ .gd/.uid)                        US4
oxide-godot/menu/settings.gd (+ .uid)              NUNCA muda (Marco E)
CLAUDE.md                                          US3: linha da feature + catálogo do erro intermitente de main.tscn headless (edição operacional)
docs/v2-backlog.md                                 itens 19–24
docs/upstream-bugs.md                              NÃO muda (2 entradas)
../oxide_godot_origins/                            referência intocada do GDScript original
```

Decisões fechadas (valem para todas as stories — instrução, não opção; detalhes em research.md):

- **Nomes**: `FlyingForklift` (base **`CharacterBody3D`** — tipo do node, Princípio II v1.3.1),
  `Level` (`Node3D`), `Menu` (`Node`), `Main` (`Node`, arquivo `src/main_scene.rs`, `mod main_scene;`).
  Reconferir colisões com o grep do `CLAUDE.md` antes de cada port; colisão → parar.
- **Strings**: comparar com `GString::from("metal")` / `GString::from("headless")` (D3 — `"x".into()` é ambíguo).
- **Settings** (exceção do Princípio II, 3 formas): `self.base().get_node_as::<Node>("/root/Settings")` →
  `.get("config_file").to::<Gd<ConfigFile>>()`; `.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()])`;
  `.call("save_settings", &[])`.
- **US2**: task própria de visibilidade ANTES do build — `pub(crate) fn set_player_id` (`player.rs`)
  e `pub(crate) fn exploded();` (`red_robot.rs`), 2 pares −/+ conferidos por `git diff --stat`,
  citados na mensagem do commit (D2). `add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)`
  e `del_player(&mut self, id: i32)` **privados**, conectados por closure tipada
  (`|this, id: i64| this.add_player(id as i32, None)`); `spawned_nodes.add_child(&player)` **sem**
  `force_readable_name` (quirk `level.gd:121`); `spawn_robot` tipado `Gd<EnemyRobot>` com
  `add_child_ex(&robot).force_readable_name(true).done()`; `lightmap_gi` mantido `Some` após
  `queue_free` (**sem** `take()`); GI por constantes inteiras locais; `EnvironmentSdfgiRayCount::COUNT_96/COUNT_32`,
  `VoxelGiQuality::HIGH/LOW`; `LightmapGi::new_alloc()` + `set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"))`;
  `#[signal] fn quit();` no bloco principal (D5–D6).
- **US3**: task própria para `Cargo.toml` ANTES do build (regenera bindings — anotar o novo hash) e
  task própria para o `CLAUDE.md`; ambos citados na mensagem do commit. `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);`;
  `#[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>`; 85 `OnReady`
  exatamente como a tabela de `contracts/menu.md`; `_make_button_group` privado, 15 chamadas
  explícitas; `load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done()`,
  `load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done()` (`progress: VarArray`),
  `load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>()`; enums do engine por `.ord() as i64`
  (D9); `godot::global::is_equal_approx`; `call_deferred("_on_host_pressed", &[])` em headless;
  9 handlers `#[func]` (10 conexões) (D7–D9).
- **US4**: `get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false)`;
  `Engine::singleton().set_max_fps(60)` em headless; `window::Mode::from_ord(x as i32)`;
  `go_to_main_menu`/`replace_main_scene`/`change_scene_to_packed` **todos `#[func]`**;
  `call_deferred("change_scene_to_packed", &[resource.to_variant()])` por nome; `has_signal` +
  `connect(nome, &Callable::from_object_method(&self.to_gd(), "…"))` — duck typing do original,
  **não** trocar por `try_cast`; `ResourceLoader::singleton().load("res://menu/menu.tscn")` (fiel
  ao `ResourceLoader.load`); `remove_child` antes de `queue_free`, na ordem (D10).
- Um commit por script na `main`, mensagem no formato do quickstart §8; fix após checkpoint =
  commit `Fix port …`. **Antes de qualquer commit que não seja o do port, conferir que não há
  deleções de `.gd` em staging** (lição do Marco C).

---

## Phase 1: Setup

**Purpose**: registrar a baseline contra a qual cada port é comparado — inclusive a regra do erro
intermitente de `main.tscn` headless. Nada de infraestrutura (Princípio I).

- [x] T001 Confirmar pré-condições: nenhum editor Godot aberto (`pgrep -a godot` vazio — se houver, avisar o usuário e aguardar, nunca matar); `git status --short` vazio na `main`; anotar `git rev-parse --short HEAD` (esperado `fe4975f` ou posterior sem commits "Port …"). O commit-base `a866428` usado por `quickstart.md`/T054 continua válido; `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 5; `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = "0.5.5"` (sem feature ainda)
- [x] T002 Registrar baseline do build: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → esperado `0`; anotar em `specs/004-v1-level-menu-main/quickstart.md` §"Baseline" se diferir
- [x] T003 Registrar baseline do import: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirmar `grep -n 'Initialize godot-rust' /tmp/import.log` (linha 1) e `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` vazio (ou só os 3 erros do upstream do `CLAUDE.md`)
- [x] T004 [P] Confirmar os diretórios de bindings: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out` → esperado dois (`4eba5d49e15a0d7e` com a feature, do build de teste do plan; `aea5c50e7fda9d57` sem); anotar. Conferir colisão de nomes com o grep do `CLAUDE.md`: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | awk '{print $NF}' | sort -u | grep -x 'FlyingForklift\|Level\|Menu\|Main'` → vazio; `ls $(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1)/classes/ | grep -x 'flying_forklift.rs\|level.rs\|menu.rs\|main.rs'` → vazio
- [x] T005 Medir a baseline das 4 cenas (quickstart §"Baseline"): `cd oxide-godot && for s in level/forklift/flying_forklift.tscn level/level.tscn menu/menu.tscn main/main.tscn; do timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/base_$(basename $s .tscn).log 2>&1; done`; esperado exit 124 em todas, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class'` vazio, WARNINGs = 1/2/1/2 (HDR; Physics interpolation nas cenas com Player). **`main.tscn`**: se aparecerem exatamente as 5 linhas do renderizador dummy (`Initializing already initialized RID`, `Parameter "mem" is null.`, 3× `Parameter "m" is null.`), executar de novo — somem = baseline (erro intermitente do engine, research §E.2); qualquer outra linha `ERROR` → parar e reportar. Confirmar `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2 e `grep -c '^| [0-9]' docs/v2-backlog.md` = 18

**Checkpoint**: baseline conhecida — 0 warnings, extensão carrega, 0 `ERROR` no import e nas 4 cenas (fora o intermitente de `main.tscn`).

---

## Phase 2: Foundational

**Não existe neste marco.** Os quatro ports são estritamente sequenciais e a única edição
compartilhada é a linha `mod <módulo>;` em `lib.rs`, feita dentro de cada story. As aberturas de
visibilidade `pub(crate)` (`Player::set_player_id`, `EnemyRobot::exploded`) **pertencem à US2**;
a feature `experimental-threads` **pertence à US3** (é o menu quem a exige). Nenhum módulo comum,
helper, trait ou constante compartilhada pode ser criado (Princípio I).

---

## Phase 3: User Story 1 — Empilhadeira voadora portada (Priority: P1) 🎯 MVP

**Goal**: `level/forklift/flying_forklift.gd` (21 l., `extends Node3D`) → `FlyingForklift: CharacterBody3D`
(tipo do node raiz, `flying_forklift.tscn:36` — regra do Princípio II v1.3.1, declarada na spec
US1). Sombra do farol conforme `shadow_mapping`; sorteio do modelo.

**Independent Test**: `flying_forklift.tscn` e `level.tscn` headless sem `ERROR`; no jogo,
empilhadeiras com modelos variados e farol sem sombra quando `shadow_mapping` está desligado.

- [x] T006 [US1] Criar `oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs` traduzindo linha a linha `oxide-godot/level/forklift/flying_forklift.gd`: `use godot::classes::{CharacterBody3D, ConfigFile, ICharacterBody3D, Node, Node3D, SpotLight3D}; use godot::global::{randf, randomize}; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct FlyingForklift { base: Base<CharacterBody3D>, #[init(node = "SpotLight3D")] spot_light: OnReady<Gd<SpotLight3D>> }`; `#[godot_api] impl ICharacterBody3D for FlyingForklift { fn ready(&mut self) { let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>(); if !config_file.get_value("rendering", "shadow_mapping").to::<bool>() { self.spot_light.set_shadow(false); } // Randomize the forklift model. // We have 3 models, may as well use them. randomize(); let children = self.base().get_child(0).unwrap().get_children(); let child_count = children.len(); let which_enabled = (randf() * child_count as f64).floor() as usize; for (i, child) in children.iter_shared().enumerate() { child.cast::<Node3D>().set_visible(i == which_enabled); } } }`. Manter o comentário `// TODO: We can maybe implement func hit():` do original. Nenhum `#[func]` (research D4)
- [x] T007 [US1] Adicionar `mod flying_forklift;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod red_robot;`; nada mais muda em lib.rs)
- [x] T008 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0. Em caso de erro, consultar research.md D3–D4 e as bindings antes de improvisar; não alterar `Cargo.toml` nesta story
- [x] T009 [US1] Editar `oxide-godot/level/forklift/flying_forklift.tscn` como texto (reconferir com `grep -n 'name="FlyingForklift" type=\|^script = ExtResource("3")\|flying_forklift.gd' oxide-godot/level/forklift/flying_forklift.tscn` → esperado l.36, l.37, l.5): na raiz trocar `type="CharacterBody3D"` por `type="FlyingForklift"`; remover `script = ExtResource("3")`; remover `[ext_resource type="Script" uid="uid://dcqnfagy55nrx" path="res://level/forklift/flying_forklift.gd" id="3"]`. **MANTER** `FlyingForkliftModel2` (l.39) e os `visible = false` dos modelos, `Collider` (l.47), `SpotLight3D` (l.114) e todos os outros `ext_resource`. Diff esperado: `4 +---` (plan.md "Edição das cenas", port 1)
- [x] T010 [US1] Apagar `oxide-godot/level/forklift/flying_forklift.gd` e `oxide-godot/level/forklift/flying_forklift.gd.uid` (`git rm`); verificar `grep -rn 'uid://dcqnfagy55nrx' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'flying_forklift.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [x] T011 [US1] Validação headless (quickstart §2–3; sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . level/forklift/flying_forklift.tscn 2>&1 | tee /tmp/run.log` e depois `level/level.tscn` → exit 124; grep de regressão (incluindo `shadows a native class`) vazio em ambas (só os WARNINGs da baseline). A empilhadeira isolada e as instâncias do level rodam `ready` (Settings dinâmico, `get_child(0).get_children()`, sorteio)
- [x] T012 [US1] Verificações mecânicas e contrato (`contracts/flying-forklift.md`): `grep -n 'name="FlyingForklift" type=' oxide-godot/level/forklift/flying_forklift.tscn` = `type="FlyingForklift"`; `grep -c 'ExtResource("3")' …` = 0; `grep -n 'name="Collider"\|name="SpotLight3D"\|name="FlyingForkliftModel2"' …` = 3 linhas intocadas; `grep -n 'flying_forklift.tscn' oxide-godot/level/level.tscn` = l.8; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `flying_forklift.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = 12 arquivos; `grep -c '#\[func\]' oxide_godot_core/oxide_godot_lib/src/flying_forklift.rs` = 0
- [x] T013 [US1] Registrar em `docs/v2-backlog.md` a linha nº 19 (research.md §"Backlog v2 candidato"): `level/forklift/flying_forklift.gd` (port 1) — não chamar `randomize()` por instância (o `Main` já re-semeia no boot); motivação: re-semear o gerador global a cada empilhadeira é redundante. Não duplicar os itens 1–18
- [x] T014 [US1] Commit único do port 1 na `main` (autor the repository author; conferir `git status` antes: só os arquivos desta story em staging), incluindo `src/flying_forklift.rs`, `src/lib.rs`, `oxide-godot/level/forklift/flying_forklift.tscn`, as deleções de `flying_forklift.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port flying_forklift.gd → FlyingForklift (CharacterBody3D); flying_forklift.tscn: node FlyingForklift type="CharacterBody3D"→"FlyingForklift"` + corpo: `- base CharacterBody3D = tipo do node na cena (o script declarava extends Node3D — constituição v1.3.1, Princípio II)`; notas (Settings dinâmico; `randomize()` + `floor(randf × n)` preservados); `- backlog v2: item 19`. Anotar o hash
- [x] T015 [US1] **Checkpoint do usuário (validação visual, plan.md port 1)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: entrar no level algumas vezes (Play → ESC → Play) — as empilhadeiras voadoras aparecem com modelos/cores variados entre si e entre reentradas; com `Shadow mapping` desligado em Settings, o farol não projeta sombra (ligado, projeta); jogador e robôs continuam colidindo com elas. No editor, `flying_forklift.tscn`: raiz `FlyingForklift` (tipo derivado de `CharacterBody3D`), sem script, `Collider` intacto. Divergência → commit `Fix port flying_forklift.gd …`. Só seguir para a US2 com o OK explícito

**Checkpoint**: 4 `.gd` restantes; `flying_forklift.tscn` com `FlyingForklift`; jogo jogável.

---

## Phase 4: User Story 2 — Level portado (Priority: P2)

**Goal**: `level/level.gd` (127 l.) → `Level: Node3D`; sinal `quit`; Settings dinâmico + GI
(SDFGI/VoxelGI/lightmap); spawn tipado de `EnemyRobot` (respawn 15 s via `exploded`) e `Player`
(`set_player_id`); `add_player`/`del_player` conectados aos sinais do `MultiplayerAPI`; ESC emite `quit`.

**Independent Test**: `level.tscn` headless sem `ERROR` (4 robôs + jogador 1 spawnados, sinais
conectados) e `main.tscn` (o `main.gd` original conecta `quit` por `has_signal` no `Level` Rust);
no jogo, GI conforme a opção, respawn, ESC volta ao menu.

- [x] T016 [US2] Alterar **só a visibilidade** em `oxide_godot_core/oxide_godot_lib/src/player.rs` (`fn set_player_id(&mut self, value: i32)` → `pub(crate) fn set_player_id(…)`, dentro do `#[godot_api] impl Player`, `#[func]` fica) e em `oxide_godot_core/oxide_godot_lib/src/red_robot.rs` (`#[signal] fn exploded();` → `#[signal] pub(crate) fn exploded();`). Conferir: `git diff --stat` = `player.rs | 2 +-` e `red_robot.rs | 2 +-` (2 pares −/+, nada mais). O acessor `robot.signals().exploded()` herda a visibilidade do `fn` do sinal — sem isso não compila (research D2)
- [x] T017 [US2] Criar `oxide_godot_core/oxide_godot_lib/src/level.rs` — parte 1 (declarações), traduzindo `level.gd:1-14`: `use godot::classes::input::MouseMode; use godot::classes::rendering_server::{EnvironmentSdfgiRayCount, VoxelGiQuality}; use godot::classes::{ConfigFile, INode3D, Input, InputEvent, LightmapGi, LightmapGiData, Marker3D, Node, Node3D, PackedScene, RenderingServer, WorldEnvironment}; use godot::global::{randi, randomize}; use godot::prelude::*; use crate::player::Player; use crate::red_robot::EnemyRobot;`; consts `i64`: `SDFGI = 0`, `VOXEL_GI = 1` (valores de `Settings.GIType`), `GI_DISABLED = 0`, `GI_LOW = 1`, `GI_HIGH = 2` (`Settings.GIQuality`) — `LIGHTMAP_GI` é o ramo `else`; `#[derive(GodotClass)] #[class(init, base=Node3D)] pub struct Level { base: Base<Node3D>, lightmap_gi: Option<Gd<LightmapGi>>, #[init(node = "WorldEnvironment")] world_environment: OnReady<Gd<WorldEnvironment>>, #[init(node = "RobotSpawnpoints")] robot_spawn_points: OnReady<Gd<Node3D>>, #[init(node = "PlayerSpawnpoints")] player_spawn_points: OnReady<Gd<Node3D>>, #[init(node = "SpawnedNodes")] spawned_nodes: OnReady<Gd<Node3D>> }`. `RedRobot`/`PlayerScene` (preload) não viram campos: `load` no ponto de uso (research D5)
- [x] T018 [US2] `level.rs` — parte 2 (virtuais e privados), traduzindo `level.gd:17-42,45-93,96-121,124-127`: `#[godot_api] impl INode3D for Level` com `fn ready(&mut self)`: `let window = self.base().get_window().unwrap(); let environment = self.world_environment.get_environment().unwrap(); let mut settings = self.base().get_node_as::<Node>("/root/Settings"); settings.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); let config_file = settings.get("config_file").to::<Gd<ConfigFile>>(); let gi_type = config_file.get_value("rendering", "gi_type").to::<i64>(); if gi_type == SDFGI { self.setup_sdfgi(); } else if gi_type == VOXEL_GI { self.setup_voxelgi(); } else { self.setup_lightmapgi(); }`; `let multiplayer = self.base().get_multiplayer().unwrap(); if multiplayer.is_server() { for child in self.robot_spawn_points.get_children().iter_shared() { self.spawn_robot(child.cast::<Node3D>()); } randomize(); let mut spawn_points = self.player_spawn_points.get_children(); spawn_points.shuffle(); let first = spawn_points.pop_front().map(|n| n.cast::<Marker3D>()); self.add_player(1, first); for id in multiplayer.get_peers().as_slice() { let next = spawn_points.pop_front().map(|n| n.cast::<Marker3D>()); self.add_player(*id, next); } multiplayer.signals().peer_connected().connect_other(&*self, |this: &mut Level, id: i64| this.add_player(id as i32, None)); multiplayer.signals().peer_disconnected().connect_other(&*self, |this: &mut Level, id: i64| this.del_player(id as i32)); }` (manter os comentários do original); `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("quit") { Input::singleton().set_mouse_mode(MouseMode::VISIBLE); self.signals().quit().emit(); } }`. Bloco `impl Level` **sem** `#[godot_api]`: `fn gi_quality(&self) -> i64` NÃO — ler `gi_quality` inline em cada `setup_*` (`self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("rendering", "gi_quality").to::<i64>()`, como o original repete `Settings.config_file.get_value(...)`); `fn setup_sdfgi(&mut self)` (`self.world_environment.get_environment().unwrap().set_sdfgi_enabled(true); self.base().get_node_as::<Node3D>("VoxelGI").hide(); self.base().get_node_as::<Node3D>("ReflectionProbes").hide(); if let Some(lightmap_gi) = &mut self.lightmap_gi { lightmap_gi.queue_free(); }` (**sem** `take()`) `+ gi_quality: == GI_HIGH → RenderingServer::singleton().environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_96); == GI_LOW → COUNT_32; else → set_sdfgi_enabled(false)`); `fn setup_voxelgi` (sdfgi false; `VoxelGI` show; `ReflectionProbes` hide; queue_free idem; `GI_HIGH → RenderingServer::singleton().voxel_gi_set_quality(VoxelGiQuality::HIGH)`; `GI_LOW → LOW`; else `VoxelGI` hide); `fn setup_lightmapgi` (sdfgi false; `VoxelGI` hide; `ReflectionProbes` show; `if self.lightmap_gi.is_none() { let mut new_gi = LightmapGi::new_alloc(); new_gi.set_light_data(&load::<LightmapGiData>("res://level/level.lmbake")); new_gi.set_name("LightmapGI"); self.lightmap_gi = Some(new_gi.clone()); self.base_mut().add_child(&new_gi); }`; `if gi_quality == GI_DISABLED { self.lightmap_gi.as_mut().unwrap().hide(); ReflectionProbes hide }`); `fn spawn_robot(&mut self, spawn_point: Gd<Node3D>)` (`let mut robot: Gd<EnemyRobot> = load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>(); robot.set_transform(spawn_point.get_transform()); robot.signals().exploded().connect_other(&*self, move |this: &mut Level| this._respawn_robot(spawn_point.clone())); self.spawned_nodes.add_child_ex(&robot).force_readable_name(true).done();`); `fn _respawn_robot(&mut self, spawn_point: Gd<Node3D>)` (`self.base().get_tree().create_timer(15.0).signals().timeout().connect_other(&*self, move |this: &mut Level| this.spawn_robot(spawn_point.clone()));`); `fn del_player(&mut self, id: i32)` (`let name = id.to_string(); if !self.spawned_nodes.has_node(&name) { return; } self.spawned_nodes.get_node_as::<Node>(&name).queue_free();`); `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)` (`let spawn_point = spawn_point.unwrap_or_else(|| { let count = self.player_spawn_points.get_child_count(); self.player_spawn_points.get_child((randi() % count as i64) as i32).unwrap().cast::<Marker3D>() }); let mut player: Gd<Player> = load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>(); player.set_name(&id.to_string()); player.bind_mut().set_player_id(id); player.set_transform(spawn_point.get_transform()); self.spawned_nodes.add_child(&player);` — **sem** `force_readable_name`) (research D5–D6)
- [x] T019 [US2] `level.rs` — parte 3 (bloco Godot), traduzindo `level.gd:4`: `#[godot_api] impl Level { #[signal] fn quit(); }` — **único** bloco `#[godot_api] impl Level`; nenhum `#[func]` (`add_player`/`del_player`/`spawn_robot`/`_respawn_robot`/`setup_*` são privados — nenhum script os chama por nome; a conexão aos sinais do `MultiplayerAPI` é por closure tipada, T018) (research D5–D6, contracts/level.md)
- [x] T020 [US2] Adicionar `mod level;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod flying_forklift;`)
- [x] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. Em caso de erro, research.md D2, D5–D6 e as bindings antes de improvisar
- [x] T022 [US2] Editar `oxide-godot/level/level.tscn` como texto (reconferir com `grep -n 'name="Level" type=\|^script = ExtResource("1")\|level.gd' oxide-godot/level/level.tscn` → esperado l.40, l.41, l.3): na raiz trocar `type="Node3D"` por `type="Level"`; remover `script = ExtResource("1")`; remover `[ext_resource type="Script" uid="uid://ccxbls23ev7u3" path="res://level/level.gd" id="1"]`. **MANTER** `SpawnedNodes`, `RobotSpawnpoints` (4 filhos), `PlayerSpawnpoints` (4 `Marker3D`), `MultiplayerSpawner` (`_spawnable_scenes`, `spawn_path = NodePath("../SpawnedNodes")`), `WorldEnvironment`, `VoxelGI`, `ReflectionProbes`, as instâncias de `flying_forklift.tscn` e todos os outros `ext_resource`. Diff esperado: `4 +---`
- [x] T023 [US2] Apagar `oxide-godot/level/level.gd` e `oxide-godot/level/level.gd.uid` (`git rm`); verificar `grep -rn 'uid://ccxbls23ev7u3' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'level.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [x] T024 [US2] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . level/level.tscn 2>&1 | tee /tmp/run.log` → exit 124, grep de regressão (incl. `shadows a native class`) vazio — level isolado: `apply_graphics_settings` dinâmico, GI, 4 `EnemyRobot` e o `Player` 1 spawnados (tipados), conexões `peer_*`; `main/main.tscn` → exit 124, grep vazio (regra do erro intermitente da baseline: as 5 linhas do renderizador dummy não contam se sumirem na 2ª execução; 3 seguidas = regressão) — `main.gd` original instancia o `Level` Rust via `replace_main_scene` e conecta `quit` por `has_signal`
- [x] T025 [US2] Verificações mecânicas e contrato (`contracts/level.md`): `grep -c 'type="Level"' oxide-godot/level/level.tscn` = 1; `grep -c 'ExtResource("1")' …` = 0; `grep -n 'spawn_path\|_spawnable_scenes' …` = 2 linhas intocadas; `grep -n 'name="RobotSpawnpoints"\|name="PlayerSpawnpoints"\|name="SpawnedNodes"\|name="WorldEnvironment"\|name="VoxelGI"\|name="ReflectionProbes"' …` = 6; `grep -n 'has_signal(&"quit")\|node.quit.connect' oxide-godot/main/main.gd` = l.30-31 (o `main.gd` segue intacto); `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `level.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'` = exatamente 4 linhas (`set_player_id`, `exploded`); `ls src/` = 13 arquivos; `grep -n '#\[signal\]\|#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/level.rs | grep 'fn '` = só `quit`
- [x] T026 [US2] Registrar em `docs/v2-backlog.md` as linhas nº 20 e 21 (research.md §"Backlog v2 candidato"): 20 — `level/level.gd` (port 2): uniformizar `add_child(player, true)` como no robô (ou `spawn_robot` sem nome legível) — os dois spawns usam formas diferentes; 21 — expor `add_player`/`del_player` com parâmetro default via dois `#[func]` (ou remover o default) — gdext não tem parâmetro default; a v1 conecta por closure. Não duplicar os itens 1–19
- [x] T027 [US2] Commit único do port 2 na `main` (autor the repository author; conferir `git status` antes), incluindo `src/level.rs`, `src/lib.rs`, `src/player.rs`, `src/red_robot.rs`, `oxide-godot/level/level.tscn`, as deleções de `level.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port level.gd → Level (Node3D); level.tscn: node Level type="Node3D"→"Level"` + corpo: notas (sinal `quit` conectado por nome pelo `main.gd`; `EnemyRobot`/`Player` tipados — `exploded` via `signals()`, `set_player_id`; Settings dinâmico — `apply_graphics_settings`, `config_file`; GI por inteiros do `settings.gd`; `add_player`/`del_player` privados conectados a `peer_connected`/`peer_disconnected` por closure tipada; quirks: `add_child(player)` sem nome legível, `randomize()`, `lightmap_gi` mantido após `queue_free`); `- player.rs / red_robot.rs: só visibilidade pub(crate) em set_player_id / exploded (acesso tipado, FR-025); nenhuma lógica movida`; `- backlog v2: itens 20, 21`. Anotar o hash
- [x] T028 [US2] **Checkpoint do usuário (validação visual, plan.md port 2)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: menu → Play → level: 4 robôs nascem nos pontos, o jogador nasce num ponto aleatório (som de pouso do quirk), matar um robô → outro nasce no mesmo ponto 15 s depois; **ESC** libera o mouse e volta ao menu; Settings → trocar `GI type` (SDFGI / VoxelGI / LightmapGI) e `GI quality` (Disabled / Low / High), Apply, Play de novo → iluminação/`ReflectionProbes` conforme a opção, sem erro no console. No editor, `level.tscn`: raiz `Level`, sem script, `MultiplayerSpawner` intacto. Divergência → commit `Fix port level.gd …`. Só seguir para a US3 com o OK explícito

**Checkpoint**: 3 `.gd` restantes; `level.tscn` com `Level`; jogo jogável.

---

## Phase 5: User Story 3 — Menu portado (Priority: P3)

**Goal**: `menu/menu.gd` (460 l.) → `Menu: Node`; sinal `replace_main_scene(PackedScene)`; 85
`OnReady`; 15 `ButtonGroup`s; loading em thread com barra (**feature `experimental-threads`**);
9 handlers (10 conexões) com os mapeamentos exatos de opções ↔ inteiros do `config_file`;
host/connect ENet; Settings dinâmico.

**Independent Test**: `menu.tscn` headless sem `ERROR` (hospeda sozinho → loading → `DoneTimer` →
emite `replace_main_scene`) e `main.tscn` (fluxo completo com `Menu` Rust e `main.gd`); no jogo,
cada botão e cada linha de Settings se comporta como o original, com persistência em `user://settings.ini`.

- [x] T029 [US3] Editar `oxide_godot_core/Cargo.toml` l.7: `godot = "0.5.5"` → `godot = { version = "0.5.5", features = ["experimental-threads"] }` (única mudança; nenhuma outra feature, versão inalterada — research D1; `CLAUDE.md` §Toolchain proíbe mudar a versão, não uma feature). Conferir `git diff --stat oxide_godot_core/Cargo.toml` = `1 +-`
- [x] T030 [US3] Build de regeneração: `cd oxide_godot_core && cargo build 2>&1 | tail -3` → `Finished` (regenera as bindings do `godot-core` com a feature; pode levar minutos na primeira vez — no ambiente atual já existe o cache `4eba5d49e15a0d7e` do build de teste do plan); `grep -c '^warning'` = 0; anotar `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` (esperado `…/godot-core-4eba5d49e15a0d7e/out`) e confirmar `grep -c 'pub fn load_threaded_request\b' $(ls -dt …/godot-core-*/out | head -1)/classes/resource_loader.rs` = 1
- [x] T031 [US3] Criar `oxide_godot_core/oxide_godot_lib/src/menu.rs` — parte 1 (declarações), traduzindo `menu.gd:1-104`: `use godot::classes::display_server::VSyncMode; use godot::classes::rendering_server::{EnvironmentSsaoQuality, EnvironmentSsilQuality}; use godot::classes::resource_loader::ThreadLoadStatus; use godot::classes::viewport::{Msaa, Scaling3DMode, ScreenSpaceAa}; use godot::classes::window::Mode as WindowMode; use godot::classes::{BaseButton, Button, ButtonGroup, ConfigFile, Control, DisplayServer, ENetMultiplayerPeer, HBoxContainer, INode, LineEdit, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene, ProgressBar, RenderingServer, ResourceLoader, SpinBox, Timer, VBoxContainer, WorldEnvironment}; use godot::global::is_equal_approx; use godot::prelude::*;`; `const LEVEL_PATH: &str = "res://level/level.tscn";`; `#[derive(GodotClass)] #[class(init, base=Node)] pub struct Menu { base: Base<Node>, #[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>, #[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"))] metalfx_supported: bool,` + os **85** campos `#[init(node = "<caminho>")] <nome>: OnReady<Gd<<Tipo>>>` **exatamente** como a tabela "Referências de cena" de `contracts/menu.md` (nome, tipo e caminho completo a partir do `Menu` — ex.: `#[init(node = "UI/Settings/MaxFPS/30")] max_fps_30: OnReady<Gd<Button>>`), na mesma ordem do original `}`. Conferir ao final: `grep -c '#\[init(node = ' menu.rs` = 85 (research D7)
- [x] T032 [US3] `menu.rs` — parte 2 (virtuais e privados), traduzindo `menu.gd:107-160`: `#[godot_api] impl INode for Menu` com `fn ready(&mut self)`: `// Apply relevant settings directly.` + `let window = self.base().get_window().unwrap(); let environment = self.world_environment.get_environment().unwrap(); self.base().get_node_as::<Node>("/root/Settings").call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); if DisplayServer::singleton().get_name() == GString::from("headless") { self.base_mut().call_deferred("_on_host_pressed", &[]); } self.play_button.grab_focus(); if !self.metalfx_supported { self.scale_filter_metalfx_spatial.hide(); self.scale_filter_metalfx_temporal.hide(); }` + **15 chamadas explícitas** `self._make_button_group(self.display_mode_menu.clone().upcast());` … na ordem do original (`display_mode_menu, vsync_menu, max_fps_menu, resolution_scale_menu, scale_filter_menu, taa_menu, msaa_menu, screen_space_aa_menu, shadow_mapping_menu, gi_type_menu, gi_quality_menu, ssao_menu, ssil_menu, bloom_menu, volumetric_fog_menu`); `fn process(&mut self, _delta: f64)`: `if self.loading.is_visible() { let progress = VarArray::new(); let status: ThreadLoadStatus = ResourceLoader::singleton().load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done(); if status == ThreadLoadStatus::IN_PROGRESS { self.loading_progress.set_value(progress.at(0).to::<f64>() * 100.0); } else if status == ThreadLoadStatus::LOADED { self.loading_progress.set_value(100.0); self.base_mut().set_process(false); self.loading_done_timer.start(); } else { godot_print!("Error while loading level: {}", status.ord()); self.main.show(); self.loading.hide(); } }`. Bloco `impl Menu` **sem** `#[godot_api]`: `fn _make_button_group(&mut self, common_parent: Gd<Node>) { let group = ButtonGroup::new_gd(); for btn in common_parent.get_children().iter_shared() { if let Ok(mut btn) = btn.try_cast::<BaseButton>() { btn.set_button_group(&group); } } }` (research D3, D8)
- [x] T033 [US3] `menu.rs` — parte 3 (bloco Godot), traduzindo `menu.gd:4,163-460`: `#[godot_api] impl Menu` **único** com `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);` e os 9 handlers `#[func]`: `_on_loading_done_timer_timeout` (`let peer = self.peer.clone(); self.base().get_multiplayer().unwrap().set_multiplayer_peer(&peer); let scene = ResourceLoader::singleton().load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>(); self.signals().replace_main_scene().emit(&scene);`); `_on_play_pressed` (`self.main.hide(); self.loading.show(); ResourceLoader::singleton().load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done();`); `_on_settings_pressed` — tradução **linha a linha** de `menu.gd:175-311`: `self.main.hide(); self.settings_menu.show(); self.settings_action_cancel.grab_focus(); let config_file = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>();` e então cada cadeia `if/elif/else` com `config_file.get_value(s, k).to::<i64>()` comparado a `WindowMode::WINDOWED.ord() as i64` / `MAXIMIZED` / `FULLSCREEN`; `VSyncMode::DISABLED/ENABLED/ADAPTIVE` (else MAILBOX); `max_fps` 30/40/60/72/90/120/144 (else Unlimited); `is_equal_approx(get_value(..).to::<f64>(), 1.0 / 3.0)` / `1.0 / 2.0` / `1.0 / 1.7` / `1.0 / 1.5` / `1.0 / 1.3` (else Native); `Scaling3DMode::BILINEAR/FSR/FSR2/METALFX_SPATIAL/METALFX_TEMPORAL` por `.ord()` e, para o ramo Nearest, a constante local `const SCALING_3D_MODE_NEAREST: i64 = 5; // ausente da API prebuilt 4.6 do gdext 0.5.5; Godot 4.7.2 = 5` (leitura e gravação — `Viewport.SCALING_3D_MODE_NEAREST` foi adicionado no 4.7; mesma técnica dos inteiros de GIType) (else: `metalfx_supported` → metalfx_temporal senão fsr2); `gi_type` 2/1/0 → `gi_lightmapgi/gi_voxelgi/gi_sdfgi`; `gi_quality` 0/1/2 → `gi_disabled/gi_low/gi_high`; `taa` bool; `Msaa::DISABLED/MSAA_2X/MSAA_4X/MSAA_8X`; `ScreenSpaceAa::DISABLED/FXAA/SMAA`; `shadow_mapping` bool; `ssao_quality` −1 / `EnvironmentSsaoQuality::MEDIUM` / `HIGH`; `ssil_quality` −1 / `EnvironmentSsilQuality::MEDIUM` / `HIGH`; `bloom`, `volumetric_fog` bool — cada ramo `btn.set_pressed(true)`; `_on_quit_pressed` (`self.base().get_tree().quit();`); `_on_apply_pressed` — tradução linha a linha de `menu.gd:318-441`: `self.main.show(); self.play_button.grab_focus(); self.settings_menu.hide(); let mut settings = …; let mut config_file = settings.get("config_file").to::<Gd<ConfigFile>>();` e cada cadeia `if btn.is_pressed() { config_file.set_value(s, k, &v.to_variant()) }` com `v`: `WindowMode::WINDOWED.ord() as i64` / `FULLSCREEN` / `EXCLUSIVE_FULLSCREEN`; vsync 4 `.ord() as i64`; fps `30_i64…144_i64`, Unlimited `0_i64`; escalas `1.0 / 3.0` … `1.0_f64`; filtros 6 `.ord() as i64`; gi_type `2_i64/1_i64/0_i64`; gi_quality `1/2/0` (ordem `low, high, disabled` como o original); `taa` `self.taa_enabled.is_pressed()`; msaa 4; screen_space_aa 3; `shadow_mapping` bool; ssao `-1_i64`/MEDIUM/HIGH; ssil idem; `bloom`/`volumetric_fog` bool; então `// Apply relevant settings directly.` + `settings.call("apply_graphics_settings", &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]); settings.call("save_settings", &[]);`; `_on_cancel_pressed` (`main.show(); play_button.grab_focus(); settings_menu.hide(); online.hide();`); `_on_play_online_pressed` (`online.show(); main.hide();`); `_on_host_pressed` (`let mut peer = ENetMultiplayerPeer::new_gd(); peer.create_server(self.online_port.get_value() as i32); self.peer = peer.upcast(); self._on_play_pressed(); self.online.hide();`); `_on_connect_pressed` (`create_client(&self.online_address.get_text(), self.online_port.get_value() as i32)`, resto idem). Nenhum outro `#[func]`/`#[signal]` (research D7, D9; data-model.md tabela de opções)
- [x] T034 [US3] Adicionar `mod menu;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod level;`)
- [x] T035 [US3] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. Em caso de erro, research.md D1, D3, D7–D9 e as bindings **com a feature** (`ls -dt … | head -1`) antes de improvisar
- [x] T036 [US3] Editar `oxide-godot/menu/menu.tscn` como texto (reconferir com `grep -n 'name="Menu" type=\|^script = ExtResource("1")\|menu.gd' oxide-godot/menu/menu.tscn` → esperado l.103, l.104, l.3): na raiz trocar `type="Node"` por `type="Menu"`; remover `script = ExtResource("1")`; remover `[ext_resource type="Script" uid="uid://4pwshyfo5i0d" path="res://menu/menu.gd" id="1"]`. **MANTER** toda a árvore `UI/…` (85 caminhos), `WorldEnvironment`, `SpotLight3D`, `DoneTimer` (`wait_time = 0.5`, `one_shot = true`) e as **10** `[connection]` do fim (l.836–845 → 835–844). Diff esperado: `4 +---`
- [x] T037 [US3] Apagar `oxide-godot/menu/menu.gd` e `oxide-godot/menu/menu.gd.uid` (`git rm`); verificar `grep -rn 'uid://4pwshyfo5i0d' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'menu/menu.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio. **`menu/settings.gd` fica** (conferir que não foi tocado: `git status --short oxide-godot/menu/` mostra só `menu.gd`/`.uid` deletados e `menu.tscn` modificado)
- [x] T038 [US3] Editar `CLAUDE.md` (edição operacional): em §Toolchain, após a linha do crate `godot = "0.5.5"`, acrescentar: `- Feature `experimental-threads` do gdext habilitada desde o Marco D (Cargo.toml): sem ela o codegen omite `ResourceLoader::load_threaded_request/get_status/get` (godot-codegen `special_cases.rs:83-86`), que o menu usa para a barra de loading. Nenhuma outra feature.`; em §Ciclo de trabalho, no parágrafo dos erros pré-existentes, acrescentar: `Em `main.tscn` headless pode aparecer, de forma intermitente (corrida entre o carregamento do level em sub-thread e o renderizador dummy), o conjunto `Initializing already initialized RID` / `Parameter "mem" is null.` / 3× `Parameter "m" is null.` — não é regressão se sumir numa segunda execução (research 004 §E.2).` Conferir `git diff --stat CLAUDE.md` = só inserções
- [x] T039 [US3] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . menu/menu.tscn 2>&1 | tee /tmp/run.log` → exit 124, grep de regressão (incl. `shadows a native class`) vazio — em headless o `Menu` Rust aplica settings, agenda `_on_host_pressed`, cria o servidor ENet, requisita o loading em thread (feature), `process` acompanha o status, `DoneTimer` dispara e `replace_main_scene` é emitido (sem consumidor); `main/main.tscn` → exit 124, grep vazio (regra do erro intermitente) — `main.gd` original conecta `replace_main_scene` do `Menu` Rust por `has_signal`, recebe a `PackedScene`, instancia o `Level` Rust
- [x] T040 [US3] Verificações mecânicas e contrato (`contracts/menu.md`): `grep -c '^\[connection' oxide-godot/menu/menu.tscn` = 10; `grep -o 'method="[^"]*"' oxide-godot/menu/menu.tscn | sort -u | wc -l` = 9 e cada nome existe como `#[func]` em `menu.rs`; `grep -n 'replace_main_scene' oxide-godot/main/main.gd` = l.20, 32, 33 (intacto); `grep -c '#\[init(node = ' oxide_godot_core/oxide_godot_lib/src/menu.rs` = 85; `grep -c 'type="Menu"' oxide-godot/menu/menu.tscn` = 1; `grep -c 'ExtResource("1")' …` = 0; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `menu.gd` (**não** `settings.gd`); `git diff HEAD -- oxide_godot_core/Cargo.toml | grep '^[-+]godot'` = 1 par com `features = ["experimental-threads"]`; `git diff --stat HEAD -- CLAUDE.md` = só inserções; `ls src/` = 14 arquivos; `grep -n '#\[signal\]\|#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/menu.rs | grep 'fn ' | wc -l` = 10 (1 sinal + 9 handlers)
- [x] T041 [US3] Registrar em `docs/v2-backlog.md` as linhas nº 22 e 23 (research.md §"Backlog v2 candidato"): 22 — `menu/menu.gd` (port 3): tabela declarativa opção→valor em vez de ~30 cadeias `if/elif` em `_on_settings_pressed`/`_on_apply_pressed`; 23 — declarar `replace_main_scene` com o parâmetro no original (já feito no port) e conectar `quit`/`replace_main_scene` no `Main` por tipo em vez de `has_signal`. Não duplicar os itens 1–21
- [x] T042 [US3] Commit único do port 3 na `main` (autor the repository author; conferir `git status` antes), incluindo `src/menu.rs`, `src/lib.rs`, `oxide_godot_core/Cargo.toml`, `CLAUDE.md`, `oxide-godot/menu/menu.tscn`, as deleções de `menu.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port menu.gd → Menu (Node); menu.tscn: node Menu type="Node"→"Menu"` + corpo: notas (sinal `replace_main_scene(PackedScene)` — parâmetro declarado, o original emitia 1 arg num sinal sem parâmetros; 85 OnReady com caminhos completos; 15 ButtonGroups; loading em thread com barra; 9 handlers/10 conexões; enums do engine gravados/lidos pelos inteiros `.ord()`; Settings dinâmico — `apply_graphics_settings`, `save_settings`, `config_file`; host/connect ENet); `- Cargo.toml: feature experimental-threads do gdext (ResourceLoader::load_threaded_* não é gerado sem ela — godot-codegen special_cases.rs:83-86); versão 0.5.5 inalterada`; `- CLAUDE.md: linha da feature + catálogo do erro intermitente do renderizador dummy em main.tscn headless`; `- backlog v2: itens 22, 23`. Anotar o hash
- [x] T043 [US3] **Checkpoint do usuário (validação visual, plan.md port 3)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: menu completo — Play → barra de loading progride até 100 % e o level abre ~0,5 s depois; Settings → cada uma das 15 linhas mostra pressionado o valor salvo; alterar algumas opções (ex.: VSync, Max FPS, MSAA, GI type, Shadow mapping) e Apply → aplicam na hora e persistem em `user://settings.ini` (reabrir Settings mostra os novos valores; fechar e reabrir o jogo mantém); Cancel/Back não gravam; Play Online → Host inicia o level como servidor, Back volta; Quit fecha; botões MetalFX ocultos (Linux); F11 (`settings.gd`) continua funcionando; navegação por teclado com foco em Play/Cancel como no original. No editor, `menu.tscn`: raiz `Menu`, sem script, 10 conexões no painel de sinais. Divergência → commit `Fix port menu.gd …`. Só seguir para a US4 com o OK explícito

**Checkpoint**: 2 `.gd` restantes; `menu.tscn` com `Menu`; feature ativa; jogo jogável.

---

## Phase 6: User Story 4 — Main portado (Priority: P4)

**Goal**: `main/main.gd` (33 l.) → `Main: Node` (arquivo `main_scene.rs`); boot (relay off, 60 fps
em headless, modo da janela salvo), `go_to_main_menu`, `replace_main_scene` (deferred por nome),
`change_scene_to_packed` com `has_signal`/`connect` por nome — duck typing do original.

**Independent Test**: `main.tscn` headless (boot Rust → `Menu` → host → `Level`) e `menu.tscn`
sem `ERROR`; no jogo, boot → menu → Play → level → ESC → menu → Play de novo, sem duplicar cenas.

- [ ] T044 [US4] Criar `oxide_godot_core/oxide_godot_lib/src/main_scene.rs` traduzindo linha a linha `oxide-godot/main/main.gd`: `use godot::classes::window::Mode as WindowMode; use godot::classes::{ConfigFile, DisplayServer, Engine, INode, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene, ResourceLoader, SceneMultiplayer}; use godot::global::randomize; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=Node)] pub struct Main { base: Base<Node> }`; `#[godot_api] impl INode for Main { fn ready(&mut self) { self.base().get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false); if DisplayServer::singleton().get_name() == GString::from("headless") { Engine::singleton().set_max_fps(60); } randomize(); let display_mode = self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("video", "display_mode").to::<i64>(); self.base().get_window().unwrap().set_mode(WindowMode::from_ord(display_mode as i32)); self.go_to_main_menu(); } }`; `#[godot_api] impl Main` **único**: `#[func] fn go_to_main_menu(&mut self) { let menu = ResourceLoader::singleton().load("res://menu/menu.tscn").unwrap().cast::<PackedScene>(); let mut multiplayer = self.base().get_multiplayer().unwrap(); multiplayer.get_multiplayer_peer().unwrap().close(); multiplayer.set_multiplayer_peer(&OfflineMultiplayerPeer::new_gd().upcast::<MultiplayerPeer>()); self.change_scene_to_packed(menu); }`; `#[func] fn replace_main_scene(&mut self, resource: Gd<PackedScene>) { self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()]); }`; `#[func] fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) { let mut node = resource.instantiate().unwrap(); for mut child in self.base().get_children().iter_shared() { self.base_mut().remove_child(&child); child.queue_free(); } self.base_mut().add_child(&node); if node.has_signal("quit") { node.connect("quit", &Callable::from_object_method(&self.to_gd(), "go_to_main_menu")); } if node.has_signal("replace_main_scene") { node.connect("replace_main_scene", &Callable::from_object_method(&self.to_gd(), "replace_main_scene")); } }`. **Não** trocar o `has_signal`/`connect` por nome por `try_cast::<Level>`/`<Menu>` (duck typing do original) (research D3, D10)
- [ ] T045 [US4] Adicionar `mod main_scene;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod menu;`)
- [ ] T046 [US4] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0
- [ ] T047 [US4] Editar `oxide-godot/main/main.tscn` como texto (reconferir com `grep -n 'name="main" type=\|^script = ExtResource("1")\|main.gd' oxide-godot/main/main.tscn` → esperado l.5, l.6, l.3): na raiz trocar `type="Node"` por `type="Main"` **mantendo `name="main"`**; remover `script = ExtResource("1")`; remover `[ext_resource type="Script" uid="uid://chrcwbh6kvb7i" path="res://main/main.gd" id="1"]`. Resultado: arquivo de 3 linhas (`[gd_scene …]`, vazia, `[node name="main" type="Main" unique_id=1021137562]`). Diff esperado: `4 +---`
- [ ] T048 [US4] Apagar `oxide-godot/main/main.gd` e `oxide-godot/main/main.gd.uid` (`git rm`); verificar `grep -rn 'uid://chrcwbh6kvb7i' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'main/main.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio; `grep -n 'run/main_scene' oxide-godot/project.godot` = `res://main/main.tscn` (intocado)
- [ ] T049 [US4] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . main/main.tscn 2>&1 | tee /tmp/run.log` → exit 124, grep de regressão (incl. `shadows a native class`) vazio (regra do erro intermitente) — boot Rust → `go_to_main_menu` → `Menu` Rust hospeda → loading → `replace_main_scene` recebido por `Callable` por nome → `call_deferred("change_scene_to_packed")` → `Level` Rust instanciado, `quit` conectado; `menu/menu.tscn` → exit 124, grep vazio
- [ ] T050 [US4] Verificações mecânicas e contrato (`contracts/main.md`): `grep -n 'name="main" type=' oxide-godot/main/main.tscn` = `type="Main"` com `name="main"`; `grep -c 'ExtResource("1")' …` = 0; `grep -n '#\[func\]' -A1 oxide_godot_core/oxide_godot_lib/src/main_scene.rs | grep 'fn '` = `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` (3, nenhum outro); `grep -n 'has_signal\|from_object_method\|call_deferred' oxide_godot_core/oxide_godot_lib/src/main_scene.rs` = `"quit"`, `"replace_main_scene"` ×2, `"change_scene_to_packed"`; `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = só `oxide-godot/menu/settings.gd`; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `main.gd`; `ls src/` = 15 arquivos
- [ ] T051 [US4] Registrar em `docs/v2-backlog.md` a linha nº 24 (research.md §"Backlog v2 candidato"): `main/main.gd` (port 4) — chamar `change_scene_to_packed` diretamente em vez de `call_deferred` por nome; motivação: chamada por string ao próprio método, mantida na v1 por fidelidade. Não duplicar os itens 1–23
- [ ] T052 [US4] Commit único do port 4 na `main` (autor the repository author; conferir `git status` antes), incluindo `src/main_scene.rs`, `src/lib.rs`, `oxide-godot/main/main.tscn`, as deleções de `main.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port main.gd → Main (Node); main.tscn: node main type="Node"→"Main"` + corpo: notas (`server_relay` via `SceneMultiplayer`; 60 fps em headless; modo da janela do `config_file`; `has_signal`/`connect` por nome e `call_deferred` por nome preservados — duck typing do original; node continua chamado `main`; arquivo `main_scene.rs`); `- backlog v2: item 24`. Anotar o hash
- [ ] T053 [US4] **Checkpoint do usuário (validação visual, plan.md port 4)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: boot direto no menu com o modo de janela salvo aplicado; Play → level; ESC → menu (peer offline recriado, mouse visível); Play de novo → level de novo (cena anterior liberada — sem duplicar jogador/robôs); Quit fecha; em headless (`timeout 20 godot --headless --path . main/main.tscn`) o boot hospeda e chega ao level sozinho. No editor, `main.tscn`: node `main` do tipo `Main`, sem script. Divergência → commit `Fix port main.gd …`. Só seguir para o Polish com o OK explícito

**Checkpoint**: 1 `.gd` restante (`settings.gd`); fluxo de jogo inteiro em Rust; jogo jogável.

---

## Phase 7: Polish — verificação final do marco

**Purpose**: apenas a verificação final do quickstart e a validação visual completa. Sem
documentação extra, sem README, sem refatoração.

- [ ] T054 Verificação mecânica do marco (quickstart §"Verificação final"): `find oxide-godot -name '*.gd' -not -path '*/addons/*'` = só `oxide-godot/menu/settings.gd`; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 1; `git diff --stat a866428 -- 'oxide-godot/**/*.gd'` mostra exatamente 4 deleções (`flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`) e **nenhuma** linha de `settings.gd` (SC-001); `git log --oneline a866428..HEAD | grep -c '^[0-9a-f]* Port '` = 4 (`Fix port …` não contam); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 14 módulos (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot flying_forklift level menu main_scene`); `grep -n '^godot' oxide_godot_core/Cargo.toml` = `godot = { version = "0.5.5", features = ["experimental-threads"] }`; `grep -c 'experimental-threads' CLAUDE.md` ≥ 1 e `grep -c 'already initialized RID' CLAUDE.md` = 1; `grep -c '^| [0-9]' docs/upstream-bugs.md` = 2; `grep -c '^| [0-9]' docs/v2-backlog.md` = 24; FR-025: `grep -nE '\.call\(' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs` → só `"apply_graphics_settings"` (level, menu ×2) e `"save_settings"` (menu); `grep -nE 'call_deferred\(' …/{menu,main_scene}.rs` → só `"_on_host_pressed"` e `"change_scene_to_packed"`; `grep -nE '\.get\("' …/{flying_forklift,level,menu,main_scene}.rs` → só `"config_file"`; `grep -nE 'has_signal|from_object_method' …/main_scene.rs` → só `"quit"`/`"replace_main_scene"`/`"go_to_main_menu"`; `grep -n 'try_cast::<Level>\|try_cast::<Menu>' …/main_scene.rs` vazio
- [ ] T055 Validação headless final (sem editor aberto): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; import headless com `Initialize godot-rust` e sem `ERROR`; `main/main.tscn`, `menu/menu.tscn`, `level/level.tscn`, `level/forklift/flying_forklift.tscn`, `player/player.tscn`, `enemies/red_robot/red_robot.tscn` headless com grep de regressão vazio (regra do erro intermitente para `main.tscn`)
- [ ] T056 **Validação visual completa do usuário (SC-002)**: boot → menu (Play, Play Online, Settings com as 15 linhas refletindo e gravando cada opção — aplicar, cancelar, reabrir, conferir `user://settings.ini`, reiniciar o jogo —, Quit) → loading com barra → level jogável (jogador, robôs com respawn 15 s, empilhadeiras com modelos variados, GI conforme a opção) → ESC volta ao menu → Play de novo — tudo indistinguível de `../oxide_godot_origins/` em sessão lado a lado; Marcos A–C continuam iguais (tiro, tremor, peças, porta em teste isolado). No editor: as 4 cenas com a raiz do tipo Rust e sem script. Marco D concluído com o OK do usuário; então marcar T001–T056 `[x]` e commitar só `tasks.md` (`Tasks 004: marco D concluído`)

---

## Dependencies & Execution Order

### Ordem obrigatória

```
Phase 1 (Setup: T001–T005)          — baseline das 4 cenas + regra do erro intermitente de main.tscn
  → Phase 3 US1 (T006–T015)  → commit 1  (FlyingForklift; base CharacterBody3D)
  → Phase 4 US2 (T016–T028)  → commit 2  (Level; + pub(crate) em player.rs/red_robot.rs)
  → Phase 5 US3 (T029–T043)  → commit 3  (Menu; + Cargo.toml feature + CLAUDE.md)
  → Phase 6 US4 (T044–T053)  → commit 4  (Main; main.gd deixa de existir — settings.gd é o único .gd)
  → Phase 7 Polish (T054–T056)
```

- **Phase 2 (Foundational)**: não existe — nada bloqueia as stories além do Setup.
- **Stories não são paralelizáveis entre si**: cada uma termina com um commit na `main` e a
  próxima parte da árvore limpa (FR-028, SC-004). US4 conecta por nome os sinais que US2/US3
  registram; até lá, `main.gd` original faz o mesmo — o jogo é jogável após cada commit.
- **Ordem interna de cada story** (dependências estritas): [US2: visibilidade `pub(crate)`] /
  [US3: `Cargo.toml` → build de regeneração] → módulo `.rs` → `mod` em `lib.rs` → `cargo build` →
  editar `.tscn` → apagar `.gd`/`.uid` → [US3: `CLAUDE.md`] → headless → verificações mecânicas +
  contrato → backlog → commit → checkpoint do usuário. A `.tscn` só é editada depois do build
  porque a classe precisa existir na lib carregada para o `type` resolver no import headless.
- **Checkpoint do usuário** (T015, T028, T043, T053, T056) é bloqueante: a story seguinte só
  começa com o OK explícito.

### Parallel Opportunities

Praticamente nenhuma, por construção:

- Setup: T004 é [P] em relação a T002/T003/T005 (só lê diretórios e faz greps).
- Dentro de cada story, nenhuma task é [P]: cada passo consome o resultado do anterior.
  T017→T018→T019 e T031→T032→T033 escrevem o mesmo arquivo em sequência; T029→T030 (feature →
  regeneração) precedem T031.
- Backlog (T013, T026, T041, T051) e `CLAUDE.md` (T038) poderiam ser escritos a qualquer momento
  antes do commit da story, mas editam arquivos compartilhados — manter sequencial.

### Parallel Example

```bash
# Único par realmente independente (Phase 1):
Task: "T002 cargo build → contar warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out; grep de colisão de nomes"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline registrada, inclusive o intermitente de `main.tscn`.
2. Phase 3: US1 (T006–T015) — `flying_forklift.gd` portado, commit 1, OK do usuário.
3. **PARAR E VALIDAR**: primeiro port em que a base da classe difere do `extends` do script
   (regra v1.3.1); qualquer problema com o `CharacterBody3D` + `Collider` aparece aqui.

### Incremental Delivery

Cada story é um port completo e o jogo fica jogável após cada commit:

1. US1 → 4 `.gd` restantes → jogável
2. US2 → 3 → jogável (`main.gd` conecta `quit` do `Level` Rust por `has_signal`)
3. US3 → 2 → jogável (feature ativa; `main.gd` conecta `replace_main_scene` do `Menu` Rust)
4. US4 → 1 (`settings.gd`) → jogável — fluxo inteiro em Rust
5. Polish → marco D concluído

### Se algo falhar no meio de uma story

Não commitar parcial. Ou o port inteiro (módulo + cena + deleção [+ visibilidade / + `Cargo.toml`
+ `CLAUDE.md`]) entra no commit, ou nada:
`git checkout -- oxide-godot/ oxide_godot_core/ docs/ CLAUDE.md && git clean -f oxide_godot_core/oxide_godot_lib/src/<módulo>.rs`
volta à árvore limpa da story anterior, que é sempre jogável. Se a implementação exigir algo não
previsto nas tasks (outro arquivo, outra visibilidade, outra feature, uma correção), parar e reportar.

---

## Notes

- Nenhuma task cria helper, trait, módulo comum, teste ou log novo. Se parecer necessário, é
  entrada em `docs/v2-backlog.md` — não task. O menu mantém as 15 chamadas de `_make_button_group`
  e as ~30 cadeias `if/elif` do original (D8–D9).
- **Nenhuma correção de bug** neste marco. Defeito objetivo → parar e reportar; em dúvida, é melhoria.
- Toda `.tscn` é editada como texto; reconferir linhas com `grep -n` imediatamente antes de cada
  `sed` (cada cena é editada uma única vez neste marco).
- Nomes de `#[func]`, sinais e os 85 caminhos de node são contrato (FR-024) — copiar do
  GDScript/`contracts/menu.md`, nunca "traduzir" (`_on_loading_done_timer_timeout`,
  `change_scene_to_packed`, `UI/Settings/ScreenSpaceAA/FXAA`).
- O grep de regressão headless é `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class`;
  os WARNINGs da baseline (`HDR output`, `Physics interpolation`) não contam; em `main.tscn`, as 5
  linhas do renderizador dummy não contam se sumirem na 2ª execução (3 seguidas = regressão).
- Nenhum editor Godot aberto durante validações headless — avisar o usuário antes; nunca matar o
  processo dele.
- O commit (T014, T027, T042, T052) vem **antes** do checkpoint visual; divergência encontrada
  pelo usuário é corrigida em commit `Fix port …` na mesma story (nunca `Port …`, para
  `git log | grep -c '^[0-9a-f]* Port '` continuar = 4).
- **Staging**: antes de qualquer commit que não seja o do port (ex.: um commit de spec do
  usuário), `git status` não pode ter deleções de `.gd` em staging (lição do Marco C).
- Arquivos que **nunca** mudam neste marco: `.gdextension`, `project.godot`,
  `docs/upstream-bugs.md`, `oxide-godot/menu/settings.gd` (+ `.uid`), `specs/` (exceto o `[x]` em
  `tasks.md` ao final), e em `player.rs`/`red_robot.rs` qualquer coisa além das palavras
  `pub(crate)` de T016. `Cargo.toml` e `CLAUDE.md` mudam **só** em T029/T038.
