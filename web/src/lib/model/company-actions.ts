/**
 * One-off owner actions that views used to send themselves (Sprint 82: views never call `fetch`).
 * Each returns what its view needs: a parsed answer, or the raw response where the view decides by
 * status (the company computer's leases).
 */
import { responseFailure } from './failure';
import { ownerRequest } from './setting.svelte';

const api = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

export function decideEmailMandateProposal(
	company: string,
	proposal: string,
	decision: 'approve' | 'decline',
	/** Why, for a decline: it reaches Exec with the decision. */
	note = ''
) {
	return ownerRequest<unknown>(
		`${api(company)}/email-mandates/proposals/${encodeURIComponent(proposal)}/decision`,
		{
			method: 'POST',
			json: { decision, ...(note.trim() ? { owner_note: note.trim() } : {}) }
		}
	);
}

export function setTeamOwner(company: string, team: string, person: string | null, reason: string) {
	return ownerRequest<unknown>(`${api(company)}/teams/${encodeURIComponent(team)}/owner`, {
		method: 'POST',
		json: { person, reason }
	});
}

/** Archive or restore a Library entry; the view reads the failure from the response. */
export function setLibraryArchived(
	company: string,
	entry: { kind: string; id: string },
	archived: boolean
): Promise<Response> {
	const base = api(company);
	const [path, body] =
		entry.kind === 'doc'
			? [`${base}/documents/${encodeURIComponent(entry.id)}/archive`, { archived }]
			: entry.kind === 'sheet'
				? [`${base}/sheets/${encodeURIComponent(entry.id)}/archive`, { archived }]
				: [`${base}/library/archive`, { artifact_id: entry.id, archived }];
	return fetch(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify(body)
	});
}

/** Ask for the computer's display size; the view acts on 401 (the attachment lapsed). */
export function requestDisplayLease(
	company: string,
	body: { client_id: string; width: number; height: number }
): Promise<Response> {
	return fetch(`${api(company)}/desktop/display-lease`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		credentials: 'same-origin'
	});
}

/** Follow a one-use desktop ticket to its client route; the view checks where it landed. */
export function resolveDesktopRoute(route: string): Promise<Response> {
	return fetch(route, { credentials: 'same-origin', redirect: 'follow' });
}

export async function readStartupDoctor<T>(company: string): Promise<T> {
	const response = await fetch(`${api(company)}/startup-doctor`, {
		cache: 'no-store',
		credentials: 'same-origin'
	});
	if (!response.ok) throw new Error(`Startup check could not be read (${response.status}).`);
	return (await response.json()) as T;
}

/** Owner-only: Library files that were archived. Null when they cannot be read. */
export async function readArchivedLibrary<T>(company: string): Promise<T | null> {
	return ownerRequest<T>(`${api(company)}/library/archived`).catch(() => null);
}

/** A file a message refers to, resolved to a viewable URL or its text. */
export function readMessageReference<T>(company: string, message: string, path: string) {
	return ownerRequest<T>(
		`${api(company)}/messages/${encodeURIComponent(message)}/reference?path=${encodeURIComponent(path)}`
	);
}

/** An attachment's bytes, for an in-place preview. */
export async function readAttachmentBytes(href: string): Promise<Response> {
	const response = await fetch(href, { credentials: 'same-origin' });
	if (!response.ok)
		throw Object.assign(new Error('The file could not be read.'), { status: response.status });
	return response;
}

/** The bytes of a file at a same-origin address (an upload's download link). */
export async function readFileBytes(href: string): Promise<ArrayBuffer> {
	const response = await fetch(href, { credentials: 'same-origin', cache: 'no-store' });
	if (!response.ok) throw await responseFailure(response);
	return response.arrayBuffer();
}
