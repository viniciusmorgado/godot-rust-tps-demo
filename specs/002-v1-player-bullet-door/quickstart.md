# Quickstart: validation of each port (Milestone B)

Execution/validation guide — what to run, in order, and what to expect. Code details in
[research.md](research.md); names to check in [contracts/](contracts/). Paths relative to the
repository root (`oxide-godot/`, where this `specs/` lives).

## Prerequisites

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (there is no `godot` on PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 already resolved (do not edit `Cargo.toml`).
- **No Godot editor open** on the project during headless validation (`pgrep -a godot` empty).
  Warn the user before starting; never kill their process.
- Clean tree on `main` (`git status --short` empty) before each port.

## Baseline (measured on 2026-09-15, commit `108584e`, before any port of this milestone)

- `cargo build`: **0 warnings**.
- Headless import: line 1 = `Initialize godot-rust (API v4.6.stable.official, runtime v4.7.2.stable.official, safeguards strict)`;
  **0 `ERROR` lines** with `.godot/` already imported. The 3 upstream errors cataloged in
  `CLAUDE.md` (`Cannon_Charge already exists`, missing `doorsimple_d.png`, `surfaces.is_empty()`)
  only appear on a clean import and do not count as regressions.
- Headless run of `level.tscn`, `player.tscn`, `player/bullet/bullet.tscn`: **0
  `ERROR`/`SCRIPT ERROR` lines**, exit **124**. Benign, pre-existing `WARNING`s: `HDR output
  requested, but it is not supported by this display server` (all scenes) and `[Physics
  interpolation] Interpolated Camera3D triggered from outside physics process` (scenes with Player).
- **`door/door.tscn`: exactly 1 `ERROR`** — `ERROR: Node not found: "DoorModel/AnimationPlayer"
  (relative to "/root/Door").` (`grep -c 'Node not found' /tmp/run.log` → `1`), exit 124, 1
  WARNING (HDR). **This error MUST disappear after port 3** (FR-027, SC-003). It is not in the
  `CLAUDE.md` catalog — nothing to remove from there.

Any `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` line that is not in this baseline is a regression of the port.

## Cycle per script (repeat 3 times, in the order 1 → 2 → 3)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
cargo build 2>&1 | grep -c '^warning'      # expected: 0
```

### 2. Headless import (did the extension load?)

```bash
cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log
grep -n 'Initialize godot-rust' /tmp/import.log        # must exist (line 1)
grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log            # expected: empty (or only the 3 upstream ones)
```

### 3. Headless run of the affected scene(s)

```bash
cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . <scene>.tscn 2>&1 | tee /tmp/run.log
grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log   # expected: empty
grep -c '^WARNING' /tmp/run.log                                                                         # expected: only the benign ones from the baseline
```

Exit 124 (timeout) is the expected one; exit 0 is also acceptable. The validation is the absence of new
lines in the `grep`.

| Port | `<scene>` to run | What it exercises |
|---|---|---|
| 1 `player.gd` | `player/player.tscn` **and** `level/level.tscn` | Player `ready` (OnReady ×10, `orientation`), `physics_process` on the server (`apply_input`, WALK, first landing → `land` RPC + sound), `BulletCache` (bullet still GDScript). In the level: `level.gd` spawns the Player (`name`, `player_id` outside the tree → setter), `red_robot.gd` resolves `is Player`/`add_camera_shake_trauma` by name — any wrong name shows up here |
| 2 `bullet.gd` | `player/bullet/bullet.tscn`, `player/player.tscn` **and** `level/level.tscn` | Isolated bullet: `ready` (server), flight, **expires at 5 s → `explode` → `Settings` read → `destroy` at 1.5 s of the animation** (all within the 20 s). `player.tscn`: `BulletCache` instantiates the Rust class. Level: full integration |
| 3 `door.gd` | `door/door.tscn` (+ `level/level.tscn` for safety) | Instantiation without `Node not found`; `_on_door_body_entered` connection resolves |

**Port 3 — mandatory additional check**:

```bash
grep -c 'Node not found' /tmp/run.log          # expected: 0 (baseline: 1)
```

### 4. Mechanical checks of the cycle (Principle II)

```bash
# the script and the uid are gone, and no scene still points to them
ls oxide-godot/<path>/<script>.gd oxide-godot/<path>/<script>.gd.uid   # expected: No such file
grep -rn '<uid of the .gd>' oxide-godot/                                          # expected: empty
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # expected: empty

# the node changed type and no longer has a script
grep -n 'type="<RustClass>"' oxide-godot/<scene>.tscn                          # expected: 1 line
grep -n 'ExtResource("1")' oxide-godot/<scene>.tscn                             # expected: empty (in the 3 ports the script id is "1")

# the 7 .gd outside the milestone did not change
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # expected: only the deletion of this port's script
```

Uids, ids and lines per port: "Scene editing" table of [plan.md](plan.md).

### 5. Name check (all ports)

Run the "Verification before the commit" section of the port's contract
([contracts/player.md](contracts/player.md), [bullet.md](contracts/bullet.md),
[door.md](contracts/door.md)); each name found must exist in the Rust class with the same name.

In port 1, also check that `player_input.rs` and `camera_noise_shake.rs` only changed in
visibility:

```bash
git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player_input.rs oxide_godot_core/oxide_godot_lib/src/camera_noise_shake.rs | grep '^[-+]' | grep -v '^[-+][-+]' | grep -v 'pub(crate)'
# expected: only the corresponding original "-" lines (10 -/+ pairs in total: 6 fields + 3 methods + add_trauma); no other line
```

### 6. Visual validation (user, in the editor/game)

Open the project in the editor, run (F5), enter the level and check the port's item in the
"Visual validation per script" table of [plan.md](plan.md), comparing with `../oxide_godot_origins/`.
When opening the edited scene: the root node must appear with the Rust type, with no script attached;
`player.tscn` must keep `ServerSynchronizer`/`InputSynchronizer`/`BulletCache` intact.

### 7. v2 backlog

Add to `docs/v2-backlog.md` the candidates from research.md §"Candidate v2 backlog"
assigned to **this** script (port 1: items 10–12; port 2: item 13; port 3: item 14), **before** the
commit. Items 1 (Settings) and 2 (`Hittable`) already exist — do not duplicate. Numbering continues from 10.

### 8. Commit

One commit per script, on `main`, including: new Rust module + `lib.rs` (+ in port 1, the two
visibility changes), edited `.tscn`, deleted `.gd` + `.gd.uid`, `docs/v2-backlog.md`
(if there is an entry) and, **in port 3, the new `docs/upstream-bugs.md`**. Author:
the repository author. Message:

```
Port <script>.gd → <RustClass> (<Base>); <scene>.tscn: node <Name> type="<Base>"→"<RustClass>"

- <notes: translation decisions, preserved quirks>
- v2 backlog: <items added, or "none">
```

Port 1 adds the note: `- player_input.rs / camera_noise_shake.rs: only pub(crate) visibility
on the fields/methods consumed by the Player (typed access, FR-010/FR-011); no logic moved`.

**Port 3 — mandatory format (requirement (c) of the bug clause)**:

```
Port door.gd → Door (Area3D); door.tscn: node Door type="Area3D"→"Door"

- upstream bug fix: door.gd referenced "DoorModel/AnimationPlayer" (non-existent node); the port
  references "DoorModel2/AnimationPlayer" — the door now opens and the "ERROR: Node not found"
  disappears (baseline 1 → 0). Minimal fix; scene node not renamed; open logic intact.
- docs/upstream-bugs.md created with entry #1 (commit: this one).
- CLAUDE.md: catalog unchanged — the door error was never in it (only the 3 import errors).
- v2 backlog: item 14
```

"commit" column of `docs/upstream-bugs.md`: the hash only exists after the commit, so the entry
identifies the commit by its **subject** (`Port door.gd → Door (Area3D); ...`) — enough for
`git log --grep`. If the literal hash is desired, `git commit --amend` right after the port
3 commit, **before** any other commit (still one commit per script).

## Final verification of the milestone (after the 3rd commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l          # expected: 7
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # expected: 7
git diff --stat 108584e -- 'oxide-godot/**/*.gd'                       # expected: only 3 deletions (player.gd, bullet.gd, door.gd)
git log --oneline 108584e..HEAD | grep -c '^[0-9a-f]* Port '           # expected: 3
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 8 modules (debug_label, part_disappear, blast, camera_noise_shake, player_input, player, bullet, door)
test -f docs/upstream-bugs.md && grep -c '^| 1 ' docs/upstream-bugs.md # 1 entry
git diff --stat 108584e -- CLAUDE.md                                   # expected: empty
grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/      # expected: 1 line (door.rs)
```

And, in the game (SC-002): menu → level → move, jump (sound), land (sound), aim, shoot with a visible
bullet that explodes and hits robots, camera shake, respawn when falling below −40 — identical
to the original. Door (SC-009): in an isolated, **uncommitted** test scene (e.g. in the scratchpad or
a temporary `.tscn` in `oxide-godot/` removed before the commit), `door.tscn` + a `Player`
entering the area → animation plays once; `door.tscn` headless with no error.
