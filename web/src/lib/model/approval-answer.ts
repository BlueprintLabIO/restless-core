import type { AttentionItem } from './view';

/* What an Inbox Approve or Decline sends to Authority's Undo window: the exact request, and the
 * answer in the shape Authority stores it. */
export type ApprovalTarget =
	| { kind: 'party'; party: string }
	| { kind: 'tool_call'; call_key: string }
	| { kind: 'email_mandate'; proposal: string }
	| { kind: 'payee'; proposal: number }
	| { kind: 'account_notice'; account: string; notice: string }
	| { kind: 'account_request'; request: string }
	| { kind: 'publication'; publication: string };
export type ApprovalDecision =
	| { action: 'grant'; always?: boolean }
	| { action: 'decline'; note?: string; changes_requested?: boolean };

/** The exact request this card answers: one prepared call, one party's first contact, or the
 * mandate, payee, account notice, account request or publication the card names. */
export function approvalTarget(
	item: Pick<AttentionItem, 'source'> & Partial<Pick<AttentionItem, 'accountUse'>>
): ApprovalTarget {
	const { kind, reference } = item.source;
	if (kind === 'email_mandate_proposal') return { kind: 'email_mandate', proposal: reference };
	if (kind === 'payee_proposal' && /^\d+$/.test(reference))
		return { kind: 'payee', proposal: Number(reference) };
	if (kind === 'account_notice' && item.accountUse?.account.id)
		return { kind: 'account_notice', account: item.accountUse.account.id, notice: reference };
	if (kind === 'account_request') return { kind: 'account_request', request: reference };
	if (kind === 'publication_authorization') return { kind: 'publication', publication: reference };
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
