# zz_r3_probe.gd — headless input driving experiment (research R3). Scratch. Root Node with this
# script; children "Reader" (priority i32::MAX) and "Listener" created in _ready.
extends Node
var seq := 0
func _ready() -> void:
	var reader := Node.new(); reader.name = "Reader"; reader.set_script(load("res://zz_r3_reader.gd")); add_child(reader)
	var listener := Node.new(); listener.name = "Listener"; listener.set_script(load("res://zz_r3_listener.gd")); add_child(listener)
func _process(_d: float) -> void:
	var f := Engine.get_process_frames()
	seq += 1
	print("F%d seq=%d root._process" % [f, seq])
	if f == 10:
		Input.action_press("move_forward"); print("F%d seq=%d action_press(move_forward)" % [f, seq])
	if f == 12:
		Input.action_release("move_forward"); print("F%d seq=%d action_release(move_forward)" % [f, seq])
	if f == 25:
		var ev := InputEventMouseMotion.new(); ev.screen_relative = Vector2(20, -10); ev.relative = Vector2(20, -10)
		Input.parse_input_event(ev); print("F%d seq=%d parse_input_event(mouse 20,-10)" % [f, seq])
# --- zz_r3_reader.gd ---
# extends Node
# func _ready() -> void: process_priority = 2147483647
# func _process(_d: float) -> void:
# 	var p := get_parent(); p.seq += 1; var f := Engine.get_process_frames()
# 	if f >= 9 and f <= 14: print("F%d seq=%d reader(MAX) strength=%.1f just_pressed=%s pressed=%s" % [f, p.seq, Input.get_action_strength("move_forward"), Input.is_action_just_pressed("move_forward"), Input.is_action_pressed("move_forward")])
# --- zz_r3_listener.gd ---
# extends Node
# func _process(_d: float) -> void:
# 	var p := get_parent(); p.seq += 1; var f := Engine.get_process_frames()
# 	if f >= 24 and f <= 27: print("F%d seq=%d listener._process" % [f, p.seq])
# func _input(event: InputEvent) -> void:
# 	if event is InputEventMouseMotion:
# 		var p := get_parent(); p.seq += 1; print("F%d seq=%d listener._input(mouse relative=%s)" % [Engine.get_process_frames(), p.seq, event.screen_relative])
# Result 2026-09-18: F10 press → reader same frame strength=1.0 just_pressed=true; F11 just_pressed=false;
# F25 parse_input_event → F26 listener._input (seq 79) BEFORE root._process (seq 80).
