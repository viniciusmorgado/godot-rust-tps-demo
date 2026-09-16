# Feature Specification: Marco C — inimigo: peça e robô vermelho (v1 raw port)

**Feature Branch**: `003-v1-enemy` (trabalho na `main`, como nos Marcos A e B)

**Created**: 2026-09-15

**Status**: Draft

**Fase**: v1 — Raw Port (Princípio I da constituição v1.3.0). Nenhuma abstração, refatoração ou otimização; melhorias percebidas vão para `docs/v2-backlog.md`. Um defeito objetivo do upstream foi encontrado na revisão do port 1 (peça: material dos escudos compartilhado — ver "Correção conservadora de bug do upstream — peça") e é corrigido sob a cláusula de correção conservadora (declarar em spec, isolar com comentário, mencionar no commit, registrar em `docs/upstream-bugs.md`). Nenhum outro defeito é conhecido.

**Input**: User description: "Portar para Rust o inimigo do Godot TPS Demo — a peça destacável (part.gd) e o robô vermelho (red_robot.gd) — mantendo o jogo jogável e idêntico ao original a cada script. Marco C de docs/port-order.md (itens 9 e 10)."

## Contexto

Continuação do porte script a script. Os Marcos A e B (`specs/001`, `specs/002`, commits até `dcf3816`) deixaram 7 scripts no original e entregaram `Player`, `PlayerInputSynchronizer`, `CameraNoiseShake`, `Blast` e `PartDisappear` como classes nativas. Este marco porta o inimigo completo: a peça que se destaca na morte do robô e o próprio robô. Os 5 scripts restantes (`flying_forklift`, `level`, `menu`, `main`, `settings`) ficam byte a byte intactos e continuam consumindo a API portada pelos nomes originais.

| # | Script original | Base | Node/cena afetada | Linhas | Consumidores que permanecem no original |
|---|---|---|---|---|---|
| 1 | `enemies/red_robot/parts/part.gd` | RigidBody3D | **sem cena própria** — attached em 3 nodes de `enemies/red_robot/red_robot.tscn`: `Death/PartShield1` (l.10833), `Death/PartShield2` (l.10885), `Death/PartHead` (l.10936); `ext_resource id="24"` (l.26) | 57 | `red_robot.gd:96-98` (`explode()` nas 3 peças — GDScript até o port 2) |
| 2 | `enemies/red_robot/red_robot.gd` | CharacterBody3D | raiz de `enemies/red_robot/red_robot.tscn` (l.10584); `ext_resource id="1"` (l.3) | 283 | `level.gd:97-100` (instancia, `transform`, `exploded.connect`, `add_child(robot, true)`); bala (`has_method("hit")` → `rpc("hit")`, já classe nativa) |

Referência de comportamento: o projeto original intocado em `../oxide_godot_origins/`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Peça do robô portada (Priority: P1)

Quando um robô morre, suas três peças (dois escudos e a cabeça) se soltam, voam com rotação aleatória, caem com física, ficam alguns segundos no chão, somem num fade e terminam com um puff de fumaça — exatamente como hoje. Tudo isso passa a ser produzido pela classe nativa da peça, enquanto o robô (ainda no original) continua chamando `explode()` nela pelo nome.

**Why this priority**: A peça é folha (só consome `PartDisappear`, já classe nativa, por API base) e é dependência do robô (`explode()` chamado por nome; no port 2 o acesso vira tipado). Sem ela portada, o robô não pode ser portado com acesso tipado (Princípio II, ordem de baixo para cima).

**Independent Test**: `red_robot.tscn` e `level.tscn` headless sem erros novos; no jogo, matar um robô produz o mesmo espetáculo de peças (voo, queda, fade, puff) do original.

**Acceptance Scenarios**:

