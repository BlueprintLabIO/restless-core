#!/usr/bin/env node

import { stat } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const fixtureRoot = path.dirname(fileURLToPath(import.meta.url));
const gameRoot = path.join(fixtureRoot, 'game');
const output = path.resolve(requiredEnvironment('RESTLESS_SCENARIO_OUTPUT'));
const target = path.join(output, 'cosmon-two-client-truck.exe');
const args = ['--headless', '--path', gameRoot, '--export-release', 'Windows Desktop', target];
const result = await run('godot', args);
if (result.code !== 0) throw new Error(`Godot Windows export failed (${result.code}): ${result.stderr}`);
const exported = await stat(target).catch(() => null);
if (!exported?.isFile() || exported.size < 1024) throw new Error('Godot reported success but no Windows executable was produced');
console.log(JSON.stringify({ target, bytes: exported.size }));

function requiredEnvironment(name) {
	if (!process.env[name]) throw new Error(`${name} is required`);
	return process.env[name];
}

async function run(command, args) {
	return await new Promise(resolve => {
		const child = spawn(command, args, { cwd: fixtureRoot, env: process.env, stdio: ['ignore', 'pipe', 'pipe'] });
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', chunk => (stdout += chunk));
		child.stderr.on('data', chunk => (stderr += chunk));
		child.on('error', error => resolve({ code: null, stdout, stderr, spawn_error: error.message }));
		child.on('close', code => resolve({ code, stdout, stderr, spawn_error: null }));
	});
}
