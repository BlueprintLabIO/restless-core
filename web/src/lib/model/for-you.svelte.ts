/* One For you source per company shell. The shell reads it for the Inbox badge and desktop
 * notices; the Inbox lists it; a document marks its own changes read. Everyone shares this one
 * query rather than polling the same endpoint on several clocks. */
import { createInfiniteQuery, useQueryClient, type InfiniteData } from '@tanstack/svelte-query';
import { getContext, setContext } from 'svelte';
import {
	getForYou,
	markAllForYouRead,
	markForYouRead,
	withRead,
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
	loadMore(): void;
	refresh(): void;
	/** Marks items read here at once and on the server; a refusal puts the server's answer back. */
	read(keys: string[] | 'all'): Promise<void>;
}

export function forYouSource(
	companyId: () => string,
	actorId: () => string,
	background: () => boolean
): ForYouSource {
	const client = useQueryClient();
	const key = () => ['for-you', companyId(), actorId()] as const;
	const query = createInfiniteQuery(() => ({
		queryKey: key(),
		queryFn: ({ pageParam }: { pageParam: string | null }) => getForYou(companyId(), pageParam),
		initialPageParam: null as string | null,
		getNextPageParam: (page: ForYouPage) => page.next_before ?? undefined,
		enabled: Boolean(companyId() && actorId()),
		staleTime: 5_000,
		gcTime: 10 * 60_000,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: background()
	}));
	return {
		get items() {
			/* Pages are read at different moments; an item read twice is listed once. */
			const seen = new Set<string>();
			return (query.data?.pages.flatMap((page) => page.items) ?? []).filter(
				(item) => !seen.has(item.key) && Boolean(seen.add(item.key))
			);
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
		loadMore() {
			if (query.hasNextPage && !query.isFetchingNextPage) void query.fetchNextPage();
		},
		refresh() {
			void query.refetch();
		},
		async read(keys) {
			if (keys !== 'all' && !keys.length) return;
			const queryKey = key();
			client.setQueryData<InfiniteData<ForYouPage, string | null>>(queryKey, (data) =>
				data ? { ...data, pages: data.pages.map((page) => withRead(page, keys)) } : data
			);
			try {
				await (keys === 'all' ? markAllForYouRead(companyId()) : markForYouRead(companyId(), keys));
			} finally {
				void client.invalidateQueries({ queryKey });
			}
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
