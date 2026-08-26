extends Node2D

func _draw() -> void:
	draw_rect(Rect2(0, 0, 960, 540), Color("101722"))
	draw_rect(Rect2(72, 362, 816, 68), Color("4c5868"), true)
	draw_rect(Rect2(272, 232, 416, 142), Color("d66e38"), true)
	draw_rect(Rect2(602, 266, 86, 108), Color("b6502e"), true)
	draw_circle(Vector2(354, 386), 28, Color("17202b"))
	draw_circle(Vector2(610, 386), 28, Color("17202b"))
	draw_rect(Rect2(184, 302, 52, 52), Color("e8c468"), true)
	draw_line(Vector2(209, 302), Vector2(209, 236), Color("d9e5ee"), 4.0)
	draw_string(ThemeDB.fallback_font, Vector2(72, 100), "COSMON — TWO-CLIENT TECHNICAL WALKING SKELETON", HORIZONTAL_ALIGNMENT_LEFT, -1, 24, Color("eaf0f5"))
	draw_string(ThemeDB.fallback_font, Vector2(72, 138), "crate → truck → destination", HORIZONTAL_ALIGNMENT_LEFT, -1, 20, Color("b7c6d6"))

func _ready() -> void:
	queue_redraw()
