# Feature Specification: Marco B — jogador, bala e porta (v1 raw port)

**Feature Branch**: `002-v1-player-bullet-door` (trabalho na `main`, como no Marco A)

**Created**: 2026-09-15

**Status**: Draft

**Fase**: v1 — Raw Port (Princípio I da constituição v1.3.0). Nenhuma abstração, refatoração ou otimização; melhorias percebidas vão para `docs/v2-backlog.md`. Esta feature usa, pela primeira vez, a cláusula de **correção conservadora de bugs do upstream** (uma única correção, na porta — ver US3 e FR-030–FR-035).

**Input**: User description: "Portar para Rust o jogador, a bala e a porta do Godot TPS Demo, mantendo o jogo jogável e idêntico ao original a cada script — com uma única correção conservadora de bug do upstream (porta). Marco B de docs/port-order.md (itens 6, 7, 8)."

## Contexto

Continuação do porte script a script. O Marco A (`specs/001-v1-leaves-and-input`, commits `6b12ebf`..`0e71a49`) deixou 10 scripts no original e entregou `PlayerInputSynchronizer` e `CameraNoiseShake` como classes nativas — por isso o jogador agora pode nascer com acesso **tipado** a elas, sem chamadas dinâmicas. Este marco porta os três scripts do "jogador completo"; os 7 restantes (`part`, `red_robot`, `flying_forklift`, `level`, `menu`, `main`, `settings`) ficam byte a byte intactos e continuam consumindo a API portada pelos nomes originais.

| # | Script original | Base | Node/cena afetada | Linhas | Consumidores que permanecem no original |
|---|---|---|---|---|---|
| 1 | `player/player.gd` (`class_name Player`) | CharacterBody3D | raiz de `player/player.tscn` | 211 | `red_robot.gd:131,133,275,281` (`is Player`, `add_camera_shake_trauma`), `level.gd:118-119` (`name`, `player_id`), `bullet.gd:31-32` (`hit` por `has_method`) |
| 2 | `player/bullet/bullet.gd` | CharacterBody3D | raiz de `player/bullet/bullet.tscn` (também instanciada como `BulletCache` em `player.tscn:679`) | 51 | `player.gd` (instancia a cena — passa a ser classe nativa neste marco) |
| 3 | `door/door.gd` | Area3D | raiz de `door/door.tscn` | 12 | nenhum (`door.tscn` não é instanciada por nenhuma cena — asset órfão) |

