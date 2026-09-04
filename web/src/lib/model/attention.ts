import type { AttentionItem, DecisionContinuation } from './view';
import type { WorkGraphSnapshot } from './generated/orgintel';
import type { OutcomeStandard } from './company';
import type {
	AgentActivityState,
	ConversationInterruptResponse,
	ConversationSendResponse,
	ConversationView
} from './generated/conversation';

export type {
	AgentActivityPhase,
	AgentActivityItem,
	AgentActivityState,
	AgentContextUsage
} from './generated/conversation';

export interface AttentionView {
	company: { id: string; name: string; mission: string; model: string };
	sourceHealth: {
		orgintel: string;
		authority: string;
		runtime: string;
		browser: string;
	};
	workGraph: WorkGraphSnapshot | null;
	items: AttentionItem[];
	continuations: DecisionContinuation[];
	refreshedAt: string;
}

export type ActorConversation = ConversationView;

export interface MessageSendResult {
	messageId: number;
	interrupted: boolean;
	contextAttached: boolean;
	contextOmitted: boolean;
	requestedOutcomeStandard?: OutcomeStandard | null;
	focus?: {
		afterMessageId: number;
		startedAt: string | null;
	} | null;
}

export interface MessageInterruptResult {
	messageId: number;
	cancelled: boolean;
	interrupted: boolean;
}

type WireItem = {
	id: string;
	work_id?: string;
	source: AttentionItem['source'];
	category: string;
	title: string;
	what_happened: string;
	why_it_matters: string;
	recommendation: string;
	requested_action: string;
	if_no_action: string;
	uncertainty?: string;
	deadline?: string;
	brief_status: string;
	brief_author?: {
		id: string;
		display: string;
		role: string;
	};
	briefed_at?: string;
	evidence: AttentionItem['evidence'];
	review_sources?: Array<{
		label: string;
		provider: string;
		reference: string;
		verification: string;
		uri?: string;
		content: string;
		observed_at: string;
	}>;
	responsible_actor?: {
		id: string;
		display: string;
		role: string;
	};
	runtime_attach?: {
		company: string;
		generation: string;
		requesting_actor?: string;
		requesting_actor_display?: string;
		kind: 'persistent-browser';
	};
	review_target?: {
		company: string;
		generation: string;
		uri: string;
		status: 'available' | 'unavailable';
		kind: 'runtime-web' | 'runtime-text' | 'runtime-file';
		label: string;
		content?: string;
		unavailable_reason?: string;
	};
	actions: Array<{
		id: string;
		label: string;
		role: string;
		consequence: string;
		next_state: string;
		href?: string;
	}>;
	can_continue: boolean;
	created_at: string;
};

type WireContinuation = {
	id: string;
	work_id: string;
	title: string;
	recorded_decision: string;
	what_it_unlocked: string;
	current_state: string;
	observed_outcome: string;
	responsible_actor?: DecisionContinuation['responsibleActor'];
	observed_at: string;
};

