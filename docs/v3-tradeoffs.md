# v3 trade-offs — engine touch points

Where the ECS abstraction touches the engine (constitution 1.5.1, Principle I v3 and Principle
III "ECS shape"). One row per touch point, added in the commit that introduces it.

| Module | What the engine owns | Why the ECS cannot hide it | How the sync layer handles it | Location |
|---|---|---|---|---|
| `part`, `level`, `red_robot` | Scene instancing: `PackedScene::instantiate` + `add_child` (and the `ready` that follows) | The node must exist before it can be a view of an entity; instancing stays in the v2 node code of the spawner (Principle I v3 scope: `level` is excluded from the ECS, `part`/`red_robot` are later milestones) | The spawned node's bridge pushes `InboundEvent::Register` from its own `ready`; the drain of the next schedule run spawns the entity (`ecs::apply_register`), so the spawner never touches the World | `part.rs:116-117` (preload) / `:196-205` (instantiate in `destroy`), `red_robot.rs:423-425`, `level.rs`'s prefab spawner; `ecs.rs::apply_register` |
| `part_disappear`, `blast` | Frame time (`_process` delta) and the `SceneTreeTimer`s v2 awaited | The ECS cannot hide time itself: a tick timer can only be as exact as the delta the engine hands the driver | `EcsWorld::process` writes `FrameDelta` before the frame schedule; the `Timer` component reproduces `SceneTree::process_timers`' subtractive arithmetic (`time_left -= dt`, fires when `<= 0`, research.md R5) so it expires on the same step as the `SceneTreeTimer` it replaces; `DisappearPhase::advance` steps it in `Phase::Gameplay`, `sync_out_puff`/`sync_out_remove` apply the outcome | `ecs/timer.rs`, `part_disappear/system.rs`, `ecs.rs::sync_out_puff` |
