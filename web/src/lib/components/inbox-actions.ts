import { queueHandoffDecision, type HandoffDecision } from '$lib/model/handoff-decisions';
import { pollEvery } from '$lib/model/connection.svelte';
/* The Inbox's own moves on one item, shared by the list's row menu, the item detail and the
 * computer view, so all say the same thing and all can be taken back: skip (with Undo),
 * snooze until a chosen time (with Undo), mark a human step done (with Undo), and copy the
 * item's link. */
import { createQuery, type QueryClient } from '@tanstack/svelte-query';
import { toast } from '$lib/ui/feedback/toast.svelte';
import { formatMoment } from '$lib/ui/time';
import {
	dismissAttention,
	restoreAttention
} from '$lib/model/attention';
import { fetchAppRequests } from '$lib/model/app-requests';
import { failureSentence } from '$lib/model/failure';
import { setInboxMarks, withMarks, type InboxMark, type InboxMarkChange } from '$lib/model/for-you';
import { queryKeys, refreshAttention } from '$lib/model/queries.svelte';
import type { AttentionItem } from '$lib/model/view';
import { getCompanyPrincipal } from '$lib/model/query-persistence';

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

/** Skip: off the Inbox and kept under Skipped, with Undo. Whoever asked stops waiting.
 * True when it was skipped. */
export async function setAsideInboxItem(
	client: QueryClient,
	companyId: string,
	itemId: string,
	who = 'Whoever asked'
): Promise<boolean> {
	let done = false;
	try {
		await dismissAttention(companyId, itemId);
		done = true;
		toast(`Skipped. ${who} carries on without it.`, {
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
		toast(failureSentence(cause, 'That was not skipped.'), { tone: 'failure' });
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

/** Mark a human step done after its durable Undo window. Returns once saved, not once applied. */
export function markDoneSoon(
    client: QueryClient,
    companyId: string,
    item: AttentionItem
): Promise<boolean> {
    return saveHandoffDecision(client, companyId, item, { kind: 'complete_human_step' });
}

export const skipsByHandoff = (item: AttentionItem) =>
    item.source.kind === 'owner_handoff' &&
    item.actions.some(
        (action) => action.id === 'record-decision' || action.id === 'complete-human-step'
    );

export function skipHandoffSoon(client: QueryClient, companyId: string, item: AttentionItem): Promise<boolean> {
    return saveHandoffDecision(client, companyId, item, { kind: 'skip' });
}

async function saveHandoffDecision(
    client: QueryClient,
    companyId: string,
    item: AttentionItem,
    decision: HandoffDecision
): Promise<boolean> {
    try {
        return await queueHandoffDecision(client, companyId, item, decision);
    } catch (cause) {
        toast(failureSentence(cause, 'That was not recorded.'), { tone: 'failure' });
        return false;
    }
}

/** The handoffs that are app requests: adding the app resumes them, so they have no Mark done.
 * The owner's only (Apps are the owner's); a refused read is the query's error to show. */
export function appRequestHandoffs(companyId: () => string, enabled: () => boolean) {
	/* Admins can set Inbox items aside too, but app requests are read by the owner alone: the
	 * gateway refuses anyone else, and the multiplayer smoke caught an admin's every screen asking. */
	const principal = createQuery(() => ({
		queryKey: queryKeys.principal(companyId()),
		queryFn: ({ signal }) => getCompanyPrincipal(companyId(), signal),
		enabled: Boolean(companyId()),
		staleTime: 60_000
	}));
	return createQuery(() => ({
		queryKey: ['app-requests', companyId()],
		queryFn: () => fetchAppRequests(companyId()),
		enabled:
			enabled() &&
			Boolean(companyId()) &&
			(principal.data as { membership_role?: string } | null | undefined)?.membership_role ===
				'owner',
		staleTime: 30_000,
		refetchInterval: pollEvery(60_000),
		retry: 1
	}));
}
