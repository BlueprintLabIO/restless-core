extends Node3D
## Swift Arrival — Dogfood 4 networked delivery: playable-core revision.
## Host-authoritative, ENet (Godot high-level multiplayer), original placeholder
## geometry. All world content is built in code; no external assets.
##
## Playable core (this revision):
##   * View-relative first-person movement with real mouse look (yaw + pitch).
##     Strafe follows the camera basis, not a fixed truck axis.
##   * Real collision instead of clamps: the player is a CharacterBody3D; the
##     truck (AnimatableBody3D), ground, fence rails and crate are solid bodies.
##     Walls block from both sides; the player can leave the truck and walk the
##     bounded site on the ground.
##   * The crate rests on real geometry (ground, road, truck bed) and the truck
##     cannot drive through a crate resting on the road.
##   * Recoverable mistakes: E unload outside the destination zone is REJECTED
##     by the host (visible red feedback, crate stays carried); Q drops the
##     crate at your feet/facing — always pick-up-able, including behind the
##     truck at route start.
##   * The host validates client movement reports (speed, bounds, solid
##     geometry) and corrects offenders authoritatively.

const PORT := 24565
const TICK_HZ := 15.0
const TRUCK_MIN_Z := 0.0          # bounded route start
const TRUCK_MAX_Z := 40.0         # bounded route end
const ROUTE_ARRIVAL_EPSILON := 0.01 # crossing this host threshold snaps to the exact visible endpoint
const TRUCK_SPEED := 3.0
const WALK_SPEED := 2.8
const GRAVITY := 9.8
const MOUSE_SENS := 0.0025
const PITCH_CLAMP := 1.25
# Destination zone (world space AABB): crate must come to rest inside it, on the ground.
const ZONE_MIN := Vector3(-2.0, 0.0, 34.0)
const ZONE_MAX := Vector3(2.0, 0.6, 37.5)
# Visible fence rails bound the walkable site (real collision, not clamps).
const FENCE_X := 10.0
const FENCE_Z_MIN := -8.0
const FENCE_Z_MAX := 55.0
const BOUND_X := FENCE_X - 0.36
const BOUND_Z_MIN := FENCE_Z_MIN + 0.36
const BOUND_Z_MAX := FENCE_Z_MAX - 0.36
# Player body metrics. peer_pos[id] = WORLD position of the capsule center.
# The placeholder truck's doorway/cab were sized for a point player; the cab was
# given real headroom (see _build_truck) so a capsule player fits through.
const PLAYER_RADIUS := 0.29
const PLAYER_HEIGHT := 1.62
const EYE_UP := 0.62              # standing eye height above capsule center
const SEAT_EYE_UP := 0.5          # seated eye height above capsule center
const SEAT_LOCAL := Vector3(0.6, 1.0, 2.6)       # capsule center while seated
const SEAT_EXIT_LOCAL := Vector3(0.0, 1.11, -3.25) # rear cargo threshold, clear of seat/divider
const HANDS_UP := 0.30
const HANDS_FWD := 0.5
const GRAB_REACH := 1.6
const SEAT_REACH := 2.2
const PLACE_FWD := 0.85           # unload/drop point this far ahead of the player
const CRATE_HALF := 0.35
const CRATE_START_LOCAL := Vector3(0.0, 0.65, -2.0)  # resting on the truck bed
# Host movement validation tolerances (checked per 1/TICK_HZ report).
const REPORT_MAX_HOP := 1.0       # ~2.8 m/s walk at 15 Hz, with margin
const REPORT_MAX_FALL := 0.75     # bounded falls only (largest step is 0.3 m)
const REPORT_STRIKES := 3

enum MissionState { IN_PROGRESS, DELIVERED }
enum JourneyStage { LOAD_AND_DRIVE, DRIVING, ARRIVED_SEATED, EXITED_AT_DESTINATION }

var mode := ""                    # "", "host", "client"
var probe := false                # scripted autopilot probe mode
var negative := false             # negative path: on-foot route-zero destination bypass
var mechanics := false            # mechanics assertions: view-relative basis + solid walls
var recovery := false             # recovery path: crate dropped behind truck at start
var shots_dir := ""
var my_peer_id := 0

# Host-authoritative replicated state
var truck_z := 0.0
var mission_state: int = MissionState.IN_PROGRESS
var deliveries := 0
var crate_holder := 0             # peer id, 0 = free
var crate_on_truck := true
var crate_local_pos := CRATE_START_LOCAL  # truck-local while resting on the truck
var crate_world_pos := Vector3.ZERO       # world while resting on the ground
var seat_occupant := 0            # peer id, 0 = free
var journey_stage: int = JourneyStage.LOAD_AND_DRIVE
var journey_driver := 0           # peer that drove loaded cargo to the route end
var drive_dir := 0                # input gathered from occupant (host reads)
var probe_phase := ""             # host probe coordination (harness info, not gameplay)

# Per-peer player state (host-validated; positions are world capsule centers)
var peer_pos := {}                # peer_id -> Vector3
var peer_yaw := {}                # peer_id -> float
var peer_seated := {}             # peer_id -> bool
var peer_strike := {}             # peer_id -> int consecutive invalid reports
var _pending_reports := {}        # peer_id -> Vector3 (validated in the physics step)
var peer_exit_grace_until := {}   # discard stale in-seat reports after authoritative exit teleport

var truck: AnimatableBody3D
var world_body: StaticBody3D
var crate_mesh: MeshInstance3D
var crate_collider: CollisionShape3D
var crate_base_color := Color(0.75, 0.55, 0.25)
var crate_flash_until_ms := 0
var crate_flash_color := Color.WHITE
var players := {}                 # peer_id -> CharacterBody3D
var my_body: CharacterBody3D
var my_yaw := 0.0
var my_pitch := 0.0
var banner_until_ms := 0
var hud_log_lines: Array[String] = []
var hud: CanvasLayer
var hud_role: Label
var hud_state: Label
var hud_controls: Label
var hud_banner: Label
var hud_log: Label
var hud_objective: Label
var hud_action: Label
var hud_crosshair: Label
var cam: Camera3D
var zone_mesh: MeshInstance3D
var truck_solids: Array[Dictionary] = []  # analytic {c,h} truck-local boxes

var _tick_accum := 0.0
var _finished := false
var _truck_block_logged := false
var _next_route_log := 10.0
var _local_drive_input := 0
var _probe_drive := false       # probe autopilot: hold driving input while seated
var _shot_timer := 0.0
var _shot_seq := 0
var _shot_dup := 0
var _p := {}                      # probe state

# Autopilot (probe modes): waypoint following through the real movement pipeline.
var _auto_wps: Array[Vector3] = []
var _auto_i := 0
var _auto_face := 999.0           # explicit facing once arrived
var _auto_crate := false          # seek a standing distance from the crate

# ---------------- lifecycle ----------------

func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		if a == "--host":
			mode = "host"
		elif a == "--client":
			mode = "client"
		elif a == "--probe":
			probe = true
		elif a == "--negative":
			negative = true
		elif a == "--mechanics":
			mechanics = true
		elif a == "--recovery":
			recovery = true
		elif a.begins_with("--shots="):
			shots_dir = a.substr(8)
	_build_world()
	_setup_multiplayer_signals()
	if mode == "host":
		start_host()
	elif mode == "client":
		start_client()
	else:
		cam.global_position = Vector3(9, 5, truck_z - 9)
		cam.look_at(Vector3(0, 1, truck_z), Vector3.UP)
		_log("Idle. Launch with --host or --client (see run-demo.sh).")
		_log("Controls: WASD move (view-relative) · mouse look · E interact (grab / seat / unload in zone) · Q drop crate · Esc free mouse")

func _physics_process(delta: float) -> void:
	if _finished:
		return
	_sim_local_player(delta)
	if multiplayer.is_server() and multiplayer.multiplayer_peer:
		_host_validate_reports()
		_host_truck_physics(delta)
	truck.position.z = truck_z
	_tick_accum += delta
	if _tick_accum >= 1.0 / TICK_HZ:
		_tick_accum = 0.0
		_network_tick()

func _process(delta: float) -> void:
	if _finished:
		return
	# Fallback body spawn in case the first authoritative state arrived before
	# connected_to_server set our peer id.
	if mode == "client" and my_peer_id != 0 and my_body == null:
		my_body = _ensure_player(my_peer_id)
		my_body.global_position = _spawn_pos_for(my_peer_id)
		my_yaw = PI
		peer_pos[my_peer_id] = my_body.global_position
		peer_yaw[my_peer_id] = my_yaw
	_update_visuals()
	_update_hud()
	if probe:
		_probe_step()
	if shots_dir != "" and DisplayServer.get_name() != "headless":
		_shot_timer += delta
		if _shot_timer >= 5.0 and _shot_seq < 120:
			_shot_timer = 0.0
			take_screenshot("seq_%03d" % _shot_seq)
			_shot_seq += 1


func _unhandled_input(event: InputEvent) -> void:
	if my_peer_id == 0:
		return
	if event is InputEventMouseMotion and Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		my_yaw = wrapf(my_yaw - event.relative.x * MOUSE_SENS, -PI, PI)
		my_pitch = clampf(my_pitch - event.relative.y * MOUSE_SENS, -PITCH_CLAMP, PITCH_CLAMP)
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if DisplayServer.get_name() != "headless":
			Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	elif event is InputEventKey and event.pressed and not event.echo:
		if event.physical_keycode == KEY_ESCAPE:
			if DisplayServer.get_name() != "headless":
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
		elif not probe and event.physical_keycode == KEY_E:
			_interact()
		elif not probe and event.physical_keycode == KEY_Q:
			_interact_drop()

# ---------------- world construction (original placeholder geometry) ----------------

func _box(size: Vector3, color: Color, name_: String, parent: Node3D) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var bm := BoxMesh.new()
	bm.size = size
	mi.mesh = bm
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mi.material_override = mat
	mi.name = name_
	parent.add_child(mi)
	return mi

func _solid(body: PhysicsBody3D, size: Vector3, pos: Vector3) -> void:
	var cs := CollisionShape3D.new()
	var sh := BoxShape3D.new()
	sh.size = size
	cs.shape = sh
	cs.position = pos
	body.add_child(cs)

func _label(text: String, pos: Vector3, parent: Node3D) -> void:
	var l := Label3D.new()
	l.text = text
	# World labels guide the walking skeleton; they must not fill the player's
	# view now that the review target is first-person.
	l.font_size = 32
	l.pixel_size = 0.001
	l.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	l.modulate = Color(1, 1, 1, 0.95)
	l.position = pos
	parent.add_child(l)

