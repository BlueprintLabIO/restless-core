import type { QueryClient } from '@tanstack/svelte-query';
import { goto } from '$app/navigation';
import { ownerRequest } from './setting.svelte';
import type { AttentionItem, OwnerReviewVersion } from './view';
import { dismissToast, toast } from '$lib/ui/feedback/toast.svelte';
import { removeConfirmedAttention, refreshAttention } from './queries.svelte';
import { failureSentence } from './failure';

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

const shown = new Map<
	string,
	{ state: HandoffDecisionReceipt['state']; toast?: number; pendingShown: boolean }
>();
const message = (receipt: HandoffDecisionReceipt) =>
	receipt.kind === 'complete_human_step'
		? 'Marked done'
		: receipt.kind === 'skip'
			? 'Skipped'
			: 'Accepted';
const itemHref = (company: string, item: string) =>
	'/' + encodeURIComponent(company) + '?item=' + encodeURIComponent(item);

async function acknowledge(client: QueryClient, company: string, id: string) {
	try {
		await ownerRequest(base(company) + '/handoff-decisions/' + encodeURIComponent(id) + '/seen', {
			method: 'POST'
		});
		void client.invalidateQueries({ queryKey: handoffDecisionsKey(company) });
	} catch {
		// This read receipt can be offered again; never hide a failed business decision on a failed acknowledgement.
	}
}

function showReceipt(
	client: QueryClient,
	company: string,
	receipt: HandoffDecisionReceipt,
	clock: Clock
) {
	const key = company + ':' + receipt.id;
	const previous = shown.get(key);
	if (receipt.state === 'pending') {
		if (previous) return; // A poll/replay never restarts the Undo clock.
		// Subtract the whole round trip: an unsynchronised device may shorten
		// the offer slightly, but cannot extend the server's original deadline.
		const expiresAt =
			clock.startedAt + Math.max(0, Date.parse(receipt.undo_until) - Date.parse(clock.server_now));
		shown.set(key, { state: 'pending', pendingShown: true });
		if (expiresAt <= Date.now()) return;
		const id = toast(message(receipt), {
			expiresAt,
			undo: async () => {
				try {
					await ownerRequest(
						base(company) + '/handoff-decisions/' + encodeURIComponent(receipt.id),
						{ method: 'DELETE' }
					);
					await refreshAttention(client, company);
					void client.invalidateQueries({ queryKey: handoffDecisionsKey(company) });
				} catch (cause) {
					toast(failureSentence(cause, 'That could not be undone.'), { tone: 'failure' });
					void refreshAttention(client, company);
				}
			}
		});
		shown.set(key, { state: 'pending', toast: id, pendingShown: true });
		return;
	}
	if (previous?.state === receipt.state) return;
	if (previous?.toast !== undefined) dismissToast(previous.toast);
	if (receipt.state === 'failed') {
		const markRead = () => void acknowledge(client, company, receipt.id);
		const id = toast(
			'That was not recorded. ' +
				failureSentence(
					new Error(receipt.problem ?? 'Open the request and try again.'),
					'Open the request and try again.'
				),
			{
				tone: 'failure',
				onDismiss: markRead,
				action: {
					label: 'Open request',
					run: () => {
						markRead();
						return goto(itemHref(company, receipt.item_id));
					}
				}
			}
		);
		shown.set(key, {
			state: receipt.state,
			toast: id,
			pendingShown: previous?.pendingShown ?? false
		});
		void refreshAttention(client, company);
	} else {
		if (!previous?.pendingShown) toast(receipt.state === 'canceled' ? 'Undone' : message(receipt));
		shown.set(key, { state: receipt.state, pendingShown: previous?.pendingShown ?? false });
		void acknowledge(client, company, receipt.id);
		void refreshAttention(client, company);
	}
	// Only UI deduplication lives here; PostgreSQL holds every decision and receipt.
	if (shown.size > 512) {
		for (const [old, value] of shown) {
			if (value.state !== 'pending') shown.delete(old);
			if (shown.size <= 256) break;
		}
	}
}

export function reconcileHandoffDecisions(
	client: QueryClient,
	company: string,
	reply: HandoffDecisionRead
) {
	for (const receipt of reply.items) showReceipt(client, company, receipt, reply);
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
	showReceipt(client, company, receipt, { server_now: receipt.server_now, startedAt });
	if (receipt.state === 'pending' || receipt.state === 'applied') {
		await removeConfirmedAttention(client, company, item.id, receipt.state === 'pending');
	}
	void client.invalidateQueries({ queryKey: handoffDecisionsKey(company) });
	return receipt.state === 'pending' || receipt.state === 'applied';
}
