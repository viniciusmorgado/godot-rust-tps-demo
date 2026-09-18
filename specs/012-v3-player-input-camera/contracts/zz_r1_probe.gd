extends Node3D
# R1 probe (AnimationTree ordering). ZZ_MODE=physics keeps callback_mode_process = 0;
# ZZ_MODE=manual sets MANUAL and advances the tree from the MAX probe after the player's tick.
var player: CharacterBody3D
var tree: AnimationTree
var mode := "physics"
func _ready() -> void:
	seed(1)
	if OS.has_environment("ZZ_MODE"): mode = OS.get_environment("ZZ_MODE")
	var floor_body := StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	player = load("res://player/player.tscn").instantiate()
	player.player_id = 1
	player.position = Vector3(0, 0.05, 0)
	add_child(player)
	tree = player.get_node("AnimationTree")
	var cam := player.get_node("CameraBase/CameraRot/SpringArm3D/Camera3D")
	print("OWNER camera.owner==player: %s ; input.owner==player: %s" % [cam.owner == player, player.get_node("InputSynchronizer").owner == player])
	if mode == "manual":
		tree.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	print("MODE %s callback_mode_process=%d" % [mode, tree.callback_mode_process])
	var lo := Node.new(); lo.name = "ProbeMin"; lo.set_script(load("res://zz_r1_min.gd")); add_child(lo)
	var hi := Node.new(); hi.name = "ProbeMax"; hi.set_script(load("res://zz_r1_max.gd")); add_child(hi)
func _process(_d: float) -> void:
	var f := Engine.get_process_frames()
	if f == 10: Input.action_press("move_forward")
	if f == 100: Input.action_release("move_forward")

# --- zz_r1_min.gd (child "ProbeMin") ---
# extends Node
# func _ready() -> void: process_physics_priority = -2147483648
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s <= 130: print("S%d rm=%s origin=%s vel=%s anim=%d blend=%s" % [s, p.tree.get_root_motion_position(), p.player.global_position, p.player.velocity, p.player.current_animation, p.tree.get("parameters/walk/blend_position")])
# --- zz_r1_max.gd (child "ProbeMax") ---
# extends Node
# func _ready() -> void: process_physics_priority = 2147483647
# func _physics_process(d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if p.mode == "manual": p.tree.advance(d)
# 	if s <= 130: print("M%d rm=%s origin=%s" % [s, p.tree.get_root_motion_position(), p.player.global_position])
# --- zz_r1.tscn: Node3D root with THIS script ---
# Run: ZZ_MODE=physics|manual godot --headless --path . --fixed-fps 60 --quit-after 135 zz_r1.tscn
# Result 2026-09-18 (research R1): v2 physics == v3 physics == v3 manual+advance, 263 lines each;
# M(n).rm == S(n+1).rm at every step (the option-A read-point shift).
