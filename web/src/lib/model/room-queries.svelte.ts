import { createInfiniteQuery, createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { InfiniteData } from '@tanstack/svelte-query';
import {
	editRoomMessagePages,
	getRoomEventSnapshot,
	getRoomMessageRevisions,
	getRoomMessages,
	getRoomParticipants,
	getRoomReadCursor,
	getRoomThread,
	getRooms,
	openRoomEventStream,
	removeRoomMessageSearchResult,
	searchRoomMessages,
	searchRooms,
	tombstoneRoomMessagePages,
	type RoomEvent,
	type RoomListCursor,
	type RoomListPage,
	type RoomMessageEditResult,
	type RoomMessagePage,
	type RoomMessageRevisionPage,
	type RoomMessageSearchPage,
	type RoomMessageSendResult,
	type RoomStreamRestartReason,
	type RoomTransport
} from './rooms';

const ROOM_STALE_MS = 5_000;
const ROOM_RETAIN_MS = 10 * 60_000;
const ROOM_REFRESH_MS = 15_000;

export const roomQueryKeys = {
	list: (company: string) => ['rooms', company] as const,
	roomSearch: (company: string, search: string) => ['room-search', company, search] as const,
	messageSearch: (company: string, search: string) =>
		['room-message-search', company, search] as const,
	messages: (company: string, room: string) => ['room-messages', company, room] as const,
	thread: (company: string, room: string, root: number) =>
		['room-thread', company, room, root] as const,
	revisions: (company: string, room: string, message: number) =>
		['room-message-revisions', company, room, message] as const,
	participants: (company: string, room: string) => ['room-participants', company, room] as const,
	readCursor: (company: string, room: string) => ['room-read-cursor', company, room] as const
};

function sourceStatus(query: {
	data?: unknown;
	isPending: boolean;
	isError: boolean;
}): 'unknown' | 'live' | 'stale' {
	if (query.isPending && !query.data) return 'unknown';
	return query.isError ? (query.data ? 'stale' : 'unknown') : 'live';
}

function roomListCursor(page: RoomListPage): RoomListCursor | undefined {
	return page.has_more && page.next_before_created_at && page.next_before_room_id
		? {
				beforeCreatedAt: page.next_before_created_at,
				beforeRoomId: page.next_before_room_id
			}
		: undefined;
}

export function roomsQuery(companyId: string) {
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.list(companyId),
		queryFn: ({ pageParam }: { pageParam: RoomListCursor | null }) =>
			getRooms(companyId, pageParam),
		initialPageParam: null as RoomListCursor | null,
		getNextPageParam: roomListCursor,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get rooms() {
			return query.data?.pages.flatMap((page) => page.rooms) ?? [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch()
	};
}

export function roomTitleSearchQuery(companyId: string, search: string) {
	const normalized = search.trim();
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.roomSearch(companyId, normalized),
		queryFn: ({ pageParam, signal }: { pageParam: RoomListCursor | null; signal: AbortSignal }) =>
			searchRooms(companyId, normalized, pageParam, 30, signal),
		initialPageParam: null as RoomListCursor | null,
		getNextPageParam: roomListCursor,
		enabled: normalized.length > 0,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get rooms() {
			return query.data?.pages.flatMap((page) => page.rooms) ?? [];
		},
		get status() {
			return normalized ? sourceStatus(query) : 'live';
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage()
	};
}

function olderSearchCursor(page: RoomMessageSearchPage): number | undefined {
	return page.has_more && page.next_before_message_id !== null
		? page.next_before_message_id
		: undefined;
}

export function roomMessageSearchQuery(companyId: string, search: string) {
	const normalized = search.trim();
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.messageSearch(companyId, normalized),
		queryFn: ({ pageParam, signal }: { pageParam: number | null; signal: AbortSignal }) =>
			searchRoomMessages(companyId, normalized, pageParam, 20, signal),
		initialPageParam: null as number | null,
		getNextPageParam: olderSearchCursor,
		enabled: normalized.length > 0,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get messages() {
			return query.data?.pages.flatMap((page) => page.messages) ?? [];
		},
		get status() {
			return normalized ? sourceStatus(query) : 'live';
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage()
	};
}

function olderMessageCursor(page: RoomMessagePage): number | undefined {
	return page.has_more && page.next_before_message_id !== null
		? page.next_before_message_id
		: undefined;
}

/* TanStack stores infinite pages in fetch order: latest, then progressively
 * older. Stable monotonic Message ids let the rendered projection prepend
 * older pages and deduplicate the Thread root anchor in one bounded merge. */
function messagesInDisplayOrder(pages: RoomMessagePage[]) {
	const byId = new Map(
		pages.flatMap((page) => page.messages).map((message) => [message.id, message])
	);
	return [...byId.values()].sort((left, right) => left.id - right.id);
}

function uniqueMentions(pages: RoomMessagePage[]) {
	const byId = new Map(
		pages.flatMap((page) => page.mentions).map((mention) => [mention.id, mention])
	);
	return [...byId.values()];
}

export function roomMessagesQuery(companyId: string, roomId: string) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.messages(companyId, roomId),
		queryFn: ({ pageParam }: { pageParam: number | null }) =>
			getRoomMessages(companyId, roomId, pageParam),
		initialPageParam: null as number | null,
		getNextPageParam: olderMessageCursor,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		refetchInterval: ROOM_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
	}));
	return {
		get messages() {
			return query.data ? messagesInDisplayOrder(query.data.pages) : [];
		},
		get mentions() {
			return query.data ? uniqueMentions(query.data.pages) : [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch(),
		accept(result: RoomMessageSendResult): void {
			client.setQueryData<InfiniteData<RoomMessagePage, number | null>>(
				roomQueryKeys.messages(companyId, roomId),
				(current) => appendCanonicalMessage(current, result)
			);
		},
		acceptEdit(messageId: number, result: RoomMessageEditResult): void {
			patchCachedMessageEdit(client, companyId, roomId, messageId, result);
		},
		acceptDelete(messageId: number, deletedAt: string): void {
			patchCachedMessageDelete(client, companyId, roomId, messageId, deletedAt);
		}
	};
}

function mapInfinitePages(
	current: InfiniteData<RoomMessagePage, number | null> | undefined,
	map: (pages: RoomMessagePage[]) => RoomMessagePage[]
): InfiniteData<RoomMessagePage, number | null> | undefined {
	return current ? { ...current, pages: map(current.pages) } : current;
}

function patchCachedMessageEdit(
	client: ReturnType<typeof useQueryClient>,
	companyId: string,
	roomId: string,
	messageId: number,
	result: RoomMessageEditResult
): void {
	const apply = (current: InfiniteData<RoomMessagePage, number | null> | undefined) =>
		mapInfinitePages(current, (pages) =>
			editRoomMessagePages(
				pages,
				messageId,
				result.revision.body,
				result.revision.created_at,
				result.revision.revision_number
			)
		);
	client.setQueryData(roomQueryKeys.messages(companyId, roomId), apply);
	client.setQueriesData({ queryKey: ['room-thread', companyId, roomId] }, apply);
	const removeFromSearch = (
		current: InfiniteData<RoomMessageSearchPage, number | null> | undefined
	): InfiniteData<RoomMessageSearchPage, number | null> | undefined =>
		current
			? {
					...current,
					pages: removeRoomMessageSearchResult(current.pages, messageId)
				}
			: current;
	// Whether the edited text still matches each active query is only knowable
	// to the server. Remove it until the bounded search projection revalidates.
	client.setQueriesData({ queryKey: ['room-message-search', companyId] }, removeFromSearch);
	void client.invalidateQueries({ queryKey: ['room-message-search', companyId] });
	void client.invalidateQueries({
		queryKey: roomQueryKeys.revisions(companyId, roomId, messageId)
	});
}

function patchCachedMessageDelete(
	client: ReturnType<typeof useQueryClient>,
	companyId: string,
	roomId: string,
	messageId: number,
	deletedAt: string
): void {
	const apply = (current: InfiniteData<RoomMessagePage, number | null> | undefined) =>
		mapInfinitePages(current, (pages) => tombstoneRoomMessagePages(pages, messageId, deletedAt));
	client.setQueryData(roomQueryKeys.messages(companyId, roomId), apply);
	client.setQueriesData({ queryKey: ['room-thread', companyId, roomId] }, apply);
	const removeFromSearch = (
		current: InfiniteData<RoomMessageSearchPage, number | null> | undefined
	): InfiniteData<RoomMessageSearchPage, number | null> | undefined =>
		current
			? { ...current, pages: removeRoomMessageSearchResult(current.pages, messageId) }
			: current;
	client.setQueriesData({ queryKey: ['room-message-search', companyId] }, removeFromSearch);
	client.removeQueries({ queryKey: roomQueryKeys.revisions(companyId, roomId, messageId) });
}

function appendCanonicalMessage(
	current: InfiniteData<RoomMessagePage, number | null> | undefined,
	result: RoomMessageSendResult
): InfiniteData<RoomMessagePage, number | null> {
	if (!current) {
		return {
			pages: [
				{
					messages: [result.message],
					mentions: result.mentions,
					next_before_message_id: null,
					has_more: false
				}
			],
			pageParams: [null]
		};
	}
	if (
		current.pages.some((page) => page.messages.some((message) => message.id === result.message.id))
	) {
		return current;
	}
	const pages = current.pages.map((page) => ({
		...page,
		messages: [...page.messages],
		mentions: [...page.mentions]
	}));
	/* Infinite-query pages are [latest, older, oldest]. New canonical writes
	 * belong to the latest page; its older-history cursor must stay unchanged. */
	const latest = pages[0];
	latest.messages.push(result.message);
	latest.mentions.push(...result.mentions);
	return { ...current, pages };
}

export function roomThreadQuery(companyId: string, roomId: string, rootMessageId: number) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.thread(companyId, roomId, rootMessageId),
		queryFn: ({ pageParam }: { pageParam: number | null }) =>
			getRoomThread(companyId, roomId, rootMessageId, pageParam),
		initialPageParam: null as number | null,
		getNextPageParam: olderMessageCursor,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		refetchInterval: ROOM_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
	}));
	return {
		get messages() {
			return query.data ? messagesInDisplayOrder(query.data.pages) : [];
		},
		get mentions() {
			return query.data ? uniqueMentions(query.data.pages) : [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch(),
		accept(result: RoomMessageSendResult): void {
			client.setQueryData<InfiniteData<RoomMessagePage, number | null>>(
				roomQueryKeys.thread(companyId, roomId, rootMessageId),
				(current) => appendCanonicalMessage(current, result)
			);
		},
		acceptEdit(messageId: number, result: RoomMessageEditResult): void {
			patchCachedMessageEdit(client, companyId, roomId, messageId, result);
		},
		acceptDelete(messageId: number, deletedAt: string): void {
			patchCachedMessageDelete(client, companyId, roomId, messageId, deletedAt);
		}
	};
}

function olderRevisionCursor(page: RoomMessageRevisionPage): number | undefined {
	return page.has_more && page.next_before_revision_number !== null
		? page.next_before_revision_number
		: undefined;
}

export function roomMessageRevisionsQuery(
	companyId: string,
	roomId: string,
	messageId: number,
	enabled = true
) {
	const query = createInfiniteQuery(() => ({
		queryKey: roomQueryKeys.revisions(companyId, roomId, messageId),
		queryFn: ({ pageParam, signal }: { pageParam: number | null; signal: AbortSignal }) =>
			getRoomMessageRevisions(companyId, roomId, messageId, pageParam, 10, signal),
		initialPageParam: null as number | null,
		getNextPageParam: olderRevisionCursor,
		enabled: enabled && messageId > 0,
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get revisions() {
			return query.data?.pages.flatMap((page) => page.revisions) ?? [];
		},
		get status() {
			return enabled ? sourceStatus(query) : 'live';
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch()
	};
}

export function roomParticipantsQuery(companyId: string, roomId: string) {
	const query = createQuery(() => ({
		queryKey: roomQueryKeys.participants(companyId, roomId),
		queryFn: () => getRoomParticipants(companyId, roomId),
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get participants() {
			return query.data ?? [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		}
	};
}

export function roomReadCursorQuery(companyId: string, roomId: string) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: roomQueryKeys.readCursor(companyId, roomId),
		queryFn: () => getRoomReadCursor(companyId, roomId),
		staleTime: ROOM_STALE_MS,
		gcTime: ROOM_RETAIN_MS,
		retry: 1
	}));
	return {
		get cursor() {
			return query.data?.cursor ?? null;
		},
		get actorId() {
			return query.data?.actor_id ?? '';
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		accept(cursor: NonNullable<Awaited<ReturnType<typeof getRoomReadCursor>>['cursor']>): void {
			client.setQueryData<Awaited<ReturnType<typeof getRoomReadCursor>>>(
				roomQueryKeys.readCursor(companyId, roomId),
				(current) => ({ actor_id: current?.actor_id ?? cursor.actor_id, cursor })
			);
		}
	};
}

/**
 * One selected Room owns one recoverable SSE connection. The initial event
 * snapshot is read before the message query is revalidated, so events committed
 * during that read are replayed after the snapshot cursor rather than falling
 * into a read-then-subscribe gap.
 */
export function roomActivityStream(companyId: string, roomId: string) {
	const client = useQueryClient();
	let transport = $state<RoomTransport>('connecting');
	let stop: (() => void) | null = null;
	let retryTimer: ReturnType<typeof setTimeout> | null = null;
	let generation = 0;
	const applyLifecycleEvent = (event: RoomEvent): void => {
		if (event.message_id === null) return;
		const messageId = event.message_id;
		const removeFromSearch = (
			current: InfiniteData<RoomMessageSearchPage, number | null> | undefined
		): InfiniteData<RoomMessageSearchPage, number | null> | undefined =>
			current
				? { ...current, pages: removeRoomMessageSearchResult(current.pages, messageId) }
				: current;
		if (event.kind === 'room.message.edited.v1') {
			// The event deliberately carries no replacement body. Remove a stale
			// search hit until the authoritative search projection is revalidated.
			client.setQueriesData({ queryKey: ['room-message-search', companyId] }, removeFromSearch);
			return;
		}
		if (event.kind !== 'room.message.deleted.v1') return;
		const apply = (
			current: InfiniteData<RoomMessagePage, number | null> | undefined
		): InfiniteData<RoomMessagePage, number | null> | undefined =>
			current
				? {
						...current,
						pages: tombstoneRoomMessagePages(current.pages, messageId, event.created_at)
					}
				: current;
		client.setQueryData(roomQueryKeys.messages(companyId, roomId), apply);
		client.setQueriesData({ queryKey: ['room-thread', companyId, roomId] }, apply);
		client.setQueriesData({ queryKey: ['room-message-search', companyId] }, removeFromSearch);
		client.removeQueries({ queryKey: roomQueryKeys.revisions(companyId, roomId, messageId) });
	};

	const invalidate = async (event?: RoomEvent): Promise<void> => {
		await client.invalidateQueries({ queryKey: roomQueryKeys.messages(companyId, roomId) });
		if (event?.message_id) {
			await client.invalidateQueries({
				/* A reply event carries the reply id, not its root. Revalidate every
				 * cached Thread under this Room instead of guessing that relation. */
				queryKey: ['room-thread', companyId, roomId]
			});
		}
		if (event?.kind === 'room.message.edited.v1' || event?.kind === 'room.message.deleted.v1') {
			await client.invalidateQueries({ queryKey: ['room-message-search', companyId] });
		}
	};

	return {
		get transport() {
			return transport;
		},
		attach(): () => void {
			stop?.();
			if (retryTimer) clearTimeout(retryTimer);
			const currentGeneration = ++generation;
			transport = 'connecting';
			let latestEventId = 0;

			const scheduleRestart = (reason: RoomStreamRestartReason): void => {
				if (currentGeneration !== generation) return;
				transport = 'reconnecting';
				if (retryTimer) clearTimeout(retryTimer);
				retryTimer = setTimeout(
					() => (reason === 'resync' ? void connect() : subscribe(latestEventId)),
					reason === 'resync' ? 0 : 3_000
				);
			};

			const scheduleSnapshot = (): void => {
				if (currentGeneration !== generation) return;
				transport = 'reconnecting';
				if (retryTimer) clearTimeout(retryTimer);
				retryTimer = setTimeout(() => void connect(), 3_000);
			};

			const subscribe = (afterEventId: number): void => {
				if (currentGeneration !== generation) return;
				stop = openRoomEventStream(
					companyId,
					roomId,
					afterEventId,
					(event) => {
						latestEventId = Math.max(latestEventId, event.id);
						applyLifecycleEvent(event);
						void invalidate(event);
					},
					(next) => (transport = next),
					scheduleRestart
				);
			};

			const connect = async (): Promise<void> => {
				try {
					const snapshot = await getRoomEventSnapshot(companyId, roomId);
					if (currentGeneration !== generation) return;
					await invalidate();
					if (currentGeneration !== generation) return;
					latestEventId = snapshot.snapshot_cursor;
					subscribe(latestEventId);
				} catch {
					scheduleSnapshot();
				}
			};
			void connect();
			return () => {
				generation += 1;
				if (retryTimer) clearTimeout(retryTimer);
				retryTimer = null;
				stop?.();
				stop = null;
			};
		}
	};
}
