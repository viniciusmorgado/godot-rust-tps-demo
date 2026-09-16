# Contract: `Door` (port 3 — with conservative upstream bug fix)

- Registered class: `Door` (free name — no `class_name`; no script references the type).
  Verified: no `Door` class exists in the engine (`out/classes/`).
- Base: `Area3D`.
- Node in the scene: root of `door/door.tscn` (l.10). **Orphan asset**: no scene instantiates
  `door.tscn` (`grep -rn 'door.tscn' oxide-godot --include=*.tscn --include=*.gd` → empty).

## Methods (`#[func]`)

| Godot signature | Rust | Consumer | Effect |
|---|---|---|---|
| `_on_door_body_entered(body: Node3D) -> void` | `#[func] fn _on_door_body_entered(&mut self, body: Gd<Node3D>)` | `door.tscn:37` — `[connection signal="body_entered" from="." to="." method="_on_door_body_entered"]` | `!open && body is Player` → `AnimationPlayer.play("doorsimple_opening")`, `open = true` |

## Properties

None registered. `open` is internal.

## Corrected reference

| | Original (`door.gd:6`) | Port (`src/door.rs`) |
|---|---|---|
| Path | `$DoorModel/AnimationPlayer` — **non-existent node** | `DoorModel2/AnimationPlayer` |
| Actual node in the scene | `door.tscn:13` `[node name="DoorModel2" ...]`; `door.tscn:25` `[node name="AnimationPlayer" parent="DoorModel2" ...]` (autoplay `doorsimple_closed`) | same, **without renaming** |
| Animation | `doorsimple_opening` exists in `door/model/door.dae` (1 occurrence) | same |
| Effect | `ERROR: Node not found: "DoorModel/AnimationPlayer" (relative to "/root/Door")` on instantiation; door never opens | no error; opens once for a `Player` |

Clause requirements (constitution v1.3.0, Principle I): (a) declared in `spec.md` US3 /
FR-030–FR-035 ✓; (b) comment `// upstream bug fix: ...` on the line immediately above the
`#[init(node = "DoorModel2/AnimationPlayer")]`; (c) the port 3 commit message mentions the
fix; (d) `docs/upstream-bugs.md` created in the same commit with the entry.

## Implemented virtuals

None (`impl IArea3D for Door {}` empty).

## Verification before the commit

```bash
cd oxide-godot
grep -n 'method="_on_door_body_entered"' door/door.tscn                 # l.37 — identical name
grep -n 'name="DoorModel2"\|name="AnimationPlayer" parent="DoorModel2"' door/door.tscn   # l.13, l.25 — untouched
grep -c 'doorsimple_opening' door/model/door.dae                        # 1
grep -n 'upstream bug fix' ../oxide_godot_core/oxide_godot_lib/src/door.rs   # 1 line, above the #[init(node = ...)]
```
