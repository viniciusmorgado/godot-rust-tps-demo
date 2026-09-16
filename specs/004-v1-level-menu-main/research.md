# Research: Marco D — empilhadeira, level, menu e main (gdext 0.5.5)

**Fase**: v1 — Raw Port. Assinaturas exatas do gdext 0.5.5 usadas pelos quatro ports e decisões de
tradução, com alternativas descartadas. Fontes: crate `~/.cargo/registry/src/*/godot-core-0.5.5/`,
`godot-codegen-0.5.5/`, `godot-macros-0.5.5/`; bindings geradas em
`oxide_godot_core/target/debug/build/godot-core-*/out/`. **Atenção**: este marco ativa a feature
`experimental-threads` (D1), o que gera um **segundo** diretório de bindings — em 2026-09-16
existem `godot-core-aea5c50e7fda9d57/out` (sem a feature, usado pelos Marcos A–C) e
`godot-core-4eba5d49e15a0d7e/out` (com a feature). As linhas citadas abaixo são do diretório
**com** a feature; `ls -dt .../godot-core-*/out | head -1` devolve o mais recente.

**Método de confirmação**: todo o mapeamento foi escrito como módulo temporário
(`zz_research.rs`, classes `ZzFlyingForklift`/`ZzLevel`/`ZzMenu`/`ZzMain` — o menu com um
subconjunto representativo dos 85 `@onready`, mas com todas as *formas* de API que os handlers
usam) dentro do crate **com a feature ativada temporariamente**, compilado com `cargo build` →
**0 erros, 0 warnings** após quatro correções impostas pelo compilador (D3, D9, D10 — registradas
porque o input do comando as tinha diferente), e exercitado em headless por script `-s` (§E).
Módulo, `Cargo.toml` e as alterações temporárias de visibilidade foram revertidos (`git status`
limpo).

## D1 — Feature `experimental-threads` (mudança de configuração do crate, decidida pelo usuário)

- **Decisão**: em `oxide_godot_core/Cargo.toml`, `[workspace.dependencies]`:
  `godot = { version = "0.5.5", features = ["experimental-threads"] }`. Entra no commit do
  **port 3 (menu)**, citada na mensagem; `CLAUDE.md` §Toolchain ganha uma linha registrando a
  feature e o motivo. Versão do crate inalterada; nenhuma outra feature.
- **Racional**: `godot-codegen-0.5.5/src/special_cases/special_cases.rs:83-86` exclui
  `ResourceLoader::load_threaded_request`, `load_threaded_get_status` e `load_threaded_get` do
  codegen quando a feature está desligada (`#[cfg(not(feature = "experimental-threads"))]`);
  `menu.gd:130,158,163` usa os três (barra de loading). Sem a feature, a única alternativa seria
  `ResourceLoader::singleton().call("load_threaded_request", …)` — chamada dinâmica ao engine,
  contrária ao espírito do Princípio II (código sem tipo). `godot-0.5.5/Cargo.toml:78`:
  `experimental-threads = ["godot-core/experimental-threads"]`.
- **Confirmado**: com a feature, `resource_loader.rs` (bindings novas) tem
  `load_threaded_request(path) -> Error` (l.37) / `load_threaded_request_ex(path)` com
  `.use_sub_threads(bool)` (l.42, 307); `load_threaded_get_status(path) -> ThreadLoadStatus`
  (l.58) / `_ex(path)` com `.progress(&AnyArray)` (l.63, 341);
  `load_threaded_get(path) -> Option<Gd<Resource>>` (l.67). `ThreadLoadStatus` tem
  `INVALID_RESOURCE`, `IN_PROGRESS`, `FAILED`, `LOADED` (l.452+). Build do crate com a feature:
  0 warnings; as bindings sem a feature continuam em cache (troca de volta é instantânea).
- **Efeito colateral aceito**: a feature também habilita outras APIs marcadas como thread-unsafe
  no gdext; nenhuma é usada. É a única mudança de configuração da v1 até aqui — registrada em
  plan.md §Complexity Tracking.

## D2 — Estrutura e visibilidade

