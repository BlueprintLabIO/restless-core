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
	legacy_read_at: string | null;
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

export interface RoomMessagePage {
	messages: RoomMessage[];
	mentions: RoomMention[];
	next_before_message_id: number | null;
	has_more: boolean;
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

async function roomError(response: Response): Promise<Error & { status: number }> {
	let message = `${response.status} ${response.statusText}`;
	try {
		const body = (await response.json()) as { message?: string };
		message = body.message ?? message;
	} catch {
		// An intermediary may return plain text. Keep the transport status.
	}
	return Object.assign(new Error(message), { status: response.status });
}

async function roomJson<T>(url: string, init?: RequestInit): Promise<T> {
	const response = await fetch(url, {
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
	mentions: NewRoomMention[] = []
): Promise<RoomMessageSendResult> {
	const endpoint = parentMessageId
		? `${roomPath(company, room)}/messages/${encodeURIComponent(parentMessageId)}/replies`
		: `${roomPath(company, room)}/messages`;
	return roomJson(endpoint, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ body, command_id: commandId, mentions })
	});
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
	onrestart: (reason: RoomStreamRestartReason) => void
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
