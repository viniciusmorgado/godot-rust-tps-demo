# Quickstart: Milestone V2-E — level, menu, main: the scene manager and the end of v2

Mirrors `specs/009-v2-enemy/quickstart.md`'s shape (gates, headless, worktree parity).

## 1. Gates (run after every commit, from `oxide_godot_core/`)

```sh
cd oxide_godot_core
cargo build
cargo clippy -- -D warnings   # no new warnings
cargo test                     # pure-module unit tests: level::model, flying_forklift::pure,
                                # menu::model
```

## 2. Headless validation (from `oxide-godot/oxide-godot/`)

```sh
/usr/bin/godot.x86_64 --headless --import --path .          # confirms "Initialize godot-rust (...)"
/usr/bin/godot.x86_64 --headless --path . main/main.tscn --quit-after 120   # end-to-end boot
/usr/bin/godot.x86_64 --headless --path . menu/menu.tscn --quit-after 120
/usr/bin/godot.x86_64 --headless --path . level/level.tscn --quit-after 120
```

Known pre-existing baseline errors/warnings (per `CLAUDE.md`, and the `--quit-after`
ObjectDB-leak-on-abrupt-shutdown artifact confirmed identical across every prior milestone's
runs) are not regressions; anything new is.

## 3. Parity worktree setup (once per milestone)

```sh
cd /home/morgado/Experimento/oxide-godot
git worktree add ../oxide-godot-v1 v1
cd ../oxide-godot-v1/oxide_godot_core && cargo build
cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .
```

## 4. Running the harness on both trees

**Case (a)** reuses `specs/006-v2-typed-settings/contracts/zz_settings_parity.gd` UNCHANGED —
copy that exact file (not a new one) to both `oxide-godot/oxide-godot/` and
`../oxide-godot-v1/oxide-godot/`, run it as V2-A originally did.

**Cases (b)/(c)/(d)** use `zz_end_parity.tscn`/`.gd` (built from
`contracts/zz_end_parity.gd` during implementation), copied identically to both trees:

```sh
for case in b c d; do
  XDG_DATA_HOME=/tmp/parity-v1 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path ../oxide-godot-v1/oxide-godot \
    res://zz_end_parity.tscn -- --case=$case > /tmp/v1-$case.txt 2>/tmp/v1-$case.stderr
  XDG_DATA_HOME=/tmp/parity-v2 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path oxide-godot/oxide-godot \
    res://zz_end_parity.tscn -- --case=$case > /tmp/v2-$case.txt 2>/tmp/v2-$case.stderr
done
```

Diffing:
- **(a)**: identical `settings.ini` bytes + applied engine state on both trees (V2-A's own
  original pass/fail criteria, unchanged).
- **(b)**: identical `sdfgi_enabled`/`VoxelGI`/`ReflectionProbes`/`LightmapGI`-presence dumps
  for all 9 `(gi_type × gi_quality)` cells on both trees.
- **(c)**: identical `SpawnedNodes` child count/names and robot respawn frame count on both
  trees; the forklift's picked model INDEX is a DOCUMENTED DIVERGENCE (`v1`'s draw is
  unseeded) — compare "exactly one model visible" only, not the specific index.
- **(d)**: identical child-node-type-under-`Main` sequence and identical transition frame
  numbers on both trees (research.md R1 predicts zero timing delta, since `Callable
  ::call_deferred` and `Object::call_deferred` share the same engine queue — confirm this
  empirically here, don't just assume it).

## 5. Structural checks (SC-002 through SC-005)

```sh
# SC-002: menu.rs's two settings handlers each <= 40 lines
awk '/fn _on_settings_pressed/,/^    }$/' oxide_godot_core/oxide_godot_lib/src/menu.rs | wc -l
awk '/fn _on_apply_pressed/,/^    }$/' oxide_godot_core/oxide_godot_lib/src/menu.rs | wc -l

# SC-003: zero load(/get_node_as inside level.rs's spawn_robot/add_player/ready bodies
grep -n 'load(\|get_node_as' oxide_godot_core/oxide_godot_lib/src/level.rs
# expect: no matches inside those specific function bodies (review any hits outside them --
# e.g. #[init] attributes, or the ONE remaining per-level-instance .lmbake load in
# apply_gi_plan, which research.md R3 explicitly keeps)

# SC-004: zero create_timer(...).connect_other chains remain in level.rs
grep -n 'connect_other' oxide_godot_core/oxide_godot_lib/src/level.rs
# expect: only the ALREADY-TYPED peer_connected/peer_disconnected connections (unchanged,
# not a timer chain) -- no create_timer(...).signals().timeout().connect_other left

# SC-005: crate-wide -- the ONLY by-name dynamic access left is .rpc(
grep -rn 'has_signal\|has_method\|\.call(\|\.get("\|call_deferred(".*"\|Callable::from_object_method' \
  oxide_godot_core/oxide_godot_lib/src/
# expect: no matches anywhere in the crate
grep -rn '\.rpc(' oxide_godot_core/oxide_godot_lib/src/
# expect: exactly the 7 sites listed in contracts/end-of-v2-api.md's "Crate-wide residual"
```

## 6. Cleanup (final commit of the milestone)

```sh
rm -f oxide-godot/oxide-godot/zz_settings_parity.tscn oxide-godot/oxide-godot/zz_settings_parity.gd oxide-godot/oxide-godot/zz_settings_parity.gd.uid
rm -f oxide-godot/oxide-godot/zz_end_parity.tscn oxide-godot/oxide-godot/zz_end_parity.gd oxide-godot/oxide-godot/zz_end_parity.gd.uid
git worktree remove ../oxide-godot-v1
git worktree prune
git status   # clean on v2
git worktree list   # only the main checkout
```

## 7. User visual checkpoints (STOP for confirmation, not automated)

1. **After commit 3** (`main_scene.rs` + `level.rs` + `flying_forklift.rs`): boot the game —
   menu appears; Play loads the level; robots and players spawn; killing a robot brings a new
   one back ~15 seconds later (note, without acting on it, whether backlog #32's respawn hitch
   changed); flying forklifts show one of three models; pressing Esc in the level returns to
   the menu; Play again still works.
2. **After commit 4** (`menu.rs`): open Settings, change EVERY row (display mode, vsync, max
   FPS, resolution scale, scale filter, TAA, MSAA, screen-space AA, shadow mapping, GI type,
   GI quality, SSAO, SSIL, bloom, volumetric fog), Apply, reopen Settings — every row shows the
   change that was just made; F11 toggles fullscreen.