1. **Given** `red_robot.tscn` é instanciada, **When** cada uma das 3 peças entra na cena, **Then** o processamento por frame está desligado e, fora de servidor dedicado, a peça passa a ter uma cópia própria do material da superfície 0 do seu modelo (e do `next_pass` dessa cópia), de modo que o fade de uma peça não afeta as outras nem o modelo original.
2. **Given** a peça está congelada na cena (`freeze`), **When** o robô morre e chama `explode()`, **Then** a sincronização da peça passa a ser pública, o congelamento é desligado e, no servidor, as duas colisões são habilitadas, a velocidade linear é 3 unidades/s para cima e a angular é `(normalizado(rand, rand, rand) × 2 − ONE) × 10` (três aleatórios em [0, 1], normalizados, depois × 2 − 1 em cada componente — fórmula exata do FR-003, não um aleatório uniforme em [−1, 1]).
3. **Given** a peça explodiu no servidor, **When** passam `lifetime + lifetime_random × aleatório` segundos (3 a 6 s), **Then** o processamento por frame é ligado e o fade começa.
4. **Given** o fade está ativo, **When** frames passam, **Then** `fade_value = (contador / disappearing_time)²` (0 → 1 em 0,5 s, com o expoente 2) e o valor é aplicado ao parâmetro `emission_cutout` do shader do `next_pass` do material da peça; quando o contador atinge `disappearing_time − 0,2`, o RPC `destroy` é disparado e o processamento é desligado.
5. **Given** o RPC `destroy` roda, **When** executa, **Then** um efeito `part_disappear.tscn` (classe nativa `PartDisappear`) é instanciado como filho do **pai** da peça, na posição global da peça; 0,2 s depois a peça é removida da cena.
6. **Given** o peer não é servidor, **When** `explode()` é chamado, **Then** apenas a visibilidade pública da sincronização e o descongelamento acontecem; colisões, velocidades e o temporizador do fade não (ramo por leitura de código).
7. **Given** a replicação configurada na cena (`fade_value`, `position`, `rotation`, `linear_velocity`, `angular_velocity` — `red_robot.tscn:10419-10431`), **When** a peça é sincronizada, **Then** `fade_value` continua acessível por nome com o setter aplicando o valor ao shader; as demais são propriedades da base.
8. **Given** `red_robot.gd` continua no original até o port 2, **When** chama `death_shield1.explode()` etc. (`red_robot.gd:96-98`), **Then** resolve pelo nome sem edição.

---

### User Story 2 - Robô vermelho portado (Priority: P2)

O robô continua o mesmo inimigo: fica parado até detectar o jogador, vira e anda até ficar de frente, prepara a mira (laser visível, clipado no cenário), atira (impacto no ponto atingido e tremor forte na câmera do jogador se o acertar), reage a cada tiro recebido com animação de dano e som, e morre no quinto tiro — peças voando, faíscas, som de explosão — sendo removido 10 s depois; o level continua recebendo o sinal `exploded` e respawnando outro robô 15 s depois.

**Why this priority**: Maior script do marco e último com dependentes fora dele (`level.gd` via `exploded`; bala via `hit`). Consome `Player` (tipado), `Part` (tipado, port 1) e `Blast` (API base). Fecha o inimigo inteiro como classes nativas.

**Independent Test**: `red_robot.tscn` e `level.tscn` headless sem erros novos; contrato conferido (`hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered/_exited`, `exploded`, propriedades replicadas); no jogo, o ciclo detectar → aproximar → mirar → atirar → ser atingido → morrer → respawn indistinguível do original.

**Acceptance Scenarios**:

