# SKELETON — not wired into any scene yet; a task in tasks.md turns this into a real,
# runnable harness script (attached to a throwaway `zz_settings_parity.tscn`, root type `Node`).
# Design and rationale: ../research.md, section R10. Run once per branch (v1 worktree, this
# branch), headless, outputs diffed by the task/CI step that drives both runs — this script only
# produces the dump, it does not itself diff across branches.
extends Node

const OUTPUT_PATH := "user://zz_parity_dump.json"

func _ready() -> void:
	var dump := {}
	dump["case_a_boot"] = _case_a_boot_and_first_apply()
	dump["case_b_options"] = _case_b_every_menu_option()
	dump["case_c_malformed"] = _case_c_malformed_value()
	var file := FileAccess.open(OUTPUT_PATH, FileAccess.WRITE)
	file.store_string(JSON.stringify(dump, "\t"))
	get_tree().quit()

## Case (a): delete settings.ini, boot, assert no file; Apply defaults; copy the file.
## TODO: instance main.tscn (or drive Settings/Menu directly), assert
## !FileAccess.file_exists("user://settings.ini") before any Apply, then trigger one Apply with
## no changes and read back the raw bytes of user://settings.ini for the dump.
func _case_a_boot_and_first_apply() -> Dictionary:
	return {}

## Case (b): for each of the 15 rows x each option, press the corresponding button in the
## Settings menu and invoke _on_apply_pressed() — the SAME path a player takes on both branches
## (v2 has no #[var] config_file to poke directly once US2's last commit lands). After each
## Apply, dump: the ini bytes; window mode/vsync/max_fps; viewport scaling scale+mode/taa/msaa/
## screen_space_aa; environment ssao_enabled/ssil_enabled/glow_enabled/volumetric_fog_enabled;
## and has_shadow() for every Light3D found under the level root (see research.md R10 for the
## explicit SSAO/SSIL quality+half_size coverage gap — not dumped here, covered by plan() unit
## tests instead).
## TODO: enumerate the Settings menu's option buttons by group (mirrors menu.rs's #[init(node=..)]
## list) and drive each one + Apply; collect the observable-state dictionary per option.
func _case_b_every_menu_option() -> Dictionary:
	return {}

## Case (c): hand-edit `gi_type = 7` into user://settings.ini, boot, and record whether a warning
## was logged and what GI setup actually took effect. Run this branch-side only on `v2` — on `v1`
## it is a known (undiffed) difference, not a parity assertion.
## TODO: write the malformed key directly to the ConfigFile on disk before boot; capture stdout/
## stderr for the godot_warn! line; record the effective GiType applied.
func _case_c_malformed_value() -> Dictionary:
	return {}