func _build_world() -> void:
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-55, 30, 0)
	sun.light_energy = 1.2
	add_child(sun)
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.45, 0.6, 0.85)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.7, 0.75, 0.8)
	env.environment = e
	add_child(env)

	world_body = StaticBody3D.new()
	world_body.name = "WorldCollision"
	world_body.collision_layer = 1
	world_body.collision_mask = 0
	add_child(world_body)

	var ground := _box(Vector3(30, 0.2, 80), Color(0.35, 0.5, 0.3), "Ground", self)
	ground.position = Vector3(0, -0.1, 20)
	_solid(world_body, Vector3(30, 0.2, 80), Vector3(0, -0.1, 20))
	var road := _box(Vector3(6, 0.05, 50), Color(0.25, 0.25, 0.28), "Road", self)
	road.position = Vector3(0, 0.03, 20)
	_label("SWIFT ARRIVAL — BOUNDED ROUTE", Vector3(0, 2.0, -3.0), self)

	# Visible site boundary rails: real collision, walkable area is honestly bounded.
	var rail_col := Color(0.55, 0.45, 0.35)
	var rail := _box(Vector3(2.0 * FENCE_X + 0.4, 1.0, 0.2), rail_col, "FenceBack", self)
	rail.position = Vector3(0, 0.5, FENCE_Z_MIN)
	_solid(world_body, Vector3(2.0 * FENCE_X + 0.4, 1.0, 0.2), Vector3(0, 0.5, FENCE_Z_MIN))
	rail = _box(Vector3(2.0 * FENCE_X + 0.4, 1.0, 0.2), rail_col, "FenceFar", self)
	rail.position = Vector3(0, 0.5, FENCE_Z_MAX)
	_solid(world_body, Vector3(2.0 * FENCE_X + 0.4, 1.0, 0.2), Vector3(0, 0.5, FENCE_Z_MAX))
	for sx in [-1.0, 1.0]:
		rail = _box(Vector3(0.2, 1.0, FENCE_Z_MAX - FENCE_Z_MIN + 0.4), rail_col, "FenceSide", self)
		rail.position = Vector3(sx * FENCE_X, 0.5, (FENCE_Z_MIN + FENCE_Z_MAX) / 2.0)
		_solid(world_body, Vector3(0.2, 1.0, FENCE_Z_MAX - FENCE_Z_MIN + 0.4), Vector3(sx * FENCE_X, 0.5, (FENCE_Z_MIN + FENCE_Z_MAX) / 2.0))

	zone_mesh = _box(ZONE_MAX - ZONE_MIN + Vector3(0, 0.1, 0), Color(0.9, 0.8, 0.1), "DestinationZone", self)
	zone_mesh.position = (ZONE_MIN + ZONE_MAX) / 2.0 + Vector3(0, 0.05, 0)
	zone_mesh.material_override.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	zone_mesh.material_override.albedo_color.a = 0.45
	_label("DESTINATION ZONE", Vector3(0, 1.6, (ZONE_MIN.z + ZONE_MAX.z) / 2.0), self)
	# High-contrast placeholder beacon keeps the compact route's destination
	# visible from the truck ramp without introducing external art.
	for bx in [-2.15, 2.15]:
		var beacon := _box(Vector3(0.18, 3.2, 0.18), Color(1.0, 0.82, 0.08), "DestinationBeacon", self)
		beacon.position = Vector3(bx, 1.6, (ZONE_MIN.z + ZONE_MAX.z) / 2.0)

	_build_truck()

	cam = Camera3D.new()
	add_child(cam)
	cam.fov = 82.0
	_build_first_person_hands()

	hud = CanvasLayer.new()
	add_child(hud)
	hud_role = Label.new()
	hud_role.position = Vector2(16, 16)
	hud_role.add_theme_font_size_override("font_size", 22)
	hud.add_child(hud_role)
	hud_state = Label.new()
	hud_state.position = Vector2(16, 52)
	hud_state.add_theme_font_size_override("font_size", 20)
	hud.add_child(hud_state)
	hud_objective = Label.new()
	hud_objective.position = Vector2(16, 92)
	hud_objective.add_theme_font_size_override("font_size", 24)
	hud_objective.add_theme_color_override("font_color", Color(1.0, 0.92, 0.35))
	hud.add_child(hud_objective)
	hud_action = Label.new()
	hud_action.position = Vector2(16, 126)
	hud_action.add_theme_font_size_override("font_size", 21)
	hud_action.add_theme_color_override("font_color", Color(0.6, 1.0, 0.7))
	hud.add_child(hud_action)
	hud_log = Label.new()
	hud_log.position = Vector2(16, 560)
	hud_log.add_theme_font_size_override("font_size", 13)
	hud.add_child(hud_log)
	hud_controls = Label.new()
	hud_controls.position = Vector2(16, 688)
	hud_controls.add_theme_font_size_override("font_size", 14)
	hud_controls.add_theme_color_override("font_color", Color(1, 1, 1, 0.85))
	hud_controls.text = "WASD move / drive · MOUSE look · E use shown action · Q safely drop / recover · Esc free mouse"
	hud.add_child(hud_controls)
	hud_banner = Label.new()
	hud_banner.position = Vector2(190, 205)
	hud_banner.add_theme_font_size_override("font_size", 26)
	hud.add_child(hud_banner)
	hud_crosshair = Label.new()
	hud_crosshair.position = Vector2(632, 344)
	hud_crosshair.text = "+"
	hud_crosshair.add_theme_font_size_override("font_size", 24)
	hud.add_child(hud_crosshair)

func _build_truck() -> void:
	truck = AnimatableBody3D.new()
	truck.name = "Truck"
	truck.sync_to_physics = true
	truck.collision_layer = 1
	truck.collision_mask = 0
	add_child(truck)
	truck.position = Vector3(0, 0, truck_z)

	var floor_size := Vector3(2.4, 0.3, 7.4)
	var floor_m := _box(floor_size, Color(0.5, 0.35, 0.2), "TruckFloor", truck)
	floor_m.position = Vector3(0, 0.15, 0)
	_solid(truck, floor_size, Vector3(0, 0.15, 0))
	for wz in [3.2, -3.0]:
		for wx in [-1.35, 1.35]:
			var w := _box(Vector3(0.3, 0.6, 0.6), Color(0.1, 0.1, 0.1), "Wheel", truck)
			w.position = Vector3(wx, 0.3, wz)
			_solid(truck, Vector3(0.3, 0.6, 0.6), Vector3(wx, 0.3, wz))
			truck_solids.append({"c": Vector3(wx, 0.3, wz), "h": Vector3(0.15, 0.3, 0.3)})
	var dash_size := Vector3(2.4, 0.9, 0.3)
	var dash := _box(dash_size, Color(0.85, 0.25, 0.15), "CabDash", truck)
	dash.position = Vector3(0, 0.75, 3.55)
	_solid(truck, dash_size, Vector3(0, 0.75, 3.55))
	truck_solids.append({"c": Vector3(0, 0.75, 3.55), "h": Vector3(1.2, 0.45, 0.15)})
	for sx in [-1.2, 1.2]:
		# Cab walls extended up (1.6 -> 2.05 tall) so the cab roof clears a
		# capsule player: the placeholder cab was sized for a point player.
		var cw := _box(Vector3(0.15, 2.05, 2.7), Color(0.85, 0.25, 0.15), "CabWall", truck)
		cw.position = Vector3(sx, 1.325, 2.35)
		_solid(truck, Vector3(0.15, 2.05, 2.7), Vector3(sx, 1.325, 2.35))
		truck_solids.append({"c": Vector3(sx, 1.325, 2.35), "h": Vector3(0.075, 1.025, 1.35)})
	var roof := _box(Vector3(2.4, 0.15, 2.7), Color(0.85, 0.25, 0.15), "CabRoof", truck)
	roof.position = Vector3(0, 2.425, 2.35)
	_solid(truck, Vector3(2.4, 0.15, 2.7), Vector3(0, 2.425, 2.35))
	# Divider wall between cab and cargo with an open doorway (|x| < 0.6 at z = 1.0)
	for sx in [-0.95, 0.95]:
		# Divider walls extended to meet the raised top; doorway stays |x| < 0.6
		# but is now tall enough (clear 0.3..2.05) for a capsule player.
		var dw := _box(Vector3(0.7, 1.75, 0.15), Color(0.85, 0.25, 0.15), "DividerWall", truck)
		dw.position = Vector3(sx, 1.175, 1.0)
		_solid(truck, Vector3(0.7, 1.75, 0.15), Vector3(sx, 1.175, 1.0))
		truck_solids.append({"c": Vector3(sx, 1.175, 1.0), "h": Vector3(0.35, 0.875, 0.075)})
	var dtop := _box(Vector3(2.4, 0.3, 0.15), Color(0.85, 0.25, 0.15), "DividerTop", truck)
	dtop.position = Vector3(0, 2.2, 1.0)
	_solid(truck, Vector3(2.4, 0.3, 0.15), Vector3(0, 2.2, 1.0))
	truck_solids.append({"c": Vector3(0, 2.2, 1.0), "h": Vector3(1.2, 0.15, 0.075)})
	for sx in [-1.2, 1.2]:
		var sw := _box(Vector3(0.15, 2.0, 4.7), Color(0.3, 0.45, 0.7), "CargoWall", truck)
		sw.position = Vector3(sx, 1.3, -1.35)
		_solid(truck, Vector3(0.15, 2.0, 4.7), Vector3(sx, 1.3, -1.35))
		truck_solids.append({"c": Vector3(sx, 1.3, -1.35), "h": Vector3(0.075, 1.0, 2.35)})
	var croof := _box(Vector3(2.4, 0.15, 4.7), Color(0.3, 0.45, 0.7), "CargoRoof", truck)
	croof.position = Vector3(0, 2.4, -1.35)
	_solid(truck, Vector3(2.4, 0.15, 4.7), Vector3(0, 2.4, -1.35))
	_label("CARGO", Vector3(1.0, 2.2, -1.0), truck)
	_label("CAB", Vector3(0.0, 2.2, 2.4), truck)

	# Loading ramp at the open rear: real sloped collision, so the site can be
	# entered and left on foot (and a dropped crate can be recovered). The foot
	# is buried below the road surface so a capsule only ever meets the slope
	# itself — an exposed leading edge behaves like a wall and blocks climbing.
	var slope := 0.35  # rise per unit run (~19.4°, walkable)
	var s_z := -3.7    # top of ramp EXACTLY at the bed floor edge, flush with its top
	var s_y := 0.3     # (any lower exposes a lip of the floor face that wedges the capsule)
	var e_z := -4.9    # foot, buried below the road surface
	var e_y := s_y - slope * (s_z - e_z)
	var run2 := s_z - e_z
	var rise2 := s_y - e_y
	var ramp_len := sqrt(run2 * run2 + rise2 * rise2)
	var ramp := _box(Vector3(2.0, 0.1, ramp_len), Color(0.6, 0.42, 0.24), "LoadingRamp", truck)
	var mid := Vector3(0, (s_y + e_y) / 2.0, (s_z + e_z) / 2.0)
	var n := Vector3(0, run2, rise2).normalized()
	ramp.position = mid - n * 0.05
	ramp.rotation_degrees = Vector3(-rad_to_deg(atan(slope)), 0, 0)
	# The collision shape must carry the mesh's FULL transform (rotation AND
	# position): _solid()'s position-only path produced an unrotated flat plate
	# that ended at the bed lip and blocked the ramp climb (owner evidence).
	var rcs := CollisionShape3D.new()
	var rsh := BoxShape3D.new()
	rsh.size = Vector3(2.0, 0.1, ramp_len)
	rcs.shape = rsh
	rcs.position = ramp.position
	rcs.rotation_degrees = ramp.rotation_degrees
	truck.add_child(rcs)
	# Analytic AABB for host validation: conservative bounds of the tilted slab.
	truck_solids.append({"c": ramp.position, "h": Vector3(1.0, 0.05 + 0.62 * slope / sqrt(1 + slope * slope), ramp_len / 2.0)})

	var seat_marker := Node3D.new()
	seat_marker.name = "DriverSeat"
	truck.add_child(seat_marker)
	seat_marker.position = Vector3(0.6, 0.6, 2.6)
	var seat_size := Vector3(0.7, 0.5, 0.7)
	var seat_box := _box(seat_size, Color(0.9, 0.75, 0.2), "Seat", truck)
	seat_box.position = seat_marker.position + Vector3(0, -0.15, 0)
	_solid(truck, seat_size, seat_box.position)
	_label("DRIVER", Vector3(0.6, 1.4, 2.6), truck)
	_label("RAMP", Vector3(0, 1.2, -4.1), truck)

	crate_mesh = _box(Vector3(0.7, 0.7, 0.7), crate_base_color, "Crate", self)
	_label("CRATE", Vector3(0, 0.05, 0), crate_mesh)
	var crate_body := StaticBody3D.new()
	crate_body.name = "CrateBody"
	crate_body.collision_layer = 2
	crate_body.collision_mask = 0
	crate_mesh.add_child(crate_body)
	crate_collider = CollisionShape3D.new()
	var csh := BoxShape3D.new()
	csh.size = Vector3(0.7, 0.7, 0.7)
	crate_collider.shape = csh
	crate_body.add_child(crate_collider)

