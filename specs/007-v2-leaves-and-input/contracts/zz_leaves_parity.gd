extends Node
# SKELETON — filled in during /speckit-implement. Throwaway: never committed, copied to a `v1`
# git worktree and run there too (see quickstart.md §3-4), deleted from both trees at milestone
# end (matching specs/006-v2-typed-settings's convention). Root node of zz_leaves_parity.tscn.
#
# Usage: `godot.x86_64 --headless --path <project> res://zz_leaves_parity.tscn -- --case=<a|b|c|d>`
# reading `OS.get_cmdline_user_args()` to pick ONE case per process (keeps each case's setup —
# player instancing, RNG seeding — independent, matching V2-A's case-(c) redesign).

const PLAYER_SCENE := preload("res://player/player.tscn")
const PART_DISAPPEAR_SCENE := preload("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn")
const IMPACT_EFFECT_SCENE := preload("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn")

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var case := "a"
	for a in args:
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	match case:
		"a": await _case_a_input_state_machine()
		"b": await _case_b_camera_shake()
		"c": await _case_c_debug_label()
		"d": await _case_d_part_and_blast()
		_: push_error("unknown --case=%s" % case)
	get_tree().quit()

# --- (a) player_input: aim state machine, rotation, jump rpc, shoot raycast, fall-to-black ---
func _case_a_input_state_machine() -> void:
	var player := PLAYER_SCENE.instantiate()
	# offline, authority = 1 — matches how main.tscn's headless auto-host already runs locally
	add_child(player)
	var dump := []

	# scripted press/release sequence — one process_frame between each edge so
	# is_action_just_pressed/released has exactly one frame to be true (edge-triggered, clears
	# at the next frame boundary)
	var steps := [
		# [action, "press"|"release", frames_to_wait_after]
		["aim", "press", 1],     # -> Held, cue Shoot (aiming while held)
		[null, null, 5],         # hold under threshold (5 frames * delta << 0.4s at test framerate)
		["aim", "release", 1],   # released EARLY (<= 0.4s) -> Toggled: aiming STAYS on (v1 toggle rule)
		[null, null, 30],        # toggled, timer keeps counting past 0.4s
		["aim", "press", 1],     # press while toggled -> Held (still aiming, no cue)
		["aim", "release", 1],   # accumulated > 0.4s -> Idle, cue Far (aiming ends)
		["aim", "press", 1],
		[null, null, 30],        # hold well past 0.4s
		["aim", "release", 1],   # released LATE -> Idle (a long hold never toggles)
		["jump", "press", 1],    # .rpc("jump") — dump confirms no crash / replicated state intact
		["shoot", "press", 1],
	]
	for step in steps:
		if step[0] != null:
			if step[1] == "press":
				Input.action_press(step[0])
			else:
				Input.action_release(step[0])
		for i in range(step[2]):
			await get_tree().process_frame
			dump.append({
				"aiming": player.aiming,
				"motion": var_to_str(player.motion),
				"shoot_target": var_to_str(player.shoot_target),
			})

	# fall-to-black: teleport below the fade threshold, dump alpha across the window, then recover
	player.global_position.y = -20.0
	for i in range(10):
		await get_tree().process_frame
		dump.append({"alpha": player.get_node("...ColorRect").modulate.a})  # exact path TBD at implementation
	player.global_position.y = 0.0
	for i in range(10):
		await get_tree().process_frame
		dump.append({"alpha": player.get_node("...ColorRect").modulate.a})

	print("CASE_A:", JSON.stringify(dump))

# --- (b) camera shake: fixed RNG seed, triggered via Player's own #[rpc] entry points ---
func _case_b_camera_shake() -> void:
	seed(12345)  # BEFORE instancing — player_input's CameraNoiseShake.noise_seed reads randi() at ready
	var player := PLAYER_SCENE.instantiate()
	add_child(player)
	var dump := []
	player.rpc("add_camera_shake_trauma", 0.75)  # same entry point Player::hit() uses — unchanged by this milestone
	for i in range(30):
		await get_tree().process_frame
		var cam := player.get_node("...camera_camera")  # exact path TBD at implementation
		dump.append(var_to_str(cam.rotation))
	print("CASE_B:", JSON.stringify(dump))

# --- (c) debug label: force visible, dump composed text (VRAM line stripped by the diff script) ---
func _case_c_debug_label() -> void:
	# instance whatever scene owns DebugLabel today (main_scene.tscn's overlay), force visible
	# bypassing the toggle action, wait one frame, dump `.text`
	await get_tree().process_frame
	print("CASE_C:", "TBD at implementation")

# --- (d) part_disappear / blast: count frames until the instance is queued-free ---
func _case_d_part_and_blast() -> void:
	var part := PART_DISAPPEAR_SCENE.instantiate()
	add_child(part)
	var part_frames := 0
	while is_instance_valid(part):
		await get_tree().process_frame
		part_frames += 1

	var impact := IMPACT_EFFECT_SCENE.instantiate()
	add_child(impact)
	var blast_frames := 0
	while is_instance_valid(impact):
		await get_tree().process_frame
		blast_frames += 1

	print("CASE_D:", JSON.stringify({"part_frames": part_frames, "blast_frames": blast_frames}))