- **Decisão**: `src/flying_forklift.rs` (`FlyingForklift: CharacterBody3D`), `src/level.rs`
  (`Level: Node3D`), `src/menu.rs` (`Menu: Node`), `src/main_scene.rs` (`Main: Node` — nome de
  arquivo `main_scene.rs` para não sugerir um `main.rs` de binário; `mod main_scene;`).
- **Visibilidade (só a palavra, commit do port 2 — Level)**: `player.rs`
  `fn set_player_id` → `pub(crate)` (o level faz `player.bind_mut().set_player_id(id)`);
  `red_robot.rs` `#[signal] fn exploded();` → `pub(crate) fn exploded();` — **imposto pelo
  compilador**: o acessor `robot.signals().exploded()` herda a visibilidade do `fn` do sinal
  (`error[E0624]: method exploded is private`). Precedentes Marcos B/C.
- **Nomes**: conferidos pelo grep do `CLAUDE.md` e contra `out/classes/`: `FlyingForklift`,
  `Level`, `Menu`, `Main` sem colisão (`menu.gd` tem `var main` — minúsculo, e é portado antes de
  `Main` existir).
- **Alternativas descartadas**: `player.set("player_id", …)` (dinâmico entre classes Rust, viola
  FR-025); `robot.connect("exploded", …)` por nome (idem — o robô já é Rust).

## D3 — Comparações de string e `GString`

- **Correção imposta pelo compilador**: `x == "metal".into()` é ambíguo (`GString: PartialEq<_>`
  tem várias impls). Usar `GString::from("metal")` / `GString::from("headless")`:
  `RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal")`
  (`rendering_server.rs:5391`), `DisplayServer::singleton().get_name() == GString::from("headless")`
  (`display_server.rs:35`). Mesma lição do `StringName::from("Target")` do Marco C.
- `metalfx_supported` pode ser inicializado no `#[init(val = …)]` — o singleton está disponível
  na construção (compilou; o original também avalia no inicializador da `var`).

## D4 — FlyingForklift

- **Base `CharacterBody3D`** (constituição v1.3.1, Princípio II: `extends Node3D` é ancestral do
  tipo do node `flying_forklift.tscn:36`). `#[init(node = "SpotLight3D")] spot_light: OnReady<Gd<SpotLight3D>>`;
  `Light3D::set_shadow(false)` (`light_3d.rs:62`).
- `Settings.config_file.get_value(...)`: `self.base().get_node_as::<Node>("/root/Settings").get("config_file").to::<Gd<ConfigFile>>().get_value("rendering", "shadow_mapping").to::<bool>()`
  (padrão D14 do Marco B).
- `randomize()` → `godot::global::randomize()` (`utilities.rs:792`); `get_child(0)` →
  `self.base().get_child(0).unwrap()` (`node.rs:368`, `Option`); `get_children()` →
  `Array<Gd<Node>>` (`node.rs:347`) com `.len()` e `.iter_shared()`;
  `floori(randf() * float(n))` → `(randf() * n as f64).floor() as usize`;
  `children[i].visible = …` → `child.cast::<Node3D>().set_visible(i == which)` (`node_3d.rs:620`;
  os filhos do modelo são `Node3D`).
- **Confirmado (§E.1)**: `ZzFlyingForklift.new() is CharacterBody3D == true`.

## D5 — Level: sinal, Settings dinâmico, GI

- `#[signal] fn quit();` no bloco `#[godot_api] impl Level` principal; emissão
  `self.signals().quit().emit()` (padrão D6 do Marco C). **Confirmado (§E.1)**: `has_signal("quit")`,
  0 argumentos, conexão por nome a partir de GDScript recebe a emissão (= o que `main.gd:30-31` faz).
- `Settings.apply_graphics_settings(get_window(), world_environment.environment, self)` →
  ```rust
  let window = self.base().get_window().unwrap();                       // node.rs:1163, Option
  let environment = self.world_environment.get_environment().unwrap();  // world_environment.rs:179, Option
  let mut settings = self.base().get_node_as::<Node>("/root/Settings");
  settings.call("apply_graphics_settings",
      &[window.to_variant(), environment.to_variant(), self.to_gd().to_variant()]);   // object.rs:419
  ```
  — exceção `Settings` do Princípio II (chamada dinâmica ao autoload). `save_settings` idem:
  `settings.call("save_settings", &[])`.
