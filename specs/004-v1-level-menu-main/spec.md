# Feature Specification: Marco D — empilhadeira, level, menu e main (v1 raw port)

**Feature Branch**: `004-v1-level-menu-main` (trabalho na `main`, como nos Marcos A–C)

**Created**: 2026-09-15

**Status**: Draft

**Fase**: v1 — Raw Port (Princípio I da constituição v1.3.1). Nenhuma abstração, refatoração ou otimização; melhorias percebidas vão para `docs/v2-backlog.md`. Nenhum bug do upstream é conhecido nos quatro scripts; se um defeito objetivo surgir durante o port, a cláusula de correção conservadora se aplica (declarar em spec, isolar com comentário, mencionar no commit, registrar em `docs/upstream-bugs.md`) — caso contrário, nada muda.

**Input**: User description: "Portar para Rust a empilhadeira voadora, o level, o menu e o main do Godot TPS Demo, mantendo o jogo jogável e idêntico a cada script. Só settings.gd (autoload) fica em GDScript (marco E). Marco D de docs/port-order.md (itens 11–14)."

## Contexto

Penúltimo marco da v1. Os Marcos A–C (`specs/001`–`003`, commits até `b8f7124`) deixaram 5 scripts no original e entregaram `Player`, `PlayerInputSynchronizer`, `CameraNoiseShake`, `Bullet`, `Door`, `Blast`, `PartDisappear`, `Part` e `EnemyRobot` como classes nativas. Este marco porta o fluxo de jogo inteiro — boot (`main`), menu (`menu`), fase (`level`) e a empilhadeira decorativa (`flying_forklift`) — deixando apenas o autoload `settings.gd` em GDScript, acessado dinamicamente via `/root/Settings` (exceção única do Princípio II; backlog v2 item 1).

Nomes de classe conferidos (regra do `CLAUDE.md`): `FlyingForklift`, `Level`, `Menu`, `Main` não coincidem com nenhuma classe do engine nem com identificadores de topo dos `.gd` remanescentes (`menu.gd` tem `var main` em minúsculas — identificadores do GDScript diferenciam maiúsculas; e `menu.gd` já estará portado quando `Main` for registrado).

| # | Script original | Base do script | Tipo do node raiz | Cena | Linhas | Consumidores que permanecem no original |
|---|---|---|---|---|---|---|
| 1 | `level/forklift/flying_forklift.gd` | `Node3D` | **`CharacterBody3D`** (`flying_forklift.tscn:36`, filho `Collider`) | `level/forklift/flying_forklift.tscn` (instanciada por `level.tscn:8`) | 21 | nenhum (usa `Settings.config_file`) |
| 2 | `level/level.gd` | `Node3D` | `Node3D` (`level.tscn:40`) | `level/level.tscn` | 127 | `main.gd:30-31` (`has_signal("quit")` + conexão, até o port 4); `settings.gd` (`apply_graphics_settings` recebe o level como `scene_root`) |
| 3 | `menu/menu.gd` | `Node` | `Node` (`menu.tscn:103`) | `menu/menu.tscn` | 460 | `main.gd:32-33` (`has_signal("replace_main_scene")` + conexão, até o port 4) |
| 4 | `main/main.gd` | `Node` | `Node` (`main.tscn:5`, node chamado `main`) | `main/main.tscn` (cena principal, `project.godot:15`) | 33 | nenhum |

