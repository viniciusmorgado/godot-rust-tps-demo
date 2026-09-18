# zz_ecs_parity.gd — V3-B parity harness (research R9), V3-A's shape. SCRATCH: copied into
# oxide-godot/ on both trees for a run, never committed there. Scene zz_ecs_parity.tscn = the six
# lines of specs/011 contracts/ecs-api.md §4 (Node3D root with this script); zz_ecs_observer.gd =
# `extends Node` + `func _process(_d: float) -> void: get_parent()._observe()`.
# Run: godot --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=a|b|c|d|e|f
# Log: "$XDG_DATA_HOME/godot/app_userdata/Third-Person Shooter Demo/zz_ecs_parity_<case>.log"
#   "F<frame> key=value ..."  observer lines, state at the START of the frame's process pass (parity evidence)
#   "P<step> key=value ..."   physics-step lines from a probe at physics priority i32::MIN (case a, f)
#   "RAW F<frame> <event>"    signal/transition stamps as they happen (timing table input)
extends Node3D

const PLAYER_SCENE := "res://player/player.tscn"
var case := "a"
var log_file: FileAccess
var player: CharacterBody3D
var floor_body: StaticBody3D
var anim_tree: AnimationTree
var input_node: Node
var cooldown: Timer
var camera: Camera3D
var camera_base: Node3D
var camera_rot: Node3D
var color_rect: ColorRect
var snd := {}
var snd_playing := {}
var last_bullets := 0
var last_origin_y := 0.0

func _ready() -> void:
	seed(1)  # FIRST: CameraNoiseShake draws randi() when player.tscn is instantiated below
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	log_file = FileAccess.open("user://zz_ecs_parity_%s.log" % case, FileAccess.WRITE)
	var observer := Node.new(); observer.name = "Observer"
	observer.process_priority = -2147483648
	observer.set_script(load("res://zz_ecs_observer.gd"))   # BEFORE add_child (V3-A lesson)
	add_child(observer)
	floor_body = StaticBody3D.new()
	var cs := CollisionShape3D.new(); var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); cs.shape = box
	floor_body.add_child(cs); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	player = load(PLAYER_SCENE).instantiate()
	player.player_id = 1
	player.position = Vector3(0, 0.05, 0)   # BEFORE add_child (V3-A lesson)
	add_child(player)
	anim_tree = player.get_node("AnimationTree")
	input_node = player.get_node("InputSynchronizer")
	cooldown = player.get_node("FireCooldown")
	camera_base = player.get_node("CameraBase"); camera_rot = player.get_node("CameraBase/CameraRot")
	camera = player.get_node("CameraBase/CameraRot/SpringArm3D/Camera3D")
	color_rect = player.get_node("ColorRect")
	for n in ["Jump", "Land", "Shoot"]:
		snd[n] = player.get_node("SoundEffects/" + n); snd_playing[n] = false
	if case in ["a", "f"]:
		var probe := Node.new(); probe.name = "PhysicsProbe"; probe.set_script(load("res://zz_ecs_physics_probe.gd")); add_child(probe)
	# zz_ecs_physics_probe.gd: extends Node; _ready: process_physics_priority = -2147483648;
	# _physics_process: get_parent()._observe_physics()

func _log(line: String) -> void:
	log_file.store_line(line)
func _frame() -> int:
	return Engine.get_process_frames()

func _observe() -> void:
	var f := _frame()
	for n in snd.keys():
		var p: bool = snd[n].playing
		if p != snd_playing[n]:
			_log("RAW F%d %s.playing=%s" % [f, n, p]); snd_playing[n] = p
	match case:
		"a", "f":
			_log("F%d anim=%d motion=%s in_tree=%s alpha=%.6f" % [f, player.current_animation, player.motion, player.is_inside_tree(), color_rect.modulate.a])
		"b":
			_log("F%d anim=%d on_floor=%s vel_y=%.6f" % [f, player.current_animation, player.is_on_floor(), player.velocity.y])
		"c":
			var bullets := 0
			for c in get_children(): if c.name.begins_with("Bullet") or c.get_class() == "Bullet": bullets += 1
			_log("F%d shoot_target=%s aiming=%s shooting=%s cooldown=%.6f bullets=%d aim_add=%s" % [f, input_node.shoot_target, input_node.aiming, input_node.shooting, cooldown.time_left, bullets, anim_tree.get("parameters/aim/add_amount")])
			if bullets > last_bullets:
				var b := get_children()[get_child_count() - 1]
				_log("RAW F%d bullet_spawned transform=%s" % [f, b.global_transform]); last_bullets = bullets
		"d":
			_log("F%d cam_rot=%s" % [f, camera.rotation])
		"e":
			_log("F%d base_y=%.6f rot_x=%.6f" % [f, camera_base.rotation.y, camera_rot.rotation.x])

func _observe_physics() -> void:
	var s := Engine.get_physics_frames()
	_log("P%d origin=%s vel=%s motion=%s anim=%d basis=%s" % [s, player.global_position, player.velocity, player.motion, player.current_animation, player.get_node("PlayerModel").global_transform.basis])
	if case == "f" and player.global_position.y > last_origin_y + 5.0 and s > 60:
		_log("RAW P%d respawned" % s)
	last_origin_y = player.global_position.y

func _process(_d: float) -> void:
	var f := _frame()
	match case:
		"a":
			if f == 10: Input.action_press("move_forward")
			if f == 100: Input.action_release("move_forward")
		"b":
			if f == 30: Input.action_press("jump")
			if f == 31: Input.action_release("jump")
		"c":
			if f == 20: Input.action_press("aim")
			if f == 40: Input.action_press("shoot")
		"d":
			if f == 30: player.hit()
		"e":
			if f == 25:
				var ev := InputEventMouseMotion.new(); ev.screen_relative = Vector2(20, -10); ev.relative = Vector2(20, -10)
				Input.parse_input_event(ev)
		"f":
			if f == 50 and is_instance_valid(floor_body):
				_log("RAW F%d floor_removed" % f); floor_body.queue_free()
