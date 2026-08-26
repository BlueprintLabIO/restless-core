#!/usr/bin/env node

import { createHash } from 'node:crypto';
import dgram from 'node:dgram';
import { createServer } from 'node:net';
import { createReadStream, createWriteStream } from 'node:fs';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const fixtureRoot = path.dirname(fileURLToPath(import.meta.url));
const gameRoot = path.join(fixtureRoot, 'game');
const output = requiredEnvironment('RESTLESS_SCENARIO_OUTPUT');
const oneWayDelayMs = 60;
const requestedLossesPerDirection = 1;
const timeoutMs = 30_000;

function requiredEnvironment(name) {
	const value = process.env[name];
	if (!value) throw new Error(`${name} is required`);
	return path.resolve(value);
}

async function writeJson(file, value) {
	await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
}

async function sha256(file) {
	const hash = createHash('sha256');
	await new Promise((resolve, reject) => {
		const source = createReadStream(file);
		source.on('data', chunk => hash.update(chunk));
		source.on('error', reject);
		source.on('end', resolve);
	});
	return hash.digest('hex');
}

async function choosePort() {
	const server = createServer();
	await new Promise((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', resolve);
	});
	const address = server.address();
	if (!address || typeof address === 'string') throw new Error('could not reserve an IPv4 port');
	await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
	return address.port;
}

