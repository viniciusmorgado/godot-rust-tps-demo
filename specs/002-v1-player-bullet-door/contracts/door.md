# Contrato: `Door` (port 3 — com correção conservadora do bug do upstream)

- Classe registrada: `Door` (nome livre — sem `class_name`; nenhum script referencia o tipo).
  Conferido: não existe classe `Door` no engine (`out/classes/`).
- Base: `Area3D`.
- Node na cena: raiz de `door/door.tscn` (l.10). **Asset órfão**: nenhuma cena instancia
  `door.tscn` (`grep -rn 'door.tscn' oxide-godot --include=*.tscn --include=*.gd` → vazio).

## Métodos (`#[func]`)

| Assinatura Godot | Rust | Consumidor | Efeito |
|---|---|---|---|
| `_on_door_body_entered(body: Node3D) -> void` | `#[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>)` | `door.tscn:37` — `[connection signal="body_entered" from="." to="." method="_on_door_body_entered"]` | `!open && body is Player` → `AnimationPlayer.play("doorsimple_opening")`, `open = true` |

## Propriedades

Nenhuma registrada. `open` é interno.

## Referência corrigida

| | Original (`door.gd:6`) | Port (`src/door.rs`) |
|---|---|---|
| Caminho | `$DoorModel/AnimationPlayer` — **node inexistente** | `DoorModel2/AnimationPlayer` |
| Node real na cena | `door.tscn:13` `[node name="DoorModel2" ...]`; `door.tscn:25` `[node name="AnimationPlayer" parent="DoorModel2" ...]` (autoplay `doorsimple_closed`) | idem, **sem renomear** |
| Animação | `doorsimple_opening` existe em `door/model/door.dae` (1 ocorrência) | idem |
| Efeito | `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` na instanciação; porta nunca abre | sem erro; abre uma vez para um `Player` |

Requisitos da cláusula (constituição v1.3.0, Princípio I): (a) declarado em `spec.md` US3 /
FR-030–FR-035 ✓; (b) comentário `// upstream bug fix: ...` na linha imediatamente acima do
`#[init(node = "DoorModel2/AnimationPlayer")]`; (c) mensagem do commit do port 3 menciona a
correção; (d) `docs/upstream-bugs.md` criado no mesmo commit com a entrada.

## Virtuais implementados

Nenhum (`impl IArea3D for Door {}` vazio).

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'method="_on_door_body_entered"' door/door.tscn                 # l.37 — nome idêntico
grep -n 'name="DoorModel2"\|name="AnimationPlayer" parent="DoorModel2"' door/door.tscn   # l.13, l.25 — intocados
grep -c 'doorsimple_opening' door/model/door.dae                        # 1
grep -n 'upstream bug fix' ../oxide_godot_core/oxide_godot_lib/src/door.rs   # 1 linha, acima do #[init(node = ...)]
```
