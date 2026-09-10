import type { RoomMessage, RoomMessagePage } from './rooms';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const POSITIVE_DECIMAL = /^[1-9][0-9]*$/;

/**
 * A notification return path is intentionally more specific than ordinary
 * Room navigation. Keeping the four source-owned coordinates together lets
 * the client reject partial or ambiguous links before it reads Room history.
 */
export interface RoomMessageTarget {
	roomId: string;
	threadRootMessageId: number;
	messageId: number;
	mentionId: string;
}

export type RoomMessageTargetParseResult =
	{ kind: 'none' } | { kind: 'invalid' } | { kind: 'target'; target: RoomMessageTarget };

export type RoomMessageTargetUnavailableReason =
	'deleted' | 'missing' | 'coordinate-mismatch' | 'page-limit' | 'paging-unavailable';

export type RoomMessageTargetLocation =
	| { state: 'found'; message: RoomMessage }
	| { state: 'unavailable'; reason: RoomMessageTargetUnavailableReason };

export const ROOM_MESSAGE_TARGET_MAX_PAGE_LOADS = 20;

function canonicalUuid(value: string | null): string | null {
	return value !== null && UUID.test(value) ? value.toLowerCase() : null;
}

export function parsePositiveRoomMessageId(value: string | null): number | null {
	if (value === null || !POSITIVE_DECIMAL.test(value)) return null;
	const parsed = Number(value);
	return Number.isSafeInteger(parsed) ? parsed : null;
}

/**
 * `message` or `mention` opts into exact-target handling. Plain Room and
 * Thread URLs predate notification deep links and remain ordinary navigation.
 */
export function parseRoomMessageTarget(
	searchParams: URLSearchParams
): RoomMessageTargetParseResult {
	if (!searchParams.has('message') && !searchParams.has('mention')) return { kind: 'none' };

	const fields = ['room', 'thread', 'message', 'mention'] as const;
	if (fields.some((field) => searchParams.getAll(field).length !== 1)) {
		return { kind: 'invalid' };
	}

	const roomId = canonicalUuid(searchParams.get('room'));
	const threadRootMessageId = parsePositiveRoomMessageId(searchParams.get('thread'));
	const messageId = parsePositiveRoomMessageId(searchParams.get('message'));
	const mentionId = canonicalUuid(searchParams.get('mention'));
	if (
		roomId === null ||
		threadRootMessageId === null ||
		messageId === null ||
		mentionId === null ||
		messageId < threadRootMessageId
	) {
		return { kind: 'invalid' };
	}

	return {
		kind: 'target',
		target: { roomId, threadRootMessageId, messageId, mentionId }
	};
}

export interface RoomMessageTargetPageSource {
	pages(): readonly RoomMessagePage[];
	hasMore(): boolean;
	loadMore(): Promise<void>;
}

function targetInPages(
	pages: readonly RoomMessagePage[],
	target: RoomMessageTarget
): RoomMessageTargetLocation | null {
	const message = pages
		.flatMap((page) => page.messages)
		.find((candidate) => candidate.id === target.messageId);
	if (!message) return null;

	const canonicalThreadId = message.thread_root_message_id ?? message.id;
	if (
		message.room_id.toLowerCase() !== target.roomId ||
		canonicalThreadId !== target.threadRootMessageId
	) {
		return { state: 'unavailable', reason: 'coordinate-mismatch' };
	}
	if (message.deleted_at !== null) return { state: 'unavailable', reason: 'deleted' };

	const mention = pages
		.flatMap((page) => page.mentions)
		.find((candidate) => candidate.id.toLowerCase() === target.mentionId);
	if (
		!mention ||
		mention.room_id.toLowerCase() !== target.roomId ||
		mention.message_id !== target.messageId ||
		mention.thread_root_message_id !== target.threadRootMessageId
	) {
		return { state: 'unavailable', reason: 'coordinate-mismatch' };
	}

	return { state: 'found', message };
}

/**
 * Walk older Thread pages through the API's immutable Message-id cursor. The
 * hard page ceiling and repeated-cursor checks turn a stale or broken link into
 * an explicit result instead of an unbounded fetch loop.
 */
export async function locateRoomMessageTarget(
	target: RoomMessageTarget,
	source: RoomMessageTargetPageSource,
	maxPageLoads = ROOM_MESSAGE_TARGET_MAX_PAGE_LOADS
): Promise<RoomMessageTargetLocation> {
	const requestedPageLimit = Number.isFinite(maxPageLoads) ? Math.floor(maxPageLoads) : 0;
	const pageLimit = Math.min(ROOM_MESSAGE_TARGET_MAX_PAGE_LOADS, Math.max(0, requestedPageLimit));
	const seenCursors = new Set<number>();
	let loadedPages = 0;

	while (true) {
		const pages = source.pages();
		const located = targetInPages(pages, target);
		if (located) return located;

		if (!source.hasMore()) return { state: 'unavailable', reason: 'missing' };
		const cursor = pages.at(-1)?.next_before_message_id ?? null;
		if (
			cursor === null ||
			!Number.isSafeInteger(cursor) ||
			cursor <= 0 ||
			seenCursors.has(cursor)
		) {
			return { state: 'unavailable', reason: 'paging-unavailable' };
		}

		/* The current page already spans every reply at or above its oldest
		 * cursor. A missing target in that range cannot appear on an older page. */
		if (target.messageId >= cursor) return { state: 'unavailable', reason: 'missing' };
		if (loadedPages >= pageLimit) return { state: 'unavailable', reason: 'page-limit' };

		seenCursors.add(cursor);
		const previousPageCount = pages.length;
		try {
			await source.loadMore();
		} catch {
			return { state: 'unavailable', reason: 'paging-unavailable' };
		}
		loadedPages += 1;
		if (source.pages().length <= previousPageCount) {
			return { state: 'unavailable', reason: 'paging-unavailable' };
		}
	}
}