function command(name, argv, logDirectory) {
	const startedAt = new Date().toISOString();
	const started = Date.now();
	const child = spawn('godot', argv, {
		cwd: fixtureRoot,
		env: { ...process.env },
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	const stdoutPath = path.join(logDirectory, `${name}.stdout.log`);
	const stderrPath = path.join(logDirectory, `${name}.stderr.log`);
	child.stdout.pipe(createWriteStream(stdoutPath));
	child.stderr.pipe(createWriteStream(stderrPath));
	const finished = new Promise(resolve => {
		child.on('error', error => resolve({
			name,
			pid: child.pid ?? null,
			argv,
			started_at: startedAt,
			elapsed_ms: Date.now() - started,
			exit_code: null,
			signal: null,
			spawn_error: error.message,
		}));
		child.on('close', (exitCode, signal) => resolve({
			name,
			pid: child.pid ?? null,
			argv,
			started_at: startedAt,
			elapsed_ms: Date.now() - started,
			exit_code: exitCode,
			signal,
			spawn_error: null,
		}));
	});
	return { child, finished };
}

async function waitForFile(file, deadline) {
	while (Date.now() < deadline) {
		if ((await stat(file).catch(() => null))?.isFile()) return;
		await new Promise(resolve => setTimeout(resolve, 25));
	}
	throw new Error(`timed out waiting for ${path.basename(file)}`);
}

async function waitForCompletion(processes, deadline) {
	const completion = Promise.all(processes.map(item => item.finished));
	const remaining = Math.max(1, deadline - Date.now());
	const timed = await Promise.race([
		completion,
		new Promise(resolve => setTimeout(() => resolve(null), remaining)),
	]);
	if (timed) return timed;
	for (const processInfo of processes) processInfo.child.kill('SIGTERM');
	const afterTermination = await completion;
	throw new Error(`timed out waiting for Godot processes: ${JSON.stringify(afterTermination)}`);
}

function startDelayLossProxy({ listenPort, serverPort }) {
	const front = dgram.createSocket('udp4');
	const profile = {
		schema: 'cosmon-enet-delay-loss-observation/v1',
		test_world: true,
		proxy: 'node-udp-forwarder',
		configured: {
			one_way_delay_ms: oneWayDelayMs,
			expected_round_trip_delay_ms: oneWayDelayMs * 2,
			intentional_drops_per_direction_per_client: requestedLossesPerDirection,
		},
		observed: {
			client_to_server_packets: 0,
			server_to_client_packets: 0,
			client_to_server_drops: 0,
			server_to_client_drops: 0,
			flows: [],
		},
	};
	const flows = new Map();
	let closed = false;

	function schedule(socket, message, port, host) {
		setTimeout(() => {
			if (!closed) socket.send(message, port, host);
		}, oneWayDelayMs);
	}

	function flowFor(client) {
		const key = `${client.address}:${client.port}`;
		let flow = flows.get(key);
		if (flow) return flow;
		const upstream = dgram.createSocket('udp4');
		flow = {
			key,
			client,
			upstream,
			client_to_server_seen: 0,
			server_to_client_seen: 0,
			client_to_server_dropped: 0,
			server_to_client_dropped: 0,
		};
		upstream.on('message', message => {
			flow.server_to_client_seen += 1;
			profile.observed.server_to_client_packets += 1;
			if (flow.server_to_client_dropped < requestedLossesPerDirection) {
				flow.server_to_client_dropped += 1;
				profile.observed.server_to_client_drops += 1;
				return;
			}
			schedule(front, message, flow.client.port, flow.client.address);
		});
		upstream.bind(0, '127.0.0.1');
		flows.set(key, flow);
		return flow;
	}

	front.on('message', (message, client) => {
		const flow = flowFor(client);
		flow.client_to_server_seen += 1;
		profile.observed.client_to_server_packets += 1;
		if (flow.client_to_server_dropped < requestedLossesPerDirection) {
			flow.client_to_server_dropped += 1;
			profile.observed.client_to_server_drops += 1;
			return;
		}
		schedule(flow.upstream, message, serverPort, '127.0.0.1');
	});

	const ready = new Promise((resolve, reject) => {
		front.once('error', reject);
		front.bind(listenPort, '127.0.0.1', resolve);
	});
	return {
		ready,
		async close() {
			closed = true;
			profile.observed.flows = [...flows.values()].map(flow => ({
				client: flow.key,
				client_to_server_packets: flow.client_to_server_seen,
				server_to_client_packets: flow.server_to_client_seen,
				client_to_server_drops: flow.client_to_server_dropped,
				server_to_client_drops: flow.server_to_client_dropped,
			}));
			for (const flow of flows.values()) flow.upstream.close();
			await new Promise(resolve => front.close(resolve));
			return profile;
		},
	};
}

async function fingerprintProject() {
	const files = ['project.godot', 'main.tscn', 'main.gd', 'network_runner.tscn', 'network_runner.gd', 'export_presets.cfg'];
	const entries = [];
	for (const relative of files) {
		const file = path.join(gameRoot, relative);
		entries.push({ path: `game/${relative}`, sha256: await sha256(file) });
	}
	return { schema: 'cosmon-project-fingerprint/v1', test_world: true, files: entries };
}

await mkdir(output, { recursive: true });
const processLogs = path.join(output, 'game-process-logs');
await mkdir(processLogs, { recursive: true });
await writeJson(path.join(output, 'project-fingerprint.json'), await fingerprintProject());

const serverPort = await choosePort();
const proxyPort = await choosePort();
const deadline = Date.now() + timeoutMs;
const shared = ['--headless', '--path', gameRoot, 'res://network_runner.tscn', '--', '--output', output, '--timeout-ms', String(timeoutMs - 1_000)];
const server = command('server', [...shared, '--role', 'server', '--port', String(serverPort)], processLogs);
const processes = [server];
let proxy;
try {
	await waitForFile(path.join(output, 'server-ready.json'), deadline);
	proxy = startDelayLossProxy({ listenPort: proxyPort, serverPort });
	await proxy.ready;
	processes.push(
		command('driver', [...shared, '--role', 'driver', '--host', '127.0.0.1', '--port', String(proxyPort)], processLogs),
		command('unloader', [...shared, '--role', 'unloader', '--host', '127.0.0.1', '--port', String(proxyPort)], processLogs),
	);
	const results = await waitForCompletion(processes, deadline);
	const profile = await proxy.close();
	proxy = null;
	await writeJson(path.join(output, 'network-observation.json'), profile);
	await writeJson(path.join(output, 'process-observation.json'), {
		schema: 'cosmon-godot-process-observation/v1',
		test_world: true,
		godot_version: (await readFile(path.join(processLogs, 'server.stdout.log'), 'utf8').catch(() => '')),
		processes: results,
		logs_directory: 'game-process-logs'
	});
	const report = JSON.parse(await readFile(path.join(output, 'server-report.json'), 'utf8'));
	await writeJson(path.join(output, 'input-trace.json'), {
		schema: 'cosmon-server-observed-input-trace/v1',
		test_world: true,
		transport: report.transport,
		actions: report.received_actions
	});
	if (results.some(result => result.exit_code !== 0 || result.spawn_error)) {
		throw new Error(`one or more Godot processes failed: ${JSON.stringify(results)}`);
	}
	console.log(JSON.stringify({ server_port: serverPort, proxy_port: proxyPort, process_count: results.length }));
} finally {
	if (proxy) await proxy.close();
}
