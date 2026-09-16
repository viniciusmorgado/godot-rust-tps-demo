# Quickstart: validation of the Settings autoload port (Milestone E — end of v1)

Execution/validation guide — what to run, in order, and what to expect. Code details in
[research.md](research.md); names to check in [contracts/settings.md](contracts/settings.md).
Paths relative to the repository root.

## Prerequisites

- `/usr/bin/godot.x86_64` = Godot 4.7.2 stable. `cargo`/`rustc` 1.98.x; crate `godot` 0.5.5 with
  `experimental-threads` (do not edit `Cargo.toml`).
- **No Godot editor open** during headless validation (`pgrep -a godot` empty) — warn the
  user beforehand; never kill their process.
- Clean tree on `main` (`git status --short` empty); HEAD at `619575d` or later without `Port `.
- `INI="$HOME/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini"` — real
  path of `user://settings.ini` (`config/name` of `project.godot`). **Before anything else**:
  `cp "$INI" <scratch>/settings.ini.baseline` (written by the original `settings.gd`; md5 on
  2026-09-16: `99dac170ad99078f46d2428e650516e7`).

## 1. Baseline (measured on 2026-09-16 — research §E.1)

- `cargo build`: 0 warnings. Import: `Initialize godot-rust`, 0 new `ERROR` (the 3 from `CLAUDE.md`
  do not count).
- `main/main.tscn` exit 124, 0 regressions, 2 WARNINGs (HDR, Physics interpolation);
  `menu/menu.tscn` 124/0/1; `level/level.tscn` 124/0/2. Rule of the **intermittent error** of
  `main.tscn` (`CLAUDE.md`): the 5 lines of the dummy renderer do not count if they disappear on the 2nd
  run; 3 in a row = regression.
- `docs/upstream-bugs.md` 2 entries; `docs/v2-backlog.md` 24 items; 1 `.gd` + 1 `.gd.uid`.

Regression = any `ERROR`, `SCRIPT ERROR`, `Invalid call`, `Invalid get`, `Invalid set`,
`Nonexistent`, `panicked` or `shadows a native class` outside the baseline.

## 2. Port cycle (once)

1. Check the name: `grep -rhoE '^(const|class_name|var|@onready var|@export var) [A-Za-z_]+' oxide-godot --include='*.gd' | sort -u | grep -w Settings` empty;
   `ls $(ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1)/classes/ | grep -ix 'settings.rs'` empty.
2. Create `oxide_godot_core/oxide_godot_lib/src/settings.rs` (research D3–D8; draft compiled in
   `scratchpad/settings_draft.rs` with `ZzSettings` → rename to `Settings`); `mod settings;` in
   `lib.rs` after `mod main_scene;`.
3. `cd oxide_godot_core && cargo build 2>&1 | tail -3` → `Finished`; `grep -c '^warning'` = 0.
4. Create `oxide-godot/menu/settings.tscn` (3 lines — plan §"Binding edit").
5. `project.godot`: `grep -n '^Settings=' oxide-godot/project.godot` → l.25; `sed -i '25s|settings\.gd"|settings.tscn"|'`;
   `git diff --stat -- oxide-godot/project.godot` = `2 +-`.
6. `git rm oxide-godot/menu/settings.gd oxide-godot/menu/settings.gd.uid`;
   `grep -rn 'b04fekxdgdq0k\|menu/settings.gd' oxide-godot/ | grep -v /.godot/` empty.
7. Headless (§3). 8. Contract (§4) + probes (§5). 9. `docs/upstream-bugs.md` #3 and `docs/v2-backlog.md` 25
   (texts in research §"Entry #3" and §"Candidate v2 backlog"). 10. Commit (§7). 11. User checkpoint (§6).

## 3. Headless validation (from `oxide-godot/oxide-godot/`)

```bash
/usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log | grep -c 'Initialize godot-rust'   # 1
grep -E 'ERROR|SCRIPT ERROR' /tmp/import.log | grep -vE 'Cannon_Charge already exists|doorsimple_d.png|surfaces.is_empty'   # empty
ls menu/                                                                     # settings.tscn present; NO .gd/.uid; no settings.tscn.uid expected
for s in main/main.tscn menu/menu.tscn level/level.tscn; do
  timeout 20 /usr/bin/godot.x86_64 --headless --path . $s > /tmp/run.log 2>&1; echo "$s exit=$?"   # 124
  grep -E 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked|shadows a native class' /tmp/run.log   # empty (intermittent rule in main)
done
md5sum "$INI"                                                                # equal to the baseline: boot does not write the .ini
```

**Never** run `menu/settings.tscn` in isolation (it would instantiate the autoload + the scene: two `Settings`).
`main.tscn` exercises the whole flow with the autoload in Rust: `Settings.ready` → `load_settings`;
`Main.ready` reads `display_mode`; `Menu.ready` calls `apply_graphics_settings` (3 args) and the automatic
host; `Level.ready` calls `apply_graphics_settings` and reads `gi_type`/`gi_quality`;
`FlyingForklift.ready` reads `shadow_mapping`.

## 4. Contract and mechanics

