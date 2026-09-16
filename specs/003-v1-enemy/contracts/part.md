# Contrato: `Part` (port 1)

Superfície consumida por código que **permanece em GDScript** (`red_robot.gd`, até o port 2) e
pela cena `red_robot.tscn`. Nomes são contrato: copiar do original, nunca traduzir.

- Classe registrada: `Part` (nome livre — sem `class_name`; nenhum script referencia o tipo).
  Conferido: não existe classe `Part` no engine (`out/classes/`).
- Base: `RigidBody3D`.
- Nodes na cena: `enemies/red_robot/red_robot.tscn` → `Death/PartShield1` (l.10833),
  `Death/PartShield2` (l.10885), `Death/PartHead` (l.10936); script `ext_resource id="24"` (l.26).
  **Não há cena própria** — o port é a troca de `type` nesses 3 nodes.

## Métodos e RPCs

| Assinatura Godot | Rust | Consumidor | Efeito |
|---|---|---|---|
| `explode() -> void` | `#[func] pub(crate) fn explode(&mut self)` | `red_robot.gd:96-98` (`death_shield1.explode()` etc., por nome); `RedRobot` tipado no port 2 | sync pública, `freeze = false`; servidor: colisões, velocidades, temporizador do fade |
| `destroy() -> void` (`@rpc("call_local")`) | `#[rpc(authority, call_local, unreliable)] fn destroy(&mut self)` | interno (`rpc("destroy")`) | puff no pai da peça; `queue_free` após 0,2 s |
| `set_fade_value(value: float)` | `#[func] fn set_fade_value(&mut self, value: f32)` | ninguém (setter registrado de `fade_value`) | exposto por exigência do gdext |

## Propriedades exportadas

| Nome | Tipo Godot | Rust | Default | Consumidor |
|---|---|---|---|---|
| `lifetime` | `float` | `#[export] lifetime: f32` | 3,0 | inspector |
| `lifetime_random` | `float` | `#[export] lifetime_random: f32` | 3,0 | inspector |
| `disappearing_time` | `float` | `#[export] disappearing_time: f32` | 0,5 | inspector |
| `fade_value` | `float` | `#[export] #[var(set = set_fade_value)] fade_value: f32` | 0,0 | `red_robot.tscn:10419` (`properties/0/path = NodePath(".:fade_value")`, replicado por frame) |

Nenhum dos 4 valores está gravado nos 3 nodes da cena (conferido: os blocos dos nodes só têm
propriedades da base). Replicação restante (`.:position`, `.:rotation`, `.:linear_velocity`,
`.:angular_velocity`, l.10422-10431) é API base de `RigidBody3D`/`Node3D`.

## Virtuais implementados

`_ready`, `_process(delta)` — via `impl IRigidBody3D`.

## O que a peça consome

`part_disappear.tscn` → `PartDisappear` (Marco A) só por API base (`CpuParticles3D`:
`set_global_position`). Nada de GDScript.

## Verificação antes do commit

```bash
cd oxide-godot
grep -n 'explode()' enemies/red_robot/red_robot.gd                          # l.96, 97, 98 — nome idêntico ao #[func]
grep -n 'properties/0/path = NodePath(".:fade_value")' enemies/red_robot/red_robot.tscn   # l.10419 (−1 após remover o ext_resource)
grep -c 'type="Part"' enemies/red_robot/red_robot.tscn                       # 3
grep -c 'ExtResource("24")' enemies/red_robot/red_robot.tscn                 # 0
grep -n 'public_visibility = false' enemies/red_robot/red_robot.tscn | wc -l # 3 (intocados)
```
