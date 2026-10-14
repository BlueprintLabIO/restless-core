import { responseFailure } from './failure.ts';
import { bootFetch } from './boot-fetch.ts';
export type RoomKind = 'company' | 'group' | 'direct';

export interface Room {
	id: string;
	kind: RoomKind;
	title: string;
	created_by: string;
	canonical_key: string | null;
	created_at: string;
	archived_at: string | null;
}

export interface RoomListCursor {
	beforeCreatedAt: string;
	beforeRoomId: string;
}

export interface RoomListPage {
	rooms: Room[];
	next_before_created_at: string | null;
	next_before_room_id: string | null;
	has_more: boolean;
}

export interface RecentDirectConversation {
	room_id: string;
	person_actor_id: string;
	last_message_id: number;
	last_message_at: string;
	/** Who wrote the latest message, and its opening text (bounded by the server). */
	last_message_from?: string;
	last_message_preview?: string;
	/** The viewer's own server read cursor, the same on every device. */
	last_read_message_id?: number | null;
}

/** A group room by its latest message. */
export interface RecentGroupConversation {
	room_id: string;
	title: string;
	last_message_id: number;
	last_message_at: string;
	last_message_from: string;
	last_message_preview: string;
	last_read_message_id?: number | null;
}

/**
 * Whether a conversation row has something the viewer has not read: a newer message than their
 * server read cursor, written by someone else. Your own message sent from another device is read.
 */
export function conversationUnread(
	row: {
		last_message_id: number;
		last_message_from?: string;
		last_read_message_id?: number | null;
	},
	viewerActorId: string
): boolean {
	if (!viewerActorId || row.last_message_from === viewerActorId) return false;
	return row.last_message_id > (row.last_read_message_id ?? 0);
}

export interface RoomParticipant {
	room_id: string;
	actor_id: string;
	role: 'owner' | 'member';
	joined_at: string;
	left_at: string | null;
}

export interface RoomMessage {
	id: number;
	room_id: string;
	from_actor: string;
	to_actor: string | null;
	body: string;
	outcome_standard: string | null;
	parent_message_id: number | null;
	thread_root_message_id: number | null;
	client_command_id: string | null;
	created_at: string;
	revision_number: number;
	edited_at: string | null;
	deleted_at: string | null;
	legacy_read_at: string | null;
	/** Files the author attached; absent when there are none. */
	attachments?: RoomMessageAttachment[];
}

/** A file on a Room message. Its bytes are served by the company attachment route. */
export interface RoomMessageAttachment {
	attachment_id: string;
	name: string;
	media_type: string;
	size_bytes: number;
}

export interface RoomMention {
	id: string;
	room_id: string;
	message_id: number;
	thread_root_message_id: number;
	mentioned_actor_id: string;
	kind: 'direct' | 'exec';
	work_id: string | null;
	why_this_actor: string | null;
	expected_response: string | null;
	recommendation: string | null;
	alternatives: unknown;
	evidence: unknown;
	uncertainty: string | null;
	affected_scope: string | null;
	deadline_at: string | null;
	fallback: string | null;
	independent_work_can_continue: boolean;
	created_event_id: number;
	resolution_message_id: number | null;
	resolved_event_id: number | null;
	cancelled_event_id: number | null;
	cancelled_by: string | null;
	cancellation_reason: string | null;
	created_at: string;
	resolved_at: string | null;
	cancelled_at: string | null;
}

export interface RoomThreadSummary {
	root_message_id: number;
	reply_count: number;
	last_reply_at: string;
	/** Distinct reply authors, most recent first, at most three. */
	repliers: string[];
}

export interface RoomMessagePage {
	messages: RoomMessage[];
	mentions: RoomMention[];
	next_before_message_id: number | null;
	has_more: boolean;
	/** Reply counts for roots in this page; absent on Thread pages. */
	threads?: RoomThreadSummary[];
}

export interface RoomMessageRevision {
	id: string;
	revision_number: number;
	editor_actor_id: string;
	body: string;
	created_at: string;
}

export interface RoomMessageRevisionPage {
	revisions: RoomMessageRevision[];
	next_before_revision_number: number | null;
	has_more: boolean;
}

export interface RoomMessageEditResult {
	revision: RoomMessageRevision;
	created: boolean;
}

export interface RoomMessageTombstone {
	id: string;
	room_id: string;
	message_id: number;
	deleted_by_actor_id: string;
	created_event_id: number;
	created_at: string;
}

export interface RoomMessageDeleteResult {
	tombstone: RoomMessageTombstone;
	created: boolean;
}