"Verification before the commit" block of [contracts/settings.md](contracts/settings.md) — all
expected results listed there. Plus:

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l                # 0
find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l            # 0
ls oxide_godot_core/oxide_godot_lib/src/ | wc -l                             # 16 (lib.rs + 15 modules)
git diff --stat HEAD -- oxide-godot/project.godot                            # 1 file, 1 insertion(+), 1 deletion(-)
git diff HEAD -- oxide-godot/project.godot | grep '^[-+]Settings='           # -…settings.gd" / +…settings.tscn"
git status --short | grep -v 'settings\|lib.rs\|project.godot\|upstream-bugs\|v2-backlog'   # empty (nothing outside the list)
grep -c '^| [0-9]' docs/upstream-bugs.md                                     # 3
grep -c '^| [0-9]' docs/v2-backlog.md                                        # 25
git diff --quiet HEAD -- CLAUDE.md && echo unchanged                         # unchanged
```

## 5. Disposable probes (`-s`, outside the repo — scratchpad; research §E.3/E.3b/E.4 have the scripts)

Run from `oxide-godot/oxide-godot/` with `timeout 30 /usr/bin/godot.x86_64 --headless --path . -s <scratch>/probe.gd`:

- **SC-003 (`.ini` idempotence)**: probe calls `/root/Settings.save_settings()`; then
  `diff "$INI" <scratch>/settings.ini.baseline` → empty.
- **FR-024 (SSAO disabled)**: probe sets `config_file.set_value("rendering","ssao_quality",-1)`,
  calls `apply_graphics_settings(root, Environment.new(), Node.new())` and prints
  `environment.ssao_enabled` → `false`; with MEDIUM/HIGH → `true`. Restore the value at the end (the
  probe does not save).
- **Defaults (with the `.ini` moved aside and restored — check md5 afterwards)**: probe compares
  `config_file` after `load_settings` with the section/key/type/default table of `data-model.md` (already checked programmatically against the original's `DEFAULTS` in research §E.3b — the `.gd` no longer exists at that point).
  `load("res://…")` does not reach it; temporarily copying to the scratch does not solve `res://` —
  so run this probe **before step 6 of the cycle**, or accept the result already measured in
  research §E.3b, 15/15).
- The probe's `WARNING … leaked at exit` (objects created by the script itself) do not count.

## 6. User checkpoint (visual validation — plan §"Visual validation")

Boot → menu with the saved window mode; Settings reflects the current `.ini`; Apply applies and persists
(reopen; restart); Cancel discards; delete the `.ini` → defaults and the file only comes back after Apply;
`Shadow mapping` off → no shadows; **SSAO Disabled → actually disabled** (intentional difference
from the original: upstream-bugs #3); SSAO Medium/High, SSIL 3 options, bloom, fog as in the
original; F11 and Alt+Enter toggle fullscreen; editor: `settings.tscn` root `Settings` without script,
Autoload pointing to the scene, no `.gd` in the FileSystem. Milestones A–D remain the same.
Divergence → commit `Fix port settings.gd …`.

## 7. Commit (`git status`/`git diff --cached` beforehand; never partial)

Files: `src/settings.rs` (new), `src/lib.rs`, `oxide-godot/menu/settings.tscn` (new),
`oxide-godot/project.godot`, deletions of `menu/settings.gd` + `.uid`, `docs/upstream-bugs.md`,
`docs/v2-backlog.md`. Nothing else (no existing `.rs` besides `lib.rs`; `CLAUDE.md` untouched).

Subject: `Port settings.gd → Settings (Node, autoload); menu/settings.tscn nova; project.godot: Settings="*res://menu/settings.tscn"`

Body: autoload binding by minimal scene (a native class cannot be a direct autoload; only
edit of `project.godot` in v1); `#[var] config_file`/`metalfx_supported`; `DEFAULTS` as an
iterated nested dictionary (`.ini` order preserved); 6 `#[constant]` of the enums; manual
`init`; `apply_graphics_settings` line by line with own `get_window()` and "medium applies
high" quirk preserved; **conservative fix**: SSAO `if`→`else if` (settings.gd:90) with comment
`// upstream bug fix` — `docs/upstream-bugs.md` #3; v2 backlog item 25; **v1: zero `.gd` in the
project**; consumers untouched (backlog 1).

## 8. Final v1 verification (after the commit, before the final OK)

```bash
find oxide-godot -name '*.gd' -not -path '*/addons/*'; find oxide-godot -name '*.gd.uid' -not -path '*/addons/*'   # both empty
git log --oneline 619575d..HEAD | grep -c '^[0-9a-f]* Port '                 # 1
git diff --stat 619575d -- '*.rs'                                            # only lib.rs (+1) and settings.rs (new)
git diff --stat 619575d -- oxide-godot/project.godot                         # 1 +-  (1 insertion, 1 deletion)
grep -n '^Settings=' oxide-godot/project.godot                               # res://menu/settings.tscn
ls oxide_godot_core/oxide_godot_lib/src/ | wc -l                             # 16
grep -c '^| [0-9]' docs/upstream-bugs.md; grep -c '^| [0-9]' docs/v2-backlog.md   # 3; 25
cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'                 # 0
```

With the user's OK at the checkpoint: v1 is **declared complete** (Principle I). Tag and branch
`v2` are separate decisions of the user.
