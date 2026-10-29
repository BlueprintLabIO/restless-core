import { openLiveEvents } from './live-events.ts';
/**
 * Owner-facing server state belongs to TanStack Query. The daemon remains the
 * writer; these helpers only choose cache keys, freshness and invalidation.
 *
 * Live ACP activity is intentionally separate from durable records: SSE is a
 * tiny overlay that invalidates the relevant query when the turn settles.
 */

import { getApplianceStatus } from './appliance';
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { onMount } from 'svelte';
import type { QueryClient } from '@tanstack/svelte-query';
import {
	getActorConversation,
	getAttention,
	getWorkGraph,
	getWorkContext,
	openAgentActivityStream,
	sendActorMessage,
	interruptActorMessage,
	retryActorMessage,
	type AttentionView,
	type ActorConversation,
	type AgentActivityState,
	type MessageSendResult
} from './attention';
import { getCockpit, getCompanies, type CockpitView, type CompanyCatalogEntry } from './cockpit';
import {
	collaborationMatchesPrincipal,
	getCollaborationBootstrap,
	type CollaborationBootstrap
} from './collaboration';
import {
	getBrowserStatus,
	getCompany,
	getCompanySpend,
	type BrowserStatus,
	type CompanySpendView,
	type CompanyView
} from './company';
import { getCompanyIdentity, type CompanyIdentitySnapshot } from './identity';
import { getCompanyPrincipal, type CompanyPrincipal } from './query-persistence';
import type { ThreadMessage } from './view';
import { changes, pollEvery } from './connection.svelte';
import { hintTouches, parseHint } from './steady';

export type QuerySourceStatus = 'unknown' | 'live' | 'stale';
export type ActivityTransport = 'idle' | 'connecting' | 'live' | 'reconnecting';
type QueryEnabled = boolean | (() => boolean);

const STALE_MS = 5_000;
const REFRESH_MS = 10_000;
const RETAIN_MS = 10 * 60_000;
/* How long an ambient turn's working line outlives the turn, in case the next one starts. */
const AMBIENT_GAP_MS = 1_200;

function queryEnabled(value: QueryEnabled): boolean {
	return typeof value === 'function' ? value() : value;
}

export const queryKeys = {
	companies: ['companies'] as const,
	appliance: ['appliance'] as const,
	principal: (company: string) => ['company-principal', company] as const,
	collaboration: (company: string, principal: CompanyPrincipal | null | undefined) =>
		[
			'company-collaboration',
			company,
			principal?.actor_id ?? null,
			principal?.membership_role ?? null,
			principal?.cache_partition ?? null
		] as const,
	attention: (company: string) => ['attention', company] as const,
	workGraph: (company: string) => ['work-graph', company] as const,
	workContext: (company: string) => ['work-context', company] as const,
	cockpit: (company: string) => ['cockpit', company] as const,
	company: (company: string, probeCredentials: boolean) =>
		['company', company, { probeCredentials }] as const,
	companySpend: (company: string) => ['company-spend', company] as const,
	identity: (company: string) => ['company-identity', company] as const,
	conversation: (company: string, actor: string, workId?: string) =>
		['conversation', company, actor, workId ?? null] as const,
	browserStatus: (company: string) => ['browser-status', company] as const
};

/** Host/account identity is shared for navigation, not polled independently by each shell. */
export function applianceQuery(enabled: QueryEnabled = true) {
	const query = createQuery(() => ({
		queryKey: queryKeys.appliance,
		queryFn: ({ signal }) => getApplianceStatus(signal),
		enabled: queryEnabled(enabled),
		staleTime: 60_000,
		gcTime: RETAIN_MS,
		retry: false,
		refetchOnWindowFocus: false
	}));
	return {
		get view() {
			return query.data ?? null;
		},
		get settled() {
			return !query.isPending;
		},
		get failure() {
			return query.error as Error | null;
		},
		refresh: () => refresh(query)
	};
}

