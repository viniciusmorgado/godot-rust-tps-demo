# Contract: `Part` (port 1)

Surface consumed by code that **remains in GDScript** (`red_robot.gd`, until port 2) and
by the `red_robot.tscn` scene. Names are contract: copy from the original, never translate.

- Registered class: `Part` (free name — no `class_name`; no script references the type).
  Checked: there is no `Part` class in the engine (`out/classes/`).
- Base: `RigidBody3D`.
- Nodes in the scene: `enemies/red_robot/red_robot.tscn` → `Death/PartShield1` (l.10833),
  `Death/PartShield2` (l.10885), `Death/PartHead` (l.10936); script `ext_resource id="24"` (l.26).
  **There is no scene of its own** — the port is the `type` swap on those 3 nodes.

## Methods and RPCs

| Godot signature | Rust | Consumer | Effect |
|---|---|---|---|
| `explode() -> void` | `#[func] pub(crate) fn explode(&mut self)` | `red_robot.gd:96-98` (`death_shield1.explode()` etc., by name); typed `EnemyRobot` in port 2 | public sync, `freeze = false`; server: collisions, velocities, fade timer |
| `destroy() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn destroy(&mut self)` | internal (`rpc("destroy")`) | puff on the part's parent; `queue_free` after 0.2 s |
| `set_fade_value(value: float)` | `#[func] fn set_fade_value(&mut self, value: f32)` | nobody (registered setter of `fade_value`) | exposed by gdext requirement |

## Exported properties

| Name | Godot type | Rust | Default | Consumer |
|---|---|---|---|---|
| `lifetime` | `float` | `#[export] lifetime: f32` | 3.0 | inspector |
| `lifetime_random` | `float` | `#[export] lifetime_random: f32` | 3.0 | inspector |
| `disappearing_time` | `float` | `#[export] disappearing_time: f32` | 0.5 | inspector |
| `fade_value` | `float` | `#[export] #[var(set = set_fade_value)] fade_value: f32` | 0.0 | `red_robot.tscn:10419` (`properties/0/path = NodePath(".:fade_value")`, replicated per frame) |

None of the 4 values is written on the 3 nodes of the scene (checked: the node blocks only have
base properties). The remaining replication (`.:position`, `.:rotation`, `.:linear_velocity`,
`.:angular_velocity`, l.10422-10431) is base API of `RigidBody3D`/`Node3D`.

## Implemented virtuals

`_ready`, `_process(delta)` — via `impl IRigidBody3D`.

## What the part consumes

`part_disappear.tscn` → `PartDisappear` (Milestone A) only through base API (`CpuParticles3D`:
`set_global_position`). No GDScript.

## Verification before the commit

```bash
cd oxide-godot
grep -n 'explode()' enemies/red_robot/red_robot.gd                          # l.96, 97, 98 — name identical to the #[func]
grep -n 'properties/0/path = NodePath(".:fade_value")' enemies/red_robot/red_robot.tscn   # l.10419 (−1 after removing the ext_resource)
grep -c 'type="Part"' enemies/red_robot/red_robot.tscn                       # 3
grep -c 'ExtResource("24")' enemies/red_robot/red_robot.tscn                 # 0
grep -n 'public_visibility = false' enemies/red_robot/red_robot.tscn | wc -l # 3 (untouched)
```
