import type { AttentionItem } from './view';

/** Projection only: resolution always uses the original source reference. */
export function ownerAttention(items: AttentionItem[]): AttentionItem[] {
	const seen = new Set<string>();
	return items.filter((item) => {
		// A failed scheduled opportunity has no owner decision. Its outcome is
		// available in Schedules; an explicit handoff still enters Attention.
		if (item.source.kind === 'blocked_opportunity') return false;
		const key = `${item.source.plane}:${item.source.kind}:${item.source.reference}`;
		if (seen.has(key)) return false;
		seen.add(key);
		return true;
	});
}

export function attentionTitle(item: AttentionItem): string {
	return item.title.replace(/^(Blocked|Decision|Approval|Outcome review):\s*/i, '');
}

/** What kind of ask this is, in a word, for a tooltip. The Inbox does not
 * file items into categories; the owner reads one list. */
export function attentionKindLabel(item: AttentionItem): string {
	if (item.category === 'approval') return 'Approval';
	if (item.category === 'review') return 'Review';
	if (item.category === 'blocker') return 'Setup';
	if (item.category === 'human_step') return 'Your step';
	if (item.category === 'conversation') return 'Question';
	return 'Decision';
}

/** Who is asking: the accountable lead, or whoever wrote the brief. */
export function attentionAsker(item: AttentionItem): string {
	return item.responsibleActor?.display ?? item.briefAuthor?.display ?? 'Exec';
}

/** One list: what blocks the company first, then the oldest ask first, so
 * nothing waits behind newer noise. */
export function inboxOrder(items: AttentionItem[]): AttentionItem[] {
	const blocking = (item: AttentionItem) => (item.category === 'blocker' ? 0 : 1);
	return [...items].sort(
		(a, b) => blocking(a) - blocking(b) || +new Date(a.createdAt) - +new Date(b.createdAt)
	);
}
