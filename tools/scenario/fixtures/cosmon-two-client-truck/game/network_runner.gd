extends Node

# This is intentionally a tiny low-level ENet exercise. It avoids a production
# replication layer: the server is the only source of truth for five fixed
# technical facts, and clients simply send their next deterministic input after
# receiving an observed fact.

const EXPECTED_ACTIONS := [
	{"action": "pickup_crate", "role": "driver", "fact": "crate_picked_up"},
	{"action": "enter_driver_seat", "role": "driver", "fact": "driver_entered"},
	{"action": "move_truck", "role": "driver", "fact": "truck_moved"},
	{"action": "unload_crate", "role": "unloader", "fact": "crate_unloaded"}
]

var settings: Dictionary = {}
var transport: ENetMultiplayerPeer
var role := ""
var output_dir := ""
var started_ms := 0
var deadline_ms := 0
var hello_sent := false
var assigned := false
var assigned_peers := false
var expected_index := 0
var known_peers: Dictionary = {}
var received_actions: Array = []
var observed_events: Array = []
var sent_actions: Dictionary = {}
var completion_acks: Dictionary = {}
var mission_completed_at := -1
var client_complete_at := -1
var report_written := false

func _ready() -> void:
	settings = _parse_settings(OS.get_cmdline_user_args())
	role = _setting("role", "")
	output_dir = _setting("output", "")
	if role != "server" and role != "driver" and role != "unloader":
		_fatal("role must be server, driver, or unloader")
		return
	if output_dir.is_empty():
		_fatal("--output is required")
		return
	if DirAccess.make_dir_recursive_absolute(output_dir) != OK:
		_fatal("could not create output directory")
		return
	started_ms = Time.get_ticks_msec()
	deadline_ms = started_ms + int(_setting("timeout-ms", "25000"))
	if role == "server":
		_start_server()
	else:
		_start_client()

func _process(_delta: float) -> void:
	if transport == null:
		return
	if Time.get_ticks_msec() > deadline_ms:
		_fatal("scenario deadline elapsed")
		return
	# This fixture uses MultiplayerPeer directly instead of attaching a
	# MultiplayerAPI to a production scene tree, so it must advance ENet itself.
	if transport.get_connection_status() != MultiplayerPeer.CONNECTION_DISCONNECTED:
		transport.poll()
	_drain_packets()
	if role == "server":
		_maybe_assign_peers()
		_maybe_finish_server()
	else:
		_maybe_send_hello()
		_maybe_finish_client()

func _start_server() -> void:
	transport = ENetMultiplayerPeer.new()
	var result := transport.create_server(int(_setting("port", "0")), 4)
	if result != OK:
		_fatal("ENet server could not bind: %s" % result)
		return
	_write_json("server-ready.json", {
		"schema": "cosmon-enet-server-ready/v1",
		"test_world": true,
		"transport": "godot-enet",
		"port": int(_setting("port", "0")),
		"pid": OS.get_process_id()
	})

func _start_client() -> void:
	transport = ENetMultiplayerPeer.new()
	var result := transport.create_client(_setting("host", "127.0.0.1"), int(_setting("port", "0")))
	if result != OK:
		_fatal("ENet client could not start: %s" % result)

func _maybe_send_hello() -> void:
	if hello_sent or transport.get_connection_status() != MultiplayerPeer.CONNECTION_CONNECTED:
		return
	hello_sent = true
	_send_to(1, {"type": "hello", "role": role, "pid": OS.get_process_id()})

func _drain_packets() -> void:
	while transport.get_available_packet_count() > 0:
		var sender := transport.get_packet_peer()
		var raw := transport.get_packet().get_string_from_utf8()
		var parsed = JSON.parse_string(raw)
		if typeof(parsed) != TYPE_DICTIONARY:
			continue
		if role == "server":
			_handle_server_message(sender, parsed)
		else:
			_handle_client_message(parsed)

func _handle_server_message(sender: int, message: Dictionary) -> void:
	var message_type := String(message.get("type", ""))
	if message_type == "hello":
		var announced_role := String(message.get("role", ""))
		if announced_role == "driver" or announced_role == "unloader":
			known_peers[sender] = announced_role
		return
	if message_type == "action":
		if expected_index >= EXPECTED_ACTIONS.size():
			return
		var expected: Dictionary = EXPECTED_ACTIONS[expected_index]
		var action := String(message.get("action", ""))
		var actor_role := String(known_peers.get(sender, ""))
		if action != String(expected.action) or actor_role != String(expected.role):
			return
		var fact := String(expected.fact)
		received_actions.append({
			"sequence": expected_index + 1,
			"action": action,
			"fact": fact,
			"actor_role": actor_role,
			"peer_id": sender,
			"elapsed_ms": Time.get_ticks_msec() - started_ms
		})
		observed_events.append({
			"sequence": expected_index + 1,
			"fact": fact,
			"elapsed_ms": Time.get_ticks_msec() - started_ms
		})
		expected_index += 1
		_broadcast({"type": "event", "fact": fact, "sequence": expected_index})
		if expected_index == EXPECTED_ACTIONS.size():
			mission_completed_at = Time.get_ticks_msec()
			observed_events.append({
				"sequence": expected_index + 1,
				"fact": "mission_completed",
				"elapsed_ms": Time.get_ticks_msec() - started_ms
			})
			_broadcast({"type": "event", "fact": "mission_completed", "sequence": expected_index + 1})
		return
	if message_type == "ack" and String(message.get("fact", "")) == "mission_completed":
		completion_acks[sender] = true

