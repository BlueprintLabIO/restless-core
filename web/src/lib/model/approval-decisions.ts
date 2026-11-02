import type { QueryClient } from '@tanstack/svelte-query';
import { ownerRequest } from './setting.svelte';
import type { AttentionItem } from './view';
import { removeConfirmedAttention } from './queries.svelte';
import { showHumanDecisionReceipt, type HumanDecisionReceipt } from './human-decision-receipts';
import { approvalTarget, type ApprovalDecision } from './approval-answer';
export { approvalDecision, approvalTarget, type ApprovalDecision } from './approval-answer';

/* An Inbox Approve or Decline is saved with Authority, which holds its Undo window: nothing is
 * granted, declined or told to the company until the window ends (C1, instant-decisions.md). */
export type ApprovalDecisionReceipt = HumanDecisionReceipt & {
	kind: 'approve' | 'approve_always' | 'decline' | 'request_changes';
};
type Clock = { server_now: string; startedAt: number };
export type ApprovalDecisionRead = Clock & { items: ApprovalDecisionReceipt[] };
const base = (company: string) => '/api/companies/' + encodeURIComponent(company);
export const approvalDecisionsKey = (company: string) => ['approval-decisions', company] as const;

export async function readApprovalDecisions(
	company: string,
	signal?: AbortSignal
): Promise<ApprovalDecisionRead> {
	const startedAt = Date.now();
	const reply = await ownerRequest<Omit<ApprovalDecisionRead, 'startedAt'>>(
		base(company) + '/approval-decisions',
		{ signal }
	);
	return { ...reply, startedAt };
}

const receiptSource = {
	keyPrefix: 'approval',
	resource: (company: string) => base(company) + '/approval-decisions',
	queryKey: approvalDecisionsKey
};

export function reconcileApprovalDecisions(
	client: QueryClient,
	company: string,
	reply: ApprovalDecisionRead
) {
	for (const receipt of reply.items)
		showHumanDecisionReceipt(client, company, receipt, reply, receiptSource);
}

/** Save the answer, then take the card off the Inbox. True once saved, not once applied. */
export async function queueApprovalDecision(
	client: QueryClient,
	company: string,
	item: AttentionItem,
	decision: ApprovalDecision
): Promise<boolean> {
	const target = approvalTarget(item);
	const startedAt = Date.now();
	const receipt = await ownerRequest<ApprovalDecisionReceipt & { server_now: string }>(
		base(company) + '/approvals/pending-decision',
		{
			method: 'POST',
			json: { id: crypto.randomUUID(), item_id: item.id, target, decision }
		}
	);
	showHumanDecisionReceipt(
		client,
		company,
		receipt,
		{ server_now: receipt.server_now, startedAt },
		receiptSource
	);
	const saved = receipt.state === 'pending' || receipt.state === 'applied';
	if (saved) await removeConfirmedAttention(client, company, item.id, receipt.state === 'pending');
	void client.invalidateQueries({ queryKey: approvalDecisionsKey(company) });
	return saved;
}