- `Settings.GIType.SDFGI` etc. → constantes locais `const SDFGI: i64 = 0; VOXEL_GI = 1;`
  (`LIGHTMAP_GI = 2` é o ramo `else`, não precisa de constante) e `GI_DISABLED = 0; GI_LOW = 1;
  GI_HIGH = 2` (valores de `settings.gd:3-13`); leitura `get_value(..).to::<i64>()`. Backlog v2
  item 1 já cobre o acesso tipado ao enum.
- `environment.sdfgi_enabled = x` → `environment.set_sdfgi_enabled(x)` (`environment.rs:822`);
  `$VoxelGI.hide()` → `self.base().get_node_as::<Node3D>("VoxelGI").hide()` (`node_3d.rs:659`;
  `VoxelGI` e `ReflectionProbes` são buscados inline com `$` no original — mantidos inline);
  `RenderingServer.environment_set_sdfgi_ray_count(ENV_SDFGI_RAY_COUNT_96)` →
  `RenderingServer::singleton().environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_96)`
  (`rendering_server.rs:3493`; constantes `COUNT_32/64/96/128`, l.14262+ — grafia gdext sem o
  prefixo `RAY_`); `voxel_gi_set_quality(VoxelGiQuality::HIGH | LOW)` (`:1795`, consts l.9967/9972).
- **`lightmap_gi` após `queue_free`**: o original chama `lightmap_gi.queue_free()` e **mantém**
  a referência (`lightmap_gi != null` continua verdadeiro), e `setup_lightmapgi` só cria um novo
  se `== null`. Reproduzir literalmente: `if let Some(lightmap_gi) = &mut self.lightmap_gi { lightmap_gi.queue_free(); }`
  **sem** `take()`; `setup_lightmapgi` cria só `if self.lightmap_gi.is_none()`. (Na prática o
  level é recriado a cada Play e cada `ready` chama um único `setup_*`, então a referência nunca
  fica pendurada; a fidelidade é ao código.) Criação: `LightmapGi::new_alloc()` (grafia gdext
  `LightmapGi`/`LightmapGiData`), `set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"))`
  (`lightmap_gi.rs:173`), `set_name("LightmapGI")` (`node.rs:243`), `self.base_mut().add_child(&new_gi)`;
  guardar `Some(new_gi.clone())` **antes** do `add_child` (a variável é movida para a árvore só
  por referência, mas o clone evita disputa de borrow).

## D6 — Level: spawn, jogadores, sinais do MultiplayerAPI

- Servidor: `let multiplayer = self.base().get_multiplayer().unwrap(); if multiplayer.is_server() { … }`.
- `for child in robot_spawn_points.get_children(): spawn_robot(child)` →
  `for child in self.robot_spawn_points.get_children().iter_shared() { self.spawn_robot(child.cast::<Node3D>()); }`
  (`spawn_point` sem tipo no original; o port tipa `Gd<Node3D>` — só `.transform` é usado).
- `spawn_points.shuffle()` → `Array<Gd<Node>>` faz `Deref` para `AnyArray`, que tem `shuffle()`
  (`any_array.rs:366`); `pop_front() -> Option<Gd<Node>>` (`array.rs:386`);
  `multiplayer.get_peers() -> PackedInt32Array` (`multiplayer_api.rs:134`) → `.as_slice()`.
- `spawn_robot(spawn_point)`: `let mut robot: Gd<EnemyRobot> = load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>();`
  (**tipado** — o robô é Rust desde o Marco C); `robot.set_transform(spawn_point.get_transform())`;
  `robot.exploded.connect(_respawn_robot.bind(spawn_point))` →
  `robot.signals().exploded().connect_other(&*self, move |this: &mut Level| this._respawn_robot(spawn_point.clone()))`
  (o `bind` vira captura por `move`; exige `pub(crate) fn exploded()` — D2);
  `spawned_nodes.add_child(robot, true)` → `self.spawned_nodes.add_child_ex(&robot).force_readable_name(true).done()`.
