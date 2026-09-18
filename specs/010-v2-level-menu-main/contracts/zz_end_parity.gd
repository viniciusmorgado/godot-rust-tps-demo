extends Node
# SKELETON — filled in during /speckit-implement. Throwaway: never committed, copied to a `v1`
# git worktree and run there too (see quickstart.md), deleted from both trees at milestone end.
#
# Case (a) reuses specs/006-v2-typed-settings/contracts/zz_settings_parity.gd UNCHANGED --
# not duplicated here. This script covers cases (b)/(c)/(d) only.
#
# Usage: godot.x86_64 --headless --fixed-fps 60 --path <project> res://zz_end_parity.tscn -- --case=<b|c|d>

const LEVEL_SCENE := preload("res://level/level.tscn")

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var case := "b"
	for a in args:
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	match case:
		"b": await _case_b_gi_plan()
		"c": await _case_c_spawn()
		"d": await _case_d_main_end_to_end()
		_: push_error("unknown --case=%s" % case)
	get_tree().quit()

# --- (b) GI plan per (gi_type, gi_quality) cell ---
# On v2: set Settings' typed graphics()/set_graphics() before instancing level.tscn. On v1:
# write the equivalent config_file.set_value("rendering", "gi_type"/"gi_quality", <wire code>)
# instead -- the two trees configure Settings differently but the DUMP is the same shape.
func _case_b_gi_plan() -> void:
	# TODO(/speckit-implement): for each of the 9 (gi_type, gi_quality) combinations, configure
	# Settings, instance LEVEL_SCENE, await a frame, dump WorldEnvironment.environment
	# .sdfgi_enabled, VoxelGI/ReflectionProbes .visible, whether a LightmapGI child exists and
	# its .visible, free the level, repeat.
	print("CASE_B:", JSON.stringify([]))

# --- (c) spawn dump: robot/player under SpawnedNodes, respawn timing, forklift model ---
func _case_c_spawn() -> void:
	# seed() ONCE here, matching where Main's boot-time randomize() would have already run --
	# this harness instances Level standalone, bypassing Main entirely.
	seed(12345)
	var level := LEVEL_SCENE.instantiate()
	add_child(level)
	await get_tree().process_frame
	# TODO(/speckit-implement): dump SpawnedNodes' children (count, names); kill a robot (rpc
	# "hit" x5 or emit exploded directly) and count frames to the NEXT robot appearing under
	# SpawnedNodes (~900 @60fps for the 15s respawn); find a FlyingForklift instance (if the
	# level scene has one) and dump which model child ended up visible (documented #19
	# exclusion -- v1's own forklift draw is unseeded, compare "exactly one visible", not the
	# index).
	print("CASE_C:", JSON.stringify({}))

# --- (d) main.tscn end-to-end: menu -> play -> level -> quit -> menu ---
func _case_d_main_end_to_end() -> void:
	# TODO(/speckit-implement): this case likely needs to run main/main.tscn directly rather
	# than being driven from this harness node (Main IS the scene root) -- reconsider at
	# implementation time whether this is a separate `godot.x86_64 --headless --path .
	# main/main.tscn --quit-after N` invocation with stdout scraped for the child-type-under-
	# Main at each step, rather than a --case dispatch inside this shared harness script.
	print("CASE_D:", JSON.stringify({}))
