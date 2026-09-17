extends Node
# SKELETON — filled in during /speckit-implement. Throwaway: never committed, copied to a `v1`
# git worktree and run there too (see quickstart.md), deleted from both trees at milestone end.
#
# Usage: godot.x86_64 --headless --fixed-fps 60 --path <project> res://zz_player_parity.tscn -- --case=<a|b|c>

const PLAYER_SCENE := preload("res://player/player.tscn")
const BULLET_SCENE := preload("res://player/bullet/bullet.tscn")
const ROBOT_SCENE := preload("res://enemies/red_robot/red_robot.tscn")
const DOOR_SCENE := preload("res://door/door.tscn")

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var case := "a"
	for a in args:
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	match case:
		"a": await _case_a_player_trace()
		"b": await _case_b_bullet()
		"c": await _case_c_door()
		_: push_error("unknown --case=%s" % case)
	get_tree().quit()

func _make_floor() -> StaticBody3D:
	var floor_body := StaticBody3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(200, 1, 200)
	var collision := CollisionShape3D.new()
	collision.shape = shape
	floor_body.add_child(collision)
	floor_body.global_position = Vector3(0, -0.5, 0)
	add_child(floor_body)
	return floor_body

# --- (a) player movement/animation/jump/land/shoot trace ---
# Frames 0/1 (backlog #10) and the frames right after the scripted below-(-40) teleport
# (backlog #11) are dumped but MUST be excluded from the equality diff (documented divergences).
func _case_a_player_trace() -> void:
	_make_floor()
	var player := PLAYER_SCENE.instantiate()
	add_child(player)
	await get_tree().process_frame

	var dump := []
	var record := func():
		dump.append({
			"position": var_to_str(player.global_position),
			"velocity": var_to_str(player.velocity),
			"anim": player.current_animation,
		})

	# TBD at implementation: scripted move_*/aim/jump/shoot sequence (Input.action_press/release
	# + process_frame awaits, per V2-B's harness pattern — remember the ~1-frame action-flag
	# latency found there). Then a scripted teleport below -40 to exercise backlog #11's respawn
	# velocity-zeroing (documented divergence, not asserted equal).
	for i in range(120):
		await get_tree().process_frame
		record.call()

	print("CASE_A:", JSON.stringify(dump))

# --- (b) bullet: lifetime/explode vs a wall, vs a robot, and the same-tick expiry+collision case ---
func _case_b_bullet() -> void:
	_make_floor()

	# (b1) bullet vs. wall: count frames until AnimationPlayer's current animation becomes
	# "explode".
	# TBD at implementation: instantiate BULLET_SCENE aimed at a static wall a short, known
	# distance away; dump the frame count.

	# (b2) bullet vs. robot: dump ROBOT_SCENE's `health` (an #[export] i32, readable directly)
	# before and after a bullet hits it.

	# (b3) same-tick expiry + collision (backlog #13, documented divergence): construct a
	# bullet whose remaining lifetime crosses zero on the exact physics tick it collides with
	# the wall; dump whether the explode animation restarted once or twice.

	print("CASE_B:", JSON.stringify({"wall_frames": "TBD", "robot_health_before": "TBD", "robot_health_after": "TBD", "same_tick_explodes": "TBD"}))

# --- (c) door: player opens it, a robot does not ---
func _case_c_door() -> void:
	var door := DOOR_SCENE.instantiate()
	add_child(door)
	await get_tree().process_frame

	var anim_player := door.get_node("DoorModel2/AnimationPlayer")

	# TBD at implementation: move a scripted Player body into the door's Area3D; dump whether
	# anim_player.current_animation == "doorsimple_opening". Then, on a FRESH door instance,
	# move an EnemyRobot body through and dump the same check (expect: not playing) plus confirm
	# no new stderr line appears (capture via a separate process run, not from inside this script).

	print("CASE_C:", JSON.stringify({"player_opens": "TBD", "robot_opens": "TBD"}))
