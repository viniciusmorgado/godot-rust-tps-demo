# Feature Specification: Marco E — autoload Settings (v1 raw port, último script)

**Feature Branch**: `005-v1-settings-autoload` (trabalho na `main`, como nos Marcos A–D)

**Created**: 2026-09-16

**Status**: Draft

**Fase**: v1 — Raw Port (Princípio I da constituição v1.3.1). Nenhuma abstração, refatoração ou otimização; melhorias percebidas vão para `docs/v2-backlog.md`. Nenhum bug do upstream é conhecido em `settings.gd`; dois **quirks** (SSAO, ver Edge Cases) são preservados e registrados no backlog — não são correção. Se um defeito objetivo surgir durante o port, a cláusula de correção conservadora se aplica (declarar em spec, isolar com comentário, mencionar no commit, registrar em `docs/upstream-bugs.md`) — caso contrário, nada muda.

**Input**: User description: "Portar o autoload Settings (menu/settings.gd, 107 l., Node) para Rust, concluindo a v1 com zero .gd no projeto e o jogo inteiro em Rust. Vínculo especial de autoload: cena mínima menu/settings.tscn + troca da linha do project.godot. Os 5 consumidores Rust (bullet, flying_forklift, level, menu, main_scene) continuam acessando /root/Settings dinamicamente sem edição. Marco E, item 15 de docs/port-order.md."

## Contexto

Último marco da v1. Os Marcos A–D (`specs/001`–`004`, commits até `6f2fc7f`) portaram 14 dos 15 scripts; `menu/settings.gd` é o **único** `.gd` do projeto (com seu `.gd.uid`). Ele é o autoload `Settings` (`project.godot:25`, `[autoload] Settings="*res://menu/settings.gd"`), acessível em `/root/Settings`, e não está attached a nenhuma cena.

Consumidores: **nenhum GDScript resta**. Os 5 módulos Rust acessam o autoload **dinamicamente** (exceção documentada do Princípio II, constituição l.79; backlog v2 item 1) em 10 `get_node_as("/root/Settings")` (12 acessos): `flying_forklift.rs:19` (`get("config_file")`), `bullet.rs:72` (`get("config_file")`), `level.rs:43,116,144,173` (`call("apply_graphics_settings", …)` + 4× `get("config_file")`), `menu.rs:208,308,479` (`call("apply_graphics_settings", …)` ×2, `get("config_file")` ×2, `call("save_settings")`), `main_scene.rs:29` (`get("config_file")`). Esses acessos MUST continuar resolvendo por nome contra a classe nova, **sem editar os 5 módulos**. O acesso tipado (`Gd<Settings>`) fica para a v2 (backlog item 1) — a exceção se encerra no código novo (a classe nasce em Rust), não nos consumidores.

Nome de classe conferido (regra do `CLAUDE.md`): `Settings` não é classe do engine (existem `ProjectSettings`, `EditorSettings`, `LabelSettings` — nomes distintos) e não há `.gd` remanescente além do próprio script a ser apagado.

| # | Script original | Base | Vínculo hoje | Vínculo após o port | Linhas | Consumidores |
|---|---|---|---|---|---|---|
| 1 | `menu/settings.gd` | `Node` | autoload por **script** (`project.godot:25`) | autoload por **cena**: `menu/settings.tscn` nova (raiz `[node name="Settings" type="Settings"]`) + `project.godot:25` → `Settings="*res://menu/settings.tscn"` | 107 | 5 módulos Rust, dinâmicos, intocados |

Referência de comportamento: o projeto original intocado em `../oxide_godot_origins/` (`menu/settings.gd` idêntico ao do repo).

### Vínculo por tipo de um autoload (regra declarada nesta spec)

O Princípio II exige vínculo por troca de `type` na cena, sem `.gd` ponte. Um autoload é registrado como **script ou cena**; uma classe nativa não pode ser autoload direto. O equivalente exato da "troca de `type`" para um autoload é: (1) criar uma cena mínima cuja raiz tem `type` = classe portada e `name` = chave do autoload; (2) apontar a entrada `[autoload]` para essa cena; (3) apagar `.gd` + `.gd.uid` no mesmo commit. O node continua em `/root/Settings` (o nome vem da chave do autoload, não da cena), com `*` (singleton habilitado) preservado. Esta é a **primeira e única edição de `project.godot` na v1** — exatamente uma linha; nada mais no arquivo muda.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Autoload Settings em Rust: carga e persistência (Priority: P1)

