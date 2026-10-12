/* One For you source per company shell. The shell reads it for the Inbox badge and desktop
 * notices; the Inbox lists it; a document or sheet marks its own changes read. Everyone shares
 * this one query rather than polling the same endpoint on several clocks. */
import { createInfiniteQuery, useQueryClient, type InfiniteData } from '@tanstack/svelte-query';
import { getContext, setContext } from 'svelte';
import {
	archiveForYou,
	getForYou,
	markAllForYouRead,
	markForYouRead,
	markForYouUnread,
	withoutItems,
	withRead,
	withUnread,
	type ForYouFilter,
	type ForYouItem,
	type ForYouPage
} from './for-you.ts';

const REFRESH_MS = 20_000;
const CONTEXT = Symbol('for-you');

export interface ForYouSource {
	readonly items: ForYouItem[];
	readonly unreadCount: number;
	readonly status: 'unknown' | 'live' | 'stale';
	readonly failure: Error | null;
	readonly hasMore: boolean;
	readonly loadingMore: boolean;
	/** All items, or only the unread ones. */
	readonly filter: ForYouFilter;
	setFilter(filter: ForYouFilter): void;
	loadMore(): void;
	refresh(): void;
	/** Marks items read here at once and on the server; a refusal puts the server's answer back.
	 * Answers which listed items were unread, so the caller can offer to undo it. */
	read(keys: string[] | 'all'): Promise<string[]>;
	/** Unread again until opened. */
	unread(keys: string[]): Promise<void>;
	/** Off the list until something new happens there; `false` brings them back. */
	archive(keys: string[], archived?: boolean): Promise<void>;
}

type Pages = InfiniteData<ForYouPage, string | null>;

export function forYouSource(
	companyId: () => string,
	actorId: () => string,
	background: () => boolean
): ForYouSource {
	const client = useQueryClient();
	let filter = $state<ForYouFilter>('all');
	const base = () => ['for-you', companyId(), actorId()] as const;
	const key = () => [...base(), filter] as const;
	const query = createInfiniteQuery(() => ({
		queryKey: key(),
		queryFn: ({ pageParam }: { pageParam: string | null }) =>
			getForYou(companyId(), pageParam, filter),
		initialPageParam: null as string | null,
		getNextPageParam: (page: ForYouPage) => page.next_before ?? undefined,
		enabled: Boolean(companyId() && actorId()),
		staleTime: 5_000,
		gcTime: 10 * 60_000,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: background()
	}));
	const items = () => {
		/* Pages are read at different moments; an item read twice is listed once. */
		const seen = new Set<string>();
		return (query.data?.pages.flatMap((page) => page.items) ?? []).filter(
			(item) => !seen.has(item.key) && Boolean(seen.add(item.key))
		);
	};
	/* Applies the change here at once, on every filter's copy, then asks the server. */
	async function change(update: (page: ForYouPage) => ForYouPage, send: () => Promise<void>) {
		client.setQueriesData<Pages>({ queryKey: base() }, (data) =>
			data ? { ...data, pages: data.pages.map(update) } : data
		);
		try {
			await send();
		} finally {
			void client.invalidateQueries({ queryKey: base() });
		}
	}
	return {
		get items() {
			return items();
		},
		get unreadCount() {
			return query.data?.pages[0]?.unread_count ?? 0;
		},
		get status() {
			if (query.isPending && !query.data) return 'unknown';
			return query.isError ? (query.data ? 'stale' : 'unknown') : 'live';
		},
		get failure() {
			return (query.error as Error | null) ?? null;
		},
		get hasMore() {
			return Boolean(query.hasNextPage);
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		get filter() {
			return filter;
		},
		setFilter(next) {
			filter = next;
		},
		loadMore() {
			if (query.hasNextPage && !query.isFetchingNextPage) void query.fetchNextPage();
		},
		refresh() {
			void query.refetch();
		},
		async read(keys) {
			if (keys !== 'all' && !keys.length) return [];
			const wasUnread = items()
				.filter((item) => item.unread && (keys === 'all' || keys.includes(item.key)))
				.map((item) => item.key);
			await change(
				(page) => withRead(page, keys),
				() => (keys === 'all' ? markAllForYouRead(companyId()) : markForYouRead(companyId(), keys))
			);
			return wasUnread;
		},
		async unread(keys) {
			if (!keys.length) return;
			await change(
				(page) => withUnread(page, keys),
				() => markForYouUnread(companyId(), keys)
			);
		},
		async archive(keys, archived = true) {
			if (!keys.length) return;
			await change(
				(page) => (archived ? withoutItems(page, keys) : page),
				() => archiveForYou(companyId(), keys, archived)
			);
		}
	};
}

export function provideForYou(source: ForYouSource): void {
	setContext(CONTEXT, source);
}

/** The shell's For you, or null outside a company shell. */
export function useForYou(): ForYouSource | null {
	return getContext<ForYouSource | undefined>(CONTEXT) ?? null;
}
