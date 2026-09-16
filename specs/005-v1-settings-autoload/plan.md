# Implementation Plan: Milestone E — Settings autoload (v1 raw port, last script)

**Branch**: `main` (v1 lives on `main`; the port is an atomic commit that leaves the game playable) | **Date**: 2026-09-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-v1-settings-autoload/spec.md`

**Phase**: v1 — Raw Port (Principle I, constitution v1.3.1). Direct translation; no abstraction,
refactoring or optimization. **One** conservative upstream bug fix (SSAO disabled,
spec FR-020–FR-024) inside the milestone's single commit — `docs/upstream-bugs.md` goes to 3 entries.

## Summary

Port `menu/settings.gd` (107 l., autoload `Settings`, `extends Node`) to `Settings: Node` in
`oxide_godot_core/oxide_godot_lib/src/settings.rs`, completing v1 with **zero `.gd`** in the project.
Special autoload binding: a native class cannot be a direct autoload, so the equivalent
of the "`type` swap" is a new minimal scene `oxide-godot/menu/settings.tscn` (one node,
`name="Settings" type="Settings"`) pointed to by `project.godot:25`
(`Settings="*res://menu/settings.gd"` → `Settings="*res://menu/settings.tscn"`) — first and
only edit of `project.godot` in v1. Technical approach: manual `fn init` (the order of the `var`s in the
original: `metalfx_supported` → `DEFAULTS` → `config_file`), `DEFAULTS` as a nested `VarDictionary`
(`vdict!`) iterated in `load_settings` (insertion order preserved → identical `.ini`);
`#[var] config_file: Gd<ConfigFile>` so the 10 `get("config_file")` accesses of the 5 Rust consumers
remain valid **without editing**; `#[func] load_settings/save_settings/apply_graphics_settings`
(3 arguments, target of `call` by name); `GIType`/`GIQuality` enums as 6 `#[constant]`; SSAO
with `else if` + comment `// upstream bug fix`. The whole mapping compiled in a draft (0
warnings) and was exercised headless with the autoload **in a scene** pointing to the draft
class: `main.tscn`/`menu.tscn`/`level.tscn` without error, 15 defaults identical (value **and** type)
to those of `settings.gd`, `.ini` rewritten byte-for-byte identical, SSAO bug reproduced in the original and
fixed in the draft — [research.md](research.md) §E.

## Technical Context

**Language/Version**: Rust 1.98.1 (edition 2024); crate `godot` 0.5.5 (godot-rust/gdext) in
`[workspace.dependencies]` with `features = ["experimental-threads"]` (since Milestone D). **No
crate change in this milestone.**

**Primary Dependencies**: gdext 0.5.5 (prebuilt API 4.6 — keep); Godot 4.7.2 stable at
`/usr/bin/godot.x86_64`. `.gdextension` with `reloadable = true`, debug lib at
`oxide_godot_core/target/debug/liboxide_godot.so`. Bindings in
`oxide_godot_core/target/debug/build/godot-core-4eba5d49e15a0d7e/out/` (`ls -dt … | head -1`).

**Storage**: `user://settings.ini` via `ConfigFile` — on this machine
`~/.local/share/godot/app_userdata/Third-Person Shooter Demo/settings.ini` (`config/name` of
`project.godot`). Created **only** by `save_settings` (Apply); `load_settings` does not write.

**Testing**: `cargo build` (debug profile, 0 warnings) + Godot headless (import + `main.tscn`,
`menu.tscn`, `level.tscn`; **never** `settings.tscn` in isolation). Disposable `-s` probes outside the
repo (research §E) for SC-003 and FR-024. Visual validation by the user (full Settings, F11,
persistence across runs).

**Target Platform**: Linux x86_64 desktop (Vulkan driver → `metalfx_supported = false`)

**Project Type**: GDExtension cdylib (`oxide_godot_core/oxide_godot_lib`) + Godot project (`oxide-godot/`)

**Performance Goals**: parity with the original (do not optimize — Principle I)

**Constraints**: Principles I and II of constitution v1.3.1; names `config_file`, `load_settings`,
`save_settings`, `apply_graphics_settings` identical; the 5 consumer modules byte-for-byte
intact; a single commit; `.gd` + `.gd.uid` in the same commit; `project.godot` changes in exactly 1
line; improvements only in `docs/v2-backlog.md`; **single** SSAO fix with the 4 requirements of the
clause; `CLAUDE.md` untouched (decision FR-019 below).

