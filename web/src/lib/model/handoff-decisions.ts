import type { QueryClient } from '@tanstack/svelte-query';
import { ownerRequest } from './setting.svelte';
import type { AttentionItem, OwnerReviewVersion } from './view';
import { removeConfirmedAttention } from './queries.svelte';
import { showHumanDecisionReceipt } from './human-decision-receipts';

export type HandoffDecision =
	{ kind: 'complete_human_step' } | { kind: 'skip' } | { kind: 'accept_review'; feedback: string };
export type HandoffDecisionReceipt = {
	id: string;
	handoff_id: string;
	item_id: string;
	kind: HandoffDecision['kind'];
	undo_until: string;
	state: 'pending' | 'canceled' | 'applied' | 'failed';
	problem: string | null;
};
type Clock = { server_now: string; startedAt: number };
export type HandoffDecisionRead = Clock & { items: HandoffDecisionReceipt[] };
const base = (company: string) => '/api/companies/' + encodeURIComponent(company);
export const handoffDecisionsKey = (company: string) => ['handoff-decisions', company] as const;

export async function readHandoffDecisions(
	company: string,
	signal?: AbortSignal
): Promise<HandoffDecisionRead> {
	const startedAt = Date.now();
	const reply = await ownerRequest<Omit<HandoffDecisionRead, 'startedAt'>>(
		base(company) + '/handoff-decisions',
		{ signal }
	);
	return { ...reply, startedAt };
}

const receiptSource = {
	keyPrefix: 'handoff',
	resource: (company: string) => base(company) + '/handoff-decisions',
	queryKey: handoffDecisionsKey
};

export function reconcileHandoffDecisions(
	client: QueryClient,
	company: string,
	reply: HandoffDecisionRead
) {
	for (const receipt of reply.items)
		showHumanDecisionReceipt(client, company, receipt, reply, receiptSource);
}

export async function queueHandoffDecision(
	client: QueryClient,
	company: string,
	item: AttentionItem,
	decision: HandoffDecision
): Promise<boolean> {
	const expected: OwnerReviewVersion | undefined = item.reviewVersion;
	if (!expected) throw new Error('Open the current request before deciding.');
	const startedAt = Date.now();
	const receipt = await ownerRequest<HandoffDecisionReceipt & { server_now: string }>(
		base(company) + '/handoffs/' + encodeURIComponent(item.source.reference) + '/pending-decision',
		{ method: 'POST', json: { id: crypto.randomUUID(), expected_version: expected, decision } }
	);
	showHumanDecisionReceipt(
		client,
		company,
		receipt,
		{ server_now: receipt.server_now, startedAt },
		receiptSource
	);
	if (receipt.state === 'pending' || receipt.state === 'applied') {
		await removeConfirmedAttention(client, company, item.id, receipt.state === 'pending');
	}
	void client.invalidateQueries({ queryKey: handoffDecisionsKey(company) });
	return receipt.state === 'pending' || receipt.state === 'applied';
}