func _handle_client_message(message: Dictionary) -> void:
	var message_type := String(message.get("type", ""))
	if message_type == "assigned":
		if String(message.get("role", "")) != role:
			return
		assigned = true
		if role == "driver":
			_send_action("pickup_crate")
		return
	if message_type != "event":
		return
	var fact := String(message.get("fact", ""))
	if role == "driver" and fact == "crate_picked_up":
		_send_action("enter_driver_seat")
	elif role == "driver" and fact == "driver_entered":
		_send_action("move_truck")
	elif role == "unloader" and fact == "truck_moved":
		_send_action("unload_crate")
	elif fact == "mission_completed":
		_send_to(1, {"type": "ack", "fact": "mission_completed", "role": role})
		client_complete_at = Time.get_ticks_msec()

func _maybe_assign_peers() -> void:
	if assigned_peers or known_peers.size() != 2:
		return
	var roles := known_peers.values()
	if not roles.has("driver") or not roles.has("unloader"):
		return
	assigned_peers = true
	for peer_id in known_peers:
		_send_to(int(peer_id), {"type": "assigned", "role": String(known_peers[peer_id])})

func _maybe_finish_server() -> void:
	if mission_completed_at < 0:
		return
	if completion_acks.size() < 2 and Time.get_ticks_msec() - mission_completed_at < 1500:
		return
	if not report_written:
		report_written = true
		var peer_rows: Array = []
		for peer_id in known_peers:
			peer_rows.append({"peer_id": int(peer_id), "role": String(known_peers[peer_id])})
		peer_rows.sort_custom(func(a, b): return int(a.peer_id) < int(b.peer_id))
		_write_json("server-report.json", {
			"schema": "cosmon-two-client-truck-server-report/v1",
			"test_world": true,
			"transport": "godot-enet",
			"peer_count": known_peers.size(),
			"peers": peer_rows,
			"received_actions": received_actions,
			"events": observed_events,
			"completion_ack_count": completion_acks.size(),
			"mission": {
				"crate": "unloaded",
				"driver": "exited_after_move",
				"truck": "moved_down_straight_road",
				"completed": true
			},
			"elapsed_ms": Time.get_ticks_msec() - started_ms
		})
	get_tree().quit(0)

func _maybe_finish_client() -> void:
	if client_complete_at >= 0 and Time.get_ticks_msec() - client_complete_at >= 150:
		get_tree().quit(0)

func _send_action(action: String) -> void:
	if not assigned or sent_actions.has(action):
		return
	sent_actions[action] = true
	_send_to(1, {"type": "action", "action": action, "role": role})

func _send_to(peer_id: int, message: Dictionary) -> void:
	transport.set_target_peer(peer_id)
	transport.put_packet(JSON.stringify(message).to_utf8_buffer())
	transport.set_target_peer(0)

func _broadcast(message: Dictionary) -> void:
	transport.set_target_peer(0)
	transport.put_packet(JSON.stringify(message).to_utf8_buffer())

func _parse_settings(arguments: PackedStringArray) -> Dictionary:
	var parsed := {}
	var index := 0
	while index < arguments.size():
		var argument := arguments[index]
		if argument.begins_with("--") and index + 1 < arguments.size():
			parsed[argument.substr(2)] = arguments[index + 1]
			index += 2
		else:
			index += 1
	return parsed

func _setting(key: String, fallback: String) -> String:
	return String(settings.get(key, fallback))

func _write_json(name: String, value: Dictionary) -> void:
	var file := FileAccess.open(output_dir.path_join(name), FileAccess.WRITE)
	if file == null:
		push_error("could not write " + name)
		return
	file.store_string(JSON.stringify(value, "\t") + "\n")
	file.close()

func _fatal(message: String) -> void:
	push_error("cosmon network fixture: " + message)
	if not output_dir.is_empty():
		_write_json("network-error.json", {"error": message, "role": role, "elapsed_ms": Time.get_ticks_msec() - started_ms})
	get_tree().quit(1)
