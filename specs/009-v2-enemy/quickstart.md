# Quickstart: Milestone V2-D — enemy: `part.rs` and `red_robot.rs`

Mirrors `specs/008-v2-player-bullet-door/quickstart.md`'s shape (gates, headless, worktree
parity).

## 1. Gates (run after every commit, from `oxide_godot_core/`)

```sh
cd oxide_godot_core
cargo build
cargo clippy -- -D warnings   # no new warnings
cargo test                     # pure-module unit tests: red_robot::model, part::pure
```

## 2. Headless validation (from `oxide-godot/oxide-godot/`)

```sh
/usr/bin/godot.x86_64 --headless --import --path .          # confirms "Initialize godot-rust (...)"
/usr/bin/godot.x86_64 --headless --path . main/main.tscn --quit-after 120
/usr/bin/godot.x86_64 --headless --path . level/level.tscn --quit-after 120
```

Known pre-existing baseline errors/warnings (per `CLAUDE.md`, and the `--quit-after`
ObjectDB-leak-on-abrupt-shutdown artifact confirmed identical across V2-B/C's runs) are not
regressions; anything new is.

## 3. Parity worktree setup (once per milestone)

```sh
cd /home/morgado/Experimento/oxide-godot
git worktree add ../oxide-godot-v1 v1
cd ../oxide-godot-v1/oxide_godot_core && cargo build
cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .
```

## 4. Running the harness on both trees

Copy `zz_enemy_parity.tscn`/`.gd` (built from `contracts/zz_enemy_parity.gd` during
implementation) into BOTH `oxide-godot/oxide-godot/` (this branch) and
`../oxide-godot-v1/oxide-godot/`. Run each case as its own process (`--fixed-fps 60` mandatory
for determinism, per V2-B/C's established finding):

```sh
for case in a b; do
  XDG_DATA_HOME=/tmp/parity-v1 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path ../oxide-godot-v1/oxide-godot \
    res://zz_enemy_parity.tscn -- --case=$case > /tmp/v1-$case.txt 2>/tmp/v1-$case.stderr
  XDG_DATA_HOME=/tmp/parity-v2 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path oxide-godot/oxide-godot \
    res://zz_enemy_parity.tscn -- --case=$case > /tmp/v2-$case.txt 2>/tmp/v2-$case.stderr
done
```

Diffing:
- **(a)**: `state`, `target_position`, `aim_preparing`, `aim/blend_position`, the frame
  `play_shoot` fires, and the camera-shake proxy must all match exactly between `v1`/`v2` —
  research.md R8 confirms no harness-side collision-mask override is needed here (unlike V2-C's
  door case), since `PlayerDetectionArea`'s `mask=2` already intersects `player.tscn`'s
  `layer=6` under Godot's one-directional area-detection rule.
- **(b)**: `dead`, `Death` visibility, each part's `fade_value` trace, the puff's WORLD POSITION
  and the `exploded` signal count must match exactly; the removal frame count (~600 @60fps for
  the 10s delay) must match within a small tolerance (async-timer scheduling jitter, if any); the
  puff's PARENT PATH is a DOCUMENTED DIVERGENCE (backlog #15 — expect `Death` on `v1`, the
  robot's own parent on `v2`) — report it, do not assert equality.

## 5. Structural checks (SC-002 / SC-003 / SC-004)

```sh
# SC-002: exactly ONE intersect_ray call site in red_robot.rs
grep -c 'intersect_ray' oxide_godot_core/oxide_godot_lib/src/red_robot.rs
# expect: 1

# SC-003: zero load(/get_node_as/has_feature inside physics_process/shoot/hit/explode/destroy
grep -n 'load(\|get_node_as\|has_feature' oxide_godot_core/oxide_godot_lib/src/red_robot.rs \
  oxide_godot_core/oxide_godot_lib/src/part.rs
# expect: no matches inside those specific function bodies (code review confirms the grep hits,
# if any, are outside them — e.g. the OnReady #[init] attributes themselves are not calls
# "inside" a callback body)

# SC-004: zero nested connect_other timer chains in either module
grep -n 'connect_other' oxide_godot_core/oxide_godot_lib/src/red_robot.rs \
  oxide_godot_core/oxide_godot_lib/src/part.rs
# expect: no matches (both replaced by godot::task::spawn + to_future())

# Principle III: pure modules stay engine-free
grep -n 'Gd<\|Input::singleton\|RenderingServer::singleton\|Os::singleton\|GString\|StringName' \
  oxide_godot_core/oxide_godot_lib/src/red_robot/model.rs
# expect: no matches (part.rs's inline `mod pure` likewise, checked by code review since grep on
# the whole file would also match the surrounding glue)
```

## 6. Cleanup (final commit of the milestone)

```sh
rm -f oxide-godot/oxide-godot/zz_enemy_parity.tscn oxide-godot/oxide-godot/zz_enemy_parity.gd oxide-godot/oxide-godot/zz_enemy_parity.gd.uid
git worktree remove ../oxide-godot-v1
git worktree prune
git status   # clean on v2
git worktree list   # only the main checkout
```

## 7. User visual checkpoints (STOP for confirmation, not automated)

1. **After commit 2** (`red_robot.rs` glue): a robot detects the player, turns to face, prepares,
   aims, fires its laser (impact effect visible, camera shakes if it hits the player), and — if
   hit 5 times (`shoot_check`/manual RPCs, or via `test_shoot`) — dies, its three parts fly apart,
   and it vanishes roughly 10 seconds later. Note, without acting on it, whether the robot's
   first laser shot of a sequence still shows the previously-observed hitch (backlog #28).
2. **After commit 3** (`part.rs`): parts visibly fly, spin, fade near the end of their lifetime,
   disappear, and leave a brief particle puff at the correct position (the puff's new PARENT —
   backlog #15 — has no visible effect under normal play; nothing to confirm visually beyond
   "it still looks the same").
