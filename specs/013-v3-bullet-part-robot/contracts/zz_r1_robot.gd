extends Node3D
# R1 twin for the ROBOT (specs/013 research R1). ZZ_MODE=physics keeps callback_mode_process = 0;
# ZZ_MODE=manual sets MANUAL and advances the tree from the MAX probe after the robot's tick.
# A player.tscn is added at physics step 10 inside the detection area (sphere r=20, red_robot.tscn:10211),
# 8 m in front of the robot, so the robot enters Approach and walks by root motion.
var robot: CharacterBody3D
var player: CharacterBody3D
var tree: AnimationTree
var mode := "physics"
var player_added := false
func _ready() -> void:
	seed(1)
	for a in InputMap.get_actions():
		for ev in InputMap.action_get_events(a):
			if ev is InputEventJoypadMotion or ev is InputEventJoypadButton: InputMap.action_erase_event(a, ev)
	if OS.has_environment("ZZ_MODE"): mode = OS.get_environment("ZZ_MODE")
	var floor_body := StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	robot = load("res://enemies/red_robot/red_robot.tscn").instantiate()
	robot.position = Vector3(0, 0.05, 0)
	add_child(robot)
	tree = robot.get_node("AnimationTree")
	if mode == "manual":
		tree.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	print("MODE %s callback_mode_process=%d tree_active=%s" % [mode, tree.callback_mode_process, tree.active])
	var probe := "zz_r1"
	if OS.has_environment("ZZ_PROBE"): probe = OS.get_environment("ZZ_PROBE")
	var lo := Node.new(); lo.name = "ProbeMin"; lo.set_script(load("res://%s_min.gd" % probe)); add_child(lo)
	var hi := Node.new(); hi.name = "ProbeMax"; hi.set_script(load("res://%s_max.gd" % probe)); add_child(hi)
func add_player() -> void:
	player = load("res://player/player.tscn").instantiate()
	player.player_id = 1
	var pz := -8.0
	if OS.has_environment("ZZ_PZ"): pz = float(OS.get_environment("ZZ_PZ"))
	player.position = Vector3(0, 0.05, pz)
	add_child(player)
	player_added = true

# --- zz_r1_min.gd (child "ProbeMin") ---
# extends Node
# func _ready() -> void: process_physics_priority = -2147483648
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s == 10 and not p.player_added: p.add_player()
# 	if s <= 300: print("S%d rm=%s rmr=%s origin=%s basis_z=%s state=%d node=%s target=%s aim=%s" % [s, p.tree.get_root_motion_position(), p.tree.get_root_motion_rotation(), p.robot.global_position, p.robot.global_transform.basis.z, p.robot.state, p.tree.get("parameters/state/current_state"), p.robot.target_position, p.tree.get("parameters/aim/blend_position")])
# --- zz_r1_max.gd (child "ProbeMax") ---
# extends Node
# func _ready() -> void: process_physics_priority = 2147483647
# func _physics_process(d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if p.mode == "manual": p.tree.advance(d)
# 	if s <= 300: print("M%d rm=%s rmr=%s origin=%s" % [s, p.tree.get_root_motion_position(), p.tree.get_root_motion_rotation(), p.robot.global_position])
# --- zz_r3_min.gd / zz_r3_max.gd (ZZ_PROBE=zz_r3: research R8, the laser RayCast3D) ---
# extends Node
# # R3-bis: the laser RayCast3D's update order. S<n> = before the robot's tick (priority MIN),
# # M<n> = after it and after every child's internal physics processing (priority MAX).
# func _ready() -> void: process_physics_priority = -2147483648
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s == 10 and not p.player_added: p.add_player()
# 	var rc: RayCast3D = p.robot.get_node("RedRobotModel/Armature/Skeleton3D/RayFrom/RayCast")
# 	if s >= 300 and s <= 520: print("S%d state=%d colliding=%s point=%s origin=%s" % [s, p.robot.state, rc.is_colliding(), rc.get_collision_point(), p.robot.global_position])
# extends Node
# func _ready() -> void: process_physics_priority = 2147483647
# func _physics_process(d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if p.mode == "manual": p.tree.advance(d)
# 	var rc: RayCast3D = p.robot.get_node("RedRobotModel/Armature/Skeleton3D/RayFrom/RayCast")
# 	if s >= 300 and s <= 520: print("M%d state=%d colliding=%s point=%s" % [s, p.robot.state, rc.is_colliding(), rc.get_collision_point()])
# --- zz_r1_robot.tscn: Node3D root with THIS script ---
# Run: ZZ_PZ=8 ZZ_MODE=physics|manual [ZZ_PROBE=zz_r3] godot --headless --path . --fixed-fps 60 --quit-after 305 zz_r1_robot.tscn
# Result 2026-09-19 (research R1): ZZ_PZ=8 (player in FRONT — the robot's front is +Z): v2 physics == v3 physics
# == v3 manual+advance, 601 lines each, only the MODE line differs; M(n).rm == S(n+1).rm at every step
# (M12 0.000016 = S13, M13 0.000032 = S14, M200 0.014937 = S201). ZZ_PZ=-8: the robot only turns
# (node=turn_left, rm stays zero) — the same three verdicts. R8 (ZZ_PROBE=zz_r3): colliding=false at
# every step S300–S520 through Aim (S371) and Shooting (S432): the laser RayCast is enabled=false.