func _ensure_player(id: int) -> CharacterBody3D:
	if players.has(id):
		return players[id]
	var b := CharacterBody3D.new()
	b.name = "Player_%d" % id
	b.collision_layer = 4
	# Players collide with the world (1) and the crate (2) but not each other:
	# body-blocking in a doorway or at the driver seat would be a soft-lock.
	b.collision_mask = 1 | 2
	add_child(b)
	var cs := CollisionShape3D.new()
	var cap := CapsuleShape3D.new()
	cap.radius = PLAYER_RADIUS
	cap.height = PLAYER_HEIGHT
	cs.shape = cap
	b.add_child(cs)
	# Visuals hang under a child node so the local player can hide their own body.
	var vis := Node3D.new()
	vis.name = "Visual"
	b.add_child(vis)
	var body_col := Color(0.2, 0.7, 0.4) if id == 1 else Color(0.7, 0.4, 0.8)
	var body := _box(Vector3(0.5, 1.3, 0.4), body_col, "Body", vis)
	body.position = Vector3(0, 0.15, 0)
	var head := _box(Vector3(0.35, 0.35, 0.35), Color(0.95, 0.85, 0.7), "Head", vis)
	head.position = Vector3(0, 1.0, 0)
	# Two visible world-space hands
	var hl := _box(Vector3(0.18, 0.18, 0.28), Color(0.95, 0.85, 0.7), "HandL", vis)
	hl.position = Vector3(-0.35, 0.10, 0.45)
	var hr := _box(Vector3(0.18, 0.18, 0.28), Color(0.95, 0.85, 0.7), "HandR", vis)
	hr.position = Vector3(0.35, 0.10, 0.45)
	_label("P%d" % id, Vector3(0, 1.55, 0), vis)
	if id == my_peer_id:
		vis.hide()  # first person: the local player sees the camera hand rig instead
	players[id] = b
	return b

func _build_first_person_hands() -> void:
	# The local player sees their own two delivery gloves. Remote peers still
	# receive the complete employee body assembled above. This is deliberately
	# simple placeholder geometry, but it keeps the first-person interaction
	# contract visible in the live review instead of only in host logs.
	var local_hand_rig := Node3D.new()
	local_hand_rig.name = "FirstPersonHands"
	cam.add_child(local_hand_rig)
	for side in [-1.0, 1.0]:
		var arm := _box(Vector3(0.08, 0.08, 0.42), Color(0.16, 0.29, 0.5), "DeliveryArm", local_hand_rig)
		arm.position = Vector3(side * 0.24, -0.30, -1.25)
		arm.rotation_degrees = Vector3(17.0, side * 8.0, side * -12.0)
		var glove := _box(Vector3(0.14, 0.12, 0.18), Color(0.98, 0.75, 0.19), "DeliveryGlove", local_hand_rig)
		glove.position = Vector3(side * 0.30, -0.37, -1.56)

# ---------------- basis helpers ----------------

func _fwd(yaw: float) -> Vector3:
	return Vector3(-sin(yaw), 0.0, -cos(yaw))

func _rgt(yaw: float) -> Vector3:
	return Vector3(cos(yaw), 0.0, -sin(yaw))

func _yaw_toward(from: Vector3, to: Vector3) -> float:
	return atan2(-(to.x - from.x), -(to.z - from.z))

func _hands_world_pos(peer: int) -> Vector3:
	var pos: Vector3 = peer_pos.get(peer, Vector3(0, 1.11, truck_z - 3.0))
	if peer_seated.get(peer, false):
		pos = truck.to_global(SEAT_LOCAL)
	return pos + Vector3(0, HANDS_UP, 0) + _fwd(peer_yaw.get(peer, 0.0)) * HANDS_FWD

func _crate_world_pos_now() -> Vector3:
	if crate_holder != 0:
		return _hands_world_pos(crate_holder) + _fwd(peer_yaw.get(crate_holder, 0.0)) * 0.45
	if crate_on_truck:
		return truck.to_global(crate_local_pos)
	return crate_world_pos

# ---------------- networking ----------------

func _setup_multiplayer_signals() -> void:
	multiplayer.peer_connected.connect(_on_peer_connected)
	multiplayer.peer_disconnected.connect(_on_peer_disconnected)
	multiplayer.connected_to_server.connect(_on_connected_to_server)
	multiplayer.connection_failed.connect(_on_connection_failed)
	multiplayer.server_disconnected.connect(_on_server_disconnected)

func start_host() -> void:
	var peer := ENetMultiplayerPeer.new()
	var err := peer.create_server(PORT, 4)
	if err != OK:
		_log("ENet HOST FAILED to bind port %d: %s" % [PORT, error_string(err)])
		_finish(false)
		return
	multiplayer.multiplayer_peer = peer
	my_peer_id = multiplayer.get_unique_id()
	DisplayServer.window_set_title("Swift Arrival Dogfood 4 — HOST (peer %d)" % my_peer_id)
	# No startup pointer capture: in a two-window local session only one window
	# can hold the X grab. Each window captures when clicked (see _unhandled_input),
	# and look works uncaptured too.
	my_body = _ensure_player(1)
	my_body.global_position = Vector3(-0.3, 1.11, truck_z + 2.0)  # host spawns in the cab (clear of the seat)
	my_yaw = 0.0  # facing the cargo doorway
	peer_pos[1] = my_body.global_position
	peer_yaw[1] = my_yaw
	peer_seated[1] = false
	_log("ENet: server listening on 127.0.0.1:%d — HOST is peer %d" % [PORT, my_peer_id])

func start_client() -> void:
	var peer := ENetMultiplayerPeer.new()
	var err := peer.create_client("127.0.0.1", PORT)
	if err != OK:
		_log("ENet CLIENT FAILED to start: %s" % error_string(err))
		_finish(false)
		return
	multiplayer.multiplayer_peer = peer
	_log("ENet: client connecting to 127.0.0.1:%d ..." % PORT)

func _caller_id() -> int:
	var c := multiplayer.get_remote_sender_id()
	return 1 if c == 0 else c # 0 = local call made on the host itself

func _spawn_pos_for(id: int) -> Vector3:
	# Joining players spawn in the cargo area, facing the cab.
	return Vector3(0, 1.11, truck_z - 3.0)

func _on_peer_connected(id: int) -> void:
	_log("ENet: peer_connected id=%d (transport-level connection evidence)" % id)
	if multiplayer.is_server():
		_ensure_player(id)
		peer_pos[id] = _spawn_pos_for(id)
		peer_yaw[id] = PI  # facing the cab (+Z)
		peer_seated[id] = false
		_sync_state.rpc_id(id, truck_z, mission_state, deliveries, crate_holder, crate_on_truck, crate_local_pos, crate_world_pos, seat_occupant, journey_stage, probe_phase)
		for pid in peer_pos.keys():
			_sync_peer.rpc_id(id, pid, peer_pos[pid], peer_yaw.get(pid, 0.0), peer_seated.get(pid, false))
		_log("HOST: sent authoritative world state to new peer %d" % id)

func _on_peer_disconnected(id: int) -> void:
	_log("ENet: peer_disconnected id=%d" % id)
	if multiplayer.is_server():
		if crate_holder == id:
			# A leaving holder puts the crate down where they stood — never lost.
			var rest := _crate_rest(peer_pos.get(id, Vector3.ZERO).x, peer_pos.get(id, Vector3.ZERO).z)
			crate_holder = 0
			_place_crate(peer_pos.get(id, Vector3.ZERO), rest)
			_log("HOST: holder %d left; crate placed at world %s (recoverable)" % [id, str(crate_world_pos if not crate_on_truck else crate_local_pos)])
		if seat_occupant == id:
			seat_occupant = 0
		peer_pos.erase(id)
		peer_yaw.erase(id)
		peer_seated.erase(id)
		peer_strike.erase(id)
		_pending_reports.erase(id)
	if players.has(id):
		players[id].queue_free()
		players.erase(id)

