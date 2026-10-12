/* What a person opened lately in a company, for ⌘K before anything is typed: documents, sheets
 * and conversations. A per-device convenience like a browser's history; it names places, never
 * their contents, and opening one still asks the server. */

export type RecentKind = 'document' | 'sheet' | 'person' | 'room';
export type Recent = { key: string; kind: RecentKind; id: string; label: string; href: string };

export const MAX_RECENTS = 8;

/** The place a company URL shows, if it is one worth returning to. */
export function recentPlace(
	companyId: string,
	url: URL
): { kind: RecentKind; id: string; href: string } | null {
	const root = `/${encodeURIComponent(companyId)}`;
	const at = (kind: RecentKind, id: string, href: string) =>
		id ? { kind, id, href: `${root}${href}${encodeURIComponent(id)}` } : null;
	if (url.pathname === `${root}/library/documents`)
		return at('document', url.searchParams.get('document') ?? '', '/library/documents?document=');
	if (url.pathname === `${root}/library/sheets`)
		return at('sheet', url.searchParams.get('sheet') ?? '', '/library/sheets?sheet=');
	if (url.pathname === `${root}/people`) {
		const room = url.searchParams.get('room');
		if (room) return at('room', room, '/people?room=');
		return at('person', url.searchParams.get('person') ?? '', '/people?person=');
	}
	return null;
}

/** Newest first, each place once, at most MAX_RECENTS. */
export function remember(list: Recent[], entry: Recent): Recent[] {
	return [entry, ...list.filter((item) => item.key !== entry.key)].slice(0, MAX_RECENTS);
}

const storeKey = (companyId: string, actorId: string) => `restless:${companyId}:${actorId}:recents`;

export function readRecents(companyId: string, actorId: string): Recent[] {
	try {
		const raw = JSON.parse(localStorage.getItem(storeKey(companyId, actorId)) ?? '[]');
		return Array.isArray(raw)
			? raw
					.filter(
						(item): item is Recent =>
							!!item &&
							typeof item.key === 'string' &&
							typeof item.label === 'string' &&
							typeof item.href === 'string' &&
							item.href.startsWith(`/${encodeURIComponent(companyId)}/`)
					)
					.slice(0, MAX_RECENTS)
			: [];
	} catch {
		return [];
	}
}

export function writeRecents(companyId: string, actorId: string, list: Recent[]) {
	try {
		localStorage.setItem(storeKey(companyId, actorId), JSON.stringify(list));
	} catch {
		/* Recents are a convenience; ⌘K still searches everything. */
	}
}
