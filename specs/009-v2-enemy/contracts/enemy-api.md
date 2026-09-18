# Contract: public API of `red_robot.rs`/`part.rs` after Milestone V2-D

Mirrors `specs/008-v2-player-bullet-door/contracts/player-bullet-door-api.md`'s shape: what
exists after the milestone, and exactly what `level.rs`/`bullet.rs`/`hittable.rs`/the two
`.tscn`-embedded scenes are guaranteed to still find.

## `EnemyRobot` (`red_robot.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = CharacterBody3D)]
pub struct EnemyRobot {
    #[export] target_position: Vector3,   // unchanged
    #[export] #[init(val = 5)] health: i32,   // unchanged
    #[export] #[init(val = State::Idle)] state: State,   // unchanged wire enum, unchanged codes
    #[export] dead: bool,                 // unchanged

    // REMOVED #[export] (backlog #18) — plain #[var] now:
    #[var] aim_preparing: f32,
    #[var] test_shoot: bool,

    // unchanged, non-replicated, non-exported (v1 shape kept, research.md R2):
    shoot_countdown: f32,
    aim_countdown: f32,

    // CHANGED (backlog #16): Option<Gd<Node3D>> -> Option<Gd<Player>>
    player: Option<Gd<Player>>,
    orientation: Transform3D,              // unchanged

    // NEW private fields (glue):
    impact_effect_scene: OnReady<Gd<PackedScene>>,   // was load() per shot
    is_dedicated_server: bool,                        // was has_feature() per frame in _clip_ray

    // unchanged OnReady node fields: animation_tree, shoot_animation, model, ray_from, ray_mesh,
    // laser_raycast, collision_shape, explosion_sound, hit_sound, death, death_shield1/2,
    // death_head, death_detach_spark1/2, laser_ember (NEW: was get_node_as per shot)
}

#[godot_api]
impl EnemyRobot {
    #[signal] pub(crate) fn exploded();          // unchanged

    #[func] fn resume_approach(&mut self);        // unchanged signature; body now calls the
                                                   // shared pure resume_approach_reset()
    #[func] fn shoot_check(&mut self);            // unchanged
    #[func] fn _on_area_body_entered(&mut self, body: Gd<Node3D>);  // unchanged signature
                                                   // (the try_cast<Player> still happens here,
                                                   // exactly once, at the boundary)
    #[func] fn _on_area_body_exited(&mut self, body: Gd<Node3D>);   // unchanged

    #[rpc(authority, call_local, unreliable)] fn hit(&mut self);         // unchanged signature
    #[rpc(authority, call_local, unreliable)] fn play_shoot(&mut self);  // unchanged
}
```

No `red_robot.tscn` edit required: every `[connection]`, method-track call, and
`SceneReplicationConfig` property references a name, not a storage detail — the two `#[export]`
removals (`aim_preparing`, `test_shoot`) are confirmed (research context) to have no stored
override anywhere in the scene, so nothing there references them by inspector value either.

## `Part` (`part.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = RigidBody3D)]
pub struct Part {
    #[export] #[init(val = 3.0)] lifetime: f32,              // unchanged
    #[export] #[init(val = 3.0)] lifetime_random: f32,        // unchanged
    #[export] #[init(val = 0.5)] disappearing_time: f32,      // unchanged
    #[export] #[var(set = set_fade_value)] fade_value: f32,   // unchanged

    // renamed from _mat / _disappearing_counter (Rust naming, not exported/replicated either way):
    material: Option<Gd<Material>>,
    disappearing_counter: f32,

    // NEW private fields (glue):
    synchronizer: OnReady<Gd<MultiplayerSynchronizer>>,   // was get_node_as per explode() call
    col1: OnReady<Gd<CollisionShape3D>>,                   // was get_node_as per explode() call
    col2: OnReady<Gd<CollisionShape3D>>,                   // was get_node_as per explode() call
    model_mesh: OnReady<Gd<MeshInstance3D>>,               // was get_node_as + get_child(0) in ready()
    part_disappear_scene: OnReady<Gd<PackedScene>>,        // was load() per destroy() call
}

#[godot_api]
impl Part {
    #[func] fn set_fade_value(&mut self, value: f32);   // unchanged signature (replication path)
    #[func] pub(crate) fn explode(&mut self);           // unchanged signature (typed call from
                                                          // red_robot.rs::hit)
    #[rpc(authority, call_local, unreliable)] fn destroy(&mut self);  // unchanged (animation
                                                          // method track calls it by name)
}
```

No standalone `part.tscn` exists (the three `Part` instances live inline inside
`red_robot.tscn`'s `Death` node) — no scene edit required.

## `HitTarget`/`hittable.rs` — untouched

`EnemyRobot` remains a valid `HitTarget::Robot(Gd<EnemyRobot>)` variant, resolved by
`bullet.rs` via `try_cast`, `.rpc("hit", &[])` on it. Nothing in this milestone changes
`hittable.rs`, and `EnemyRobot::hit`'s signature/attributes are the FR-001-guaranteed contract
that keeps this working.

## `level.rs` — untouched call sites

```rust
// unchanged:
let mut robot: Gd<EnemyRobot> = load::<PackedScene>("res://enemies/red_robot/red_robot.tscn")
    .instantiate_as::<EnemyRobot>();
robot.signals().exploded().connect_other(&*self, move |this: &mut Level| { ... });
```