Referência de comportamento: o projeto original intocado em `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Empilhadeira voadora portada (Priority: P1)

As empilhadeiras que flutuam pelo level continuam iguais: cada uma escolhe aleatoriamente um dos modelos disponíveis ao nascer e, se o jogador desligou o sombreamento nas configurações, o farol dela deixa de projetar sombra.

**Why this priority**: Menor script do marco, folha (só lê `Settings`), instanciada pelo `level.tscn` — prova, antes do level, a regra de base declarada abaixo (script `Node3D` em node `CharacterBody3D`).

**Regra de base declarada nesta spec**: o script diz `extends Node3D`, mas o node raiz da cena é `CharacterBody3D` (com `CollisionShape3D` filho). Quando o `extends` do script é **ancestral** do tipo do node na cena, a classe portada MUST usar o tipo do node (`CharacterBody3D`): a troca de `type` não pode rebaixar o node nem descartar o corpo físico que a cena declara. O "mesma base" do Princípio II é lido como "mesma base efetiva do node".

**Independent Test**: `flying_forklift.tscn` e `level.tscn` headless sem erros novos; no jogo, as empilhadeiras aparecem com modelos variados (cor diferente entre elas) e, com sombras desligadas, o farol não projeta sombra.

**Acceptance Scenarios**:

1. **Given** a empilhadeira entra na cena com `rendering/shadow_mapping` falso nas configurações, **When** `ready` roda, **Then** o `SpotLight3D` fica com sombra desligada; com verdadeiro, fica como está na cena.
2. **Given** o primeiro filho da empilhadeira (`FlyingForkliftModel2`) tem *n* filhos (modelos), **When** `ready` roda, **Then** exatamente um deles — o de índice `floor(aleatório × n)` — fica visível e os demais invisíveis.
3. **Given** o level instancia várias empilhadeiras, **When** o jogo roda, **Then** cada uma sorteia independentemente (modelos variados entre instâncias).
4. **Given** o node raiz é `CharacterBody3D` na cena, **When** o `type` é trocado, **Then** o `Collider` filho continua válido e o corpo físico continua colidindo com o jogador e os robôs como antes.

---

### User Story 2 - Level portado (Priority: P2)

A fase continua a mesma: ao carregar, aplica as configurações gráficas e escolhe a técnica de iluminação global (SDFGI, VoxelGI ou lightmap) conforme as configurações; o servidor spawna quatro robôs e os jogadores em pontos aleatórios, e respawna cada robô 15 s depois que ele explode; ESC libera o mouse e volta ao menu.

**Why this priority**: Consome `EnemyRobot` (sinal `exploded`) e `Player` (`player_id`) com acesso **tipado** — fecha o grafo de dependências das classes já portadas — e emite `quit` para o `main` (ainda GDScript até o port 4, conectando por `has_signal`).

**Independent Test**: `level.tscn` headless sem erros novos (spawn dos 4 robôs e do jogador 1, conexão de `exploded` tipada); `main.tscn` headless ainda com o `main.gd` original chegando ao level; no jogo, iluminação conforme cada opção de GI, respawn de robôs, ESC volta ao menu.

**Acceptance Scenarios**:

1. **Given** o level entra na cena, **When** `ready` roda, **Then** `Settings.apply_graphics_settings(janela, ambiente do WorldEnvironment, level)` é chamado (dinâmico) e, conforme `rendering/gi_type` (0 = SDFGI, 1 = VoxelGI, senão lightmap), o setup correspondente roda.
2. **Given** `gi_type` = SDFGI, **When** o setup roda, **Then** `sdfgi_enabled` liga no ambiente, `VoxelGI` e `ReflectionProbes` ficam ocultos, um `LightmapGI` criado anteriormente é liberado, e `gi_quality` define a contagem de raios (2 = 96, 1 = 32, senão SDFGI desligado).
3. **Given** `gi_type` = VoxelGI, **When** o setup roda, **Then** SDFGI desliga, `VoxelGI` visível, `ReflectionProbes` ocultos, `LightmapGI` anterior liberado, e `gi_quality` define a qualidade do VoxelGI (2 = alta, 1 = baixa, senão `VoxelGI` oculto).
4. **Given** `gi_type` = lightmap, **When** o setup roda, **Then** SDFGI desliga, `VoxelGI` oculto, `ReflectionProbes` visíveis; se não existe, um `LightmapGI` chamado "LightmapGI" é criado com `light_data = res://level/level.lmbake` e adicionado ao level; se `gi_quality` = 0, o `LightmapGI` e os `ReflectionProbes` ficam ocultos.
5. **Given** o peer é servidor, **When** `ready` roda, **Then** um robô é spawnado em cada filho de `RobotSpawnpoints` (transform do ponto, `exploded` conectado a `_respawn_robot` com o ponto vinculado, filho de `SpawnedNodes` com nome legível), os pontos de `PlayerSpawnpoints` são embaralhados, o jogador 1 e cada peer já conectado recebem um ponto, e `peer_connected`/`peer_disconnected` ficam conectados a `add_player`/`del_player`.
6. **Given** um robô emite `exploded`, **When** 15 s passam, **Then** outro robô nasce no mesmo ponto.
7. **Given** `add_player(id)` sem ponto, **When** roda, **Then** um filho aleatório de `PlayerSpawnpoints` é escolhido; o jogador é instanciado com `name = str(id)`, `player_id = id` (acesso tipado a `Player`), transform do ponto, filho de `SpawnedNodes`.
8. **Given** `del_player(id)`, **When** `SpawnedNodes` tem um filho chamado `str(id)`, **Then** ele é removido; senão nada acontece.
9. **Given** o jogador pressiona a ação `quit` (ESC), **When** `_input` recebe o evento, **Then** o mouse fica visível e o sinal `quit` é emitido — o `main` volta ao menu.
10. **Given** `main.gd` continua no original até o port 4, **When** chama `node.has_signal("quit")` e conecta, **Then** resolve por nome sem edição.
11. **Given** o `MultiplayerSpawner` da cena (`level.tscn:73-75`), **When** robôs e jogadores são adicionados a `SpawnedNodes`, **Then** a replicação continua válida (API base).
12. **Given** o peer não é servidor, **When** `ready` roda, **Then** só as configurações gráficas e o GI são aplicados (ramo por leitura de código).

