#!/usr/bin/env node

// A deliberately small, Runtime-owned scenario runner. It runs ordinary
// scenario-local commands and writes ordinary evidence files. It does not know
// about Work state, scheduling, retries, agents, external effects, or whether
// a human accepts the result.

import { createHash, randomUUID } from 'node:crypto';
import { createReadStream } from 'node:fs';
import {
	mkdir,
	readdir,
	readFile,
	stat,
	writeFile,
} from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { spawn } from 'node:child_process';

const PACKAGE_SCHEMA = 'restless.scenario-package/v1';
const RUN_SCHEMA = 'restless.scenario-run/v1';
const DOCTOR_SCHEMA = 'restless.scenario-doctor/v1';
const LOG_LIMIT_BYTES = 64 * 1024;
const DEFAULT_PHASE_TIMEOUT_MS = 60_000;
const MAX_PHASE_TIMEOUT_MS = 5 * 60_000;
const ID_PATTERN = /^[a-z][a-z0-9-]{1,62}$/;

function usage() {
	console.error(`usage:
  restless-scenario validate <package-directory>
  restless-scenario doctor <package-directory>
  restless-scenario run <package-directory> --output <new-output-directory> [--seed <seed>]

The runner verifies only declared mechanical facts. A \"verified\" run still
requires the package's declared human/lead review before it becomes accepted work.`);
}

function fail(message) {
	throw new Error(message);
}

