#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const runner = path.join(directory, 'restless-scenario.mjs');
const menuPackage = path.join(directory, 'fixtures', 'thymelake-menu-launch');

async function run(argv) {
	return await new Promise((resolve, reject) => {
		const child = spawn(process.execPath, [runner, ...argv], { stdio: ['ignore', 'pipe', 'pipe'] });
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', chunk => (stdout += chunk));
		child.stderr.on('data', chunk => (stderr += chunk));
		child.on('error', reject);
		child.on('close', code => resolve({ code, stdout, stderr }));
	});
}

const root = await mkdtemp(path.join(os.tmpdir(), 'restless-scenario-fixtures-'));
try {
	const doctor = await run(['doctor', menuPackage]);
	assert.equal(doctor.code, 0, doctor.stderr);
	assert.equal(JSON.parse(doctor.stdout).status, 'available');

	const output = path.join(root, 'thymelake-menu');
	const result = await run(['run', menuPackage, '--output', output, '--seed', 'thymelake-smoke']);
	assert.equal(result.code, 0, result.stderr);
	const manifest = JSON.parse(await readFile(path.join(output, 'run-manifest.json'), 'utf8'));
	assert.equal(manifest.mechanical_status, 'verified');
	assert.equal(manifest.run_kind, 'test_world_only');
	assert.equal(manifest.human_review_required, true);
	assert.equal(manifest.evidence.every(evidence => evidence.available), true);
	const menu = JSON.parse(await readFile(path.join(output, 'normalized-menu.json'), 'utf8'));
	assert.equal(menu.items.find(item => item.id === 'main-pasta').allergens, null);
	const review = await readFile(path.join(output, 'review.md'), 'utf8');
	assert.match(review, /does not prove a restaurant launch/);
	console.log('scenario fixture tests passed');
} finally {
	await rm(root, { recursive: true, force: true });
}