---

### User Story 3 - Menu portado (Priority: P3)

O menu continua igual: Play carrega o level com barra de progresso e troca a cena; Play Online mostra host/connect; Settings mostra 15 linhas de opções gráficas que refletem a configuração atual, com Apply gravando e aplicando e Cancel descartando; Quit fecha o jogo. Em modo headless, o menu hospeda automaticamente.

**Why this priority**: Maior script do marco (460 linhas, ~90 referências de UI, 15 grupos de botões, ~30 opções mapeadas para inteiros do engine). Consome só `Settings` (dinâmico) e emite `replace_main_scene` para o `main`.

**Independent Test**: `menu.tscn` headless sem erros novos (em headless o menu hospeda e carrega o level sozinho — exercita `_on_host_pressed`, `_on_play_pressed`, `_process`, `_on_loading_done_timer_timeout`); no jogo, cada botão do menu e cada linha de Settings se comporta como no original, com persistência em `user://settings.ini`.

**Acceptance Scenarios**:

1. **Given** o menu entra na cena, **When** `ready` roda, **Then** `Settings.apply_graphics_settings` é chamado; em headless, `_on_host_pressed` é agendado (deferred); `Play` recebe foco; sem MetalFX (driver ≠ "metal"), os botões `MetalFXSpatial` e `MetalFXTemporal` ficam ocultos; cada uma das 15 linhas de opção recebe um `ButtonGroup` próprio atribuído a todos os `BaseButton` filhos.
2. **Given** `Loading` está visível, **When** frames passam, **Then** o status do carregamento em thread de `res://level/level.tscn` é consultado: em progresso → barra = progresso × 100; carregado → barra = 100, processamento desligado, `DoneTimer` (0,5 s) iniciado; erro → mensagem impressa, `Main` visível, `Loading` oculto.
3. **Given** Play é pressionado, **When** `_on_play_pressed` roda, **Then** `Main` oculta, `Loading` visível, carregamento em thread do level requisitado (com sub-threads).
4. **Given** `DoneTimer` expira, **When** `_on_loading_done_timer_timeout` roda, **Then** o `multiplayer_peer` recebe o `peer` do menu e `replace_main_scene` é emitido com a cena carregada.
5. **Given** Settings é pressionado, **When** `_on_settings_pressed` roda, **Then** `Main` oculta, `Settings` visível, `Cancel` com foco, e em cada linha o botão correspondente ao valor atual do `config_file` fica pressionado, com os mapeamentos exatos de `menu.gd:175-311` (modo de janela: Windowed/Maximized → Windowed, Fullscreen, senão ExclusiveFullscreen; vsync 4 valores; max_fps 30/40/60/72/90/120/144/senão Unlimited; escala de resolução por comparação aproximada com 1/3, 1/2, 1/1,7, 1/1,5, 1/1,3, senão Native; filtro de escala 6 modos com fallback MetalFX temporal se suportado ou FSR2; tipo de GI 3; qualidade de GI 3; TAA; MSAA 4; AA de tela 3; sombras; SSAO −1/médio/alto; SSIL idem; bloom; névoa volumétrica).
6. **Given** Apply é pressionado, **When** `_on_apply_pressed` roda, **Then** `Main` visível, `Play` com foco, `Settings` oculto; cada opção é gravada no `config_file` conforme o botão pressionado, com os valores exatos de `menu.gd:318-441` (Unlimited grava 0; escalas gravam 1/3, 1/2, 1/1,7, 1/1,5, 1/1,3, 1,0; TAA/sombras/bloom/névoa gravam o booleano do botão "Enabled"; SSAO/SSIL desligados gravam −1); depois `Settings.apply_graphics_settings` e `Settings.save_settings` (dinâmicos).
7. **Given** Cancel ou Back é pressionado, **When** `_on_cancel_pressed` roda, **Then** `Main` visível, `Play` com foco, `Settings` e `Online` ocultos — nada gravado.
8. **Given** Play Online, **When** pressionado, **Then** `Online` visível e `Main` oculto; Host cria um servidor ENet na porta do `SpinBox` e chama `_on_play_pressed`; Connect cria um cliente ENet com endereço e porta e chama `_on_play_pressed`; ambos ocultam `Online` (ramos verificados por leitura de código e pelo host automático em headless).
9. **Given** Quit é pressionado, **When** roda, **Then** o jogo encerra.
10. **Given** os 10 `[connection]` de `menu.tscn:836-845`, **When** os botões/timer disparam, **Then** os handlers `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed` (Back **e** Cancel), `_on_apply_pressed`, `_on_loading_done_timer_timeout` resolvem por nome.
11. **Given** `main.gd` continua no original até o port 4, **When** chama `node.has_signal("replace_main_scene")` e conecta a um método de 1 argumento, **Then** o sinal é emitido com a `PackedScene` e chega.
12. **Given** todos os valores de enum do engine (modo de janela, vsync, modos de escala 3D, MSAA, AA de tela, qualidades de SSAO/SSIL) e do `settings.gd` (tipo/qualidade de GI), **When** lidos ou gravados no `config_file`, **Then** são os mesmos inteiros que o original e o `settings.gd` usam — `user://settings.ini` gravado pelo menu portado é lido pelo `settings.gd` e vice-versa.

