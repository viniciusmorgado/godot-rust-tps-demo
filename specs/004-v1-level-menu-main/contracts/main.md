# Contrato: `Main` (port 4)

- Classe registrada: `Main` (nome livre; sem colisão — `menu.gd` tinha `var main` minúsculo e já
  está portado). Base: `Node`.
- Node na cena: raiz de `main/main.tscn` (l.5), **node chamado `main`** (minúsculo — nome
  mantido). Cena principal do projeto (`project.godot:15`).

## Métodos (`#[func]` — os três são alvos de chamada/conexão por nome)

| Original | Rust | Quem chama |
|---|---|---|
| `go_to_main_menu()` | `#[func] fn go_to_main_menu(&mut self)` | `ready`; `Callable::from_object_method(self, "go_to_main_menu")` conectado a `quit` do node atual |
| `replace_main_scene(resource: PackedScene)` | `#[func] fn replace_main_scene(&mut self, resource: Gd<PackedScene>)` | `Callable::from_object_method(self, "replace_main_scene")` conectado a `replace_main_scene` do node atual |
| `change_scene_to_packed(resource: PackedScene)` | `#[func] fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>)` | `call_deferred("change_scene_to_packed", [resource])` (por nome, como o original); `go_to_main_menu` |

## Duck typing preservado (o original já era dinâmico)

`node.has_signal("quit")` / `has_signal("replace_main_scene")` + `node.connect(nome, &Callable)`
— o `Main` não tipa `Level`/`Menu`, exatamente como `main.gd:30-33`.

## Chamadas dinâmicas (exceção `Settings`)

`get_node("/root/Settings").get("config_file")` → `get_value("video", "display_mode")`.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'run/main_scene' project.godot                                      # res://main/main.tscn
grep -n 'name="main" type=' main/main.tscn                                  # type="Main", nome "main"
grep -c 'ExtResource("1")' main/main.tscn                                   # 0
grep -n '#\[func\]' -A1 ../oxide_godot_core/oxide_godot_lib/src/main_scene.rs | grep 'fn ' # go_to_main_menu, replace_main_scene, change_scene_to_packed
find . -name '*.gd' -not -path '*/addons/*'                                 # só menu/settings.gd
```