**Scale/Scope**: 1 script, 107 lines of GDScript, 1 new scene (3 lines), 1 line of
`project.godot`, 1 commit, 0 existing Rust files touched besides `lib.rs` (+1 line).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I — Three-Phase Port

| Rule | Status | Evidence |
|---|---|---|
| Phase declared in spec/plan/tasks | ✅ | spec.md "Phase: v1 (constitution v1.3.1)"; this plan "Phase: v1" |
| Direct translation, without remodeling nodes/scenes | ✅ | No existing scene changes; the new scene has only the autoload node; `project.godot` 1 line |
| No abstraction/refactoring/optimization | ✅ | One module; `DEFAULTS` remains an iterated nested dictionary (does not become a list/struct/helper); `apply_graphics_settings` line by line with the repeated reads of `config_file` (no "cleanup" local variables beyond those required by the type); quirks preserved (research D9): **own** `get_window()` for the window mode, "medium applies high" in SSAO, shadows `FIXME` (does not re-enable), insertion order of the defaults, `metalfx_supported` evaluated at construction |
| Improvements → `docs/v2-backlog.md` in the same commit | ✅ | Item 25 (SSAO quirk 2) in research §"Candidate v2 backlog"; nothing else identified |
| Bug fixes — 4 requirements | ✅ | SSAO disabled: (a) spec FR-020–FR-024; (b) `// upstream bug fix: …` on the line immediately above the `else if` in `settings.rs` (research D8); (c) commit message (quickstart §7); (d) `docs/upstream-bugs.md` entry #3 in the same commit. Defect **reproduced** in the original headless (§E.4: `ssao_quality = -1` → `ssao_enabled = true`); the fix is the `if` → `else if` swap, nothing more |

### Principle II — Verifiable Port Cycle (v1.3.1)

| Rule | Status | Evidence |
|---|---|---|
| One class per script, same base | ✅ | `Settings: Node` (`extends Node`; the autoload has no node in a scene — the base is the script's) |
| Class name without collision | ✅ | `ls out/classes/ \| grep -i settings` → only `editor_settings`, `label_settings`, `project_settings`, `mesh_convex_decomposition_settings`, `open_xr_android_thread_settings_extension`; `CLAUDE.md` grep over the `.gd`s: no `Settings` identifier (the only `.gd` is itself, deleted in the commit) |
| Binding by `type` swap; no bridge `.gd` | ✅ (adapted) | Autoload = script **or** scene; a native class cannot be a direct autoload. Exact equivalent: new 1-node scene with `type="Settings"` + `project.godot:25`. Declared in the spec (§"Binding by type of an autoload"); recorded in Complexity Tracking as a non-violation. **Confirmed headless** (§E.2): `/root/Settings` is the native class, `name` comes from the autoload key |
| Identical `#[func]`/property names | ✅ | `config_file` (`#[var]`), `load_settings`, `save_settings`, `apply_graphics_settings(window, environment, scene_root)` — [contracts/settings.md](contracts/settings.md); probe §E.3 (`has_method` ×3, `get("config_file")` returns the same `ConfigFile`) |
| Exported/replicated properties | ✅ | `settings.gd` neither exports nor replicates; `config_file` and `metalfx_supported` become `#[var]` (usage `NONE`, no inspector) — names preserved |
| `cargo build` without new warnings | ✅ | Baseline 0; draft 0 (after `#[var]` on `metalfx_supported` — research D3) |
| Headless validation (import + scene) | ✅ | quickstart §2–4; baseline measured at `6f4d9ba` (§E.1); `settings.tscn` is **not** run in isolation (it would instantiate two `Settings`) |
| Commit with script + binding in the message | ✅ | quickstart §7: cites `settings.tscn`, `project.godot`, the SSAO fix and the end of v1 |
| `.gd` + `.gd.uid` deleted in the same commit | ✅ | `git rm menu/settings.gd menu/settings.gd.uid`; grep of the uid `b04fekxdgdq0k` empty; staging checked before any commit |
| Bottom-up order | ✅ | `docs/port-order.md` item 15 — last; all consumers are already Rust |
| Rust does not call custom GDScript API | ✅ | No GDScript remains. `propagate_call_ex("set")` is base engine API (the original does the same). The 5 consumers keep the dynamic access to `/root/Settings` (constitution exception l.79; backlog item 1 — out of scope, spec) |
| `Settings` exception | ✅ (closes) | The class is born in Rust; the exception ceases to exist for new code. The existing dynamic accesses are left for v2 |
| `CLAUDE.md` catalog reflects the baseline | ✅ | Unchanged: no error eliminated nor new. **FR-019: decision = do not edit `CLAUDE.md`** — the autoload binding by scene is declared in the spec and in the plan; `CLAUDE.md` describes the general cycle and there will be no other autoload in v1 |

**Justified non-violations** (Complexity Tracking): autoload binding by new scene +
`project.godot`; `#[var]` on `metalfx_supported`; manual `fn init`.

**Gate result (pre-Phase 0)**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/005-v1-settings-autoload/
├── plan.md              # This file
├── spec.md              # Specification (commit 0e27c68)
├── research.md          # Phase 0: signatures by compilation, headless probes (autoload in scene, defaults, .ini, SSAO), baseline
├── data-model.md        # Phase 1: Settings class, section/key/type/default table of config_file, flows
├── quickstart.md        # Phase 1: baseline, cycle, SC-003/FR-024 probes, final v1 verification, commit format
├── contracts/
│   └── settings.md      # config_file, load_settings, save_settings, apply_graphics_settings(3), 6 constants; 10 consumer sites
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks — not created by this command)
```

### Source Code (repository root)

```text
oxide_godot_core/
└── oxide_godot_lib/src/
    ├── lib.rs                          # +1 line `mod settings;` (after `mod main_scene;`)
    ├── settings.rs                     # struct Settings, base=Node  NEW
    ├── bullet.rs, flying_forklift.rs,  # UNTOUCHED (dynamic consumers of /root/Settings)
    ├── level.rs, menu.rs, main_scene.rs
    └── (remaining 9 modules)           # UNTOUCHED

