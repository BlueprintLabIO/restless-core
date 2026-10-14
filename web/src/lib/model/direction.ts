/* Direction and permission (S80): who may change what without asking, the requests that ask,
 * the changes applied for others with their Undo, and the standing grants. Every read and write
 * goes to /direction; the server decides who may see or answer each. See
 * docs/specs/direction-and-permission.md. */
import { responseFailure } from './failure.ts';
import type {
	DirectionApplied,
	DirectionDecisionInput,
	DirectionDigest,
	DirectionGrantList,
	DirectionRequest,
	DirectionRequestPage,
	DirectionTarget
} from './generated/direction';

function path(companyId: string, rest: string): string {
	return `/api/companies/${encodeURIComponent(companyId)}/direction/${rest}`;
}

async function read<T>(url: string): Promise<T> {
	const response = await fetch(url, { cache: 'no-store', credentials: 'same-origin' });
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

async function post<T>(url: string, body?: unknown): Promise<T> {
	const response = await fetch(url, {
		method: 'POST',
		credentials: 'same-origin',
		...(body === undefined
			? {}
			: { headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

/** Open requests routed to this person, or this person's own requests. */
export function directionRequests(
	companyId: string,
	state: 'open' | 'mine'
): Promise<DirectionRequestPage> {
	return read(path(companyId, `requests?state=${state}`));
}

/** Approve (optionally always allowing, or appointing the team's person) or decline. A request
 * someone else already answered is refused with their answer as the message. */
export function decideDirectionRequest(
	companyId: string,
	requestId: string,
	input: DirectionDecisionInput
): Promise<DirectionRequest> {
	return post(path(companyId, `requests/${encodeURIComponent(requestId)}/decision`), input);
}

/** Put back what one applied change replaced. A change made on top since is never overwritten:
 * the answer says `changed_since` instead. */
export function undoDirectionApplied(
	companyId: string,
	appliedId: string
): Promise<DirectionApplied> {
	return post(path(companyId, `applied/${encodeURIComponent(appliedId)}/undo`));
}

export function directionGrants(companyId: string): Promise<DirectionGrantList> {
	return read(path(companyId, 'grants'));
}

export function revokeDirectionGrant(companyId: string, grantId: string): Promise<unknown> {
	return post(path(companyId, `grants/${encodeURIComponent(grantId)}/revoke`));
}

export function directionDigest(companyId: string, day?: string): Promise<DirectionDigest> {
	return read(path(companyId, `digest${day ? `?day=${encodeURIComponent(day)}` : ''}`));
}

/** What a change touched, in a few words: "the Site Renovation Studio brief". */
export function targetPhrase(target: DirectionTarget | null): string {
	if (!target) return '';
	switch (target.kind) {
		case 'document':
		case 'sheet':
		case 'work':
		case 'schedule':
			return target.title;
		case 'charter':
			return 'the charter';
		case 'team_brief':
			return `the ${target.team.name} brief`;
		case 'agent':
			return `${target.name}'s setup`;
	}
}

/** Where a change can be opened, when it has a page of its own. Given when it was applied, a
 * document opens on its versions showing what changed since just before, and a sheet on its
 * history, so the change itself is what the page shows. */
export function targetHref(
	companyId: string,
	target: DirectionTarget | null,
	appliedAt?: string
): string | null {
	const company = `/${encodeURIComponent(companyId)}`;
	if (!target) return null;
	const before = appliedAt ? Date.parse(appliedAt) - 1000 : NaN;
	switch (target.kind) {
		case 'document':
			return `${company}/library/documents?document=${encodeURIComponent(target.id)}${
				Number.isNaN(before)
					? ''
					: `&inspect=versions&since=${encodeURIComponent(new Date(before).toISOString())}`
			}`;
		case 'sheet':
			return `${company}/library/sheets?sheet=${encodeURIComponent(target.id)}${appliedAt ? '&inspect=history' : ''}`;
		case 'work':
			return `${company}/work/${encodeURIComponent(target.id)}`;
		case 'charter':
			return `${company}/company/charter`;
		case 'schedule':
			return `${company}/company/schedules`;
		case 'agent':
			return `${company}/people?person=${encodeURIComponent(target.actor_id)}`;
		case 'team_brief':
			return `${company}/people`;
	}
}