export function companyPrincipalQuery(companyId: string) {
	const query = createQuery(() => ({
		queryKey: queryKeys.principal(companyId),
		queryFn: ({ signal }) => getCompanyPrincipal(companyId, signal),
		enabled: Boolean(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 60_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as CompanyPrincipal | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number; code?: string }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

export function collaborationBootstrapQuery(
	companyId: string,
	principal: () => CompanyPrincipal | null | undefined
) {
	const query = createQuery(() => ({
		queryKey: queryKeys.collaboration(companyId, principal()),
		queryFn: async ({ signal }) => {
			const expected = principal();
			if (!expected) {
				throw Object.assign(new Error('The company principal changed during collaboration.'), {
					code: 'collaboration_principal_changed'
				});
			}
			const view = await getCollaborationBootstrap(companyId, signal);
			if (!collaborationMatchesPrincipal(view, expected)) {
				throw Object.assign(new Error('The company collaboration principal does not match.'), {
					code: 'collaboration_principal_changed'
				});
			}
			return view;
		},
		enabled: Boolean(companyId) && Boolean(principal()),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 60_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as CollaborationBootstrap | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number; code?: string }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

function statusOf(query: {
	data?: unknown;
	isPending: boolean;
	isError: boolean;
}): QuerySourceStatus {
	if (query.isPending && !query.data) return 'unknown';
	return query.isError ? 'stale' : 'live';
}

function refresh<T>(query: { refetch: () => Promise<T> }): Promise<T> {
	return query.refetch();
}

/** Current Work references suffice for navigation and conversation context. */
export function workContextQuery(
	companyId: string | (() => string),
	enabled: QueryEnabled = true,
	fullGraph: QueryEnabled = false
) {
	const company = () => (typeof companyId === 'function' ? companyId() : companyId);
	const graph = createQuery(() => ({
		queryKey: queryKeys.workGraph(company()),
		queryFn: ({ queryKey, signal }) => getWorkGraph(queryKey[1], signal),
		enabled: queryEnabled(enabled) && queryEnabled(fullGraph),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 120_000),
		refetchIntervalInBackground: false
	}));
	const context = createQuery(() => ({
		queryKey: queryKeys.workContext(company()),
		queryFn: ({ queryKey, signal }) => getWorkContext(queryKey[1], signal),
		enabled: queryEnabled(enabled) && !queryEnabled(fullGraph),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 120_000),
		refetchIntervalInBackground: false
	}));
	const current = () => (queryEnabled(fullGraph) ? graph : context);
	return {
		get view() {
			return current().data ?? null;
		},
		get status() {
			return statusOf(current());
		},
		get failure() {
			return current().error;
		}
	};
}

export function attentionQuery(
	companyId: string | (() => string),
	enabled: QueryEnabled = true,
	withGraph: QueryEnabled = false
) {
	const currentCompany = () => (typeof companyId === 'function' ? companyId() : companyId);
	const client = useQueryClient();
	onMount(() => {
		if (typeof BroadcastChannel === 'undefined') return;
		const channel = new BroadcastChannel('restless-attention');
		channel.onmessage = (event) => {
			if (event.data === currentCompany() && queryEnabled(enabled)) {
				void invalidateAttentionReads(client, currentCompany());
			}
		};
		return () => channel.close();
	});
	const query = createQuery(() => ({
		queryKey: queryKeys.attention(currentCompany()),
		queryFn: ({ queryKey, signal }) => getAttention(queryKey[1], signal),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 30_000),
		refetchIntervalInBackground: false
	}));
	const graph = createQuery(() => ({
		queryKey: queryKeys.workGraph(currentCompany()),
		queryFn: ({ queryKey, signal }) => getWorkGraph(queryKey[1], signal),
		enabled: queryEnabled(enabled) && queryEnabled(withGraph),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 120_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			// Track both results even if one is still pending. Query observers only notify
			// properties their consumer has read; short-circuiting can miss the second answer.
			const queue = query.data;
			const work = graph.data;
			if (!queue || (queryEnabled(withGraph) && !work)) return null;
			return queryEnabled(withGraph) ? { ...queue, workGraph: work ?? null } : queue;
		},
		get status() {
			const queue = statusOf(query);
			const work = statusOf(graph);
			// Persisted summaries omit live-only sources and actions. Their absence
			// is never evidence that an Authority request has been resolved.
			if (query.data?.sourceHealth.authority === 'unknown' && !query.isError) return 'unknown';
			return queryEnabled(withGraph) && work !== 'live' ? work : queue;
		},
		get failure() {
			return (
				((query.error ?? (queryEnabled(withGraph) ? graph.error : null)) as
					(Error & { status?: number }) | null) ?? null
			);
		},
		refresh: () => refreshAttention(client, currentCompany()),
		// An explicit open must perform a read even while cached observers settle.
		reload: () =>
			Promise.all([refresh(query), ...(queryEnabled(withGraph) ? [refresh(graph)] : [])]).then(
				([result, ...graphs]) => ({
					...result,
					error: result.error ?? graphs.find((value) => value.error)?.error ?? null
				})
			)
	};
}

async function invalidateAttentionReads(client: QueryClient, company: string) {
	await Promise.all(
		[
			queryKeys.attention(company),
			queryKeys.workGraph(company),
			queryKeys.workContext(company)
		].map((queryKey) => client.invalidateQueries({ queryKey }))
	);
}

/** One server-owned queue, refreshed in this window and other open cockpit tabs. */
export async function refreshAttention(client: QueryClient, companyId: string) {
	void client.invalidateQueries({ queryKey: ['handoff-decisions', companyId] });
	if (typeof BroadcastChannel !== 'undefined') {
		const channel = new BroadcastChannel('restless-attention');
		channel.postMessage(companyId);
		channel.close();
	}
	await invalidateAttentionReads(client, companyId);
}

/** A confirmed decision leaves the queue without waiting for a full company read. */
export async function removeConfirmedAttention(
	client: QueryClient,
	companyId: string,
	itemId: string,
	pendingHandoffDecision = false
) {
	const queryKey = queryKeys.attention(companyId);
	// An older in-flight read must not put the decided card back into the queue.
	await client.cancelQueries({ queryKey });
	client.setQueryData<AttentionView>(
		queryKey,
		(view) =>
			view && {
				...view,
				...(pendingHandoffDecision ? { pendingHandoffDecisions: [...new Set([...(view.pendingHandoffDecisions ?? []), itemId])] } : {}),
				items: view.items.filter((item) => item.id !== itemId)
			}
	);
	void refreshAttention(client, companyId);
}

/** `background` keeps the list reading while the tab is hidden (desktop notifications). */
export function companiesQuery(
	enabled: QueryEnabled = true,
	background: () => boolean = () => false
) {
	const query = createQuery(() => ({
		queryKey: queryKeys.companies,
		queryFn: ({ signal }) => getCompanies(signal),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		// The list carries each company's card, so it refreshes as often as the portfolio did.
		refetchInterval: pollEvery(30_000),
		refetchIntervalInBackground: background()
	}));
	return {
		get view() {
			return query.data ?? [];
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

export function cockpitQuery(companyId: string | (() => string), enabled: QueryEnabled = true) {
	const currentCompany = () => (typeof companyId === 'function' ? companyId() : companyId);
	const query = createQuery(() => ({
		queryKey: queryKeys.cockpit(currentCompany()),
		queryFn: ({ queryKey, signal }) => getCockpit(queryKey[1], false, signal),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 30_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as CockpitView | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

export function companyQuery(companyId: string, enabled: QueryEnabled = true) {
	const client = useQueryClient();
	let probeCredentials = $state(false);
	const options = (probe: boolean, on: () => boolean) => ({
		queryKey: queryKeys.company(companyId, probe),
		queryFn: ({ signal }: { signal: AbortSignal }) => getCompany(companyId, probe, signal),
		enabled: queryEnabled(enabled) && on(),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(60_000),
		refetchIntervalInBackground: false
	});
	/* The ordinary read always runs. A page that asks for a live credential
	 * probe (which can take seconds) shows the ordinary read — where every
	 * reference says it is unprobed — until the probe answers. */
	const plain = createQuery(() => options(false, () => true));
	const probed = createQuery(() => options(true, () => probeCredentials));
	const current = () => (probeCredentials && (probed.data || probed.error) ? probed : plain);
	return {
		/* The probed copy speaks only on a page that asked for it. Elsewhere a copy cached by an
		 * earlier visit is stale: it would hide every save and refresh this page makes. */
		get view() {
			const data = probeCredentials ? (probed.data ?? plain.data) : plain.data;
			return (data as CompanyView | undefined) ?? null;
		},
		get status() {
			if (probeCredentials && !probed.data && !probed.error) return 'unknown';
			return statusOf(current());
		},
		get failure() {
			return (current().error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(current()),
		accept(view: CompanyView): void {
			if (probeCredentials) client.setQueryData(queryKeys.company(companyId, true), view);
			else void client.invalidateQueries({ queryKey: queryKeys.company(companyId, true) });
			client.setQueryData(queryKeys.company(companyId, false), view);
		},
		attach(probe = false): () => void {
			if (probe) probeCredentials = true;
			return () => {};
		}
	};
}

/** What the company spends, for an admin who reads it beside the owner; the owner reads it from
 * the whole company view. */
export function companySpendQuery(companyId: string, enabled: QueryEnabled = true) {
	const query = createQuery(() => ({
		queryKey: queryKeys.companySpend(companyId),
		queryFn: () => getCompanySpend(companyId),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(60_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as CompanySpendView | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

export function identityQuery(companyId: string, enabled: QueryEnabled = true) {
	const query = createQuery(() => ({
		enabled: queryEnabled(enabled),
		queryKey: queryKeys.identity(companyId),
		queryFn: () => getCompanyIdentity(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		// Settings change by a save here, which writes its own answer; a slow poll is the fallback.
		refetchInterval: pollEvery(REFRESH_MS, 60_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as CompanyIdentitySnapshot | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

export function browserStatusQuery(companyId: string) {
	const query = createQuery(() => ({
		queryKey: queryKeys.browserStatus(companyId),
		queryFn: () => getBrowserStatus(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 60_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return (query.data as BrowserStatus | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query)
	};
}

function threadMessage(
	message: ActorConversation['messages'][number],
	actorDisplay: string,
	viewerActorId: string
): ThreadMessage {
	const fromViewer = message.from_actor === viewerActorId;
	return {
		id: String(message.id),
		from: fromViewer ? 'you' : 'agent',
		author: fromViewer ? 'You' : actorDisplay,
		text: message.body,
		createdAt: message.created_at,
		replyToMessageId: null,
		assetId: null,
		runId: null,
		attachments: message.attachments ?? [],
		details: message.details ?? null,
		intent: message.intent ?? null,
		/* Core sends each receipt as the account_use block (S98-09); its shape lives in owner-accounts. */
		accountUses: (message.account_uses ?? []) as unknown as ThreadMessage['accountUses'],
		contextPath: message.context_path ?? null,
		readAt: message.read_at ?? null
	};
}

export interface ActiveAgentTurn {
	triggerMessageId: number;
	since: Date | string;
	live: AgentActivityState | null;
	transport: ActivityTransport;
	/** The agent's saved reply is already in the conversation, so the streamed copy steps aside. */
	replyLanded?: boolean;
}

/* A turn stays on screen after it ends so the dock can say "Replied" or why it failed; only a
 * turn still in progress is something a new message can interrupt or queue behind. */
export function turnWorking(turn: ActiveAgentTurn | null | undefined): boolean {
	if (!turn) return false;
	const phase = turn.live?.phase ?? 'queued';
	return (
		phase !== 'complete' &&
		phase !== 'failed' &&
		phase !== 'unavailable' &&
		turn.transport !== 'idle'
	);
}

/**
 * One conversation query plus, only while needed, one SSE subscription. There
 * is no client Map, second transcript cache, timer or synthetic completion
 * state: a completed event simply invalidates its OrgIntel query.
 */
export function conversationQuery(
	companyId: string,
	actorId: string,
	workId?: string,
	attentionId?: string,
	enabled: QueryEnabled = true,
	viewerActorId = 'owner',
	followLiveActivity: QueryEnabled = true,
	/** Also follow the actor's ambient turns (only the rail does: one more open connection). */
	followAmbient: QueryEnabled = false
) {
	const client = useQueryClient();
	const key = queryKeys.conversation(companyId, actorId, workId);
	const query = createQuery(() => ({
		queryKey: key,
		queryFn: () => getActorConversation(companyId, actorId, workId),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 60_000),
		refetchIntervalInBackground: false
	}));

	let live = $state<AgentActivityState | null>(null);
	let transport = $state<ActivityTransport>('idle');
	let pending = $state<ThreadMessage | null>(null);
	/* How much of a message's files has gone, while they go. */
	let uploadProgress = $state<number | null>(null);
	/* A sent message keeps the key it was drawn with once its saved id arrives. */
	const sentKeys = new Map<string, string>();
	let followingMessageId = $state<number | null>(null);
	let stop: (() => void) | null = null;
	let uncertainCommand: {
		id: string;
		body: string;
		files: File[];
		contextPath?: string;
		newFocus: boolean;
		interrupt: boolean;
		skills: string[];
	} | null = null;

	const follow = (messageId: number, since: Date | string): void => {
		if (!queryEnabled(followLiveActivity)) return;
		if (followingMessageId === messageId && stop) return;
		stop?.();
		followingMessageId = messageId;
		transport = 'connecting';
		live = null;
		let invalidatedCommittedMessageId: number | null = null;
		let invalidatedTerminalWithoutReply = false;
		stop = openAgentActivityStream(
			companyId,
			actorId,
			// A Work-linked owner message still has one message-scoped turn.
			// The Work inspector owns its independent work-scoped subscription;
			// sending both selectors is deliberately invalid at the API boundary.
			{ messageId },
			(state) => {
				if (followingMessageId !== messageId) return;
				live = state;
				transport = 'live';
				if (
					state.completedMessageId !== null &&
					state.completedMessageId !== invalidatedCommittedMessageId
				) {
					invalidatedCommittedMessageId = state.completedMessageId;
					void client.invalidateQueries({ queryKey: key });
				} else if (
					state.completedMessageId === null &&
					(state.phase === 'complete' ||
						state.phase === 'failed' ||
						state.phase === 'unavailable') &&
					!invalidatedTerminalWithoutReply
				) {
					invalidatedTerminalWithoutReply = true;
					void client.invalidateQueries({ queryKey: key });
				}
			},
			() => {
				if (
					followingMessageId === messageId &&
					live?.phase !== 'complete' &&
					live?.phase !== 'failed' &&
					live?.phase !== 'unavailable'
				)
					transport = 'reconnecting';
			}
		);
		void since;
	};

	$effect(() => {
		const conversation = query.data as ActorConversation | undefined;
		const last = conversation?.messages.at(-1);
		if (last?.from_actor === viewerActorId && queryEnabled(followLiveActivity)) {
			follow(last.id, last.created_at);
			return;
		}
		if (last && last.from_actor !== viewerActorId) {
			if (live?.phase === 'complete' || live?.phase === 'failed' || live?.phase === 'unavailable') {
				stop?.();
				stop = null;
				followingMessageId = null;
				transport = 'idle';
			}
		}
	});

	/* What the actor is doing when no person's message started it: a schedule, a Staff result,
	 * a mention. One quiet connection per open conversation, held between turns. When such a
	 * turn ends, its reply (if any) is in the conversation. */
	let ambient = $state<AgentActivityState | null>(null);
	$effect(() => {
		if (
			!queryEnabled(enabled) ||
			!queryEnabled(followLiveActivity) ||
			!queryEnabled(followAmbient) ||
			typeof EventSource === 'undefined'
		) {
			ambient = null;
			return;
		}
		ambient = null;
		let wasLive = false;
		/* One turn often ends just before the next starts (a Staff result wakes the actor again).
		 * The working line holds through that gap rather than vanishing and coming back. */
		let ending: ReturnType<typeof setTimeout> | null = null;
		const close = openAgentActivityStream(companyId, actorId, {}, (state) => {
			const isLive =
				state.phase !== 'complete' && state.phase !== 'failed' && state.phase !== 'unavailable';
			if (ending) clearTimeout(ending);
			ending = null;
			if (wasLive && !isLive) {
				ending = setTimeout(() => {
					ending = null;
					ambient = state;
				}, AMBIENT_GAP_MS);
				void client.invalidateQueries({ queryKey: key });
			} else ambient = state;
			wasLive = isLive;
		});
		// A query can be discarded while its parent derived is recomputing.
		// Cleanup releases the subscription; reactive state is reset in the effect body.
		return () => {
			if (ending) clearTimeout(ending);
			close();
		};
	});

	/* The query holds the newest 100 messages; earlier ones are paged in on request and kept
	 * here. Messages that later fall out of the newest 100 join them, so no gap opens. */
	const PAGE = 100;
	let older = $state<ActorConversation['messages']>([]);
	let olderComplete = $state(false);
	let lastHeld: ActorConversation['messages'] = [];
	$effect(() => {
		const held = (query.data as ActorConversation | undefined)?.messages ?? [];
		if (older.length && held.length) {
			const first = held[0].id;
			const known = new Set(older.map((message) => message.id));
			const fell = lastHeld.filter((message) => message.id < first && !known.has(message.id));
			if (fell.length) older = [...older, ...fell].sort((a, b) => a.id - b.id);
		}
		lastHeld = held;
	});

	/* The placeholder leaves only once the server's copy is held. No read can drop that copy:
	 * a local write cancels every older read in flight. */
	$effect(() => {
		const held = (query.data as ActorConversation | undefined)?.messages;
		if (pending && held?.some((message) => String(message.id) === pending?.id)) pending = null;
	});

	return {
		get actor() {
			const actor = (query.data as ActorConversation | undefined)?.actor;
			return actor ? { ...actor, display: actor.id === 'exec' ? 'Exec' : actor.display } : null;
		},
		get messages() {
			const conversation = query.data as ActorConversation | undefined;
			const actorDisplay =
				conversation?.actor.id === 'exec' ? 'Exec' : (conversation?.actor.display ?? actorId);
			const held = conversation?.messages ?? [];
			const first = held[0]?.id ?? Infinity;
			const earlier = older.filter((message) => message.id < first);
			const messages = [...earlier, ...held].map((message) => {
				const thread = threadMessage(message, actorDisplay, viewerActorId);
				const key = sentKeys.get(thread.id);
				return key ? { ...thread, clientKey: key } : thread;
			});
			return pending && !messages.some((message) => message.id === pending?.id)
				? [...messages, pending]
				: messages;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get focusAfterMessageId() {
			return (query.data as ActorConversation | undefined)?.focus?.after_message_id ?? 0;
		},
		get focusStartedAt() {
			return (query.data as ActorConversation | undefined)?.focus?.started_at ?? null;
		},
		/** The Work this agent is executing now; a message waits for it unless it interrupts. */
		get busyWork(): { id: string; title: string } | null {
			return (query.data as ActorConversation | undefined)?.busy_work ?? null;
		},
		get activeTurn(): ActiveAgentTurn | null {
			if (!queryEnabled(followLiveActivity)) return null;
			const current = live;
			const messageId = current?.triggerMessageId ?? followingMessageId ?? Number(pending?.id);
			if (!Number.isFinite(messageId)) return null;
			/* The saved reply can arrive before the stream says it finished; showing both drew the
			 * reply twice for a moment, then dropped one (S85-04). */
			const held = (query.data as ActorConversation | undefined)?.messages ?? [];
			const replyLanded = held.some(
				(message) => message.id > messageId && message.from_actor === actorId
			);
			if (current?.phase === 'unavailable' && replyLanded) return null;
			return {
				triggerMessageId: messageId,
				since: current?.startedAt ?? pending?.createdAt ?? new Date(),
				live: current,
				transport,
				replyLanded
			};
		},
		get working() {
			return turnWorking(this.activeTurn);
		},
		/** A turn no person's message started, while it runs; its reply text is not shown. */
		get ambientTurn(): ActiveAgentTurn | null {
			const current = ambient;
			if (
				!current ||
				current.phase === 'complete' ||
				current.phase === 'failed' ||
				current.phase === 'unavailable'
			)
				return null;
			return {
				triggerMessageId: 0,
				since: current.startedAt ?? current.updatedAt,
				live: current,
				transport: 'live'
			};
		},
		get uploadProgress() {
			return uploadProgress;
		},
		/** The key a retry must reuse while the last send's outcome is unknown. */
		get uncertainCommandId() {
			return uncertainCommand?.id ?? null;
		},
		refresh: () => refresh(query),
		/** Whether the server holds messages older than the ones here. */
		get hasEarlier() {
			if (workId || olderComplete) return false;
			const held = (query.data as ActorConversation | undefined)?.messages ?? [];
			return older.length > 0 || held.length >= PAGE;
		},
		/** Page in the next earlier messages; resolves to how many arrived. */
		async loadEarlier(): Promise<number> {
			if (workId || olderComplete) return 0;
			const held = (query.data as ActorConversation | undefined)?.messages ?? [];
			const oldest = older[0]?.id ?? held[0]?.id;
			if (!oldest) return 0;
			const page = await getActorConversation(companyId, actorId, undefined, oldest);
			const known = new Set([...older, ...held].map((message) => message.id));
			const fresh = page.messages.filter((message) => !known.has(message.id));
			older = [...fresh, ...older];
			if (page.messages.length < PAGE) olderComplete = true;
			return fresh.length;
		},
		/** Stop the turn answering the viewer's message, sending nothing new. */
		async stopTurn() {
			const messageId = live?.triggerMessageId ?? followingMessageId;
			if (!messageId) throw new Error('There is no reply in progress to stop.');
			/* Stop keeps trying while the company is unreachable (a release restarting the plane),
			 * for up to a minute, instead of failing the moment the connection drops (21 October). */
			const deadline = Date.now() + 60_000;
			for (let wait = 1_000; ; wait = Math.min(wait * 2, 8_000)) {
				try {
					const result = await interruptActorMessage(companyId, actorId, messageId);
					void client.invalidateQueries({ queryKey: key });
					return result;
				} catch (cause) {
					const status = (cause as { status?: number } | null)?.status;
					const unreachable =
						typeof status !== 'number' || status >= 500 || status === 408 || status === 429;
					if (!unreachable || Date.now() + wait > deadline) throw cause;
					await new Promise((resolve) => setTimeout(resolve, wait));
				}
			}
		},
		/** Answer the same message again after its turn failed; it streams on the open connection. */
		async retryTurn() {
			const messageId = live?.triggerMessageId ?? followingMessageId;
			if (!messageId) throw new Error('There is no failed reply to retry.');
			await retryActorMessage(companyId, actorId, messageId);
		},
		attach(): () => void {
			return () => {
				stop?.();
				stop = null;
				transport = 'idle';
			};
		},
		async send(
			body: string,
			files: File[] = [],
			contextPath?: string,
			newFocus = false,
			interrupt = false,
			skills: string[] = [],
			/** A key kept from an unconfirmed send of this exact message (it survives a reload). */
			retryCommandId?: string
		): Promise<MessageSendResult> {
			const sameUncertainIntent =
				uncertainCommand?.body === body &&
				uncertainCommand.skills.join(' ') === skills.join(' ') &&
				uncertainCommand.contextPath === contextPath &&
				uncertainCommand.newFocus === newFocus &&
				uncertainCommand.interrupt === interrupt &&
				uncertainCommand.files.length === files.length &&
				uncertainCommand.files.every((file, index) => file === files[index]);
			const clientCommandId =
				retryCommandId ?? (sameUncertainIntent ? uncertainCommand!.id : crypto.randomUUID());
			uncertainCommand = {
				id: clientCommandId,
				body,
				files: [...files],
				contextPath,
				newFocus,
				interrupt,
				skills: [...skills]
			};
			const sentAt = new Date();
			const optimisticId = `optimistic:${clientCommandId}`;
			pending = {
				id: optimisticId,
				from: 'you',
				author: 'You',
				text: body,
				createdAt: sentAt,
				replyToMessageId: null,
				assetId: null,
				runId: null,
				/* The files show at once, from this browser's copy, at their final size: a message
				 * with an image no longer sends text-only and then jumps when the image arrives. */
				attachments: files.map((file, index) => ({
					uploadId: `${optimisticId}:${index}`,
					name: file.name,
					mediaType: file.type || 'application/octet-stream',
					sizeBytes: file.size,
					localUrl: URL.createObjectURL(file)
				})),
				contextPath: contextPath ?? null
			};
			const localUrls = pending.attachments.flatMap((file) =>
				file.localUrl ? [file.localUrl] : []
			);
			/* Long enough for the server's copy to replace them. */
			const releaseLocal = () =>
				window.setTimeout(() => localUrls.forEach((url) => URL.revokeObjectURL(url)), 120_000);
			let result: MessageSendResult;
			try {
				result = await sendActorMessage(
					companyId,
					actorId,
					body,
					workId,
					files,
					contextPath,
					newFocus,
					interrupt,
					attentionId,
					clientCommandId,
					skills,
					files.length ? (fraction) => (uploadProgress = fraction) : undefined
				);
				uncertainCommand = null;
			} catch (error) {
				if (pending?.id === optimisticId) pending = null;
				releaseLocal();
				// A server response is definitive (including semantic conflict). A
				// transport error is not: retain this exact intent's key so the
				// owner's next retry asks for the committed receipt instead of
				// creating a second Message/attachment set/wake.
				const status = (error as { status?: unknown }).status;
				if (
					typeof status === 'number' &&
					status >= 400 &&
					status < 500 &&
					![408, 425, 429].includes(status)
				) {
					uncertainCommand = null;
				}
				throw error;
			} finally {
				uploadProgress = null;
			}
			releaseLocal();
			sentKeys.set(String(result.messageId), optimisticId);
			/* The saved message goes straight into the conversation. A read already in flight
			 * started before it existed, so that read is cancelled (steady.ts) rather than
			 * allowed to land over it and reopen whatever it answered. Attachments come back
			 * from the server's read, so a message with files keeps its placeholder until then. */
			if (!files.length) {
				client.setQueryData<ActorConversation>(key, (conversation) =>
					conversation && !conversation.messages.some((message) => message.id === result.messageId)
						? {
								...conversation,
								messages: [
									...conversation.messages,
									{
										id: result.messageId,
										from_actor: viewerActorId,
										to_actor: actorId,
										body,
										attachments: [],
										details: null,
										intent: null,
										context_path: contextPath ?? null,
										created_at: sentAt.toISOString(),
										read_at: null,
										account_uses: []
									}
								]
							}
						: conversation
				);
			}
			if (pending?.id === optimisticId) {
				pending.clientKey = optimisticId;
				pending.id = String(result.messageId);
			}
			if (queryEnabled(followLiveActivity)) follow(result.messageId, sentAt);
			void client.invalidateQueries({ queryKey: key });
			void client.invalidateQueries({ queryKey: ['recent-direct-conversations', companyId] });
			return result;
		}
	};
}

/** A bounded SSE-only projection for Work inspection. Durable Work data stays
 * in its TanStack query; this object has no cache and is discarded on unmount. */
export function workActivityStream(companyId: string, actorId: string, workId: string) {
	let live = $state<AgentActivityState | null>(null);
	let transport = $state<ActivityTransport>('idle');
	let stop: (() => void) | null = null;
	return {
		get live() {
			return live;
		},
		get transport() {
			return transport;
		},
		attach(): () => void {
			stop?.();
			transport = 'connecting';
			stop = openAgentActivityStream(
				companyId,
				actorId,
				{ workId },
				(state) => {
					live = state;
					transport = 'live';
				},
				() => (transport = 'reconnecting')
			);
			return () => {
				stop?.();
				stop = null;
				transport = 'idle';
			};
		}
	};
}

/* An open Room has its own exact event stream (room-queries.svelte.ts) with
 * snapshot and replay; refetching its messages here as well only doubles the
 * same request and cancels one of them. */
const ROOM_STREAM_OWNED = new Set(['room-messages', 'room-thread']);
const HINT_GATHER_MS = 1_500;

/**
 * One change stream per open company. Each hint refetches the company's
 * queries that are on screen — a single rule, cheap because hints are
 * coalesced server-side and only active queries refetch. EventSource
 * reconnects by itself; while it is down, polling returns to its ordinary pace.
 */
export function companyChangeStream(companyId: () => string, enabled: QueryEnabled = true) {
	const client = useQueryClient();
	$effect(() => {
		const company = companyId();
		if (!company || !queryEnabled(enabled) || typeof EventSource === 'undefined') return;
		const source = openLiveEvents(`/api/companies/${encodeURIComponent(company)}/changes`);
		/* The stream ends when the session does (a role change, removal, an update). Ask at once
		 * whether this person may still be here, so a lapsed session re-enters within a second
		 * instead of at the next slow poll; once it is back, catch up on any hint it missed. */
		let dropped = false;
		source.onopen = () => {
			changes.live = true;
			if (!dropped) return;
			dropped = false;
			void client.invalidateQueries({
				predicate: (query) =>
					query.queryKey.includes(company) &&
					!ROOM_STREAM_OWNED.has(String(query.queryKey[0])) &&
					hintTouches(['resync'], query.queryKey),
				refetchType: 'active'
			});
		};
		source.onerror = () => {
			changes.live = false;
			if (dropped) return;
			dropped = true;
			void client.invalidateQueries({ queryKey: queryKeys.principal(company) });
		};
		/* A send and its reply, or a Work commission's several rows, arrive as a burst of hints;
		 * each refetch of the screen is a dozen requests through the edge. So hints are gathered
		 * for a moment and refetched once. A hidden tab only marks them stale, and catches up
		 * once when it is shown (refetchOnWindowFocus). */
		let pending = new Set<string>();
		let flush: ReturnType<typeof setTimeout> | undefined;
		source.addEventListener('change', (event) => {
			/* A hint names what kind of thing changed, and only queries that kind can affect
			 * refetch: a teammate's message never reloads settings, the principal or model
			 * connections. An open Room's messages stay with the Room's own stream. A bare
			 * `room_event` is not "only rooms": a message to the Exec is one too. */
			for (const kind of parseHint((event as MessageEvent).data)) pending.add(kind);
			if (flush) return;
			flush = setTimeout(() => {
				flush = undefined;
				const kinds = [...pending];
				pending = new Set();
				void client.invalidateQueries({
					predicate: (query) =>
						query.queryKey.includes(company) &&
						!ROOM_STREAM_OWNED.has(String(query.queryKey[0])) &&
						hintTouches(kinds, query.queryKey),
					refetchType: document.hidden ? 'none' : 'active'
				});
			}, HINT_GATHER_MS);
		});
		return () => {
			clearTimeout(flush);
			source.close();
			changes.live = false;
		};
	});
}