func _on_connected_to_server() -> void:
	my_peer_id = multiplayer.get_unique_id()
	DisplayServer.window_set_title("Swift Arrival Dogfood 4 — CLIENT (peer %d)" % my_peer_id)
	# No startup pointer capture (see start_host); click this window to capture.
	_log("ENet: CONNECTED to host as CLIENT peer %d" % my_peer_id)

func _on_connection_failed() -> void:
	_log("ENet: connection FAILED (no host at 127.0.0.1:%d)" % PORT)
	_finish(false)

func _on_server_disconnected() -> void:
	_log("ENet: server disconnected")
	if probe and not _finished and _p.has("passed"):
		_log("PROBE RESULT: PASS (clean shutdown observed after local PASS)")
	_finish(mission_state == MissionState.DELIVERED or _p.has("passed"))

func _network_tick() -> void:
	if mode == "client" and my_peer_id != 0 and my_body != null:
		_report_move.rpc_id(1, my_body.global_position, my_yaw)
		if seat_occupant == my_peer_id:
			_request_drive.rpc_id(1, _local_drive_input)
	if multiplayer.is_server() and multiplayer.multiplayer_peer:
		_sync_state.rpc(truck_z, mission_state, deliveries, crate_holder, crate_on_truck, crate_local_pos, crate_world_pos, seat_occupant, journey_stage, probe_phase)
		for id in peer_pos.keys():
			_sync_peer.rpc(id, peer_pos[id], peer_yaw.get(id, 0.0), peer_seated.get(id, false))

@rpc("any_peer", "call_local", "unreliable_ordered")
func _report_move(pos: Vector3, yaw: float) -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if caller == 1 or not peer_pos.has(caller):
		return
	peer_yaw[caller] = yaw
	_pending_reports[caller] = pos

@rpc("authority", "call_local", "unreliable_ordered")
func _sync_peer(id: int, pos: Vector3, yaw: float, seated: bool) -> void:
	peer_pos[id] = pos
	peer_yaw[id] = yaw
	peer_seated[id] = seated
	_ensure_player(id)
	if id == my_peer_id:
		return  # the local simulation owns its own body between corrections

@rpc("authority", "call_local", "unreliable_ordered")
func _sync_state(t_z: float, ms: int, dels: int, holder: int, on_truck: bool, c_local: Vector3, c_world: Vector3, seat: int, journey: int, phase: String) -> void:
	var was := mission_state
	truck_z = t_z
	deliveries = dels
	crate_holder = holder
	crate_on_truck = on_truck
	crate_local_pos = c_local
	crate_world_pos = c_world
	seat_occupant = seat
	journey_stage = journey
	probe_phase = phase
	if ms != mission_state:
		mission_state = ms
		if mission_state == MissionState.DELIVERED and was != MissionState.DELIVERED:
			_log("MISSION: DELIVERY COMPLETED (host-authoritative) — visible on this %s (peer %d)" % [mode, my_peer_id])
	# Late body spawn: the client builds its own body once the first
	# authoritative state (including truck position) has arrived.
	if mode == "client" and my_peer_id != 0 and my_body == null:
		my_body = _ensure_player(my_peer_id)
		my_body.global_position = _spawn_pos_for(my_peer_id)
		my_yaw = PI
		peer_pos[my_peer_id] = my_body.global_position
		peer_yaw[my_peer_id] = my_yaw

@rpc("authority", "call_local", "reliable")
func _correct_pos(pos: Vector3, reason: String) -> void:
	if my_peer_id == 0 or my_body == null:
		return
	my_body.global_position = pos
	my_body.velocity = Vector3.ZERO
	peer_pos[my_peer_id] = pos
	if reason == "exited at rear cargo threshold":
		seat_occupant = 0
		peer_seated[my_peer_id] = false
	_log("HOST corrected your position (%s)" % reason)

@rpc("authority", "call_local", "reliable")
func _feedback(kind: String, msg: String, shot: String) -> void:
	_log("FEEDBACK[%s]: %s" % [kind, msg])
	if kind == "reject":
		_p["saw_reject"] = true  # probe evidence: rejection was visible to the requester
	if hud_banner:
		hud_banner.text = msg
		hud_banner.add_theme_color_override("font_color",
			Color(1.0, 0.35, 0.35) if kind == "reject" else (Color(0.5, 1.0, 0.6) if kind == "ok" else Color(1.0, 0.9, 0.45)))
		banner_until_ms = Time.get_ticks_msec() + 3500
	if shot != "":
		take_screenshot(shot)

@rpc("any_peer", "call_local", "reliable")
func _probe_signal(tag: String) -> void:
	if not multiplayer.is_server():
		return
	_p["sig_" + tag] = Time.get_ticks_msec() / 1000.0
	_log("PROBE(host): received client signal '%s'" % tag)

func _send_feedback(target: int, kind: String, msg: String, shot: String = "") -> void:
	if target == 0:
		_feedback.rpc(kind, msg, shot)
	else:
		_feedback.rpc_id(target, kind, msg, shot)

# ---- interaction requests (peers -> host; host validates and resolves) ----

@rpc("any_peer", "call_local", "reliable")
func _request_grab() -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if crate_holder != 0:
		_log("HOST: REJECTED grab from %d (crate already held by %d)" % [caller, crate_holder])
		_send_feedback(caller, "reject", "REJECTED: crate already held by peer %d" % crate_holder, "ev_reject_grab")
		return
	var hands := _hands_world_pos(caller)
	var crate_w := _crate_world_pos_now()
	if hands.distance_to(crate_w) > GRAB_REACH:
		_log("HOST: REJECTED grab from %d (hands %.2fm from crate, limit %.1fm)" % [caller, hands.distance_to(crate_w), GRAB_REACH])
		_send_feedback(caller, "reject", "REJECTED: too far from the crate (%.1f m limit)" % GRAB_REACH, "ev_reject_grab")
		return
	crate_holder = caller
	crate_on_truck = false
	_flash_crate(Color(0.3, 1.0, 0.4))
	_log("HOST: RESOLVED grab by peer %d (crate now held)" % caller)
	_send_feedback(caller, "ok", "Picked up the crate (E unload in zone · Q drop)", "ev_grab")

@rpc("any_peer", "call_local", "reliable")
func _request_unload() -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if crate_holder != caller:
		_log("HOST: REJECTED unload from %d (not holder)" % caller)
		_send_feedback(caller, "reject", "REJECTED: you are not carrying the crate", "ev_reject_unload")
		return
	var pos: Vector3 = peer_pos.get(caller, Vector3.ZERO)
	var drop := Vector3(pos.x + _fwd(peer_yaw.get(caller, 0.0)).x * PLACE_FWD, 0.0, pos.z + _fwd(peer_yaw.get(caller, 0.0)).z * PLACE_FWD)
	if drop.x < ZONE_MIN.x or drop.x > ZONE_MAX.x or drop.z < ZONE_MIN.z or drop.z > ZONE_MAX.z:
		_p["rejected_unload"] = true
		_log("HOST: REJECTED unload from %d (drop point outside destination zone at %s)" % [caller, drop])
		_send_feedback(caller, "reject", "REJECTED: unload outside the destination zone — carry it to the marked zone", "ev_reject_unload")
		return
	if journey_stage != JourneyStage.EXITED_AT_DESTINATION:
		_p["rejected_journey_bypass"] = true
		_log("HOST: REJECTED unload from %d (truck journey incomplete: stage=%d route=%.1f)" % [caller, journey_stage, truck_z])
		_send_feedback(caller, "reject", "REJECTED: truck journey incomplete — drive the loaded truck to 40 m, then exit there", "ev_reject_journey")
		return
	var rest := _crate_rest(drop.x, drop.z)
	if rest["on_truck"]:
		_p["rejected_unload"] = true
		_log("HOST: REJECTED unload from %d (truck is over the drop point)" % caller)
		_send_feedback(caller, "reject", "REJECTED: the truck is over that spot — unload onto the ground", "ev_reject_unload")
		return
	if not rest["valid"]:
		_p["rejected_unload"] = true
		_log("HOST: REJECTED unload from %d (no resting place at %s)" % [caller, drop])
		_send_feedback(caller, "reject", "REJECTED: no room to set the crate down there", "ev_reject_unload")
		return
	crate_holder = 0
	crate_on_truck = false
	crate_world_pos = Vector3(drop.x, rest["y"], drop.z)
	_log("HOST: RESOLVED unload by peer %d — crate at world %s" % [caller, crate_world_pos])
	_send_feedback(caller, "ok", "Unloaded the crate in the destination zone", "ev_unload")
	_check_delivery()

@rpc("any_peer", "call_local", "reliable")
func _request_drop() -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if crate_holder != caller:
		_log("HOST: REJECTED drop from %d (not holder)" % caller)
		_send_feedback(caller, "reject", "REJECTED: you are not carrying the crate", "ev_reject_drop")
		return
	var pos: Vector3 = peer_pos.get(caller, Vector3.ZERO)
	var f := _fwd(peer_yaw.get(caller, 0.0))
	var drop := Vector3(pos.x + f.x * PLACE_FWD, 0.0, pos.z + f.z * PLACE_FWD)
	var rest := _crate_rest(drop.x, drop.z)
	if not rest["valid"]:
		drop = Vector3(pos.x, 0.0, pos.z)  # fall back to the player's own standing spot
		rest = _crate_rest(drop.x, drop.z)
	crate_holder = 0
	_place_crate(drop, rest)
	_flash_crate(Color(1.0, 0.7, 0.2))
	_log("HOST: RESOLVED drop by peer %d — crate at %s (recoverable)" % [caller, str(crate_world_pos if not crate_on_truck else crate_local_pos)])
	_send_feedback(caller, "ok", "Dropped the crate — walk up and press E to pick it up again", "ev_drop")

func _place_crate(drop: Vector3, rest: Dictionary) -> void:
	if rest["on_truck"]:
		crate_on_truck = true
		crate_local_pos = Vector3(drop.x, rest["y"], drop.z - truck_z)
	else:
		crate_on_truck = false
		crate_world_pos = Vector3(drop.x, rest["y"], drop.z)