O jogo continua a ler e gravar as configurações exatamente como antes: ao iniciar, o autoload carrega `user://settings.ini` (se existir) e completa em memória tudo o que faltar com os padrões, de modo que nenhum consumidor precise de valor default; Apply no menu grava o arquivo. Os cinco módulos já portados continuam funcionando sem nenhuma alteração.

**Why this priority**: É o contrato do autoload (propriedade `config_file`, métodos `load_settings`/`save_settings`) do qual todo o jogo depende; sem ele nada mais liga. Fecha a v1: zero `.gd`.

**Independent Test**: `main.tscn` headless (boot completo: autoload em cena → `Main` lê `display_mode` → `Menu` aplica → host automático → `Level` aplica) sem erros novos; `menu.tscn` e `level.tscn` headless idem; no jogo, com `user://settings.ini` removido, o boot ocorre com os padrões e o menu de Settings mostra os padrões; após Apply o arquivo existe com os mesmos nomes, ordem e valores do original; reiniciar o jogo preserva as escolhas.

**Acceptance Scenarios**:

1. **Given** `project.godot` registra `Settings="*res://menu/settings.tscn"` e a cena tem só a raiz `Settings` do tipo portado, **When** o jogo inicia, **Then** `/root/Settings` existe, é a classe portada, e os 10 acessos dinâmicos dos 5 módulos (`get("config_file")`, `call("apply_graphics_settings", janela, ambiente, raiz)`, `call("save_settings")`) resolvem sem erro (`Invalid get`/`Invalid call` ausentes em headless).
2. **Given** o autoload entra na árvore, **When** `ready` roda, **Then** `load_settings` roda: `config_file.load("user://settings.ini")` (erro ignorado se o arquivo não existir) e, para cada (seção, chave) dos padrões que **não** exista no arquivo, o padrão é gravado **em memória** — na ordem do original: `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) e `rendering` (`taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `gi_type`, `gi_quality`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`).
3. **Given** `user://settings.ini` não existe, **When** o jogo inicia, **Then** **nenhum arquivo é criado** pela carga (o original não salva em `load_settings`); o arquivo só passa a existir após `save_settings` (Apply no menu) — e então contém as 15 chaves com os padrões (ou os valores escolhidos), com os mesmos tipos do original (`max_fps` inteiro `0`, `resolution_scale` real `1.0`, booleanos, inteiros de enum).
4. **Given** o arquivo existe com valores parciais (chaves ausentes), **When** `load_settings` roda, **Then** as chaves presentes são preservadas e só as ausentes recebem padrão.
5. **Given** os padrões: `video/display_mode` = tela cheia exclusiva; `vsync` = habilitado; `max_fps` = 0; `resolution_scale` = 1.0; `scale_filter` = MetalFX temporal se `metalfx_supported` senão FSR2; `rendering/taa` = falso; `msaa` = desligado; `screen_space_aa` = desligado; `shadow_mapping` = verdadeiro; `gi_type` = VoxelGI (1); `gi_quality` = baixa (1); `ssao_quality` = média; `ssil_quality` = −1; `bloom` = verdadeiro; `volumetric_fog` = verdadeiro, **When** gravados/lidos, **Then** são os **mesmos inteiros/booleanos/reais** que o original grava — o `settings.ini` gravado pelo `settings.gd` original é lido pelo port sem alteração e vice-versa (compatibilidade do arquivo do usuário).
6. **Given** `metalfx_supported`, **When** o autoload é construído, **Then** vale (nome do driver de renderização atual == "metal") — falso nesta plataforma; o padrão de `scale_filter` é FSR2.
7. **Given** `save_settings` é chamado pelo menu (dinâmico), **When** roda, **Then** `config_file.save("user://settings.ini")` grava o estado atual.
8. **Given** o `config_file` é uma propriedade exposta com esse nome, **When** um consumidor faz `get("config_file")`, **Then** recebe o **mesmo** objeto que o autoload usa internamente (alterações do menu via `set_value` são vistas por `save_settings` e `apply_graphics_settings`).
9. **Given** `settings.gd` e `settings.gd.uid` são apagados no commit, **When** o projeto é importado, **Then** nenhuma referência a `res://menu/settings.gd` ou ao uid `uid://b04fekxdgdq0k` resta fora de `.godot/`; o projeto tem **zero** `.gd` e **zero** `.gd.uid`.

