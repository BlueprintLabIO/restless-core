import type { AttentionItem } from './view';

export interface AttentionView {
	company: { id: string; name: string; mission: string; model: string };
	sourceHealth: {
		orgintel: string;
		authority: string;
		runtime: string;
		browser: string;
	};
	items: AttentionItem[];
	refreshedAt: string;
}

type WireItem = {
	id: string;
	source: AttentionItem['source'];
	category: string;
	title: string;
	what_happened: string;
	why_it_matters: string;
	recommendation: string;
	requested_action: string;
	if_no_action: string;
	evidence: AttentionItem['evidence'];
	runtime_attach?: {
		company: string;
		generation: string;
		requesting_actor?: string;
		kind: 'persistent-browser';
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
		items: (wire.items as WireItem[]).map((item) => ({
			id: item.id,
			source: item.source,
			category: item.category,
			title: item.title,
			whatHappened: item.what_happened,
			whyItMatters: item.why_it_matters,
			recommendation: item.recommendation,
			requestedAction: item.requested_action,
			ifNoAction: item.if_no_action,
			evidence: item.evidence,
			runtimeAttach: item.runtime_attach
				? {
						company: item.runtime_attach.company,
						generation: item.runtime_attach.generation,
						requestingActor: item.runtime_attach.requesting_actor,
						kind: item.runtime_attach.kind
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
