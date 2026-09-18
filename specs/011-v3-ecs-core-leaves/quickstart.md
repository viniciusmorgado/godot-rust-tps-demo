# Quickstart: Milestone V3-A — validating the ECS core and the three leaf effects

Prerequisites: `CLAUDE.md` toolchain (Godot 4.7.2 at `/usr/bin/godot.x86_64`, stable Rust
1.98), branch `v3` at or after `08bc3bd` (the milestone baseline), a `v2` worktree for the parity runs
(research.md R10). Paths below are from the repository root.

## 1. Gates (before every commit)

```sh
cd oxide_godot_core && cargo build && cargo clippy && cargo test
```

Expected after commit 1: build/clippy clean, 150 tests (133 + queue 2, timer 4, index 5, apply 3,
setup 3); after commit 3: 152 (the three v2 door tests leave `door.rs`, five land in
`door/system.rs`); after commit 4 and to the end: 156 (SC-001's floor is ≥ 148). After commit
1 also record the crate-graph delta (SC-007):

```sh
cd oxide_godot_core && cargo tree --prefix none | sort -u | wc -l   # 22 on 85186f6
```

## 2. Headless validation (after every commit from 2 on)

```sh
cd oxide-godot
/usr/bin/godot.x86_64 --headless --import --path .          # expect: Initialize godot-rust (...)
/usr/bin/godot.x86_64 --headless --path . main.tscn          # no new errors vs CLAUDE.md's baseline
/usr/bin/godot.x86_64 --headless --path . level.tscn
```

Autoload check (commit 2, scratch, not committed): a `zz_autoload_check.tscn` whose root script
does `print(get_node("/root/EcsWorld").get_class())` and quits — expected output `EcsWorld`
(SC-003; the GDScript path lookup is scratch code, FR-003 governs Rust code).

## 3. Parity harness (after commits 3, 4, 5)

Follow research.md R10 verbatim: copy `contracts/zz_ecs_parity.gd` (+ its observer script and
the one-node `zz_ecs_parity.tscn`) into both trees, run each case with `--fixed-fps 60
--quit-after 400` under separate `XDG_DATA_HOME`s, `diff` the observer lines (parity evidence)
and the `RAW` lines (timing table input). Expected:

| Case | Observer diff | RAW diff (prediction, research R5 — the harness decides) |
|---|---|---|
| (a) door | empty | empty (same physics step) |
| (b) part_disappear | empty | `puff_tree_exited` and the emitting transition stamped one frame LATER on v2 (deferred future resumption) |
| (c) blast | empty | empty (v2's future resumes in the same frame's message flush) |

Any non-empty observer diff is a parity failure to fix; any RAW difference goes into the spec's
"Measured timing differences" table with the exact frame delta and cause (FR-030). Delete the
`zz_*` files from both trees before committing.

## 4. User visual checkpoints

- After commit 4 — checkpoint (1): run the game, shoot a robot until it explodes; the debris
  puffs appear, emit and vanish as in v2.
- After commit 5 — checkpoint (2): let a robot's laser hit walls and the player; the impact
  blasts face the camera as it moves and disappear when their animation ends; a bullet
  explosion (unaffected by this milestone) still looks as in v2.

No door checkpoint (orphaned scene, backlog #30).

## 5. Review checklist (SC-006, constitution 1.5.1)

```sh
grep -n 'fn process\|fn physics_process' oxide_godot_core/oxide_godot_lib/src/{door,part_disappear,blast}.rs   # expect nothing
grep -rn 'run_schedule\|\.run(&mut' oxide_godot_core/oxide_godot_lib/src | grep -v '^.*ecs.rs'                    # expect nothing outside ecs.rs
grep -rn 'godot::task::spawn' oxide_godot_core/oxide_godot_lib/src/{door,part_disappear,blast}.rs                # expect nothing
grep -rn 'get_autoload_by_name\|/root/EcsWorld' oxide_godot_core/oxide_godot_lib/src                             # expect nothing (or only typed uses)
git diff 08bc3bd -- oxide_godot_core/oxide_godot_lib/src/{settings,menu,main_scene,level,debug_label,part,bullet,red_robot}* | wc -l   # expect 0
grep -c '^| ' docs/v3-tradeoffs.md    # ≥ 5 (header + 4 entries)
grep -n '^| 30 ' docs/v2-backlog.md   # still "open — deferred"
```
