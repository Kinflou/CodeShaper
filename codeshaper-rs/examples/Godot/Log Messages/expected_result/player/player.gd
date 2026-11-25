class_name Player
extends CharacterBody2D


func _ready() -> void:
	GameLog.info("Player", "ready")


func jump() -> void:
	if is_on_floor():
		GameLog.info("Player", "jumped from %s" % position)