Referência de comportamento: o projeto original intocado em `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Jogador portado (Priority: P1)

O jogador continua a ser exatamente o mesmo personagem: anda, corre, pula, pousa com som, mira e atira; ao cair do mapa reaparece no ponto inicial. Tudo isso passa a ser produzido pela classe nativa `Player`, enquanto o level (que o instancia e lhe dá um `player_id`), o robô (que testa `is Player` e chama `add_camera_shake_trauma`) e a bala (que chama `hit`) continuam no original, sem nenhuma edição, encontrando tudo pelos nomes de sempre.

**Why this priority**: É o maior script do marco, o único com dependentes fora dele (`red_robot`, `level`, `bullet`) e pré-requisito direto da bala (que ele instancia) e da porta (que testa `is Player`). Também é o primeiro script portado que **consome** classes já portadas (`PlayerInputSynchronizer`, `CameraNoiseShake`) — prova a integração tipada entre classes nativas.

**Independent Test**: `player.tscn` e `level.tscn` headless sem erros novos; no jogo, mover/pular/mirar/atirar/pouso/respawn indistinguíveis do original; o robô ainda atinge o jogador (tremor 13,0) e o level ainda o spawna com nome e id corretos.

**Acceptance Scenarios**:

1. **Given** o level instancia o jogador, define `name` e `player_id` **antes** de adicioná-lo à árvore, **When** o node entra na cena, **Then** o `InputSynchronizer` filho tem autoridade multiplayer igual a `player_id` (o setter roda fora da árvore, como no original) e a replicação de `player_id` (spawn), `motion` e `current_animation` (por frame) continua válida pelos nomes da cena.
2. **Given** o jogador está no chão sem input, **When** frames de física passam, **Then** anima WALK com `blend_position = (0, 0)` e não se move.
3. **Given** o jogador segura `move_forward`, **When** frames de física passam, **Then** `motion` interpola linearmente para o input a 10×delta, a orientação do modelo gira (slerp a 10×delta) para a direção da câmera achatada em Y, a animação WALK recebe `blend_position = (|motion|, 0)` e o deslocamento vem do root motion da animação (velocidade horizontal = deslocamento/delta), com gravidade aplicada e `move_and_slide` com up = +Y.
4. **Given** o jogador está no chão e o sincronizador de input sinaliza `jumping`, **When** o frame de física roda, **Then** `velocity.y = 5`, o jogador entra em `on_air`, o RPC `jump` toca a animação JUMP_UP e o som Jump, e `jumping` é zerado no sincronizador.
5. **Given** o jogador está no ar, **When** `velocity.y > 0`, **Then** anima JUMP_UP; **When** `velocity.y ≤ 0`, **Then** anima JUMP_DOWN.
6. **Given** o jogador esteve no ar por mais de 0,5 s, **When** toca o chão, **Then** o RPC `land` toca JUMP_DOWN e o som Land; `airborne_time` volta a 0.
7. **Given** o jogador acabou de entrar na cena (`airborne_time` inicial = 100), **When** toca o chão pela primeira vez, **Then** o RPC `land` dispara uma vez (quirk do original, preservado).
8. **Given** o jogador está mirando, **When** frames de física passam, **Then** a orientação faz slerp para o quaternion da base da câmera, anima STRAFE com `aim/add_amount = get_aim_rotation()` e `strafe/blend_position = (motion.x, −motion.y)`, e o root motion do AnimationTree é aplicado.
9. **Given** o jogador está mirando, `shooting` ativo e o cooldown de 0,4 s zerado, **When** o frame de física roda, **Then** uma bala é instanciada como filha do **pai** do jogador (nome legível), posicionada na origem global de `ShootFrom`, orientada para `shoot_target`, com exceção de colisão contra o jogador; o RPC `shoot` reinicia e liga `ShootParticle` e `MuzzleFlash`, reinicia o cooldown, toca o som Shoot e chama `add_camera_shake_trauma(0.35)`.
10. **Given** o cooldown ainda não zerou, **When** `shooting` continua ativo, **Then** nenhuma bala nova é criada.
11. **Given** o robô acerta o jogador com o laser, **When** `red_robot.gd:133` chama `player.add_camera_shake_trauma(13.0)`, **Then** a câmera (`CameraNoiseShake`, via `player_input.camera_camera`) recebe `add_trauma(13.0)` — chamada tipada, sem `.call()`.
12. **Given** uma bala de **outro** jogador atinge este jogador (só em multiplayer — as balas do próprio jogador têm exceção de colisão com ele, `player.gd:142`), **When** `bullet` chama `hit.rpc()` no colisor, **Then** o jogador recebe `add_camera_shake_trauma(0.75)`. Verificado por leitura de código; não observável single-player.
13. **Given** o jogador cai abaixo de y = −40, **When** o frame de física roda, **Then** é teleportado para a posição inicial capturada ao entrar na cena.
14. **Given** o peer local não é servidor, **When** o node entra na cena, **Then** o processamento por frame é desligado e, a cada frame de física, apenas `current_animation` (replicado) é animado — ramo verificado por leitura de código e headless (validação é single-player, onde o peer é servidor e autoridade).
15. **Given** `red_robot.gd`, `level.gd`, `door.gd` (até o port 3) e `part.gd` continuam no original, **When** usam `is Player`, `player_id`, `name`, `add_camera_shake_trauma`, `hit`, **Then** tudo resolve sem edição nesses scripts.

---

### User Story 2 - Bala portada (Priority: P2)

Ao atirar, a bala visível sai do cano, voa em linha reta a 20 unidades/s, explode ao acertar algo (chamando `hit` no alvo se ele tiver esse método — jogador ou robô) ou após 5 s, e some ao fim da animação de explosão. Se a opção de sombras estiver ligada nas configurações, a luz da explosão projeta sombra.

**Why this priority**: Depende do jogador (que a instancia) e é o primeiro consumidor da exceção do autoload `Settings` (acesso dinâmico ao `config_file`). Introduz duck typing preservado do original (`has_method("hit")` + `rpc`) e um method track de animação (`destroy`) — dois nomes de contrato.

**Independent Test**: `bullet.tscn` isolado e `player.tscn` (que carrega o `BulletCache`) headless sem erros novos; no jogo, a bala é visível, explode ao colidir e ao expirar, o robô reage ao `hit`.

**Acceptance Scenarios**:

1. **Given** a bala é instanciada pelo jogador no servidor, **When** frames de física passam, **Then** ela se desloca `−delta × 20 × basis.z` por frame com `move_and_collide`.
2. **Given** a bala colide com um corpo que tem método `hit` (robô; ou outro jogador, só em multiplayer), **When** a colisão ocorre, **Then** `hit` é chamado por RPC no colisor (duck typing do original), a colisão é desabilitada, o RPC `explode` dispara e `hit` interno vira `true`.
3. **Given** a bala colide com um corpo **sem** método `hit` (parede), **When** a colisão ocorre, **Then** apenas desabilita a colisão e explode.
4. **Given** a bala voa há 5 s sem colidir, **When** `time_alive` fica negativo, **Then** marca `hit = true` e explode; nos frames seguintes não faz mais nada.
5. **Given** o RPC `explode` dispara, **When** roda, **Then** toca a animação "explode" e, se `Settings.config_file` tem `rendering/shadow_mapping` verdadeiro, liga `shadow_enabled` na luz.
6. **Given** a animação "explode" chega ao method track `destroy`, **When** o peer é servidor, **Then** a bala é removida da cena; **When** não é servidor, **Then** nada acontece (o servidor replica a remoção).
7. **Given** o peer não é servidor, **When** a bala entra na cena, **Then** o processamento de física é desligado e a colisão desabilitada (ramo por leitura de código/headless).
8. **Given** `player.tscn` carrega, **When** o `BulletCache` (instância de `bullet.tscn`) é criado, **Then** a classe portada é instanciada ali sem erro e sem efeito visível (pré-aquecimento, como no original).
9. **Given** a replicação da cena (`global_transform`), **When** a bala se move, **Then** a configuração de sincronização continua válida sem edição.

---

### User Story 3 - Porta portada, com correção conservadora do bug do upstream (Priority: P3)

A porta abre (animação "doorsimple_opening") quando o jogador entra na sua área, uma única vez. No original isso **nunca acontece**: o script procura `DoorModel/AnimationPlayer`, mas o node da cena chama-se `DoorModel2`; a referência fica nula, o Godot imprime `Node not found` ao instanciar e a porta não abre. A intenção do código é inequívoca (abrir ao entrar um `Player`) e o resultado a contradiz — é **bug**, não melhoria (constituição v1.3.0, Princípio I).

**Correção mínima declarada (requisito (a) da cláusula)**: a classe portada referencia `DoorModel2/AnimationPlayer`. Nada mais muda: o node da cena não é renomeado, a lógica de `open` não é tocada, `door.tscn` só recebe a troca de tipo e a remoção do script. A animação `doorsimple_opening` existe nesse `AnimationPlayer` (verificado no modelo `door/model/door.dae`).

**Why this priority**: Menor script, sem consumidores (asset órfão — nenhuma cena instancia `door.tscn`), e o único com correção de bug: fica por último para que a correção seja um commit isolado e auditável. Depende de US1 (`is Player`).

**Independent Test**: `door.tscn` headless instancia **sem** o `ERROR: Node not found` que o original produz; em cena de teste isolada (fora do repo ou temporária, não commitada) um `Player` entrando na área faz a animação tocar uma vez.

**Acceptance Scenarios**:

1. **Given** `door.tscn` é instanciada isoladamente, **When** entra na cena, **Then** nenhum erro é impresso (no original: `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")`).
2. **Given** a porta está fechada (`open = false`), **When** um corpo que é `Player` entra na área (sinal `body_entered` conectado na cena a `_on_door_body_entered`), **Then** a animação "doorsimple_opening" toca e `open = true`.
3. **Given** a porta já está aberta, **When** um `Player` entra de novo, **Then** nada acontece.
4. **Given** um corpo que **não** é `Player` (bala, robô, peça) entra na área, **When** o sinal dispara, **Then** nada acontece.
5. **Given** a correção foi aplicada, **When** a revisão de conformidade roda, **Then** encontra os quatro requisitos: declaração nesta spec, comentário `// upstream bug fix: ...` no ponto exato da referência, menção na mensagem do commit, e entrada em `docs/upstream-bugs.md` (defeito, script/cena, correção, commit).
6. **Given** o jogo roda de ponta a ponta, **When** comparado ao original, **Then** nada visível muda (a porta não está em nenhuma cena do jogo).

