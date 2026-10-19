import assert from 'node:assert/strict';
import test from 'node:test';
import { likelyWindows, machineStatus, machineSummary } from './machine-view.ts';

test('a computer summary says what it reported and leaves out what it did not', () => {
	assert.equal(
		machineSummary({ os: 'Windows 11 Pro', cpus: 8, memory_bytes: 32 * 1024 ** 3 }),
		'Windows 11 Pro · 8 CPUs · 32 GB'
	);
	assert.equal(machineSummary({ os: '', cpus: 1, memory_bytes: 0 }), '1 CPU');
	assert.equal(machineSummary({ os: '', cpus: 0, memory_bytes: 0 }), '');
});

test('a computer status says whether it can run anything now', () => {
	assert.equal(machineStatus({ kind: 'plane', online: false, paused: false }).label, 'Always on');
	assert.equal(machineStatus({ kind: 'own', online: true, paused: true }).label, 'Paused');
	assert.deepEqual(machineStatus({ kind: 'own', online: false, paused: false }), {
		label: 'Offline',
		tone: 'warn'
	});
	assert.equal(machineStatus({ kind: 'own', online: true, paused: false }).tone, 'ok');
});

test('Windows gets the PowerShell line', () => {
	assert.equal(likelyWindows('Mozilla/5.0 (Windows NT 10.0; Win64; x64)'), true);
	assert.equal(likelyWindows('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5)'), false);
});
