import { deadlineAt } from './attention-presentation.ts';
import type { AttentionItem } from './view';
import type { ForYouItem } from './for-you';

export type InboxEntry =
	| { kind: 'attention'; key: string; item: AttentionItem }
	| { kind: 'update'; key: string; item: ForYouItem };

/** One queue: company blockers, owner actions by deadline, unread updates, then read updates. */
export function inboxFeed(
	attention: readonly AttentionItem[],
	updates: readonly ForYouItem[]
): InboxEntry[] {
	const rows: InboxEntry[] = [
		...attention.map((item): InboxEntry => ({
			kind: 'attention',
			key: 'attention:' + item.id,
			item
		})),
		...updates.map((item): InboxEntry => ({ kind: 'update', key: 'update:' + item.key, item }))
	];
	const rank = (row: InboxEntry) =>
		row.kind === 'attention'
			? row.item.category === 'blocker'
				? 0
				: 1
			: ['review', 'access', 'comment-access', 'edit-access'].includes(row.item.kind)
				? 1
				: row.item.unread
					? 2
					: 3;
	const due = (row: InboxEntry) =>
		row.kind === 'attention' ? (deadlineAt(row.item) ?? Infinity) : Infinity;
	const time = (row: InboxEntry) => {
		const at = Date.parse(row.kind === 'attention' ? String(row.item.createdAt) : row.item.at);
		return Number.isFinite(at) ? at : 0;
	};
	return rows.sort(
		(a, b) =>
			rank(a) - rank(b) ||
			(due(a) === due(b) ? 0 : due(a) < due(b) ? -1 : 1) ||
			Number(a.kind === 'attention' && a.item.canContinue) -
				Number(b.kind === 'attention' && b.item.canContinue) ||
			(rank(a) <= 1 ? time(a) - time(b) : time(b) - time(a)) ||
			a.key.localeCompare(b.key)
	);
}
