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

export function attentionGroup(item: AttentionItem): string {
	if (item.category === 'approval') return 'Approve';
	if (item.category === 'review') return 'Review';
	if (item.category === 'blocker') return 'Fix setup';
	return 'Decide';
}