- `_respawn_robot(spawn_point)`: `self.base().get_tree().create_timer(15.0).signals().timeout().connect_other(&*self, move |this: &mut Level| this.spawn_robot(spawn_point.clone()))`
  — callable *linked*: se o level for liberado (ESC → menu) antes dos 15 s, não roda
  (equivalente ao `await` do original num objeto liberado).
- `add_player(id: int, spawn_point: Marker3D = null)` → `fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>)`
  **privado** (sem `#[func]`): o GDScript conecta `peer_connected` (1 argumento) ao método com
  parâmetro default; gdext não tem parâmetro default em `#[func]`, então a conexão é por closure
  tipada — `multiplayer.signals().peer_connected().connect_other(&*self, |this: &mut Level, id: i64| this.add_player(id as i32, None))`
  e `peer_disconnected().connect_other(&*self, |this, id: i64| this.del_player(id as i32))`
  (`multiplayer_api.rs:388,394`; o argumento do sinal é **`i64`**, l.418 — `id as i32` na
  fronteira, como `player_id: i32` do Marco B). Nenhum script chama `add_player`/`del_player` por
  nome (só os sinais), então FR-008 ("conectáveis aos sinais pelos nomes") é satisfeito pela
  conexão tipada; spec Key Entities lista-os como métodos do Level — contrato de comportamento,
  não de `has_method`. Registrado em plan.md.
- Corpo de `add_player`: `let spawn_point = spawn_point.unwrap_or_else(|| { let count = self.player_spawn_points.get_child_count(); self.player_spawn_points.get_child((randi() % count as i64) as i32).unwrap().cast::<Marker3D>() });`
  (`randi() -> i64`, `get_child_count() -> i32`); `let mut player: Gd<Player> = load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>();`
  (**tipado**); `player.set_name(&id.to_string())` (`node.rs:243`, `AsArg<StringName>` aceita `&String`);
  `player.bind_mut().set_player_id(id)` (setter Rust, `pub(crate)` — D2; é exatamente o que
  `player.player_id = id` dispara no original); `player.set_transform(spawn_point.get_transform())`;
  `self.spawned_nodes.add_child(&player)` — **sem** `force_readable_name` (quirk: `level.gd:121`).
- `del_player(id)`: `let name = id.to_string(); if !self.spawned_nodes.has_node(&name) { return; } self.spawned_nodes.get_node_as::<Node>(&name).queue_free();`
  (`node.rs:377`).
- `_input`: `fn input(&mut self, input_event: Gd<InputEvent>) { if input_event.is_action_pressed("quit") { Input::singleton().set_mouse_mode(MouseMode::VISIBLE); self.signals().quit().emit(); } }`.

## D7 — Menu: sinal com parâmetro, estado, 85 referências

- `signal replace_main_scene` (declarado sem parâmetro, emitido com 1 — spec Edge Cases) →
  `#[signal] fn replace_main_scene(scene: Gd<PackedScene>);`; emissão
  `self.signals().replace_main_scene().emit(&scene)`. **Confirmado (§E.1)**: o sinal registra 1
  argumento `scene: PackedScene`; conexão por nome a partir de GDScript (como `main.gd:33`)
  recebe a `PackedScene`.
- `var peer: MultiplayerPeer = OfflineMultiplayerPeer.new()` →
  `#[init(val = OfflineMultiplayerPeer::new_gd().upcast())] peer: Gd<MultiplayerPeer>`;
  `metalfx_supported` → `#[init(val = RenderingServer::singleton().get_current_rendering_driver_name() == GString::from("metal"))]` (D3).
- `const LEVEL_PATH: &str = "res://level/level.tscn"`.
- Os **85** `@onready` de `menu.gd:11-104` viram `#[init(node = "<caminho>")] OnReady<Gd<T>>`
  com o caminho **completo** a partir do menu (os encadeamentos `x.get_node(^"Y")` do original
  viram `X/Y`, como nos marcos anteriores). Tabela completa (nome, tipo, caminho) em
  [contracts/menu.md](contracts/menu.md) — é contrato de cena. Tipos: `WorldEnvironment`,
  `Control`, `Button`, `SpinBox`, `LineEdit`, `VBoxContainer`, `HBoxContainer`, `ProgressBar`,
  `Timer`. Campos nunca lidos (`ui`, `settings_button`, `quit_button`, `settings_actions`,
  `settings_action_apply`, os 15 `*_menu` só usados em `_make_button_group`) ficam por
  fidelidade — o derive referencia o campo, sem warning (precedente `crosshair`, Marco B).

