/* Archiving an agent and re-hiring one (S88-01): owner only. OrgIntel decides whether it may happen
 * and says why not; a refusal comes back as a sentence the page shows as written. */
import { ownerJson } from './failure';

const actors = (company: string) => `/api/companies/${encodeURIComponent(company)}/actors`;

export interface ArchivedAgent {
	actor_id: string;
	display: string;
	role: string;
	kind: string;
	team_id: string | null;
	model: string | null;
	retired_at: string;
	retired_by: string | null;
	retirement_reason: string;
}

/** Archived agents, newest first. A daemon without the list yet has none to show. */
export async function listArchivedAgents(company: string): Promise<ArchivedAgent[]> {
	const response = await fetch(`${actors(company)}/archived`, { credentials: 'same-origin' });
	if (response.status === 404) return [];
	return (await ownerJson<{ agents: ArchivedAgent[] }>(response)).agents;
}

function staffing(company: string, actorId: string, action: 'archive' | 'rehire', reason = '') {
	return fetch(`${actors(company)}/${encodeURIComponent(actorId)}/${action}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(reason.trim() ? { reason: reason.trim() } : {}),
		credentials: 'same-origin'
	}).then((response) => ownerJson<{ archived: boolean; team_id: string | null }>(response));
}

export const archiveAgent = (company: string, actorId: string, reason = '') =>
	staffing(company, actorId, 'archive', reason);
export const rehireAgent = (company: string, actorId: string, reason = '') =>
	staffing(company, actorId, 'rehire', reason);
