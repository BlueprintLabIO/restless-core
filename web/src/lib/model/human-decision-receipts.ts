import type { QueryClient } from '@tanstack/svelte-query';
import { goto } from '$app/navigation';
import { ownerRequest } from './setting.svelte';
import { dismissToast, toast } from '$lib/ui/feedback/toast.svelte';
import { refreshAttention } from './queries.svelte';
import { failureSentence } from './failure';
export type HumanDecisionReceipt = {
	id: string;
	item_id: string;
	kind: string;
	undo_until: string;
	state: 'pending' | 'canceled' | 'applied' | 'failed';
	problem: string | null;
};
export type ReceiptClock = { server_now: string; startedAt: number };
export type ReceiptSource<R extends HumanDecisionReceipt> = {
	keyPrefix: string;
	resource: (company: string) => string;
	queryKey: (company: string) => readonly unknown[];
	changed?: (receipt: R) => unknown;
};
const shown = new Map<
	string,
	{ state: HumanDecisionReceipt['state']; toast?: number; pendingShown: boolean }
>();
const message = (receipt: HumanDecisionReceipt) =>
	receipt.kind === 'skip'
		? 'Skipped'
		: ['complete_human_step', 'complete_document_collaboration'].includes(receipt.kind)
			? 'Marked done'
			: 'Accepted';
const itemHref = (company: string, item: string) =>
	'/' + encodeURIComponent(company) + '?item=' + encodeURIComponent(item);
async function acknowledge<R extends HumanDecisionReceipt>(
	client: QueryClient,
	company: string,
	id: string,
	source: ReceiptSource<R>
) {
	try {
		await ownerRequest(source.resource(company) + '/' + encodeURIComponent(id) + '/seen', {
			method: 'POST'
		});
		void client.invalidateQueries({ queryKey: source.queryKey(company) });
	} catch {
		/* A failed acknowledgement leaves the durable result available next time. */
	}
}
/** Shared notice rendering, never a business-decision store or HTTP replay queue. */
export function showHumanDecisionReceipt<R extends HumanDecisionReceipt>(
	client: QueryClient,
	company: string,
	receipt: R,
	clock: ReceiptClock,
	source: ReceiptSource<R>
) {
	const key = source.keyPrefix + ':' + company + ':' + receipt.id,
		previous = shown.get(key);
	if (receipt.state === 'pending') {
		if (previous) return;
		// Subtract the entire round trip; an unsynchronised device can shorten but never extend the offer.
		const expiresAt =
			clock.startedAt + Math.max(0, Date.parse(receipt.undo_until) - Date.parse(clock.server_now));
		shown.set(key, { state: 'pending', pendingShown: true });
		if (expiresAt <= Date.now()) return;
		const id = toast(message(receipt), {
			expiresAt,
			undo: async () => {
				try {
					await ownerRequest(source.resource(company) + '/' + encodeURIComponent(receipt.id), {
						method: 'DELETE'
					});
					await refreshAttention(client, company);
					void client.invalidateQueries({ queryKey: source.queryKey(company) });
					void source.changed?.(receipt);
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
		const markRead = () => void acknowledge(client, company, receipt.id, source);
		const id = toast(
			'That was not recorded. ' +
				failureSentence(
					new Error(receipt.problem ?? 'Open the request and try again.'),
					'Open the request and try again.'
				),
			{
				tone: 'failure',
				dedupeKey: key,
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
	} else {
		if (!previous?.pendingShown) toast(receipt.state === 'canceled' ? 'Undone' : message(receipt));
		shown.set(key, { state: receipt.state, pendingShown: previous?.pendingShown ?? false });
		void acknowledge(client, company, receipt.id, source);
	}
	void refreshAttention(client, company);
	void source.changed?.(receipt);
	if (shown.size > 512) {
		for (const [old, value] of shown) {
			if (value.state !== 'pending') shown.delete(old);
			if (shown.size <= 256) break;
		}
	}
}