1. **Given** o level instancia o robô (`level.gd:97-100`: `transform` atribuído, `exploded` conectado, `add_child(robot, true)`), **When** o node entra na cena, **Then** `orientation` = transform global com origem zerada, o `AnimationTree` fica ativo, `shoot_countdown` = 0 se `test_shoot`, e se `dead` o modelo fica invisível, a colisão desabilitada e o `AnimationTree` inativo; `animate(0)` roda uma vez.
2. **Given** o robô está IDLE e o jogador entra na `PlayerDetectionArea`, **When** `body_entered` dispara `_on_area_body_entered(body)`, **Then** se `body` é `Player` **ou** `body.name == "Target"`, `player = body` e `state = APPROACH`; ao sair, se `body` é `Player`, `player = null` e `state = IDLE`.
3. **Given** o robô está em APPROACH com o jogador fora da tolerância de ±15°, **When** frames de física passam, **Then** a animação é `turn_left` (ângulo > 15°) ou `turn_right` (< −15°); dentro da tolerância, `walk`; sem alvo (`target_position` zero), `idle`; fora de APPROACH, sempre `idle`. O ângulo é `atan2(x, z)` do alvo no espaço local do robô (frente = +Z).
4. **Given** o robô está de frente para o jogador em APPROACH, **When** `shoot_countdown` (6 s, decrementado só de frente) fica negativo, **Then** um raio da origem de `RayFrom` até a origem do jogador + UP (máscara 0xFFFFFFFF, excluindo o próprio robô) é lançado; se atinge o jogador: `state = AIM`, `aim_countdown = 1 s`, `aim_preparing = 0`; senão `shoot_countdown = 6 s`.
5. **Given** o robô está em AIM ou SHOOTING, **When** frames passam, **Then** o laser é clipado na distância do `RayCast` (ou 1000), `aim_preparing` sobe até 0,5 s, `aim_countdown` decrementa; ao ficar negativo em AIM, o mesmo raio é lançado: se atinge o jogador, `state = SHOOTING`, `shoot_countdown = 6 s` e o RPC `play_shoot` toca a animação "shoot"; senão `resume_approach()` (APPROACH, `aim_preparing = 0,5`, `shoot_countdown = 6`).
6. **Given** a animação "shoot" está tocando, **When** seus method tracks disparam (`shoot_check` aos 2,25 s, `resume_approach` aos 3 s — `red_robot.tscn:10296-10299`), **Then** `shoot_check` marca `test_shoot = true`, o próximo frame de física chama `shoot()` e zera `test_shoot`; `resume_approach` volta a APPROACH.
7. **Given** `shoot()` roda, **When** o raio da origem de `RayFrom` na direção do eixo Y da sua base global (alcance 1000, máscara 0xFFFFFFFF, excluindo o robô) colide, **Then** `max_dist` = distância ao ponto; o laser é clipado em `max_dist`; `LaserEmber` é posicionado em `(0, 0, −max_dist/2 − offset_z)` com `emission_box_extents.z = (max_dist − |offset_z|)/2`; um `impact_effect.tscn` (classe nativa `Blast`) é instanciado como filho da raiz da árvore na posição do impacto; se o colisor é o `player` e ele é `Player`, 0,1 s depois `add_camera_shake_trauma(13.0)` é chamado nele com acesso tipado.
8. **Given** o alvo existe (`target_position ≠ 0`), **When** `animate` roda, **Then** `parameters/aiming/blend_amount = clamp(aim_preparing / 0,5, 0, 1)` e `parameters/aim/blend_position` é ajustado incrementalmente (`0,05 × delta × −h_angle` em x, `0,05 × delta × v_angle` em y, ambos em graus, clampados em [−1, 1]) a partir dos ângulos do alvo + UP no espaço local do `RayMesh`.
9. **Given** o robô é atingido por uma bala (`hit` por RPC), **When** roda, **Then** se `dead` retorna; senão uma das três animações de dano (`parameters/hit{1|2|3}/request = 1`, sorteio) dispara, o som Hit toca e `health` decrementa.
10. **Given** `health` chega a 0, **When** `hit` roda, **Then** `dead = true`, `AnimationTree` inativo, modelo invisível, `Death` visível, colisão desabilitada, `DetachSpark1/2` emitindo, `explode()` nas 3 peças com acesso **tipado** (`Part`), som Explosion, sinal `exploded` emitido; no servidor, 10 s depois o robô é removido da cena.
11. **Given** `level.gd:99` conectou `exploded` a `_respawn_robot`, **When** o sinal é emitido, **Then** o level espera 15 s e spawna outro robô no mesmo ponto — sem edição em `level.gd`.
12. **Given** não há jogador detectado, **When** frames de física passam no servidor, **Then** `target_position = 0`, `animate`, velocidade = gravidade × delta, `move_and_slide` com up = +Y, e o restante é pulado.
13. **Given** o peer não é servidor, **When** frames de física passam, **Then** apenas `animate(delta)` roda (ramo por leitura de código); se `dead`, nada roda.
14. **Given** a replicação configurada (`red_robot.tscn:30-42`: `global_transform`, `health`, `state`, `target_position`, `dead`), **When** o robô é sincronizado, **Then** `health`, `state`, `target_position`, `dead` continuam acessíveis por nome; `aim_preparing` e `test_shoot` são exportados mas não replicados (como no original).
15. **Given** `level.gd` e a bala continuam como estão, **When** usam `exploded`, `hit`, `transform`, **Then** tudo resolve sem edição.

