# Quickstart: Milestone V2-C — player, bullet, door

Mirrors `specs/007-v2-leaves-and-input/quickstart.md`'s shape (gates, headless, worktree parity).

## 1. Gates (run after every commit, from `oxide_godot_core/`)

```sh
cd oxide_godot_core
cargo build
cargo clippy -- -D warnings   # no new warnings
cargo test                     # pure-module unit tests: player::model, bullet::pure, door::pure
```

## 2. Headless validation (from `oxide-godot/oxide-godot/`)

```sh
/usr/bin/godot.x86_64 --headless --import --path .          # confirms "Initialize godot-rust (...)"
/usr/bin/godot.x86_64 --headless --path . main/main.tscn --quit-after 120
/usr/bin/godot.x86_64 --headless --path . level/level.tscn --quit-after 120
```

Known pre-existing baseline errors/warnings (per `CLAUDE.md`, and the `--quit-after`
ObjectDB-leak-on-abrupt-shutdown artifact confirmed identical across V2-B's runs) are not
regressions; anything new is.

## 3. Parity worktree setup (once per milestone)

```sh
cd /home/morgado/Experimento/oxide-godot
git worktree add ../oxide-godot-v1 v1
cd ../oxide-godot-v1/oxide_godot_core && cargo build
cd ../oxide-godot-v1/oxide-godot && /usr/bin/godot.x86_64 --headless --import --path .
```

## 4. Running the harness on both trees

Copy `zz_player_parity.tscn`/`.gd` (built from `contracts/zz_player_parity.gd` during
implementation) into BOTH `oxide-godot/oxide-godot/` (this branch) and
`../oxide-godot-v1/oxide-godot/`. Run each case as its own process (`--fixed-fps 60` is
mandatory for determinism — V2-B found that variable `delta` jitters slightly between separate
process launches and shows up as noise in any dt-accumulated value):

```sh
for case in a b c; do
  XDG_DATA_HOME=/tmp/parity-v1 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path ../oxide-godot-v1/oxide-godot \
    res://zz_player_parity.tscn -- --case=$case > /tmp/v1-$case.txt 2>/tmp/v1-$case.stderr
  XDG_DATA_HOME=/tmp/parity-v2 /usr/bin/godot.x86_64 --headless --fixed-fps 60 --path oxide-godot/oxide-godot \
    res://zz_player_parity.tscn -- --case=$case > /tmp/v2-$case.txt 2>/tmp/v2-$case.stderr
done
```

Diffing:
- **(a)**: diff the two dumps EXCLUDING frames 0-1 (backlog #10) and the frames immediately
  after the scripted below-`-40` teleport (backlog #11) — a small Python/jq script, not plain
  `diff`, since specific frame indices are excluded, not whole-file lines.
- **(b)**: `wall_frames` and `robot_health_before/after` must match exactly; `same_tick_explodes`
  is a DOCUMENTED DIVERGENCE (expect `v1` = 2, `v2` = 1) — report it, do not assert equality.
- **(c)**: `player_opens`/`robot_opens` must match exactly (`true`/`false` respectively on
  both branches); also grep each `.stderr` file for any new warning/error the robot-entry case
  might have introduced (expect none on either branch).

## 5. Structural checks (SC-002 / SC-003 / SC-004)

```sh
# SC-002: zero get_node_as/load::<PackedScene> inside player.rs's physics_process/shoot/apply_input
grep -n 'get_node_as\|load::<PackedScene>' oxide_godot_core/oxide_godot_lib/src/player.rs
# expect: no matches (both moved to OnReady/preloaded fields, resolved outside the frame callbacks)

# SC-003: at most one player_input.bind()/bind_mut() call per physics frame — code review, not
# grep (the pattern legitimately appears once); read player.rs's physics_process/apply_input
# and the non-authority branch to confirm by inspection.

# SC-004: zero has_method calls in bullet.rs
grep -n 'has_method' oxide_godot_core/oxide_godot_lib/src/bullet.rs
# expect: no matches

# Principle III: pure modules stay engine-free
grep -n 'Gd<\|Input::singleton\|RenderingServer::singleton\|Os::singleton\|GString\|StringName' \
  oxide_godot_core/oxide_godot_lib/src/player/model.rs
# expect: no matches (bullet.rs/door.rs's inline `mod pure` blocks likewise, checked by code review
# since grep on the whole file would also match the surrounding glue)
```

## 6. Cleanup (final commit of the milestone)

```sh
rm -f oxide-godot/oxide-godot/zz_player_parity.tscn oxide-godot/oxide-godot/zz_player_parity.gd oxide-godot/oxide-godot/zz_player_parity.gd.uid
git worktree remove ../oxide-godot-v1
git worktree prune
git status   # clean on v2
git worktree list   # only the main checkout
```

## 7. User visual checkpoints (STOP for confirmation, not automated)

1. **After US1** (`player.rs` glue commit): walk, strafe while aiming, jump, land, shoot at a
   wall/robot, fall below the map and confirm respawn (no leftover fall velocity — backlog
   #11 — and no spurious landing sound right at spawn — backlog #10). Note, without acting on
   it, whether the first shot of a sequence still shows the ~10-frame hitch (backlog #28).
2. **After US2+US3** (`bullet.rs`/`door.rs` commits): shoot a wall and a robot — both still
   explode/take damage identically; walk into a door — it opens; try to get a robot to open a
   door by walking it through one — it should not open.
