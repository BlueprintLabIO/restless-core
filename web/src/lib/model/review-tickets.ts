import { issueReviewTicket } from './attention';
import type { AttentionItem } from './view';

/* The next Inbox item shows at once (S109 C1). Opening a prepared outcome asks the server for a
 * ticket, which reads the whole Inbox first; while one item is on screen, the ticket for the item
 * that takes its place is already asked for. A ticket is handed out once, and a stale or failed
 * one is never reused. */
const FRESH_MS = 5 * 60_000;
type Held = { key: string; at: number; ticket: Promise<string | null> };
const held = new Map<string, Held>();

/** What a ticket opens; a sleeping computer or an inline text outcome has nothing to ask for. */
function keyOf(companyId: string, item: AttentionItem): string | null {
	const target = item.reviewTarget;
	if (!target || target.status !== 'available' || target.kind === 'runtime-text') return null;
	return [companyId, item.id, target.uri, target.generation].join(':');
}

export function prefetchReviewTicket(companyId: string, item: AttentionItem): void {
	const key = keyOf(companyId, item);
	if (!key) return;
	const now = Date.now();
	for (const [id, entry] of held) if (now - entry.at >= FRESH_MS) held.delete(id);
	if (held.get(item.id)?.key === key) return;
	const ticket = issueReviewTicket(companyId, item.id);
	ticket.catch(() => {
		if (held.get(item.id)?.ticket === ticket) held.delete(item.id);
	});
	held.set(item.id, { key, at: now, ticket });
}

/** This item's ticket: the one already asked for while it is fresh, otherwise a new one. */
export function reviewTicket(
	companyId: string,
	item: AttentionItem,
	signal: AbortSignal
): Promise<string | null> {
	const saved = held.get(item.id);
	held.delete(item.id);
	if (saved && saved.key === keyOf(companyId, item) && Date.now() - saved.at < FRESH_MS)
		return saved.ticket;
	return issueReviewTicket(companyId, item.id, signal);
}