---

### Edge Cases

- **Ordem `player_id` → árvore**: o level define `player_id` antes de `add_child`; o setter precisa alcançar o filho `InputSynchronizer` fora da árvore (o filho já existe como parte da cena instanciada). Quirk preservado.
- **Primeiro pouso**: `airborne_time` inicia em 100 → o primeiro contato com o chão dispara `land` (som de pouso ao spawnar), como no original.
- **`jumping` zerado pelo jogador**: a cada frame de física o jogador escreve `player_input.jumping = false`, mesmo sem pular — contrato com `PlayerInputSynchronizer.jumping` (escrita tipada).
- **Tiro com cooldown**: `FireCooldown` é `autostart` de 0,4 s — nos primeiros 0,4 s após o spawn não é possível atirar (original).
- **Bala nascida dentro de um colisor**: `add_collision_exception_with(self)` evita acertar o próprio jogador; qualquer outro corpo na origem explode no primeiro frame (original).
- **Bala expira e colide no mesmo frame**: `time_alive < 0` marca `hit` e explode; o `move_and_collide` do mesmo frame ainda roda e pode explodir de novo (dois RPCs `explode`) — comportamento do original, preservado.
- **`Settings` em execução isolada**: `explode` lê `/root/Settings` dinamicamente. Na validação prevista (execução headless de `bullet.tscn`/`player.tscn` via `--path`) o autoload **está** carregado (verificado), então o acesso funciona como no jogo. O autoload só falta em harness de `SceneTree` (`-s`) fora do repo — cenário do revisor, não do implementador; nesse caso o erro é o mesmo do original e não conta.
- **Porta: corpo que sai e volta**: só o primeiro `Player` abre; não há fechamento (original).
- **Porta: erro do upstream eliminado**: `Node not found: "DoorModel/AnimationPlayer"` deixa de existir — ele **não** está no catálogo do `CLAUDE.md` (que lista só os 3 erros de import), portanto o catálogo não muda.
- **Respawn**: abaixo de −40 o jogador é teleportado, mas `velocity` não é zerada (original); o fade do `ColorRect` (Marco A) já cobre a queda.

