/**
 * Owner-facing server state belongs to TanStack Query. The daemon remains the
 * writer; these helpers only choose cache keys, freshness and invalidation.
 *
 * Live ACP activity is intentionally separate from durable records: SSE is a
 * tiny overlay that invalidates the relevant query when the turn settles.
 */

import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { onMount } from 'svelte';
import type { QueryClient } from '@tanstack/svelte-query';
import {
	getActorConversation,
	getAttention,
	openAgentActivityStream,
	sendActorMessage,
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
import { getBrowserStatus, getCompany, type BrowserStatus, type CompanyView } from './company';
import { getCompanyIdentity, type CompanyIdentitySnapshot } from './identity';
import { getCompanyPrincipal, type CompanyPrincipal } from './query-persistence';
import type { ThreadMessage } from './view';
import { changes, pollEvery } from './connection.svelte';

export type QuerySourceStatus = 'unknown' | 'live' | 'stale';
export type ActivityTransport = 'idle' | 'connecting' | 'live' | 'reconnecting';
type QueryEnabled = boolean | (() => boolean);

const STALE_MS = 5_000;
const REFRESH_MS = 10_000;
const RETAIN_MS = 10 * 60_000;

function queryEnabled(value: QueryEnabled): boolean {
	return typeof value === 'function' ? value() : value;
}

export const queryKeys = {
	companies: ['companies'] as const,
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
	cockpit: (company: string) => ['cockpit', company] as const,
	company: (company: string, probeCredentials: boolean) =>
		['company', company, { probeCredentials }] as const,
	identity: (company: string) => ['company-identity', company] as const,
	conversation: (company: string, actor: string, workId?: string) =>
		['conversation', company, actor, workId ?? null] as const,
	browserStatus: (company: string) => ['browser-status', company] as const
};

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
	return query.isError ? (query.data ? 'stale' : 'unknown') : 'live';
}

function refresh<T>(query: { refetch: () => Promise<T> }): Promise<T> {
	return query.refetch();
}