---

### User Story 2 - Aplicação das configurações gráficas (Priority: P2)

Ao entrar no menu ou no level, as configurações são aplicadas exatamente como antes: modo de janela, vsync, limite de fps, escala e filtro de resolução, TAA, MSAA, AA de tela, sombras (desligadas em todas as luzes da cena quando a opção está desligada), SSAO, SSIL, bloom e névoa volumétrica — incluindo os dois quirks do original no SSAO.

**Why this priority**: É o método de 3 argumentos chamado dinamicamente por `Level` e `Menu`; determina a aparência do jogo e é onde a fidelidade é mais visível.

**Independent Test**: `level.tscn`/`menu.tscn`/`main.tscn` headless sem erro na chamada de 3 argumentos; no jogo, alternar cada opção em Settings → Apply produz o mesmo efeito visual que no original (lado a lado), incluindo sombras desligadas e SSAO/SSIL.

**Acceptance Scenarios**:

1. **Given** `apply_graphics_settings(window, environment, scene_root)` é chamado por nome com 3 argumentos, **When** roda, **Then** o modo de janela recebe `video/display_mode` — aplicado à **janela do próprio autoload** (`get_window()`), não ao parâmetro `window` (preservar); o modo de vsync do servidor de display recebe `video/vsync`; o limite de fps do engine recebe `video/max_fps`.
2. **Given** o parâmetro `window`, **When** roda, **Then** `scaling_3d_scale` = `video/resolution_scale`, `scaling_3d_mode` = `video/scale_filter`, `use_taa` = `rendering/taa`, `msaa_3d` = `rendering/msaa`, `screen_space_aa` = `rendering/screen_space_aa`.
3. **Given** `rendering/shadow_mapping` falso, **When** roda, **Then** `scene_root.propagate_call("set", ["shadow_enabled", false])` desliga a sombra de todas as luzes da cena (API base do engine; chamada por nome que o original já faz); com verdadeiro nada é feito (as sombras **não** são religadas — limitação `FIXME` do original, preservada).
4. **Given** `rendering/ssao_quality`, **When** roda, **Then** se for −1 → `environment.ssao_enabled = false` **e nada mais** (correção conservadora do bug do upstream, FR-020–FR-024: no original o `if` seguinte não é `elif` e religava o SSAO); senão, se for média → `ssao_enabled = true` e o servidor de renderização recebe qualidade **alta**, `half_size` falso, 0.5, 2, 50, 300 (**quirk preservado**: "média" aplica alta — intenção ambígua, backlog v2); senão → `ssao_enabled = true` e qualidade **média**, `half_size` verdadeiro, 0.5, 2, 50, 300.
5. **Given** `rendering/ssil_quality` (`if`/`elif`/`else`, correto): **When** roda, **Then** −1 → `ssil_enabled = false`; média → `true` + qualidade média, `half_size` falso, 0.5, 2, 50, 300; senão → `true` + qualidade alta, `half_size` verdadeiro, 0.5, 2, 50, 300.
6. **Given** `rendering/bloom` e `rendering/volumetric_fog`, **When** roda, **Then** `environment.glow_enabled` e `environment.volumetric_fog_enabled` recebem os booleanos.
7. **Given** o original lê os valores por `Settings.config_file` (o próprio autoload via nome global), **When** o port lê por sua própria propriedade `config_file`, **Then** é o mesmo objeto — nada observável muda.
8. **Given** o menu chama `apply_graphics_settings` duas vezes (ao entrar e no Apply) e o level uma vez, **When** ocorrem, **Then** cada chamada aplica o estado corrente do `config_file` (sem cache).

---

### User Story 3 - Alternar tela cheia (Priority: P3)