## D8 — Menu: ready, process, `_make_button_group`

- `_on_host_pressed.call_deferred()` → `self.base_mut().call_deferred("_on_host_pressed", &[])`
  (`object.rs:438`; chamada por nome ao próprio `#[func]` — é o que o original faz).
- `play_button.grab_focus()` (`control.rs:793`); `hide()`/`show()`/`is_visible()` de
  `CanvasItem` (`canvas_item.rs:66,75`).
- `_make_button_group(menu)` ×15 → método privado `fn _make_button_group(&mut self, common_parent: Gd<Node>)`:
  `let group = ButtonGroup::new_gd(); for btn in common_parent.get_children().iter_shared() { if let Ok(mut btn) = btn.try_cast::<BaseButton>() { btn.set_button_group(&group); } }`
  (`base_button.rs:418`; `try_cast` reproduz `if not btn is BaseButton: continue`). As 15
  chamadas ficam explícitas (`self._make_button_group(self.display_mode_menu.clone().upcast())` …)
  — o `for menu in [...]` do original vira 15 linhas; sem helper adicional.
- `_process`: ```rust
  if self.loading.is_visible() {
      let progress = VarArray::new();                       // &VarArray deref-coerce → &AnyArray
      let status = ResourceLoader::singleton().load_threaded_get_status_ex(LEVEL_PATH).progress(&progress).done();
      if status == ThreadLoadStatus::IN_PROGRESS { self.loading_progress.set_value(progress.at(0).to::<f64>() * 100.0); }
      else if status == ThreadLoadStatus::LOADED { self.loading_progress.set_value(100.0); self.base_mut().set_process(false); self.loading_done_timer.start(); }
      else { godot_print!("Error while loading level: {}", status.ord()); self.main.show(); self.loading.hide(); }
  }
  ``` (`Range::set_value(f64)`, `range.rs:276`; `print("…" + str(status))` → `godot_print!` com o
  inteiro do enum via `.ord()`, mesmo texto que `str()` de um enum produz).

## D9 — Menu: handlers, `config_file` e inteiros dos enums

- Handlers `#[func]`: `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`,
  `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`,
  `_on_apply_pressed`, `_on_loading_done_timer_timeout` (10 conexões, `menu.tscn:836-845`).
  **Confirmado (§E.1)**: todos visíveis por `has_method`; `_make_button_group` não.
- `_on_play_pressed`: `ResourceLoader::singleton().load_threaded_request_ex(LEVEL_PATH).use_sub_threads(true).done()`
  (retorno `Error` ignorado, como no original).
- `_on_loading_done_timer_timeout`: `let peer = self.peer.clone(); self.base().get_multiplayer().unwrap().set_multiplayer_peer(&peer);`
  (`multiplayer_api.rs:43`); `let scene = ResourceLoader::singleton().load_threaded_get(LEVEL_PATH).unwrap().cast::<PackedScene>(); self.signals().replace_main_scene().emit(&scene);`.
- `_on_quit_pressed`: `self.base().get_tree().quit()` (`scene_tree.rs:377`).
- `_on_host_pressed`: `let mut peer = ENetMultiplayerPeer::new_gd(); peer.create_server(self.online_port.get_value() as i32); self.peer = peer.upcast(); self._on_play_pressed(); self.online.hide();`
  (`enet_multiplayer_peer.rs:135`; `Range::get_value() -> f64`, `int(x)` → `as i32`).
  `_on_connect_pressed`: `peer.create_client(&self.online_address.get_text(), self.online_port.get_value() as i32)` (`:156`; `line_edit.rs:445`).