export function attentionQuery(companyId: string | (() => string), enabled: QueryEnabled = true) {
	const currentCompany = () => (typeof companyId === 'function' ? companyId() : companyId);
	const client = useQueryClient();
	onMount(() => {
		if (typeof BroadcastChannel === 'undefined') return;
		const channel = new BroadcastChannel('restless-attention');
		channel.onmessage = (event) => {
			if (event.data === currentCompany() && queryEnabled(enabled)) {
				void client.invalidateQueries({ queryKey: queryKeys.attention(currentCompany()) });
			}
		};
		return () => channel.close();
	});
	const query = createQuery(() => ({
		queryKey: queryKeys.attention(currentCompany()),
		queryFn: ({ queryKey }) => getAttention(queryKey[1]),
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: pollEvery(REFRESH_MS, 30_000),
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return query.data ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refreshAttention(client, currentCompany()),
		// An explicit open must perform a read even while cached observers settle.
		reload: () => refresh(query)
	};
}

/** One server-owned queue, refreshed in this window and other open cockpit tabs. */
export async function refreshAttention(client: QueryClient, companyId: string) {
	if (typeof BroadcastChannel !== 'undefined') {
		const channel = new BroadcastChannel('restless-attention');
		channel.postMessage(companyId);
		channel.close();
	}
	await client.invalidateQueries({ queryKey: queryKeys.attention(companyId) });
}

/** A confirmed decision leaves the queue without waiting for a full company read. */
export async function removeConfirmedAttention(
	client: QueryClient,
	companyId: string,
	itemId: string
) {
	const queryKey = queryKeys.attention(companyId);
	// An older in-flight read must not put the decided card back into the queue.
	await client.cancelQueries({ queryKey });
	client.setQueryData<AttentionView>(
		queryKey,
		(view) =>
			view && {
				...view,
				items: view.items.filter((item) => item.id !== itemId)
			}
	);
	void refreshAttention(client, companyId);
}

export function companiesQuery(enabled: QueryEnabled = true) {
	const query = createQuery(() => ({
		queryKey: queryKeys.companies,
		queryFn: getCompanies,
		enabled: queryEnabled(enabled),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		// The list carries each company's card, so it refreshes as often as the portfolio did.
		refetchInterval: 30_000,
		refetchIntervalInBackground: false
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
		queryFn: ({ queryKey }) => getCockpit(queryKey[1]),
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
		queryFn: () => getCompany(companyId, probe),
		enabled: queryEnabled(enabled) && on(),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: 60_000,
		refetchIntervalInBackground: false
	});
	/* The ordinary read always runs. A page that asks for a live credential
	 * probe (which can take seconds) shows the ordinary read — where every
	 * reference says it is unprobed — until the probe answers. */
	const plain = createQuery(() => options(false, () => true));
	const probed = createQuery(() => options(true, () => probeCredentials));
	const current = () => (probeCredentials && (probed.data || probed.error) ? probed : plain);
	return {
		get view() {
			return ((probed.data ?? plain.data) as CompanyView | undefined) ?? null;
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
			client.setQueryData(queryKeys.company(companyId, false), view);
		},
		attach(probe = false): () => void {
			if (probe) probeCredentials = true;
			return () => {};
		}
	};
}

export function identityQuery(companyId: string) {
	const query = createQuery(() => ({
		queryKey: queryKeys.identity(companyId),
		queryFn: () => getCompanyIdentity(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
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
		refetchInterval: REFRESH_MS,
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
		contextPath: message.context_path ?? null,
		readAt: message.read_at ?? null
	};
}

export interface ActiveAgentTurn {
	triggerMessageId: number;
	since: Date | string;
	live: AgentActivityState | null;
	transport: ActivityTransport;
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
	followLiveActivity: QueryEnabled = true
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
	let followingMessageId = $state<number | null>(null);
	let stop: (() => void) | null = null;
	let uncertainCommand: {
		id: string;
		body: string;
		files: File[];
		contextPath?: string;
		newFocus: boolean;
		interrupt: boolean;
		outcomeStandard?: import('./company').OutcomeStandard;
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
					(state.phase === 'complete' || state.phase === 'failed') &&
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
					live?.phase !== 'failed'
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
			pending = null;
			if (live?.phase === 'complete' || live?.phase === 'failed') {
				stop?.();
				stop = null;
				followingMessageId = null;
				transport = 'idle';
			}
		}
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
			const messages = (conversation?.messages ?? []).map((message) =>
				threadMessage(message, actorDisplay, viewerActorId)
			);
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
		get activeTurn(): ActiveAgentTurn | null {
			if (!queryEnabled(followLiveActivity)) return null;
			const current = live;
			const messageId = current?.triggerMessageId ?? followingMessageId ?? Number(pending?.id);
			if (!Number.isFinite(messageId)) return null;
			return {
				triggerMessageId: messageId,
				since: current?.startedAt ?? pending?.createdAt ?? new Date(),
				live: current,
				transport
			};
		},
		refresh: () => refresh(query),
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
			outcomeStandard?: import('./company').OutcomeStandard,
			skills: string[] = []
		): Promise<MessageSendResult> {
			const sameUncertainIntent =
				uncertainCommand?.body === body &&
				uncertainCommand.skills.join(' ') === skills.join(' ') &&
				uncertainCommand.contextPath === contextPath &&
				uncertainCommand.newFocus === newFocus &&
				uncertainCommand.interrupt === interrupt &&
				uncertainCommand.outcomeStandard === outcomeStandard &&
				uncertainCommand.files.length === files.length &&
				uncertainCommand.files.every((file, index) => file === files[index]);
			const clientCommandId = sameUncertainIntent ? uncertainCommand!.id : crypto.randomUUID();
			uncertainCommand = {
				id: clientCommandId,
				body,
				files: [...files],
				contextPath,
				newFocus,
				interrupt,
				outcomeStandard,
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
				attachments: [],
				contextPath: contextPath ?? null
			};
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
					outcomeStandard,
					attentionId,
					clientCommandId,
					skills
				);
				uncertainCommand = null;
			} catch (error) {
				if (pending?.id === optimisticId) pending = null;
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
			}
			if (pending?.id === optimisticId) pending.id = String(result.messageId);
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

export function invalidateCompany(client: QueryClient, companyId: string): Promise<void> {
	return Promise.all([
		client.invalidateQueries({ queryKey: queryKeys.attention(companyId) }),
		client.invalidateQueries({ queryKey: queryKeys.cockpit(companyId) }),
		client.invalidateQueries({ queryKey: ['company-collaboration', companyId] }),
		client.invalidateQueries({ queryKey: queryKeys.company(companyId, false) })
	]).then(() => undefined);
}

/* An open Room has its own exact event stream (room-queries.svelte.ts) with
 * snapshot and replay; refetching its messages here as well only doubles the
 * same request and cancels one of them. */
const ROOM_STREAM_OWNED = new Set(['room-messages', 'room-thread']);

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
		const source = new EventSource(`/api/companies/${encodeURIComponent(company)}/changes`);
		source.onopen = () => (changes.live = true);
		source.onerror = () => (changes.live = false);
		source.addEventListener('change', () => {
			/* Every kind refreshes every active query for the company, except
			 * an open Room's messages, which the Room's own stream owns. A
			 * bare `room_event` is not "only rooms": a message to the Exec is
			 * one too, and the Exec rail must see it. */
			void client.invalidateQueries({
				predicate: (query) =>
					query.queryKey.includes(company) && !ROOM_STREAM_OWNED.has(String(query.queryKey[0])),
				refetchType: 'active'
			});
		});
		return () => {
			source.close();
			changes.live = false;
		};
	});
}
