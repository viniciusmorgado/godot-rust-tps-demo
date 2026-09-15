# Feature Specification: Marco A — folhas e input do jogador (v1 raw port)

**Feature Branch**: `001-v1-leaves-and-input`

**Created**: 2026-09-15

**Status**: Draft

**Fase**: v1 — Raw Port (Princípio I da constituição). Nenhuma abstração, refatoração ou otimização é permitida nesta feature; melhorias percebidas vão para `docs/v2-backlog.md`.

**Input**: User description: "Marco A do porte — portar para Rust os 5 scripts folha e o input do jogador do Godot TPS Demo, mantendo o jogo jogável e com comportamento idêntico ao original a cada script portado. Insumo: docs/port-order.md, itens 1 a 5 da ordem de migração."

## Contexto

O projeto é a migração do Godot TPS Demo (15 scripts) para código nativo, script por script, mantendo o jogo jogável ao final de cada passo. `docs/port-order.md` define a ordem de baixo para cima; este marco cobre os itens 1 a 5 — os scripts que não dependem de nenhum outro e que podem ser trocados individualmente enquanto os 10 restantes continuam no original. Cada um dos cinco é entregue como um passo independente, verificável sozinho, com o jogo jogável antes e depois.

Os cinco scripts, na ordem de entrega:

| # | Script original | Node/cena afetada | Linhas |
|---|---|---|---|
| 1 | `level/debug.gd` | `Debug` (Label) em `level/level.tscn` | 15 |
| 2 | `enemies/red_robot/parts/part_disappear_effect/part_disappear.gd` | raiz de `part_disappear.tscn` (CPUParticles3D) | 9 |
| 3 | `enemies/red_robot/laser/impact_effect/blast.gd` | raiz de `impact_effect.tscn` (Node3D) | 15 |
| 4 | `player/camera_noise_shake_effect.gd` | `Camera3D` em `player/player.tscn` | 61 |
| 5 | `player/player_input.gd` (`class_name PlayerInputSynchronizer`) | `InputSynchronizer` (MultiplayerSynchronizer) em `player/player.tscn` | 142 |

