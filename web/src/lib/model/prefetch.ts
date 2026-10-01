/* Start loading what a link will show while the pointer rests on it, so the
 * click opens onto data instead of a skeleton. The route's code is already
 * preloaded by SvelteKit (data-sveltekit-preload-data="hover"); this adds
 * the owner-API reads the page will make. Each read uses the page's own
 * query options, so the page finds the exact cache entry, and prefetch
 * respects staleTime: a fresh entry costs nothing. */
import type { QueryClient } from '@tanstack/svelte-query';
import type { RecentDirectConversation } from './rooms';
import {
	roomMessagesOptions,
	roomParticipantsOptions,
	roomQueryKeys,
	roomReadCursorOptions
} from './room-queries.svelte';
import { documentDetailOptions } from './document-queries.svelte';

/** Where a same-origin company link leads, reduced to what it will read. */
export function prefetchTarget(
	url: URL,
	company: string,
	directRooms: RecentDirectConversation[] | undefined
): { room: string } | { document: string } | null {
	const base = `/${encodeURIComponent(company)}`;
	if (url.pathname === `${base}/people`) {
		const room = url.searchParams.get('room');
		if (room) return { room };
		const person = url.searchParams.get('person');
		const known = person && directRooms?.find((entry) => entry.person_actor_id === person);
		return known ? { room: known.room_id } : null;
	}
	if (url.pathname === `${base}/work/documents`) {
		const document = url.searchParams.get('document');
		return document ? { document } : null;
	}
	return null;
}

export function prefetchLink(client: QueryClient, company: string, href: string): void {
	let url: URL;
	try {
		url = new URL(href, window.location.href);
	} catch {
		return;
	}
	if (url.origin !== window.location.origin) return;
	const target = prefetchTarget(
		url,
		company,
		client.getQueryData<RecentDirectConversation[]>(roomQueryKeys.recentDirect(company))
	);
	if (!target) return;
	if ('room' in target) {
		void client.prefetchInfiniteQuery(roomMessagesOptions(company, target.room));
		void client.prefetchQuery(roomParticipantsOptions(company, target.room));
		void client.prefetchQuery(roomReadCursorOptions(company, target.room));
	} else {
		void client.prefetchQuery(documentDetailOptions(client, company, target.document));
	}
}

/** Prefetch on a short hover (intent, not a pass-over), on focus, and on touch. */
export function prefetchOnIntent(
	root: HTMLElement,
	options: { client: QueryClient; company: () => string }
) {
	let timer: number | undefined;
	const linkFrom = (event: Event) =>
		(event.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null;
	const go = (link: HTMLAnchorElement) =>
		prefetchLink(options.client, options.company(), link.href);
	const over = (event: PointerEvent) => {
		const link = linkFrom(event);
		window.clearTimeout(timer);
		if (link) timer = window.setTimeout(() => go(link), 70);
	};
	const now = (event: Event) => {
		const link = linkFrom(event);
		if (link) go(link);
	};
	root.addEventListener('pointerover', over);
	root.addEventListener('focusin', now);
	root.addEventListener('touchstart', now, { passive: true });
	return {
		destroy() {
			window.clearTimeout(timer);
			root.removeEventListener('pointerover', over);
			root.removeEventListener('focusin', now);
			root.removeEventListener('touchstart', now);
		}
	};
}
