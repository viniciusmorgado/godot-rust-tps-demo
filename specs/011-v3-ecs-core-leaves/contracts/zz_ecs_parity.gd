# zz_ecs_parity.gd — V3-A parity harness (research R10). SCRATCH: copied into oxide-godot/ on
# both trees for a run, never committed there. Scene: zz_ecs_parity.tscn = a Node3D root with
# this script. Run: godot --headless --path . --fixed-fps 60 --quit-after 400 zz_ecs_parity.tscn -- --case=a|b|c
# Output: user://zz_ecs_parity_<case>.log (under the split XDG_DATA_HOME). Fixed line formats:
#   "F<frame> <key>=<value> ..."  observer lines (state at the START of frame <frame>'s process pass,
#                                  i.e. what the previous frame left) — these are the parity evidence
#   "RAW F<frame> <event>"        signal/timer stamps as they happen — informational, diffed separately
extends Node3D

const DOOR_SCENE := "res://door/door.tscn"
const PLAYER_SCENE := "res://player/player.tscn"
const PUFF_SCENE := "res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn"
const BLAST_SCENE := "res://enemies/red_robot/laser/impact_effect/impact_effect.tscn"

var case := "a"
var log_file: FileAccess
var observer: Node
# case a
var door: Node3D
var door2: Node3D
var player: CharacterBody3D
var mover: CharacterBody3D
var door_anim: AnimationPlayer
var door2_anim: AnimationPlayer
var opened_logged := false
# case b
var puff: CPUParticles3D
var puff_id := 0
# case c
var camera: Camera3D
var blast: Node3D
var blast_id := 0

func _ready() -> void:
	seed(1)
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--case="):
			case = a.substr(len("--case="))
	log_file = FileAccess.open("user://zz_ecs_parity_%s.log" % case, FileAccess.WRITE)
	observer = Node.new()
	observer.name = "Observer"
	observer.process_priority = -2147483648  # i32::MIN — first in every process pass (research R1)
	# The script MUST be set BEFORE add_child: a script attached to a node already in the tree does
	# not get its _ready/processing enabled, and the observer never logs (found at T021).
	observer.set_script(load("res://zz_ecs_observer.gd"))
	add_child(observer)
	# zz_ecs_observer.gd (second scratch file, copied alongside this one) is exactly:
	#   extends Node
	#   func _process(_d: float) -> void: get_parent()._observe()
	match case:
		"a": _setup_a()
		"b": _setup_b()
		"c": _setup_c()
		_: push_error("unknown --case=%s" % case)

func _log(line: String) -> void:
	log_file.store_line(line)

func _frame() -> int:
	return Engine.get_process_frames()

# ---------- observer (called first in every process pass) ----------
func _observe() -> void:
	match case:
		"a":
			_log("F%d door_anim=%s door2_anim=%s" % [_frame(), door_anim.current_animation, door2_anim.current_animation])
		"b":
			var in_tree := is_instance_valid(puff) and puff.is_inside_tree()
			var e := str(puff.emitting) if in_tree else "-"
			var m := str(puff.get_node("MiniBlasts").emitting) if in_tree else "-"
			_log("F%d puff_in_tree=%s puff_emitting=%s mini_emitting=%s" % [_frame(), in_tree, e, m])
		"c":
			var in_tree := is_instance_valid(blast) and blast.is_inside_tree()
			var basis := str(blast.get_node("LightRays").global_transform.basis) if in_tree else "-"
			_log("F%d blast_in_tree=%s rays_basis=%s" % [_frame(), in_tree, basis])

# ---------- (a) door: a Player body opens it once, a non-player body never ----------
func _setup_a() -> void:
	var floor_body := StaticBody3D.new()
	var shape := CollisionShape3D.new()
	var box := BoxShape3D.new(); box.size = Vector3(200, 1, 200); shape.shape = box
	floor_body.add_child(shape); floor_body.position = Vector3(0, -0.5, 0); add_child(floor_body)
	# Positions are set BEFORE add_child: a body added at the origin sits inside door's area for
	# its first physics step and opens it at F1 regardless of where it is moved afterwards
	# (found at T021).
	door = load(DOOR_SCENE).instantiate(); door.position = Vector3(0, 0, 0); add_child(door)
	door2 = load(DOOR_SCENE).instantiate(); door2.position = Vector3(0, 0, 40); add_child(door2)
	door_anim = door.get_node("DoorModel2/AnimationPlayer")
	door2_anim = door2.get_node("DoorModel2/AnimationPlayer")
	player = load(PLAYER_SCENE).instantiate()
	player.position = Vector3(0, 1, -12); player.collision_layer = 1  # harness-only: bypass backlog #30's mask
	add_child(player)
	mover = CharacterBody3D.new()
	var ms := CollisionShape3D.new(); var mb := BoxShape3D.new(); mb.size = Vector3(1, 2, 1); ms.shape = mb
	mover.add_child(ms); mover.position = Vector3(0, 1, 28); mover.collision_layer = 1; add_child(mover)
	door_anim.animation_started.connect(func(n): _log("RAW F%d door_anim_started=%s" % [_frame(), n]))
	door2_anim.animation_started.connect(func(n): _log("RAW F%d door2_anim_started=%s" % [_frame(), n]))

func _physics_process(_d: float) -> void:
	match case:
		"a":
			# physics-step body entry, as bodies move in the game (FR-029 (a))
			player.velocity = Vector3(0, 0, 6); player.move_and_slide()
			mover.velocity = Vector3(0, 0, 6); mover.move_and_slide()
		"c":
			if Engine.get_physics_frames() == 5:
				# physics-step instancing, as red_robot.rs:423-425 does (FR-029 (c))
				blast = load(BLAST_SCENE).instantiate()
				get_tree().root.add_child(blast)
				blast.global_position = Vector3(0, 0, -5)
				blast_id = blast.get_instance_id()
				blast.tree_exited.connect(func(): _log("RAW F%d blast_tree_exited" % _frame()))
				_log("RAW F%d blast_instanced_in_physics_step_%d" % [_frame(), Engine.get_physics_frames()])

# ---------- (b) puff instanced from a SceneTreeTimer.timeout callback (part.rs:196-205's path) ----------
func _setup_b() -> void:
	get_tree().create_timer(0.5).timeout.connect(func():
		puff = load(PUFF_SCENE).instantiate()
		add_child(puff)
		puff.global_position = Vector3.ZERO
		puff_id = puff.get_instance_id()
		puff.tree_exited.connect(func(): _log("RAW F%d puff_tree_exited" % _frame()))
		_log("RAW F%d puff_instanced_from_timer_timeout" % _frame()))

# ---------- (c) blast with a camera that moves every frame ----------
func _setup_c() -> void:
	camera = Camera3D.new(); add_child(camera); camera.position = Vector3(3, 2, 4); camera.make_current()

func _process(_d: float) -> void:
	if case == "c":
		# The camera moves at priority 0, from this node. On v2 the Blast's own process runs later in
		# the same pass (the blast sits under /root after the current scene in tree order); on v3 the
		# driver runs last (i32::MAX). Both compute look_at AFTER this frame's move, and the observer
		# line of the next frame records the resulting basis — identical if the plan is right.
		camera.position.x += 0.1
