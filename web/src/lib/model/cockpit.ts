import type { CockpitView } from './generated/cockpit';
import type { CompanyCatalogEntry } from '../product/contracts';

export type { CompanyCatalogEntry } from '../product/contracts';

export type {
	CockpitEffectReceipt as EffectReceipt,
	CockpitLegalProfile as LegalProfile,
	CockpitMoneyEnvelope as MoneyEnvelope,
	CockpitPaymentIntent as PaymentIntent,
	CockpitPerson,
	CockpitTeam,
	CockpitView
} from './generated/cockpit';

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
