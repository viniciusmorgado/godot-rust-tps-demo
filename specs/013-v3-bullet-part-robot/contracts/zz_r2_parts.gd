extends Node3D
# R2 (specs/013): RigidBody3D determinism of the parts. The robot is killed at physics step 30 by
# five hit() calls (hit is a callable #[rpc] method; call_local in v2). Per step, each part's
# position / linear_velocity / angular_velocity / fade_value, for 300 steps; the explode step's
# angular velocities prove the seeded 13-draw order.
var robot: CharacterBody3D
var parts := []
var killed := false
func _ready() -> void:
	seed(1)
	for a in InputMap.get_actions():
		for ev in InputMap.action_get_events(a):
			if ev is InputEventJoypadMotion or ev is InputEventJoypadButton: InputMap.action_erase_event(a, ev)
	var floor_body := StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	robot = load("res://enemies/red_robot/red_robot.tscn").instantiate()
	robot.position = Vector3(0, 0.05, 0)
	add_child(robot)
	for n in ["PartShield1", "PartShield2", "PartHead"]: parts.append(robot.get_node("Death/" + n))
	var lo := Node.new(); lo.name = "ProbeMin"; lo.set_script(load("res://zz_r2_min.gd")); add_child(lo)

# --- zz_r2_min.gd (child "ProbeMin"; NOTE: test is_instance_valid(part) BEFORE reading it — this version read first and stopped at S269) ---
# extends Node
# func _ready() -> void: process_physics_priority = -2147483648
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s == 30 and not p.killed:
# 		for i in 5: p.robot.hit()
# 		p.killed = true
# 		print("RAW S%d killed health=%d dead=%s" % [s, p.robot.health, p.robot.dead])
# 		for i in 3: print("RAW S%d part%d angular=%s wait_started" % [s, i, p.parts[i].angular_velocity])
# 	if s <= 330:
# 		var line := "S%d" % s
# 		for i in 3:
# 			var q = p.parts[i]
# 			line += " | p%d pos=%s lv=%s av=%s fade=%.6f valid=%s" % [i, q.global_position, q.linear_velocity, q.angular_velocity, q.fade_value, is_instance_valid(q)]
# 		print(line)
# 	if s == 330: get_tree().quit()
# --- zz_r2_parts.tscn: Node3D root with THIS script ---
# Run: godot --headless --path . --fixed-fps 60 --quit-after 340 zz_r2_parts.tscn
# Result 2026-09-19 (research R2): v2 run1 == v2 run2 == v3 run1 (273 lines each, exit 0): the RigidBody3D
# solver is deterministic under --fixed-fps 60; explode-step angular velocities identical on both trees
# (part0 (-1.435281, 8.067381, -9.53594), part1 (8.159636, -1.621307, -9.841543), part2 (3.475226, 4.696961,
# -8.445145)) — the seeded thirteen-draw order reproduces; PartHead fades from S241, destroyed ~S259, freed ~S270.