---

### Edge Cases

- **Peça: `fade_value` antes do material existir**: o setter só aplica ao shader se o material já foi duplicado em `ready`; valores atribuídos antes (replicação no spawn, inspector) só ficam guardados — comportamento do original.
- **Peça: servidor dedicado**: sem duplicação de material (feature `dedicated_server`); o fade não tem efeito visual. Ramo por leitura de código.
- **Peça: `explode()` antes de `ready`**: não ocorre (o robô só chama na morte, muito depois do spawn).
- **Peça: puff instanciado no pai da peça** (`Death`, filho do robô): o robô é removido 10 s após a morte e o fade das peças termina em ≤ 6,5 s (+ 0,2 s do puff), então o efeito nasce e some antes disso; se os tempos se cruzassem, o puff sairia junto com o robô. Comportamento do original, preservado.
- **Robô: `body.name == "Target"`**: nenhuma cena do projeto tem node chamado `Target` (grep vazio); o ramo é preservado como código morto do original.
- **Robô: `player` tipado como `Node3D`**: pode ser um `Target` (não `Player`); por isso `shoot()` testa `player is Player` antes do `add_camera_shake_trauma`. A tipagem da referência permanece `Node3D`; só a chamada é tipada após o teste.
- **Robô: `hit` durante `await` dos 10 s**: `dead` já é `true`, retorna. Balas que ainda colidem chamam `hit` sem efeito.
- **Robô: `shoot()` com `col.collider == player`**: o `pass # Kill.` do original não faz nada — preservado (nada de "implementar a morte do jogador").
- **Robô: exclusão no raycast**: `[self]` exclui o próprio corpo (RID válido do `CharacterBody3D`) — aqui a exclusão **é** efetiva, diferente do quirk do `player_input.gd`; preservar a semântica (excluir o robô).
- **Robô: `orientation` vs `global_transform`**: o robô grava `global_transform.basis = orientation.basis` no próprio corpo (não num modelo filho, como o jogador).
- **Robô morto no spawn (`dead = true` replicado)**: modelo invisível e sem colisão desde `ready`; `_physics_process` retorna sempre.
- **Robô: `hit` com `health` já negativo**: só `== 0` dispara a morte; se um frame com duas balas levar `health` a −1, o segundo `hit` já vê `dead` — original.

## Requirements *(mandatory)*

### Functional Requirements

**Comportamento — peça (US1)**