F11 ou Alt+Enter alternam entre tela cheia exclusiva e janela em qualquer ponto do jogo, como antes.

**Why this priority**: Único input tratado pelo autoload; independente do resto, verificável em segundos pelo usuário.

**Independent Test**: No jogo, F11 alterna tela cheia exclusiva ↔ janela; Alt+Enter idem; em headless não é exercitado (sem janela).

**Acceptance Scenarios**:

1. **Given** a ação `toggle_fullscreen` (`project.godot`, F11 e Alt+Enter — intocada) é pressionada, **When** `_input` recebe o evento, **Then** se a janela **não** está em tela cheia nem em tela cheia exclusiva → modo = tela cheia exclusiva; senão → modo = janela; e o evento é marcado como tratado no viewport.
2. **Given** qualquer outro evento, **When** `_input` recebe, **Then** nada acontece.

---

### Edge Cases

- **Autoload → cena**: única forma de vincular classe nativa como autoload; `settings.tscn` tem exatamente um node (`name="Settings"`, `type="Settings"`), sem script, sem `ext_resource`. O nome em `/root/Settings` vem da chave do autoload. `project.godot` muda em **uma** linha (`:25`); `run/main_scene`, ações de input e tudo o mais ficam intocados (conferido por diff).
- **Cena do autoload rodada isoladamente**: `godot --path . menu/settings.tscn` instanciaria o autoload **e** a cena (dois nodes `Settings` sob `/root`, um renomeado pelo engine) — não é um cenário do jogo; a validação headless usa `main.tscn`, `menu.tscn`, `level.tscn`, nunca `settings.tscn` sozinha.
- **Enums `GIType`/`GIQuality`**: eram consumidos como `Settings.GIType.X` por scripts já portados que hoje usam literais inteiros (0/1/2). Nenhum consumidor os lê por nome; a spec **não exige** expô-los — expor como constantes inteiras da classe (com os mesmos valores) ou manter só os inteiros nos padrões é decisão do plan. Os valores gravados são 0/1/2.
- **`load_settings` não salva**: o arquivo nasce só no primeiro `save_settings` (Apply). Em headless, `main.tscn` nunca pressiona Apply → o boot não cria o arquivo (comportamento do original). Não é defeito.
- **`config_file.load` em arquivo ausente**: o erro de retorno é ignorado (como o original); nenhuma mensagem de erro do engine é esperada (a `ConfigFile.load` não imprime erro para arquivo inexistente).
- **Padrões condicionais**: `scale_filter` depende de `metalfx_supported`, avaliado uma vez na construção do autoload (nome do driver == "metal"). Nesta plataforma: FSR2.
- **Ordem e tipos no `.ini`**: a ordem das seções/chaves do arquivo gravado segue a ordem de inserção dos padrões (dicionário do original) — o arquivo gravado pelo port MUST ter a mesma ordem e os mesmos tipos (`0` inteiro, `1.0` real, `true/false`, inteiros de enum) do original, para o `settings.ini` existente do usuário continuar válido e idêntico byte a byte após um Apply sem mudanças.
- **`apply_graphics_settings`: janela própria vs. parâmetro** — o modo de janela vai para `get_window()` do autoload (a janela raiz), e escala/filtro/TAA/MSAA/AA vão para o parâmetro `window` (na prática a mesma janela raiz). Preservar a distinção.
- **SSAO — bug e quirk** (US2 cenário 4): (1) `if == -1` seguido de `if == média / else` **sem `elif`** → escolher "SSAO: Disabled" no menu não desliga o SSAO (o `else` religa em média + `half_size`). Intenção inequívoca (desligar) contradita pelo resultado, e o bloco SSIL logo abaixo — idêntico em estrutura — usa `elif` corretamente: é **bug**, corrigido sob a cláusula da constituição (FR-020–FR-024). (2) "média" aplica qualidade **alta** sem `half_size`, e o `else` aplica média com `half_size`: intenção ambígua (pode ser troca de nomes deliberada) → **preservado**, backlog v2 (item 25).
- **`FIXME` do original (sombras não religadas no menu)**: comentário do upstream em `settings.gd:83-85`; limitação conhecida e preservada (não é defeito objetivo desta feature).
- **`propagate_call("set", …)`**: chamada por nome à API base do engine (não a script customizado) — permitida, é o que o original faz.
- **Headless**: modo de janela, vsync e fullscreen não têm efeito no servidor de display headless; as chamadas MUST ocorrer sem erro.
- **Auto-referência `Settings.config_file`**: dentro de `apply_graphics_settings` o original usa o nome global do autoload para ler o próprio `config_file`; o port usa a própria propriedade — mesmo objeto.

