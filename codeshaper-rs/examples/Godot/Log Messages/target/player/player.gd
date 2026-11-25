class_name Player
extends CharacterBody2D


func _ready() -> void:
	print("ready")


func jump() -> void:
	if is_on_floor():
		print("jumped from %s" % position)
