# zz_order_probe.gd — the R1 driver-order experiment, kept for re-verification. SCRATCH, never
# committed under oxide-godot/. Three scripts merged here for documentation; at run time split
# them as noted. Register the autoload temporarily in project.godot:
#   ZzOrder="*res://zz_order_autoload.gd"
# and run: ZZ_PRIO=2147483647 godot --headless --path . --fixed-fps 60 --quit-after 16 zz_order.tscn
# (then ZZ_PRIO=0 for the baseline). Revert project.godot and delete the files afterwards.
#
# --- zz_order_autoload.gd (the future EcsWorld's stand-in) ---
# extends Node
# func _ready() -> void:
# 	var prio := 0
# 	if OS.has_environment("ZZ_PRIO"): prio = int(OS.get_environment("ZZ_PRIO"))
# 	process_priority = prio
# 	process_physics_priority = prio   # NOTE: the GDScript property is process_physics_priority;
# 	                                  # the gdext setter is Node::set_physics_process_priority
# 	print("F%d autoload ready prio=%d" % [Engine.get_process_frames(), prio])
# func _process(_d: float) -> void: print("F%d autoload _process" % Engine.get_process_frames())
# func _physics_process(_d: float) -> void: print("P%d autoload _physics_process" % Engine.get_physics_frames())
#
# --- zz_order_observer.gd (child "Observer" of the scene root) ---
# extends Node
# func _ready() -> void:
# 	process_priority = -2147483648
# 	process_physics_priority = -2147483648
# func _process(_d: float) -> void: print("F%d observer(i32::MIN) _process" % Engine.get_process_frames())
# func _physics_process(_d: float) -> void: print("P%d observer(i32::MIN) _physics_process" % Engine.get_physics_frames())
#
# --- zz_order.tscn: Node3D root with THIS script; children: Observer (script above),
#     AnimationPlayer (one animation "blink", length 0.0166667, autoplay) ---
extends Node3D
var t_left := 0.2
func _ready() -> void:
	print("F%d scene ready" % Engine.get_process_frames())
	$AnimationPlayer.animation_finished.connect(func(n): print("F%d anim_finished(%s) [emitted during AnimationPlayer internal process]" % [Engine.get_process_frames(), n]))
	get_tree().create_timer(0.0).timeout.connect(func():
		print("F%d timer0 timeout" % Engine.get_process_frames())
		get_tree().create_timer(0.0).timeout.connect(func(): print("F%d timer1(created in timeout) timeout" % Engine.get_process_frames()))
		var probe := Node.new(); probe.name = "Probe"; add_child(probe)
		probe.tree_exited.connect(func(): print("F%d probe tree_exited" % Engine.get_process_frames()))
		probe.queue_free()
		print("F%d probe queue_free called from timeout" % Engine.get_process_frames()))
	get_tree().create_timer(1.0/60.0).timeout.connect(func(): print("F%d timer(1/60) timeout" % Engine.get_process_frames()))
	get_tree().create_timer(0.2).timeout.connect(func(): print("F%d timer(0.2) timeout" % Engine.get_process_frames()))
func _process(d: float) -> void:
	t_left -= d  # GDScript mirror of SceneTreeTimer's subtraction, to see where 0.2 lands
	print("F%d scene _process (delta=%s, mirror time_left=%s)" % [Engine.get_process_frames(), d, t_left])
func _physics_process(_d: float) -> void:
	print("P%d scene _physics_process" % Engine.get_physics_frames())

# Observed on Godot 4.7.2, 2026-09-18 (research.md R1): at ZZ_PRIO=2147483647 the autoload's
# _physics_process printed after the scene's, and its _process printed after the scene's _process
# AND after anim_finished, but before every SceneTreeTimer timeout; timer(0.2) fired at F12
# (13th step); the probe queue_free'd from a timeout left the tree in the same frame.
