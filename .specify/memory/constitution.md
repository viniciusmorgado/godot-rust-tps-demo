<!--
SYNC IMPACT REPORT
Version change: 1.1.0 → 1.2.0 (MINOR — expansão do Princípio II e novo bullet no Princípio I)

Princípios:
  Expandido: II. Ciclo de Porte Verificável → adicionadas as subseções "Ordem de porte de baixo
  para cima" e "Preservação de nomes de propriedades" (inseridas entre "Vínculo por tipo, nunca
  por script" e "Build obrigatório")
  Expandido: I. Porte em Três Fases (v1 → v2 → v3) → novo bullet em "Regras de governança das
  fases" exigindo registro de melhorias em `docs/v2-backlog.md`
  Demais conteúdo de ambos os princípios mantido intacto por instrução explícita

Seções:
  Modificada: Governança → parágrafo "Revisão de conformidade" recebeu uma frase adicional ao
  final, cobrindo ordem de dependências, nomes de propriedades e registro em
  `docs/v2-backlog.md`. Os demais parágrafos de Governança foram mantidos intactos por instrução
  explícita.

Novos caminhos documentais referenciados pelos princípios (não fazem parte desta constituição e
não foram criados por este comando — ver Scope Guard):
  - `docs/port-order.md` (grafo/ordem de dependências do port)
  - `docs/v2-backlog.md` (registro de melhorias para a v2)

Templates verificados (não modificados — fora do escopo deste comando):
  - .specify/templates/plan-template.md: gate "Constitution Check" segue válido; nenhuma
    alteração necessária.
  - .specify/templates/spec-template.md, tasks-template.md, checklist-template.md: nenhuma
    referência direta à constituição encontrada; nenhuma ação necessária agora.

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

### II. Ciclo de Porte Verificável

Todo script GDScript portado segue um ciclo fixo, e um port só é considerado concluído quando todas as etapas abaixo foram cumpridas e evidenciadas:

**Vínculo por tipo, nunca por script**
- Cada script `.gd` DEVE virar exatamente uma classe Rust registrada via gdext, com a MESMA classe base do script original (ex.: `extends CharacterBody3D` → `#[class(base=CharacterBody3D)]`).
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

**Build obrigatório**
- Toda alteração em código Rust DEVE ser seguida de `cargo build` (perfil debug) com sucesso e sem warnings novos, antes de qualquer validação ou commit. A biblioteca dinâmica em `target/debug/` é o artefato que o Godot carrega; código não compilado não existe para o jogo.

**Validação headless como definition of done**
- Nenhum port é dado como concluído sem validação executada em Godot headless: no mínimo (a) o import do projeto confirmando o carregamento da extensão (`Initialize godot-rust ...`) e (b) a execução da cena afetada sem erros novos em relação à baseline conhecida do demo upstream.
- Erros pré-existentes do demo upstream não contam como regressão, mas DEVEM estar catalogados no guia operacional (`CLAUDE.md`) para serem distinguíveis de erros introduzidos pelo port.
- O commit de cada port DEVE registrar em sua mensagem qual script foi portado e qual(is) cena(s) tiveram o tipo do node trocado.

**Separação entre regra e operação**
- Esta constituição define O QUE é obrigatório. Caminhos de binários, comandos exatos, versões de ferramentas e notas de API pertencem ao `CLAUDE.md` na raiz do repositório e NÃO devem ser incorporados aqui; mudanças nesses detalhes não constituem emenda.

## Governança

Esta constituição tem precedência sobre qualquer outra prática, convenção, documentação ou decisão ad-hoc dentro do projeto. Em caso de conflito entre esta constituição e qualquer outro documento ou hábito de código, esta constituição prevalece.

**Processo de emenda**: qualquer alteração a esta constituição é feita através do comando `/speckit-constitution`, exige justificativa explícita para a mudança e resulta em uma nova versão do documento, acompanhada do respectivo Sync Impact Report.

**Política de versionamento**: esta constituição segue versionamento semântico (MAJOR.MINOR.PATCH):
- MAJOR: remoção ou redefinição incompatível de um princípio ou de uma das fases v1/v2/v3 (ex.: eliminar uma fase, alterar sua ordem, ou remover uma restrição não negociável).
- MINOR: adição de um novo princípio ou seção, ou expansão material das regras de uma fase existente.
- PATCH: esclarecimentos, correções de redação ou ajustes não semânticos.

**Revisão de conformidade**: toda spec, plan e task DEVE declarar explicitamente a fase (v1, v2 ou v3) à qual pertence, conforme o Princípio I. Revisões de planejamento e de código DEVEM verificar que o trabalho respeita as restrições da fase declarada — em particular, que nenhuma abstração, refatoração ou otimização seja introduzida durante a v1. Revisões DEVEM igualmente verificar o cumprimento do Princípio II (Ciclo de Porte Verificável) em todo port de script GDScript. Trabalho que viole a fase vigente deve ser rejeitado ou redirecionado ao backlog da fase correta. Revisões de port DEVEM ainda confirmar que a ordem de dependências foi respeitada, que nenhum nome de propriedade exportada ou replicada foi alterado, e que melhorias percebidas foram registradas em `docs/v2-backlog.md`.

**Versão**: 1.2.0 | **Ratificação**: 2026-09-15 | **Última Emenda**: 2026-09-15