export interface RoomMessageSearchResult {
	id: number;
	room_id: string;
	from_actor: string;
	thread_root_message_id: number | null;
	snippet: string;
}

export interface RoomMessageSearchPage {
	messages: RoomMessageSearchResult[];
	next_before_message_id: number | null;
	has_more: boolean;
}

/**
 * Apply a body-free deletion event before any network revalidation. Once the
 * server has told this client that a Message is deleted, stale or failed
 * refetches must never resurrect its body or structured mention context.
 */
export function tombstoneRoomMessagePages(
	pages: RoomMessagePage[],
	messageId: number,
	deletedAt: string
): RoomMessagePage[] {
	return pages.map((page) => ({
		...page,
		messages: page.messages.map((message) =>
			message.id === messageId
				? { ...message, body: '', deleted_at: message.deleted_at ?? deletedAt }
				: message
		),
		mentions: page.mentions.filter((mention) => mention.message_id !== messageId)
	}));
}

export function editRoomMessagePages(
	pages: RoomMessagePage[],
	messageId: number,
	body: string,
	editedAt: string,
	revisionNumber: number
): RoomMessagePage[] {
	return pages.map((page) => ({
		...page,
		messages: page.messages.map((message) =>
			message.id === messageId && !message.deleted_at
				? { ...message, body, edited_at: editedAt, revision_number: revisionNumber }
				: message
		)
	}));
}

export function removeRoomMessageSearchResult(
	pages: RoomMessageSearchPage[],
	messageId: number
): RoomMessageSearchPage[] {
	return pages.map((page) => ({
		...page,
		messages: page.messages.filter((message) => message.id !== messageId)
	}));
}

export interface RoomReadCursor {
	room_id: string;
	actor_id: string;
	last_read_message_id: number | null;
	updated_at: string;
}

export interface RoomReadState {
	actor_id: string;
	cursor: RoomReadCursor | null;
}

export interface RoomEvent {
	id: number;
	kind: string;
	room_id: string;
	actor_id: string;
	message_id: number | null;
	created_at: string;
}

export interface RoomEventPage {
	events: RoomEvent[];
	requested_after_event_id: number;
	next_after_event_id: number;
	snapshot_cursor: number;
	compacted_through_event_id: number;
	oldest_available_event_id: number | null;
	has_more: boolean;
	resync_required: boolean;
}

export interface NewRoomMention {
	actor_id: string;
}

export interface RoomMessageSendResult {
	message: RoomMessage;
	event_id: number;
	created: boolean;
	mentions: RoomMention[];
	resolved_mention: RoomMention | null;
}

export type RoomTransport = 'connecting' | 'live' | 'reconnecting';
export type RoomStreamRestartReason = 'transport' | 'resync';

const roomPath = (company: string, room?: string): string => {
	const base = `/api/companies/${encodeURIComponent(company)}/rooms`;
	return room ? `${base}/${encodeURIComponent(room)}` : base;
};

const roomError = responseFailure;

async function roomJson<T>(url: string, init?: RequestInit): Promise<T> {
	// Only a read may take a response app.html started; a write always goes out.
	const read = !init?.method || init.method === 'GET';
	const response = await (read ? bootFetch : fetch)(url, {
		credentials: 'same-origin',
		cache: 'no-store',
		...init
	});
	if (!response.ok) throw await roomError(response);
	return (await response.json()) as T;
}

export function getRooms(
	company: string,
	cursor: RoomListCursor | null = null,
	limit = 30
): Promise<RoomListPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (cursor) {
		query.set('before_created_at', cursor.beforeCreatedAt);
		query.set('before_room_id', cursor.beforeRoomId);
	}
	return roomJson(`${roomPath(company)}?${query}`);
}

export function getRecentDirectConversations(company: string): Promise<RecentDirectConversation[]> {
	return roomJson(`/api/companies/${encodeURIComponent(company)}/direct-conversations`);
}

export function getRecentGroupConversations(company: string): Promise<RecentGroupConversation[]> {
	return roomJson(`/api/companies/${encodeURIComponent(company)}/group-conversations`);
}

/* The machine blocks a stored body carries after its visible text: attachment paths, the cockpit
 * link, Attention context and agent receipts. A preview stops at the first of them. */
const MACHINE_BLOCK = /\n\n\[(?:Restless |Owner cockpit context\])|<!--restless-/;