@rpc("any_peer", "call_local", "reliable")
func _request_seat(enter: bool) -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if enter:
		if seat_occupant != 0:
			_log("HOST: REJECTED seat by %d (occupied by %d)" % [caller, seat_occupant])
			_send_feedback(caller, "reject", "REJECTED: driver seat occupied by peer %d" % seat_occupant, "ev_reject_seat")
			return
		if _hands_world_pos(caller).distance_to(truck.to_global(Vector3(0.6, 0.6, 2.6))) > SEAT_REACH:
			_log("HOST: REJECTED seat by %d (too far from driver position)" % caller)
			_send_feedback(caller, "reject", "REJECTED: too far from the driver seat", "ev_reject_seat")
			return
		seat_occupant = caller
		peer_seated[caller] = true
		peer_pos[caller] = truck.to_global(SEAT_LOCAL)
		if recovery and _p.has("recovery_early_exit"):
			_p["recovery_reseat"] = true
			_log("PROBE(host,rec): natural driver-seat RE-ENTRY accepted after early exit")
		_log("HOST: peer %d occupies DRIVER position" % caller)
		_send_feedback(caller, "info", "SEATED — W/S drive · E exits at the rear cargo threshold", "ev_seat")
	else:
		if seat_occupant != caller:
			_log("HOST: REJECTED seat exit by %d (driver is %d)" % [caller, seat_occupant])
			_send_feedback(caller, "reject", "REJECTED: you are not in the driver seat", "ev_reject_seat")
			return
		if journey_stage == JourneyStage.ARRIVED_SEATED and journey_driver == caller:
			journey_stage = JourneyStage.EXITED_AT_DESTINATION
			_log("MISSION: loaded journey advanced through valid route-end seat exit by peer %d" % caller)
		elif recovery and journey_stage == JourneyStage.DRIVING:
			_p["recovery_early_exit"] = true
			_log("PROBE(host,rec): incomplete route exit observed at z=%.2f; journey remains recoverable" % truck_z)
		seat_occupant = 0
		drive_dir = 0  # latched input must not outlive its driver
		peer_seated[caller] = false
		peer_pos[caller] = truck.to_global(SEAT_EXIT_LOCAL)
		peer_exit_grace_until[caller] = Time.get_ticks_msec() + 1000
		_correct_pos.rpc_id(caller, truck.to_global(SEAT_EXIT_LOCAL), "exited at rear cargo threshold")
		_log("HOST: peer %d left driver position at rear cargo threshold" % caller)
		var exit_msg := "EXITED AT DESTINATION — unload the crate in the yellow zone" if journey_stage == JourneyStage.EXITED_AT_DESTINATION else "EXITED EARLY — re-enter the DRIVER seat through the cargo doorway and continue to 40 m"
		_send_feedback(caller, "info", exit_msg, "ev_exit")

@rpc("any_peer", "call_local", "unreliable")
func _request_drive(dir: int) -> void:
	if not multiplayer.is_server():
		return
	var caller := _caller_id()
	if seat_occupant == caller:
		drive_dir = dir
		if not _p.has("drive_seen") and dir != 0:
			_p["drive_seen"] = true
			_log("HOST: received drive input from driver %d" % caller)

# ---------------- host simulation ----------------

func _host_truck_physics(delta: float) -> void:
	if seat_occupant == 1:
		drive_dir = _local_drive_input
	# drive_dir is LATCHED: the remote driver refreshes it at TICK_HZ, so it must
	# survive between reports (zeroing it every physics frame throttled the truck
	# to TICK_HZ/physics_hz of its speed).
	var proposed := clampf(truck_z + drive_dir * TRUCK_SPEED * delta, TRUCK_MIN_Z, TRUCK_MAX_Z)
	var blocked := false
	if crate_holder == 0 and not crate_on_truck and crate_world_pos.y < 1.6:
		# A crate resting on the road is solid cargo: the truck may not drive through it.
		if absf(crate_world_pos.x) <= 1.5 + CRATE_HALF + 0.05:
			var lo := proposed - 3.7 - (CRATE_HALF + 0.05)
			var hi := proposed + 3.7 + (CRATE_HALF + 0.05)
			if crate_world_pos.z >= lo and crate_world_pos.z <= hi:
				blocked = true
	if blocked:
		if drive_dir != 0 and not _truck_block_logged:
			_truck_block_logged = true
			_log("TRUCK: BLOCKED by the crate on the road (truck z=%.1f, crate z=%.1f)" % [truck_z, crate_world_pos.z])
	else:
		_truck_block_logged = false
		var before := truck_z
		truck_z = proposed
		var loaded_motion := absf(truck_z - before) > 0.0001 and seat_occupant != 0 and crate_holder == seat_occupant
		if loaded_motion and journey_stage != JourneyStage.EXITED_AT_DESTINATION:
			if journey_stage == JourneyStage.LOAD_AND_DRIVE:
				journey_stage = JourneyStage.DRIVING
				journey_driver = seat_occupant
				_log("MISSION: loaded truck journey started by peer %d" % journey_driver)
			elif journey_stage == JourneyStage.ARRIVED_SEATED and truck_z < before:
				# If the route-end driver vanished, a loaded replacement can reverse
				# and drive back to the end; the mission never remains soft-locked.
				journey_stage = JourneyStage.DRIVING
				journey_driver = seat_occupant
				_log("MISSION: route-end journey recovery started by peer %d" % journey_driver)
			elif journey_stage == JourneyStage.DRIVING and journey_driver != seat_occupant:
				journey_driver = seat_occupant
				_log("MISSION: loaded journey continued by replacement driver %d" % journey_driver)
			if journey_stage == JourneyStage.DRIVING and truck_z >= TRUCK_MAX_Z - ROUTE_ARRIVAL_EPSILON:
				# Snap authority and presentation together: %.0f previously rendered
				# 39.5 as 40 while the host required 39.99, making an ordinary exit
				# look valid before the host had advanced the journey stage.
				truck_z = TRUCK_MAX_Z
				journey_stage = JourneyStage.ARRIVED_SEATED
				_log("MISSION: loaded truck reached exact route end; valid driver exit now required")
		if truck_z >= _next_route_log - 0.01:
			_log("TRUCK: route position z=%.1f (bounded %.0f..%.0f)" % [truck_z, TRUCK_MIN_Z, TRUCK_MAX_Z])
			_next_route_log = _next_route_log + 10.0
	if seat_occupant != 0 and peer_pos.has(seat_occupant):
		peer_pos[seat_occupant] = truck.to_global(SEAT_LOCAL)

func _in_bounds(p: Vector3) -> bool:
	return absf(p.x) <= BOUND_X and p.z >= BOUND_Z_MIN and p.z <= BOUND_Z_MAX and p.y >= 0.5 and p.y <= 3.2

func _inside_truck_solid(world_pos: Vector3, pad: float) -> bool:
	var local := truck.global_transform.affine_inverse() * world_pos
	for s in truck_solids:
		if absf(local.x - s["c"].x) <= s["h"].x + pad \
				and absf(local.y - s["c"].y) <= s["h"].y + pad \
				and absf(local.z - s["c"].z) <= s["h"].z + pad:
			return true
	return false

func _host_validate_reports() -> void:
	for id in _pending_reports.keys():
		if Time.get_ticks_msec() < int(peer_exit_grace_until.get(id, 0)):
			# Reports queued while seated still contain the old cab pose. The exit
			# correction is authoritative; ignore those packets rather than pulling
			# the player back through the truck.
			continue
		var claim: Vector3 = _pending_reports[id]
		if seat_occupant == id:
			peer_pos[id] = truck.to_global(SEAT_LOCAL)
			continue
		var last: Vector3 = peer_pos.get(id, claim)
		var dh := Vector2(claim.x - last.x, claim.z - last.z).length()
		var dy := absf(claim.y - last.y)
		var bad := ""
		if dh > REPORT_MAX_HOP * 3.0 or dy > 2.0:
			bad = "teleport (%.2fm horizontal, %.2fm vertical)" % [dh, dy]
		elif not _in_bounds(claim):
			bad = "out of the bounded site"
		elif _inside_truck_solid(claim, 0.12) or _inside_truck_solid(last.lerp(claim, 0.5), 0.12):
			bad = "inside solid truck geometry"
		elif dh > REPORT_MAX_HOP or dy > REPORT_MAX_FALL:
			bad = "too fast (%.2fm horizontal, %.2fm vertical in one tick)" % [dh, dy]
		if bad != "":
			peer_strike[id] = peer_strike.get(id, 0) + 1
			if peer_strike[id] >= REPORT_STRIKES or bad.begins_with("teleport") or bad == "out of the bounded site":
				_log("HOST: corrected peer %d movement — %s" % [id, bad])
				_correct_pos.rpc_id(id, last, bad)
				peer_strike[id] = 0
		else:
			peer_strike[id] = 0
			peer_pos[id] = claim
	_pending_reports.clear()

# ---------------- crate resting (host logic, real geometry) ----------------

func _crate_rest(x: float, z: float) -> Dictionary:
	# A drop point outside the rails can never be recovered from; treat as invalid.
	if absf(x) > FENCE_X - 0.4 or z < FENCE_Z_MIN + 0.4 or z > FENCE_Z_MAX - 0.4:
		return {"y": 0.0, "on_truck": false, "valid": false}
	var lz := z - truck_z
	var on_bed := absf(x) <= 1.125 and lz >= -3.7 and lz <= 3.4
	if on_bed:
		if not _inside_truck_solid(Vector3(x, 0.65, z), CRATE_HALF - 0.01):
			return {"y": 0.65, "on_truck": true, "valid": true}  # truck bed top 0.3 + half crate
		return {"y": 0.0, "on_truck": false, "valid": false}
	var ground_top := 0.055 if (absf(x) <= 3.0 and z >= -5.0 and z <= 45.0) else 0.0
	return {"y": ground_top + CRATE_HALF, "on_truck": false, "valid": true}

func _check_delivery() -> void:
	if mission_state == MissionState.DELIVERED:
		return
	if journey_stage != JourneyStage.EXITED_AT_DESTINATION:
		_log("MISSION: delivery blocked — loaded truck journey and route-end exit incomplete (stage=%d route=%.1f)" % [journey_stage, truck_z])
		return
	if crate_holder == 0 and not crate_on_truck \
			and crate_world_pos.x >= ZONE_MIN.x and crate_world_pos.x <= ZONE_MAX.x \
			and crate_world_pos.y >= ZONE_MIN.y and crate_world_pos.y <= ZONE_MAX.y + 0.5 \
			and crate_world_pos.z >= ZONE_MIN.z and crate_world_pos.z <= ZONE_MAX.z:
		deliveries += 1
		mission_state = MissionState.DELIVERED
		_flash_crate(Color(0.2, 1.0, 0.3))
		_log("MISSION: HOST completed delivery #%d (loaded drive + route-end exit + zone unload) — broadcasting" % deliveries)
		_send_feedback(0, "ok", "DELIVERY COMPLETED — loaded route and destination unload verified", "ev_delivered")
	else:
		_log("MISSION: crate NOT in destination zone — delivery not completed")
	_sync_state.rpc(truck_z, mission_state, deliveries, crate_holder, crate_on_truck, crate_local_pos, crate_world_pos, seat_occupant, journey_stage, probe_phase)