## Requirements *(mandatory)*

### Functional Requirements

**Comportamento (US1)**

- **FR-001**: O autoload MUST virar exatamente uma classe nativa `Settings` com base `Node`, contendo: constante `CONFIG_FILE_PATH = "user://settings.ini"`; estado `metalfx_supported` (nome do driver de renderização atual == "metal", avaliado na construção); os padrões de `settings.gd:20-40` (mesmas seções, chaves, ordem, tipos e valores); e a propriedade **exposta** `config_file` (um `ConfigFile` criado na construção).
- **FR-002**: Ao entrar na árvore MUST chamar `load_settings`.
- **FR-003**: `load_settings` MUST ser exposto com esse nome e reproduzir `settings.gd:55-62`: carregar `CONFIG_FILE_PATH` ignorando o erro de retorno; para cada (seção, chave) dos padrões ausente no `config_file`, gravar o padrão em memória; MUST NOT salvar o arquivo.
- **FR-004**: `save_settings` MUST ser exposto com esse nome e salvar o `config_file` em `CONFIG_FILE_PATH`.
- **FR-005**: Os valores dos padrões MUST ser os mesmos inteiros/booleanos/reais do original (enums do engine para modo de janela, vsync, filtro de escala, MSAA, AA de tela, qualidade de SSAO; `GIType`/`GIQuality` como 1/1; `−1` para SSIL; `0` para fps; `1.0` para escala), de modo que um `settings.ini` gravado pelo original é lido pelo port e vice-versa sem alteração de valores.

**Comportamento (US2)**

- **FR-006**: `apply_graphics_settings(window, environment, scene_root)` MUST ser exposto com esse nome e essa aridade (3 argumentos, chamada dinâmica de `level.rs:44` e `menu.rs:208,606`) e reproduzir `settings.gd:69-107` linha a linha: modo da janela **própria** (`get_window()`) = `video/display_mode`; vsync do servidor de display = `video/vsync`; fps máximo do engine = `video/max_fps`; no parâmetro `window`: `scaling_3d_scale`, `scaling_3d_mode`, `use_taa`, `msaa_3d`, `screen_space_aa`; se `rendering/shadow_mapping` falso → `scene_root.propagate_call("set", ["shadow_enabled", false])`.
- **FR-007**: SSAO MUST reproduzir `settings.gd:88-95` com a estrutura `if` / `else if` / `else` (−1 → desliga e nada mais — correção FR-020–FR-024; média → alta com `half_size` falso — quirk preservado; senão → média com `half_size` verdadeiro; parâmetros 0.5, 2, 50, 300). SSIL MUST reproduzir `settings.gd:97-104` (`if`/`elif`/`else`: −1 desliga; média → média sem `half_size`; senão alta com `half_size`; mesmos parâmetros). Depois `glow_enabled` = `rendering/bloom` e `volumetric_fog_enabled` = `rendering/volumetric_fog`.
- **FR-008**: Os valores lidos do `config_file` MUST ser interpretados como no original (inteiros de enum convertidos para os enums do engine correspondentes; booleanos; real para a escala).

**Comportamento (US3)**

- **FR-009**: `_input` MUST, quando a ação `toggle_fullscreen` for pressionada, colocar a janela em tela cheia exclusiva se ela não estiver em tela cheia nem em tela cheia exclusiva, senão em modo janela, e marcar o input como tratado no viewport (`settings.gd:49-52`).

**Ciclo de porte (Princípio II adaptado ao autoload)**

