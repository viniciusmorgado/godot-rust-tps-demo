# Contrato: `FlyingForklift` (port 1)

- Classe registrada: `FlyingForklift` (nome livre; sem colisão). **Base: `CharacterBody3D`** —
  tipo do node raiz (`flying_forklift.tscn:36`), não o `extends Node3D` do script (constituição
  v1.3.1, Princípio II, "Vínculo por tipo"). Filho `Collider` (`CollisionShape3D`, l.47) continua
  válido.
- Node na cena: raiz de `level/forklift/flying_forklift.tscn`; instanciada por `level.tscn:8`
  (várias vezes no level).

## Contrato externo

Nenhum: sem sinais, sem `#[func]`, sem propriedades exportadas. Nenhum script referencia a classe
ou o script (`grep -rn 'flying_forklift' --include=*.gd oxide-godot` → vazio).

## Referências de cena

`spot_light: OnReady<Gd<SpotLight3D>>` (`SpotLight3D`, l.114). `get_child(0)` =
`FlyingForkliftModel2` (l.39), cujos filhos são os modelos sorteados.

## Chamadas dinâmicas (exceção `Settings`)

`get_node("/root/Settings").get("config_file")` → `get_value("rendering", "shadow_mapping")`.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'name="FlyingForklift" type=' level/forklift/flying_forklift.tscn   # type="FlyingForklift"
grep -c 'ExtResource("3")' level/forklift/flying_forklift.tscn               # 0
grep -n 'name="Collider"\|name="SpotLight3D"\|name="FlyingForkliftModel2"' level/forklift/flying_forklift.tscn   # 3 linhas intocadas
grep -n 'flying_forklift.tscn' level/level.tscn                              # l.8 — instâncias continuam válidas
```