---

### User Story 4 - Main portado (Priority: P4)

O boot do jogo continua igual: a cena principal desliga o relay de multiplayer, limita a 60 fps em headless, aplica o modo de janela salvo e vai ao menu; trocar de cena remove a anterior e conecta os sinais `quit` (→ voltar ao menu) e `replace_main_scene` (→ trocar para a cena recebida) do novo node, se ele os tiver.

**Why this priority**: Último e menor; depende de `Level` e `Menu` terem os sinais como classes nativas (o `has_signal` continua dinâmico — duck typing do original). Fecha o fluxo completo em código portado.

**Independent Test**: `main.tscn` headless (boot → menu → host automático → level) sem erros novos; no jogo, Play → level, ESC → menu, Quit encerra.

**Acceptance Scenarios**:

1. **Given** `main.tscn` inicia, **When** `ready` roda, **Then** `server_relay` desliga; em headless `max_fps = 60`; o modo da janela recebe `video/display_mode` do `config_file` (dinâmico); `go_to_main_menu` roda.
2. **Given** `go_to_main_menu`, **When** roda, **Then** `menu.tscn` é carregado, o `multiplayer_peer` atual é fechado e substituído por um `OfflineMultiplayerPeer`, e `change_scene_to_packed(menu)` roda.
3. **Given** `replace_main_scene(cena)` é chamado (pelo sinal do menu), **When** roda, **Then** `change_scene_to_packed` é chamado **em deferred, pelo nome** — por isso `change_scene_to_packed` MUST ser exposto com esse nome.
4. **Given** `change_scene_to_packed(cena)`, **When** roda, **Then** a cena é instanciada, todos os filhos atuais são removidos e liberados, o novo node é adicionado; se ele tem sinal `quit` (`has_signal` — duck typing preservado), conecta a `go_to_main_menu`; se tem `replace_main_scene`, conecta a `replace_main_scene`.
5. **Given** `Level` e `Menu` já são classes nativas com esses sinais, **When** o `has_signal` dinâmico roda, **Then** encontra ambos (conferido em headless).
6. **Given** o node raiz de `main.tscn` chama-se `main` (minúsculo), **When** o `type` vira `Main`, **Then** o nome do node não muda (nome de node ≠ nome de classe).

---

### Edge Cases