/** A message's opening text as one plain line: markdown links, emphasis and code marks dropped. */
export function messagePreview(body: string | undefined): string {
	const text = body ?? '';
	const machine = text.search(MACHINE_BLOCK);
	return (machine >= 0 ? text.slice(0, machine) : text)
		.replace(/!\[[^\]]*\]\([^)]*\)/g, '')
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/[*_`>#~]+/g, '')
		.replace(/\s+/g, ' ')
		.trim();
}

export function searchRooms(
	company: string,
	search: string,
	cursor: RoomListCursor | null = null,
	limit = 30,
	signal?: AbortSignal
): Promise<RoomListPage> {
	const query = new URLSearchParams({ q: search, limit: String(limit) });
	if (cursor) {
		query.set('before_created_at', cursor.beforeCreatedAt);
		query.set('before_room_id', cursor.beforeRoomId);
	}
	return roomJson(`${roomPath(company)}/search?${query}`, { signal });
}

export function searchRoomMessages(
	company: string,
	search: string,
	beforeMessageId: number | null = null,
	limit = 20,
	signal?: AbortSignal,
	roomId = ''
): Promise<RoomMessageSearchPage> {
	const query = new URLSearchParams({ q: search, limit: String(limit) });
	if (roomId) query.set('room_id', roomId);
	if (beforeMessageId !== null) query.set('before_message_id', String(beforeMessageId));
	return roomJson(`/api/companies/${encodeURIComponent(company)}/room-messages/search?${query}`, {
		signal
	});
}

export async function getRoomParticipants(
	company: string,
	room: string
): Promise<RoomParticipant[]> {
	const result = await roomJson<{ participants: RoomParticipant[] }>(
		`${roomPath(company, room)}/participants`
	);
	return result.participants;
}

export function getRoomMessages(
	company: string,
	room: string,
	beforeMessageId: number | null = null,
	limit = 50
): Promise<RoomMessagePage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (beforeMessageId !== null) query.set('before_message_id', String(beforeMessageId));
	return roomJson(`${roomPath(company, room)}/messages?${query}`);
}

export function getRoomThread(
	company: string,
	room: string,
	rootMessageId: number,
	beforeMessageId: number | null = null,
	limit = 50
): Promise<RoomMessagePage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (beforeMessageId !== null) query.set('before_message_id', String(beforeMessageId));
	return roomJson(
		`${roomPath(company, room)}/threads/${encodeURIComponent(rootMessageId)}?${query}`
	);
}

export function getRoomMessageRevisions(
	company: string,
	room: string,
	messageId: number,
	beforeRevisionNumber: number | null = null,
	limit = 10,
	signal?: AbortSignal
): Promise<RoomMessageRevisionPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (beforeRevisionNumber !== null) {
		query.set('before_revision_number', String(beforeRevisionNumber));
	}
	return roomJson(
		`${roomPath(company, room)}/messages/${encodeURIComponent(messageId)}/revisions?${query}`,
		{ signal }
	);
}

export function editRoomMessage(
	company: string,
	room: string,
	messageId: number,
	body: string,
	expectedRevisionNumber: number,
	commandId: string,
	/** Everyone the new body asks; the server asks only those the message had not asked. */
	mentions: NewRoomMention[] = []
): Promise<RoomMessageEditResult> {
	return roomJson(`${roomPath(company, room)}/messages/${encodeURIComponent(messageId)}`, {
		method: 'PATCH',
		headers: {
			'content-type': 'application/json',
			'idempotency-key': commandId
		},
		body: JSON.stringify({
			body,
			expected_revision_number: expectedRevisionNumber,
			...(mentions.length ? { mentions } : {})
		})
	});
}

/* A mention travels as `@actor-id`, but people write and read names. While composing, a person
 * whose name is unique among those who could be named reads as `@Their Name`; sending turns it
 * back into the id. A shared name keeps its id, so nobody is asked by mistake. */
export interface MentionPerson {
	actor_id: string;
	display: string;
}
const MENTION_END = '(?=$|[\\s.,!?;:)])';
function escapeRegExp(text: string): string {
	return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
function uniquelyNamed(people: readonly MentionPerson[]): MentionPerson[] {
	const counts = new Map<string, number>();
	for (const person of people) {
		const key = person.display.trim().toLowerCase();
		counts.set(key, (counts.get(key) ?? 0) + 1);
	}
	return people.filter(
		(person) =>
			person.display.trim() &&
			person.display.trim() !== person.actor_id &&
			counts.get(person.display.trim().toLowerCase()) === 1
	);
}
/** The name a mention shows while composing: `@Name` when unique, else `@actor-id`. */
export function mentionInsert(person: MentionPerson, people: readonly MentionPerson[]): string {
	return uniquelyNamed(people).some((candidate) => candidate.actor_id === person.actor_id)
		? `@${person.display.trim()} `
		: `@${person.actor_id} `;
}
/** `@actor-id` as `@Name`, for a body put back into an editor. */
export function readableMentions(body: string, people: readonly MentionPerson[]): string {
	const named = new Map(uniquelyNamed(people).map((person) => [person.actor_id, person.display]));
	return body.replace(/(^|[\s(])@([\w-]+(?:[.:][\w-]+)*)/g, (whole, lead: string, id: string) => {
		const name = named.get(id);
		return name ? `${lead}@${name.trim()}` : whole;
	});
}
/** `@Name` (or a unique first name) back to `@actor-id`, as the server routes on. */
export function resolveReadableMentions(body: string, people: readonly MentionPerson[]): string {
	const named = uniquelyNamed(people).toSorted((a, b) => b.display.length - a.display.length);
	let resolved = body;
	for (const person of named) {
		const pattern = new RegExp(
			`(^|[\\s(])@${escapeRegExp(person.display.trim())}${MENTION_END}`,
			'giu'
		);
		resolved = resolved.replace(pattern, (_whole, lead: string) => `${lead}@${person.actor_id}`);
	}
	/* "@Ines" names Ines Garcia when no one else could be meant. */
	const firstNames = new Map<string, MentionPerson[]>();
	for (const person of people) {
		const first = person.display.trim().split(/\s+/)[0]?.toLowerCase();
		if (first) firstNames.set(first, [...(firstNames.get(first) ?? []), person]);
	}
	const ids = new Set(people.map((person) => person.actor_id.toLowerCase()));
	for (const [first, candidates] of firstNames) {
		if (candidates.length !== 1 || ids.has(first)) continue;
		const pattern = new RegExp(`(^|[\\s(])@${escapeRegExp(first)}${MENTION_END}`, 'giu');
		resolved = resolved.replace(
			pattern,
			(_whole, lead: string) => `${lead}@${candidates[0].actor_id}`
		);
	}
	return resolved;
}

export function deleteRoomMessage(
	company: string,
	room: string,
	messageId: number,
	commandId: string
): Promise<RoomMessageDeleteResult> {
	return roomJson(`${roomPath(company, room)}/messages/${encodeURIComponent(messageId)}`, {
		method: 'DELETE',
		headers: { 'idempotency-key': commandId }
	});
}

export async function getRoomReadCursor(company: string, room: string): Promise<RoomReadState> {
	return roomJson(`${roomPath(company, room)}/read-cursor`);
}

export function markRoomRead(
	company: string,
	room: string,
	throughMessageId: number
): Promise<RoomReadCursor> {
	return roomJson(`${roomPath(company, room)}/read-cursor`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ through_message_id: throughMessageId })
	});
}

export function sendRoomMessage(
	company: string,
	room: string,
	body: string,
	commandId: string,
	parentMessageId: number | null,
	mentions: NewRoomMention[] = [],
	/** A mention of the sender this Thread reply answers. */
	resolvesMentionId: string | null = null,
	/** Files the sender already uploaded and kept. */
	attachmentIds: readonly string[] = []
): Promise<RoomMessageSendResult> {
	const endpoint = parentMessageId
		? `${roomPath(company, room)}/messages/${encodeURIComponent(parentMessageId)}/replies`
		: `${roomPath(company, room)}/messages`;
	return roomJson(endpoint, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			body,
			command_id: commandId,
			mentions,
			...(resolvesMentionId ? { resolves_mention_id: resolvesMentionId } : {}),
			...(attachmentIds.length ? { attachment_ids: attachmentIds } : {})
		})
	});
}

/** Say this person is typing in the Room; the others see it for a few seconds. */
export function sendRoomTyping(company: string, room: string): void {
	void fetch(`${roomPath(company, room)}/typing`, {
		method: 'POST',
		credentials: 'same-origin'
	}).catch(() => undefined);
}

export function getRoomEventSnapshot(company: string, room: string): Promise<RoomEventPage> {
	return roomJson(`${roomPath(company, room)}/events?after_event_id=0&limit=1`);
}

/** Events are body-free hints. Callers refetch the exact Room projection. */
export function openRoomEventStream(
	company: string,
	room: string,
	afterEventId: number,
	onevent: (event: RoomEvent) => void,
	ontransport: (transport: RoomTransport) => void,
	onrestart: (reason: RoomStreamRestartReason) => void,
	ontyping?: (actorId: string) => void
): () => void {
	ontransport('connecting');
	const source = new EventSource(
		`${roomPath(company, room)}/events/live?after_event_id=${encodeURIComponent(afterEventId)}`
	);
	let retired = false;
	const retire = (reason: RoomStreamRestartReason) => {
		if (retired) return;
		retired = true;
		source.close();
		ontransport('reconnecting');
		onrestart(reason);
	};
	source.onopen = () => {
		if (!retired) ontransport('live');
	};
	source.onerror = () => retire('transport');
	source.addEventListener('room-event', ((event: MessageEvent<string>) => {
		if (retired) return;
		try {
			onevent(JSON.parse(event.data) as RoomEvent);
		} catch {
			retire('transport');
		}
	}) as EventListener);
	/* Someone else typing in the Room: a passing signal, never an event to replay. */
	source.addEventListener('typing', ((event: MessageEvent<string>) => {
		if (retired || !ontyping) return;
		try {
			const actorId = (JSON.parse(event.data) as { actor_id?: unknown }).actor_id;
			if (typeof actorId === 'string') ontyping(actorId);
		} catch {
			/* A malformed signal is simply not shown. */
		}
	}) as EventListener);
	/* A compacted cursor is an explicit projection reset, not an ordinary
	 * network retry. The owner server sends this named event and closes. */
	source.addEventListener('resync', (() => retire('resync')) as EventListener);
	return () => {
		retired = true;
		source.close();
	};
}

export interface StoredRoomDraft {
	body: string;
	commandId: string | null;
	updatedAt: string;
}

export function roomDraftKey(
	company: string,
	actor: string,
	room: string,
	threadRootId: number | null
): string {
	const scope = [company, actor, room, threadRootId ?? 'room']
		.map((part) => encodeURIComponent(String(part)))
		.join(':');
	return `restless:room-draft:v2:${scope}`;
}

export function readRoomDraft(key: string): StoredRoomDraft {
	if (typeof localStorage === 'undefined') return { body: '', commandId: null, updatedAt: '' };
	try {
		const value = JSON.parse(
			localStorage.getItem(key) ?? 'null'
		) as Partial<StoredRoomDraft> | null;
		return {
			body: typeof value?.body === 'string' ? value.body : '',
			commandId: typeof value?.commandId === 'string' ? value.commandId : null,
			updatedAt: typeof value?.updatedAt === 'string' ? value.updatedAt : ''
		};
	} catch {
		return { body: '', commandId: null, updatedAt: '' };
	}
}

export function writeRoomDraft(key: string, draft: StoredRoomDraft): void {
	if (typeof localStorage === 'undefined') return;
	try {
		if (!draft.body && !draft.commandId) {
			localStorage.removeItem(key);
			return;
		}
		localStorage.setItem(key, JSON.stringify(draft));
	} catch {
		// Storage can be unavailable or full. The in-memory draft remains usable.
	}
}

export function createRoom(
	company: string,
	input: { kind: RoomKind; title: string; participant_actor_ids: string[]; command_id: string }
): Promise<Room> {
	return roomJson(roomPath(company), {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(input)
	});
}

export function addRoomParticipant(
	company: string,
	room: string,
	actor: string
): Promise<RoomParticipant> {
	return roomJson(`${roomPath(company, room)}/participants`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ actor_id: actor })
	});
}

export function removeRoomParticipant(
	company: string,
	room: string,
	actor: string
): Promise<RoomParticipant> {
	return roomJson(`${roomPath(company, room)}/participants/${encodeURIComponent(actor)}`, {
		method: 'DELETE'
	});
}

/** The Room owner hands a group to another person in it, and stays as a member. */
export function handOverRoom(
	company: string,
	room: string,
	actor: string
): Promise<RoomParticipant> {
	return roomJson(`${roomPath(company, room)}/owner`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ actor_id: actor })
	});
}

/** The Room owner renames a group; everyone sees the new name. */
export function renameRoom(company: string, room: string, title: string): Promise<Room> {
	return roomJson(roomPath(company, room), {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ title })
	});
}

/** The Room owner closes a group: it leaves every list, and its messages are kept. */
export function archiveRoom(company: string, room: string): Promise<Room> {
	return roomJson(`${roomPath(company, room)}/archive`, { method: 'POST' });
}

/** The archived groups the viewer is still in, to read; only those they own can they bring back. */
export async function getArchivedRooms(
	company: string
): Promise<{ rooms: Room[]; ownedRoomIds: string[] }> {
	const body = await roomJson<{ rooms: Room[]; owned_room_ids?: string[] }>(
		`${roomPath(company)}/archived`
	);
	return { rooms: body.rooms, ownedRoomIds: body.owned_room_ids ?? [] };
}

/** Bring an archived group back for everyone still in it. */
export function restoreRoom(company: string, room: string): Promise<Room> {
	return roomJson(`${roomPath(company, room)}/restore`, { method: 'POST' });
}
