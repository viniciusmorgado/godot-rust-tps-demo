# Quickstart: Milestone V2-B — Leaves and player input

Mirrors `specs/006-v2-typed-settings/quickstart.md`'s shape (gates, headless, worktree parity).

## 1. Gates (run after every commit, from `oxide_godot_core/`)

```sh
cd oxide_godot_core
cargo build
cargo clippy -- -D warnings   # no new warnings
cargo test                     # pure-module unit tests: player_input::model, camera_noise_shake::model, debug_label::pure
```

## 2. Headless validation (from `oxide-godot/oxide-godot/`)

```sh
/usr/bin/godot.x86_64 --headless --import --path .          # confirms "Initialize godot-rust (...)"
/usr/bin/godot.x86_64 --headless --path . player/player.tscn      # or main.tscn — the affected scene
```

Known pre-existing baseline errors (per `CLAUDE.md`) are not regressions; anything new is.

## 3. Parity worktree setup (once per milestone, reused across all 3 checkpoints)

```sh
cd /home/morgado/Experimento/oxide-godot
git worktree add ../oxide-godot-v1 v1
cd ../oxide-godot-v1/oxide_godot_core && cargo build
```

## 4. Running the harness on both trees

Copy `zz_leaves_parity.tscn`/`.gd` (built from `contracts/zz_leaves_parity.gd` during
implementation) into BOTH `oxide-godot/oxide-godot/` (this branch) and
`../oxide-godot-v1/oxide-godot/`. Run each case as its own process (separate `XDG_DATA_HOME` per
tree so `user://` doesn't collide between the two checkouts of the same project name, per
V2-A's precedent):

```sh
for case in a b c d; do
  XDG_DATA_HOME=/tmp/parity-v1 /usr/bin/godot.x86_64 --headless --path ../oxide-godot-v1/oxide-godot \
    res://zz_leaves_parity.tscn -- --case=$case > /tmp/v1-$case.txt
  XDG_DATA_HOME=/tmp/parity-v2 /usr/bin/godot.x86_64 --headless --path oxide-godot/oxide-godot \
    res://zz_leaves_parity.tscn -- --case=$case > /tmp/v2-$case.txt
done
diff /tmp/v1-a.txt /tmp/v2-a.txt   # expect identical (self-hit raycast case excluded from the script)
diff /tmp/v1-b.txt /tmp/v2-b.txt   # expect identical given the fixed seed(12345)
diff <(sed '/VRAM:/d' /tmp/v1-c.txt) <(sed '/VRAM:/d' /tmp/v2-c.txt)   # VRAM line is v2-only by design
diff /tmp/v1-d.txt /tmp/v2-d.txt   # expect identical frame counts
```

Case (a)'s script must avoid the self-hit raycast scenario (backlog #7 closes real self-exclusion
in `v2`; `v1` has no exclusion at all, so a shoot-target aimed through the player's own collider
would legitimately diverge — this is a pre-authorized, documented behavior change, not a harness
bug).

## 5. Structural checks (SC-002 / SC-003)

```sh
# SC-002: zero per-frame get_parent().cast()/.unwrap() on the 6 node references in player_input.rs
grep -n 'get_parent().*cast\|\.unwrap()' oxide_godot_core/oxide_godot_lib/src/player_input.rs
# expect: no matches inside `process`/`input` (OnEditor/OnReady resolve once, outside the frame callbacks)

# SC-003: zero nested connect_other in the two timer modules
grep -n 'connect_other' oxide_godot_core/oxide_godot_lib/src/part_disappear.rs oxide_godot_core/oxide_godot_lib/src/blast.rs
# expect: no matches

# Principle III: pure modules stay engine-free
grep -n 'Gd<\|Input::singleton\|RenderingServer::singleton\|Os::singleton\|GString\|StringName' \
  oxide_godot_core/oxide_godot_lib/src/player_input/model.rs \
  oxide_godot_core/oxide_godot_lib/src/camera_noise_shake/model.rs
# expect: no matches
```

## 6. Cleanup (final commit of the milestone)

```sh
rm -f oxide-godot/oxide-godot/zz_leaves_parity.tscn oxide-godot/oxide-godot/zz_leaves_parity.gd oxide-godot/oxide-godot/zz_leaves_parity.gd.uid
git worktree remove ../oxide-godot-v1
git worktree prune
git status   # clean on v2
git worktree list   # only the main checkout
```

## 7. User visual checkpoints (STOP for confirmation, not automated)

1. **After US1**: move (WASD-equivalent), hold-aim vs. tap-aim vs. toggle-aim, jump, shoot
   crosshair target tracking, fall below the map and watch the screen fade to black and back.
2. **After US2+US3**: shoot/get hit and observe camera shake; open the debug overlay (its
   toggle key) and confirm FPS/VSync/Memory/VRAM/Online lines, confirm it stops updating while
   hidden and refreshes immediately on the frame it's shown again.
3. **After US4**: kill a robot and watch its parts puff-disappear; fire a laser at a wall and
   watch the impact blast play and vanish — same timing as before.
