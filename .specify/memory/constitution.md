<!--
SYNC IMPACT REPORT
Version change: 1.3.0 → 1.3.1 (PATCH — dois esclarecimentos no Princípio II; nenhuma regra nova,
nenhuma removida, nenhuma redefinição)

Princípios:
  Esclarecido: II. Ciclo de Porte Verificável → "Vínculo por tipo, nunca por script", primeiro
  bullet: quando o `extends` do script é ANCESTRAL do tipo do node ao qual está attached na cena,
  a base da classe Rust é o tipo do node na cena (a troca de `type` não rebaixa o node). Caso que
  motivou: `flying_forklift.gd` (Marco D) — `extends Node3D` attached a um node `CharacterBody3D`
  com `CollisionShape3D` filho.
  Esclarecido: II. Ciclo de Porte Verificável → "Preservação de nomes de propriedades", novo
  bullet ao final: o nome da classe Rust registrada não pode coincidir com classe do engine nem com
  identificador de topo (`const`, `class_name`, `var`) de nenhum `.gd` remanescente — o GDScript
  rejeita o script inteiro ("The member X shadows a native class"). Caso que motivou: `RedRobot`
  → `EnemyRobot` (Marco C) — colisão com `const RedRobot` em `level.gd:6`.
  Demais bullets de ambos os princípios mantidos intactos por instrução explícita.

Seções: Governança inalterada.

Templates verificados (não modificados — fora do escopo deste comando):
  - .specify/templates/plan-template.md: gate "Constitution Check" segue válido; plans devem
    conferir a base efetiva do node e a ausência de colisão de nome antes do Phase 0.
  - .specify/templates/spec-template.md, tasks-template.md, checklist-template.md: nenhuma
    referência direta à constituição; nenhuma ação necessária.
  - CLAUDE.md: já contém o comando de conferência de colisão de nomes (regra operacional, não
    constitucional); nenhuma alteração necessária.

Itens adiados (TODO): nenhum.
-->

# Constituição Oxide Godot

## Princípios Fundamentais

### I. Porte em Três Fases (v1 → v2 → v3)

Este projeto é o porte do Godot TPS Demo (GDScript) para Rust via godot-rust/gdext, e evolui em três fases sequenciais, cada uma com um objetivo distinto e não negociável:

**v1 — Raw Port (branch `main`)**
- Objetivo: fazer o jogo rodar INTEIRAMENTE em Rust, com comportamento idêntico ao original em GDScript, script por script.
- O código DEVE ser uma tradução direta do GDScript. Fica explicitamente PROIBIDO nesta fase: remodelar nodes, criar gerenciador de cenas, redesenhar structs/enums, introduzir abstrações, otimizar performance ou "idiomatizar" o Rust.
- Código funcional porém não idiomático ("Rust com cara de GDScript", como acontece em portes C++ → Rust) é o resultado ESPERADO e aceito. Melhorias percebidas durante o porte devem ser anotadas para a v2, nunca aplicadas na v1.
- A v1 NÃO servirá como template ou referência para projetos futuros. Ela é preservada para fins históricos, consulta, baseline de benchmark e exemplo de raw port.
- Critério de conclusão: nenhum script `.gd` restante no projeto e o jogo jogável de ponta a ponta.

**v2 — Rust Idiomático (branch `v2`, criada a partir do ponto de conclusão da v1)**
- Objetivo: remodelar structs, enums e demais construções tirando proveito real do sistema de tipos e das funcionalidades do Rust, melhorando performance e legibilidade.
- Inclui adicionar novas funcionalidades de infraestrutura, como um gestor centralizado de cenas orientado a signals, entre outras que tornem o projeto um template usável para um projeto real.
- A v2 é um dos dois templates de referência definitivos: o de Rust "puro" sobre nodes do Godot.

**v3 — ECS (branch `v3`, criada a partir da v2)**
- Objetivo: adicionar `bevy_ecs` e remodelar o que foi feito na v2 em torno de uma abordagem ECS no topo dos nodes do Godot.
- A v3 NÃO substitui a v2: as duas coexistem como templates de referência independentes e não se misturam — v2 para Rust puro, v3 para ECS.

