import type { GeneralAccess } from './general-access.ts';
export interface SheetRow {
	id: string; title: string; owner_actor_id: string; visibility: 'company' | 'participants';
	/** What every person in the company may do without being added: private, view or edit. */
	general_access: GeneralAccess;
	engine_version: string; head_revision: string; sequence: number; updated_at: string;
	/** Set while the sheet is archived out of the Library. It still opens. */
	archived_at?: string | null;
	/** In a listing: whether the viewer may archive, restore and rename it. */
	may_manage?: boolean;
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
	people: { actor_id: string; access: 'read' | 'edit'; added_by_actor_id: string | null }[];
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
export function setSheetParticipant(company: string, id: string, actor: string, access: 'read' | 'edit' | null): Promise<unknown> {
	const path = `/${encodeURIComponent(id)}/participants/${encodeURIComponent(actor)}`;
	return sheetJson(company, path, access ? { method: 'PUT', body: JSON.stringify({ access }) } : { method: 'DELETE' });
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