- `_on_settings_pressed` / `_on_apply_pressed`: tradução linha a linha de `menu.gd:175-311` e
  `318-441`. `Settings.config_file` é lido **uma vez** no início de cada handler
  (`let config_file = …get("config_file").to::<Gd<ConfigFile>>()`) — o original repete
  `Settings.config_file` a cada linha, mas é o mesmo objeto (o autoload guarda um único
  `ConfigFile`); ler o handle uma vez não altera comportamento nem é abstração (é a variável
  local que qualquer tradução precisa). Leituras: `.get_value(s, k).to::<i64>()` (enums e
  `max_fps`), `.to::<f64>()` (`resolution_scale`), `.to::<bool>()` (flags). Gravações:
  `config_file.set_value(s, k, &v.to_variant())` com `v: i64` para enums/fps, `f64` para escala,
  `bool` para flags.
- **Inteiros dos enums do engine** — usar `.ord()` (`obj/traits.rs:199`, `EngineEnum::ord() -> i32`)
  dos enums gdext, convertido `as i64` para comparar/gravar: `window::Mode::{WINDOWED, MAXIMIZED, FULLSCREEN, EXCLUSIVE_FULLSCREEN}`
  (`window.rs:2342+`), `display_server::VSyncMode::{DISABLED, ENABLED, ADAPTIVE, MAILBOX}`,
  `viewport::Scaling3DMode::{BILINEAR, FSR, FSR2, METALFX_SPATIAL, METALFX_TEMPORAL}` — **`NEAREST` NÃO existe na API prebuilt 4.6** (foi adicionado no Godot 4.7; `viewport.rs` só tem BILINEAR=0, FSR=1, FSR2=2, METALFX_SPATIAL=3, METALFX_TEMPORAL=4, MAX=5): usar a constante local `const SCALING_3D_MODE_NEAREST: i64 = 5;` (valor do runtime 4.7.2, conferido em headless pelo /speckit-analyze) para ler e gravar o ramo Nearest — mesma técnica dos inteiros de GIType,
  `viewport::Msaa::{DISABLED, MSAA_2X, MSAA_4X, MSAA_8X}`, `viewport::ScreenSpaceAa::{DISABLED, FXAA, SMAA}`,
  `rendering_server::EnvironmentSsaoQuality::{MEDIUM, HIGH}`, `EnvironmentSsilQuality::{MEDIUM, HIGH}`.
  **Confirmado (§E.1)**: os inteiros do Godot batem com os `.ord()` (`MODE_WINDOWED=0, MAXIMIZED=2,
  FULLSCREEN=3, EXCLUSIVE_FULLSCREEN=4, VSYNC_MAILBOX=3, SCALING_3D_MODE_FSR2=2, MSAA_8X=3,
  SCREEN_SPACE_AA_SMAA=2, ENV_SSAO_QUALITY_HIGH=3, ENV_SSIL_QUALITY_MEDIUM=2`). `-1` (SSAO/SSIL
  desligados) e `0` (fps ilimitado) são literais `i64`. `Settings.GIType.*`/`GIQuality.*` → os
  literais de D5.
- `is_equal_approx(a, b)` → `godot::global::is_equal_approx(a: f64, b: f64)` (`utilities.rs:412`);
  `1.0 / 1.7` etc. como `f64`.
- `btn.button_pressed = true` → `set_pressed(true)`; `btn.button_pressed` → `is_pressed()`
  (`base_button.rs:226,235`).
- **Correção imposta pelo compilador**: nenhuma além de D3 nesta seção; tudo compilou no
  subconjunto (ver §E.0 para o que o subconjunto cobriu).

## D10 — Main

- `multiplayer.server_relay = false` → `self.base().get_multiplayer().unwrap().cast::<SceneMultiplayer>().set_server_relay_enabled(false)`
  (`scene_multiplayer.rs:262`; a propriedade é de `SceneMultiplayer`, subclasse concreta do
  `MultiplayerApi` default — o `cast` faz panic se o projeto usasse outra implementação, que não é
  o caso).
- `Engine.max_fps = 60` → `Engine::singleton().set_max_fps(60)` (`engine.rs:88`);
  `get_window().mode = config` → `self.base().get_window().unwrap().set_mode(window::Mode::from_ord(display_mode as i32))`
  (`window.rs:372`; `EngineEnum::from_ord(i32)`, `obj/traits.rs:201`).
