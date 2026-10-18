import type { QueryClient, QueryKey } from '@tanstack/svelte-query';

/**
 * Revalidate a Room read after a live event. A query still fetching its first answer is not
 * restarted by an invalidation: the request already in flight is reused, and it began before this
 * event, so the event's message would be missing until the next slow poll. The multiplayer smoke
 * caught exactly that (two people writing as a third opened the Room). When the first read is
 * still in flight, wait for it, then read once more.
 */
export async function revalidateAfterEvent(client: QueryClient, queryKey: QueryKey): Promise<void> {
	const state = client.getQueryState(queryKey);
	const firstReadInFlight = state?.data === undefined && state?.fetchStatus === 'fetching';
	await client.invalidateQueries({ queryKey });
	if (firstReadInFlight) await client.invalidateQueries({ queryKey });
}