# ---------------- local player + input ----------------

func _sim_local_player(delta: float) -> void:
	if my_peer_id == 0 or my_body == null:
		return
	var mv := Vector2.ZERO
	if probe and mechanics:
		# Mechanics assertions drive the real Input action path (action_press).
		mv = Input.get_vector("move_left", "move_right", "move_back", "move_forward")
	elif probe:
		mv = _autopilot_move()
		if _probe_drive:
			mv = Vector2(0, 1)  # hold W while driving the route
			my_yaw = PI         # face the direction of travel
	else:
		mv = Input.get_vector("move_left", "move_right", "move_back", "move_forward")
	var seated := seat_occupant == my_peer_id
	if seated:
		my_body.global_position = truck.to_global(SEAT_LOCAL)
		my_body.velocity = Vector3.ZERO
		_local_drive_input = 1 if mv.y > 0.4 else (-1 if mv.y < -0.4 else 0)
	else:
		_local_drive_input = 0
		var dir := _rgt(my_yaw) * mv.x + _fwd(my_yaw) * mv.y
		dir.y = 0.0
		if dir.length() > 1.0:
			dir = dir.normalized()
		my_body.velocity.x = dir.x * WALK_SPEED
		my_body.velocity.z = dir.z * WALK_SPEED
		if my_body.is_on_floor():
			my_body.velocity.y = -GRAVITY * delta  # slope-safe floor press; accumulates into real falls
		else:
			my_body.velocity.y -= GRAVITY * delta
		my_body.move_and_slide()
	peer_pos[my_peer_id] = my_body.global_position
	peer_yaw[my_peer_id] = my_yaw
	peer_seated[my_peer_id] = seated

func _interact() -> void:
	if my_peer_id == 0:
		return
	if seat_occupant == my_peer_id:
		_request_seat.rpc_id(1, false)
		return
	var hands := _hands_world_pos(my_peer_id)
	var crate_near := hands.distance_to(_crate_world_pos_now()) <= GRAB_REACH
	var seat_near := hands.distance_to(truck.to_global(Vector3(0.6, 0.6, 2.6))) <= SEAT_REACH
	if crate_holder == my_peer_id:
		# At the cab, interact enters the driver's seat while retaining cargo;
		# elsewhere it attempts the unload.
		if seat_near:
			_request_seat.rpc_id(1, true)
		else:
			_request_unload.rpc_id(1)
	elif seat_near and not crate_near:
		_request_seat.rpc_id(1, true)
	else:
		_request_grab.rpc_id(1)

func _interact_drop() -> void:
	if my_peer_id == 0:
		return
	if crate_holder == my_peer_id:
		_request_drop.rpc_id(1)
	else:
		_request_grab.rpc_id(1)

# ---------------- autopilot (probe modes only; drives the real pipeline) ------

func _auto_set(wps: Array[Vector3], face := 999.0, seek_crate := false) -> void:
	_auto_wps = wps
	_auto_i = 0
	_auto_face = face
	_auto_crate = seek_crate
	if seek_crate:
		_auto_wps = []

func _auto_arrived() -> bool:
	# "Arrived" means the whole route is consumed (final waypoint reached),
	# not merely the next intermediate waypoint.
	if _auto_crate:
		return my_body != null and _hands_world_pos(my_peer_id).distance_to(_crate_world_pos_now()) <= 1.3
	if _auto_i >= _auto_wps.size():
		return true
	if _auto_i != _auto_wps.size() - 1:
		return false
	var t: Vector3 = _auto_wps[_auto_i]
	var p := my_body.global_position if my_body != null else Vector3.ZERO
	return absf(t.x - p.x) <= 0.18 and absf(t.z - p.z) <= 0.18

func _autopilot_move() -> Vector2:
	if my_body == null:
		return Vector2.ZERO
	var p := my_body.global_position
	var target := Vector3.ZERO
	if _auto_crate:
		var cw := _crate_world_pos_now()
		var away := Vector2(p.x - cw.x, p.z - cw.z)
		if away.length() < 0.01:
			away = Vector2(0, -1)
		away = away.normalized()
		target = Vector3(cw.x + away.x * 0.85, p.y, cw.z + away.y * 0.85)
		var d := Vector2(target.x - p.x, target.z - p.z)
		if d.length() <= 0.1:
			if _auto_face < 99.0:
				my_yaw = _auto_face
			return Vector2.ZERO
		my_yaw = atan2(-d.x, -d.y)
		return Vector2(0, 1)
	while _auto_i < _auto_wps.size():
		target = _auto_wps[_auto_i]
		var d := Vector2(target.x - p.x, target.z - p.z)
		if d.length() <= 0.14:
			_auto_i += 1
			continue
		my_yaw = atan2(-d.x, -d.y)
		return Vector2(0, 1)
	if _auto_face < 99.0:
		my_yaw = _auto_face
	return Vector2.ZERO

# ---------------- visuals ----------------

func _flash_crate(color: Color) -> void:
	crate_flash_color = color
	crate_flash_until_ms = Time.get_ticks_msec() + 400

func _update_visuals() -> void:
	for id in players:
		var b: CharacterBody3D = players[id]
		if id != my_peer_id or my_body == null or my_body != b:
			b.global_position = peer_pos.get(id, b.global_position)
		b.rotation.y = peer_yaw.get(id, 0.0) + PI  # visual front faces the view direction
	if crate_holder != 0 and peer_pos.has(crate_holder):
		crate_mesh.global_position = _hands_world_pos(crate_holder) + Vector3(0, -0.45, 0) + _fwd(peer_yaw.get(crate_holder, 0.0)) * 0.6
	elif crate_on_truck:
		crate_mesh.global_position = truck.to_global(crate_local_pos)
	else:
		crate_mesh.global_position = crate_world_pos
	if crate_collider:
		crate_collider.set_deferred("disabled", crate_holder != 0)
	var mat: StandardMaterial3D = crate_mesh.material_override
	if Time.get_ticks_msec() < crate_flash_until_ms:
		mat.albedo_color = crate_flash_color
	else:
		mat.albedo_color = crate_base_color
	if my_peer_id != 0 and my_body != null:
		if seat_occupant == my_peer_id:
			cam.global_position = truck.to_global(SEAT_LOCAL + Vector3(0, SEAT_EYE_UP, 0))
		else:
			cam.global_position = my_body.global_position + Vector3(0, EYE_UP, 0)
		cam.rotation = Vector3(my_pitch, my_yaw, 0)
	if hud_banner and Time.get_ticks_msec() > banner_until_ms:
		hud_banner.text = ""
	if zone_mesh:
		var zmat: StandardMaterial3D = zone_mesh.material_override
		if mission_state == MissionState.DELIVERED:
			zmat.albedo_color = Color(0.1, 0.9, 0.3, 0.55)
		else:
			zmat.albedo_color = Color(0.9, 0.8, 0.1, 0.45)

func _update_hud() -> void:
	if hud_role == null:
		return
	var role := "IDLE" if mode == "" else mode.to_upper()
	hud_role.text = "SWIFT ARRIVAL · %s · peer %d · connected %d" % [role, my_peer_id, multiplayer.get_peers().size()]
	var ms := "IN PROGRESS" if mission_state == MissionState.IN_PROGRESS else "DELIVERED"
	var hold := "free" if crate_holder == 0 else ("CARRIED BY YOU" if crate_holder == my_peer_id else "held by peer %d" % crate_holder)
	var doing := "ON FOOT" if seat_occupant != my_peer_id else "DRIVING"
	# Floor partial progress so the HUD cannot display 40 until the host has
	# snapped a valid loaded journey to the exact authoritative endpoint.
	var route_display := floori(clampf(truck_z, TRUCK_MIN_Z, TRUCK_MAX_Z) + 0.0001)
	hud_state.text = "%s · DELIVERY %s · CRATE %s · ROUTE %d / %.0f m" % [doing, ms, hold, route_display, TRUCK_MAX_Z]
	var objective := ""
	var action := ""
	if mission_state == MissionState.DELIVERED:
		objective = "DELIVERY COMPLETE"
		action = "Crate accepted in the yellow destination zone"
	elif seat_occupant == my_peer_id:
		if journey_stage == JourneyStage.ARRIVED_SEATED:
			objective = "ROUTE END REACHED — EXIT TO DELIVER"
			action = "E  EXIT AT REAR CARGO THRESHOLD"
		else:
			objective = "DRIVE TO ROUTE END"
			action = "W / S  DRIVE   ·   E  EXIT"
	elif crate_holder == my_peer_id:
		var pos := my_body.global_position if my_body != null else Vector3.ZERO
		var drop := pos + _fwd(my_yaw) * PLACE_FWD
		var rest := _crate_rest(drop.x, drop.z)
		var in_unload_zone: bool = drop.x >= ZONE_MIN.x and drop.x <= ZONE_MAX.x and drop.z >= ZONE_MIN.z and drop.z <= ZONE_MAX.z and bool(rest["valid"]) and not bool(rest["on_truck"])
		var can_unload := in_unload_zone and journey_stage == JourneyStage.EXITED_AT_DESTINATION
		if can_unload:
			objective = "DESTINATION REACHED"
			action = "E  UNLOAD CRATE NOW"
		elif in_unload_zone and journey_stage != JourneyStage.EXITED_AT_DESTINATION:
			objective = "TRUCK JOURNEY REQUIRED"
			action = "Drive loaded truck 0 → 40 m, then exit at route end"
		elif truck_z < TRUCK_MAX_Z - 0.2 and _hands_world_pos(my_peer_id).distance_to(truck.to_global(Vector3(0.6, 0.6, 2.6))) <= SEAT_REACH:
			objective = "TAKE THE CRATE TO THE ROUTE END"
			action = "E  ENTER DRIVER SEAT"
		else:
			objective = "COMPLETE THE LOADED TRUCK JOURNEY"
			action = "Re-enter the yellow DRIVER seat through cargo · drive to 40 m · exit"
	else:
		objective = "PICK UP THE CRATE"
		action = "Approach crate · E  PICK UP"
	hud_objective.text = "OBJECTIVE: " + objective
	hud_action.text = action
	# Runtime events remain attributable in stdout/log files. Normal play does
	# not render the debug transcript over the first-person scene.
	if probe:
		while hud_log_lines.size() > 3:
			hud_log_lines.pop_front()
		hud_log.text = "\n".join(hud_log_lines)
	else:
		hud_log.text = ""

