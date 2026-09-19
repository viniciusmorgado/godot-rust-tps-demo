extends Node3D
# R3-bis (specs/013): does a RayCast3D CHILD update before or after its PARENT's _physics_process
# (both priority 0)? Holder (priority 0, script) owns the RayCast pointing down at a floor; the
# MIN probe moves Holder +0.01 in x each step; Holder prints the ray's collision point it sees;
# the MAX probe prints what the driver would see.
var holder: Node3D
var rc: RayCast3D
func _ready() -> void:
	var floor_body := StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	var lo := Node.new(); lo.name = "ProbeMin"; lo.set_script(load("res://zz_r3o_min.gd")); add_child(lo)
	holder = Node3D.new(); holder.name = "Holder"; holder.set_script(load("res://zz_r3o_holder.gd"))
	rc = RayCast3D.new(); rc.target_position = Vector3(0, -2, 0); rc.enabled = true
	holder.add_child(rc); holder.position = Vector3(0, 1, 0); add_child(holder)
	var hi := Node.new(); hi.name = "ProbeMax"; hi.set_script(load("res://zz_r3o_max.gd")); add_child(hi)

# --- zz_r3o_min.gd ---
# extends Node
# func _ready() -> void: process_physics_priority = -2147483648
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s >= 5 and s <= 12:
# 		p.holder.position.x = s * 0.01
# 		print("S%d holder.x=%.2f (set now) ray_point=%s" % [s, p.holder.position.x, p.rc.get_collision_point()])
# 	if s == 13: get_tree().quit()
# --- zz_r3o_holder.gd (Holder, priority 0, the RayCast3D's parent) ---
# extends Node3D
# func _physics_process(_d: float) -> void:
# 	var s := Engine.get_physics_frames()
# 	var rc: RayCast3D = get_child(0)
# 	if s >= 5 and s <= 12: print("H%d (parent, priority 0) sees ray_point.x=%.2f colliding=%s while holder.x=%.2f" % [s, rc.get_collision_point().x, rc.is_colliding(), position.x])
# --- zz_r3o_max.gd ---
# extends Node
# func _ready() -> void: process_physics_priority = 2147483647
# func _physics_process(_d: float) -> void:
# 	var p = get_parent(); var s := Engine.get_physics_frames()
# 	if s >= 5 and s <= 12: print("M%d (MAX) sees ray_point.x=%.2f" % [s, p.rc.get_collision_point().x])
# --- zz_r3_order.tscn: Node3D root with THIS script ---
# Run: godot --headless --path . --fixed-fps 60 zz_r3_order.tscn
# Result 2026-09-19 (research R8): H(n) (the parent's priority-0 _physics_process) sees the PREVIOUS step's
# ray point (H5 0.00 while holder.x=0.05; H6 0.05 while 0.06); M(n) (i32::MAX) sees THIS step's (M5 0.05):
# an engine-updated child updates AFTER its parent's callback and BEFORE the driver.
