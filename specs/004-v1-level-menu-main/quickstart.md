# Quickstart: validation of each port (Milestone D)

Execution/validation guide — what to run, in order, and what to expect. Code details in
[research.md](research.md); names to check in [contracts/](contracts/). Paths relative to the
repository root (`oxide-godot/`, where this `specs/` lives).

## Prerequisites

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable (there is no `godot` on PATH).
- `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5. **In this milestone `Cargo.toml` changes once** (port 3:
  `features = ["experimental-threads"]`, research D1) — other than that, do not edit it.
- **No Godot editor open** on the project during headless validation (`pgrep -a godot` empty).
  Warn the user before starting; never kill their process.
- Clean tree on `main` (`git status --short` empty) before each port.

## Baseline (measured on 2026-09-16, commit `a866428`, before any port of this milestone)

- `cargo build`: **0 warnings** (without the feature).
- Headless import: line 1 = `Initialize godot-rust (…)`; **0 `ERROR` lines**. The 3 upstream errors
  cataloged in `CLAUDE.md` only appear on a clean import and do not count.
- `level/forklift/flying_forklift.tscn`: exit 124, 0 `ERROR`, 1 WARNING (HDR).
- `level/level.tscn`: exit 124, 0 `ERROR`, 2 WARNINGs (HDR, Physics interpolation).
- `menu/menu.tscn`: exit 124, 0 `ERROR`, 1 WARNING (HDR). In headless the menu **hosts by itself**
  (deferred `_on_host_pressed`) and loads the level in a thread; without the `main`, the signal
  `replace_main_scene` is emitted and nothing consumes it.
- `main/main.tscn`: exit 124, 2 WARNINGs. Full flow boot → menu → automatic host → level.
  **Intermittent engine error** (1 in 6 runs, the first after `--import`): 5 `ERROR` lines
  from the dummy renderer — `Initializing already initialized RID`, `Parameter "mem" is null.`,
  3× `Parameter "m" is null.` (`rid_owner.h`, `dummy/storage/mesh_storage.h`). It comes from the race
  between the level loading in a sub-thread and the dummy renderer; the untouched original has the
  same architecture. **Rule**: if those exact 5 lines appear, run again — they are not a
  regression if they disappear; any other `ERROR` line, or those same ones in 3 consecutive runs, is a
  regression. Catalog in `CLAUDE.md` (operational edit) in the port 3 commit together with the
  feature line.
- `docs/upstream-bugs.md`: 2 entries (does not change); `docs/v2-backlog.md`: 18 items.

Any `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` or **`shadows a native class`** line not in this baseline is a regression.

## Cycle per script (repeat 4 times, in order 1 → 4)

### 1. Build

```bash
cd oxide_godot_core && cargo build 2>&1 | tail -20
cargo build 2>&1 | grep -c '^warning'      # expected: 0
```

In port 3, the first build after enabling the feature regenerates the `godot-core` bindings (a few
minutes the first time; the directory `target/debug/build/godot-core-*/out` gains a second
hash). After that, `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` is
the valid directory for looking up signatures.

### 2. Headless import (did the extension load?)

```bash
cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log
grep -n 'Initialize godot-rust' /tmp/import.log        # must exist (line 1)
grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log            # expected: empty (or only the 3 upstream ones)
```

### 3. Headless run of the affected scene(s)

```bash
cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . <scene>.tscn 2>&1 | tee /tmp/run.log
grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/run.log   # expected: empty
grep -c '^WARNING' /tmp/run.log                                                                                              # only the benign ones from the baseline
```

| Port | `<scene>` to run | What it exercises |
|---|---|---|
| 1 `flying_forklift.gd` | `level/forklift/flying_forklift.tscn` **and** `level/level.tscn` | the forklift's `ready` (dynamic Settings, model pick) in isolation and in the level's instances |
| 2 `level.gd` | `level/level.tscn` **and** `main/main.tscn` | Level in isolation: dynamic Settings, GI, typed spawn of 4 `EnemyRobot` and of `Player` 1, `peer_*` connections. `main.tscn` (still `main.gd`): menu hosts → `replace_main_scene` → `main.gd` instantiates the Rust `Level` and connects `quit` via `has_signal` |
| 3 `menu.gd` | `menu/menu.tscn` **and** `main/main.tscn` | Menu in isolation in headless: `ready` (85 `OnReady`, 15 `ButtonGroup`), automatic host, threaded loading (feature), `DoneTimer`, emission of `replace_main_scene`. `main.tscn` (still `main.gd`): full flow with the Rust `Menu` |
| 4 `main.gd` | `main/main.tscn` | Rust boot → `Menu` → host → `Level`; `has_signal`/`connect` by name resolve on the Rust classes |

Exit 124 (timeout) is expected; the validation is the absence of new lines in the `grep`. For
`main.tscn`, apply the baseline's intermittent-error rule.

### 4. Mechanical checks of the cycle (Principle II)

```bash
ls oxide-godot/<path>/<script>.gd oxide-godot/<path>/<script>.gd.uid   # expected: No such file
grep -rn '<uid of the .gd>' oxide-godot/ | grep -v '/.godot/'                     # expected: empty
grep -rn '<script>.gd' oxide-godot/ --include='*.tscn' --include='*.gd'      # expected: empty
grep -c 'type="<Class>"' oxide-godot/<scene>.tscn                             # 1
grep -c 'ExtResource("<id>")' oxide-godot/<scene>.tscn                         # 0  (ids: forklift "3"; level/menu/main "1")
git diff --stat HEAD -- 'oxide-godot/**/*.gd'   # only the deletion of this port's script; settings.gd never appears
```

Uids, ids and lines per port: "Scene editing" table of [plan.md](plan.md).

### 5. Name check (all ports)

Run the "Verification before the commit" section of the port's contract
([contracts/flying-forklift.md](contracts/flying-forklift.md), [level.md](contracts/level.md),
[menu.md](contracts/menu.md), [main.md](contracts/main.md)).

In port 2, check that `player.rs` and `red_robot.rs` only changed in visibility:

```bash
git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep '^[-+]' | grep -v '^[-+][-+]'
# expected: exactly 4 lines — "-    fn set_player_id(" / "+    pub(crate) fn set_player_id(" and "-    fn exploded();" / "+    pub(crate) fn exploded();"
```

In port 3, check `Cargo.toml` and `CLAUDE.md`:

```bash
git diff HEAD -- oxide_godot_core/Cargo.toml | grep '^[-+]godot'   # 1 pair: features = ["experimental-threads"]
git diff --stat HEAD -- CLAUDE.md                                   # only added lines (feature + intermittent error)
```

### 6. Visual validation (user, in the editor/game)

Open the project in the editor, run (F5) and check the port's item in the "Visual validation per
script" table of [plan.md](plan.md), comparing with `../oxide_godot_origins/`. When opening the edited scene:
root with the Rust type, no script; `menu.tscn` with the 10 connections in the signals panel; `main.tscn`
with the node still named `main`.

### 7. v2 backlog

Add to `docs/v2-backlog.md` the candidates from research.md §"v2 backlog candidates" for the script
(port 1: 19; port 2: 20–21; port 3: 22–23; port 4: 24), **before** the commit. Numbering continues
from 19.

### 8. Commit

One commit per script, on `main`, including: new Rust module + `lib.rs` (+ port 2: `player.rs`,
`red_robot.rs` visibility only; + port 3: `Cargo.toml`, `CLAUDE.md`), edited `.tscn`, `.gd` +
`.gd.uid` deleted, `docs/v2-backlog.md`.

```
Port flying_forklift.gd → FlyingForklift (CharacterBody3D); flying_forklift.tscn: node FlyingForklift type="CharacterBody3D"→"FlyingForklift"