function isObject(value) {
	return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function string(value, field, { optional = false, max = 240 } = {}) {
	if (optional && value === undefined) return undefined;
	if (typeof value !== 'string' || value.trim() === '' || value.length > max || value.includes('\0')) {
		fail(`${field} must be a non-empty string of at most ${max} characters`);
	}
	return value;
}

function boolean(value, field, { optional = false } = {}) {
	if (optional && value === undefined) return undefined;
	if (typeof value !== 'boolean') fail(`${field} must be a boolean`);
	return value;
}

function relativePath(value, field) {
	string(value, field, { max: 512 });
	if (path.isAbsolute(value) || value.split(/[\\/]/).includes('..')) {
		fail(`${field} must be a relative path contained by the scenario output directory`);
	}
	return value;
}

function command(value, field) {
	if (!Array.isArray(value) || value.length === 0 || value.length > 32) {
		fail(`${field} must be a non-empty argv array with at most 32 entries`);
	}
	return value.map((argument, index) => string(argument, `${field}[${index}]`, { max: 4096 }));
}

function unique(items, field) {
	const seen = new Set();
	for (const item of items) {
		if (seen.has(item)) fail(`${field} contains a duplicate ${JSON.stringify(item)}`);
		seen.add(item);
	}
}

function validateManifest(manifest) {
	if (!isObject(manifest)) fail('scenario.json must contain an object');
	if (manifest.schema !== PACKAGE_SCHEMA) fail(`scenario.json.schema must be ${PACKAGE_SCHEMA}`);
	string(manifest.id, 'scenario.json.id', { max: 63 });
	if (!ID_PATTERN.test(manifest.id)) {
		fail('scenario.json.id must be lowercase kebab-case and begin with a letter');
	}
	string(manifest.version, 'scenario.json.version', { max: 80 });
	string(manifest.title, 'scenario.json.title', { max: 240 });
	if (!['test_world_only', 'internal_only'].includes(manifest.run_kind)) {
		fail('scenario.json.run_kind must be test_world_only or internal_only');
	}
	boolean(manifest.human_review_required, 'scenario.json.human_review_required');

	if (!Array.isArray(manifest.capabilities) || manifest.capabilities.length > 24) {
		fail('scenario.json.capabilities must be an array with at most 24 entries');
	}
	const capabilityIds = [];
	for (const [index, capability] of manifest.capabilities.entries()) {
		if (!isObject(capability)) fail(`capabilities[${index}] must be an object`);
		string(capability.id, `capabilities[${index}].id`, { max: 80 });
		capabilityIds.push(capability.id);
		command(capability.argv, `capabilities[${index}].argv`);
		boolean(capability.required, `capabilities[${index}].required`);
		if (capability.timeout_ms !== undefined) {
			if (!Number.isInteger(capability.timeout_ms) || capability.timeout_ms < 100 || capability.timeout_ms > 10_000) {
				fail(`capabilities[${index}].timeout_ms must be an integer between 100 and 10000`);
			}
		}
	}
	unique(capabilityIds, 'scenario.json.capabilities');

	if (!Array.isArray(manifest.phases) || manifest.phases.length > 8) {
		fail('scenario.json.phases must be an array with at most 8 entries');
	}
	const phaseIds = [];
	for (const [index, phase] of manifest.phases.entries()) {
		if (!isObject(phase)) fail(`phases[${index}] must be an object`);
		string(phase.id, `phases[${index}].id`, { max: 80 });
		if (!ID_PATTERN.test(phase.id)) fail(`phases[${index}].id must be lowercase kebab-case`);
		phaseIds.push(phase.id);
		command(phase.argv, `phases[${index}].argv`);
		if (phase.timeout_ms !== undefined) {
			if (!Number.isInteger(phase.timeout_ms) || phase.timeout_ms < 100 || phase.timeout_ms > MAX_PHASE_TIMEOUT_MS) {
				fail(`phases[${index}].timeout_ms must be an integer between 100 and ${MAX_PHASE_TIMEOUT_MS}`);
			}
		}
	}
	unique(phaseIds, 'scenario.json.phases');

	if (!Array.isArray(manifest.evidence) || manifest.evidence.length === 0 || manifest.evidence.length > 32) {
		fail('scenario.json.evidence must be a non-empty array with at most 32 entries');
	}
	const evidenceIds = [];
	for (const [index, evidence] of manifest.evidence.entries()) {
		if (!isObject(evidence)) fail(`evidence[${index}] must be an object`);
		string(evidence.id, `evidence[${index}].id`, { max: 80 });
		evidenceIds.push(evidence.id);
		relativePath(evidence.path, `evidence[${index}].path`);
		boolean(evidence.required, `evidence[${index}].required`);
		string(evidence.label, `evidence[${index}].label`, { optional: true, max: 240 });
	}
	unique(evidenceIds, 'scenario.json.evidence');

	if (!isObject(manifest.review_target)) fail('scenario.json.review_target must be an object');
	if (!['file', 'url'].includes(manifest.review_target.kind)) {
		fail('scenario.json.review_target.kind must be file or url');
	}
	string(manifest.review_target.label, 'scenario.json.review_target.label', { max: 240 });
	if (manifest.review_target.kind === 'file') {
		relativePath(manifest.review_target.target, 'scenario.json.review_target.target');
	} else {
		string(manifest.review_target.target, 'scenario.json.review_target.target', { max: 2048 });
	}

	return manifest;
}

async function sha256File(file) {
	const hash = createHash('sha256');
	await new Promise((resolve, reject) => {
		const stream = createReadStream(file);
		stream.on('data', chunk => hash.update(chunk));
		stream.on('error', reject);
		stream.on('end', resolve);
	});
	return hash.digest('hex');
}

function sha256Buffer(value) {
	return createHash('sha256').update(value).digest('hex');
}

function containedPath(root, relative, field) {
	const resolved = path.resolve(root, relative);
	const boundary = `${path.resolve(root)}${path.sep}`;
	if (resolved !== path.resolve(root) && !resolved.startsWith(boundary)) {
		fail(`${field} escapes its declared root`);
	}
	return resolved;
}

async function loadPackage(packageDirectory) {
	const root = path.resolve(packageDirectory);
	const rootStat = await stat(root).catch(() => null);
	if (!rootStat?.isDirectory()) fail(`scenario package is not a directory: ${root}`);
	const manifestPath = path.join(root, 'scenario.json');
	const raw = await readFile(manifestPath).catch(() => fail(`scenario package has no scenario.json: ${root}`));
	let parsed;
	try {
		parsed = JSON.parse(raw);
	} catch (error) {
		fail(`scenario.json is not valid JSON: ${error.message}`);
	}
	return { root, manifest: validateManifest(parsed), manifestSha256: sha256Buffer(raw) };
}

function parseCli(argv) {
	const [action, packageDirectory, ...options] = argv;
	if (!action || !packageDirectory || !['validate', 'doctor', 'run'].includes(action)) {
		usage();
		process.exit(2);
	}
	const parsed = { action, packageDirectory, output: null, seed: 'default' };
	for (let index = 0; index < options.length; index += 1) {
		const option = options[index];
		if (option === '--output') {
			if (!options[index + 1] || parsed.output) fail('provide --output exactly once');
			parsed.output = options[++index];
		} else if (option === '--seed') {
			if (!options[index + 1]) fail('provide a value after --seed');
			parsed.seed = options[++index];
		} else {
			fail(`unknown option ${option}`);
		}
	}
	if (action !== 'run' && (parsed.output || parsed.seed !== 'default')) {
		fail('--output and --seed are valid only with run');
	}
	if (action === 'run' && !parsed.output) fail('run requires --output <new-output-directory>');
	return parsed;
}

function boundedCapture() {
	const chunks = [];
	let bytes = 0;
	let truncated = false;
	return {
		append(chunk) {
			const buffer = Buffer.from(chunk);
			const remaining = LOG_LIMIT_BYTES - bytes;
			if (remaining <= 0) {
				truncated = true;
				return;
			}
			if (buffer.length > remaining) {
				chunks.push(buffer.subarray(0, remaining));
				bytes += remaining;
				truncated = true;
				return;
			}
			chunks.push(buffer);
			bytes += buffer.length;
		},
		result() {
			return { text: Buffer.concat(chunks).toString('utf8'), truncated };
		},
	};
}

async function execute(argv, { cwd, env, timeoutMs }) {
	const startedAt = new Date().toISOString();
	const started = Date.now();
	return await new Promise(resolve => {
		let child;
		try {
			child = spawn(argv[0], argv.slice(1), {
				cwd,
				env,
				shell: false,
				windowsHide: true,
			});
		} catch (error) {
			resolve({
				argv,
				started_at: startedAt,
				elapsed_ms: Date.now() - started,
				exit_code: null,
				signal: null,
				timed_out: false,
				spawn_error: error.message,
				stdout: '',
				stderr: '',
				stdout_truncated: false,
				stderr_truncated: false,
			});
			return;
		}
		const stdout = boundedCapture();
		const stderr = boundedCapture();
		let spawnError = null;
		let timedOut = false;
		const timeout = setTimeout(() => {
			timedOut = true;
			child.kill('SIGTERM');
		}, timeoutMs);
		child.stdout?.on('data', chunk => stdout.append(chunk));
		child.stderr?.on('data', chunk => stderr.append(chunk));
		child.on('error', error => {
			spawnError = error.message;
		});
		child.on('close', (exitCode, signal) => {
			clearTimeout(timeout);
			const stdoutResult = stdout.result();
			const stderrResult = stderr.result();
			resolve({
				argv,
				started_at: startedAt,
				elapsed_ms: Date.now() - started,
				exit_code: exitCode,
				signal,
				timed_out: timedOut,
				spawn_error: spawnError,
				stdout: stdoutResult.text,
				stderr: stderrResult.text,
				stdout_truncated: stdoutResult.truncated,
				stderr_truncated: stderrResult.truncated,
			});
		});
	});
}

async function probeCapabilities(pkg) {
	const results = [];
	for (const capability of pkg.manifest.capabilities) {
		const execution = await execute(capability.argv, {
			cwd: pkg.root,
			env: process.env,
			timeoutMs: capability.timeout_ms ?? 5_000,
		});
		results.push({
			id: capability.id,
			required: capability.required,
			available: execution.exit_code === 0 && !execution.timed_out && !execution.spawn_error,
			...execution,
		});
	}
	return results;
}

async function emptyOrNewOutput(packageRoot, rawOutput) {
	const output = path.resolve(rawOutput);
	if (output === path.parse(output).root || output === packageRoot || output.startsWith(`${packageRoot}${path.sep}`)) {
		fail('scenario output must be a new directory outside the source package');
	}
	const current = await stat(output).catch(() => null);
	if (current) {
		if (!current.isDirectory()) fail(`scenario output is not a directory: ${output}`);
		const entries = await readdir(output);
		if (entries.length > 0) fail(`scenario output already contains evidence; choose a new directory: ${output}`);
	} else {
		await mkdir(output, { recursive: true });
	}
	await mkdir(path.join(output, 'logs'), { recursive: true });
	return output;
}

async function writeJson(file, value) {
	await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
}

async function writePhaseLogs(output, phaseId, result) {
	const stdoutPath = path.join(output, 'logs', `${phaseId}.stdout.log`);
	const stderrPath = path.join(output, 'logs', `${phaseId}.stderr.log`);
	await writeFile(stdoutPath, result.stdout);
	await writeFile(stderrPath, result.stderr);
	return {
		stdout: path.relative(output, stdoutPath),
		stderr: path.relative(output, stderrPath),
	};
}

async function collectEvidence(pkg, output) {
	const evidence = [];
	for (const declaration of pkg.manifest.evidence) {
		const file = containedPath(output, declaration.path, `evidence ${declaration.id}`);
		const fileStat = await stat(file).catch(() => null);
		if (!fileStat?.isFile()) {
			evidence.push({
				id: declaration.id,
				label: declaration.label ?? declaration.id,
				path: declaration.path,
				required: declaration.required,
				available: false,
			});
			continue;
		}
		evidence.push({
			id: declaration.id,
			label: declaration.label ?? declaration.id,
			path: declaration.path,
			required: declaration.required,
			available: true,
			bytes: fileStat.size,
			sha256: await sha256File(file),
		});
	}
	return evidence;
}

async function reviewTarget(pkg, output) {
	const declaration = pkg.manifest.review_target;
	if (declaration.kind === 'url') {
		return { ...declaration, available: true };
	}
	const file = containedPath(output, declaration.target, 'review_target');
	const fileStat = await stat(file).catch(() => null);
	return { ...declaration, available: Boolean(fileStat?.isFile()) };
}

async function runScenario(pkg, output, seed) {
	const runId = `${pkg.manifest.id}-${randomUUID()}`;
	const startedAt = new Date().toISOString();
	const capabilities = await probeCapabilities(pkg);
	const blocked = capabilities.filter(capability => capability.required && !capability.available);
	const base = {
		schema: RUN_SCHEMA,
		run_id: runId,
		run_kind: pkg.manifest.run_kind,
		scenario: {
			id: pkg.manifest.id,
			version: pkg.manifest.version,
			title: pkg.manifest.title,
			manifest_sha256: pkg.manifestSha256,
		},
		seed,
		started_at: startedAt,
		human_review_required: pkg.manifest.human_review_required,
		acceptance: pkg.manifest.human_review_required
			? 'requires_human_or_lead_review'
			: 'mechanical_evidence_only',
		capabilities,
		phases: [],
	};
	if (blocked.length > 0) {
		const result = {
			...base,
			mechanical_status: 'blocked',
			blocked_by: blocked.map(capability => capability.id),
			completed_at: new Date().toISOString(),
			evidence: await collectEvidence(pkg, output),
			review_target: await reviewTarget(pkg, output),
		};
		await writeJson(path.join(output, 'run-manifest.json'), result);
		return result;
	}

	const env = {
		...process.env,
		RESTLESS_SCENARIO_ID: pkg.manifest.id,
		RESTLESS_SCENARIO_VERSION: pkg.manifest.version,
		RESTLESS_SCENARIO_RUN_ID: runId,
		RESTLESS_SCENARIO_SEED: seed,
		RESTLESS_SCENARIO_OUTPUT: output,
	};
	let failed = false;
	for (const phase of pkg.manifest.phases) {
		if (failed) break;
		const execution = await execute(phase.argv, {
			cwd: pkg.root,
			env,
			timeoutMs: phase.timeout_ms ?? DEFAULT_PHASE_TIMEOUT_MS,
		});
		const logs = await writePhaseLogs(output, phase.id, execution);
		base.phases.push({
			id: phase.id,
			argv: execution.argv,
			started_at: execution.started_at,
			elapsed_ms: execution.elapsed_ms,
			exit_code: execution.exit_code,
			signal: execution.signal,
			timed_out: execution.timed_out,
			spawn_error: execution.spawn_error,
			stdout: logs.stdout,
			stderr: logs.stderr,
			stdout_truncated: execution.stdout_truncated,
			stderr_truncated: execution.stderr_truncated,
		});
		failed = execution.exit_code !== 0 || execution.timed_out || execution.spawn_error !== null;
	}

	const evidence = await collectEvidence(pkg, output);
	const target = await reviewTarget(pkg, output);
	const missingEvidence = evidence.filter(item => item.required && !item.available);
	const result = {
		...base,
		mechanical_status: failed || missingEvidence.length > 0 || !target.available ? 'failed' : 'verified',
		completed_at: new Date().toISOString(),
		evidence,
		review_target: target,
	};
	if (missingEvidence.length > 0) result.missing_evidence = missingEvidence.map(item => item.id);
	if (!target.available) result.missing_review_target = target.target;
	await writeJson(path.join(output, 'run-manifest.json'), result);
	return result;
}

async function main() {
	const cli = parseCli(process.argv.slice(2));
	const pkg = await loadPackage(cli.packageDirectory);
	if (cli.action === 'validate') {
		console.log(JSON.stringify({ ok: true, schema: PACKAGE_SCHEMA, id: pkg.manifest.id, version: pkg.manifest.version }, null, 2));
		return;
	}
	if (cli.action === 'doctor') {
		const capabilities = await probeCapabilities(pkg);
		const blocked = capabilities.filter(capability => capability.required && !capability.available);
		console.log(JSON.stringify({
			schema: DOCTOR_SCHEMA,
			scenario: { id: pkg.manifest.id, version: pkg.manifest.version },
			status: blocked.length > 0 ? 'blocked' : 'available',
			blocked_by: blocked.map(capability => capability.id),
			capabilities,
		}, null, 2));
		if (blocked.length > 0) process.exitCode = 2;
		return;
	}
	const output = await emptyOrNewOutput(pkg.root, cli.output);
	const result = await runScenario(pkg, output, cli.seed);
	console.log(JSON.stringify({
		run_manifest: path.join(output, 'run-manifest.json'),
		mechanical_status: result.mechanical_status,
		acceptance: result.acceptance,
	}, null, 2));
	if (result.mechanical_status === 'blocked') process.exitCode = 2;
	if (result.mechanical_status === 'failed') process.exitCode = 1;
}

main().catch(error => {
	console.error(`restless-scenario: ${error.message}`);
	process.exitCode = 1;
});
