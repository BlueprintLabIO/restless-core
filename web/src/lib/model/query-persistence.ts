import type { QueryClient } from '@tanstack/query-core';

export const QUERY_CACHE_SCHEMA = 2;
export const QUERY_CACHE_MAX_AGE_MS = 12 * 60 * 60_000;
export const QUERY_CACHE_MAX_ENTRIES = 12;
export const QUERY_CACHE_MAX_ENTRY_BYTES = 128 * 1024;
export const QUERY_CACHE_MAX_TOTAL_BYTES = 512 * 1024;
export const QUERY_CACHE_MAX_ROOM_MESSAGES = 50;
export const QUERY_CACHE_MAX_ROOM_MENTIONS = 50;
export const QUERY_CACHE_MAX_ATTENTION_ITEMS = 40;

const DATABASE_NAME = 'restless-safe-query-cache';
const DATABASE_VERSION = 1;
const STORE_NAME = 'partitions';

type QueryKey = readonly unknown[];

export interface CompanyPrincipal {
	actor_id: string;
	membership_role: string;
	cache_partition: string;
}

export interface QuerySnapshot {
	queryKey: QueryKey;
	data: unknown;
	dataUpdatedAt: number;
}

export interface PersistedQueryEntry {
	queryKey: unknown[];
	data: unknown;
	dataUpdatedAt: number;
}

export interface PersistedQueryEnvelope {
	id: string;
	company: string;
	cachePartition: string;
	schema: number;
	savedAt: number;
	entries: PersistedQueryEntry[];
}

export interface SafeQueryStore {
	load(id: string): Promise<unknown>;
	save(envelope: PersistedQueryEnvelope): Promise<void>;
	remove(id: string): Promise<void>;
	clearCompany(company: string, keepId?: string): Promise<void>;
}

function record(value: unknown): Record<string, unknown> | null {
	return value !== null && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null;
}

function string(value: unknown, max = 16_384): string | null {
	return typeof value === 'string' && value.length <= max ? value : null;
}

function nullableString(value: unknown, max = 16_384): string | null {
	return value === null || value === undefined ? null : string(value, max);
}