- **Empilhadeira: base do script ≠ tipo do node** — regra declarada na US1: prevalece o tipo do node (`CharacterBody3D`). É a primeira vez que isso ocorre no projeto; fica registrado para os revisores.
- **Empilhadeira: sorteio com `floor(aleatório × n)`** — para *n* = 3, cada modelo tem 1/3; `randomize()` é chamado antes (como no original — o gerador global é re-semeado a cada empilhadeira; preservar).
- **Level: `LightmapGI` criado em runtime** — o node só existe após um `setup_lightmapgi`; `setup_sdfgi`/`setup_voxelgi` liberam-no se existir (o level é recriado a cada Play, então na prática nasce nulo).
- **Level: `add_player` conectado a `peer_connected(id)`** — o sinal passa 1 argumento; o parâmetro `spawn_point` default nulo cobre o caso (o método MUST aceitar a chamada com 1 argumento vindo do sinal e com 2 vindo do `ready`).
- **Level: `_respawn_robot` após o level ser destruído** — o timer de 15 s é criado pela árvore; se o level já saiu (ESC → menu), o callback não deve tocar um level liberado (comportamento equivalente ao do original, cujo `await` num objeto liberado é descartado).
- **Level: `spawn_robot(spawn_point)` sem tipo** — o original recebe `Variant`; só usa `.transform`; o port tipa como `Node3D` (o que a cena garante).
- **Menu: `signal replace_main_scene` declarado sem parâmetros** e emitido com 1 argumento (`emit_signal("replace_main_scene", cena)`). O port declara o sinal **com** o parâmetro (`PackedScene`), porque a emissão tipada exige — o consumidor (`main.gd`/`Main`) sempre recebeu 1 argumento; nada observável muda. Registrado aqui por transparência, não é correção de bug.
- **Menu: `_on_host_pressed` diferido em headless** — em `--headless`, o menu hospeda e carrega o level sozinho: a validação headless de `menu.tscn` e `main.tscn` exercita o fluxo Play inteiro (loading, timer, `replace_main_scene`).
- **Menu: carregamento em thread** — `load_threaded_get_status` recebe um array para progresso; o port usa a forma da API que devolve o progresso (mesmo valor).
- **Menu: `metalfx_supported`** — falso fora de macOS; os dois botões MetalFX ficam ocultos e o fallback do filtro de escala é FSR2 (original).
- **Menu: `_make_button_group`** — pula filhos que não são `BaseButton` (rótulos das linhas).
- **Main: `remove_child` + `queue_free`** dos filhos atuais, na ordem do original (remove antes de liberar).
- **Main: `randomize()`** — re-semeia o gerador global no boot (além das chamadas do level e das empilhadeiras).
- **Settings**: as 3 chamadas dinâmicas ao autoload (`config_file` get/set, `apply_graphics_settings(window, environment, scene_root)`, `save_settings()`) são a exceção do Princípio II; `settings.gd` fica intacto e é o Marco E.

## Requirements *(mandatory)*

### Functional Requirements

**Comportamento — empilhadeira (US1)**

- **FR-001**: A empilhadeira MUST ter base `CharacterBody3D` (tipo do node raiz em `flying_forklift.tscn:36`; regra "tipo do node prevalece quando o `extends` do script é seu ancestral") e referência ao `SpotLight3D`.
- **FR-002**: Ao entrar na cena MUST desligar a sombra do `SpotLight3D` se `Settings.config_file` `rendering/shadow_mapping` for falso; re-semear o gerador aleatório; e, entre os *n* filhos do primeiro filho, deixar visível só o de índice `floor(aleatório × n)`.

**Comportamento — level (US2)**

