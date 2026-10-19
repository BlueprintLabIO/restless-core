/* How a machine reads in the owner's lists (S99). No requests here, so tests run without a page. */

/** "Windows 11 · 8 CPUs · 32 GB", from what the machine reports. */
export function machineSummary(facts: { os: string; cpus: number; memory_bytes: number }): string {
	const parts: string[] = [];
	if (facts.os) parts.push(facts.os);
	if (facts.cpus) parts.push(`${facts.cpus} CPU${facts.cpus === 1 ? '' : 's'}`);
	if (facts.memory_bytes) parts.push(`${Math.round(facts.memory_bytes / 1024 ** 3)} GB`);
	return parts.join(' · ');
}

/** What a row says about whether the machine can run anything now. */
export function machineStatus(machine: { kind: string; online: boolean; paused: boolean }): {
	label: string;
	tone: 'ok' | 'muted' | 'warn';
} {
	if (machine.kind === 'plane') return { label: 'Always on', tone: 'ok' };
	if (machine.paused) return { label: 'Paused', tone: 'muted' };
	return machine.online ? { label: 'Connected', tone: 'ok' } : { label: 'Offline', tone: 'warn' };
}

/** Which install line fits this browser's computer. */
export function likelyWindows(userAgent: string): boolean {
	return /Windows/i.test(userAgent);
}
