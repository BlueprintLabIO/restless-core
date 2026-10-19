import { describe, expect, it } from 'vitest';
import { likelyWindows, machineStatus, machineSummary } from './machines';

describe('machines', () => {
	it('summarizes what a computer reports, leaving out what it did not', () => {
		expect(machineSummary({ os: 'Windows 11 Pro', cpus: 8, memory_bytes: 32 * 1024 ** 3 })).toBe(
			'Windows 11 Pro · 8 CPUs · 32 GB'
		);
		expect(machineSummary({ os: '', cpus: 1, memory_bytes: 0 })).toBe('1 CPU');
		expect(machineSummary({ os: '', cpus: 0, memory_bytes: 0 })).toBe('');
	});

	it('says whether a computer can run anything now', () => {
		expect(machineStatus({ kind: 'plane', online: false, paused: false }).label).toBe('Always on');
		expect(machineStatus({ kind: 'own', online: true, paused: true }).label).toBe('Paused');
		expect(machineStatus({ kind: 'own', online: false, paused: false })).toEqual({
			label: 'Offline',
			tone: 'warn'
		});
		expect(machineStatus({ kind: 'own', online: true, paused: false }).tone).toBe('ok');
	});

	it('offers the Windows line on Windows', () => {
		expect(likelyWindows('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe(true);
		expect(likelyWindows('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5)')).toBe(false);
	});
});
