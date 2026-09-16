# Quickstart: validating Milestone V2-A

All commands assume the repo root is `/home/morgado/Experimento/oxide-godot` and the Rust crate
root is `oxide_godot_core/`. Run everything on branch `v2`, local commits only — never push.

## 1. Gates (after every commit, per constitution Principle III)

```bash
cd oxide_godot_core
cargo build                 # mandatory after every Rust change
cargo clippy --quiet        # must report zero warnings from this milestone on
cargo test                  # must pass; ≥ 8 tests once US1 lands (round-trip, defaults, merge, SSAO/SSIL)
```

## 2. Headless validation (per constitution Principle II / `CLAUDE.md`)

Always from `oxide-godot/oxide-godot/`:

```bash
/usr/bin/godot.x86_64 --headless --import --path .        # confirms "Initialize godot-rust ..."
/usr/bin/godot.x86_64 --headless --path . main.tscn
/usr/bin/godot.x86_64 --headless --path . menu/menu.tscn
/usr/bin/godot.x86_64 --headless --path . level/level.tscn
```

No new errors beyond `CLAUDE.md`'s documented baseline (3 known import errors; the intermittent
`main.tscn` race, which is not a regression if a second run is clean).

## 3. `v1` worktree, for parity only (never built from `v2`, never referenced by its Cargo.toml)

```bash
git worktree add ../oxide-godot-v1 v1
cd ../oxide-godot-v1/oxide_godot_core && cargo build
```

Remove the worktree when the milestone's parity checks are done:
`git worktree remove ../oxide-godot-v1` (from the `v2` checkout).

## 4. Parity harness run + diff

1. Copy `specs/006-v2-typed-settings/contracts/zz_settings_parity.gd` (once turned into a real
   `zz_settings_parity.tscn` by a task) into BOTH the `v1` worktree and this branch's
   `oxide-godot/oxide-godot/` project directory.
2. Run headless on each, with a SEPARATE `XDG_DATA_HOME` per branch — both checkouts share the
   project name, so without it they share `~/.local/share/godot/app_userdata/Third-Person Shooter
   Demo/` and overwrite each other's `settings.ini` and dump:
   ```bash
   # in ../oxide-godot-v1/oxide-godot/
   XDG_DATA_HOME=/tmp/parity-v1 /usr/bin/godot.x86_64 --headless --path . zz_settings_parity.tscn
   # in oxide-godot/oxide-godot/ (this branch)
   XDG_DATA_HOME=/tmp/parity-v2 /usr/bin/godot.x86_64 --headless --path . zz_settings_parity.tscn
   ```
3. Diff the two dumps (and the ini files the cases copy):
   ```bash
   diff <(jq -S . "/tmp/parity-v1/godot/app_userdata/Third-Person Shooter Demo/zz_parity_dump.json") \
        <(jq -S . "/tmp/parity-v2/godot/app_userdata/Third-Person Shooter Demo/zz_parity_dump.json")
   ```
   Expect: `case_a_boot` and `case_b_options` identical; `case_c_malformed` intentionally
   different (documented deviation, not a failure).
4. Delete both `zz_settings_parity.*` files and the `v1` worktree once satisfied — they are
   validation scaffolding, not part of either branch's normal scene set.

## 5. Grep acceptance checks (SC-001, backlog #1)

```bash
cd oxide_godot_core
grep -rn 'get_node_as::<Node>("/root/Settings")\|\.get("config_file")\|\.call("apply_graphics_settings\|\.call("save_settings' oxide_godot_lib/src
```

Expect zero matches after User Story 2's last commit.

## 6. Visual checkpoints (SC-007, user-confirmed, not automatable)

After US1: game boots with defaults, Settings menu shows them, Apply persists across a restart.
After US2: same, plus SDFGI/VoxelGI/LightmapGI each render as before and disabling shadows in the
menu actually disables them in the level. After US3: nothing visually changes.