- **FR-001**: A peça MUST ter base `RigidBody3D` e exportar `lifetime` (3,0), `lifetime_random` (3,0), `disappearing_time` (0,5) e `fade_value` (0,0); o setter de `fade_value` MUST guardar o valor e, se o material já existir, definir o parâmetro de shader `emission_cutout` do `next_pass` do material.
- **FR-002**: Ao entrar na cena MUST desligar o processamento por frame e, fora de servidor dedicado, duplicar o material da superfície 0 do mesh do primeiro filho de `Model`, atribuí-lo à superfície 0 e duplicar o `next_pass` da cópia.
- **FR-003**: `explode()` MUST existir com esse nome (chamado por `red_robot.gd:96-98`): tornar pública a sincronização do `MultiplayerSynchronizer` filho, descongelar; se não servidor, retornar; habilitar `Col1`/`Col2`; `linear_velocity = 3 × UP`; `angular_velocity = (aleatório normalizado × 2 − ONE) × 10`; esperar `lifetime + lifetime_random × aleatório` s e ligar o processamento.
- **FR-004**: Por frame MUST fazer `fade_value = (contador / disappearing_time)²`, incrementar o contador por `delta` e, ao atingir `disappearing_time − 0,2`, disparar o RPC `destroy` e desligar o processamento.
- **FR-005**: O RPC `destroy` (`authority`, `call_local`, `unreliable`) MUST instanciar `part_disappear.tscn` como filho do pai da peça na posição global da peça, esperar 0,2 s e remover a peça. Só API base de `CpuParticles3D`/`Node3D` é usada no efeito.
- **FR-006**: O carregamento de `part_disappear.tscn` MUST ocorrer por `load` no ponto de uso (padrão do Marco B para `preload`).

**Comportamento — robô (US2)**

- **FR-007**: O robô MUST ter base `CharacterBody3D`, sinal `exploded` (sem argumentos), enum `State {IDLE=0, APPROACH=1, AIM=2, SHOOTING=3}`, constantes `PLAYER_AIM_TOLERANCE_DEGREES` = 15° em radianos, `SHOOT_WAIT` 6,0, `AIM_TIME` 1,0, `AIM_PREPARE_TIME` 0,5, `BLEND_AIM_SPEED` 0,05.
- **FR-008**: MUST exportar `test_shoot` (false), `target_position` (zero), `health` (5), `state` (IDLE), `dead` (false), `aim_preparing` (0,5); `target_position`, `health`, `state`, `dead` MUST continuar acessíveis por nome para a replicação (`red_robot.tscn:30-42`).
- **FR-009**: Estado interno: `shoot_countdown` (6,0), `aim_countdown` (1,0), `player` (referência a `Node3D`, nula), `orientation`; referências de cena `AnimationTree`, `ShootAnimation`, `RedRobotModel`, `RedRobotModel/Armature/Skeleton3D/RayFrom` (+ `RayMesh`, `RayCast`, `LaserEmber`), `CollisionShape3D`, `SoundEffects/Explosion`, `SoundEffects/Hit`, `Death` (+ `PartShield1`, `PartShield2`, `PartHead`, `DetachSpark1`, `DetachSpark2`).
- **FR-010**: Ao entrar na cena MUST reproduzir `red_robot.gd:57-69` (orientação, `AnimationTree` ativo, `test_shoot` → `shoot_countdown = 0`, ramo `dead`, `animate(0)`).
- **FR-011**: `resume_approach()` e `shoot_check()` MUST existir com esses nomes (method tracks da animação "shoot", `red_robot.tscn:10296-10299`) e os efeitos de `red_robot.gd:72-75,264-265`.
- **FR-012**: O RPC `hit` (`authority`, `call_local`, `unreliable`) MUST reproduzir `red_robot.gd:78-105`: retorno se `dead`; animação de dano sorteada (`parameters/hit{1|2|3}/request = 1`); som Hit; `health −= 1`; morte em `health == 0` (flags, visibilidade, colisão, faíscas, `explode()` **tipado** nas 3 peças, som Explosion, sinal `exploded`); no servidor, remoção após 10 s.
- **FR-013**: `shoot()` MUST reproduzir `red_robot.gd:108-133`: raio de `RayFrom` na direção do eixo Y da sua base global, alcance 1000, máscara 0xFFFFFFFF, excluindo o próprio robô (exclusão efetiva); clip do laser; posição e extensão do `LaserEmber`; instanciação de `impact_effect.tscn` (classe `Blast`, API base) como filho da raiz da árvore no ponto de impacto; se `col.collider == player` e `player` é `Player`, após 0,1 s `add_camera_shake_trauma(13.0)` com acesso tipado. O `pass # Kill.` MUST permanecer sem efeito.
- **FR-014**: `animate(delta)` MUST reproduzir `red_robot.gd:136-168`, incluindo a transformação inversa (`target_position * global_transform` do GDScript = ponto no espaço local do robô; idem para o `RayMesh`), `atan2(x, z)` para o ângulo ao jogador, `atan2(x, −z)`/`atan2(y, −z)` em graus para a mira, e a leitura/escrita de `parameters/aim/blend_position`.
- **FR-015**: O frame de física MUST reproduzir `red_robot.gd:171-256` na ordem: `dead` → return; não servidor → `animate` e return; `test_shoot` → `shoot()`; sem `player` → alvo zero, `animate`, gravidade, `move_and_slide`, return; APPROACH (decremento de `aim_preparing`, tolerância, `shoot_countdown`, raycast, transição AIM); AIM/SHOOTING (clip, `aim_preparing`, `aim_countdown`, raycast, transição SHOOTING + RPC `play_shoot` ou `resume_approach`); `animate`; root motion; velocidade; gravidade; `move_and_slide`; ortonormalização; `global_transform.basis = orientation.basis`.
- **FR-016**: O RPC `play_shoot` (`authority`, `call_local`, `unreliable`) MUST tocar "shoot" no `ShootAnimation`; `_clip_ray(length)` MUST, fora de servidor dedicado, definir o parâmetro `clip` do material override da superfície 0 do `RayMesh` como `length + offset_z`.
- **FR-017**: `_on_area_body_entered(body)` e `_on_area_body_exited(body)` MUST existir com esses nomes (conexões `red_robot.tscn:11050-11051`) e a lógica de `red_robot.gd:274-283` (`is Player` ou `name == "Target"`; `is Player`).
- **FR-018**: O acesso ao `Player` (`add_camera_shake_trauma`) e à peça (`explode`) MUST ser tipado; para isso, e só para isso, a visibilidade de `add_camera_shake_trauma` no jogador portado e de `explode` na peça pode ser aberta internamente (nunca para GDScript além do que já é exposto) no commit do robô (precedente do Marco B: só visibilidade, citada na mensagem). O código portado MUST NOT chamar API customizada de GDScript (o robô não consome nenhum script remanescente; `Settings` não é usado). Os `blast`/`puff` instanciados MUST ser tipados pela base (`Node3D`/`CpuParticles3D`), nunca por `Blast`/`PartDisappear`.
- **FR-019**: `impact_effect.tscn` MUST ser carregado por `load` no ponto de uso (padrão `preload` → `load`).

