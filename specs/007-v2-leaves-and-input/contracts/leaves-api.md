# Contract: public API of the 5 leaves after Milestone V2-B

Mirrors `specs/006-v2-typed-settings/contracts/settings-api.md`'s shape: what exists after the
milestone, and exactly what `player.rs` (unchanged, V2-C) is guaranteed to still find.

## `PlayerInputSynchronizer` (`player_input.rs`)

```rust
#[derive(GodotClass)]
#[class(base = CharacterBody3D-sibling-base-per-scene)]  // unchanged base, see player.tscn
pub struct PlayerInputSynchronizer {
    #[export] motion: Vector2,                      // replicated, unchanged name/type
    #[export] aiming: bool,                          // replicated, unchanged name/type — AimState projection
    #[export] shooting: bool,                         // replicated, unchanged name/type
    #[export] jumping: bool,                          // unchanged name/type (US1 decides export vs replicate — recorded in spec)
    #[export] shoot_target: Vector3,                  // replicated, unchanged name/type

    #[export] camera_animation: OnEditor<Gd<AnimationPlayer>>,  // was Option<Gd<...>>
    #[export] crosshair: OnEditor<Gd<Control>>,
    #[export] camera_base: OnEditor<Gd<Node3D>>,
    #[export] camera_rot: OnEditor<Gd<Node3D>>,
    #[export] camera_camera: OnEditor<Gd<Camera3D>>,             // player.rs:138 call site changes
    #[export] color_rect: OnEditor<Gd<ColorRect>>,

    aim_state: AimState,                              // NEW, private, not exported
    parent: OnReady<Gd<CharacterBody3D>>,              // NEW, private — resolved once
    parent_rid: OnReady<Rid>,                          // NEW, private — resolved once
    // fall-fade `prev_alpha` and any other frame-carried state: private fields, not exported
}

impl PlayerInputSynchronizer {
    // unchanged signatures, still #[func] pub(crate), still called by player.rs:
    pub(crate) fn get_aim_rotation(&self) -> f64;
    pub(crate) fn get_camera_rotation_basis(&self) -> Basis;   // exact return type per current code
    pub(crate) fn get_camera_base_quaternion(&self) -> Quaternion;
}

#[godot_api]
impl PlayerInputSynchronizer {
    #[rpc(authority, call_local, unreliable)]
    fn jump(&mut self);   // unchanged — the milestone's one permanent residual (.rpc("jump"))
}
```

Removed from the type (compared to `v1`): the standalone `toggled_aim: bool`, `aiming_timer:
f32` fields (folded into `aim_state: AimState`) — neither was exported/replicated, so this is
invisible to `player.tscn` and to `player.rs`.

## `CameraNoiseShake` (`camera_noise_shake.rs`)

```rust
#[derive(GodotClass)]
#[class(base = Camera3D)]
pub struct CameraNoiseShake {
    start_rotation: Vector3,      // unchanged, private
    trauma: f32,                  // unchanged, private
    time: f32,                    // unchanged, private (noise sample position accumulator)
    noise_seed: i32,              // unchanged, #[init(val = randi() as i32)]
    noise_yaw: Gd<FastNoiseLite>,   // NEW split from the single `noise` field — seeded once at ready with noise_seed
    noise_pitch: Gd<FastNoiseLite>, // seeded once at ready with noise_seed.wrapping_add(1)
    noise_roll: Gd<FastNoiseLite>,  // seeded once at ready with noise_seed.wrapping_add(2)
    tuning: CameraShakeTuning,      // NEW, Default
}

impl CameraNoiseShake {
    pub(crate) fn add_trauma(&mut self, amount: f64);  // no #[func] (FR-013) — Player calls it typed via player.rs
}
```

`player.rs`'s only touch point, `add_camera_shake_trauma` (on `Player`, `#[rpc]`, unchanged),
casts `camera_camera` to `CameraNoiseShake` and calls `.bind_mut().add_trauma(amount)` exactly as
today — this call is typed, not by-name, and is unaffected by `add_trauma` losing `#[func]`.

## `DebugLabel` (`debug_label.rs`)

```rust
#[derive(GodotClass)]
#[class(base = Label)]  // unchanged base
pub struct DebugLabel {
    multiplayer_id: /* unchanged private tracking, per current code */,
}
```

No `player.rs` dependency (confirmed: `debug_label.rs` is never referenced from `player.rs`).
Public surface unchanged except the new VRAM line in the composed text (backlog #26) and the
hidden-frame skip (backlog #4) — both are behavior, not API, changes.

## `PartDisappear` (`part_disappear.rs`), `Blast` (`blast.rs`)

No field/method signature changes — both are pure internal rewrites (nested `connect_other` →
one `async fn` spawned via `godot::task::spawn`). No `player.rs` dependency (confirmed: neither
is referenced from `player.rs`; both are triggered from `level.rs`/`red_robot.rs`/the laser
impact scene, all out of scope here).

## `player.rs` (UNCHANGED except one line — V2-C is out of scope)

```rust
// player.rs:138, BEFORE:
let camera = self.player_input.bind().camera_camera.clone().unwrap();
// player.rs:138, AFTER:
let camera = self.player_input.bind().camera_camera.clone();
```

No other line of `player.rs` changes. `player.rs`'s own `#[rpc] fn shoot`/`fn hit`/
`fn add_camera_shake_trauma` are read-only evidence for this milestone (research.md R9's harness
trigger path), not edited.
