/**
 * Owner-facing server state belongs to TanStack Query. The daemon remains the
 * writer; these helpers only choose cache keys, freshness and invalidation.
 *
 * Live ACP activity is intentionally separate from durable records: SSE is a
 * tiny overlay that invalidates the relevant query when the turn settles.
 */

import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { QueryClient } from '@tanstack/svelte-query';
import {
	getActorConversation,
	getAttention,
	openAgentActivityStream,
	sendActorMessage,
	type ActorConversation,
	type AgentActivityState,
	type MessageSendResult
} from './attention';
import { getCockpit, getCompanies, type CockpitView, type CompanyCatalogEntry } from './cockpit';
import { getBrowserStatus, getCompany, type BrowserStatus, type CompanyView } from './company';
import { getCompanyIdentity, type CompanyIdentitySnapshot } from './identity';
import type { ThreadMessage } from './view';

export type QuerySourceStatus = 'unknown' | 'live' | 'stale';
export type ActivityTransport = 'idle' | 'connecting' | 'live' | 'reconnecting';

const STALE_MS = 5_000;
const REFRESH_MS = 10_000;
const RETAIN_MS = 10 * 60_000;

export const queryKeys = {
	companies: ['companies'] as const,
	portfolio: ['portfolio'] as const,
	attention: (company: string) => ['attention', company] as const,
	cockpit: (company: string) => ['cockpit', company] as const,
	company: (company: string, probeCredentials: boolean) =>
		['company', company, { probeCredentials }] as const,
	identity: (company: string) => ['company-identity', company] as const,
	conversation: (company: string, actor: string, workId?: string) =>
		['conversation', company, actor, workId ?? null] as const,
	browserStatus: (company: string) => ['browser-status', company] as const
};

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