Referência de comportamento: o projeto original intocado em `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Overlay de debug (F3) portado (Priority: P1)

O jogador está dentro do level e pressiona F3: um painel de texto aparece (ou desaparece, se já estava visível) mostrando FPS, estado do VSync, memória estática em MiB, se a sessão está online e — só quando online — o ID multiplayer. Os valores mudam a cada frame. Tudo exatamente como no original, mas sem o script original no projeto.

**Why this priority**: É o menor script, sem dependências e sem estado, e introduz a mecânica básica que todos os outros passos reutilizam (processamento por frame, leitura de input, troca de tipo de node numa cena existente, remoção do script). Serve de prova do ciclo de porte completo com risco mínimo.

**Independent Test**: Abrir o jogo, entrar no level, pressionar F3 repetidas vezes e comparar o painel com o do original lado a lado. Validação headless: import do projeto confirmando a extensão carregada e execução de `level/level.tscn` sem erros novos.

**Acceptance Scenarios**:

1. **Given** o jogador está no level com o overlay no estado inicial da cena, **When** pressiona F3, **Then** a visibilidade do overlay inverte; pressionando de novo, volta ao estado anterior.
2. **Given** o overlay está visível, **When** um frame se passa, **Then** o texto mostra, em linhas separadas: `FPS: <n>`, `VSync: Enabled|Disabled`, `Memory: <x.xx> MiB` (duas casas decimais) e `Online: Yes|No`.
3. **Given** a sessão está offline (single-player), **When** o overlay é exibido, **Then** mostra `Online: No` e não mostra a linha de ID multiplayer.
4. **Given** a sessão está online, **When** o overlay é exibido, **Then** mostra `Online: Yes` seguido de `Multiplayer ID: <id>`.
5. **Given** o overlay está oculto, **When** frames se passam, **Then** o texto continua sendo atualizado (como no original), de modo que ao reaparecer já mostra valores atuais.

---

### User Story 2 - Efeito de desaparecimento de peça portado (Priority: P2)

Quando um robô é destruído, suas peças caem e, ao sumirem, cada uma dispara um efeito de partículas: uma explosão de "mini blasts" imediata, seguida de um sopro de fumaça que começa 0,2 s depois, e o efeito se remove sozinho da cena passado 2× a própria duração de vida das partículas.

**Why this priority**: Nove linhas, sem dependências, e introduz o primeiro comportamento baseado em tempo (esperas encadeadas) e auto-remoção da cena — capacidade necessária para os passos 3 e 4.

**Independent Test**: Destruir um robô no level e observar cada peça sumir com o puff, comparando com o original. Validação headless: execução de `part_disappear.tscn` sem erros novos.

**Acceptance Scenarios**:

1. **Given** o efeito é instanciado e entra na cena, **When** o primeiro frame roda, **Then** o filho `MiniBlasts` começa a emitir imediatamente e o emissor principal ainda não emite.
2. **Given** o efeito está na cena há 0,2 s, **When** esse instante é atingido, **Then** o emissor principal começa a emitir.
3. **Given** o emissor principal tem `lifetime` L, **When** transcorrem 0,2 s + 2×L desde a entrada na cena, **Then** o node do efeito é removido da cena.
4. **Given** o efeito está em andamento, **When** observado ao lado do original nas mesmas condições, **Then** os tempos e a aparência são indistinguíveis.

---

### User Story 3 - Impacto do laser portado (Priority: P3)

Quando o laser do robô atinge algo, aparece um efeito de impacto animado: a cada frame, os "raios de luz" do efeito se orientam para a câmera ativa (se ela ainda existir), e o efeito se remove da cena assim que sua animação termina.

**Why this priority**: Quinze linhas, sem dependências. Introduz a resposta a um sinal (fim de animação), referência a nodes filhos e consulta à câmera ativa — capacidades usadas pelos passos 4 e 5.

**Independent Test**: Deixar o robô atirar no jogador ou numa parede e observar o impacto orientado para a câmera e sumindo ao fim da animação, comparando com o original. Validação headless: execução de `impact_effect.tscn` sem erros novos.

**Acceptance Scenarios**:

1. **Given** existe uma câmera ativa quando o efeito entra na cena, **When** cada frame roda, **Then** o filho `LightRays` fica orientado para a posição global da câmera.
2. **Given** a câmera capturada na entrada foi destruída, **When** frames seguintes rodam, **Then** o efeito continua sem orientar os raios e sem produzir erros.
3. **Given** o efeito está na cena, **When** a animação do `AnimationPlayer` termina, **Then** o node do efeito é removido da cena.
4. **Given** não há câmera ativa no momento em que o efeito entra na cena, **When** o efeito roda, **Then** ele anima e some normalmente, sem orientar os raios e sem erros.

---

### User Story 4 - Tremor de câmera portado (Priority: P4)

Ao atirar, ao ser atingido ou ao ser acertado pelo robô, a câmera do jogador treme: um "trauma" se acumula (limitado a 1,2), decai a 1,5 por segundo, e enquanto houver trauma a câmera recebe uma rotação de guinada/arfagem/rolagem por ruído proporcional ao quadrado do trauma, somada à sua rotação inicial. O restante do jogo (ainda no original) continua chamando `add_trauma(...)` com 0,35 ao atirar, 0,75 ao ser atingido e 13,0 quando o robô acerta o jogador, sem nenhuma alteração.

**Why this priority**: Primeiro script cuja API é chamada por código que permanece no original (`player.gd`, `red_robot.gd`). Comprova que a interface pública (nome de método) sobrevive à troca de tipo do node. Também introduz estado interno persistente e geração de ruído.

**Independent Test**: Atirar e ser atingido no level; a câmera deve tremer com a mesma intensidade e duração que no original e voltar exatamente à rotação inicial ao final. Validação headless: execução de `player/player.tscn` sem erros novos.

**Acceptance Scenarios**:

1. **Given** trauma = 0, **When** `add_trauma(0.35)` é chamado, **Then** trauma = 0,35 e a câmera começa a tremer no frame seguinte.
2. **Given** trauma = 1,0, **When** `add_trauma(13.0)` é chamado, **Then** trauma = 1,2 (teto), não mais.
3. **Given** trauma > 0, **When** um frame de duração `delta` roda, **Then** trauma diminui 1,5×`delta` (sem ficar negativo) e a rotação da câmera = rotação inicial + (arfagem, guinada, rolagem) onde cada componente = limite × trauma² × ruído em [-1, 1], com limites 0,05 (guinada), 0,05 (arfagem) e 0,1 (rolagem).
4. **Given** trauma acaba de chegar a 0 num frame, **When** esse frame termina, **Then** a rotação da câmera é exatamente a rotação inicial (ruído × 0) e nos frames seguintes a câmera não é mais alterada.
5. **Given** o script `player.gd` e o `red_robot.gd` continuam no original, **When** o jogador atira, é atingido ou é acertado pelo robô, **Then** as chamadas existentes a `add_camera_shake_trauma`/`add_trauma` funcionam sem qualquer edição nesses scripts.
6. **Given** duas execuções do jogo, **When** o tremor acontece, **Then** o padrão de ruído pode diferir entre execuções (semente aleatória, como no original), mas a intensidade e duração são as mesmas.

---

### User Story 5 - Sincronizador de input do jogador portado (Priority: P5)

Todo o input do jogador — mover, olhar (analógico e mouse), mirar (toggle por toque curto ou hold), pular, atirar com alvo por raycast — e o fade para preto ao cair do mapa passam a ser produzidos pelo node portado. O `player.gd`, que continua no original, lê as mesmas propriedades (`aiming`, `shoot_target`, `motion`, `shooting`, `jumping`) e chama os mesmos métodos (`get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`) sem nenhuma alteração. As quatro primeiras propriedades continuam sendo replicadas pela configuração de sincronização já existente na cena.

**Why this priority**: É o maior e mais rico dos cinco scripts (RPC, propriedades replicadas, referências a nodes preenchidas pela cena, raycast, input contínuo e por evento) e depende de todas as capacidades provadas nos passos anteriores. Ao mesmo tempo é o de maior valor: ele é a metade da experiência de jogo, e é pré-requisito direto do Marco B (`player.gd`).

**Independent Test**: Entrar no level em single-player e exercitar cada input, comparando com o original: velocidade de giro da câmera com analógico e mouse (e as reduções ao mirar), limite de pitch, toggle e hold de mira com as animações "shoot"/"far", pulo, tiro acertando o ponto sob o crosshair, queda pelo buraco do mapa com fade para preto e retorno com fade-out. Validação headless: execução de `player/player.tscn` sem erros novos.

**Acceptance Scenarios**:

1. **Given** o node é a autoridade multiplayer (caso single-player), **When** ele entra na cena, **Then** sua câmera se torna a câmera ativa e o mouse é capturado.
2. **Given** o node NÃO é a autoridade multiplayer, **When** ele entra na cena, **Then** ele para de processar frames e input, e o retângulo de fade fica oculto.
3. **Given** o jogador segura `move_right`, **When** um frame roda, **Then** `motion` = (1, 0); com `move_forward`, `motion` = (0, -1); combinações e intensidades analógicas produzem o vetor correspondente (direita−esquerda, trás−frente).
4. **Given** o jogador desloca o analógico de visão com intensidade 1 por 1 s sem mirar, **When** os frames rodam, **Then** a câmera gira 3,0 rad em guinada; mirando, 1,5 rad.
5. **Given** o mouse se move N pixels, **When** o evento é recebido, **Then** a câmera gira 0,001×N rad sem mirar e 0,00075×N rad mirando.
6. **Given** a câmera está olhando para cima ou para baixo, **When** o jogador continua girando verticalmente, **Then** o pitch é limitado ao intervalo [-89,9°, 70°].
7. **Given** o jogador não está mirando, **When** pressiona e solta `aim` em menos de 0,4 s, **Then** a mira fica ligada (toggle) e a animação de câmera "shoot" toca; ao pressionar `aim` de novo, a mira desliga e "far" toca.
8. **Given** o jogador não está mirando, **When** segura `aim` por mais de 0,4 s e solta, **Then** a mira fica ligada enquanto segura e desliga ao soltar ("shoot" ao ligar, "far" ao desligar).
9. **Given** o jogador pressiona `jump`, **When** o frame roda, **Then** um RPC local `jump` é disparado e `jumping` passa a `true` (o `player.gd` original o consome e reseta, sem alteração).
10. **Given** o jogador segura `shoot`, **When** o frame roda, **Then** `shooting` = true e `shoot_target` = ponto de colisão do raio lançado do centro do crosshair (alcance 1000, máscara de camadas 0b11, sem exclusão efetiva — quirk do original, ver FR-017); se nada é atingido, `shoot_target` = origem + direção × 1000.
11. **Given** o jogador cai e sua altura fica abaixo de −17, **When** o frame roda, **Then** a opacidade do retângulo preto = min((−17 − y)/15, 1) — totalmente preto em y ≤ −32.
12. **Given** o jogador foi teleportado de volta (y ≥ −17) com o retângulo ainda opaco, **When** frames rodam, **Then** a opacidade é multiplicada por (1 − 4×`delta`) a cada frame até desaparecer gradualmente.
13. **Given** `player.gd` continua no original, **When** ele lê `aiming`, `shoot_target`, `motion`, `shooting`, `jumping` e chama `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`, **Then** tudo funciona sem nenhuma edição em `player.gd`.
14. **Given** a cena `player.tscn` preenche `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect` via `node_paths`, **When** a cena é carregada, **Then** todas as seis referências estão preenchidas no node portado (mesmos nomes e tipos compatíveis).
15. **Given** `get_aim_rotation()` é chamado, **When** o pitch da câmera é ≥ 0, **Then** retorna −pitch/70°; quando < 0, retorna pitch/(−89,9°) — ou seja, valor em [−1, 1] normalizado pelos limites.

---

### Edge Cases

- **Overlay oculto**: o texto do overlay continua sendo recalculado a cada frame mesmo invisível (comportamento do original; não otimizar).
- **F3 fora do level**: a ação `toggle_debug` só tem efeito quando o node `Debug` existe na cena atual (menu não tem overlay).
- **Efeito de peça removido antes dos timers**: se a cena for descarregada antes de 0,2 s ou de 2×lifetime, o efeito não deve produzir erros novos além dos que o original produziria.
- **Câmera destruída durante o impacto** (ex.: jogador trocado de cena): os raios param de se orientar sem erros.
- **Trauma acumulado acima do teto**: qualquer soma acima de 1,2 é truncada em 1,2.
- **Rotação inicial da câmera**: é capturada quando a câmera entra na cena; se outras animações moverem a câmera depois, o tremor continua somando à rotação inicial capturada (quirk do original, preservado).
- **Peer não-autoridade**: nenhum input é processado, nenhum evento de mouse é tratado, e o fade permanece oculto.
- **Mira: pressionar `aim` enquanto já em toggle**: desliga o toggle (o toque que desliga não religa).
- **Mira: toque exatamente em 0,4 s**: conta como toque curto (≤ 0,4 s liga o toggle).
- **Raycast sem colisão**: alvo vira o ponto a 1000 unidades na direção do olhar.
- **Raycast atingindo o próprio jogador**: a lista de exclusão do original resolve para um RID inválido (nenhuma exclusão efetiva), e o corpo do jogador está na máscara 0b11; se o raio atingir o próprio corpo, `shoot_target` é esse ponto — comportamento do original, preservado (não corrigir).
- **Pitch nos limites**: continuar girando além de −89,9° ou 70° não altera o pitch.
- **Fade sem queda**: com y ≥ −17 e opacidade já 0, a multiplicação por (1 − 4×delta) mantém 0.

## Requirements *(mandatory)*

### Functional Requirements

**Comportamento — overlay de debug (US1)**

- **FR-001**: O overlay de debug MUST alternar visibilidade ao pressionar a ação `toggle_debug` (F3), uma vez por pressionamento.
- **FR-002**: O overlay MUST atualizar, a cada frame, um texto com FPS, VSync (`Enabled`/`Disabled`), memória estática em MiB com duas casas decimais e estado online (`Yes`/`No`), acrescentando o ID multiplayer somente quando online. "Online" significa que o peer multiplayer atual não é o peer offline padrão.

**Comportamento — efeito de desaparecimento de peça (US2)**

- **FR-003**: Ao entrar na cena, o efeito MUST ligar imediatamente a emissão do filho `MiniBlasts`.
- **FR-004**: O efeito MUST ligar sua própria emissão 0,2 s após entrar na cena.
- **FR-005**: O efeito MUST se remover da cena 2× o próprio `lifetime` após ligar a própria emissão.

**Comportamento — impacto do laser (US3)**

- **FR-006**: Ao entrar na cena, o efeito MUST capturar a câmera ativa daquele momento e, a cada frame, orientar o filho `LightRays` para a posição global dessa câmera enquanto ela existir.
- **FR-007**: O efeito MUST se remover da cena quando a animação do filho `AnimationPlayer` terminar.

**Comportamento — tremor de câmera (US4)**

- **FR-008**: A câmera MUST expor o método `add_trauma(amount)`, que acumula trauma limitado a 1,2.
- **FR-009**: Enquanto trauma > 0, a cada frame a câmera MUST reduzir o trauma em 1,5 × delta (sem ficar negativo) e então aplicar rotação = rotação inicial + (pitch, yaw, roll), com pitch = 0,05 × trauma² × ruído, yaw = 0,05 × trauma² × ruído, roll = 0,1 × trauma² × ruído, cada ruído em [−1, 1] obtido de um gerador de ruído 1D com sementes distintas (seed, seed+1, seed+2) e posição temporal acumulada a 5000 × delta por frame.
- **FR-010**: A semente do ruído MUST ser aleatória por instância, e a rotação inicial MUST ser capturada quando a câmera entra na cena.

**Comportamento — sincronizador de input (US5)**

- **FR-011**: Ao entrar na cena, se o node for a autoridade multiplayer, MUST tornar `camera_camera` a câmera ativa e capturar o mouse; caso contrário MUST desligar seu processamento por frame e de input e ocultar `color_rect`.
- **FR-012**: A cada frame MUST calcular `motion` = (força(`move_right`) − força(`move_left`), força(`move_back`) − força(`move_forward`)).
- **FR-013**: A cada frame MUST girar a câmera pelo analógico de visão (`view_right`−`view_left`, `view_up`−`view_down`) a 3,0 rad/s, reduzido à metade ao mirar; e, por evento de movimento do mouse, a 0,001 rad/pixel, reduzido a 0,75× ao mirar.
- **FR-014**: Girar a câmera MUST aplicar guinada em `camera_base` (rotação em Y por −move.x, seguida de reortonormalização) e arfagem em `camera_rot` (rotação X somada a move.y e limitada a [−89,9°, 70°]).
- **FR-015**: A mira MUST seguir a lógica de toggle/hold: soltar `aim` após ≤ 0,4 s pressionado liga o toggle; pressionar `aim` desliga o toggle; a mira está ativa se o toggle está ligado ou `aim` está pressionado; o contador de tempo pressionado acumula enquanto a mira está ativa e zera quando não está. Ao mudar de estado, MUST tocar a animação "shoot" (ligou) ou "far" (desligou) em `camera_animation`.
- **FR-016**: Ao pressionar `jump`, MUST disparar o RPC `jump` (modo `call_local`), cujo efeito é `jumping = true`.
- **FR-017**: A cada frame `shooting` MUST refletir se `shoot` está pressionado; quando pressionado, `shoot_target` MUST ser o ponto de colisão de um raio lançado do centro de `crosshair` através de `camera_camera`, com alcance 1000, máscara de colisão 0b11 e lista de exclusão contendo apenas o RID do próprio sincronizador — que, por não ser um corpo físico, resolve para um RID inválido (`RID(0)`, verificado no Godot 4.7.2); o efeito prático é **nenhuma exclusão**, e esse comportamento MUST ser preservado (NÃO excluir o corpo do jogador: seria correção de bug, proibida na v1 pelo Princípio I). Sem colisão, `shoot_target` = origem + direção × 1000.
- **FR-018**: A cada frame, se a altura global do pai (jogador) for < −17, a opacidade de `color_rect` MUST ser min((−17 − y)/15, 1); senão MUST ser multiplicada por (1 − 4 × delta).
- **FR-019**: O node MUST expor `get_aim_rotation()` (pitch limitado normalizado: ≥ 0 → −pitch/70°; < 0 → pitch/(−89,9°)), `get_camera_base_quaternion()` (quaternion de rotação da base global de `camera_base`) e `get_camera_rotation_basis()` (base global de `camera_rot`).
- **FR-020**: O node MUST expor com os nomes originais as propriedades `aiming`, `shoot_target`, `motion`, `shooting`, `jumping` e as referências `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect`, de modo que a configuração de replicação e os `node_paths` já existentes em `player.tscn` continuem válidos sem edição.

**Ciclo de porte — comuns aos cinco (Princípio II)**

- **FR-021**: Cada script MUST virar exatamente uma classe registrada pela extensão nativa, com a mesma classe base do script original (Label, CPUParticles3D, Node3D, Camera3D, MultiplayerSynchronizer). O item 5 MUST manter o nome de classe `PlayerInputSynchronizer`.
- **FR-022**: O vínculo à cena MUST ser feito trocando o `type` do node na `.tscn` e removendo a linha `script` e o `ext_resource` do `.gd` órfão; nenhum `.gd` pode permanecer attached como ponte.
- **FR-023**: O `.gd` e o `.gd.uid` correspondentes MUST ser apagados no mesmo commit em que o node passa a usar a classe portada.
- **FR-024**: Nomes de métodos expostos e de propriedades exportadas/replicadas MUST ser idênticos aos do GDScript (verificados contra a `.tscn` afetada antes de concluir cada port).
- **FR-025**: Cada alteração de código MUST ser seguida de build de debug bem-sucedido sem warnings novos, antes de validar ou commitar.
- **FR-026**: Cada port MUST ser validado em modo headless: (a) import do projeto confirmando o carregamento da extensão e (b) execução da cena afetada sem erros novos além dos três catalogados no `CLAUDE.md` (`Cannon_Charge already exists`, `doorsimple_d.png` ausente, `surfaces.is_empty()`).
- **FR-027**: Cada port MUST ser um commit próprio cuja mensagem informa o script portado e a(s) cena(s) com tipo de node trocado.
- **FR-028**: Toda melhoria percebida durante o port MUST ser registrada em `docs/v2-backlog.md` (origem + motivação) no mesmo commit, e NUNCA aplicada no código.
- **FR-029**: Os 10 scripts fora deste marco (`player.gd`, `bullet.gd`, `door.gd`, `part.gd`, `red_robot.gd`, `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`) MUST permanecer byte a byte intactos.
- **FR-030**: A entrega MUST seguir a ordem 1 → 5; ao final de cada passo o jogo MUST estar jogável de ponta a ponta (menu → level → jogar).

### Key Entities

- **Contrato do sincronizador de input**: estado observável lido pelo jogador original — `motion` (vetor 2D), `aiming` (bool), `shooting` (bool), `shoot_target` (ponto 3D), `jumping` (bool, consumido pelo jogador após o pulo). Os quatro primeiros são replicados pela configuração de sincronização da cena; `jumping` é propagado por RPC. Métodos: `get_aim_rotation()`, `get_camera_base_quaternion()`, `get_camera_rotation_basis()`, `jump` (RPC).
- **Referências de cena do sincronizador**: `camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect` — preenchidas pela cena, nunca buscadas por caminho no código.
- **Trauma da câmera**: escalar em [0, 1,2], alimentado por `add_trauma(amount)`, decaído a 1,5/s; a intensidade do tremor é trauma².
- **Overlay de debug**: texto multilinha recalculado por frame; visibilidade alternada por `toggle_debug`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ao final do marco, o projeto contém exatamente 10 arquivos `.gd` (os 5 deste marco não existem mais, nem seus `.gd.uid`), e os 10 restantes são idênticos ao estado anterior ao marco.
- **SC-002**: O jogo abre pelo menu, entra no level e, numa sessão single-player, mover, olhar (analógico e mouse), mirar (toggle e hold), pular, atirar, tremor de câmera, F3, impacto do laser e desaparecimento das peças do robô são indistinguíveis do original em `../oxide_godot_origins/` numa comparação lado a lado feita pelo usuário.
- **SC-003**: Para cada um dos 5 ports, a validação headless (import + execução da cena afetada) reporta zero erros novos além dos 3 catalogados no `CLAUDE.md`, e o import mostra a linha de carregamento da extensão.
- **SC-004**: Após cada um dos 5 commits, o jogo é jogável de ponta a ponta — nenhum estado intermediário quebra o menu, o level ou o jogador.
- **SC-005**: O histórico do marco contém exatamente 5 commits de port (um por script), cada um nomeando o script portado e a cena alterada; nenhum commit toca os 10 scripts fora de escopo.
- **SC-006**: O build de debug termina sem nenhum warning novo em relação ao estado anterior ao marco, em todos os 5 commits.
- **SC-007**: Nenhuma linha de código portado introduz abstração, refatoração ou otimização; toda melhoria percebida aparece como entrada em `docs/v2-backlog.md` no commit em que foi percebida (revisão de conformidade com o Princípio I).
- **SC-008**: `player.gd`, `red_robot.gd` e as cenas `player.tscn`/`level.tscn` continuam encontrando todos os métodos e propriedades pelos nomes originais — nenhum aviso de método/propriedade inexistente aparece nos logs headless ou no editor.

## Assumptions

- A fase é a v1 (raw port); as restrições do Princípio I se aplicam integralmente. Tradução direta, "Rust com cara de GDScript" é o resultado esperado.
- A validação funcional (SC-002) é visual, feita pelo usuário no editor/jogo comparando com `../oxide_godot_origins/`; a validação automatizada é exclusivamente a headless (SC-003).
- A validação é single-player/offline: o peer local é a autoridade multiplayer, logo o ramo "não-autoridade" do sincronizador (FR-011) é verificado por leitura de código e pelo headless, não por sessão com dois peers. Testes multiplayer reais com dois peers estão fora de escopo.
- Fora de escopo: `player.gd`, `bullet.gd`, `door.gd` e todos os scripts dos marcos B, C e D; acesso tipado ao autoload `Settings` (nenhum dos 5 scripts o usa); qualquer melhoria de comportamento, mesmo trivial.
- Os comportamentos "não-idiomáticos" do original são preservados propositalmente: texto do overlay recalculado enquanto oculto; rotação inicial da câmera capturada uma única vez; `jumping` exportado mas não replicado (propagado por RPC); raycast refeito a cada frame enquanto atira; lista de exclusão do raycast sem efeito prático (RID inválido do sincronizador).
- A ordem 1 → 5 é a de entrega; cada passo é independente, mas os passos 4 e 5 tocam a mesma cena (`player.tscn`) e por isso são commits separados sobre a mesma cena.
- A semente do ruído do tremor é aleatória por instância (como no original); o critério de comparação é intensidade/duração, não o padrão exato do ruído.
- Os três erros pré-existentes do demo upstream catalogados no `CLAUDE.md` são a baseline; qualquer outro erro no headless conta como regressão.
- `docs/port-order.md` e `docs/v2-backlog.md` já existem e são a fonte da ordem e o destino das melhorias, respectivamente.