**Ciclo de porte — comuns aos dois (Princípio II)**

- **FR-020**: Cada script MUST virar exatamente uma classe nativa com a mesma base (`RigidBody3D`, `CharacterBody3D`).
- **FR-021**: O vínculo MUST ser por troca de `type` na `.tscn`: no port 1, nos 3 nodes `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` (remoção das 3 linhas `script = ExtResource("24")` e do `ext_resource id="24"`); no port 2, na raiz `RedRobot` (remoção de `script = ExtResource("1")` e do `ext_resource id="1"`). Nenhum `.gd` ponte. Propriedades da base nos 3 nodes de peça (`transform`, `collision_layer/mask`, `mass`, `physics_material_override`, `freeze = true`, `angular_damp`) e seus filhos (`MultiplayerSynchronizer` com `public_visibility = false`, `Model`, `Col1`, `Col2`) MUST ficar intactos.
- **FR-022**: `.gd` e `.gd.uid` MUST ser apagados no mesmo commit do port.
- **FR-023**: Nomes de métodos expostos, RPCs, sinal e propriedades exportadas/replicadas MUST ser idênticos ao GDScript: peça — `explode`, `destroy`, `lifetime`, `lifetime_random`, `disappearing_time`, `fade_value`; robô — `hit`, `play_shoot`, `shoot_check`, `resume_approach`, `_on_area_body_entered`, `_on_area_body_exited`, `exploded`, `test_shoot`, `target_position`, `health`, `state`, `dead`, `aim_preparing`. Conferidos contra `red_robot.tscn` (replicação l.30-42 e l.10419-10431; method tracks l.10296-10299; conexões l.11050-11051) e `level.gd:99`.
- **FR-024**: Cada alteração de código MUST ser seguida de build de debug sem warnings novos.
- **FR-025**: Cada port MUST ser validado em headless: import com carregamento da extensão + `red_robot.tscn` e `level.tscn` sem erros novos além da baseline (os 3 do `CLAUDE.md`).
- **FR-026**: Um commit por script, na ordem 1 → 2; o jogo MUST ficar jogável após cada commit.
- **FR-027**: Os 5 scripts fora do marco MUST permanecer byte a byte intactos; melhorias percebidas MUST ir para `docs/v2-backlog.md` no mesmo commit; nenhuma correção de bug é prevista — se um defeito objetivo surgir, MUST seguir os 4 requisitos da cláusula (spec, comentário, commit, `docs/upstream-bugs.md`) ou, em dúvida, ir para o backlog como melhoria.

