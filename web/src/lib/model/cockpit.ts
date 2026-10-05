import type { CockpitView } from './generated/cockpit';
import { responseFailure } from './failure.ts';
import { bootFetch } from './boot-fetch.ts';

export type {
	CockpitEffectReceipt as EffectReceipt,
	CockpitLegalProfile as LegalProfile,
	CockpitMoneyEnvelope as MoneyEnvelope,
	CockpitPaymentIntent as PaymentIntent,
	CockpitPerson,
	CockpitTeam,
	CockpitView
} from './generated/cockpit';

export interface CompanyCatalogEntry {
	id: string;
	name: string;
	mission: string;
	model: string;
	spend_ceiling_usd: number;
	runtime_status: 'running' | 'asleep' | 'stopped' | 'absent' | 'unavailable';
	lifecycle_status: 'active' | 'archived';
	/**
	 * Present when the account plane could not admit a model route for this
	 * company at boot. The company is configured but cannot start until the
	 * reason is resolved. Absent for every company that can start.
	 */
	unstartable_reason?: string;
	/**
	 * The portfolio card: the counts this plane would sign for Fleet (company projection v2).
	 * Absent when the company's state could not be read in time; never a guessed zero.
	 */
	card?: {
		decisions_waiting: number;
		last_activity_at: string | null;
		people_working: number;
		outcomes_last_day: number;
		exec_ready: boolean;
	};
}

export async function getCompanies(): Promise<CompanyCatalogEntry[]> {
	const response = await bootFetch('/api/companies', {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}

export async function createCompany(input: {
	display_name?: string;
	name: string;
	mission: string;
	model?: string;
}): Promise<CompanyCatalogEntry> {
	const response = await fetch('/api/companies', {
		method: 'POST',
		credentials: 'same-origin',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(input)
	});
	if (!response.ok) {
		let message = 'Could not create the company. Please try again.';
		try {
			message = (await response.json()).message ?? message;
		} catch {
			/* Keep a useful transport error. */
		}
		throw new Error(message);
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
	if (!response.ok) throw await responseFailure(response);
}

export function archiveCompany(company: string): Promise<void> {
	return changeCompanyLifecycle(company, 'archive');
}

export function restoreCompany(company: string): Promise<void> {
	return changeCompanyLifecycle(company, 'restore');
}

export async function getCockpit(company: string, probeCredentials = false): Promise<CockpitView> {
	const query = probeCredentials ? '?probe_credentials=true' : '';
	const response = await bootFetch(
		`/api/companies/${encodeURIComponent(company)}/cockpit${query}`,
		{
			credentials: 'same-origin',
			cache: 'no-store'
		}
	);
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}

/** The runtime truth used by the Exec rail. Presence in config is not enough. */
export function execCanReceive(view: CockpitView | null): boolean {
	return actorCanReceive(view, 'exec');
}

/** Conversation route availability follows durable OrgIntel identity. This is
 * not a probe of ACP session health or whether a turn is running. */
export function actorCanReceive(view: CockpitView | null, actorId: string): boolean {
	if (!view || view.source_health.orgintel !== 'available') return false;
	return (
		actorId === 'exec' ||
		view.teams.some(
			(team) =>
				team.lead_actor_id === actorId &&
				view.people.some((person) => person.actor_id === actorId && person.kind === 'staff')
		)
	);
}

/** Stable identity tile derived from the actor id, independent of display order. */
export function personTone(actor: { id?: string; actor_id?: string }): number {
	const id = actor.id ?? actor.actor_id ?? '';
	let hash = 0;
	for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) | 0;
	return Math.abs(hash) % 5;
}
