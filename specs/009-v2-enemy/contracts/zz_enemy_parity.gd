extends Node
# SKELETON — filled in during /speckit-implement. Throwaway: never committed, copied to a `v1`
# git worktree and run there too (see quickstart.md), deleted from both trees at milestone end.
#
# Usage: godot.x86_64 --headless --fixed-fps 60 --path <project> res://zz_enemy_parity.tscn -- --case=<a|b>

const ROBOT_SCENE := preload("res://enemies/red_robot/red_robot.tscn")
const PLAYER_SCENE := preload("res://player/player.tscn")

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var case := "a"
	for a in args:
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	match case:
		"a": await _case_a_robot_and_player()
		"b": await _case_b_hit_to_death()
		_: push_error("unknown --case=%s" % case)
	get_tree().quit()

func _make_floor() -> StaticBody3D:
	var floor_body := StaticBody3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(200, 1, 200)
	var collision := CollisionShape3D.new()
	collision.shape = shape
	floor_body.add_child(collision)
	add_child(floor_body)
	floor_body.global_position = Vector3(0, -0.5, 0)
	return floor_body

# --- (a) robot approach/aim/shoot/laser-hit trace against a scripted Player ---
# PlayerDetectionArea (collision_layer=2, mask=2) DOES detect player.tscn (collision_layer=6)
# natively -- research.md R8 -- no harness-side mask override needed here (unlike V2-C's door).
# Dump per frame: state, target_position, aim_preparing, parameters/aim/blend_position, the
# frame ShootAnimation's current_animation becomes "shoot", and the player's CameraNoiseShake
# rotation (the only observable proxy for add_camera_shake_trauma actually firing).
func _case_a_robot_and_player() -> void:
	_make_floor()
	var robot := ROBOT_SCENE.instantiate()
	add_child(robot)
	var player := PLAYER_SCENE.instantiate()
	add_child(player)
	# TODO(/speckit-implement): position player inside PlayerDetectionArea's shape, await frames,
	# dump the trace, then move the player back out and dump the Idle transition too.
	print("CASE_A:", JSON.stringify([]))

# --- (b) hit x5 -> dead -> parts explode -> exploded signal -> removal frame count ---
# Also exercises US2 (part.rs) for free, exactly as V2-C's bullet case exercised hittable.rs.
func _case_b_hit_to_death() -> void:
	var robot := ROBOT_SCENE.instantiate()
	add_child(robot)
	await get_tree().process_frame
	# TODO(/speckit-implement): robot.rpc("hit") x5 a few frames apart; dump dead, Death
	# visibility, each part's fade_value per frame, the puff's resolved parent path (documented
	# #15 divergence, not asserted equal) and world position (asserted equal), the exploded
	# signal fire count, and the frame is_instance_valid(robot) first turns false (~600 @60fps).
	print("CASE_B:", JSON.stringify({}))
