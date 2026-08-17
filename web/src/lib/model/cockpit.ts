import type { GoalRow } from './generated/orgintel';

export interface CockpitPerson {
	actor_id: string;
	role: string;
	display: string;
	model: string | null;
	team_id: string | null;
	spent_usd: number;
	session_running: boolean;
	model_cooldown: {
		model: string;
		kind: string;
		reason: string;
		retry_at: string;
	} | null;
}

export interface CockpitTeam {
	id: string;
	name: string;
	brief: string;
	lead_actor_id: string;
	created_by: string;
	created_at: string;
	member_count: number;
	in_motion_count: number;
	blocked_count: number;
}

export interface EffectReceipt {
	id: number;
	effect_class: string | null;
	tool: string | null;
	success: boolean | null;
	party: string | null;
	actor: string | null;
	outcome: unknown;
	evidence_quality: 'governed' | 'legacy_unverified';
	at: string;
}

export interface CockpitView {
	company: { id: string; name: string; mission: string; model: string };
	source_health: Record<'orgintel' | 'authority' | 'runtime', string>;
	people: CockpitPerson[];
	teams: CockpitTeam[];
	goals: GoalRow[];
	spend: {
		accounted_usd: number;
		ceiling_usd: number;
		remaining_usd: number | null;
		poisoned: boolean;
	};
	authority: {
		approved_parties: string[];
		credentials: Array<{ binding: string; status: string; detail: string }>;
	};
	receipts: EffectReceipt[];
	refreshed_at: string;
}

export interface CompanyCatalogEntry {
	id: string;
	name: string;
	mission: string;
	model: string;
	spend_ceiling_usd: number;
	runtime_status: 'running' | 'stopped' | 'absent' | 'unavailable';
	lifecycle_status: 'active' | 'archived';
}

export async function getCompanies(): Promise<CompanyCatalogEntry[]> {
	const response = await fetch('/api/companies', {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = await response.json();
			message = body.message ?? message;
		} catch {
			// Preserve the transport status when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	return response.json();
}

async function changeCompanyLifecycle(
	company: string,
	action: 'archive' | 'restore'
): Promise<void> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/${action}`, {
		method: 'POST',
		credentials: 'same-origin'
	});
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = await response.json();
			message = body.message ?? message;
		} catch {
			// Preserve the transport status when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
}

export function archiveCompany(company: string): Promise<void> {
	return changeCompanyLifecycle(company, 'archive');
}

export function restoreCompany(company: string): Promise<void> {
	return changeCompanyLifecycle(company, 'restore');
}

export async function signOut(): Promise<void> {
	const response = await fetch('/api/session', {
		method: 'DELETE',
		credentials: 'same-origin'
	});
	if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
}

export async function getCockpit(company: string, probeCredentials = false): Promise<CockpitView> {
	const query = probeCredentials ? '?probe_credentials=true' : '';
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/cockpit${query}`, {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = await response.json();
			message = body.message ?? message;
		} catch {
			// Preserve the transport status when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	return response.json();
}

/** The runtime truth used by the Exec rail. Presence in config is not enough. */
export function execCanReceive(view: CockpitView | null): boolean {
	return actorCanReceive(view, 'exec');
}

/** Conversation availability follows durable OrgIntel identity, not whether
 * that actor happens to have a model process running in this instant. */
export function actorCanReceive(view: CockpitView | null, actorId: string): boolean {
	if (!view || view.source_health.orgintel !== 'available') return false;
	return view.people.some((person) => person.actor_id === actorId);
}

/** Stable identity tile derived from the actor id, independent of display order. */
export function personTone(actor: { id?: string; actor_id?: string }): number {
	const id = actor.id ?? actor.actor_id ?? '';
	let hash = 0;
	for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) | 0;
	return Math.abs(hash) % 5;
}
