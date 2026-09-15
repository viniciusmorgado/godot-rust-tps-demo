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
