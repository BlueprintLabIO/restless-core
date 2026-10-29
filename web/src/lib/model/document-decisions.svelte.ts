import type { QueryClient } from '@tanstack/svelte-query';
import { createQuery } from '@tanstack/svelte-query';
import { companyPrincipalQuery, removeConfirmedAttention } from './queries.svelte';
import { invalidateDocument } from './document-queries.svelte';
import { ownerRequest } from './setting.svelte';
import { pollEvery } from './connection.svelte';
import {
	showHumanDecisionReceipt,
	type HumanDecisionReceipt,
	type ReceiptClock
} from './human-decision-receipts';
export type NativeDocumentDecision =
	| {
			kind: 'accept_review';
			expected_document_version: number;
			expected_review_version: number;
			accepted_version_name: string;
			feedback: string;
	  }
	| { kind: 'complete_collaboration'; expected_document_version: number; expected_summary: string };
export type DocumentDecisionReceipt = HumanDecisionReceipt & {
	document_id: string;
	request_id: string;
	kind: 'accept_document_review' | 'complete_document_collaboration';
};
type DecisionRead = ReceiptClock & { items: DocumentDecisionReceipt[] };
const base = (company: string) => '/api/companies/' + encodeURIComponent(company);
export const documentDecisionsKey = (company: string) => ['document-decisions', company] as const;
async function read(company: string, signal?: AbortSignal): Promise<DecisionRead> {
	const startedAt = Date.now();
	const data = await ownerRequest<Omit<DecisionRead, 'startedAt'>>(
		base(company) + '/document-decisions',
		{ signal }
	);
	return { ...data, startedAt };
}
export function documentDecisionsQuery(company: () => string) {
	const principal = $derived(companyPrincipalQuery(company()).view);
	return createQuery(() => ({
		queryKey: [...documentDecisionsKey(company()), principal?.cache_partition ?? ''],
		queryFn: ({ signal }) => read(company(), signal),
		enabled: Boolean(company() && principal),
		refetchInterval: (query) =>
			query.state.data?.items.some((item) => item.state === 'pending') ? 1000 : pollEvery(1000)(),
		retry: 1
	}));
}
function source(client: QueryClient, company: string) {
	return {
		keyPrefix: 'document',
		resource: (company: string) => base(company) + '/document-decisions',
		queryKey: documentDecisionsKey,
		changed: (receipt: DocumentDecisionReceipt) =>
			invalidateDocument(client, company, receipt.document_id)
	};
}
export function reconcileDocumentDecisions(
	client: QueryClient,
	company: string,
	reply: DecisionRead
) {
	for (const receipt of reply.items)
		showHumanDecisionReceipt(client, company, receipt, reply, source(client, company));
}
export async function queueDocumentDecision(
	client: QueryClient,
	company: string,
	document: string,
	request: string,
	id: string,
	decision: NativeDocumentDecision
): Promise<DocumentDecisionReceipt> {
	const startedAt = Date.now(),
		kind = decision.kind === 'accept_review' ? 'reviews' : 'collaboration';
	const receipt = await ownerRequest<DocumentDecisionReceipt & { server_now: string }>(
		base(company) +
			'/documents/' +
			encodeURIComponent(document) +
			'/' +
			kind +
			'/' +
			encodeURIComponent(request) +
			'/pending-decision',
		{ method: 'POST', json: { id, decision } }
	);
	showHumanDecisionReceipt(
		client,
		company,
		receipt,
		{ server_now: receipt.server_now, startedAt },
		source(client, company)
	);
	if (receipt.state === 'pending' || receipt.state === 'applied')
		await removeConfirmedAttention(client, company, receipt.item_id, true);
	void client.invalidateQueries({ queryKey: documentDecisionsKey(company) });
	return receipt;
}

export function documentDecisionDraftQuery(
	company: () => string,
	document: () => string,
	review: () => string
) {
	const principal = $derived(companyPrincipalQuery(company()).view);
	return createQuery(() => ({
		queryKey: [
			'document-decision-draft',
			company(),
			document(),
			review(),
			principal?.cache_partition ?? ''
		],
		enabled: Boolean(company() && document() && review() && principal),
		queryFn: ({ signal }) =>
			ownerRequest<{
				draft: {
					id: string;
					state: string;
					decision: Extract<NativeDocumentDecision, { kind: 'accept_review' }>;
				} | null;
			}>(
				base(company()) +
					'/documents/' +
					encodeURIComponent(document()) +
					'/reviews/' +
					encodeURIComponent(review()) +
					'/decision-draft',
				{ signal }
			),
		staleTime: 0,
		retry: 1
	}));
}