- **FR-010**: O nome `Settings` MUST ser conferido contra classes do engine e identificadores de topo dos `.gd` remanescentes (regra do `CLAUDE.md`) antes do port; colisão → parar.
- **FR-011**: O vínculo MUST ser: nova cena `oxide-godot/menu/settings.tscn` com **um único** node raiz `[node name="Settings" type="Settings"]` (sem script, sem `ext_resource`), e a linha `project.godot:25` trocada para `Settings="*res://menu/settings.tscn"` (singleton `*` preservado). Nenhuma outra linha de `project.godot` muda; nenhum `.gd` ponte.
- **FR-012**: `settings.gd` e `settings.gd.uid` MUST ser apagados no mesmo commit; nenhuma referência a `res://menu/settings.gd` nem a `uid://b04fekxdgdq0k` resta fora de `.godot/`.
- **FR-013**: Nomes MUST ser idênticos ao GDScript: propriedade `config_file`; métodos `load_settings`, `save_settings`, `apply_graphics_settings`; ação `toggle_fullscreen` tratada em `_input`. Conferidos contra os 10 `get_node_as("/root/Settings")` (12 acessos) de acesso dos 5 módulos Rust.
- **FR-014**: Os 5 módulos consumidores (`bullet.rs`, `flying_forklift.rs`, `level.rs`, `menu.rs`, `main_scene.rs`) e todos os demais módulos existentes MUST permanecer byte a byte intactos; o commit toca só o módulo novo, `lib.rs` (registro do módulo), `settings.tscn` (nova), `project.godot` (1 linha), as duas deleções e `docs/v2-backlog.md`.
- **FR-015**: O código portado MUST NOT chamar API customizada de GDScript (não resta nenhuma); `propagate_call("set", …)` é API base do engine e é permitida por ser o que o original faz.
- **FR-016**: Cada alteração de código MUST ser seguida de build de debug sem warnings novos.
- **FR-017**: O port MUST ser validado em headless: import com carregamento da extensão + `main.tscn` (boot completo com o autoload em cena), `menu.tscn` e `level.tscn` sem erros novos além da baseline (`CLAUDE.md`: 3 erros de import conhecidos; regra do erro intermitente de `main.tscn`); `settings.tscn` MUST NOT ser rodada isoladamente.
- **FR-018**: Um único commit `Port …` na `main`; o jogo MUST ficar jogável após ele; melhorias percebidas (o quirk 2 do SSAO e o que mais surgir) MUST ir para `docs/v2-backlog.md` no mesmo commit; a única correção de bug é a do SSAO (FR-020–FR-024; `docs/upstream-bugs.md` passa a 3 entradas).
- **FR-019**: `CLAUDE.md` MAY ganhar apenas a nota operacional sobre o vínculo de autoload por cena (se o plan julgar útil); nenhuma outra edição de documentação.

**Correção conservadora de bug do upstream — SSAO desligado (Princípio I, v1.3.1)**

- **FR-020**: O defeito: `settings.gd:88-95` faz `if ssao_quality == -1: ssao_enabled = false` e, na linha seguinte, `if ssao_quality == MEDIUM: … else: ssao_enabled = true …` — sem `elif`. Resultado: com "SSAO: Disabled" (−1) escolhido no menu, o SSAO é desligado e imediatamente religado em qualidade média com `half_size`; a opção não tem efeito. Intenção inequívoca (a primeira linha desliga; o bloco SSIL idêntico em `settings.gd:97-104` usa `elif`) contradita pelo resultado → **bug**.
- **FR-021**: A correção MUST ser mínima: o segundo teste vira `else if` (equivalente ao `elif` do SSIL). Nada mais muda: os ramos "média → alta sem `half_size`" e "senão → média com `half_size`" ficam como estão (quirk 2 preservado), parâmetros idênticos, SSIL intacto. É PROIBIDO reestruturar o método, unificar SSAO/SSIL ou "corrigir" o quirk 2.
- **FR-022**: A correção MUST estar isolada e identificável, com comentário `// upstream bug fix: ...` na linha imediatamente acima do `else if`, dentro do commit do port (é o único commit do marco), cuja mensagem menciona a correção.
- **FR-023**: `docs/upstream-bugs.md` MUST receber a entrada #3 (defeito, script/cena, correção aplicada, commit por assunto) no mesmo commit.
- **FR-024**: Resultado esperado: com `rendering/ssao_quality = -1` no `config_file`, após `apply_graphics_settings` o `environment.ssao_enabled` é **falso** (no original: verdadeiro); os demais valores produzem o mesmo resultado do original.

