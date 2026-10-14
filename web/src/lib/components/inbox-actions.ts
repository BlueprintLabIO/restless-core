/* The Inbox's own moves on one item, shared by the list's row menu and the item detail, so
 * both say the same thing and both can be taken back: set aside (with Undo), snooze until a
 * chosen time (with Undo), and copy the item's link. */
import type { QueryClient } from '@tanstack/svelte-query';
import { toast } from '$lib/ui/feedback/toast.svelte';
import { formatMoment } from '$lib/ui/time';
import { dismissAttention, restoreAttention } from '$lib/model/attention';
import { failureSentence } from '$lib/model/failure';
import { setInboxMarks, withMarks, type InboxMark, type InboxMarkChange } from '$lib/model/for-you';
import { refreshAttention } from '$lib/model/queries.svelte';

export const inboxMarksKey = (companyId: string) => ['inbox-marks', companyId] as const;

/** Keep what this person opened or snoozed, on every device. */
export async function markInbox(
	client: QueryClient,
	companyId: string,
	changes: InboxMarkChange[],
	quiet = false
) {
	const key = inboxMarksKey(companyId);
	client.setQueryData<InboxMark[]>(key, (data) => withMarks(data ?? [], changes));
	try {
		await setInboxMarks(companyId, changes);
	} catch (cause) {
		/* A seen mark that did not stick only leaves a dot; it is tried again next time. */
		if (!quiet) toast(failureSentence(cause, 'That was not kept.'), { tone: 'failure' });
	} finally {
		void client.invalidateQueries({ queryKey: key });
	}
}

/** Off the Inbox until the given moment, with Undo. */
export function snoozeInboxItem(
	client: QueryClient,
	companyId: string,
	itemId: string,
	until: Date
) {
	void markInbox(client, companyId, [{ item_id: itemId, snoozed_until: until.toISOString() }]);
	toast(`Snoozed until ${formatMoment(until)}`, {
		undo: () => markInbox(client, companyId, [{ item_id: itemId, snoozed_until: null }])
	});
}

export function wakeInboxItem(client: QueryClient, companyId: string, itemId: string) {
	return markInbox(client, companyId, [{ item_id: itemId, snoozed_until: null }]);
}

/** Set aside: off the Inbox and kept under Set aside, with Undo. Whoever asked stops waiting.
 * True when it was set aside. */
export async function setAsideInboxItem(
	client: QueryClient,
	companyId: string,
	itemId: string
): Promise<boolean> {
	let done = false;
	try {
		await dismissAttention(companyId, itemId);
		done = true;
		toast('Set aside', {
			undo: async () => {
				try {
					await restoreAttention(companyId, itemId);
				} catch (cause) {
					toast(failureSentence(cause, 'That was not restored.'), { tone: 'failure' });
				}
				await refreshAttention(client, companyId);
			}
		});
	} catch (cause) {
		toast(failureSentence(cause, 'That was not set aside.'), { tone: 'failure' });
	}
	await refreshAttention(client, companyId);
	return done;
}

export const inboxItemHref = (companyId: string, itemId: string) =>
	`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(itemId)}`;

export async function copyInboxLink(companyId: string, itemId: string) {
	try {
		await navigator.clipboard.writeText(
			new URL(inboxItemHref(companyId, itemId), location.origin).href
		);
		toast('Link copied');
	} catch {
		toast('Could not copy the link', { tone: 'failure' });
	}
}
