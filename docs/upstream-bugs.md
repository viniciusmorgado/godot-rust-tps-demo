# Bugs do upstream corrigidos na v1

Registro exigido pela constituição v1.3.0 (Princípio I, cláusula de correção conservadora de bugs
do upstream). Cada entrada é uma correção **mínima** de um defeito objetivo do Godot TPS Demo
original — comportamento que o próprio código claramente pretendia e não entregava — declarada
na spec da feature, isolada no código por um comentário `// upstream bug fix: ...` no ponto exato
e mencionada na mensagem do commit. Serve de referência para a v2 e para eventual contribuição ao
upstream. Melhorias não entram aqui: vão para `docs/v2-backlog.md`.

| # | Defeito | Script / cena | Correção aplicada | Commit |
|---|---|---|---|---|
| 1 | `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` ao instanciar a porta — `door.gd:6` referenciava um node inexistente; a referência ficava nula e a porta nunca abria ao entrar um `Player` | `door/door.gd:6` (`$DoorModel/AnimationPlayer`) vs `door/door.tscn:13` (o node chama-se `DoorModel2`); declarado em `specs/002-v1-player-bullet-door` US3 / FR-030–FR-035 | `oxide_godot_core/oxide_godot_lib/src/door.rs`: `#[init(node = "DoorModel2/AnimationPlayer")]` com o comentário `// upstream bug fix` na linha acima. Nada renomeado na cena; lógica de `open` intacta; `door.tscn` só recebeu a troca de `type` e a remoção do script. Headless: `Node not found` 1 → 0 | `Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"` |
