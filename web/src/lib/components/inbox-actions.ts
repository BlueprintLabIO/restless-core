import { goto } from '$app/navigation';
import { SHOW_CONVERSATION_EVENT } from '$lib/model/ask-exec';
/* The Inbox's own moves on one item, shared by the list's row menu, the item detail and the
 * computer view, so all say the same thing and all can be taken back: set aside (with Undo),
 * snooze until a chosen time (with Undo), mark a human step done (with Undo), and copy the
 * item's link. */
import { createQuery, type QueryClient } from '@tanstack/svelte-query';
import { dismissToast, toast } from '$lib/ui/feedback/toast.svelte';
import { formatMoment } from '$lib/ui/time';
import { completeHandoffHumanStep, dismissAttention, restoreAttention } from '$lib/model/attention';
import { fetchAppRequests } from '$lib/model/app-requests';
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

/* Marking a step done resumes the Work, which cannot be taken back once it lands; so it waits
 * out an Undo window first, whichever way it was pressed (button, row menu or E). */
const DONE_UNDO_MS = 5000;
const markingDone = new Set<string>();

/** Mark a human step done after a few seconds in which Undo takes it back. Resolves true once
 * it is recorded, false when undone, already pending, or refused (the refusal is said). */
export function markDoneSoon(
	client: QueryClient,
	companyId: string,
	handoff: string
): Promise<boolean> {
	const key = `${companyId} ${handoff}`;
	if (markingDone.has(key)) return Promise.resolve(false);
	markingDone.add(key);
	return new Promise((resolve) => {
		const timer = setTimeout(async () => {
			dismissToast(id);
			try {
				await completeHandoffHumanStep(companyId, handoff);
				resolve(true);
			} catch (cause) {
				toast(failureSentence(cause, 'That was not recorded.'), { tone: 'failure' });
				resolve(false);
			} finally {
				markingDone.delete(key);
				await refreshAttention(client, companyId);
			}
		}, DONE_UNDO_MS);
		const id = toast('Marking done. The work resumes in a moment.', {
			undo: () => {
				clearTimeout(timer);
				markingDone.delete(key);
				resolve(false);
			}
		});
	});
}

/** The handoffs that are app requests: adding the app resumes them, so they have no Mark done.
 * The owner's only (Apps are the owner's); a refused read is the query's error to show. */
export function appRequestHandoffs(companyId: () => string, enabled: () => boolean) {
	return createQuery(() => ({
		queryKey: ['app-requests', companyId()],
		queryFn: () => fetchAppRequests(companyId()),
		enabled: enabled() && Boolean(companyId()),
		staleTime: 30_000,
		refetchInterval: 60_000,
		retry: 1
	}));
}

/** An explicit Discuss click opens chat after navigation. Returning from a preview alone must
 * not reopen the phone overlay; modifier clicks retain ordinary link behavior. */
export async function discussInbox(event: MouseEvent) {
	if (
		event.defaultPrevented ||
		event.button !== 0 ||
		event.metaKey ||
		event.ctrlKey ||
		event.shiftKey ||
		event.altKey
	)
		return;
	const link = event.currentTarget as HTMLAnchorElement;
	event.preventDefault();
	try {
		await goto(link.href);
		window.dispatchEvent(new Event(SHOW_CONVERSATION_EVENT));
	} catch (cause) {
		toast(failureSentence(cause, 'The conversation could not be opened.'));
	}
}