export async function getAttention(company: string): Promise<AttentionView> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/attention`, {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await ownerError(response);
	const wire = await response.json();
	return {
		company: wire.company,
		sourceHealth: {
			orgintel: wire.source_health.orgintel,
			authority: wire.source_health.authority,
			runtime: wire.source_health.runtime,
			browser: wire.source_health.browser
		},
		workGraph: (wire.work_graph as WorkGraphSnapshot | undefined) ?? null,
		items: (wire.items as WireItem[]).map((item) => ({
			id: item.id,
			workId: item.work_id,
			source: item.source,
			category: item.category,
			title: item.title,
			whatHappened: item.what_happened,
			whyItMatters: item.why_it_matters,
			recommendation: item.recommendation,
			requestedAction: item.requested_action,
			ifNoAction: item.if_no_action,
			uncertainty: item.uncertainty,
			deadline: item.deadline,
			briefStatus: item.brief_status,
			briefAuthor: item.brief_author,
			briefedAt: item.briefed_at,
			evidence: item.evidence,
			reviewSources: (item.review_sources ?? []).map((source) => ({
				label: source.label,
				provider: source.provider,
				reference: source.reference,
				verification: source.verification,
				uri: source.uri,
				content: source.content,
				observedAt: source.observed_at
			})),
			responsibleActor: item.responsible_actor
				? {
						id: item.responsible_actor.id,
						display: item.responsible_actor.display,
						role: item.responsible_actor.role
					}
				: undefined,
			runtimeAttach: item.runtime_attach
				? {
						company: item.runtime_attach.company,
						generation: item.runtime_attach.generation,
						requestingActor: item.runtime_attach.requesting_actor,
						requestingActorDisplay: item.runtime_attach.requesting_actor_display,
						kind: item.runtime_attach.kind
					}
				: undefined,
			reviewTarget: item.review_target
				? {
						company: item.review_target.company,
						generation: item.review_target.generation,
						uri: item.review_target.uri,
						status: item.review_target.status,
						kind: item.review_target.kind,
						label: item.review_target.label,
						content: item.review_target.content,
						unavailableReason: item.review_target.unavailable_reason
					}
				: undefined,
			actions: item.actions.map((action) => ({
				id: action.id,
				label: action.label,
				role: action.role,
				consequence: action.consequence,
				nextState: action.next_state,
				href: action.href
			})),
			canContinue: item.can_continue,
			createdAt: item.created_at
		})),
		continuations: ((wire.continuations as WireContinuation[] | undefined) ?? []).map(
			(continuation) => ({
				id: continuation.id,
				workId: continuation.work_id,
				title: continuation.title,
				recordedDecision: continuation.recorded_decision,
				whatItUnlocked: continuation.what_it_unlocked,
				currentState: continuation.current_state,
				observedOutcome: continuation.observed_outcome,
				responsibleActor: continuation.responsible_actor,
				observedAt: continuation.observed_at
			})
		),
		refreshedAt: wire.refreshed_at
	};
}

export async function getActorConversation(
	company: string,
	actor: string,
	workId?: string
): Promise<ActorConversation> {
	const query = workId ? `?work_id=${encodeURIComponent(workId)}` : '';
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/actors/${encodeURIComponent(actor)}/conversation${query}`,
		{ credentials: 'same-origin', cache: 'no-store' }
	);
	if (!response.ok) throw await ownerError(response);
	return (await response.json()) as ConversationView;
}

export type AgentActivityScope =
	{ messageId: number; workId?: never } | { workId: string; messageId?: never };

/** Follow one live agent projection. EventSource owns reconnection; every
 * event is a complete snapshot, so a dropped delta cannot corrupt the reply. */
export function openAgentActivityStream(
	company: string,
	actor: string,
	scope: AgentActivityScope,
	onstate: (state: AgentActivityState) => void,
	onerror?: () => void
): () => void {
	const query =
		scope.messageId !== undefined
			? `message_id=${encodeURIComponent(scope.messageId)}`
			: `work_id=${encodeURIComponent(scope.workId!)}`;
	const source = new EventSource(
		`/api/companies/${encodeURIComponent(company)}/actors/${encodeURIComponent(actor)}/activity${query ? `?${query}` : ''}`
	);
	const receive = (event: MessageEvent<string>) => {
		try {
			onstate(JSON.parse(event.data) as AgentActivityState);
		} catch {
			onerror?.();
		}
	};
	source.addEventListener('activity', receive as EventListener);
	source.onerror = () => onerror?.();
	return () => source.close();
}

export async function reviewAction(
	company: string,
	handoff: string,
	decision: 'accept' | 'request_changes',
	feedback = ''
): Promise<void> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/handoffs/${encodeURIComponent(handoff)}/review`,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ decision, feedback }),
			credentials: 'same-origin'
		}
	);
	if (!response.ok) throw await ownerError(response);
}

export async function resolveHandoffDecision(
	company: string,
	handoff: string,
	resolution: string
): Promise<void> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/handoffs/${encodeURIComponent(handoff)}/decision`,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ resolution }),
			credentials: 'same-origin'
		}
	);
	if (!response.ok) throw await ownerError(response);
}

