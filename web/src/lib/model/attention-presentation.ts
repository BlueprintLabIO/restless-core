import type { AttentionItem } from './view';
import type { OwnerFailure } from './failure.ts';

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

/** The Work waiting on the owner is the Work the Inbox asks about once its step is prepared.
 * Judgement a lead or Exec still holds, and what the owner set aside, never reach the Inbox. */
export function workWaitingOnOwner(items: readonly AttentionItem[]): Set<string> {
	return new Set(items.flatMap((item) => (item.workId && !item.preparing ? [item.workId] : [])));
}

/** Who still holds a pending judgement before it reaches the owner, by Work. */
export function judgementHolders(
	handoffs: readonly { work_id: string; state?: string; assigned_to?: string | null }[]
): Map<string, string> {
	return new Map(
		handoffs.flatMap((handoff) =>
			handoff.state === 'pending' && handoff.assigned_to
				? [[handoff.work_id, handoff.assigned_to] as const]
				: []
		)
	);
}

/** OrgIntel holds the Work and most of what needs the owner. When a company's coordination is
 * unavailable the projection still answers, so an empty list is not a verdict: say so instead. */
export function coordinationFailure(
	view: { sourceHealth: { orgintel: string } } | null | undefined
): OwnerFailure | null {
	if (!view || view.sourceHealth.orgintel === 'available') return null;
	const message =
		'Company coordination is unavailable, so Work and what it needs from you may be missing. This refreshes on its own.';
	return Object.assign(new Error(message), {
		status: 503,
		code: 'orgintel',
		serverMessage: message
	});
}
