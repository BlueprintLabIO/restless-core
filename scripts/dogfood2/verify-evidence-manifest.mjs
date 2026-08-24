#!/usr/bin/env node

// Validate only the small Dogfood 2 evidence-file contract. This is not a
// provider registry or a durable source-health service.

import { readFile } from 'node:fs/promises';
import process from 'node:process';

const ACCESS_STATES = new Set([
	'available_public',
	'available_authenticated',
	'unverified_provider',
	'rate_limited',
	'unavailable',
	'unknown',
]);
const REQUIRED_FIELDS = [
	'id',
	'locator',
	'source_type',
	'claim_supported',
	'observed_at',
	'freshness_expectation',
	'access_state',
	'limitation',
];

function usage() {
	console.error('usage: node verify-evidence-manifest.mjs <manifest.json> [--require-states a,b]');
	process.exit(2);
}

function requiredStates() {
	const index = process.argv.indexOf('--require-states');
	if (index === -1) return [];
	const raw = process.argv[index + 1];
	if (!raw) usage();
	return raw.split(',').filter(Boolean);
}

const manifestPath = process.argv[2];
if (!manifestPath || manifestPath.startsWith('-')) usage();
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const errors = [];

if (manifest.schema !== 'restless.research-source-evidence/v1') {
	errors.push('schema must be restless.research-source-evidence/v1');
}
if (!['live_research', 'test_world_only'].includes(manifest.run_kind)) {
	errors.push('run_kind must be live_research or test_world_only');
}
if (!Array.isArray(manifest.sources) || manifest.sources.length === 0) {
	errors.push('sources must be a non-empty array');
}

const ids = new Set();
const states = new Set();
for (const [index, source] of (manifest.sources ?? []).entries()) {
	const prefix = `sources[${index}]`;
	if (!source || typeof source !== 'object' || Array.isArray(source)) {
		errors.push(`${prefix} must be an object`);
		continue;
	}
	for (const field of REQUIRED_FIELDS) {
		if (typeof source[field] !== 'string' || source[field].trim() === '') {
			errors.push(`${prefix}.${field} must be a non-empty string`);
		}
	}
	if (typeof source.id === 'string') {
		if (ids.has(source.id)) errors.push(`${prefix}.id is duplicated: ${source.id}`);
		ids.add(source.id);
	}
	if (!ACCESS_STATES.has(source.access_state)) {
		errors.push(`${prefix}.access_state is not recognised: ${source.access_state}`);
	} else {
		states.add(source.access_state);
	}
	if (typeof source.observed_at === 'string' && Number.isNaN(Date.parse(source.observed_at))) {
		errors.push(`${prefix}.observed_at is not an ISO timestamp`);
	}
	if (source.as_of !== null && source.as_of !== undefined && typeof source.as_of !== 'string') {
		errors.push(`${prefix}.as_of must be a string or null`);
	}
	if (source.access_state === 'available_authenticated') {
		if (typeof source.authenticated_probe !== 'object' || source.authenticated_probe === null) {
			errors.push(`${prefix} must carry an authenticated_probe object before claiming available_authenticated`);
		}
	}
}

for (const state of requiredStates()) {
	if (!states.has(state)) errors.push(`missing required access_state: ${state}`);
}

if (errors.length > 0) {
	console.error(JSON.stringify({ ok: false, errors }, null, 2));
	process.exit(1);
}

console.log(
	JSON.stringify(
		{
			ok: true,
			run_id: manifest.run_id,
			run_kind: manifest.run_kind,
			source_count: manifest.sources.length,
			access_states: [...states].sort(),
		},
		null,
		2,
	),
);
