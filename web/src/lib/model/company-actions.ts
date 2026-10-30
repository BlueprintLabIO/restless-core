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

/** Form a team around an accountable lead. */
export function createTeam(company: string, body: { name: string; brief: string; lead: string }) {
	return ownerRequest<{ team_id: string }>(`${api(company)}/teams`, { method: 'POST', json: body });
}

/** Rename a team or revise what it is accountable for. */
export function changeTeam(company: string, team: string, body: { name?: string; brief?: string }) {
	return ownerRequest<unknown>(`${api(company)}/teams/${encodeURIComponent(team)}`, {
		method: 'PATCH',
		json: body
	});
}

/** Replace a team's accountable lead. */
export function setTeamLead(company: string, team: string, lead: string) {
	return ownerRequest<unknown>(`${api(company)}/teams/${encodeURIComponent(team)}/lead`, {
		method: 'POST',
		json: { lead }
	});
}

/** Disband a team: members become unassigned; judgement it still owed falls to Exec. */
export function disbandTeam(company: string, team: string) {
	return ownerRequest<{ reassigned_judgements: number }>(
		`${api(company)}/teams/${encodeURIComponent(team)}/disband`,
		{ method: 'POST', json: {} }
	);
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
export async function readArchivedLibrary<T>(
	company: string,
	signal?: AbortSignal
): Promise<T | null> {
	return ownerRequest<T>(`${api(company)}/library/archived`, { signal });
}

/** Owner-only: when each Library file last changed on the computer, by its path. An empty map
 * when the computer is asleep or the read fails. */
/** One file agents wrote under /company/outputs, linked as an output or not. */
export interface OutputFile {
	path: string;
	bytes: number;
	modified: string | null;
}

/** A stable position in the company's output listing. */
export interface OutputCursor { modified: number; path: string; revision?: string }

/** One bounded page of output files for the Library's Folders view. Owner only. */
export async function readLibraryOutputs(
	company: string,
	signal?: AbortSignal,
	cursor?: OutputCursor,
	fresh = false
): Promise<{ files: OutputFile[]; truncated: boolean; next: OutputCursor | null; skippedNames: number; revision: string | null; stale: boolean }> {
	const params = cursor ? new URLSearchParams({ modified: String(cursor.modified), path: cursor.path, ...(cursor.revision ? { revision: cursor.revision } : {}) }) : new URLSearchParams(fresh ? { fresh: 'true' } : {});
	const query = params.size ? `?${params}` : '';
	const body = await ownerRequest<{ files?: OutputFile[]; truncated?: boolean; next_cursor?: OutputCursor | null; skipped_names?: number; revision?: string; stale?: boolean }>(
		`${api(company)}/library/outputs${query}`, { signal }
	);
	return { files: body?.files ?? [], truncated: body?.truncated === true, next: body?.next_cursor ?? null, skippedNames: Number.isSafeInteger(body?.skipped_names) && (body?.skipped_names ?? 0) > 0 ? body!.skipped_names! : 0, revision: body?.revision ?? null, stale: body?.stale === true };
}

export async function readLibraryModified(
	company: string,
	signal?: AbortSignal
): Promise<Record<string, string>> {
	const body = await ownerRequest<{ modified?: Record<string, string> }>(
		`${api(company)}/library/modified`,
		{ signal }
	).catch(() => null);
	return body?.modified ?? {};
}

/** The later of a row's own time and when its file last changed. */
export function latestTime(recorded: string, changed: string | undefined): string {
	if (!changed) return recorded;
	const a = Date.parse(recorded);
	const b = Date.parse(changed);
	return Number.isNaN(a) || (!Number.isNaN(b) && b > a) ? changed : recorded;
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
	if (!response.ok) throw await responseFailure(response);
	return response;
}

/** The bytes of a file at a same-origin address (an upload's download link). */
export async function readFileBytes(href: string): Promise<ArrayBuffer> {
	const response = await fetch(href, { credentials: 'same-origin', cache: 'no-store' });
	if (!response.ok) throw await responseFailure(response);
	return response.arrayBuffer();
}
