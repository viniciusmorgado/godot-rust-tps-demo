# Contract: the player entity (V3-B) — extends `specs/011-v3-ecs-core-leaves/contracts/ecs-api.md`

The V3-A contract (bridge rules, push API, system-author rules, tradeoffs header) applies
unchanged; this file adds what the player introduces.

## 1. Root bridge and sub-bridges

- **Root bridge** (`Player`): registers the ONE entity in `ready` with every handle of the three
  nodes (data-model.md `Handles::Player`) and `Initial::Player`; unregisters in `exit_tree`. It
  may read the sub-bridges' fields through `bind()` in `ready` (children are ready first).
- **Sub-bridge** (`PlayerInputSynchronizer`, `CameraNoiseShake`): registers NOTHING; `ready` is
  v2's one-shot engine setup only; resolves the root's `InstanceId` once
  (`OnReady::from_base_fn(|b| b.get_owner().unwrap().instance_id())`); its callbacks push
  events keyed by that id. A sub-bridge never touches the World and holds no `Entity`.
- **Setters that run before `ready`** (`set_player_id`): one-shot engine writes through typed
  child handles; never touch the ECS.

## 2. Projections per peer role

| Node field | Written by | Read by |
|---|---|---|
| `Player.motion`, `Player.current_animation` | fixed `sync_out_player`, `Simulates` only | fixed `sync_in_player` on non-`Simulates` (replay) |
| `Player.player_id` | `set_player_id` (before `ready`), never again | registration → `PeerId` |
| `InputSynchronizer.{aiming, shoot_target, motion, shooting}` | frame `sync_out_input`, `OwnsInput` only | fixed `sync_in_player` on EVERY player (read directly into `InputFrameC` — the projection's consumer; the frame run holds `ReplicatedInput` only on `OwnsInput`, where it writes it) |
| `CameraBase.rotation`, `CameraRot.rotation` | frame `camera_and_ray`, `OwnsInput` only | fixed `sync_in_player` (the three camera reads) on every player |
| `Player.transform`, `PlayerModel.transform` | engine (`move_and_slide`), `sync_out_player` (`set_global_basis`, respawn) | engine replication |

The engine samples these in `SceneMultiplayer::poll()` at the top of `SceneTree::process`, after
the physics steps and before the frame run, once per rendered frame (research R2). A frame-run
write is therefore sampled only in physics-less iterations; nothing written by the frame run may
be a value that a physics-step write would have replaced — the reason the `Simulates` entity's
local RPC effects are applied inline by the fixed `SyncOut` and `jump`/`land`/`shoot` are
`call_remote` (option (b)): no `PlayerFx` ever occurs on `Simulates`; `apply_player_fx` is
remote-only.

## 3. The fixed tick (seven sets)

`SyncIn` (read every value once; clear `JumpQueued`) → `Gameplay` (`tick_decide`: v2 steps 2–4,
branch, plan, walk target, shoot decision → `TickIntents`) → `EngineQueryOrient`
(`orient_and_anim`: slerp/looking_at, tree parameters, root motion read, bullet spawn) →
`GameplayIntegrate` (`tick_integrate`) → `EngineQueryMove` (`move_body`) → `GameplaySettle`
(`tick_settle`) → `SyncOut` (`sync_out_player`: basis, respawn, projection, RPCs, `advance`).
Order within `SyncOut`: acting systems `.before(sync_out_remove)`; `sync_out_player`'s last engine
write is `advance(delta)`; the RPC calls push events only.

## 4. The frame run for the player

`SyncIn` (`sync_in_input`) → `Gameplay` (`input_decide`, `shake_decide`) → `EngineQuery`
(`camera_and_ray` — rotation BEFORE raycast) → `SyncOut` (`sync_out_input`, `sync_out_shake` — samples + offsets + write, no `EngineQuery` member,
`apply_player_fx`, `sync_out_shake`, all `.before(sync_out_remove)`). Mouse events drained by
either schedule run of an iteration sit in `PendingMouseLook` until this frame's `input_decide`.

## 5. `docs/v3-tradeoffs.md` rows this milestone adds

root motion read (`orient_and_anim`); `move_and_slide` + post-move origin (`move_body`);
crosshair raycast after the in-set camera rotation (`camera_and_ray`); noise samples
(`sync_out_shake`, sync-only); `slerp`/`looking_at` engine-backed math (`orient_and_anim`); `AnimationTree` parameter writes in `sync_out_player` before `advance` (not in `EngineQueryOrient`); bullet instancing
from the tick (`orient_and_anim`); the `AnimationTree` MANUAL + `advance` decision
(`sync_out_player`, `player.tscn:592`); replication as projection (§2); RPC timing — `jump`/`land`/`shoot` are `call_remote` (option (b)): local effects inline in the fixed `SyncOut`, handlers only on remote peers
(`apply_player_fx` in the same iteration's frame run).

## 6. Harness contract files

- `zz_ecs_parity.gd` — the six-case harness (research R9) with its observer; scene
  `zz_ecs_parity.tscn` as in V3-A's contract §4; `zz_ecs_observer.gd` as quoted inside.
- `zz_r1_probe.gd` — the `AnimationTree` ordering experiment (two physics probes, `ZZ_MODE`).
- `zz_r3_probe.gd` — the headless input experiment.

All scratch: copied into the game project for a run, deleted before every commit.