## Requirements *(mandatory)*

### Functional Requirements

**Comportamento — jogador (US1)**

- **FR-001**: A classe do jogador MUST chamar-se `Player` (obrigatório: `red_robot.gd:131,275,281` e `door.gd:10` fazem `is Player`) e ter base `CharacterBody3D`.
- **FR-002**: `player_id` (inteiro, default 1) MUST ser exportado, replicado no spawn, e seu setter MUST guardar o valor e chamar `set_multiplayer_authority(value)` no filho `InputSynchronizer` — funcionando quando chamado antes de o node entrar na árvore (`level.gd:119`).
- **FR-003**: `current_animation` MUST ser exportado como enumeração `Animations {JUMP_UP=0, JUMP_DOWN=1, STRAFE=2, WALK=3}`, default WALK, replicado por frame; `motion` (vetor 2D) MUST continuar acessível por nome para a replicação (`player.tscn:26`).
- **FR-004**: Ao entrar na cena, MUST capturar `orientation` = transform global do `PlayerModel` com origem zerada e a posição inicial; se não for servidor, MUST desligar o processamento por frame.
- **FR-005**: A cada frame de física, no servidor MUST executar `apply_input(delta)`; nos demais peers MUST apenas animar `current_animation`.
- **FR-006**: `animate(anim)` MUST gravar `current_animation` e configurar o `AnimationTree` pelas propriedades dinâmicas `parameters/state/transition_request` ("jump_up" | "jump_down" | "strafe" | "walk"), `parameters/aim/add_amount` (`get_aim_rotation()` em STRAFE, 0 em WALK), `parameters/strafe/blend_position` (`(motion.x, −motion.y)`) e `parameters/walk/blend_position` (`(|motion|, 0)`).
- **FR-007**: `apply_input(delta)` MUST reproduzir, na ordem, a lógica de `player.gd:86-176`: interpolação de `motion` (10×delta); eixos X/Z da base da câmera achatados e normalizados; `airborne_time += delta`; no chão, `land` se `airborne_time > 0.5` e zerar; `on_air = airborne_time > 0.1`; pulo (`velocity.y = 5`, `airborne_time = 0.1`, RPC `jump`) se não `on_air` e `jumping`; `jumping = false` sempre; ramos ar / mira (slerp para a base da câmera, STRAFE, root motion, tiro com cooldown) / andar (slerp para `Basis.looking_at(alvo)` se |alvo| > 0,001, WALK, root motion); `orientation *= root_motion`; velocidade horizontal = `orientation.origin / delta`; gravidade; `move_and_slide` com up +Y; zerar origem e ortonormalizar `orientation`; base global do modelo = `orientation.basis`; respawn se y < −40.
- **FR-008**: O tiro MUST instanciar `player/bullet/bullet.tscn` como filho do pai do jogador (`add_child` com nome legível), posicionar na origem global de `ShootFrom`, orientar com `look_at` para `shoot_target`, adicionar exceção de colisão com o jogador e disparar o RPC `shoot`.
- **FR-009**: Os RPCs `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount)` MUST existir com esses nomes, modo `authority`, `call_local`, transferência `unreliable` (defaults de `@rpc("call_local")`), com os efeitos de `player.gd:179-211`.
- **FR-010**: `add_camera_shake_trauma` MUST chamar `add_trauma(amount)` na câmera referenciada por `player_input.camera_camera` com acesso **tipado** (`CameraNoiseShake`), nunca por chamada dinâmica.
- **FR-011**: O acesso ao `InputSynchronizer` MUST ser tipado (`PlayerInputSynchronizer`): leitura de `motion`, `aiming`, `shooting`, `shoot_target`, `jumping`, escrita de `jumping`, e chamadas a `get_aim_rotation`, `get_camera_base_quaternion`, `get_camera_rotation_basis`.