- base CharacterBody3D = the node's type (script extends Node3D — constitution v1.3.1, Principle II)
- <notes>; v2 backlog: item 19
```

```
Port level.gd → Level (Node3D); level.tscn: node Level type="Node3D"→"Level"

- <notes: quit signal; EnemyRobot/Player typed; dynamic Settings; add_player/del_player via closure>
- player.rs / red_robot.rs: only pub(crate) visibility on set_player_id / exploded (typed access, FR-025); no logic moved
- v2 backlog: items 20, 21
```

```
Port menu.gd → Menu (Node); menu.tscn: node Menu type="Node"→"Menu"

- <notes: replace_main_scene(PackedScene) signal; 85 OnReady; 9 handlers; threaded loading>
- Cargo.toml: gdext experimental-threads feature (ResourceLoader::load_threaded_* is not generated without it — godot-codegen special_cases.rs:83-86); version 0.5.5 unchanged
- CLAUDE.md: feature line + catalog of the intermittent dummy-renderer error in headless main.tscn
- v2 backlog: items 22, 23
```

```
Port main.gd → Main (Node); main.tscn: node main type="Node"→"Main"

- <notes: has_signal/connect by name preserved; call_deferred by name; node still named "main">
- v2 backlog: item 24
```

## Final verification of the milestone (after the 4th commit)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*'                  # only oxide-godot/menu/settings.gd
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l      # 1
git diff --stat a866428 -- 'oxide-godot/**/*.gd'                       # exactly 4 deletions (flying_forklift, level, menu, main); settings.gd absent
git log --oneline a866428..HEAD | grep -c '^[0-9a-f]* Port '           # 4
ls oxide_godot_core/oxide_godot_lib/src/                               # lib.rs + 14 modules (… flying_forklift, level, menu, main_scene)
grep -n '^godot' oxide_godot_core/Cargo.toml                           # godot = { version = "0.5.5", features = ["experimental-threads"] }
grep -n 'experimental-threads' CLAUDE.md                               # 1+ lines
grep -c '^| [0-9]' docs/upstream-bugs.md                               # 2 (unchanged)
grep -c '^| [0-9]' docs/v2-backlog.md                                  # 24
# FR-025: dynamic calls only the allowed ones
grep -nE '\.call\(' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs   # only "apply_graphics_settings" and "save_settings" (level.rs, menu.rs)
grep -nE 'call_deferred\(' oxide_godot_core/oxide_godot_lib/src/{menu,main_scene}.rs                   # only "_on_host_pressed" (menu) and "change_scene_to_packed" (main_scene)
grep -nE '\.get\("' oxide_godot_core/oxide_godot_lib/src/{flying_forklift,level,menu,main_scene}.rs   # only "config_file"
grep -nE 'has_signal|from_object_method' oxide_godot_core/oxide_godot_lib/src/main_scene.rs           # "quit" and "replace_main_scene" — the original's duck typing
```

And, in the game (SC-002): boot → menu → Play (loading bar) → playable level (player, robots with
15 s respawn, forklifts with varied models, GI according to the option) → ESC returns to the menu →
Settings: apply, cancel, reopen, check `user://settings.ini` → Quit exits — identical to the
original.
