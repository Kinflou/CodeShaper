extends Node


func info(script_name: String, message: String) -> void:
	print("%s: %s" % [script_name, message])


func warn(script_name: String, message: String) -> void:
	push_warning("%s: %s" % [script_name, message])
