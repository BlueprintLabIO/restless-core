import type { AttentionItem } from './view';

/* What an Inbox Approve or Decline sends to Authority's Undo window: the exact request, and the
 * answer in the shape Authority stores it. */
export type ApprovalTarget =
	{ kind: 'party'; party: string } | { kind: 'tool_call'; call_key: string };
export type ApprovalDecision =
	| { action: 'grant'; always?: boolean }
	| { action: 'decline'; note?: string; changes_requested?: boolean };

/** The exact request this card answers: one prepared call, or one party's first contact. */
export function approvalTarget(item: Pick<AttentionItem, 'source'>): ApprovalTarget {
	if (item.source.call_key) return { kind: 'tool_call', call_key: item.source.call_key };
	if (item.source.party) return { kind: 'party', party: item.source.party };
	throw new Error('This request has no approval target. Refresh and try again.');
}

/** A note only with a decline, `always` only with a grant. */
export function approvalDecision(
	action: 'grant' | 'decline',
	answer: { always?: boolean; note?: string; changesRequested?: boolean } = {}
): ApprovalDecision {
	if (action === 'grant') return { action, ...(answer.always ? { always: true } : {}) };
	const note = answer.note?.trim();
	return {
		action,
		...(note ? { note } : {}),
		...(answer.changesRequested ? { changes_requested: true } : {})
	};
}
