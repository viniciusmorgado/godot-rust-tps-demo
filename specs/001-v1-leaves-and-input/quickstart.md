# Quickstart: validation of each port (Milestone A)

Execution/validation guide — what to run, in order, and what to expect. Code details are
in [research.md](research.md); the names to check, in [contracts/](contracts/). All paths
are relative to the repository root (`oxide-godot/`, where this `specs/` lives).

## Prerequisites

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (there is no `godot` on the PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 already resolved (do not edit `Cargo.toml`).
- No Godot editor open on the project during headless validation (avoids concurrent
  hot-reload and `.godot/` lock). If it is open, close it before running step 2.

## Baseline (measured on 2026-09-15, commit `6b22de3`, before any port)

- `cargo build`: **0 warnings**.
- Headless import: line 1 = `Initialize godot-rust (API v4.6.stable.official, runtime v4.7.2.stable.official, safeguards strict)`;
  **0 `ERROR` lines**. The 3 upstream errors catalogued in `CLAUDE.md` (`Cannon_Charge already exists`,
  missing `doorsimple_d.png`, `surfaces.is_empty()`) only appear on a clean import (`.godot/` deleted);
  they do not count as a regression in any case.
- Headless run of `level.tscn`, `player.tscn`, `part_disappear.tscn`, `impact_effect.tscn`:
  **0 `ERROR`/`SCRIPT ERROR` lines**, exit **124** (timeout — game scenes do not end on their own;
  `part_disappear.tscn` does not either: `queue_free` of the root does not end the `SceneTree`). Two
  benign, pre-existing `WARNING`s: `HDR output requested, but it is not supported by this
  display server` and `[Physics interpolation] Interpolated Camera3D triggered from outside physics
  process` (only in the scenes with Player).

Any `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Nonexistent function`, `Invalid
get/set index` or `panicked` line that is not in this baseline is a regression of the port.

## Cycle per script (repeat 5 times, in order 1 → 5)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
```

Expected: `Finished 'dev' profile`, **zero new warnings** (baseline 0 → must remain 0).
Quick count: `cargo build 2>&1 | grep -c '^warning'` → `0`.

### 2. Headless import (did the extension load?)

```bash
cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log
grep -n 'Initialize godot-rust' /tmp/import.log        # must exist (line 1)
grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log            # expected: empty (or only the 3 from upstream)
```

### 3. Headless run of the affected scene(s)

```bash
cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . <scene>.tscn 2>&1 | tee /tmp/run.log
grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log   # expected: empty
```

Exit 124 (timeout) is the expected result; exit 0 is also acceptable. The validation is the absence of
new lines in the `grep`, not the exit code.

| Port | `<scene>` to run |
|---|---|
| 1 `debug.gd` | `level/level.tscn` |
| 2 `part_disappear.gd` | `enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn` |
| 3 `blast.gd` | `enemies/red_robot/laser/impact_effect/impact_effect.tscn` |
| 4 `camera_noise_shake_effect.gd` | `player/player.tscn` **and** `level/level.tscn` |
| 5 `player_input.gd` | `player/player.tscn` **and** `level/level.tscn` |

For ports 2 and 3 it is also worth running `level/level.tscn` (the robot instantiates the two scenes at
runtime, but only under interaction — headless does not exercise it; the isolated instantiation above already proves that
the class resolves and `ready()` runs).

### 4. Mechanical checks of the cycle (Principle II)

```bash
# the script and the uid are gone, and no scene still points to them
ls oxide-godot/<path>/<script>.gd oxide-godot/<path>/<script>.gd.uid   # expected: No such file
grep -rn '<uid of the .gd>' oxide-godot/                                          # expected: empty
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # expected: empty

# the node changed type and no longer has a script
grep -n 'type="<RustClass>"' oxide-godot/<scene>.tscn                          # expected: 1 line
grep -n 'ExtResource("<script id>")' oxide-godot/<scene>.tscn                # expected: empty

# the other 10 .gd did not change
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # expected: only the deletion of this port's script
```

Uids and ids per port are in the "Scene edits" table of [plan.md](plan.md).

### 5. Name check (ports 4 and 5)

Run the commands of the section "Verification before the commit" in
[contracts/camera-noise-shake.md](contracts/camera-noise-shake.md) and
[contracts/player-input-synchronizer.md](contracts/player-input-synchronizer.md); each name
found must exist in the Rust class with the same name.

### 6. Visual validation (user, in the editor/game)

Open the project in the editor, run (F5), enter the level and check the corresponding item of the table
"Visual validation per script" of [plan.md](plan.md), comparing with `../oxide_godot_origins/`.
When opening the edited scene in the editor: the node must appear with the Rust type, with no script attached, and
(port 5) with the 6 references and the `replication_config` preserved in the inspector.

### 7. v2 backlog

If any improvement was noticed during the port, add a line to `docs/v2-backlog.md`
(origin + improvement + motivation) **before** the commit. Candidates already identified: research.md
§"Candidate v2 backlog".

### 8. Commit

One commit per script, on `main`, including: new Rust module + `lib.rs`, edited `.tscn`,
`.gd` + `.gd.uid` deleted, `docs/v2-backlog.md` (if there is an entry). Message:

```
Port <script>.gd → <RustClass> (<Base>); <scene>.tscn: node <Name> type="<Base>"→"<RustClass>"

- <relevant notes: translation decisions, quirks preserved>
- v2 backlog: <items added, or "none">
```

## Final verification of the milestone (after the 5th commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l          # expected: 10
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # expected: 10
git diff --stat 6b22de3 -- 'oxide-godot/**/*.gd'                       # expected: only 5 deletions
git log --oneline 6b22de3..HEAD                                        # expected: 5 "Port ..." commits
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 5 modules
```

And, in the game: menu → level → move, look, aim (toggle and hold), jump, shoot, shake, F3,
laser impact, parts disappearing — identical to the original (SC-002).
