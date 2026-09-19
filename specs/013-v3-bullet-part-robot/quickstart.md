# Quickstart: Milestone V3-C — validating the enemy entities

Prerequisites: `CLAUDE.md` toolchain, branch `v3` at or after `e5ca11a`, the `v2` worktree
(`../oxide-godot-v2` at `e2932b4`, re-created on 2026-09-19 for research; if absent: `git worktree
add ../oxide-godot-v2 v2` + `cargo build` + `--headless --import`). Paths from the repository root.

## 1. Gates (before every commit)

```sh
cd oxide_godot_core && cargo build && cargo clippy && cargo test
```

Expected: 187 after commit 1 (179 + 8 drain arms); 192 after commit 2 (+ 5 bullet/system); 214
after commit 3 (+ 5 part/system + 14 red_robot/system + 1 setup) — data-model.md "Tests by
name"; the 33 pure tests preserved by name; clippy at zero warnings.

## 2. Headless validation (after commits 2 and 3)

```sh
cd oxide-godot
/usr/bin/godot.x86_64 --headless --import --path .            # Initialize godot-rust (...)
/usr/bin/godot.x86_64 --headless --path . --quit-after 120 main/main.tscn
/usr/bin/godot.x86_64 --headless --path . --quit-after 120 level/level.tscn
```

No new errors versus `CLAUDE.md`'s catalog (its intermittent items included).

## 3. Parity harness (after commit 2: c; after commit 3: a, b, d, e)

Copy `contracts/zz_ecs_parity.gd`, the two-line `zz_ecs_observer.gd`, the three-line
`zz_ecs_physics_probe.gd` (both quoted in the harness) and the six-line `zz_ecs_parity.tscn`
(V3-A contract §4) into BOTH trees — into `oxide-godot/oxide-godot/` and
`../oxide-godot-v2/oxide-godot/`, the Godot project directories, not the repository roots; then:

```sh
for c in a b c d e; do
  q=900; [ $c = c ] && q=450; [ $c = d ] && q=200; [ $c = e ] && q=305
  ( cd ../oxide-godot-v2/oxide-godot && XDG_DATA_HOME=/tmp/xdg-v2 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after $q zz_ecs_parity.tscn -- --case=$c )
  ( cd oxide-godot                    && XDG_DATA_HOME=/tmp/xdg-v3 /usr/bin/godot.x86_64 --headless --path . --fixed-fps 60 --quit-after $q zz_ecs_parity.tscn -- --case=$c )
  L2="/tmp/xdg-v2/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_$c.log"; L3="/tmp/xdg-v3/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_$c.log"
  diff "$L2" "$L3" && echo "case $c: IDENTICAL"
  diff <(grep '^RAW' "$L2") <(grep '^RAW' "$L3") || echo "case $c: RAW stamps differ (timing table)"
done
```

Paste the actual diff outputs; never write "identical" without them. Predictions (research
R1–R8; the harness decides): all five identical, including the parts' positions in (b) (R2:
the solver is deterministic on both trees). Delete the `zz_*` files from both trees before
committing.

## 4. User visual checkpoints (both required, after commit 3)

1. Single player: a robot approaches, aims, fires the laser (clip, ember, blast); the player is
   hit (shake after 0.1 s); the player kills a robot (parts fly, fade, puff; the robot disappears
   after 10 s and respawns); bullets explode on walls and on expiry — all as in v2; note whether
   backlog #29's FPS observation persists.
2. Multiplayer on one machine: host in one instance, join from a second; the client sees the
   host's robots move, aim and shoot; parts fly and fade on the client; a client's bullet kills a
   robot and the death replicates (parts, puffs, removal after 10 s).

## 5. Review checklist (SC-006, constitution 1.5.2)

```sh
grep -n 'fn process\|fn physics_process' oxide_godot_core/oxide_godot_lib/src/{bullet,part,red_robot}.rs           # expect nothing
grep -rn 'godot::task::spawn\|bind_mut::<EcsWorld>\|get_autoload_by_name::<EcsWorld>' oxide_godot_core/oxide_godot_lib/src/{bullet,part,red_robot}*   # expect nothing
grep -rn 'godot::classes' oxide_godot_core/oxide_godot_lib/src/{bullet,part,red_robot}/system.rs                    # expect nothing (pure)
git diff 30d1a4d -- oxide_godot_core/oxide_godot_lib/src/red_robot/model.rs | wc -l                                  # expect 0
git diff 30d1a4d -- oxide_godot_core/oxide_godot_lib/src/{bullet,part}.rs | grep '^[-+]' | grep -c 'mod pure'         # expect 2 (only the pub(crate) token, one per file)
git diff 30d1a4d --stat -- oxide_godot_core/oxide_godot_lib/src/hittable.rs                                          # additions only: HitKind, kind_of, impl HitKind
git diff 30d1a4d -- oxide_godot_core/oxide_godot_lib/src/hittable.rs | grep '^-' | grep -v '^---' | wc -l             # expect 0 (no line removed)
(cd oxide_godot_core/oxide_godot_lib/src && git diff 30d1a4d --stat -- player.rs player/ player_input.rs player_input/ camera_noise_shake.rs camera_noise_shake/ door.rs door/ part_disappear.rs part_disappear/ blast.rs level.rs flying_forklift.rs settings.rs settings/ menu.rs menu/ main_scene.rs debug_label.rs | wc -l)   # expect 0
git diff 30d1a4d -- oxide-godot/enemies/red_robot/red_robot.tscn                                                     # exactly the callback_mode_process line
git diff 30d1a4d --stat -- oxide-godot/ | grep -v red_robot.tscn | wc -l                                             # expect 0 (no other scene change)
grep -c '^| ' docs/v3-tradeoffs.md    # ≥ 23 (14 existing + 9)
grep -n '^| \(29\|30\|31\) ' docs/v2-backlog.md   # 29/30 open; 31 open unless closed at plan review
```
