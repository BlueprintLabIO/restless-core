/**
 * The owner's attention queue.
 *
 * This is the one read that does not go through `client.ts`, because it does
 * not go through the CLI's line protocol. `attention::project` is served by the
 * owner gateway at `/api`, which is a deliberately narrow BFF rather than a
 * generic facade — so the envelope here is the projection itself, not the
 * `{ok, data}` wrapper the socket commands share.
 *
 * Keeping the two shapes distinct is the point. A single `Outcome` type over
 * both would have to invent an `ok` field the gateway never sends, and the
 * first thing to rot would be the failure path.
 *
 * ## What the queue is
 *
 * Approvals (Authority) and blocked commitments (OrgIntel), unioned and sorted.
 * It is a **projection** — nothing here can resolve an item. `item.source`
 * names the plane that owns resolution and `item.actions` name what may be
 * done, which is why `grant`/`decline` below post to Authority's routes rather
 * than mutating anything in this module.
 *
 * Messages are deliberately not in it. Mail is read separately and shown
 * separately: the Inbox stack is what is *waiting on a decision*, and folding
 * unread mail into it would make the count mean two different things.
 */

import type { AttentionItem, AttentionView } from '$lib/model/view';

/**
 * Distinguishing "no credential" from "no answer" matters more here than
 * anywhere else in the app. An unauthenticated queue that rendered as empty
 * would tell the owner nothing needs them — the one lie this surface cannot
 * tell — so sign-in is its own outcome and the surface must handle it.
 */
export type AttentionOutcome =
	| { state: 'ok'; view: AttentionView }
	| { state: 'unauthenticated' }
	| { state: 'failed'; message: string };

/** The wire shape, snake_case exactly as `attention.rs` serialises it. */
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

export async function getAttention(company: string): Promise<AttentionOutcome> {
	let response: Response;
	try {
		response = await fetch(`/api/companies/${encodeURIComponent(company)}/attention`, {
			credentials: 'same-origin',
			cache: 'no-store'
		});
	} catch (cause) {
		return {
			state: 'failed',
			message: `the owner gateway is not answering — is restlessd running? (${String(cause)})`
		};
	}
	if (response.status === 401) return { state: 'unauthenticated' };
	if (!response.ok) return { state: 'failed', message: await errorText(response) };

	let wire: {
		company: AttentionView['company'];
		source_health: Record<string, string>;
		items: WireItem[];
		refreshed_at: string;
	};
	try {
		wire = await response.json();
	} catch {
		return { state: 'failed', message: `${response.status}, and the body was not the projection` };
	}

	return {
		state: 'ok',
		view: {
			company: wire.company,
			sourceHealth: {
				orgintel: wire.source_health.orgintel,
				authority: wire.source_health.authority,
				runtime: wire.source_health.runtime,
				browser: wire.source_health.browser
			},
			items: wire.items.map((item) => ({
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
		}
	};
}

/**
 * Exchange the owner token for the session cookie. The token is printed once by
 * `restless owner-token --rotate`; only its digest is stored, so it cannot be
 * recovered from the daemon and is not kept anywhere in this app.
 */
export async function signIn(token: string): Promise<string | null> {
	const response = await fetch('/api/session', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ token }),
		credentials: 'same-origin'
	});
	if (response.ok) return null;
	if (response.status === 401) return 'That credential was not accepted.';
	return await errorText(response);
}

/**
 * Resolve an approval. Posts to Authority's own route — attention is a
 * projection and must not become a second writer of the thing it displays.
 */
export async function approvalAction(
	company: string,
	action: 'grant' | 'decline' | 'revoke',
	party: string
): Promise<string | null> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/approvals/${action}`,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ party }),
			credentials: 'same-origin'
		}
	);
	return response.ok ? null : await errorText(response);
}

/**
 * A ticket for the live company browser, for the prepared last mile. Single-use
 * and short-lived, so the URL it returns must be opened rather than stored.
 */
export async function issueDesktopTicket(
	company: string,
	itemId: string,
	clientId: string
): Promise<{ url: string } | { error: string }> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/browser/ticket`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ item_id: itemId, client_id: clientId }),
		credentials: 'same-origin'
	});
	if (!response.ok) return { error: await errorText(response) };
	return { url: (await response.json()).desktop_url };
}

async function errorText(response: Response): Promise<string> {
	try {
		const body = await response.json();
		return body.message ?? `${response.status} ${response.statusText}`;
	} catch {
		// The status stays honest when an intermediary supplied a non-JSON body.
		return `${response.status} ${response.statusText}`;
	}
}
