import type { AttentionItem, MessageIntentReceipt } from './view';
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
	refreshedAt: string;
}

export interface ActorConversation {
	actor: { id: string; display: string; role: string };
	messages: Array<{
		id: number;
		from_actor: string;
		to_actor: string | null;
		body: string;
		attachments: MessageAttachment[];
		intent?: MessageIntentReceipt | null;
		context_path?: string | null;
		created_at: string;
	}>;
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
		refreshedAt: wire.refreshed_at
	};
}

export async function signIn(token: string): Promise<void> {
	const response = await fetch('/api/session', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ token }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await ownerError(response);
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

export async function sendActorMessage(
	company: string,
	actor: string,
	body: string,
	workId?: string,
	files: File[] = [],
	contextPath?: string
): Promise<{ messageId: number }> {
	const form = new FormData();
	form.set('body', body);
	if (workId) form.set('work_id', workId);
	if (contextPath) form.set('context_path', contextPath);
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
	const result = (await response.json()) as { message_id: number };
	return { messageId: result.message_id };
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