- `go_to_main_menu` (`#[func]` — é alvo de `connect` por nome): `let menu = ResourceLoader::singleton().load("res://menu/menu.tscn").unwrap().cast::<PackedScene>();`
  (`resource_loader.rs:89` — o original usa `ResourceLoader.load`, não `load()` global; manter);
  `multiplayer.get_multiplayer_peer().unwrap().close()` (`multiplayer_api.rs:34`, `multiplayer_peer.rs:111`);
  `multiplayer.set_multiplayer_peer(&OfflineMultiplayerPeer::new_gd().upcast::<MultiplayerPeer>())`;
  `self.change_scene_to_packed(menu)`.
- `replace_main_scene(resource)` (`#[func]`): `self.base_mut().call_deferred("change_scene_to_packed", &[resource.to_variant()])`
  — por nome, como o original; por isso `change_scene_to_packed` é `#[func]`.
- `change_scene_to_packed(resource)`: `let mut node = resource.instantiate().unwrap(); for mut child in self.base().get_children().iter_shared() { self.base_mut().remove_child(&child); child.queue_free(); } self.base_mut().add_child(&node);`
  (`node.rs:283`); duck typing preservado:
  `if node.has_signal("quit") { node.connect("quit", &Callable::from_object_method(&self.to_gd(), "go_to_main_menu")); }`
  (`object.rs:494`; `Object::connect(signal, &Callable) -> Error` em
  `classes/type_safe_replacements.rs:40`; `Callable::from_object_method(&Gd<T>, name)`,
  `builtin/callable.rs:50`); idem `"replace_main_scene"`.
- **Correção imposta pelo compilador**: nenhuma além de D3.

## D11 — O que NÃO muda (quirks preservados, FR-029)

`randomize()` em três lugares; `add_child(player)` sem nome legível vs `add_child(robot, true)`;
`has_signal` + `connect` por nome no `Main` (duck typing do original — o `Level`/`Menu` já são
Rust, mas o `Main` não os conhece por tipo, exatamente como o original); `call_deferred` por
nome; cadeias `if/elif` de `_on_apply_pressed` que não gravam nada se nenhum botão da linha está
pressionado; `pass`-equivalentes; `lightmap_gi` mantido após `queue_free`. Candidatos ao backlog
em §"Backlog v2 candidato".

## §E — Verificação empírica (2026-09-16, headless, sem editor aberto)

### E.0 Compilação do rascunho (feature ativa)

`ZzFlyingForklift` e `ZzMain` completos; `ZzLevel` completo (sinal, Settings dinâmico, 3
`setup_*`, spawn/respawn tipado, `add_player`/`del_player`, conexões `peer_*`, `input`);
`ZzMenu` com 20 dos 85 `OnReady` (ao menos um de cada tipo/caminho-raiz), sinal com parâmetro +
emissão, `process` com o loader em thread, `_make_button_group`, e os handlers `_on_play_pressed`,
`_on_settings_pressed` (um exemplo de cada forma de leitura: `.ord()` de `window::Mode`,
`VSyncMode`, `Msaa`, `ScreenSpaceAa`, `Scaling3DMode`, `EnvironmentSsaoQuality`/`Ssil`,
`is_equal_approx`, `bool`), `_on_apply_pressed` (um exemplo de cada forma de gravação + os dois
`call` dinâmicos), `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`,
`_on_loading_done_timer_timeout`. Resultado: **0 erros, 0 warnings** (`cargo clean -p` + build).

### E.1 Probe `-s` (classes de rascunho, sem cena)

```
forklift: is CharacterBody3D=true is Node3D=true
level: has_signal quit=true args=0 ; conexão GDScript por nome + emit → recebido
menu: has_signal replace_main_scene=true args=1 arg0=scene class=PackedScene ; conexão por nome recebe a PackedScene
menu has_method: _on_play_pressed, _on_settings_pressed, _on_quit_pressed, _on_host_pressed,
                 _on_connect_pressed, _on_apply_pressed, _on_loading_done_timer_timeout = true; _make_button_group = false
main has_method: go_to_main_menu, replace_main_scene, change_scene_to_packed = true
Window.MODE_WINDOWED=0 MAXIMIZED=2 FULLSCREEN=3 EXCLUSIVE=4 VSYNC_MAILBOX=3 FSR2=2 MSAA_8X=3 SMAA=2 SSAO_HIGH=3 SSIL_MEDIUM=2
```