**Correção conservadora de bug do upstream — peça (Princípio I, v1.3.0)**

- **FR-028**: O defeito: `part.gd:23-26` duplica o material da superfície 0 e o instala com `mesh.mesh.surface_set_material(0, _mat)` — no **recurso `Mesh`**, que é compartilhado por `Death/PartShield1` e `Death/PartShield2` (mesma cena de modelo, `red_robot.tscn` ext_resource id="12"). Resultado (verificado no original em headless): o segundo escudo a entrar duplica o material já instalado pelo primeiro e o reinstala no mesmo `Mesh`; o mesh passa a renderizar o material do escudo 2 para **ambos**; o `fade_value` do escudo 1 vai para um material que ninguém renderiza (o escudo 1 some sem fade) e o do escudo 2 esmaece os dois. A cabeça (`PartHead`) tem mesh próprio e não é afetada. Intenção inequívoca do código (cópia própria do material por peça, para o fade ser independente) contradita pelo resultado → **bug**.
- **FR-029**: A correção MUST ser mínima: instalar a cópia como *override* de superfície na instância (`MeshInstance3D.set_surface_override_material(0, cópia)`) em vez de mutar o `Mesh` compartilhado. Nada mais muda: a duplicação do material e do `next_pass`, o setter de `fade_value`, `explode`, `process`, `destroy` e a cena ficam como estão. É PROIBIDO tornar o mesh `resource_local_to_scene`, editar `red_robot.tscn` ou os modelos, ou reestruturar a peça.
- **FR-030**: A correção MUST estar isolada e identificável, com comentário `// upstream bug fix: ...` na linha imediatamente acima da chamada substituída, em commit próprio `Fix port part.gd …` (não reescrever o commit `9aee2b8`), cuja mensagem menciona a correção.
- **FR-031**: `docs/upstream-bugs.md` MUST receber a entrada #2 (defeito, script/cena, correção aplicada, commit por assunto) no mesmo commit.
- **FR-032**: Resultado esperado após a correção: os três `MeshInstance3D` das peças renderizam materiais distintos (override por instância; `get_surface_override_material(0)` distinto entre as peças e distinto do material do `Mesh`, que fica intocado), e o `emission_cutout` renderizado de cada peça acompanha o seu próprio `fade_value`.

### Key Entities

