# Contrato: `CameraNoiseShake` (port 4)

Superfície pública consumida por código que **permanece em GDScript**.

- Classe registrada: `CameraNoiseShake` (nome livre — o original não tem `class_name`; nenhum
  script referencia o tipo)
- Base: `Camera3D`
- Node na cena: `player.tscn` → `CameraBase/CameraRot/SpringArm3D/Camera3D` (l.630). É o node
  apontado por `camera_camera` do `InputSynchronizer` (`player.tscn:350`), e por isso é alcançado
  pelo GDScript como `player_input.camera_camera`.

## Métodos (`#[func]`)

| Assinatura Godot | Rust | Consumidor | Valores usados |
|---|---|---|---|
| `add_trauma(amount: float) -> void` | `fn add_trauma(&mut self, amount: f64)` | `player.gd:211` (`player_input.camera_camera.add_trauma(amount)`) | 0.35 (`player.gd:201`, atirar), 0.75 (`:206`, atingido), 13.0 (`red_robot.gd:133` → `player.gd:210`) |

## Propriedades expostas

Nenhuma. `trauma`, `time`, `start_rotation`, `noise`, `noise_seed` são internos; nenhum script os
lê (conferido: `grep -rn 'trauma\|noise_seed\|start_rotation' oxide-godot --include=*.gd` só
encontra o próprio `camera_noise_shake_effect.gd` e `add_camera_shake_trauma`/`add_trauma`).

## Virtuais implementados

`_ready`, `_process(delta)` — via `impl ICamera3D`.

## Nota de tipagem estática

`player_input.camera_camera` é tipado `Camera3D` para o analisador do GDScript, que não conhece
`add_trauma` nesse tipo → warning `UNSAFE_METHOD_ACCESS`, exatamente como hoje (o script original
também não tem `class_name`). Não é regressão.

## Verificação antes do commit

```bash
cd oxide-godot
grep -rn 'add_trauma\|add_camera_shake_trauma' --include=*.gd .
```
Deve retornar apenas `player.gd:201,206,210,211` e `red_robot.gd:133`.