### Key Entities

- **Settings (autoload, `/root/Settings`)**: propriedade exposta `config_file`; métodos expostos `load_settings()`, `save_settings()`, `apply_graphics_settings(window, environment, scene_root)`; estado `metalfx_supported`; padrões (15 chaves em 2 seções); trata `toggle_fullscreen`. Consumido dinamicamente por 5 módulos Rust (10 `get_node_as("/root/Settings")` (12 acessos)).
- **Configurações (`user://settings.ini`)**: seções `video` (`display_mode`, `vsync`, `max_fps`, `resolution_scale`, `scale_filter`) e `rendering` (`taa`, `msaa`, `screen_space_aa`, `shadow_mapping`, `gi_type`, `gi_quality`, `ssao_quality`, `ssil_quality`, `bloom`, `volumetric_fog`); criado só por `save_settings`; compatível nos dois sentidos com o original.
- **Vínculo do autoload**: `menu/settings.tscn` (1 node) + `project.godot:25`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ao final do marco o projeto contém **zero** arquivos `.gd` e **zero** `.gd.uid` (fora de `addons/`); `project.godot` difere do estado anterior em exatamente 1 linha.
- **SC-002**: O jogo completo — boot → menu (Settings com as 15 linhas refletindo e gravando cada opção; Apply aplica e persiste; Cancel descarta; reabrir e reiniciar o jogo preservam) → loading → level (efeitos gráficos conforme cada opção, inclusive sombras desligadas e SSAO/SSIL) → ESC → menu; F11/Alt+Enter alternam tela cheia — é indistinguível do original em `../oxide_godot_origins/` numa comparação lado a lado feita pelo usuário.
- **SC-003**: Um `settings.ini` gravado pelo original é lido pelo port sem alteração; um Apply sem mudanças no port regrava o arquivo idêntico (mesmas chaves, ordem, tipos e valores).
- **SC-004**: Validação headless (import + `main.tscn` + `menu.tscn` + `level.tscn`) com zero erros novos; `main.tscn` percorre boot → menu → host automático → level com o autoload em Rust.
- **SC-005**: Os 5 módulos consumidores são byte a byte iguais ao commit anterior; o histórico do marco tem exatamente 1 commit `Port …`.
- **SC-006**: Build de debug sem nenhum warning.
- **SC-007**: Nenhuma linha portada introduz abstração, refatoração ou otimização; a única correção é a do SSAO desligado (FR-020–FR-024, 4/4 requisitos da cláusula); `docs/upstream-bugs.md` passa a 3 entradas; `docs/v2-backlog.md` ganha o quirk 2 do SSAO (≥ 1 item novo).
- **SC-008**: Com o OK do usuário, a v1 é **declarada concluída** (Princípio I: nenhum script `.gd` restante e o jogo jogável de ponta a ponta). Tag e branch `v2` são passos separados, decididos pelo usuário.

## Assumptions

- Fase v1 (constituição v1.3.1); uma correção conservadora de bug (SSAO desligado, FR-020–FR-024); o quirk 2 do SSAO é preservado e vai ao backlog.
- Validação visual (SC-002, F11, persistência entre execuções) pelo usuário; validação automatizada exclusivamente headless. O revisor pode rodar um harness de paridade fora do repositório.
- O vínculo por cena é a leitura fiel do Princípio II para autoloads (regra declarada em Contexto); a edição de `project.godot` é o equivalente da troca de `type`.
- Expor ou não os enums `GIType`/`GIQuality` como constantes da classe é decisão do plan (nenhum consumidor os lê por nome).
- Nome do módulo Rust (ex.: `settings.rs`) é decisão do plan; a classe é `Settings`.
- `metalfx_supported` é falso na plataforma de validação (driver ≠ "metal"); o ramo MetalFX é verificado por leitura de código.
- Fora de escopo: acesso tipado ao `Settings` pelos consumidores (backlog 1); qualquer melhoria; multiplayer; correção dos quirks do SSAO; tag/branch v2.
