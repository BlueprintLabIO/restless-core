#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const output = path.resolve(requiredEnvironment('RESTLESS_SCENARIO_OUTPUT'));
const report = await readJson('server-report.json');
const network = await readJson('network-observation.json');
const processes = await readJson('process-observation.json');
const trace = await readJson('input-trace.json');

assert.equal(report.test_world, true);
assert.equal(report.transport, 'godot-enet');
assert.equal(report.peer_count, 2, 'the server must observe two actual ENet peers');
assert.deepEqual([...report.peers.map(peer => peer.role)].sort(), ['driver', 'unloader']);
assert.deepEqual(report.received_actions.map(action => action.action), [
	'pickup_crate',
	'enter_driver_seat',
	'move_truck',
	'unload_crate',
]);
assert.deepEqual(report.received_actions.map(action => action.actor_role), ['driver', 'driver', 'driver', 'unloader']);
assert.deepEqual(report.events.map(event => event.fact), [
	'crate_picked_up',
	'driver_entered',
	'truck_moved',
	'crate_unloaded',
	'mission_completed',
]);
assert.equal(report.mission.completed, true);
assert.equal(report.mission.crate, 'unloaded');
assert.equal(report.completion_ack_count, 2, 'both client processes must acknowledge the final event');
assert.equal(network.configured.one_way_delay_ms, 60);
assert.equal(network.configured.expected_round_trip_delay_ms, 120);
assert.ok(network.observed.client_to_server_drops >= 2, 'proxy must observe intentional client-to-server packet loss');
assert.ok(network.observed.server_to_client_drops >= 2, 'proxy must observe intentional server-to-client packet loss');
assert.equal(network.observed.flows.length, 2, 'two distinct client UDP flows must reach the proxy');
assert.ok(network.observed.flows.every(flow => (
	flow.client_to_server_drops === 1 && flow.server_to_client_drops === 1
)), 'each client flow must observe the declared loss profile in both directions');
assert.equal(processes.processes.length, 3);
assert.ok(processes.processes.every(processInfo => processInfo.exit_code === 0 && processInfo.spawn_error === null));
assert.deepEqual(trace.actions, report.received_actions);

console.log('Cosmon ENet server observation asserted');

function requiredEnvironment(name) {
	if (!process.env[name]) throw new Error(`${name} is required`);
	return process.env[name];
}

async function readJson(name) {
	return JSON.parse(await readFile(path.join(output, name), 'utf8'));
}
