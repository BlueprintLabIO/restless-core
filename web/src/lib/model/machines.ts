/* Machines the owner holds, assigned to companies (S99). The plane's own host is always first;
 * the owner adds their own computers with a one-time code and one line. Only the owner manages
 * them; a company's members see that company's machines. */
import { ownerRequest, ownerSend } from './setting.svelte';

export interface MachineFacts {
	hostname: string;
	os: string;
	arch: string;
	cpus: number;
	memory_bytes: number;
	docker_version: string;
	helper_version: string;
}

export interface Machine {
	id: string;
	label: string;
	kind: 'plane' | 'own';
	facts: MachineFacts;
	paused: boolean;
	online: boolean;
	created_at: string;
	last_seen_at: string | null;
	companies: { company: string; role: 'main' | 'helper' }[];
}

export interface Pairing {
	code: string;
	expires_at: string;
	commands: { unix: string; windows: string };
}

export interface CompanyMachine {
	id: string;
	label: string;
	kind: 'plane' | 'own';
	role: 'main' | 'helper';
	online: boolean;
	paused: boolean;
	os: string;
	cpus: number;
	memory_bytes: number;
	last_seen_at: string | null;
}

export interface MoveProgress {
	to: string;
	step: string;
	done: boolean;
	error: string | null;
	started_at: string;
}

export interface CompanyMachines {
	machines: CompanyMachine[];
	can_manage: boolean;
	move: MoveProgress | null;
}

export async function listMachines(): Promise<Machine[]> {
	return (await ownerRequest<{ machines: Machine[] }>('/api/machines')).machines;
}

export function createPairing(label?: string): Promise<Pairing> {
	return ownerRequest<Pairing>('/api/machines/pairings', {
		method: 'POST',
		json: label?.trim() ? { label: label.trim() } : {}
	});
}

export function changeMachine(id: string, change: { label?: string; paused?: boolean }) {
	return ownerRequest<Machine>(`/api/machines/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		json: change
	});
}

export function removeMachine(id: string) {
	return ownerSend(`/api/machines/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

const company = (id: string) => `/api/companies/${encodeURIComponent(id)}/machines`;

export function companyMachines(id: string): Promise<CompanyMachines> {
	return ownerRequest<CompanyMachines>(company(id));
}

export function moveCompany(id: string, machine: string) {
	return ownerSend(`${company(id)}/move`, { method: 'POST', json: { machine_id: machine } });
}

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
