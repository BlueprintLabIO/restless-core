import type { GeneralAccess } from './general-access.ts';
/** What someone added to a sheet may do: view, comment, or edit. */
export type SheetLevel = 'read' | 'comment' | 'edit';
export interface SheetRow {
	id: string; title: string; owner_actor_id: string; visibility: 'company' | 'participants';
	/** What every person in the company may do without being added: private, view or edit. */
	general_access: GeneralAccess;
	engine_version: string; head_revision: string; sequence: number; updated_at: string;
	/** Set while the sheet is archived out of the Library. It still opens. */
	archived_at?: string | null;
	/** In a listing: whether the viewer may archive, restore and rename it. */
	may_manage?: boolean;
	/** In a listing: who made the newest change. */
	last_editor_actor_id?: string | null;
}
export async function sheetJson<T>(company: string, suffix = '', init?: RequestInit): Promise<T> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/sheets${suffix}`, {
		...init, headers: { 'content-type': 'application/json', ...init?.headers }, cache: 'no-store'
	});
	// A proxy or the body limit can answer without JSON; the status is still honest.
	const value = await response.json().catch(() => ({}));
	const said = typeof value.message === 'string' ? value.message : null;
	if (!response.ok) throw Object.assign(new Error(said ?? (response.status === 413 ? 'That is more than a sheet can hold (8 MB).' : 'Sheets are unavailable')), { status: response.status, serverMessage: said });
	return value as T;
}
export function createSheet(company: string, id: string, title: string): Promise<SheetRow> {
	return sheetJson(company, '', { method: 'POST', body: JSON.stringify({ id, title, visibility: 'company' }) });
}
/** Rename a sheet: 1-200 characters, for its editors and the company's owner. */
export function renameSheet(company: string, id: string, title: string): Promise<SheetRow> {
	return sheetJson(company, `/${encodeURIComponent(id)}`, { method: 'PATCH', body: JSON.stringify({ title }) });
}
/** Who can open a sheet, for its share panel. */
export interface SheetSharing {
	general_access: GeneralAccess;
	owner_actor_id: string;
	/** Who added each person; null when that was before it was kept. */
	people: {
		actor_id: string;
		access: SheetLevel;
		added_by_actor_id: string | null;
		added_at: string | null;
	}[];
	/** Whether the reader may change any of it: its owner, the company's owner, or the Exec. */
	may_manage: boolean;
}
export function getSheetSharing(company: string, id: string): Promise<SheetSharing> {
	return sheetJson(company, `/${encodeURIComponent(id)}/participants`);
}
export function setSheetGeneralAccess(company: string, id: string, general_access: GeneralAccess): Promise<SheetRow> {
	return sheetJson(company, `/${encodeURIComponent(id)}`, { method: 'PATCH', body: JSON.stringify({ general_access }) });
}
/** Add someone to a sheet at a level, change their level, or with null remove them. */
export function setSheetParticipant(company: string, id: string, actor: string, access: SheetLevel | null): Promise<unknown> {
	const path = `/${encodeURIComponent(id)}/participants/${encodeURIComponent(actor)}`;
	return sheetJson(company, path, access ? { method: 'PUT', body: JSON.stringify({ access }) } : { method: 'DELETE' });
}
/** A copy the person owns, as the sheet reads now, with the same general access. */
export function duplicateSheet(company: string, sheet: string, copy: string): Promise<SheetRow> {
	return sheetJson<SheetRow>(company, `/${encodeURIComponent(sheet)}/duplicate`, {
		method: 'POST',
		body: JSON.stringify({ id: copy })
	});
}
/** Someone asking for a sheet: to view (`read`), to comment, or to edit. */
export interface SheetAccessRequest {
	id: string;
	actor_id: string;
	access: SheetLevel;
	requested_at: string;
}
/** Ask whoever shares a sheet for it; the answer never says whether it exists. */
export function askForSheet(company: string, sheet: string, access: SheetLevel) {
	return sheetJson<unknown>(company, `/${encodeURIComponent(sheet)}/access-requests`, {
		method: 'POST',
		body: JSON.stringify({ access })
	});
}
export async function sheetAccessRequests(
	company: string,
	sheet: string
): Promise<SheetAccessRequest[]> {
	return (
		await sheetJson<{ items: SheetAccessRequest[] }>(
			company,
			`/${encodeURIComponent(sheet)}/access-requests`
		)
	).items;
}
export function dismissSheetAccessRequest(company: string, sheet: string, request: string) {
	return sheetJson<unknown>(
		company,
		`/${encodeURIComponent(sheet)}/access-requests/${encodeURIComponent(request)}/dismiss`,
		{ method: 'POST' }
	);
}
/** One comment in a thread on a cell. */
export interface SheetComment {
	id: string;
	thread_id: string;
	author_actor_id: string;
	body: string;
	mentioned_actor_ids: string[];
	created_at: string;
	edited_at: string | null;
}
/** A thread on one cell of one tab: `worksheet_id` is the grid's id for the tab, `cell` its A1 name. */
export interface SheetCommentThread {
	id: string;
	worksheet_id: string;
	cell: string;
	created_by_actor_id: string;
	created_at: string;
	resolved_by_actor_id: string | null;
	resolved_at: string | null;
	comments: SheetComment[];
}
export interface SheetComments {
	threads: SheetCommentThread[];
	/** Whether the reader may start threads, reply and resolve: comment or edit access. */
	may_comment: boolean;
}
const commentsPath = (sheet: string, rest = '') =>
	`/${encodeURIComponent(sheet)}/comments${rest}`;
export function getSheetComments(company: string, sheet: string): Promise<SheetComments> {
	return sheetJson(company, commentsPath(sheet));
}
/** Start a thread on a cell. The id is the caller's, so a retry finds the thread it made. */
export function startSheetThread(
	company: string,
	sheet: string,
	input: { id: string; worksheet_id: string; cell: string; body: string; mentions: string[] }
): Promise<SheetCommentThread> {
	return sheetJson(company, commentsPath(sheet), { method: 'POST', body: JSON.stringify(input) });
}
export function replyToSheetThread(
	company: string,
	sheet: string,
	thread: string,
	input: { id: string; body: string; mentions: string[] }
): Promise<SheetComment> {
	return sheetJson(company, commentsPath(sheet, `/${encodeURIComponent(thread)}/replies`), {
		method: 'POST',
		body: JSON.stringify(input)
	});
}
export function resolveSheetThread(
	company: string,
	sheet: string,
	thread: string,
	resolved: boolean
): Promise<SheetCommentThread> {
	return sheetJson(company, commentsPath(sheet, `/${encodeURIComponent(thread)}/resolve`), {
		method: 'POST',
		body: JSON.stringify({ resolved })
	});
}
export function editSheetComment(
	company: string,
	sheet: string,
	thread: string,
	comment: string,
	input: { body: string; mentions: string[] }
): Promise<SheetComment> {
	return sheetJson(
		company,
		commentsPath(sheet, `/${encodeURIComponent(thread)}/${encodeURIComponent(comment)}`),
		{ method: 'PATCH', body: JSON.stringify(input) }
	);
}
export async function deleteSheetComment(
	company: string,
	sheet: string,
	thread: string,
	comment: string
): Promise<void> {
	await sheetJson(
		company,
		commentsPath(sheet, `/${encodeURIComponent(thread)}/${encodeURIComponent(comment)}`),
		{ method: 'DELETE' }
	);
}
export function sheetHref(company: string, id: string): string {
	return `/${encodeURIComponent(company)}/library/sheets?sheet=${encodeURIComponent(id)}`;
}
/** Hand the browser a file to save, named from a title without characters file systems refuse. */
export function saveFile(title: string, extension: string, body: Blob) {
	const url = URL.createObjectURL(body);
	const link = document.createElement('a');
	link.href = url;
	link.download = `${title.replace(/[\\/:*?"<>|\u0000-\u001f]+/g, '-').trim() || 'Sheet'}.${extension}`;
	link.click();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}
export const SPREADSHEET_ACCEPT ='.xlsx,.csv,text/csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
/** Make a new sheet from an .xlsx or .csv file, named after the file. A retry with the same
 * id after a lost response finds the sheet the first attempt made rather than a duplicate. */
export async function importSpreadsheet(company: string, file: File, id: string = crypto.randomUUID()): Promise<SheetRow> {
	const { sheetSource } = await import('./sheet-files');
	const from = await sheetSource(file);
	const title = file.name.replace(/\.(xlsx|csv)$/i, '').trim().slice(0, 200) || 'Imported sheet';
	try {
		return await sheetJson<SheetRow>(company, '', { method: 'POST', body: JSON.stringify({ id, title, visibility: 'company', from }) });
	} catch (cause) {
		if ((cause as { status?: number }).status !== 409) throw cause;
		const existing = await sheetJson<{ sheet: SheetRow }>(company, `/${encodeURIComponent(id)}`).catch(() => null);
		if (!existing) throw cause;
		return existing.sheet;
	}
}