oxide-godot/
├── menu/settings.tscn                  # NEW — 3 lines: `[gd_scene format=3]`, empty, `[node name="Settings" type="Settings"]`
├── menu/settings.gd (+ .uid)           # DELETE (git rm)
└── project.godot                       # l.25: Settings="*res://menu/settings.gd" → Settings="*res://menu/settings.tscn"

docs/upstream-bugs.md                   # entry #3 (SSAO disabled)
docs/v2-backlog.md                      # item 25 (quirk "medium applies high")
CLAUDE.md                               # UNTOUCHED (FR-019: decision not to edit)
```

**Structure Decision**: one module per script (Principle I); `settings.rs` / `mod settings;` — no
conflict with anything in the crate. The class is called `Settings`; the node at `/root/Settings` keeps the name
from the autoload key. Details in [research.md](research.md) D1–D10.

## Binding edit (checked on 2026-09-16 — re-check with `grep -n` before editing)

| File | Action | Before | After |
|---|---|---|---|
| `oxide-godot/menu/settings.tscn` | **create** (3 lines, no `uid=` in the header — the import neither generates a `.uid` for `.tscn` nor complains; §E.2) | — | `[gd_scene format=3]` / empty line / `[node name="Settings" type="Settings"]` |
| `oxide-godot/project.godot` l.25 | 1 line (`grep -n '^Settings=' project.godot` → 25) | `Settings="*res://menu/settings.gd"` | `Settings="*res://menu/settings.tscn"` |
| `oxide-godot/menu/settings.gd` + `settings.gd.uid` (`uid://b04fekxdgdq0k`) | `git rm` | — | — ; `grep -rn 'b04fekxdgdq0k\|menu/settings.gd' oxide-godot/ \| grep -v /.godot/` empty |

Nothing else in `project.godot` changes (`git diff --stat -- oxide-godot/project.godot` = `2 +-`); the
`toggle_fullscreen` action (l.172–177) and `run/main_scene` (l.15) stay as they are.

## Visual validation (SC-002 — done by the user in the game, comparing with `../oxide_godot_origins/`)

