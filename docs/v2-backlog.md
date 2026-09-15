# Backlog v2

Melhorias percebidas durante a v1 (raw port) e deliberadamente NÃO aplicadas, conforme o
Princípio I da constituição. Uma entrada por melhoria: origem (script/cena) e motivação.
Serão avaliadas ao abrir a branch `v2`.

| # | Origem | Melhoria | Motivação |
|---|---|---|---|
| 1 | `docs/port-order.md` (análise inicial); consumidores: forklift, bullet, level, menu, main | Acesso tipado ao autoload `Settings` (`Gd<Settings>`) em vez de `get_node("/root/Settings").get("config_file")` e constantes inteiras locais espelhando os enums | Na v1 o `Settings` é portado por último, então os consumidores em Rust nascem com acesso dinâmico |
| 2 | `player/bullet/bullet.gd` | Substituir `has_method("hit")` + `rpc("hit")` dinâmico por um trait/enum de alvo atingível (`Hittable`) | Duck typing herdado do GDScript; em Rust vira dispatch sem tipo |
| 3 | `main/main.gd` | Substituir `has_signal("quit")`/`has_signal("replace_main_scene")` por um gestor centralizado de cenas orientado a signals | Objetivo declarado da v2 na constituição; hoje é duck typing sobre o node raiz da cena carregada |
| 4 | `level/debug.gd` (port 1) | Não recalcular o texto enquanto o overlay está oculto | Trabalho por frame desnecessário; o original (e a v1, por fidelidade) reconstrói o texto a cada frame mesmo com `visible = false` |
| 5 | `enemies/red_robot/parts/part_disappear_effect/part_disappear.gd` e `enemies/red_robot/laser/impact_effect/blast.gd` (ports 2–3) | Timers/sinais como `async` (`godot::task`) quando a API estabilizar | O encadeamento de closures `connect_other` reproduz o `await` do GDScript de forma menos legível |
| 6 | `player/camera_noise_shake_effect.gd` (port 4) | Recapturar `start_rotation` quando animações/scripts movem a câmera | O comentário do original admite o problema; o tremor soma à rotação capturada uma única vez no `_ready` |
| 7 | `player/player_input.gd` (port 5) | Excluir de fato o corpo do jogador no raycast (`exclude` com o RID do `CharacterBody3D` pai) | O original passa `Array([self], TYPE_RID, "", null)` = `[RID(0)]` (o sincronizador não é corpo físico); exclusão inefetiva, preservada na v1 |
| 8 | `player/player_input.gd` (port 5) | `OnEditor<Gd<T>>` em vez de `Option<Gd<T>>` para as 6 referências obrigatórias (`camera_animation`, `crosshair`, `camera_base`, `camera_rot`, `camera_camera`, `color_rect`) | Elimina `unwrap()` por frame e faz o editor sinalizar referência ausente |
| 9 | `player/player_input.gd` (port 5) | Replicar `jumping` ou remover o `@export` | Exportado mas fora da `SceneReplicationConfig`; só faz sentido via RPC `jump` |
| 10 | `player/player.gd` (port 1) | Iniciar `airborne_time` em 0 (ou ignorar o primeiro pouso) | Com 100 inicial, o primeiro contato com o chão dispara `land` e o som de pouso ao spawnar; na v1 é mantido por fidelidade |
| 11 | `player/player.gd` (port 1) | Zerar `velocity` no respawn abaixo de −40 | O teleporte para a posição inicial preserva a velocidade de queda acumulada; na v1 é mantido por fidelidade |
| 12 | `player/player.gd` (port 1) | Remover a referência `crosshair` nunca lida (ou usá-la) | Declarada em `player.gd:30` e não lida em lugar nenhum; na v1 é mantida por fidelidade |
