# Contract: `Main` (port 4)

- Registered class: `Main` (free name; no collision — `menu.gd` had a lowercase `var main` and is
  already ported). Base: `Node`.
- Node in the scene: root of `main/main.tscn` (l.5), **node named `main`** (lowercase — name
  kept). The project's main scene (`project.godot:15`).

## Methods (`#[func]` — all three are targets of call/connection by name)

| Original | Rust | Who calls it |
|---|---|---|
| `go_to_main_menu()` | `#[func] fn go_to_main_menu(&mut self)` | `ready`; `Callable::from_object_method(self, "go_to_main_menu")` connected to the current node's `quit` |
| `replace_main_scene(resource: PackedScene)` | `#[func] fn replace_main_scene(&mut self, resource: Gd<PackedScene>)` | `Callable::from_object_method(self, "replace_main_scene")` connected to the current node's `replace_main_scene` |
| `change_scene_to_packed(resource: PackedScene)` | `#[func] fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>)` | `call_deferred("change_scene_to_packed", [resource])` (by name, like the original); `go_to_main_menu` |

## Duck typing preserved (the original was already dynamic)

`node.has_signal("quit")` / `has_signal("replace_main_scene")` + `node.connect(name, &Callable)`
— `Main` does not type `Level`/`Menu`, exactly like `main.gd:30-33`.

## Dynamic calls (`Settings` exception)

`get_node("/root/Settings").get("config_file")` → `get_value("video", "display_mode")`.

## Verification before the commit

```bash
cd oxide-godot
grep -n 'run/main_scene' project.godot                                      # res://main/main.tscn
grep -n 'name="main" type=' main/main.tscn                                  # type="Main", name "main"
grep -c 'ExtResource("1")' main/main.tscn                                   # 0
grep -n '#\[func\]' -A1 ../oxide_godot_core/oxide_godot_lib/src/main_scene.rs | grep 'fn ' # go_to_main_menu, replace_main_scene, change_scene_to_packed
find . -name '*.gd' -not -path '*/addons/*'                                 # only menu/settings.gd
```