| What to check |
|---|
| Boot → menu with the saved window mode; Settings shows the 15 rows with the values of the current `settings.ini` (the file written by the original `settings.gd` is read without change) |
| Change options → Apply: immediate effect (window mode, vsync, fps, scale/filter, TAA/MSAA/AA, bloom, fog) and persistence (reopen Settings; restart the game); Cancel discards |
| Delete `user://settings.ini` → boot with the defaults (exclusive fullscreen, vsync, FSR2, VoxelGI low, SSAO medium, SSIL off, shadows/bloom/fog on); the file only reappears after an Apply |
| Level: `Shadow mapping` off → no light casts a shadow; **SSAO: Disabled → SSAO really disabled** (in the original it stays on — the fix); SSAO Medium/High and SSIL Disabled/Medium/High as in the original |
| F11 and Alt+Enter toggle exclusive fullscreen ↔ windowed on any screen (menu and level) |
| Editor: `menu/settings.tscn` with the root `Settings` of type `Settings`, no script; Project Settings → Autoload: `Settings` → `res://menu/settings.tscn`, singleton enabled; no `.gd` in the FileSystem |

## Complexity Tracking

> Filled in to record **justified non-violations** (the gate has no violations).

| Item | Why it is necessary | Simpler alternative rejected because |
|---|---|---|
| Autoload binding by **new scene** (`menu/settings.tscn`, 1 node) + **1-line** edit of `project.godot` | Godot only accepts a script or a scene as autoload; a native class registered by GDExtension cannot be a direct autoload. The 1-node scene with `type="Settings"` is literally the "`type` swap" of Principle II for a node that did not exist in any scene; `project.godot` is where the autoload binding lives. It is the first and only edit of `project.godot` in v1, declared in the spec | Keep a bridge `settings.gd` (`extends Settings`) — forbidden by Principle II (no bridge `.gd`) and it would leave 1 `.gd`; register the singleton via `Engine.register_singleton` in the `ExtensionLibrary` — changes the architecture (it stops being a node at `/root/Settings`, breaks the 5 consumers' `get_node("/root/Settings")` and `_input`) |
| `#[var]` on `metalfx_supported` (besides `config_file`) | The field is only read in `init` (default of `scale_filter`); without `#[var]` the compiler emits `field is never read` (warning — forbidden). In the original it is a top-level `var` = script property; `#[var]` (usage `NONE`) reproduces that without inspector | Leave the value as a local variable of `init` — the original's field would disappear; `#[allow(dead_code)]` — warning suppression instead of fidelity |
| Manual `fn init(base)` instead of `#[class(init)]` + `#[init(val = …)]` | The defaults depend on `metalfx_supported` (`scale_filter`), and `#[init(val)]` cannot reference another field. The manual `init` reproduces the order of the `var`s in the original (`metalfx_supported` → `DEFAULTS` → `config_file`) | Recompute the driver name inside the `vdict!` — would duplicate the `RenderingServer` read, which the original does once |

## Constitution Check — post-design re-evaluation (Phase 1)

Re-evaluated after research.md, data-model.md, contracts/ and quickstart.md:

- No artifact introduces a common module, trait or helper; `DEFAULTS` remains an iterated nested
  dictionary; `apply_graphics_settings` keeps the repeated reads and the order of the original;
  SSIL intact; "medium applies high" quirk intact (backlog 25). ✅
- The SSAO fix is the smallest possible (`if` → `else if`), isolated by the comment
  `// upstream bug fix`, declared in the spec, with entry #3 prepared for `docs/upstream-bugs.md`
  and cited in the commit message (quickstart §7). The defect was **reproduced** in the original
  (§E.4) and the draft returns `ssao_enabled = false` for −1. ✅
- The contract reproduces the names checked at the 10 consumer sites (`get("config_file")` ×8,
  `call("apply_graphics_settings", 3 args)` ×3, `call("save_settings")` ×1) — all resolved
  headless against the draft class (full `main.tscn`). ✅
- The 15 defaults match `settings.gd` in value **and** type (§E.3, programmatic comparison), the
  section/key order is the same and the rewritten `.ini` is byte-for-byte identical (SC-003). ✅
- `Scaling3DMode::from_ord(5)` (Nearest from 4.7, absent from the 4.6 API) does not panic — the 4.6 API
  accepts 0..=5 (5 = `MAX`) and passes the ordinal on to the engine (§E.3). ✅
- `CLAUDE.md` untouched (FR-019 decided); `project.godot` 1 line; no existing scene
  edited. ✅

**Gate result (post-Phase 1)**: PASS.
