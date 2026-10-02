export interface SheetRow {
	id: string; title: string; owner_actor_id: string; visibility: 'company' | 'participants';
	engine_version: string; head_revision: string; sequence: number; updated_at: string;
}
export async function sheetJson<T>(company: string, suffix = '', init?: RequestInit): Promise<T> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/sheets${suffix}`, {
		...init, headers: { 'content-type': 'application/json', ...init?.headers }, cache: 'no-store'
	});
	const value = await response.json();
	if (!response.ok) throw Object.assign(new Error(value.message ?? value.error ?? 'Sheets are unavailable'), { status: response.status });
	return value as T;
}
export function createSheet(company: string, id: string, title: string): Promise<SheetRow> {
	return sheetJson(company, '', { method: 'POST', body: JSON.stringify({ id, title, visibility: 'company' }) });
}