function finiteNumber(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function integer(value: unknown): number | null {
	const number = finiteNumber(value);
	return number !== null && Number.isInteger(number) ? number : null;
}

function nullableInteger(value: unknown): number | null | undefined {
	return value === null || value === undefined ? null : (integer(value) ?? undefined);
}

function byteLength(value: unknown): number {
	try {
		return new TextEncoder().encode(JSON.stringify(value)).byteLength;
	} catch {
		return Number.POSITIVE_INFINITY;
	}
}

export function queryCacheRecordId(company: string, cachePartition: string): string {
	return [QUERY_CACHE_SCHEMA, company, cachePartition]
		.map((part) => encodeURIComponent(String(part)))
		.join(':');
}

export function companyForPersistableQuery(queryKey: QueryKey): string | null {
	if (
		queryKey.length === 2 &&
		queryKey[0] === 'attention' &&
		typeof queryKey[1] === 'string' &&
		queryKey[1]
	) {
		return queryKey[1];
	}
	if (
		queryKey.length === 3 &&
		queryKey[0] === 'room-messages' &&
		typeof queryKey[1] === 'string' &&
		queryKey[1] &&
		typeof queryKey[2] === 'string' &&
		queryKey[2]
	) {
		return queryKey[1];
	}
	return null;
}

function safeActor(value: unknown): Record<string, string> | undefined {
	const actor = record(value);
	if (!actor) return undefined;
	const id = string(actor.id, 256);
	const display = string(actor.display, 512);
	const role = string(actor.role, 512);
	return id && display && role ? { id, display, role } : undefined;
}

function safeAttentionItem(value: unknown): Record<string, unknown> | null {
	const item = record(value);
	const source = record(item?.source);
	/* Attention can aggregate several planes. Only the OrgIntel-owned human
	 * summary is an offline-safe projection; Authority, Runtime, browser and
	 * any future plane stay live-only until they define their own contract. */
	if (!item || !source || source.plane !== 'orgintel') return null;
	const id = string(item.id, 512);
	const plane = string(source.plane, 128);
	const kind = string(source.kind, 256);
	const reference = string(source.reference, 1_024);
	const category = string(item.category, 128);
	const title = string(item.title, 2_048);
	const whatHappened = string(item.whatHappened, 8_192);
	const whyItMatters = string(item.whyItMatters, 8_192);
	const recommendation = string(item.recommendation, 8_192);
	const requestedAction = string(item.requestedAction, 8_192);
	const ifNoAction = string(item.ifNoAction, 8_192);
	const briefStatus = string(item.briefStatus, 128);
	const createdAt = string(item.createdAt, 128);
	const workId = string(item.workId, 512);
	const uncertainty = string(item.uncertainty, 2_048);
	const deadline = string(item.deadline, 128);
	const briefedAt = string(item.briefedAt, 128);
	if (
		!id ||
		!plane ||
		!kind ||
		!reference ||
		!category ||
		!title ||
		whatHappened === null ||
		whyItMatters === null ||
		recommendation === null ||
		requestedAction === null ||
		ifNoAction === null ||
		!briefStatus ||
		!createdAt
	) {
		return null;
	}
	const briefAuthor = safeActor(item.briefAuthor);
	const responsibleActor = safeActor(item.responsibleActor);
	return {
		id,
		...(workId !== null ? { workId } : {}),
		source: { plane, kind, reference },
		category,
		title,
		whatHappened,
		whyItMatters,
		recommendation,
		requestedAction,
		ifNoAction,
		...(uncertainty !== null ? { uncertainty } : {}),
		...(deadline !== null ? { deadline } : {}),
		briefStatus,
		...(briefAuthor ? { briefAuthor } : {}),
		...(briefedAt !== null ? { briefedAt } : {}),
		evidence: [],
		reviewSources: [],
		...(responsibleActor ? { responsibleActor } : {}),
		/* A cached summary is never actionable. Even an apparently harmless
		 * action id can reconstruct a live mutation in the cockpit. Current
		 * server state restores controls only after active revalidation. */
		actions: [],
		canContinue: item.canContinue === true,
		createdAt
	};
}

function safeAttention(value: unknown, company: string): unknown | null {
	const view = record(value);
	const sourceCompany = record(view?.company);
	if (!view || !sourceCompany || sourceCompany.id !== company) return null;
	const name = string(sourceCompany.name, 512);
	const mission = string(sourceCompany.mission, 8_192);
	const model = string(sourceCompany.model, 512);
	const refreshedAt = string(view.refreshedAt, 128);
	if (!name || mission === null || model === null || !refreshedAt) return null;
	return {
		company: { id: company, name, mission, model },
		/* Runtime, browser and Authority observations are deliberately never
		 * written to disk. A live revalidation repopulates those sources. */
		sourceHealth: {
			orgintel: string(record(view.sourceHealth)?.orgintel, 128) ?? 'unknown',
			authority: 'unknown',
			runtime: 'unknown',
			browser: 'unknown'
		},
		workGraph: null,
		items: (Array.isArray(view.items) ? view.items : [])
			.slice(0, QUERY_CACHE_MAX_ATTENTION_ITEMS)
			.map(safeAttentionItem)
			.filter((item): item is Record<string, unknown> => item !== null),
		/* Decision history and possible Authority continuations are not part of
		 * the personal Attention read cache. */
		continuations: [],
		refreshedAt
	};
}

function safeRoomMessage(value: unknown, room: string): Record<string, unknown> | null {
	const message = record(value);
	if (!message || message.room_id !== room) return null;
	const id = integer(message.id);
	const fromActor = string(message.from_actor, 256);
	const body = string(message.body, 16_384);
	const createdAt = string(message.created_at, 128);
	const revisionNumber = integer(message.revision_number);
	const editedAt = nullableString(message.edited_at, 128);
	const deletedAt = nullableString(message.deleted_at, 128);
	const parentMessageId = nullableInteger(message.parent_message_id);
	const threadRootMessageId = nullableInteger(message.thread_root_message_id);
	if (
		id === null ||
		!fromActor ||
		body === null ||
		!createdAt ||
		revisionNumber === null ||
		revisionNumber < 0 ||
		parentMessageId === undefined ||
		threadRootMessageId === undefined
	)
		return null;
	return {
		id,
		room_id: room,
		from_actor: fromActor,
		to_actor: nullableString(message.to_actor, 256),
		body,
		outcome_standard: nullableString(message.outcome_standard, 128),
		parent_message_id: parentMessageId,
		thread_root_message_id: threadRootMessageId,
		client_command_id: null,
		created_at: createdAt,
		revision_number: revisionNumber,
		edited_at: editedAt,
		deleted_at: deletedAt,
		legacy_read_at: null
	};
}

function safeRoomMention(value: unknown, room: string, messageIds: Set<number>) {
	const mention = record(value);
	if (!mention || mention.room_id !== room) return null;
	const id = string(mention.id, 512);
	const messageId = integer(mention.message_id);
	const rootId = integer(mention.thread_root_message_id);
	const actorId = string(mention.mentioned_actor_id, 256);
	const kind = mention.kind === 'direct' || mention.kind === 'exec' ? mention.kind : null;
	const createdAt = string(mention.created_at, 128);
	const createdEventId = integer(mention.created_event_id);
	const resolutionMessageId = nullableInteger(mention.resolution_message_id);
	const resolvedEventId = nullableInteger(mention.resolved_event_id);
	const cancelledEventId = nullableInteger(mention.cancelled_event_id);
	if (
		!id ||
		messageId === null ||
		!messageIds.has(messageId) ||
		rootId === null ||
		!actorId ||
		!kind ||
		!createdAt ||
		createdEventId === null ||
		resolutionMessageId === undefined ||
		resolvedEventId === undefined ||
		cancelledEventId === undefined
	) {
		return null;
	}
	return {
		id,
		room_id: room,
		message_id: messageId,
		thread_root_message_id: rootId,
		mentioned_actor_id: actorId,
		kind,
		work_id: nullableString(mention.work_id, 512),
		why_this_actor: nullableString(mention.why_this_actor, 2_048),
		expected_response: nullableString(mention.expected_response, 2_048),
		recommendation: nullableString(mention.recommendation, 2_048),
		alternatives: null,
		evidence: null,
		uncertainty: nullableString(mention.uncertainty, 2_048),
		affected_scope: nullableString(mention.affected_scope, 2_048),
		deadline_at: nullableString(mention.deadline_at, 128),
		fallback: nullableString(mention.fallback, 2_048),
		independent_work_can_continue: mention.independent_work_can_continue === true,
		created_event_id: createdEventId,
		resolution_message_id: resolutionMessageId,
		resolved_event_id: resolvedEventId,
		cancelled_event_id: cancelledEventId,
		cancelled_by: nullableString(mention.cancelled_by, 256),
		cancellation_reason: nullableString(mention.cancellation_reason, 2_048),
		resolved_at: nullableString(mention.resolved_at, 128),
		cancelled_at: nullableString(mention.cancelled_at, 128),
		created_at: createdAt
	};
}

function safeRoomMessages(value: unknown, room: string): unknown | null {
	const infinite = record(value);
	const pages = Array.isArray(infinite?.pages) ? infinite.pages : [];
	const latest = record(pages[0]);
	if (!latest || !Array.isArray(latest.messages)) return null;
	const safeMessages = latest.messages
		.slice(-QUERY_CACHE_MAX_ROOM_MESSAGES)
		.map((message) => safeRoomMessage(message, room))
		.filter((message): message is Record<string, unknown> => message !== null);
	const messageIds = new Set(safeMessages.map((message) => message.id as number));
	const safeMentions = (Array.isArray(latest.mentions) ? latest.mentions : [])
		.slice(-QUERY_CACHE_MAX_ROOM_MENTIONS)
		.map((mention) => safeRoomMention(mention, room, messageIds))
		.filter((mention) => mention !== null);
	const nextBeforeMessageId = nullableInteger(latest.next_before_message_id);
	if (nextBeforeMessageId === undefined) return null;
	const wasTruncated = latest.messages.length > QUERY_CACHE_MAX_ROOM_MESSAGES;
	const retainedOldestId = safeMessages.reduce(
		(oldest, message) => Math.min(oldest, message.id as number),
		Number.POSITIVE_INFINITY
	);
	return {
		pages: [
			{
				messages: safeMessages,
				mentions: safeMentions,
				next_before_message_id:
					wasTruncated && Number.isFinite(retainedOldestId)
						? retainedOldestId
						: nextBeforeMessageId,
				has_more: latest.has_more === true || pages.length > 1 || wasTruncated
			}
		],
		pageParams: [null]
	};
}

export function projectQueryForPersistence(
	queryKey: QueryKey,
	data: unknown,
	company: string
): unknown | null {
	if (companyForPersistableQuery(queryKey) !== company) return null;
	if (queryKey[0] === 'attention') return safeAttention(data, company);
	if (queryKey[0] === 'room-messages') return safeRoomMessages(data, String(queryKey[2]));
	return null;
}

export function buildPersistedEnvelope(
	company: string,
	cachePartition: string,
	snapshots: QuerySnapshot[],
	now = Date.now()
): PersistedQueryEnvelope {
	const entries = snapshots
		.flatMap((snapshot): PersistedQueryEntry[] => {
			if (
				!Number.isFinite(snapshot.dataUpdatedAt) ||
				snapshot.dataUpdatedAt <= 0 ||
				snapshot.dataUpdatedAt > now + 60_000 ||
				now - snapshot.dataUpdatedAt > QUERY_CACHE_MAX_AGE_MS
			) {
				return [];
			}
			const data = projectQueryForPersistence(snapshot.queryKey, snapshot.data, company);
			if (data === null) return [];
			const entry = {
				queryKey: [...snapshot.queryKey],
				data,
				dataUpdatedAt: snapshot.dataUpdatedAt
			};
			return byteLength(entry) <= QUERY_CACHE_MAX_ENTRY_BYTES ? [entry] : [];
		})
		.sort((left, right) => right.dataUpdatedAt - left.dataUpdatedAt);
	const bounded: PersistedQueryEntry[] = [];
	let bytes = 0;
	for (const entry of entries) {
		if (bounded.length >= QUERY_CACHE_MAX_ENTRIES) break;
		const size = byteLength(entry);
		if (bytes + size > QUERY_CACHE_MAX_TOTAL_BYTES) continue;
		bounded.push(entry);
		bytes += size;
	}
	const envelope: PersistedQueryEnvelope = {
		id: queryCacheRecordId(company, cachePartition),
		company,
		cachePartition,
		schema: QUERY_CACHE_SCHEMA,
		savedAt: now,
		entries: bounded
	};
	while (envelope.entries.length && byteLength(envelope) > QUERY_CACHE_MAX_TOTAL_BYTES) {
		envelope.entries.pop();
	}
	return envelope;
}

export function parsePersistedEnvelope(
	value: unknown,
	company: string,
	cachePartition: string,
	now = Date.now()
): PersistedQueryEnvelope | null {
	const envelope = record(value);
	if (
		!envelope ||
		envelope.id !== queryCacheRecordId(company, cachePartition) ||
		envelope.company !== company ||
		envelope.cachePartition !== cachePartition ||
		envelope.schema !== QUERY_CACHE_SCHEMA ||
		!Array.isArray(envelope.entries) ||
		envelope.entries.length > QUERY_CACHE_MAX_ENTRIES ||
		finiteNumber(envelope.savedAt) === null ||
		now - Number(envelope.savedAt) > QUERY_CACHE_MAX_AGE_MS ||
		Number(envelope.savedAt) > now + 60_000
	) {
		return null;
	}
	const snapshots: QuerySnapshot[] = [];
	for (const value of envelope.entries) {
		const entry = record(value);
		if (!entry || !Array.isArray(entry.queryKey)) return null;
		const dataUpdatedAt = finiteNumber(entry.dataUpdatedAt);
		if (
			dataUpdatedAt === null ||
			dataUpdatedAt > now + 60_000 ||
			now - dataUpdatedAt > QUERY_CACHE_MAX_AGE_MS
		)
			return null;
		snapshots.push({ queryKey: entry.queryKey, data: entry.data, dataUpdatedAt });
	}
	const rebuilt = buildPersistedEnvelope(company, cachePartition, snapshots, now);
	rebuilt.savedAt = Number(envelope.savedAt);
	if (
		rebuilt.entries.length !== snapshots.length ||
		byteLength(rebuilt) > QUERY_CACHE_MAX_TOTAL_BYTES
	) {
		return null;
	}
	return rebuilt;
}

export function shouldHydratePersistedEntry(
	currentUpdatedAt: number | undefined,
	persistedUpdatedAt: number
): boolean {
	return !currentUpdatedAt || currentUpdatedAt < persistedUpdatedAt;
}

export async function getCompanyPrincipal(
	company: string,
	signal?: AbortSignal
): Promise<CompanyPrincipal> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/principal`, {
		credentials: 'same-origin',
		cache: 'no-store',
		signal
	});
	if (!response.ok) {
		throw Object.assign(new Error(`${response.status} ${response.statusText}`), {
			status: response.status
		});
	}
	const value = record(await response.json());
	const actorId = string(value?.actor_id, 256);
	const membershipRole = string(value?.membership_role, 128);
	const cachePartition = string(value?.cache_partition, 512);
	if (!actorId || !membershipRole || !cachePartition) {
		throw Object.assign(new Error('The company principal response is invalid.'), {
			code: 'invalid_principal'
		});
	}
	return { actor_id: actorId, membership_role: membershipRole, cache_partition: cachePartition };
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
	return new Promise((resolve, reject) => {
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed.'));
	});
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
	return new Promise((resolve, reject) => {
		transaction.oncomplete = () => resolve();
		transaction.onabort = () =>
			reject(transaction.error ?? new Error('IndexedDB transaction aborted.'));
		transaction.onerror = () =>
			reject(transaction.error ?? new Error('IndexedDB transaction failed.'));
	});
}

export function createIndexedDbSafeQueryStore(): SafeQueryStore {
	let databasePromise: Promise<IDBDatabase> | null = null;
	const database = () => {
		if (typeof indexedDB === 'undefined')
			return Promise.reject(new Error('IndexedDB unavailable.'));
		if (!databasePromise) {
			databasePromise = new Promise((resolve, reject) => {
				const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION);
				request.onupgradeneeded = () => {
					const db = request.result;
					const store = db.objectStoreNames.contains(STORE_NAME)
						? request.transaction!.objectStore(STORE_NAME)
						: db.createObjectStore(STORE_NAME, { keyPath: 'id' });
					if (!store.indexNames.contains('company')) store.createIndex('company', 'company');
				};
				request.onsuccess = () => {
					request.result.onversionchange = () => request.result.close();
					resolve(request.result);
				};
				request.onerror = () => reject(request.error ?? new Error('IndexedDB open failed.'));
				request.onblocked = () => reject(new Error('IndexedDB upgrade blocked.'));
			});
		}
		return databasePromise;
	};
	return {
		async load(id) {
			const db = await database();
			return requestResult(db.transaction(STORE_NAME).objectStore(STORE_NAME).get(id));
		},
		async save(envelope) {
			const db = await database();
			const transaction = db.transaction(STORE_NAME, 'readwrite');
			transaction.objectStore(STORE_NAME).put(envelope);
			await transactionDone(transaction);
		},
		async remove(id) {
			const db = await database();
			const transaction = db.transaction(STORE_NAME, 'readwrite');
			transaction.objectStore(STORE_NAME).delete(id);
			await transactionDone(transaction);
		},
		async clearCompany(company, keepId) {
			const db = await database();
			const transaction = db.transaction(STORE_NAME, 'readwrite');
			const store = transaction.objectStore(STORE_NAME);
			const request = store.index('company').openKeyCursor(IDBKeyRange.only(company));
			request.onsuccess = () => {
				const cursor = request.result;
				if (!cursor) return;
				if (cursor.primaryKey !== keepId) store.delete(cursor.primaryKey);
				cursor.continue();
			};
			await transactionDone(transaction);
		}
	};
}

type ActivePrincipal = {
	actorId: string;
	membershipRole: string;
	cachePartition: string;
};

const activePrincipals = new WeakMap<QueryClient, Map<string, ActivePrincipal>>();

function samePrincipal(left: ActivePrincipal, right: ActivePrincipal): boolean {
	return (
		left.actorId === right.actorId &&
		left.membershipRole === right.membershipRole &&
		left.cachePartition === right.cachePartition
	);
}

function clearCompanyMemory(client: QueryClient, company: string): void {
	void client.resetQueries({
		predicate: (query) => query.queryKey.length > 1 && query.queryKey[1] === company
	});
	void client.invalidateQueries({ queryKey: ['companies'] });
	void client.invalidateQueries({ queryKey: ['portfolio'] });
}

function querySnapshots(client: QueryClient): QuerySnapshot[] {
	return client
		.getQueryCache()
		.getAll()
		.flatMap((query) =>
			query.state.data === undefined || companyForPersistableQuery(query.queryKey) === null
				? []
				: [
						{
							queryKey: query.queryKey,
							data: query.state.data,
							dataUpdatedAt: query.state.dataUpdatedAt
						}
					]
		);
}

export function startCompanyQueryPersistence(
	client: QueryClient,
	company: string,
	store: SafeQueryStore = createIndexedDbSafeQueryStore()
): { stop(): void; verify(): Promise<void> } {
	let disposed = false;
	let verification = 0;
	let abort: AbortController | null = null;
	let unsubscribe: (() => void) | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let activeId: string | null = null;
	let activePartition: string | null = null;
	let saveDisabled = false;

	const persist = async () => {
		if (disposed || saveDisabled || !activeId || !activePartition) return;
		const envelope = buildPersistedEnvelope(
			company,
			activePartition,
			querySnapshots(client),
			Date.now()
		);
		try {
			await store.save(envelope);
		} catch {
			/* Quota and browser-private-mode failures cannot break the live app.
			 * One bounded cleanup/retry is useful; repeated failure disables only
			 * persistence for this mounted company. */
			try {
				await store.clearCompany(company);
				await store.save(envelope);
			} catch {
				saveDisabled = true;
			}
		}
	};

	const schedulePersist = (delay = 250) => {
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			saveTimer = null;
			void persist();
		}, delay);
	};
	const flushPersist = () => {
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = null;
		/* Start the IndexedDB write in the visibility event itself. Mobile
		 * browsers may freeze timers immediately after backgrounding. */
		void persist();
	};

	const subscribe = () => {
		unsubscribe?.();
		unsubscribe = client.getQueryCache().subscribe((event) => {
			if (companyForPersistableQuery(event.query.queryKey) === company) schedulePersist();
		});
	};

	const failClosed = async () => {
		unsubscribe?.();
		unsubscribe = null;
		activeId = null;
		activePartition = null;
		activePrincipals.get(client)?.delete(company);
		clearCompanyMemory(client, company);
		try {
			await store.clearCompany(company);
		} catch {
			// Browser storage availability never controls application availability.
		}
	};

	const verify = async () => {
		const token = ++verification;
		abort?.abort();
		abort = new AbortController();
		let principal: CompanyPrincipal;
		try {
			principal = await getCompanyPrincipal(company, abort.signal);
		} catch (error) {
			if (disposed || token !== verification || (error as Error).name === 'AbortError') return;
			const status = (error as { status?: unknown }).status;
			const code = (error as { code?: unknown }).code;
			if (status === 401 || status === 403 || status === 404 || code === 'invalid_principal') {
				await failClosed();
			}
			return;
		}
		if (disposed || token !== verification) return;
		const partition = principal.cache_partition;
		const id = queryCacheRecordId(company, partition);
		const principals = activePrincipals.get(client) ?? new Map<string, ActivePrincipal>();
		activePrincipals.set(client, principals);
		const nextPrincipal = {
			actorId: principal.actor_id,
			membershipRole: principal.membership_role,
			cachePartition: partition
		};
		const previous = principals.get(company);
		const principalChanged = Boolean(previous && !samePrincipal(previous, nextPrincipal));
		if (principalChanged) clearCompanyMemory(client, company);
		principals.set(company, nextPrincipal);
		activePartition = partition;
		activeId = id;
		saveDisabled = false;
		try {
			/* A server that changes actor or membership without rotating its opaque
			 * partition has violated the expected contract. Defend in depth by
			 * purging every local projection and refusing that cycle's hydration. */
			await store.clearCompany(company, principalChanged ? undefined : id);
			const stored = principalChanged ? undefined : await store.load(id);
			if (disposed || token !== verification) return;
			const envelope = parsePersistedEnvelope(stored, company, partition);
			if (stored !== undefined && stored !== null && !envelope) await store.remove(id);
			if (envelope) {
				for (const entry of envelope.entries) {
					const current = client.getQueryState(entry.queryKey);
					if (!shouldHydratePersistedEntry(current?.dataUpdatedAt, entry.dataUpdatedAt)) continue;
					client.setQueryData(entry.queryKey, entry.data, { updatedAt: entry.dataUpdatedAt });
					void client.invalidateQueries({
						queryKey: entry.queryKey,
						exact: true,
						refetchType: 'active'
					});
				}
			}
		} catch {
			/* Corrupt, blocked or unavailable browser storage is ignored. The
			 * principal verification still succeeded, so live queries continue. */
		}
		subscribe();
		schedulePersist();
	};

	const onVisibility = () => {
		if (document.visibilityState === 'hidden') flushPersist();
		else void verify();
	};
	const onPageShow = () => void verify();
	const onPageHide = () => flushPersist();
	if (typeof document !== 'undefined') document.addEventListener('visibilitychange', onVisibility);
	if (typeof window !== 'undefined') window.addEventListener('pageshow', onPageShow);
	if (typeof window !== 'undefined') window.addEventListener('pagehide', onPageHide);
	void verify();

	return {
		stop() {
			flushPersist();
			disposed = true;
			verification += 1;
			abort?.abort();
			unsubscribe?.();
			if (typeof document !== 'undefined')
				document.removeEventListener('visibilitychange', onVisibility);
			if (typeof window !== 'undefined') window.removeEventListener('pageshow', onPageShow);
			if (typeof window !== 'undefined') window.removeEventListener('pagehide', onPageHide);
		},
		verify
	};
}
