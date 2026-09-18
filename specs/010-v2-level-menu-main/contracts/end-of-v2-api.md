# Contract: public API of `main_scene.rs`/`level.rs`/`menu.rs`/`flying_forklift.rs` after Milestone V2-E

Mirrors `specs/009-v2-enemy/contracts/enemy-api.md`'s shape: what exists after the milestone,
and exactly what scenes/other modules are guaranteed to still find.

## `Main` (`main_scene.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct Main {
    settings: OnReady<Gd<Settings>>,   // unchanged
}

impl Main {
    // ALL THREE lose #[func] -- nothing calls any of them by name after this milestone.
    fn go_to_main_menu(&mut self);                          // unchanged body
    fn replace_main_scene(&mut self, resource: Gd<PackedScene>);  // Callable::call_deferred now
    fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>);  // try_cast + connect_other
}
```

No `main.tscn` edit required: zero `[connection]`s exist today, none are added.

## `Level` (`level.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = Node3D)]
pub struct Level {
    lightmap_gi: Option<Gd<LightmapGi>>,   // unchanged

    world_environment: OnReady<Gd<WorldEnvironment>>,   // unchanged
    robot_spawn_points: OnReady<Gd<Node3D>>,             // unchanged
    player_spawn_points: OnReady<Gd<Node3D>>,            // unchanged
    spawned_nodes: OnReady<Gd<Node3D>>,                  // unchanged
    settings: OnReady<Gd<Settings>>,                     // unchanged

    // NEW:
    voxel_gi: OnReady<Gd<Node3D>>,           // was get_node_as per setup_* call
    reflection_probes: OnReady<Gd<Node3D>>,  // was get_node_as per setup_* call
    robot_scene: Gd<PackedScene>,            // was load() per spawn_robot call
    player_scene: Gd<PackedScene>,           // was load() per add_player call
}

#[godot_api]
impl Level {
    #[signal] fn quit();   // unchanged
}

impl Level {
    fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>);  // unchanged signature
    fn del_player(&mut self, id: i32);                                     // unchanged signature
}
```

No `level.tscn` edit required: every child node name (`VoxelGI`, `ReflectionProbes`, etc.)
referenced by name only, still exists exactly where it is.

## `Menu` (`menu.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct Menu {
    // every field unchanged: peer, metalfx_supported, ~50 OnReady node fields, settings
}

#[godot_api]
impl Menu {
    #[signal] fn replace_main_scene(scene: Gd<PackedScene>);  // unchanged

    // ALL TEN keep #[func] -- menu.tscn's [connection]s reach them by name.
    #[func] fn _on_loading_done_timer_timeout(&mut self);
    #[func] fn _on_play_pressed(&mut self);
    #[func] fn _on_settings_pressed(&mut self);   // body: table walk, ~40 lines (was ~115)
    #[func] fn _on_quit_pressed(&mut self);
    #[func] fn _on_apply_pressed(&mut self);      // body: table walk, ~40 lines (was ~135)
    #[func] fn _on_cancel_pressed(&mut self);
    #[func] fn _on_play_online_pressed(&mut self);
    #[func] fn _on_host_pressed(&mut self);       // called via Callable::call_deferred now,
                                                    // when headless auto-hosting
    #[func] fn _on_connect_pressed(&mut self);
}
```

No `menu.tscn` edit required: all 10 `[connection]`s reference unchanged names/signatures.

## `FlyingForklift` (`flying_forklift.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = CharacterBody3D)]
pub struct FlyingForklift {
    spot_light: OnReady<Gd<SpotLight3D>>,   // unchanged
    settings: OnReady<Gd<Settings>>,        // unchanged
}
```

No `flying_forklift.tscn` edit required.

## Crate-wide residual (`hittable.rs`, `bullet.rs`, `player.rs`, `red_robot.rs`, `part.rs` — all
untouched by this milestone, listed for SC-005's completeness)

```text
player.rs:    .rpc("jump", &[]) / .rpc("land", &[]) / .rpc("shoot", &[])
bullet.rs:    .rpc("explode", &[])  x2
hittable.rs:  .rpc("hit", &[])      x2
red_robot.rs: .rpc("play_shoot", &[])
part.rs:      .rpc("destroy", &[])
```

The ONLY by-name dynamic access anywhere in the crate after this milestone.
