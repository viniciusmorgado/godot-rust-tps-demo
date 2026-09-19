# zz_ecs_parity.gd — V3-C parity harness (research R10), V3-B's shape. SCRATCH: copied into
# oxide-godot/oxide-godot/ (the Godot project directory) on both trees for a run, never committed
# there. Scene zz_ecs_parity.tscn = the six lines of specs/011 contracts/ecs-api.md §4 (Node3D
# root with this script). zz_ecs_observer.gd is exactly:
#   extends Node
#   func _process(_d: float) -> void: get_parent()._observe()
# zz_ecs_physics_probe.gd is exactly:
#   extends Node
#   func _ready() -> void: process_physics_priority = -2147483648
#   func _physics_process(_d: float) -> void: get_parent()._observe_physics()
# Both scripts are set BEFORE add_child (V3-A's lesson). Bodies are positioned BEFORE add_child.
# Run: godot --headless --path . --fixed-fps 60 --quit-after <a,b:900 c:450 d:200 e:305> zz_ecs_parity.tscn -- --case=a|b|c|d|e
# Log: "$XDG_DATA_HOME/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_<case>.log"
#   "F<frame> key=value ..."  observer lines, state at the START of the frame's process pass (parity evidence)
#   "P<step> key=value ..."   physics-step lines from the probe at physics priority i32::MIN
#   "RAW F<frame>|P<step> <event>"  transitions as they happen (timing-table input)
extends Node3D

const ROBOT_SCENE := "res://enemies/red_robot/red_robot.tscn"
const PLAYER_SCENE := "res://player/player.tscn"
const BULLET_SCENE := "res://player/bullet/bullet.tscn"
var case := "a"
var log_file: FileAccess
var robot: CharacterBody3D
var player: CharacterBody3D
var floor_body: StaticBody3D
var anim_tree: AnimationTree
var shoot_anim: AnimationPlayer
var ray_mesh: MeshInstance3D
var ember: Node3D
var parts := []
var part_valid := [true, true, true]
var bullet: CharacterBody3D
var bullet_anim: AnimationPlayer
var bullet_shape: CollisionShape3D
var last_health := 5
var last_dead := false
var last_state := -1
var last_shoot_playing := false
var last_root_children := 0
var last_self_children := 0
var last_cam_rot := Vector3.ZERO
var player_added := false
var death_step := -1

func _ready() -> void:
	seed(1)  # FIRST: CameraNoiseShake draws randi() when player.tscn is instantiated (cases a, d, e)
	# Real joypads on the host pollute the actions (V3-B lesson): unbind every joypad event before
	# the first input flush; no Input.action_release (it raises just_released → aim toggles on).
	for a in InputMap.get_actions():
		for ev in InputMap.action_get_events(a):
			if ev is InputEventJoypadMotion or ev is InputEventJoypadButton: InputMap.action_erase_event(a, ev)
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	log_file = FileAccess.open("user://zz_ecs_parity_%s.log" % case, FileAccess.WRITE)
	var observer := Node.new(); observer.name = "Observer"
	observer.process_priority = -2147483648
	observer.set_script(load("res://zz_ecs_observer.gd"))   # BEFORE add_child
	add_child(observer)
	var probe := Node.new(); probe.name = "PhysicsProbe"
	probe.set_script(load("res://zz_ecs_physics_probe.gd"))  # BEFORE add_child
	add_child(probe)
	floor_body = StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	if case != "c":
		robot = load(ROBOT_SCENE).instantiate()
		robot.position = Vector3(0, 0.05, 0)                # facing +Z (red_robot/model.rs:90-95)
		if case == "d": robot.test_shoot = true            # → shoot_countdown = 0 at ready, shoot() on the first step
		add_child(robot)
		anim_tree = robot.get_node("AnimationTree")
		shoot_anim = robot.get_node("ShootAnimation")
		ray_mesh = robot.get_node("RedRobotModel/Armature/Skeleton3D/RayFrom/RayMesh")
		ember = robot.get_node("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber")
		for n in ["PartShield1", "PartShield2", "PartHead"]: parts.append(robot.get_node("Death/" + n))
		last_health = robot.health
		last_root_children = get_tree().get_root().get_child_count()
		last_self_children = get_child_count()
	if case == "d": _add_player(8.0)                        # in front, from step 1
	if case == "c": _spawn_bullet(Vector3(0, 1, 0), Vector3(100, 1, 0))

func _add_player(pz: float) -> void:
	player = load(PLAYER_SCENE).instantiate()
	player.player_id = 1
	player.position = Vector3(0, 0.05, pz)
	add_child(player)
	player_added = true
	last_cam_rot = player.get_node("CameraBase/CameraRot/SpringArm3D/Camera3D").rotation