### E.2 Baseline (commit `a866428`, todos os 4 scripts ainda em GDScript, sem a feature)

`cargo build` 0 warnings; import com `Initialize godot-rust` e 0 `ERROR`.
`flying_forklift.tscn` exit 124, grep vazio, 1 WARNING (HDR). `level.tscn` exit 124, grep vazio,
2 WARNINGs. `menu.tscn` exit 124, grep vazio, 1 WARNING (em headless o menu hospeda sozinho e
carrega o level em thread; sem `main`, o sinal `replace_main_scene` só é emitido).
`main.tscn` exit 124, 2 WARNINGs; em **6 execuções**, 5 sem nenhum `ERROR` e **1 (a primeira,
logo após `--import`) com 5 linhas `ERROR` do renderizador dummy**: `Initializing already
initialized RID` (`rid_owner.h:236`), `Parameter "mem" is null.` (`rid_owner.h:284`), 3×
`Parameter "m" is null.` (`dummy/storage/mesh_storage.h:74,81,110`). O projeto original intocado
(`../oxide_godot_origins/`, 4 execuções) tem 0 `ERROR` mas a mesma arquitetura (carregamento do
level em sub-thread com renderizador dummy); a corrida é do engine em headless, não do port.
**Regra para o quickstart**: essas 5 linhas exatas, quando aparecem em `main.tscn` headless,
não são regressão **se** sumirem numa segunda execução; qualquer outra linha `ERROR`, ou essas
mesmas reproduzíveis em 3 execuções seguidas, é regressão. A ser catalogada no `CLAUDE.md` (edição
operacional) junto com a linha da feature.

## Mapa por script (resumo para as tasks)

### 1. `flying_forklift.gd` → `src/flying_forklift.rs` — `FlyingForklift: CharacterBody3D` (D4)
### 2. `level.gd` → `src/level.rs` — `Level: Node3D` (D5–D6); `player.rs`/`red_robot.rs` só visibilidade (D2)
### 3. `menu.gd` → `src/menu.rs` — `Menu: Node` (D7–D9); `Cargo.toml` feature + `CLAUDE.md` linha (D1)
### 4. `main.gd` → `src/main_scene.rs` — `Main: Node` (D10)

## Backlog v2 candidato (registrar em `docs/v2-backlog.md` no commit do script correspondente)

Itens 1–18 já existem (o 1 cobre o acesso tipado ao `Settings`, inclusive `GIType`/`GIQuality`). Novos:

| # | Origem | Melhoria | Motivação |
|---|---|---|---|
| 19 | `level/forklift/flying_forklift.gd` (port 1) | Não chamar `randomize()` por instância (o `Main` já re-semeia no boot) | Re-semear o gerador global a cada empilhadeira é redundante e torna o sorteio dependente do relógio a cada spawn |
| 20 | `level/level.gd` (port 2) | `add_child(player, true)` como no robô, ou `spawn_robot` sem nome legível — uniformizar | Os dois spawns usam formas diferentes de `add_child`; funciona porque `name = str(id)` já é único |
| 21 | `level/level.gd` (port 2) | Expor `add_player`/`del_player` com parâmetro default via dois `#[func]` (ou remover o default) | gdext não tem parâmetro default; a v1 conecta por closure, o que muda a forma (não o efeito) da conexão |
| 22 | `menu/menu.gd` (port 3) | Tabela declarativa opção→valor em vez de 30 cadeias `if/elif` em `_on_settings_pressed`/`_on_apply_pressed` | ~250 linhas repetitivas; cada nova opção exige editar dois lugares |
| 23 | `menu/menu.gd` (port 3) | Declarar `replace_main_scene` com o parâmetro no original (já feito no port) e conectar `quit`/`replace_main_scene` no `Main` por tipo em vez de `has_signal` | Duck typing herdado; após a v1 todas as cenas são Rust |
| 24 | `main/main.gd` (port 4) | `change_scene_to_packed` chamado diretamente em vez de `call_deferred` por nome | Chamada por string ao próprio método; na v1 mantida por fidelidade |
