import type { AttentionItem, DecisionContinuation, MessageIntentReceipt } from './view';
import type { MessageAttachment } from './view';
import type { WorkGraphSnapshot } from './generated/orgintel';

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

export interface ActorConversation {
	actor: { id: string; display: string; role: string };
	focus?: {
		after_message_id: number;
		started_at: string | null;
	} | null;
	messages: Array<{
		id: number;
		from_actor: string;
		to_actor: string | null;
		body: string;
		attachments: MessageAttachment[];
		details?: string | null;
		intent?: MessageIntentReceipt | null;
		context_path?: string | null;
		created_at: string;
	}>;
}

export type ConversationLivePhase =
	'queued' | 'thinking' | 'acting' | 'responding' | 'complete' | 'failed';

export interface ConversationLiveActivity {
	id: string;
	kind: 'thinking' | 'tool' | 'note' | string;
	label: string;
	detail: string;
	status: string;
	replyOffset: number;
}

export interface ConversationLiveState {
	streamId: string;
	sequence: number;
	company: string;
	actorId: string;
	triggerMessageId: number;
	workId?: string | null;
	phase: ConversationLivePhase;
	reply: string;
	generatedOutputTokens?: number | null;
	activity: ConversationLiveActivity[];
	startedAt?: string | null;
	updatedAt: string;
	completedMessageId?: number | null;
	error?: string | null;
}

export interface MessageSendResult {
	messageId: number;
	contextAttached: boolean;
	contextOmitted: boolean;
	focus?: {
		afterMessageId: number;
		startedAt: string | null;
	} | null;
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
		status: 'available' | 'unavailable';
		kind: 'runtime-web';
		label: string;
	};
	actions: AttentionItem['actions'];
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
						status: item.review_target.status,
						kind: item.review_target.kind,
						label: item.review_target.label
					}
				: undefined,
			actions: item.actions,
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
	return response.json();
}

/** Follow the live projection for one durable owner message. EventSource owns
 * reconnection; every event is a complete bounded snapshot, so a dropped delta
 * cannot corrupt the visible reply. */
export function openActorConversationStream(
	company: string,
	actor: string,
	messageId: number,
	onstate: (state: ConversationLiveState) => void,
	onerror?: () => void
): () => void {
	const source = new EventSource(
		`/api/companies/${encodeURIComponent(company)}/actors/${encodeURIComponent(actor)}/conversation/live?message_id=${encodeURIComponent(messageId)}`
	);
	const receive = (event: MessageEvent<string>) => {
		try {
			onstate(JSON.parse(event.data) as ConversationLiveState);
		} catch {
			onerror?.();
		}
	};
	source.addEventListener('conversation', receive as EventListener);
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
	newFocus = false
): Promise<MessageSendResult> {
	const form = new FormData();
	form.set('body', body);
	if (workId) form.set('work_id', workId);
	if (contextPath) form.set('context_path', contextPath);
	if (newFocus) form.set('new_focus', 'true');
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
	const result = (await response.json()) as {
		message_id: number;
		context_attached?: boolean;
		context_omitted?: boolean;
		focus?: { after_message_id: number; started_at: string | null } | null;
	};
	return {
		messageId: result.message_id,
		contextAttached: result.context_attached ?? false,
		contextOmitted: result.context_omitted ?? false,
		focus: result.focus
			? {
					afterMessageId: result.focus.after_message_id,
					startedAt: result.focus.started_at
				}
			: result.focus
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