- **Part (contrato consumido por GDScript até o port 2 e pela cena)**: métodos `explode()` (chamado por nome por `red_robot.gd`), RPC `destroy()`; propriedades exportadas `lifetime`, `lifetime_random`, `disappearing_time`, `fade_value` (setter com efeito no shader; replicada); estado interno `_mat` (material duplicado), `_disappearing_counter`; sem cena própria — 3 instâncias em `red_robot.tscn`.
- **EnemyRobot (contrato consumido por `level.gd`, pela bala e pela cena)**: sinal `exploded`; RPCs `hit`, `play_shoot`; métodos `shoot_check`, `resume_approach` (method tracks), `_on_area_body_entered`, `_on_area_body_exited` (conexões); propriedades exportadas `test_shoot`, `target_position`, `health`, `state` (enum), `dead`, `aim_preparing`; métodos internos `shoot`, `animate`, `_clip_ray` (não consumidos externamente — privados, como nos marcos anteriores).
- **Estado interno do robô**: `shoot_countdown`, `aim_countdown`, `player: Node3D`, `orientation`; 15 referências de cena listadas em FR-009.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ao final do marco o projeto contém exatamente **5** arquivos `.gd` (e 5 `.uid`); os 5 são byte a byte idênticos ao estado anterior ao marco.
- **SC-002**: No jogo, o ciclo do robô — parado → detecta o jogador → vira e anda → mira com laser clipado no cenário → atira (impacto no ponto, tremor 13,0 se acertar) → recebe tiros (animação de dano + som) → morre no 5º tiro (peças voam com rotação, faíscas, som; peças somem com fade e puff entre 3 e 6,5 s) → respawn 15 s depois — é indistinguível do original em `../oxide_godot_origins/` numa comparação lado a lado feita pelo usuário.
- **SC-003**: Para cada um dos 2 ports, a validação headless (import + `red_robot.tscn` + `level.tscn`) reporta zero erros novos além dos 3 catalogados.
- **SC-004**: Após cada um dos 2 commits o jogo é jogável de ponta a ponta.
- **SC-005**: O histórico do marco tem exatamente 2 commits `Port …` e nenhum toca os 5 scripts fora de escopo.
- **SC-006**: Build de debug sem nenhum warning novo em todos os commits.
- **SC-007**: Nenhuma linha portada introduz abstração, refatoração ou otimização; a única correção é a da peça (FR-028–FR-032), com os 4 requisitos da cláusula atendidos; `docs/upstream-bugs.md` passa a ter 2 entradas.
- **SC-008**: `level.gd` continua recebendo `exploded` e respawnando; a bala continua acertando o robô via `hit`; `red_robot.gd` (até o port 2) continua chamando `explode()` nas peças — nenhum aviso de método/propriedade inexistente nos logs headless nem no editor.

## Assumptions

- Fase v1 (constituição v1.3.0); uma correção conservadora de bug (peça, FR-028–FR-032), descoberta na revisão do port 1 e aplicada em commit `Fix port …` separado.
- Validação visual (SC-002) pelo usuário; validação automatizada exclusivamente headless (`red_robot.tscn` isolada instancia o robô sem jogador: IDLE + gravidade; `level.tscn` exercita spawn e conexão de `exploded`). O revisor pode rodar um harness de paridade fora do repositório.
- Validação single-player: o peer local é servidor e autoridade; ramos "cliente" (FR-003, FR-005, FR-012, FR-015) e "servidor dedicado" (FR-002, FR-016) são verificados por leitura de código.
- Fora de escopo: `flying_forklift.gd`, `level.gd`, `menu.gd`, `main.gd`, `settings.gd`; qualquer melhoria (backlog v2); multiplayer real com dois peers.
- `part.gd` não tem cena própria: o "port" da peça é a troca de `type` nos 3 nodes de `red_robot.tscn`; as propriedades exportadas com valor default não estão gravadas na cena (conferir antes de editar — se alguma estiver, mantê-la).
- `red_robot.tscn` tem 11.053 linhas; as edições são de texto em linhas conferidas por `grep` antes de cada alteração.
- Quirks do original preservados propositalmente (candidatos ao backlog v2, não a correção): `body.name == "Target"` (código morto); `pass # Kill.` em `shoot()`; `player` tipado como `Node3D`; `aim_preparing`/`test_shoot` exportados sem replicação; `await` de 10 s dentro do RPC `hit`; efeito de puff instanciado no pai da peça (`Death`), que some com o robô.
- A abertura de visibilidade interna para acesso tipado (`add_camera_shake_trauma` no jogador; `explode` na peça) segue o precedente do Marco B: só visibilidade, no commit que a exige, citada na mensagem.