**Comportamento — bala (US2)**

- **FR-012**: A bala MUST ter base `CharacterBody3D`, velocidade 20, `time_alive` inicial 5 s, `hit` inicial falso, e referências a `AnimationPlayer`, `CollisionShape3D`, `OmniLight3D`.
- **FR-013**: Ao entrar na cena em peer não-servidor, MUST desligar o processamento de física e desabilitar a colisão.
- **FR-014**: A cada frame de física MUST reproduzir `bullet.gd:20-35`: retornar se `hit`; decrementar `time_alive` e explodir (marcando `hit`) se negativo; deslocar `−delta × 20 × basis.z` com `move_and_collide`; em colisão, chamar `hit` por RPC no colisor **se ele tiver esse método** (`has_method` — duck typing do original, preservado), desabilitar colisão, explodir, marcar `hit`.
- **FR-015**: O RPC `explode` (`authority`, `call_local`, `unreliable`) MUST tocar a animação "explode" e ligar `shadow_enabled` na luz quando `Settings.config_file` tem `rendering/shadow_mapping` verdadeiro.
- **FR-016**: O acesso a `Settings` MUST seguir a exceção do Princípio II: obter o autoload dinamicamente em `/root/Settings`, ler `config_file` e, a partir daí, usar a API tipada de `ConfigFile`. O backlog v2 item 1 já cobre isso — não duplicar.
- **FR-017**: `destroy()` MUST existir com esse nome (method track da animação "explode", `bullet.tscn:104`), retornar se não for servidor, senão remover a bala da cena.
- **FR-018**: A replicação de `global_transform` (`bullet.tscn:12`) MUST continuar válida sem edição.

**Comportamento — porta (US3)**

