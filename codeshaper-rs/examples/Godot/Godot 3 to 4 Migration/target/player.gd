extends KinematicBody2D

export var speed = 200.0
export var jump_height = 64.0
var velocity = Vector2.ZERO


func _physics_process(delta):
	velocity = move_and_slide(velocity)
