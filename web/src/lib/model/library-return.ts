/* The Library view an editor was opened from (its filter, collection, sort and facets), so the
 * editor's way back returns there rather than to a fixed list. Held per tab, in memory: after a
 * reload the way back is the list of the editor's own kind. */
let last: { company: string; search: string } | null = null;

/** The address a viewer or editor opens on top of the list; never part of the way back. */
const OPENED = ['file', 'upload', 'path'];

export function rememberLibraryView(company: string, search: string): void {
	const params = new URLSearchParams(search);
	if (OPENED.some((key) => params.has(key))) return;
	last = { company, search: params.size ? `?${params}` : '' };
}

export function libraryReturnHref(company: string, show: 'docs' | 'sheets'): string {
	const base = `/${encodeURIComponent(company)}/library`;
	return last?.company === company ? `${base}${last.search}` : `${base}?show=${show}`;
}
