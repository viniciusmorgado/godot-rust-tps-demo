# Contract: public API of `player.rs`/`bullet.rs`/`door.rs` after Milestone V2-C

Mirrors `specs/007-v2-leaves-and-input/contracts/leaves-api.md`'s shape: what exists after the
milestone, and exactly what `level.rs`/`red_robot.rs`/the three `.tscn`s are guaranteed to
still find.

## `Player` (`player.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = CharacterBody3D)]
pub struct Player {
    #[export] #[var(set = set_player_id)] player_id: i32,   // unchanged
    #[export] current_animation: Animations,                 // unchanged
    #[var] motion: Vector2,                                  // unchanged

    // NEW private fields (glue):
    shoot_particle: OnReady<Gd<CpuParticles3D>>,              // was get_node_as per shot
    muzzle_particle: OnReady<Gd<CpuParticles3D>>,              // was get_node_as per shot
    bullet_scene: Gd<PackedScene>,                             // was load() per shot; bare field,
                                                                // #[init(val = load(...))], no OnReady

    // REMOVED: crosshair (backlog #12)
    // airborne_time now #[init(val = 0.0)] (backlog #10)
}

impl Player {
    pub(crate) fn set_player_id(&mut self, value: i32);        // #[func], unchanged signature
}

#[godot_api]
impl Player {
    #[rpc(authority, call_local, unreliable)] fn jump(&mut self);   // unchanged
    #[rpc(authority, call_local, unreliable)] fn land(&mut self);   // unchanged
    #[rpc(authority, call_local, unreliable)] fn shoot(&mut self);  // unchanged
    #[rpc(authority, call_local, unreliable)] fn hit(&mut self);    // unchanged
    #[rpc(authority, call_local, unreliable)]
    pub(crate) fn add_camera_shake_trauma(&mut self, amount: f64);  // unchanged, typed call site
}
```

No `player.tscn` edit required: `SceneReplicationConfig`'s `player_id`/`motion`/
`current_animation` properties reference names, not storage details — `player_id`'s
`#[var(set = set_player_id)]` still routes replication writes through the same setter.

## `PlayerInputSynchronizer` (`player_input.rs`) — one signature-shape change

```rust
impl PlayerInputSynchronizer {
    // #[func] REMOVED from all three — nothing calls them by name (grep-confirmed empty);
    // Player calls them typed via `self.player_input.bind()...`.
    pub(crate) fn get_aim_rotation(&self) -> f64;
    pub(crate) fn get_camera_rotation_basis(&self) -> Basis;
    pub(crate) fn get_camera_base_quaternion(&self) -> Quaternion;
}
```

## `Bullet` (`bullet.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = CharacterBody3D)]
pub struct Bullet {
    state: pure::BulletState,   // was `hit: bool` + `time_alive: f32`
    // animation_player, collision_shape, omni_light, settings: unchanged
}

impl Bullet {
    pub const VELOCITY: f32 = 20.0;   // was module-level BULLET_VELOCITY
}

#[godot_api]
impl Bullet {
    #[rpc(authority, call_local, unreliable)] fn explode(&mut self);  // unchanged
    #[func] fn destroy(&mut self);                                     // unchanged, bullet.tscn:103
}
```

## `hittable.rs` (NEW crate-level module)

```rust
pub enum HitTarget {
    Player(Gd<crate::player::Player>),
    Robot(Gd<crate::red_robot::EnemyRobot>),
}
pub fn resolve(node: Gd<Node3D>) -> Option<HitTarget>;
impl HitTarget {
    pub fn rpc_hit(&mut self);   // .rpc("hit", &[]) — unchanged wire behavior
}
```

Future consumer (V2-D, not edited this milestone): `red_robot.rs::shoot`'s player-hit-detection
block (`red_robot.rs:391-399`) is a candidate to adopt `hittable::resolve`/`rpc_hit` in place of
its own direct `try_cast::<Player>()`, but that edit belongs to V2-D, not this milestone.

## `Door` (`door.rs`)

```rust
#[derive(GodotClass)]
#[class(init, base = Area3D)]
pub struct Door {
    state: pure::DoorState,   // was `open: bool`
    // animation_player: unchanged
}

#[godot_api]
impl Door {
    #[func]
    fn _on_door_body_entered(&mut self, body: Gd<Node3D>);   // UNCHANGED signature (FR-018)
}
```

No `door.tscn` edit: `[connection] body_entered → _on_door_body_entered` (`door.tscn:35`) keeps
its untyped-by-connection shape; Godot resolves the handler's declared parameter type
(`Gd<Node3D>`, unchanged) at connect time.

## Consumers (unchanged, verified read-only this milestone)

```rust
// level.rs:203-206
let mut player: Gd<Player> = load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>();
player.bind_mut().set_player_id(id);   // id: i32 — unchanged

// red_robot.rs (multiple sites)
body.try_cast::<Player>()                                    // unchanged
player.clone().bind_mut().add_camera_shake_trauma(13.0);     // unchanged
```