- **FR-019**: A porta MUST ter base `Area3D`, `open` inicial falso, e o handler `_on_door_body_entered(body)` com esse nome (conexão em `door.tscn:37`).
- **FR-020**: Ao entrar um corpo, se não `open` e o corpo é `Player`, MUST tocar "doorsimple_opening" no `AnimationPlayer` do modelo e marcar `open = true`; caso contrário não faz nada.

**Ciclo de porte — comuns aos três (Princípio II)**

- **FR-021**: Cada script MUST virar exatamente uma classe registrada pela extensão nativa, com a mesma base (`CharacterBody3D`, `CharacterBody3D`, `Area3D`).
- **FR-022**: O vínculo MUST ser por troca de `type` na `.tscn` com remoção de `script` e do `ext_resource` órfão; nenhum `.gd` ponte.
- **FR-023**: `.gd` e `.gd.uid` MUST ser apagados no mesmo commit do port.
- **FR-024**: Nomes de métodos expostos, RPCs e propriedades exportadas/replicadas MUST ser idênticos ao GDScript, conferidos contra `player.tscn` (`ServerSynchronizer`: `transform`, `player_id`, `PlayerModel:transform`, `motion`, `current_animation`), `bullet.tscn` (`global_transform`; method track `destroy`) e `door.tscn` (conexão `_on_door_body_entered`).
- **FR-025**: O código portado MUST NOT chamar API customizada de GDScript. As únicas chamadas dinâmicas permitidas são as que o original já faz dinamicamente (`has_method("hit")` + `rpc("hit")` na bala; propriedades dinâmicas do `AnimationTree`, que são API base) e a exceção `Settings`.
- **FR-026**: Cada alteração de código MUST ser seguida de build de debug sem warnings novos.
- **FR-027**: Cada port MUST ser validado em headless: import com carregamento da extensão + execução da(s) cena(s) afetada(s) sem erros novos além da baseline (os 3 do `CLAUDE.md`). Para a porta, a baseline **exclui** o `Node not found` do original — ele deve desaparecer.
- **FR-028**: Um commit por script, mensagem com script portado e cena alterada; a ordem de entrega MUST ser 1 → 2 → 3 e o jogo MUST ficar jogável após cada commit.
- **FR-029**: Os 7 scripts fora do marco MUST permanecer byte a byte intactos; melhorias percebidas MUST ir para `docs/v2-backlog.md` no mesmo commit.

**Correção conservadora de bug do upstream — porta (Princípio I, v1.3.0)**

- **FR-030**: O defeito: `door.gd:6` referencia `DoorModel/AnimationPlayer`, mas o node em `door.tscn:13` chama-se `DoorModel2`; resultado: `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` e a porta nunca abre. Classificação: **bug** (intenção inequívoca contradita pelo resultado).
- **FR-031**: A correção MUST ser mínima: referenciar `DoorModel2/AnimationPlayer`. É PROIBIDO renomear o node da cena, alterar a lógica de `open`, ou tocar em `door.tscn` além da troca de `type` e remoção do script.
- **FR-032**: A correção MUST estar isolada e identificável no código, com comentário `// upstream bug fix: ...` no ponto exato da referência.
- **FR-033**: A mensagem do commit do port da porta MUST mencionar a correção.
- **FR-034**: `docs/upstream-bugs.md` MUST ser criado neste marco (cabeçalho + primeira entrada: defeito, script/cena, correção aplicada, commit).
- **FR-035**: Nenhuma outra correção MUST ser aplicada neste marco; qualquer outro defeito ou melhoria percebida vai para `docs/v2-backlog.md` (ou, se for bug objetivo, é declarado em spec futura).

### Key Entities

