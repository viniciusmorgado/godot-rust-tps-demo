<!--
SYNC IMPACT REPORT
Version change: (template não preenchido) → 1.0.0
Tipo: ratificação inicial (não é uma emenda a uma versão anterior real)

Princípios:
  Adicionado: I. Porte em Três Fases (v1 → v2 → v3)
  Placeholders removidos (nunca preenchidos; removidos a pedido explícito do usuário em vez de
  mantidos como slots de template vazios):
    - [PRINCIPLE_2_NAME] / [PRINCIPLE_2_DESCRIPTION]
    - [PRINCIPLE_3_NAME] / [PRINCIPLE_3_DESCRIPTION]
    - [PRINCIPLE_4_NAME] / [PRINCIPLE_4_DESCRIPTION]
    - [PRINCIPLE_5_NAME] / [PRINCIPLE_5_DESCRIPTION]

Seções:
  Adicionada: Governança (processo de emenda, política de versionamento, revisão de conformidade)
  Removidas (nunca preenchidas; mesmo tratamento acima):
    - [SECTION_2_NAME] / [SECTION_2_CONTENT] (ex.: Additional Constraints)
    - [SECTION_3_NAME] / [SECTION_3_CONTENT] (ex.: Development Workflow)

Templates verificados (não modificados — fora do escopo deste comando):
  - .specify/templates/plan-template.md: possui gate "Constitution Check"; nenhuma alteração
    necessária, mas planos futuros devem declarar a fase (v1/v2/v3) para satisfazer esse gate.
  - .specify/templates/spec-template.md, tasks-template.md, checklist-template.md: nenhuma
    referência direta à constituição encontrada; nenhuma ação necessária agora.

Itens adiados (TODO): nenhum. Todos os placeholders não preenchidos foram removidos, não adiados.
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

## Governança

Esta constituição tem precedência sobre qualquer outra prática, convenção, documentação ou decisão ad-hoc dentro do projeto. Em caso de conflito entre esta constituição e qualquer outro documento ou hábito de código, esta constituição prevalece.

**Processo de emenda**: qualquer alteração a esta constituição é feita através do comando `/speckit-constitution`, exige justificativa explícita para a mudança e resulta em uma nova versão do documento, acompanhada do respectivo Sync Impact Report.

**Política de versionamento**: esta constituição segue versionamento semântico (MAJOR.MINOR.PATCH):
- MAJOR: remoção ou redefinição incompatível de um princípio ou de uma das fases v1/v2/v3 (ex.: eliminar uma fase, alterar sua ordem, ou remover uma restrição não negociável).
- MINOR: adição de um novo princípio ou seção, ou expansão material das regras de uma fase existente.
- PATCH: esclarecimentos, correções de redação ou ajustes não semânticos.

**Revisão de conformidade**: toda spec, plan e task DEVE declarar explicitamente a fase (v1, v2 ou v3) à qual pertence, conforme o Princípio I. Revisões de planejamento e de código DEVEM verificar que o trabalho respeita as restrições da fase declarada — em particular, que nenhuma abstração, refatoração ou otimização seja introduzida durante a v1. Trabalho que viole a fase vigente deve ser rejeitado ou redirecionado ao backlog da fase correta.

**Versão**: 1.0.0 | **Ratificação**: 2026-09-15 | **Última Emenda**: 2026-09-15