func _log(msg: String) -> void:
	print("[%s] %s" % [Time.get_time_string_from_system(), msg])
	hud_log_lines.append(msg)

# ---------------- screenshots (capture fallback; ffmpeg absent in runtime) -----

func take_screenshot(name_: String) -> void:
	if shots_dir == "" or DisplayServer.get_name() == "headless":
		return
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	if img:
		var path := shots_dir.path_join(name_ + ".png")
		if FileAccess.file_exists(path):
			_shot_dup += 1
			path = shots_dir.path_join("%s_%02d.png" % [name_, _shot_dup])
		img.save_png(path)
		_log("CAPTURE: saved %s" % path)

# ---------------- scripted probe (verification.md) ----------------

func _probe_step() -> void:
	var t := Time.get_ticks_msec() / 1000.0
	if mode == "host":
		_probe_host(t)
	elif mode == "client":
		_probe_client(t)

func _probe_fail(reason: String) -> void:
	if _p.has("failed"):
		return
	_p["failed"] = true
	_log("PROBE RESULT: FAIL — %s" % reason)
	_finish(false)

func _probe_pass(note: String) -> void:
	if _p.has("passed"):
		return
	_p["passed"] = true
	_log("PROBE RESULT: PASS — %s" % note)

func _probe_host(t: float) -> void:
	if t > 150:
		_probe_fail("host timeout (150s)")
		return
	if not _p.has("shot0") and multiplayer.get_peers().size() > 0:
		_p["shot0"] = true
		take_screenshot("01_host_client_connected")
	if not _p.has("shot2") and crate_holder != 0:
		_p["shot2"] = true
		take_screenshot("02_crate_carried_host_view")
	if not _p.has("shot3") and truck_z >= TRUCK_MAX_Z - 0.1:
		_p["shot3"] = true
		take_screenshot("03_truck_route_end_host_view")
	if not _p.has("delivered_seen") and mission_state == MissionState.DELIVERED:
		_p["delivered_seen"] = t
		_log("PROBE(host): observed host-authoritative DELIVERY COMPLETED")
		take_screenshot("04_delivery_completed_host_view")
	if mechanics:
		_probe_host_mechanics(t)
		return
	if negative:
		if _p.has("sig_negative_done") and _p.has("rejected_journey_bypass") and deliveries == 0 \
				and mission_state == MissionState.IN_PROGRESS and truck_z < 0.1 \
				and journey_stage == JourneyStage.LOAD_AND_DRIVE:
			_probe_pass("negative path: on-foot carry into destination at route 0 rejected; delivery NOT completed")
			await get_tree().create_timer(1.0).timeout
			_finish(true)
		return
	if recovery:
		if not _p.has("crate_behind") and crate_holder == 0 and not crate_on_truck \
				and crate_world_pos.z < -4.2 and truck_z < 1.0:
			_p["crate_behind"] = true
			_log("PROBE(host): crate observed dropped BEHIND the truck at route start (z=%.2f)" % crate_world_pos.z)
		if _p.has("crate_behind") and crate_holder != 0 and not _p.has("regrab_seen"):
			_p["regrab_seen"] = true
			_log("PROBE(host): dropped crate RECOVERED by peer %d" % crate_holder)
		if _p.has("delivered_seen") and _p.has("crate_behind") and _p.has("regrab_seen") \
				and _p.has("recovery_early_exit") and _p.has("recovery_reseat") and not _p.has("quit"):
			_probe_pass("recovery path: dropped crate recovered; early exit and natural seat re-entry recovered; delivery completed")
			_p["quit"] = true
			await get_tree().create_timer(1.0).timeout
			_finish(true)
		return
	# positive
	if _p.has("delivered_seen") and not _p.has("quit"):
		if t - _p["delivered_seen"] > 2.0 or _p.has("sig_positive_done"):
			_probe_pass("positive path: full delivery loop observed host-authoritatively")
			_p["quit"] = true
			await get_tree().create_timer(1.0).timeout
			_finish(true)

func _probe_client(t: float) -> void:
	if t > 150:
		_probe_fail("client timeout (150s)")
		return
	if _p.has("passed") or my_peer_id == 0 or my_body == null:
		return
	if mechanics:
		if probe_phase == "mechanics_done":
			_probe_pass("mechanics assertions passed on host; client observed completion phase")
			_probe_signal.rpc_id(1, "mechanics_done")
			await get_tree().create_timer(1.0).timeout
			_finish(true)
		return
	var el: float = t - float(_p.get("t0", t))
	if not _p.has("joined"):
		if multiplayer.get_peers().size() > 0:
			_p["joined"] = true
			_p["t0"] = t
			_log("PROBE(client): joined host session; running scripted delivery")
		return
	# Phase 1: walk to the crate and pick it up (hand interaction).
	if not _p.has("grab_ok"):
		if not _p.has("auto_grab"):
			_p["auto_grab"] = true
			_auto_set([], 0.0, true)
		if _auto_arrived() and not _p.has("grab_req"):
			_p["grab_req"] = true
			_request_grab.rpc_id(1)
			_log("PROBE(client): requested crate pickup via hand interaction")
		if crate_holder == my_peer_id:
			_p["grab_ok"] = el
			_auto_set([Vector3(0, 1.11, 0.3), Vector3(0, 1.11, 2.0), Vector3(-0.3, 1.11, 2.2)])
		elif _p.has("grab_req") and el > 12.0:
			_probe_fail("crate pickup was not resolved by host (crate_holder=%d, me=%d)" % [crate_holder, my_peer_id])
	# Phase 2 (negative): carry the crate on foot from route 0 all the way into
	# the destination zone, reproducing the blind bypass without touching the seat.
	elif negative:
		if not _p.has("neg_walk"):
			_neg_start_rear_walk()
			_p["neg_walk"] = true
		if _auto_arrived() and not _p.has("neg_face"):
			_p["neg_face"] = true
			_auto_set([], PI)
			my_yaw = PI  # place the attempted unload forward into the zone
			_request_unload.rpc_id(1)
			_log("PROBE(client,neg): requested ON-FOOT UNLOAD inside destination at ROUTE 0 (no seat/drive/exit)")
			_p["neg_req_t"] = el
		if _p.has("neg_req_t"):
			if _p.has("saw_reject"):
				if crate_holder == my_peer_id and mission_state == MissionState.IN_PROGRESS and deliveries == 0 \
						and truck_z < 0.1 and journey_stage == JourneyStage.LOAD_AND_DRIVE:
					_probe_pass("on-foot destination bypass at route 0 REJECTED by host; delivery not completed")
					_probe_signal.rpc_id(1, "negative_done")
					await get_tree().create_timer(1.0).timeout
					_finish(true)
				elif el - _p["neg_req_t"] > 4.0:
					_probe_fail("negative bypass: authoritative state inconsistent after rejection (holder=%d mission=%d route=%.1f journey=%d)" % [crate_holder, mission_state, truck_z, journey_stage])
			elif el - _p["neg_req_t"] > 4.0:
				_probe_fail("negative bypass: host rejection feedback never became visible on requester")
	# Phase 2 (recovery): drop the crate behind the truck at route start, then recover it.
	elif recovery and not _p.has("regrab_ok"):
		if not _p.has("rec_walk"):
			_auto_set([Vector3(0, 1.11, -3.0), Vector3(0, 1.11, -5.2)], 0.0)
			_p["rec_walk"] = true
		if _auto_arrived() and not _p.has("drop_req"):
			_p["drop_req"] = true
			my_yaw = 0.0
			_request_drop.rpc_id(1)
			_log("PROBE(client,rec): DROPPED the crate behind the truck at route start")
		# Wait until the host-resolved drop is OBSERVED locally (holder == 0)
		# before treating the crate as dropped — the stale pre-drop state must
		# not shortcut the recovery.
		if _p.has("drop_req") and not _p.has("rec_holder0"):
			if crate_holder == 0:
				_p["rec_holder0"] = true
				if not crate_on_truck and crate_world_pos.z >= -4.2:
					_probe_fail("recovery path: crate did not land behind the truck (z=%.2f)" % crate_world_pos.z)
					return
				_dropped_seen_note()
				_auto_set([], 0.0, true)  # walk back to the crate
		elif _p.has("rec_holder0"):
			if not _p.has("grab2_req") and _auto_arrived():
				_p["grab2_req"] = true
				_request_grab.rpc_id(1)
				_log("PROBE(client,rec): requested re-grab of the dropped crate")
			elif _p.has("grab2_req") and crate_holder == my_peer_id:
				_p["regrab_ok"] = el
				_log("PROBE(client,rec): crate RECOVERED from behind the truck; reboarding to deliver")
				_auto_set([Vector3(0, 1.11, -2.5), Vector3(0, 1.11, 0.3), Vector3(0, 1.11, 2.0), Vector3(-0.3, 1.11, 2.2)])
		elif _p.has("drop_req") and el > 20.0:
			_probe_fail("recovery path: drop was never resolved by host")
	# Phase 2 (positive/recovery): walk through the doorway and take the driver seat.
	elif not _p.has("seated") and not negative:
		if _auto_arrived() and not _p.has("seat_req"):
			_p["seat_req"] = true
			_request_seat.rpc_id(1, true)
			_log("PROBE(client): requested driver position (walked cargo->cab through doorway)")
		if seat_occupant == my_peer_id:
			_p["seated"] = el
			_probe_drive = false
			_log("PROBE(client): in driver seat, still holding crate")
		elif _p.has("seat_req") and el > 20.0:
			_probe_fail("seat request was not resolved by host")
	# Phase 3: in recovery mode, deliberately exit early, naturally walk from
	# the rear threshold through cargo to the cab, re-enter, then finish driving.
	elif not _p.has("drove"):
		if recovery and not _p.has("early_reentered"):
			if not _p.has("early_exit_req"):
				_probe_drive = true
				if truck_z >= 10.0:
					_probe_drive = false
					_p["early_exit_req"] = el
					_request_seat.rpc_id(1, false)
					_auto_set([Vector3(0, 1.11, truck_z - 2.5), Vector3(0, 1.11, truck_z + 0.3), Vector3(0, 1.11, truck_z + 2.0), Vector3(-0.3, 1.11, truck_z + 2.2)])
					_log("PROBE(client,rec): deliberately requested INCOMPLETE EXIT at route %.1f" % truck_z)
			elif seat_occupant == 0:
				if not _p.has("early_exit_seen"):
					_p["early_exit_seen"] = el
					_log("PROBE(client,rec): early exit resolved at rear; walking naturally through cargo to cab")
				if _auto_arrived() and not _p.has("early_reseat_req"):
					_p["early_reseat_req"] = el
					_request_seat.rpc_id(1, true)
					_log("PROBE(client,rec): requested natural driver-seat RE-ENTRY after early exit")
			elif _p.has("early_reseat_req") and seat_occupant == my_peer_id:
				_p["early_reentered"] = el
				_probe_drive = false
				_log("PROBE(client,rec): host accepted natural driver-seat RE-ENTRY; continuing journey")
			if _p.has("early_exit_req") and el - float(_p["early_exit_req"]) > 20.0 and not _p.has("early_reentered"):
				_probe_fail("natural driver-seat re-entry after incomplete exit stalled")
			return
		_probe_drive = true  # hold drive input (W) while seated
		if journey_stage == JourneyStage.ARRIVED_SEATED:
			_p["drove"] = el
			_probe_drive = false
			_log("PROBE(client): host confirmed loaded truck at exact route end z=%.2f" % truck_z)
		elif el > 70.0:
			_probe_fail("truck never reached the route end")
	# Phase 4: leave the seat, walk out of the truck, unload in the zone.
	elif not _p.has("released"):
		_probe_drive = false
		if not _p.has("stand_req"):
			_p["stand_req"] = true
			_request_seat.rpc_id(1, false)
			_auto_set([Vector3(0, 1.11, truck_z + 1.0), Vector3(0, 1.11, truck_z - 2.8), Vector3(0, 1.11, truck_z - 4.4), Vector3(0, 0.9, truck_z - 4.6)], 0.0)
			_log("PROBE(client): left driver seat, carrying crate out of the truck to unload")
		if _auto_arrived() and seat_occupant == 0 and not _p.has("rel_req"):
			_p["rel_req"] = true
			_p["rel_req_t"] = el
			my_yaw = 0.0
			_request_unload.rpc_id(1)
			_log("PROBE(client): requested UNLOAD in the destination zone")
		if crate_holder == 0 and _p.has("rel_req"):
			_p["released"] = el
		elif _p.has("rel_req") and el - _p["rel_req_t"] > 6.0:
			_probe_fail("unload in the destination zone was not resolved (holder=%d)" % crate_holder)
		elif el > 70.0:
			_probe_fail("release phase stalled")
	# Phase 5: observe host-authoritative completion on this client.
	elif not _p.has("done"):
		if mission_state == MissionState.DELIVERED:
			_probe_pass("positive delivery loop completed and visible on the client")
			_probe_signal.rpc_id(1, "positive_done")
			await get_tree().create_timer(1.0).timeout
			_finish(true)
		elif el - _p["released"] > 5.0:
			_probe_fail("delivery did not complete on client after unload")