export async function sendActorMessage(
	company: string,
	actor: string,
	body: string,
	workId?: string,
	files: File[] = [],
	contextPath?: string,
	newFocus = false,
	interrupt = false,
	outcomeStandard?: OutcomeStandard,
	attentionId?: string
): Promise<MessageSendResult> {
	const form = new FormData();
	form.set('body', body);
	if (workId) form.set('work_id', workId);
	if (attentionId) form.set('attention_id', attentionId);
	if (contextPath) form.set('context_path', contextPath);
	if (newFocus) form.set('new_focus', 'true');
	if (interrupt) form.set('interrupt', 'true');
	if (outcomeStandard) form.set('outcome_standard', outcomeStandard);
	for (const file of files) form.append('attachments', file, file.name);
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/actors/${encodeURIComponent(actor)}/conversation`,
		{
			method: 'POST',
			body: form,
			credentials: 'same-origin'
		}
	);
	if (!response.ok) throw await ownerError(response);
	const result = (await response.json()) as ConversationSendResponse;
	return {
		messageId: result.message_id,
		interrupted: result.interrupted ?? false,
		contextAttached: result.context_attached ?? false,
		contextOmitted: result.context_omitted ?? false,
		requestedOutcomeStandard: result.requested_outcome_standard,
		focus: result.focus
			? {
					afterMessageId: result.focus.after_message_id,
					startedAt: result.focus.started_at
				}
			: result.focus
	};
}

/** Interrupt one still-unread ordinary conversation request without inserting
 * a second owner message. The terminal client uses the same endpoint for its
 * first Ctrl-C; browser surfaces can opt into the identical semantics. */
export async function interruptActorMessage(
	company: string,
	actor: string,
	messageId: number
): Promise<MessageInterruptResult> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/actors/${encodeURIComponent(actor)}/conversation/${encodeURIComponent(messageId)}/interrupt`,
		{
			method: 'POST',
			credentials: 'same-origin'
		}
	);
	if (!response.ok) throw await ownerError(response);
	const result = (await response.json()) as ConversationInterruptResponse;
	return {
		messageId: result.message_id,
		cancelled: result.cancelled,
		interrupted: result.interrupted
	};
}

/** The exact local cockpit location to associate with a message. Route parsing
 * belongs here rather than in each composer, and the server remains the final
 * company-scope authority. */
export function cockpitContextPath(
	company: string,
	location: Pick<URL, 'pathname' | 'search'>
): string | undefined {
	const encodedCompany = location.pathname.split('/')[1];
	if (!encodedCompany) return undefined;
	let routeCompany: string;
	try {
		routeCompany = decodeURIComponent(encodedCompany);
	} catch {
		return undefined;
	}
	if (routeCompany !== company) return undefined;
	const context = `${location.pathname}${location.search}`;
	return context.length <= 512 ? context : undefined;
}

export async function approvalAction(
	company: string,
	action: 'grant' | 'decline' | 'revoke',
	party: string
): Promise<void> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/approvals/${action}`,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ party }),
			credentials: 'same-origin'
		}
	);
	if (!response.ok) throw await ownerError(response);
}

export async function issueDesktopTicket(
	company: string,
	itemId: string,
	clientId: string
): Promise<string> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/browser/ticket`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ item_id: itemId, client_id: clientId }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await ownerError(response);
	return (await response.json()).desktop_url;
}

export async function issueReviewTicket(company: string, itemId: string): Promise<string> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/reviews/ticket`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ item_id: itemId }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await ownerError(response);
	return (await response.json()).review_url;
}

export async function browserControl(
	company: string,
	action: 'take' | 'heartbeat' | 'return',
	clientId: string
): Promise<unknown> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/browser/${action}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ client_id: clientId }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await ownerError(response);
	return response.json();
}

async function ownerError(response: Response): Promise<Error & { status: number }> {
	let message = `${response.status} ${response.statusText}`;
	try {
		const body = await response.json();
		message = body.message ?? message;
	} catch {
		// The status remains honest when an intermediary supplied a non-JSON body.
	}
	return Object.assign(new Error(message), { status: response.status });
}