**Regras de governança das fases**
- As fases são estritamente sequenciais: nenhum trabalho de v2 começa antes da v1 ser declarada concluída, e nenhum de v3 antes da v2.
- Cada spec, plan e task DEVE declarar a que fase pertence e respeitar as restrições daquela fase. Uma task de v1 que introduza abstração, refatoração ou otimização viola esta constituição e deve ser rejeitada ou movida para o backlog da v2.
- Notas de melhoria identificadas durante a v1 são registradas como backlog para a v2, sem serem implementadas.
- Melhorias identificadas durante a v1 DEVEM ser registradas em `docs/v2-backlog.md` (uma entrada por melhoria, com o script/cena de origem e a motivação) no mesmo commit em que foram percebidas. "Anotar" sem registrar nesse arquivo não satisfaz esta regra.
- **Correção conservadora de bugs do upstream (exceção explícita ao "sem melhorias" da v1)**: um defeito objetivo do demo original — comportamento que o próprio código claramente pretendia e não entrega (ex.: caminho de node inexistente, referência nula, nome de animação errado) — PODE ser corrigido na v1. A correção DEVE ser conservadora: apenas o suficiente para o bug deixar de ocorrer, sem adentrar em grandes modificações em relação à fonte original; nada de reestruturar, extrair, renomear ou "aproveitar" para melhorar o código ao redor. Melhorias e refatorações idiomáticas continuam reservadas à v2.
- Critério para distinguir bug de melhoria: é bug quando a intenção do código original é inequívoca e o resultado observado a contradiz (a porta deveria abrir e não abre). É melhoria — e portanto proibida na v1 — quando o original funciona como escrito e a mudança o tornaria "melhor" (exclusão real no raycast, recapturar rotação da câmera, não recalcular texto oculto). Em caso de dúvida, é melhoria: vai para o backlog v2.
- Toda correção de bug na v1 DEVE ser (a) declarada na spec da feature com a descrição do defeito e da correção mínima, (b) implementada de forma isolada e identificável dentro do commit do port (comentário `// upstream bug fix: ...` no ponto exato), (c) mencionada na mensagem do commit, e (d) registrada em `docs/upstream-bugs.md` (defeito, script/cena, correção aplicada, commit) para servir de referência à v2 e a eventual contribuição upstream.

### II. Ciclo de Porte Verificável

Todo script GDScript portado segue um ciclo fixo, e um port só é considerado concluído quando todas as etapas abaixo foram cumpridas e evidenciadas:

**Vínculo por tipo, nunca por script**
- Cada script `.gd` DEVE virar exatamente uma classe Rust registrada via gdext, com a MESMA classe base do script original (ex.: `extends CharacterBody3D` → `#[class(base=CharacterBody3D)]`). Esclarecimento: quando o `extends` do script é um ANCESTRAL do tipo do node ao qual ele está attached na cena (ex.: `extends Node3D` num node `CharacterBody3D`), a base da classe Rust é o tipo do node na cena — a troca de `type` não pode rebaixar o node nem descartar o que a cena lhe deu (corpo físico, filhos que dependem do tipo). O `extends` de ancestral é apenas uma declaração mais frouxa do GDScript; o comportamento observável vem do node. A spec do port DEVE registrar o caso quando ocorrer.
- A classe Rust é vinculada à cena trocando o `type` do node na `.tscn` (equivalente ao "Change Type" do editor) e removendo o `script` e o `ext_resource` do `.gd`. É PROIBIDO manter um `.gd` attached ao node como ponte, wrapper ou fallback.
- Métodos expostos com `#[func]` DEVEM preservar o nome original do GDScript, para que as `[connection]` das cenas, chamadas `has_method()` e `.rpc()` existentes continuem válidas sem edição.
- O `.gd` e seu `.gd.uid` DEVEM ser removidos no mesmo commit em que o node passa a usar a classe Rust.

**Ordem de porte de baixo para cima**
- Um script só pode ser portado depois que todos os scripts dos quais ele depende (via método, propriedade, enum, signal ou verificação `is Tipo`) já estejam em Rust. Código Rust NÃO DEVE chamar API customizada de um script GDScript (`.call()`, `.get()`, `.set()` dinâmicos, comparação de `get_script()`), pois isso produz código sem tipo, impossível de validar e descartado no passo seguinte.
- Chamadas que usam apenas API base do Godot (métodos e propriedades da classe base do node) não constituem dependência e são livres em qualquer direção.
- Duck typing que já existe no GDScript original (`has_method`, `has_signal`, `.rpc()` por nome) PODE ser preservado como está na v1, por ser tradução direta e não dependência nova.
- Exceção única e documentada: o autoload `Settings` é portado por último, e até lá consumidores em Rust PODEM acessá-lo dinamicamente via `/root/Settings`, com o item correspondente registrado no backlog da v2.
- O grafo de dependências e a ordem derivada dele ficam em `docs/port-order.md`; esse documento é insumo de planejamento, não parte desta constituição, e pode ser revisado sem emenda.