- **Player (contrato consumido por GDScript)**: nome de classe `Player`; propriedades `player_id` (int, setter com efeito colateral), `current_animation` (enum), `motion` (vetor 2D, replicado); RPCs `jump`, `land`, `shoot`, `hit`, `add_camera_shake_trauma(amount)`; método interno `animate(anim)` (não consumido por nenhum script externo — chamado apenas de dentro da própria classe e dos RPCs `jump`/`land`; fica privado, como os métodos internos do Marco A); consumidores `red_robot.gd`, `level.gd`, `bullet` (via `has_method`), `door`.
- **Estado interno do jogador**: `airborne_time` (inicial 100), `orientation` e `root_motion` (transforms), `initial_position`; referências de cena `InputSynchronizer`, `AnimationTree`, `PlayerModel`, `ShootFrom` (+ `ShootParticle`, `MuzzleFlash`), `Crosshair`, `FireCooldown`, `SoundEffects/Jump|Land|Shoot`.
- **Bullet**: `time_alive`, `hit`; RPC `explode`; método `destroy` (method track); replicação `global_transform`; dependência dinâmica de `Settings.config_file`.
- **Door**: `open`; handler `_on_door_body_entered`; referência corrigida `DoorModel2/AnimationPlayer`.
- **Registro de bugs do upstream** (`docs/upstream-bugs.md`): entrada por correção — defeito, script/cena, correção aplicada, commit.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ao final do marco o projeto contém exatamente **7** arquivos `.gd` (e 7 `.uid`); os 7 são byte a byte idênticos ao estado anterior ao marco.
- **SC-002**: O jogo abre pelo menu, entra no level e — mover, pular (com som), pousar (com som), mirar, atirar com bala visível que explode e acerta robôs, tremor de câmera, respawn ao cair abaixo de −40 — é indistinguível do original em `../oxide_godot_origins/` numa comparação lado a lado feita pelo usuário.
- **SC-003**: Para cada um dos 3 ports, a validação headless (import + cenas afetadas) reporta zero erros novos além dos 3 catalogados; para a porta, o `ERROR: Node not found` do original **não** aparece mais.
- **SC-004**: Após cada um dos 3 commits o jogo é jogável de ponta a ponta.
- **SC-005**: O histórico do marco tem exatamente 3 commits `Port …` (um por script) e nenhum toca os 7 scripts fora de escopo; o commit da porta menciona a correção do bug.
- **SC-006**: Build de debug sem nenhum warning novo em todos os commits.
- **SC-007**: Nenhuma linha portada introduz abstração, refatoração ou otimização; a única diferença de comportamento em relação ao original é a porta abrir, e ela está isolada por comentário, declarada aqui, no commit e em `docs/upstream-bugs.md` (4/4 requisitos da cláusula).
- **SC-008**: `red_robot.gd`, `level.gd`, `part.gd` e a bala continuam encontrando `Player`, `player_id`, `name`, `hit` e `add_camera_shake_trauma` pelos nomes originais — nenhum aviso de tipo/método/propriedade inexistente nos logs headless nem no editor.
- **SC-009**: `door.tscn` instancia sem erro e, em teste isolado, abre exatamente uma vez para um `Player`.

## Assumptions

- Fase v1 (constituição v1.3.0); Princípio I com a cláusula de correção conservadora usada uma única vez (porta).
- Validação visual (SC-002, SC-009) pelo usuário; validação automatizada exclusivamente headless. O revisor pode rodar um harness de paridade fora do repositório; nada disso entra no código.
- Validação single-player: o peer local é **servidor e autoridade**. Os ramos "cliente" (FR-004, FR-005, FR-013, FR-017) são verificados por leitura de código e headless; multiplayer real com dois peers está fora de escopo.
- Fora de escopo: `part.gd`, `red_robot.gd`, `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`; acesso tipado ao `Settings` (backlog v2 item 1); qualquer outra correção ou melhoria.
- `door.tscn` é asset órfão: a correção não muda nada visível no jogo; sua validação funcional é em cena de teste isolada não commitada.
- O erro `Node not found` da porta não consta do catálogo do `CLAUDE.md`; a regra do Princípio II sobre atualizar o catálogo no mesmo commit fica satisfeita sem alteração (nada a remover) — registrar essa constatação na mensagem do commit da porta.
- Quirks do original preservados propositalmente: `airborne_time = 100` inicial; setter de `player_id` fora da árvore; `jumping` zerado pelo jogador a cada frame; possível duplo `explode` quando a bala expira e colide no mesmo frame; `velocity` não zerada no respawn. Cada um é candidato ao backlog v2, não a correção.
- `BulletCache` em `player.tscn` passa a instanciar a classe nativa da bala; como não é servidor-dependente (é só pré-aquecimento de shaders/recursos), não há efeito observável.