func _dropped_seen_note() -> void:
	_p["dropped_seen"] = true
	_log("PROBE(client,rec): crate at rest behind truck at world z=%.2f" % crate_world_pos.z)

func _neg_start_rear_walk() -> void:
	_auto_set([Vector3(0, 1.11, -3.0), Vector3(0, 1.11, -5.0), Vector3(4.0, 1.11, -5.0), Vector3(4.0, 1.11, 34.5), Vector3(0, 1.11, 34.5)], PI)

# ---- mechanics assertions (host process; real Input actions, real geometry) --

func _m_teleport(pos: Vector3, yaw: float) -> void:
	my_yaw = yaw
	my_pitch = 0.0
	my_body.global_position = pos
	my_body.velocity = Vector3.ZERO

func _m_release_all() -> void:
	for a in ["move_right", "move_forward", "move_back", "move_left"]:
		if Input.is_action_pressed(a):
			Input.action_release(a)

func _probe_host_mechanics(t: float) -> void:
	if my_body == null or multiplayer.get_peers().size() == 0:
		return
	var st := int(_p.get("mstage", 0))
	var st_t: float = _p.get("mstage_t", t)
	match st:
		0:
			_p["mstage"] = 1
			_p["mstage_t"] = t
			_log("PROBE(host,mech): client joined; beginning movement-basis assertions")
		1:  # settle after spawn
			if t - st_t > 0.4:
				_m_teleport(Vector3(0, 0.9, 12.0), 0.9)
				_p["mstage"] = 2
				_p["mstage_t"] = t
				_p["m_p0"] = my_body.global_position
				Input.action_press("move_right")
				_log("PROBE(host,mech): strafe test at yaw=0.9 — D must move along camera right %s" % _rgt(0.9))
		2:  # strafe right at yaw 0.9
			if t - st_t > 0.8:
				_m_release_all()
				_p["mstage"] = 3
				_p["mstage_t"] = t
		3:
			var d: Vector3 = my_body.global_position - _p["m_p0"]
			var dn := Vector3(d.x, 0, d.z)
			var r := _rgt(0.9)
			var dot := 0.0
			if dn.length() > 0.001:
				dot = dn.normalized().dot(r)
			_log("PROBE(host,mech): strafe displacement=%s (len %.2f), dot with camera right=%.3f" % [str(dn), dn.length(), dot])
			if dn.length() < 0.5:
				_probe_fail("strafe displacement %.2fm too small — movement did not follow input" % dn.length())
			elif dot < 0.97:
				_probe_fail("strafe moved along %s, not camera right %s (dot=%.3f) — movement is not view-relative" % [str(dn.normalized()), str(r), dot])
			elif absf(absf(dn.normalized().dot(Vector3(1, 0, 0))) - 1.0) < 0.05:
				_probe_fail("strafe ran along the fixed world X axis — that is the baseline bug, not view-relative movement")
			_p["mstage"] = 4
			_p["mstage_t"] = t
		4:  # forward at a different yaw
			if t - st_t > 0.3:
				_m_teleport(Vector3(0, 0.9, 12.0), -2.4)
				_p["mstage"] = 5
				_p["mstage_t"] = t
				_p["m_p0"] = my_body.global_position
				Input.action_press("move_forward")
		5:
			if t - st_t > 0.8:
				_m_release_all()
				_p["mstage"] = 6
				_p["mstage_t"] = t
		6:
			var d: Vector3 = my_body.global_position - _p["m_p0"]
			var dn := Vector3(d.x, 0, d.z)
			var f := _fwd(-2.4)
			var dot := 0.0
			if dn.length() > 0.001:
				dot = dn.normalized().dot(f)
			_log("PROBE(host,mech): forward displacement=%s (len %.2f), dot with camera forward=%.3f" % [str(dn), dn.length(), dot])
			if dn.length() < 0.5 or dot < 0.97:
				_probe_fail("forward displacement %s does not follow camera forward %s (dot=%.3f)" % [str(dn), str(f), dot])
			_p["mstage"] = 7
			_p["mstage_t"] = t
		7:  # inside the truck: press into the cargo wall while turning
			if t - st_t > 0.3:
				_m_teleport(Vector3(0, 1.11, truck_z - 0.5), -PI / 2.0)
				_p["mstage"] = 8
				_p["mstage_t"] = t
				Input.action_press("move_forward")
		8:
			my_yaw = -PI / 2.0 + 0.8 * clampf((t - st_t) / 0.8, 0.0, 1.0)  # turn while blocked
			if t - st_t > 0.8:
				_m_release_all()
				_p["mstage"] = 9
				_p["mstage_t"] = t
		9:
			var x := my_body.global_position.x
			_log("PROBE(host,mech): pressed into cargo wall while turning for 0.8s (attempted ~%.2fm); final |x|=%.3f (wall contact at 0.825)" % [WALK_SPEED * 0.8, absf(x)])
			if absf(x) > 0.86:
				_probe_fail("player penetrated the cargo wall from inside (|x|=%.3f > 0.86) — collision not solid" % absf(x))
			_p["mstage"] = 10
			_p["mstage_t"] = t
		10:  # outside the truck: press into the cargo wall from the road
			if t - st_t > 0.3:
				_m_teleport(Vector3(3.0, 0.9, truck_z - 0.5), PI / 2.0)
				_p["mstage"] = 11
				_p["mstage_t"] = t
				Input.action_press("move_forward")
		11:
			if t - st_t > 0.8:
				_m_release_all()
				_p["mstage"] = 12
				_p["mstage_t"] = t
		12:
			var x := my_body.global_position.x
			_log("PROBE(host,mech): pressed into cargo wall from outside for 0.8s; final x=%.3f (blocked at ~1.58)" % [x])
			if x < 1.52:
				_probe_fail("player passed through or was sucked through the cargo wall from outside (x=%.3f < 1.52)" % x)
			_p["mstage"] = 13
			_p["mstage_t"] = t
		13:  # leave the truck through the rear on foot
			if t - st_t > 0.3:
				_m_teleport(Vector3(0, 1.11, truck_z - 3.0), 0.0)
				_p["mstage"] = 14
				_p["mstage_t"] = t
				Input.action_press("move_forward")
		14:
			if t - st_t > 3.0 or my_body.global_position.z < truck_z - 4.25:
				_m_release_all()
				_p["mstage"] = 15
				_p["mstage_t"] = t
		15:
			var p := my_body.global_position
			_log("PROBE(host,mech): walked out of the truck rear; now at %s (outside footprint z<%.2f, ground level y~0.87)" % [str(p), truck_z - 3.7])
			if p.z > truck_z - 4.2:
				_probe_fail("player could not leave the truck through the rear (z=%.2f)" % p.z)
			elif p.y > 1.0:
				_probe_fail("player left the truck but did not reach ground level (y=%.2f)" % p.y)
			else:
				_log("PROBE(host,mech): MECHANICS VERIFIED — view-relative basis + solid collision + exit on foot")
				probe_phase = "mechanics_done"
				_sync_state.rpc(truck_z, mission_state, deliveries, crate_holder, crate_on_truck, crate_local_pos, crate_world_pos, seat_occupant, journey_stage, probe_phase)
			_p["mstage"] = 16
			_p["mstage_t"] = t
		16:
			if _p.has("sig_mechanics_done") or t - st_t > 12.0:
				_probe_pass("mechanics assertions passed (basis, walls both sides, leave on foot)")
				await get_tree().create_timer(1.0).timeout
				_finish(true)

func _finish(ok: bool) -> void:
	_finished = true
	_log("PROBE: exiting (exit code %d)" % (0 if ok else 1))
	if multiplayer.multiplayer_peer:
		multiplayer.multiplayer_peer.close()
	await get_tree().process_frame
	get_tree().quit(0 if ok else 1)
