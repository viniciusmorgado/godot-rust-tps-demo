# Quickstart: Milestone V3-B — validating the player entity

Prerequisites: `CLAUDE.md` toolchain, branch `v3` at or after `b309e2f`, the `v2` worktree
(`../oxide-godot-v2`, re-created in research; `git worktree add ../oxide-godot-v2 v2` + `cargo
build` + `--headless --import` if absent). Paths from the repository root.

## 1. Gates (before every commit)

```sh
cd oxide_godot_core && cargo build && cargo clippy && cargo test
```

Expected: 163 after commit 1 (156 + 2 setup + 5 apply); 177 after commit 2 (+ 9 player/system + 5 player_input/system); 179 after commit 3 (+ 2 shake) — SC-001's floor is ≥ 166
(data-model.md "Tests by name"); the 36 model tests preserved by name; clippy at zero warnings.

## 2. Headless validation (after commits 2 and 3)

```sh
cd oxide-godot
/usr/bin/godot.x86_64 --headless --import --path .            # Initialize godot-rust (...)
/usr/bin/godot.x86_64 --headless --path . --quit-after 120 main/main.tscn
/usr/bin/godot.x86_64 --headless --path . --quit-after 120 level/level.tscn
```

No new errors versus `CLAUDE.md`'s catalog (its intermittent items included).

## 3. Parity harness (after commit 2: a, b, c, e, f; after commit 3: d, and b/c again)

Copy `contracts/zz_ecs_parity.gd`, the two-line `zz_ecs_observer.gd`, the three-line
`zz_ecs_physics_probe.gd` (quoted in the harness) and the six-line `zz_ecs_parity.tscn` (V3-A
contract §4) into BOTH trees; then, per case:

```sh
for c in a b c d e f; do
  ( cd ../oxide-godot-v2/oxide-godot && XDG_DATA_HOME=/tmp/xdg-v2 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=$c )
  ( cd oxide-godot                    && XDG_DATA_HOME=/tmp/xdg-v3 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=$c )
  L2="/tmp/xdg-v2/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_$c.log"; L3="/tmp/xdg-v3/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_$c.log"
  diff "$L2" "$L3" && echo "case $c: IDENTICAL"
  diff <(grep '^RAW' "$L2") <(grep '^RAW' "$L3") || echo "case $c: RAW stamps differ (timing table)"
done
```

Paste the actual diff outputs; never write "identical" without them. Predictions (research R1–R3;
the harness decides): (a), (e), (f) identical; (b) and (c) identical on observer lines, RAW sound
stamps identical (same iteration, Edge Cases); (d) identical (seeded noise). Delete the `zz_*`
files from both trees before committing.

## 4. User visual checkpoints (both required, after commit 3)

1. Single player: walk, jump, aim, shoot a robot, get hit (shake), fall off the map and respawn,
   all as in v2; note whether backlog #29's FPS observation persists.
2. Multiplayer on one machine: host in one instance, join from a second through the menu; the
   remote player's movement, animation and aim replicate as in v2; each client controls only its
   own player.

## 5. Review checklist (SC-006, constitution 1.5.1)

```sh
grep -n 'fn process\|fn physics_process' oxide_godot_core/oxide_godot_lib/src/{player,player_input,camera_noise_shake}.rs   # expect nothing
grep -n 'fn input' oxide_godot_core/oxide_godot_lib/src/player_input.rs                                                     # the push-only callback only
grep -rn 'godot::task::spawn\|bind_mut::<EcsWorld>\|get_autoload_by_name::<EcsWorld>' oxide_godot_core/oxide_godot_lib/src/{player,player_input,camera_noise_shake}* # expect nothing
grep -rn 'godot::classes' oxide_godot_core/oxide_godot_lib/src/{player,player_input,camera_noise_shake}/system.rs           # expect nothing (pure)
git diff abfe35a -- oxide_godot_core/oxide_godot_lib/src/{player,player_input,camera_noise_shake}/model.rs | wc -l          # expect 0
git diff abfe35a -- oxide_godot_core/oxide_godot_lib/src/{settings,menu,main_scene,level,debug_label,part.rs,part/,bullet,red_robot,flying_forklift,hittable,door}* | wc -l   # expect 0
git diff abfe35a -- oxide-godot/player/player.tscn                                                                          # exactly the callback_mode_process line
grep -c '^| ' docs/v3-tradeoffs.md    # ≥ 14 (5 existing + 9)
grep -n '^| \(6\|29\) ' docs/v2-backlog.md   # still open
```
