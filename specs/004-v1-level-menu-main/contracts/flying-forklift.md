# Contract: `FlyingForklift` (port 1)

- Registered class: `FlyingForklift` (free name; no collision). **Base: `CharacterBody3D`** —
  the root node's type (`flying_forklift.tscn:36`), not the script's `extends Node3D` (constitution
  v1.3.1, Principle II, "Binding by type"). Child `Collider` (`CollisionShape3D`, l.47) remains
  valid.
- Node in the scene: root of `level/forklift/flying_forklift.tscn`; instantiated by `level.tscn:8`
  (several times in the level).

## External contract

None: no signals, no `#[func]`, no exported properties. No script references the class
or the script (`grep -rn 'flying_forklift' --include=*.gd oxide-godot` → empty).

## Scene references

`spot_light: OnReady<Gd<SpotLight3D>>` (`SpotLight3D`, l.114). `get_child(0)` =
`FlyingForkliftModel2` (l.39), whose children are the randomly picked models.

## Dynamic calls (`Settings` exception)

`get_node("/root/Settings").get("config_file")` → `get_value("rendering", "shadow_mapping")`.

## Verification before the commit

```bash
cd oxide-godot
grep -n 'name="FlyingForklift" type=' level/forklift/flying_forklift.tscn   # type="FlyingForklift"
grep -c 'ExtResource("3")' level/forklift/flying_forklift.tscn               # 0
grep -n 'name="Collider"\|name="SpotLight3D"\|name="FlyingForkliftModel2"' level/forklift/flying_forklift.tscn   # 3 untouched lines
grep -n 'flying_forklift.tscn' level/level.tscn                              # l.8 — instances remain valid
```
