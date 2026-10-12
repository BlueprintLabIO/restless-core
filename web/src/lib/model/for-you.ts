/* For you: mentions, replies, comments and changes for the signed-in person, newest first. The
 * server assembles it from the objects themselves and keeps read state per person, so reading an
 * item on one device reads it everywhere. */
import { responseFailure } from './failure.ts';

export type ForYouKind = 'mention' | 'reply' | 'comment' | 'change';

export interface ForYouItem {
	key: string;
	kind: ForYouKind;
	/** The document or Room it came from; empty for a direct conversation. */
	title: string;
	excerpt: string;
	href: string;
	actor_id: string;
	/** Everyone who acted since the person last read it, as one phrase. */
	actor_name: string;
	at: string;
	unread: boolean;
}

export interface ForYouPage {
	items: ForYouItem[];
	unread_count: number;
	next_before: string | null;
}

function path(companyId: string, rest = ''): string {
	return `/api/companies/${encodeURIComponent(companyId)}/for-you${rest}`;
}

export async function getForYou(
	companyId: string,
	before: string | null = null
): Promise<ForYouPage> {
	const query = before ? `?${new URLSearchParams({ before })}` : '';
	const response = await fetch(path(companyId, query), {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as ForYouPage;
}

async function markRead(
	companyId: string,
	body: { keys: string[] } | { all: true }
): Promise<void> {
	const response = await fetch(path(companyId, '/read'), {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify(body)
	});
	if (!response.ok) throw await responseFailure(response);
}

export const markForYouRead = (companyId: string, keys: string[]) => markRead(companyId, { keys });
export const markAllForYouRead = (companyId: string) => markRead(companyId, { all: true });

/** Follow or leave a document; answers whether the person now follows it. */
export async function setDocumentFollow(
	companyId: string,
	documentId: string,
	following: boolean
): Promise<boolean> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(companyId)}/documents/${encodeURIComponent(documentId)}/follow`,
		{ method: following ? 'PUT' : 'DELETE', credentials: 'same-origin' }
	);
	if (!response.ok) throw await responseFailure(response);
	return Boolean(((await response.json()) as { following?: unknown }).following);
}

/** One line that says what happened: "Bianca and Exec changed Pricing". */
export function forYouSentence(item: Pick<ForYouItem, 'kind' | 'actor_name' | 'title'>): string {
	const who = item.actor_name || 'Someone';
	const where = item.title.trim();
	switch (item.kind) {
		case 'mention':
			return where ? `${who} mentioned you in ${where}` : `${who} mentioned you`;
		case 'reply':
			return `${who} replied in ${where || 'a thread'}`;
		case 'comment':
			return `${who} commented on ${where || 'a document'}`;
		case 'change':
			return `${who} changed ${where || 'a document'}`;
		default:
			return where || who;
	}
}

/** The page as it reads once these items are read here, before the server answers. */
export function withRead(page: ForYouPage, keys: readonly string[] | 'all'): ForYouPage {
	const read = (item: ForYouItem) => keys === 'all' || keys.includes(item.key);
	const newlyRead = page.items.filter((item) => item.unread && read(item)).length;
	return {
		...page,
		items: page.items.map((item) =>
			item.unread && read(item) ? { ...item, unread: false } : item
		),
		unread_count: keys === 'all' ? 0 : Math.max(0, page.unread_count - newlyRead)
	};
}

/** The newest unread item that was not unread before, for a desktop notice. */
export function newlyUnread(previous: ReadonlySet<string> | null, page: ForYouPage): ForYouItem[] {
	if (!previous) return [];
	return page.items.filter((item) => item.unread && !previous.has(`${item.key}@${item.at}`));
}

export function unreadMarks(page: ForYouPage): Set<string> {
	return new Set(page.items.filter((item) => item.unread).map((item) => `${item.key}@${item.at}`));
}