- **FR-003**: O level MUST ter base `Node3D`, sinal `quit` (sem argumentos) e referências a `WorldEnvironment`, `RobotSpawnpoints`, `PlayerSpawnpoints`, `SpawnedNodes`; `lightmap_gi` inicia nulo.
- **FR-004**: Ao entrar na cena MUST chamar `Settings.apply_graphics_settings(janela, ambiente, level)` (dinâmico) e escolher `setup_sdfgi` (`gi_type` = 0), `setup_voxelgi` (1) ou `setup_lightmapgi` (demais), lendo `rendering/gi_type` do `config_file` como inteiro (os valores do enum `GIType` do `settings.gd`; literais inteiros na v1 — backlog item 1).
- **FR-005**: `setup_sdfgi`, `setup_voxelgi` e `setup_lightmapgi` MUST reproduzir `level.gd:45-93` (visibilidade de `VoxelGI`/`ReflectionProbes`, liberação/criação do `LightmapGI` com `light_data = res://level/level.lmbake` e nome "LightmapGI", chamadas ao servidor de renderização para contagem de raios 96/32 e qualidade alta/baixa do VoxelGI, com `gi_quality` lido como inteiro 0/1/2).
- **FR-006**: No servidor, `ready` MUST spawnar um robô por filho de `RobotSpawnpoints`, re-semear, embaralhar os filhos de `PlayerSpawnpoints`, `add_player(1, primeiro)` e `add_player(id, próximo)` para cada peer, e conectar `peer_connected` → `add_player` e `peer_disconnected` → `del_player`.
- **FR-007**: `spawn_robot(spawn_point)` MUST instanciar `red_robot.tscn` com acesso **tipado** a `EnemyRobot`, copiar o `transform` do ponto, conectar `exploded` a `_respawn_robot` com o ponto vinculado, e adicionar a `SpawnedNodes` com nome legível; `_respawn_robot(spawn_point)` MUST esperar 15 s e chamar `spawn_robot`.
- **FR-008**: `add_player(id, spawn_point = nulo)` MUST sortear um filho de `PlayerSpawnpoints` quando não recebe ponto, instanciar `player.tscn` com acesso **tipado** a `Player`, definir `name = str(id)`, `player_id = id`, copiar o `transform` e adicionar a `SpawnedNodes`; `del_player(id)` MUST remover o filho `str(id)` de `SpawnedNodes` se existir. Ambos MUST ser invocados pelos sinais do `MultiplayerAPI`: `peer_connected(id)` → `add_player(id)` sem ponto (sorteio) e `peer_disconnected(id)` → `del_player(id)`. A forma da conexão (por nome, ou callable tipado que chama o método com o ponto ausente — a plataforma de destino não tem parâmetro default em métodos expostos) é decisão do plan; o comportamento observável é o mesmo.
- **FR-009**: `_input` MUST, na ação `quit`, tornar o mouse visível e emitir `quit`.
- **FR-010**: `res://enemies/red_robot/red_robot.tscn` e `res://player/player.tscn` MUST ser carregados por `load` no ponto de uso (padrão `preload` → `load` dos marcos anteriores); os métodos `setup_*` e `_respawn_robot`/`spawn_robot` internos MUST manter os nomes.

**Comportamento — menu (US3)**

- **FR-011**: O menu MUST ter base `Node`, sinal `replace_main_scene(cena: PackedScene)`, constante `LEVEL_PATH = "res://level/level.tscn"`, estado `peer` (inicial `OfflineMultiplayerPeer`) e `metalfx_supported` (driver de renderização atual == "metal"), e as referências de `menu.gd:11-104` com os mesmos caminhos de node.
- **FR-012**: Ao entrar na cena MUST reproduzir `menu.gd:107-125` (aplicar configurações; `_on_host_pressed` diferido em headless; foco em Play; ocultar MetalFX quando não suportado; `_make_button_group` nas 15 linhas).
- **FR-013**: Por frame MUST reproduzir `menu.gd:128-141` (status do carregamento em thread → barra, timer, ou erro impresso + voltar ao Main).
- **FR-014**: Os 9 handlers `_on_play_pressed`, `_on_play_online_pressed`, `_on_settings_pressed`, `_on_quit_pressed`, `_on_host_pressed`, `_on_connect_pressed`, `_on_cancel_pressed`, `_on_apply_pressed`, `_on_loading_done_timer_timeout` MUST existir com esses nomes (10 conexões em `menu.tscn:836-845`) e os efeitos de `menu.gd:153-460`.
- **FR-015**: `_on_settings_pressed` e `_on_apply_pressed` MUST usar exatamente os valores e as ordens de comparação de `menu.gd:175-311` e `318-441` (inteiros dos enums do engine — modo de janela, vsync, modos de escala 3D, MSAA, AA de tela, qualidades de SSAO/SSIL — e do `settings.gd` — tipo/qualidade de GI; `−1` para SSAO/SSIL desligados; `0` para fps ilimitado; escalas `1/3`, `1/2`, `1/1,7`, `1/1,5`, `1/1,3`, `1,0` com comparação aproximada na leitura).
- **FR-016**: `_on_host_pressed`/`_on_connect_pressed` MUST criar um `ENetMultiplayerPeer` (servidor na porta do `SpinBox`; cliente com endereço e porta), chamar `_on_play_pressed` e ocultar `Online`; `_on_loading_done_timer_timeout` MUST atribuir `peer` ao `multiplayer_peer` e emitir `replace_main_scene` com o level carregado.
- **FR-017**: `_make_button_group` MUST ser interno (não consumido externamente) e atribuir um `ButtonGroup` novo a cada `BaseButton` filho da linha, pulando os demais filhos.