**Preservação de nomes de propriedades**
- Além dos nomes de métodos, toda propriedade exportada (`@export`) ou replicada por `MultiplayerSynchronizer` (listada em `SceneReplicationConfig` nas cenas) DEVE manter o nome original do GDScript ao virar `#[export]`/`#[var]`. Renomear uma dessas propriedades quebra silenciosamente valores salvos nas cenas e a replicação multiplayer, sem erro em tempo de compilação nem de import.
- Antes de concluir um port, o autor DEVE conferir na `.tscn` afetada quais propriedades do script são referenciadas (valores exportados, `properties/N/path` de replicação, `node_paths`) e garantir que todas existem na classe Rust com o mesmo nome e tipo compatível.
- O NOME da classe Rust registrada NÃO pode coincidir com nenhuma classe do engine nem com identificador de topo (`const`, `class_name`, `var`) de qualquer `.gd` que ainda exista no projeto: o GDScript rejeita o script inteiro ("The member X shadows a native class") e o jogo deixa de carregar. Conferir antes de nomear (comando no `CLAUDE.md`); em caso de colisão, o nome do port muda — nunca o `.gd` remanescente.

**Build obrigatório**
- Toda alteração em código Rust DEVE ser seguida de `cargo build` (perfil debug) com sucesso e sem warnings novos, antes de qualquer validação ou commit. A biblioteca dinâmica em `target/debug/` é o artefato que o Godot carrega; código não compilado não existe para o jogo.

**Validação headless como definition of done**
- Nenhum port é dado como concluído sem validação executada em Godot headless: no mínimo (a) o import do projeto confirmando o carregamento da extensão (`Initialize godot-rust ...`) e (b) a execução da cena afetada sem erros novos em relação à baseline conhecida do demo upstream.
- Erros pré-existentes do demo upstream não contam como regressão, mas DEVEM estar catalogados no guia operacional (`CLAUDE.md`) para serem distinguíveis de erros introduzidos pelo port.
- O commit de cada port DEVE registrar em sua mensagem qual script foi portado e qual(is) cena(s) tiveram o tipo do node trocado.
- Erros do upstream que deixam de existir por correção de bug DEVEM ser removidos do catálogo de erros pré-existentes do `CLAUDE.md` no mesmo commit, para que o catálogo reflita sempre a baseline vigente.

**Separação entre regra e operação**
- Esta constituição define O QUE é obrigatório. Caminhos de binários, comandos exatos, versões de ferramentas e notas de API pertencem ao `CLAUDE.md` na raiz do repositório e NÃO devem ser incorporados aqui; mudanças nesses detalhes não constituem emenda.

## Governança

Esta constituição tem precedência sobre qualquer outra prática, convenção, documentação ou decisão ad-hoc dentro do projeto. Em caso de conflito entre esta constituição e qualquer outro documento ou hábito de código, esta constituição prevalece.

**Processo de emenda**: qualquer alteração a esta constituição é feita através do comando `/speckit-constitution`, exige justificativa explícita para a mudança e resulta em uma nova versão do documento, acompanhada do respectivo Sync Impact Report.

**Política de versionamento**: esta constituição segue versionamento semântico (MAJOR.MINOR.PATCH):
- MAJOR: remoção ou redefinição incompatível de um princípio ou de uma das fases v1/v2/v3 (ex.: eliminar uma fase, alterar sua ordem, ou remover uma restrição não negociável).
- MINOR: adição de um novo princípio ou seção, ou expansão material das regras de uma fase existente.
- PATCH: esclarecimentos, correções de redação ou ajustes não semânticos.

**Revisão de conformidade**: toda spec, plan e task DEVE declarar explicitamente a fase (v1, v2 ou v3) à qual pertence, conforme o Princípio I. Revisões de planejamento e de código DEVEM verificar que o trabalho respeita as restrições da fase declarada — em particular, que nenhuma abstração, refatoração ou otimização seja introduzida durante a v1. Revisões DEVEM igualmente verificar o cumprimento do Princípio II (Ciclo de Porte Verificável) em todo port de script GDScript. Trabalho que viole a fase vigente deve ser rejeitado ou redirecionado ao backlog da fase correta. Revisões de port DEVEM ainda confirmar que a ordem de dependências foi respeitada, que nenhum nome de propriedade exportada ou replicada foi alterado, e que melhorias percebidas foram registradas em `docs/v2-backlog.md`. Revisões DEVEM confirmar que toda correção de bug na v1 atende aos quatro requisitos do Princípio I (spec, isolamento no código, commit, `docs/upstream-bugs.md`) e que nenhuma melhoria foi introduzida sob o rótulo de correção.

**Versão**: 1.3.1 | **Ratificação**: 2026-09-15 | **Última Emenda**: 2026-09-15
