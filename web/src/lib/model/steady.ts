/**
 * What is on screen only moves when a newer fact says so (Sprint 82).
 *
 * Two rules, enforced once here instead of at every call site:
 *
 * 1. A local write wins over any read already in flight. A save, a send or a
 *    decision writes its own answer into the cache; a read that started before
 *    that write describes the past and is cancelled rather than allowed to land
 *    over it. (TanStack's own cancel reverts to the pre-fetch state, which would
 *    erase the write, so the cancel happens first and does not revert.)
 * 2. A change hint refetches only the queries that kind of change can affect.
 *    The server names the kinds (`message`, `work_changed`, ...); company
 *    settings, the principal and model connections are never refetched by a
 *    teammate's message.
 *
 * The tripwire makes the original bug loud: a conversation message that was on
 * screen and disappears without a retract is reported (and thrown in tests).
 */

import type { QueryClient } from '@tanstack/svelte-query';

/** Make every local write on this client win over reads already in flight. */
export function steady<T extends QueryClient>(client: T): T {
	const write = client.setQueryData.bind(client);
	client.setQueryData = ((queryKey, updater, options) => {
		const query = client.getQueryCache().find({ queryKey, exact: true });
		if (query?.state.fetchStatus === 'fetching') void query.cancel({ revert: false, silent: true });
		return write(queryKey, updater as never, options);
	}) as typeof client.setQueryData;
	return client;
}

/**
 * Refresh everything on screen once: after an outage, or on coming back from the background,
 * when anything read meanwhile may be behind. The only blanket refresh in the app; a change
 * hint refreshes by kind instead (`hintTouches`), and lint keeps it that way.
 */
export function resyncVisible(client: QueryClient): Promise<void> {
	return client.invalidateQueries({ refetchType: 'active' });
}

/* ---- Change hints ------------------------------------------------------- */

/** Queries a teammate's activity can never change: settings, identity, connections. */
const SETTINGS_KEYS = new Set([
	'companies',
	'company',
	'company-principal',
	'company-identity',
	'intelligence',
	'connected-models',
	'account-connections',
	'browser-status',
	'inbox-marks',
	'model-catalog'
]);

/** Queries a message or mention can change. */
const CONVERSATION_KEYS = new Set([
	'conversation',
	'recent-direct-conversations',
	'recent-group-conversations',
	'rooms',
	'attention',
	'for-you',
	'direction-digest',
	'direction-requests',
	'agent-exchanges',
	'message-reactions'
]);

/** Kinds that touch messages only. Anything else (Work, handoffs, library,
 * schedules, resync, an unknown kind) may change any non-settings query. */
const CONVERSATION_KINDS = new Set(['message', 'room_event', 'mention']);

/**
 * Whether a hint naming `kinds` should refetch the query with this key.
 * Rooms that own an exact event stream are excluded by the caller.
 */
export function hintTouches(kinds: readonly string[], queryKey: readonly unknown[]): boolean {
	const head = String(queryKey[0]);
	if (kinds.includes('resync')) return !SETTINGS_KEYS.has(head);
	if (SETTINGS_KEYS.has(head)) return false;
	if (kinds.every((kind) => kind === 'handoff_decision_changed'))
		return head === 'handoff-decisions' || head === 'attention';
	if (head === 'schedules') return kinds.includes('schedule_changed');
	if (kinds.every((kind) => CONVERSATION_KINDS.has(kind))) {
		return (
			CONVERSATION_KEYS.has(head) || (kinds.includes('mention') && head.startsWith('document'))
		);
	}
	return true;
}

export function parseHint(data: unknown): string[] {
	const kinds = String(data ?? '')
		.split(',')
		.map((kind) => kind.trim())
		.filter(Boolean);
	return kinds.length ? kinds : ['resync'];
}

/* ---- Tripwire ----------------------------------------------------------- */

const retracted = new Set<string>();

/** Mark a message the viewer took back, so its disappearance is expected. */
export function expectRemoval(messageId: string | number): void {
	retracted.add(String(messageId));
}

export function vanished(
	before: readonly { id: string | number }[] | undefined,
	after: readonly { id: string | number }[] | undefined
): string[] {
	if (!before?.length || !after) return [];
	const now = new Set(after.map((message) => String(message.id)));
	// A conversation read returns the newest window; ids older than its oldest are out of
	// range, not gone.
	const oldest = Math.min(...after.map((message) => Number(message.id)).filter(Number.isFinite));
	return before
		.map((message) => String(message.id))
		.filter(
			(id) =>
				!now.has(id) && !retracted.has(id) && !(Number.isFinite(oldest) && Number(id) < oldest)
		);
}

/**
 * Watch conversation caches for a shown message disappearing. In tests (and
 * when `window.__restlessStrict` is set by the smoke suite) it throws; otherwise it
 * logs once, so production users never see an error but the console shows it.
 */
export function installTripwire(client: QueryClient): () => void {
	if (
		typeof window !== 'undefined' &&
		(window as unknown as { __restlessStrict?: boolean }).__restlessStrict
	) {
		// The smoke suite drives reads at exact moments (a read in flight during a send).
		(window as unknown as { __restlessQueryClient?: QueryClient }).__restlessQueryClient = client;
	}
	const seen = new Map<string, readonly { id: number }[]>();
	const reported = new Set<string>();
	return client.getQueryCache().subscribe((event) => {
		if (event.type !== 'updated' || event.query.queryKey[0] !== 'conversation') return;
		const data = event.query.state.data as { messages?: { id: number }[] } | undefined;
		if (!data?.messages) return;
		const hash = event.query.queryHash;
		const gone = vanished(seen.get(hash), data.messages);
		seen.set(hash, data.messages);
		if (!gone.length) return;
		const report = `Steady: conversation message ${gone.join(', ')} disappeared without a retract`;
		const strict =
			typeof window !== 'undefined' &&
			(window as unknown as { __restlessStrict?: boolean }).__restlessStrict;
		if (strict) {
			(window as unknown as { __restlessViolations?: string[] }).__restlessViolations ??= [];
			(window as unknown as { __restlessViolations: string[] }).__restlessViolations.push(report);
		}
		if (!reported.has(report)) {
			reported.add(report);
			console.error(report);
		}
	});
}