export function attentionQuery(companyId: string) {
	const query = createQuery(() => ({
		queryKey: queryKeys.attention(companyId),
		queryFn: () => getAttention(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
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
		refresh: () => refresh(query)
	};
}

export function companiesQuery() {
	const query = createQuery(() => ({
		queryKey: queryKeys.companies,
		queryFn: getCompanies,
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
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

export type PortfolioProjection = {
	attentionCount: number | null;
	nextProof: string | null;
	nextProofDetail: string;
	spendAccounted: number | null;
};

export type PortfolioView = {
	companies: CompanyCatalogEntry[];
	projections: Record<string, PortfolioProjection>;
};

async function getPortfolio(client: QueryClient): Promise<PortfolioView> {
	const companies = await client.fetchQuery({
		queryKey: queryKeys.companies,
		queryFn: getCompanies,
		staleTime: STALE_MS
	});
	const active = companies.filter((company) => company.lifecycle_status === 'active');
	const entries = await Promise.all(
		active.map(async (company): Promise<[string, PortfolioProjection]> => {
			const [cockpitResult, attentionResult] = await Promise.allSettled([
				client.fetchQuery({
					queryKey: queryKeys.cockpit(company.id),
					queryFn: () => getCockpit(company.id),
					staleTime: STALE_MS
				}),
				client.fetchQuery({
					queryKey: queryKeys.attention(company.id),
					queryFn: () => getAttention(company.id),
					staleTime: STALE_MS
				})
			]);
			const cockpit = cockpitResult.status === 'fulfilled' ? cockpitResult.value : null;
			const attention = attentionResult.status === 'fulfilled' ? attentionResult.value : null;
			const work = attention?.workGraph?.work ?? [];
			const next =
				work.find((item) => item.status === 'active') ??
				work.find((item) => item.status === 'blocked') ??
				work.find((item) => item.status === 'proposed') ??
				null;
			return [
				company.id,
				{
					attentionCount: attention ? attention.items.length : null,
					nextProof: next?.title ?? null,
					nextProofDetail: next
						? next.expected_artifact || next.outcome || workState(next.status)
						: attention
							? 'No open Work is recorded.'
							: 'Work projection unavailable.',
					spendAccounted: cockpit?.spend.accounted_usd ?? null
				}
			];
		})
	);
	return { companies, projections: Object.fromEntries(entries) };
}

function workState(value: string): string {
	return value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function portfolioQuery() {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: queryKeys.portfolio,
		queryFn: () => getPortfolio(client),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
	}));
	return {
		get view() {
			return (query.data as PortfolioView | undefined) ?? null;
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

export function cockpitQuery(companyId: string) {
	const query = createQuery(() => ({
		queryKey: queryKeys.cockpit(companyId),
		queryFn: () => getCockpit(companyId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
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

export function companyQuery(companyId: string) {
	const client = useQueryClient();
	let probeCredentials = $state(false);
	const query = createQuery(() => ({
		queryKey: queryKeys.company(companyId, probeCredentials),
		queryFn: () => getCompany(companyId, probeCredentials),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
	}));
	return {
		get view() {
			return (query.data as CompanyView | undefined) ?? null;
		},
		get status() {
			return statusOf(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => refresh(query),
		accept(view: CompanyView): void {
			client.setQueryData(queryKeys.company(companyId, probeCredentials), view);
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
		refetchIntervalInBackground: true,
		retry: 1
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
		refetchIntervalInBackground: true,
		retry: 1
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
	actorDisplay: string
): ThreadMessage {
	return {
		id: String(message.id),
		from: message.from_actor === 'owner' ? 'you' : 'agent',
		author: message.from_actor === 'owner' ? 'You' : actorDisplay,
		text: message.body,
		createdAt: message.created_at,
		replyToMessageId: null,
		assetId: null,
		runId: null,
		attachments: message.attachments ?? [],
		details: message.details ?? null,
		intent: message.intent ?? null,
		contextPath: message.context_path ?? null
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
export function conversationQuery(companyId: string, actorId: string, workId?: string) {
	const client = useQueryClient();
	const key = queryKeys.conversation(companyId, actorId, workId);
	const query = createQuery(() => ({
		queryKey: key,
		queryFn: () => getActorConversation(companyId, actorId, workId),
		staleTime: STALE_MS,
		gcTime: RETAIN_MS,
		refetchInterval: REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: 1
	}));

	let live = $state<AgentActivityState | null>(null);
	let transport = $state<ActivityTransport>('idle');
	let pending = $state<ThreadMessage | null>(null);
	let followingMessageId = $state<number | null>(null);
	let stop: (() => void) | null = null;

	const follow = (messageId: number, since: Date | string): void => {
		if (followingMessageId === messageId && stop) return;
		stop?.();
		followingMessageId = messageId;
		transport = 'connecting';
		live = null;
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
				if (state.phase === 'complete' || state.phase === 'failed') {
					void client.invalidateQueries({ queryKey: key });
				}
			},
			() => {
				if (followingMessageId === messageId) transport = 'reconnecting';
			}
		);
		void since;
	};

	$effect(() => {
		const conversation = query.data as ActorConversation | undefined;
		const last = conversation?.messages.at(-1);
		if (last?.from_actor === 'owner') {
			follow(last.id, last.created_at);
			return;
		}
		if (last && last.from_actor !== 'owner') {
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
				threadMessage(message, actorDisplay)
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
			outcomeStandard?: import('./company').OutcomeStandard
		): Promise<MessageSendResult> {
			const result = await sendActorMessage(
				companyId,
				actorId,
				body,
				workId,
				files,
				contextPath,
				newFocus,
				interrupt,
				outcomeStandard
			);
			const sentAt = new Date();
			pending = {
				id: String(result.messageId),
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
			follow(result.messageId, sentAt);
			void client.invalidateQueries({ queryKey: key });
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
		client.invalidateQueries({ queryKey: queryKeys.company(companyId, false) })
	]).then(() => undefined);
}