**Comportamento — main (US4)**

- **FR-018**: O main MUST ter base `Node` e reproduzir `main.gd:4-10` em `ready` (`server_relay = false`; `max_fps = 60` em headless; re-semear; modo da janela = `video/display_mode` do `config_file`; `go_to_main_menu`).
- **FR-019**: `go_to_main_menu`, `replace_main_scene(cena)` e `change_scene_to_packed(cena)` MUST existir com esses nomes; `replace_main_scene` MUST chamar `change_scene_to_packed` em deferred **pelo nome** (como o original), o que exige `change_scene_to_packed` exposto.
- **FR-020**: `change_scene_to_packed` MUST instanciar, remover e liberar todos os filhos atuais, adicionar o novo node e conectar `quit` → `go_to_main_menu` e `replace_main_scene` → `replace_main_scene` **se** o node tiver esses sinais (`has_signal` — duck typing do original, preservado).

**Ciclo de porte — comuns aos quatro (Princípio II)**

- **FR-021**: Cada script MUST virar exatamente uma classe nativa: `FlyingForklift: CharacterBody3D`, `Level: Node3D`, `Menu: Node`, `Main: Node`. Os nomes MUST ser conferidos contra classes do engine e identificadores de topo dos `.gd` remanescentes antes de cada port (regra do `CLAUDE.md`); colisão → parar.
- **FR-022**: O vínculo MUST ser por troca de `type` na raiz de cada cena (`flying_forklift.tscn:36`, `level.tscn:40`, `menu.tscn:103`, `main.tscn:5`), com remoção de `script` e do `ext_resource` órfão; nenhum `.gd` ponte; nenhum node renomeado (inclusive `main`).
- **FR-023**: `.gd` e `.gd.uid` MUST ser apagados no mesmo commit do port.
- **FR-024**: Nomes de sinais, métodos expostos e handlers MUST ser idênticos ao GDScript: `quit`, `replace_main_scene`, os 9 handlers do menu, `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed`, `add_player`, `del_player`, `_respawn_robot`, `spawn_robot`. Conferidos contra `menu.tscn:836-845`, `main.gd:30-33` e os sinais do `MultiplayerAPI`.
- **FR-025**: O código portado MUST NOT chamar API customizada de GDScript, exceto o autoload `Settings` (`config_file` get/set, `apply_graphics_settings`, `save_settings` — dinâmicos via `/root/Settings`, exceção do Princípio II, backlog item 1). Os acessos a `EnemyRobot.exploded` e `Player.player_id` MUST ser tipados. O `has_signal` do `main` é duck typing do original (permitido).
- **FR-026**: Cada alteração de código MUST ser seguida de build de debug sem warnings novos.
- **FR-027**: Cada port MUST ser validado em headless: import com carregamento da extensão + `flying_forklift.tscn`, `level.tscn`, `menu.tscn`, `main.tscn` (as que a story afeta) sem erros novos além da baseline (os 3 do `CLAUDE.md`).
- **FR-028**: Um commit por script, ordem 1 → 2 → 3 → 4; o jogo MUST ficar jogável após cada commit (o `main.gd` original conecta os sinais das classes nativas por `has_signal` até o port 4).
- **FR-029**: `settings.gd` MUST permanecer byte a byte intacto; melhorias percebidas MUST ir para `docs/v2-backlog.md` no mesmo commit; nenhuma correção de bug é prevista (defeito objetivo → parar, declarar em spec, então os 4 requisitos da cláusula).

### Key Entities