# The player's spawn path (player/sync.rs::orient_and_anim): instantiate, add_child under the
# harness root, set_global_position, look_at (the bullet flies along -basis.z, bullet.rs:112).
func _spawn_bullet(from: Vector3, target: Vector3) -> void:
	bullet = load(BULLET_SCENE).instantiate()
	add_child(bullet)
	bullet.set_global_position(from)
	bullet.look_at(target)
	bullet_anim = bullet.get_node("AnimationPlayer")
	bullet_shape = bullet.get_node("CollisionShape3D")

func _log(line: String) -> void:
	log_file.store_line(line)
func _frame() -> int:
	return Engine.get_process_frames()

func _observe() -> void:
	var f := _frame()
	match case:
		"a", "e":
			if is_instance_valid(robot):
				var playing := shoot_anim.is_playing()
				if playing != last_shoot_playing:
					_log("RAW F%d ShootAnimation.playing=%s" % [f, playing]); last_shoot_playing = playing
				var rc := get_tree().get_root().get_child_count()
				if rc != last_root_children:
					_log("RAW F%d root_children=%d (blast spawn)" % [f, rc]); last_root_children = rc
		"b":
			var sc := get_child_count()
			if sc != last_self_children:
				_log("RAW F%d harness_children=%d (bullet/puff spawn or free)" % [f, sc]); last_self_children = sc
		"c":
			if is_instance_valid(bullet):
				_log("F%d origin=%s anim=%s disabled=%s" % [f, bullet.global_position, bullet_anim.current_animation, bullet_shape.disabled])
			elif bullet != null:
				_log("RAW F%d bullet_freed" % f); bullet = null
		"d":
			var rc := get_tree().get_root().get_child_count()
			if rc != last_root_children:
				_log("RAW F%d root_children=%d (blast spawn)" % [f, rc]); last_root_children = rc
			var mat: ShaderMaterial = ray_mesh.get_surface_override_material(0)
			var clip = mat.get_shader_parameter("clip") if mat != null else null
			var cam := player.get_node("CameraBase/CameraRot/SpringArm3D/Camera3D")
			if cam.rotation != last_cam_rot:
				_log("RAW F%d camera_rotation_changed (trauma arrival)" % f); last_cam_rot = cam.rotation
			_log("F%d clip=%s ember=%s cam_rot=%s" % [f, clip, ember.position, cam.rotation])

func _observe_physics() -> void:
	var s := Engine.get_physics_frames()
	match case:
		"a":
			if s == 10 and not player_added: _add_player(8.0)
			if is_instance_valid(robot):
				if robot.state != last_state:
					_log("RAW P%d state=%d" % [s, robot.state]); last_state = robot.state
				_log("P%d state=%d target=%s aim_preparing=%.6f origin=%s node=%s aim=%s" % [s, robot.state, robot.target_position, robot.aim_preparing, robot.global_position, anim_tree.get("parameters/state/current_state"), anim_tree.get("parameters/aim/blend_position")])
		"e":
			if s == 10 and not player_added: _add_player(8.0)
			if is_instance_valid(robot) and s <= 300:
				_log("P%d rm=%s origin=%s state=%d node=%s" % [s, anim_tree.get_root_motion_position(), robot.global_position, robot.state, anim_tree.get("parameters/state/current_state")])
		"b":
			if s >= 30 and s % 30 == 0 and death_step < 0:
				_spawn_bullet(Vector3(0, 1.2, 6), robot.global_position + Vector3(0, 1.2, 0))
				_log("RAW P%d bullet_spawned" % s)
			if is_instance_valid(robot):
				if robot.health != last_health:
					_log("RAW P%d health=%d" % [s, robot.health]); last_health = robot.health
				if robot.dead != last_dead:
					_log("RAW P%d dead=%s" % [s, robot.dead]); last_dead = robot.dead; death_step = s
					for i in 3: _log("RAW P%d part%d angular=%s" % [s, i, parts[i].angular_velocity])
				var line := "P%d health=%d dead=%s" % [s, robot.health, robot.dead]
				for i in 3:
					if part_valid[i] and not is_instance_valid(parts[i]):   # validity FIRST (R2's lesson)
						part_valid[i] = false; _log("RAW P%d part%d_freed" % [s, i])
					if part_valid[i]:
						var q = parts[i]
						line += " | p%d pos=%s lv=%s av=%s fade=%.6f" % [i, q.global_position, q.linear_velocity, q.angular_velocity, q.fade_value]
				_log(line)
			elif robot != null:
				_log("RAW P%d robot_freed" % s); robot = null
		"c":
			if is_instance_valid(bullet):
				_log("P%d origin=%s" % [s, bullet.global_position])
