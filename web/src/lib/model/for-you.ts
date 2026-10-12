/* For you: mentions, replies, comments, shares, review asks and changes for the signed-in person,
 * newest first. The server assembles it from the objects themselves and keeps read, unread and
 * archive marks per person, so what happens to an item on one device happens everywhere. */
import { responseFailure } from './failure.ts';

export type ForYouKind =
	| 'mention'
	| 'reply'
	| 'comment'
	| 'change'
	| 'share'
	| 'review'
	| 'access'
	| 'comment-access'
	| 'edit-access';

export interface ForYouItem {
	key: string;
	kind: ForYouKind;
	/** The document, sheet or Room it came from; empty for a direct conversation. */
	title: string;
	/** What was said; for a share, the access it gave ("Can edit"). */
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

export type ForYouFilter = 'all' | 'unread';

function path(companyId: string, rest = ''): string {
	return `/api/companies/${encodeURIComponent(companyId)}/for-you${rest}`;
}

export async function getForYou(
	companyId: string,
	before: string | null = null,
	filter: ForYouFilter = 'all'
): Promise<ForYouPage> {
	const params = new URLSearchParams();
	if (before) params.set('before', before);
	if (filter === 'unread') params.set('unread', 'true');
	const query = params.toString() ? `?${params}` : '';
	const response = await fetch(path(companyId, query), {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as ForYouPage;
}

async function post(companyId: string, rest: string, body: unknown): Promise<void> {
	const response = await fetch(path(companyId, rest), {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify(body)
	});
	if (!response.ok) throw await responseFailure(response);
}

export const markForYouRead = (companyId: string, keys: string[]) =>
	post(companyId, '/read', { keys });
export const markAllForYouRead = (companyId: string) => post(companyId, '/read', { all: true });
/** Unread again until opened, even past "mark all read". */
export const markForYouUnread = (companyId: string, keys: string[]) =>
	post(companyId, '/unread', { keys });
/** Off the list until something new happens there; `false` brings it back. */
export const archiveForYou = (companyId: string, keys: string[], archived = true) =>
	post(companyId, '/archive', { keys, archived });

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

/** Whether the person follows a sheet, or with `following`, follow or leave it. */
export async function sheetFollow(
	companyId: string,
	sheetId: string,
	following?: boolean
): Promise<boolean> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(companyId)}/sheets/${encodeURIComponent(sheetId)}/follow`,
		{
			method: following === undefined ? 'GET' : following ? 'PUT' : 'DELETE',
			credentials: 'same-origin',
			cache: 'no-store'
		}
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
		case 'share':
			return `${who} shared ${where || 'a document'} with you`;
		case 'review':
			return `${who} asked you to review ${where || 'a document'}`;
		case 'access':
			return `${who} asked to open ${where || 'a document'}`;
		case 'comment-access':
			return `${who} asked to comment on ${where || 'a document'}`;
		case 'edit-access':
			return `${who} asked to edit ${where || 'a document'}`;
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

/** The page once these items are unread again here. */
export function withUnread(page: ForYouPage, keys: readonly string[]): ForYouPage {
	const newlyUnread = page.items.filter((item) => !item.unread && keys.includes(item.key)).length;
	return {
		...page,
		items: page.items.map((item) =>
			!item.unread && keys.includes(item.key) ? { ...item, unread: true } : item
		),
		unread_count: page.unread_count + newlyUnread
	};
}

/** The page once these items are archived here: gone, and no longer counted. */
export function withoutItems(page: ForYouPage, keys: readonly string[]): ForYouPage {
	const leaving = page.items.filter((item) => keys.includes(item.key));
	return {
		...page,
		items: page.items.filter((item) => !keys.includes(item.key)),
		unread_count: Math.max(0, page.unread_count - leaving.filter((item) => item.unread).length)
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

/* The owner Inbox's own marks: which brief of each item the person last opened, and what they
 * snoozed. Kept on the server per person, so the phone and the laptop agree. */
export interface InboxMark {
	item_id: string;
	seen_stamp: string | null;
	snoozed_until: string | null;
}

export interface InboxMarkChange {
	item_id: string;
	seen_stamp?: string;
	/** Absent keeps the snooze; null wakes the item. */
	snoozed_until?: string | null;
}

function marksPath(companyId: string): string {
	return `/api/companies/${encodeURIComponent(companyId)}/inbox-marks`;
}

export async function getInboxMarks(companyId: string): Promise<InboxMark[]> {
	const response = await fetch(marksPath(companyId), {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	return ((await response.json()) as { marks: InboxMark[] }).marks;
}

export async function setInboxMarks(companyId: string, marks: InboxMarkChange[]): Promise<void> {
	if (!marks.length) return;
	const response = await fetch(marksPath(companyId), {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify({ marks })
	});
	if (!response.ok) throw await responseFailure(response);
}

/** Folds changes into the marks the list holds, as the server will once it answers. */
export function withMarks(
	marks: readonly InboxMark[],
	changes: readonly InboxMarkChange[]
): InboxMark[] {
	const next = new Map(marks.map((mark) => [mark.item_id, mark]));
	for (const change of changes) {
		const current = next.get(change.item_id) ?? {
			item_id: change.item_id,
			seen_stamp: null,
			snoozed_until: null
		};
		next.set(change.item_id, {
			item_id: change.item_id,
			seen_stamp: change.seen_stamp ?? current.seen_stamp,
			snoozed_until:
				change.snoozed_until === undefined ? current.snoozed_until : change.snoozed_until
		});
	}
	return [...next.values()];
}