- **FlyingForklift**: referência `spot_light`; sem contrato externo além da cena (nenhum método/propriedade consumido por outros scripts).
- **Level (contrato consumido por `main.gd`/`Main`, `settings.gd`, `MultiplayerAPI`)**: sinal `quit`; métodos `add_player(id, spawn_point = nulo)`, `del_player(id)` (conectados a sinais do `MultiplayerAPI`), `spawn_robot`, `_respawn_robot`, `setup_sdfgi/voxelgi/lightmapgi`; estado `lightmap_gi`; referências `WorldEnvironment`, `RobotSpawnpoints`, `PlayerSpawnpoints`, `SpawnedNodes`, `VoxelGI`, `ReflectionProbes`.
- **Menu (contrato consumido por `menu.tscn` e `main.gd`/`Main`)**: sinal `replace_main_scene(cena)`; 9 handlers (10 conexões); estado `peer`, `metalfx_supported`; ~90 referências de UI (`menu.gd:11-104`); método interno `_make_button_group`.
- **Main (raiz do jogo)**: métodos `go_to_main_menu`, `replace_main_scene`, `change_scene_to_packed` (este chamado por nome em deferred).
- **Configurações (`user://settings.ini`, via `settings.gd`)**: seções `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) e `rendering` (`gi_type`, `gi_quality`, `taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`) — lidas/gravadas com os mesmos inteiros/booleanos/floats do original.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ao final do marco o projeto contém exatamente **1** arquivo `.gd` (`menu/settings.gd`, e 1 `.uid`), byte a byte idêntico ao estado anterior ao marco.
- **SC-002**: O jogo completo — boot → menu (Play, Play Online, Settings com as 15 linhas refletindo e gravando cada opção, Quit) → loading com barra → level jogável (jogador, robôs, respawn 15 s, empilhadeiras com modelos variados, iluminação conforme o GI escolhido) → ESC volta ao menu — é indistinguível do original em `../oxide_godot_origins/` numa comparação lado a lado feita pelo usuário, incluindo aplicar/cancelar/reabrir Settings e conferir a persistência em `user://settings.ini`.
- **SC-003**: Para cada um dos 4 ports, a validação headless (import + cenas afetadas) reporta zero erros novos além dos 3 catalogados; `main.tscn` headless percorre boot → menu → host automático → level sem erro.
- **SC-004**: Após cada um dos 4 commits o jogo é jogável de ponta a ponta.
- **SC-005**: O histórico do marco tem exatamente 4 commits `Port …` e nenhum toca `settings.gd`.
- **SC-006**: Build de debug sem nenhum warning novo em todos os commits.
- **SC-007**: Nenhuma linha portada introduz abstração, refatoração, otimização ou correção; `docs/upstream-bugs.md` permanece com 2 entradas.
- **SC-008**: `settings.gd` continua funcionando com as cenas portadas: `apply_graphics_settings` recebe janela/ambiente/level dos dois chamadores, e o `settings.ini` gravado pelo menu portado é lido sem alteração de valores.

## Assumptions

- Fase v1 (constituição v1.3.1); nenhuma correção de bug prevista.
- Validação visual (SC-002) pelo usuário, incluindo o menu de configurações completo; validação automatizada exclusivamente headless. O revisor pode rodar um harness de paridade fora do repositório.
- Validação single-player: o peer local é servidor (`OfflineMultiplayerPeer`); host/connect reais e ramos "cliente" (FR-006, FR-016) são verificados por leitura de código e pelo host automático em headless.
- Fora de escopo: `settings.gd` (Marco E); acesso tipado ao `Settings`; qualquer melhoria; multiplayer real com dois peers.
- Regra de base declarada (US1): quando o `extends` do script é ancestral do tipo do node raiz, a classe usa o tipo do node — `FlyingForklift: CharacterBody3D`.
- `signal replace_main_scene` do menu passa a declarar o parâmetro `PackedScene` que o original já emite; não é correção, é exigência da emissão tipada (documentado em Edge Cases).
- Quirks preservados propositalmente (candidatos ao backlog v2, não a correção): `randomize()` chamado em três lugares (main, level, cada empilhadeira); `spawn_robot(spawn_point)` sem tipo no original; `add_player` sem `force_readable_name` (o nome já é `str(id)`) enquanto `spawn_robot` usa; `_on_apply_pressed` com cadeias `if/elif` que não gravam nada se nenhum botão da linha estiver pressionado; leitura de `gi_type`/`gi_quality` como inteiros literais em vez do enum do `settings.gd`.
- O node raiz de `main.tscn` chama-se `main` (minúsculo) e continua assim; a classe chama-se `Main`.
