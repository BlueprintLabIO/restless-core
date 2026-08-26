#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const runner = path.join(directory, 'restless-scenario.mjs');

async function command(argv, cwd) {
	return await new Promise((resolve, reject) => {
		const child = spawn(process.execPath, [runner, ...argv], { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', chunk => (stdout += chunk));
		child.stderr.on('data', chunk => (stderr += chunk));
		child.on('error', reject);
		child.on('close', code => resolve({ code, stdout, stderr }));
	});
}

function manifest({ capability = ['node', '--version'], phase = ['node', 'phase.mjs'], evidence = 'proof.txt' } = {}) {
	return {
		schema: 'restless.scenario-package/v1',
		id: 'runner-test',
		version: '1',
		title: 'Runner test',
		run_kind: 'test_world_only',
		human_review_required: true,
		capabilities: [{ id: 'runtime', argv: capability, required: true }],
		phases: [{ id: 'exercise', argv: phase, timeout_ms: 5000 }],
		evidence: [{ id: 'proof', path: evidence, required: true }],
		review_target: { kind: 'file', target: evidence, label: 'Proof file' },
	};
}

async function makePackage(root, options = {}) {
	const packageDirectory = path.join(root, options.name ?? 'package');
	await mkdir(packageDirectory, { recursive: true });
	await writeFile(path.join(packageDirectory, 'scenario.json'), `${JSON.stringify(manifest(options), null, 2)}\n`);
	await writeFile(
		path.join(packageDirectory, 'phase.mjs'),
		options.phaseSource ?? "import { writeFile } from 'node:fs/promises'; await writeFile(`${process.env.RESTLESS_SCENARIO_OUTPUT}/proof.txt`, 'observed');\n",
	);
	return packageDirectory;
}

async function readManifest(output) {
	return JSON.parse(await readFile(path.join(output, 'run-manifest.json'), 'utf8'));
}

const root = await mkdtemp(path.join(os.tmpdir(), 'restless-scenario-runner-'));
try {
	const packageDirectory = await makePackage(root);
	const output = path.join(root, 'verified-output');
	const verified = await command(['run', packageDirectory, '--output', output, '--seed', 'fixed-seed'], root);
	assert.equal(verified.code, 0, verified.stderr);
	const verifiedManifest = await readManifest(output);
	assert.equal(verifiedManifest.mechanical_status, 'verified');
	assert.equal(verifiedManifest.seed, 'fixed-seed');
	assert.equal(verifiedManifest.acceptance, 'requires_human_or_lead_review');
	assert.equal(verifiedManifest.evidence[0].available, true);
	assert.equal(verifiedManifest.phases[0].exit_code, 0);

	const blockedPackage = await makePackage(root, {
		name: 'blocked-package',
		capability: ['restless-scenario-command-that-does-not-exist', '--version'],
	});
	const blockedOutput = path.join(root, 'blocked-output');
	const blocked = await command(['run', blockedPackage, '--output', blockedOutput], root);
	assert.equal(blocked.code, 2, blocked.stderr);
	const blockedManifest = await readManifest(blockedOutput);
	assert.equal(blockedManifest.mechanical_status, 'blocked');
	assert.deepEqual(blockedManifest.blocked_by, ['runtime']);
	assert.equal(blockedManifest.phases.length, 0);

	const missingEvidencePackage = await makePackage(root, {
		name: 'missing-evidence-package',
		evidence: 'missing.txt',
		phaseSource: "console.log('completed without declared evidence');\n",
	});
	const missingOutput = path.join(root, 'missing-output');
	const missing = await command(['run', missingEvidencePackage, '--output', missingOutput], root);
	assert.equal(missing.code, 1, missing.stderr);
	const missingManifest = await readManifest(missingOutput);
	assert.equal(missingManifest.mechanical_status, 'failed');
	assert.deepEqual(missingManifest.missing_evidence, ['proof']);

	const noisyPackage = await makePackage(root, {
		name: 'noisy-package',
		phaseSource: "import { writeFile } from 'node:fs/promises'; console.log('x'.repeat(70 * 1024)); await writeFile(`${process.env.RESTLESS_SCENARIO_OUTPUT}/proof.txt`, 'observed');\n",
	});
	const noisyOutput = path.join(root, 'noisy-output');
	const noisy = await command(['run', noisyPackage, '--output', noisyOutput], root);
	assert.equal(noisy.code, 0, noisy.stderr);
	const noisyManifest = await readManifest(noisyOutput);
	assert.equal(noisyManifest.phases[0].stdout_truncated, true);

	console.log('scenario runner tests passed');
} finally {
	await rm(root, { recursive: true, force: true });
}
