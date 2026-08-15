/**
 * The transport to `restlessd`.
 *
 * The daemon answers one envelope for everything, success or failure:
 *
 *   { ok: true,  data: ... }
 *   { ok: false, error: { kind, message } }
 *
 * `kind` is the part that matters. An owner who sees `authority` needs to change
 * what they are allowed to do; one who sees `transport` needs to check the
 * daemon. Collapsing both into "something went wrong" throws away the only
 * distinction that tells them what to do next, so this module never flattens it.
 *
 * Three outcomes, not two — "not built yet" is a real answer and must not be
 * mistaken for "empty". See `docs/api/MISSING.md`.
 */

export type Outcome<T> =
	| { state: 'ok'; data: T }
	| { state: 'stub'; what: string }
	| { state: 'failed'; kind: string; message: string };

export interface Envelope<T> {
	ok: boolean;
	data?: T;
	error?: { kind: string; message: string };
	stub?: { implemented: boolean; what: string; see: string };
}

/**
 * Which company this window is looking at. Not a login — there is no auth yet,
 * and pretending otherwise with a fake user menu would be the dishonest kind of
 * placeholder.
 */
export function company(): string {
	if (typeof localStorage !== 'undefined') {
		const stored = localStorage.getItem('company');
		if (stored) return stored;
	}
	return import.meta.env.VITE_RESTLESS_COMPANY ?? 'aris';
}

export function setCompany(name: string): void {
	localStorage?.setItem('company', name);
}

const base = () => `/v1/companies/${encodeURIComponent(company())}`;

async function request<T>(path: string, init?: RequestInit): Promise<Outcome<T>> {
	let response: Response;
	try {
		response = await fetch(path, {
			...init,
			headers: { 'content-type': 'application/json', ...(init?.headers ?? {}) }
		});
	} catch (cause) {
		// The daemon is not answering at all. This is `transport`, and it is a
		// different problem from anything the daemon could have told us.
		return {
			state: 'failed',
			kind: 'transport',
			message: `restlessd is not answering on /v1 — is it running? (${String(cause)})`
		};
	}

	let envelope: Envelope<T>;
	try {
		envelope = await response.json();
	} catch {
		return {
			state: 'failed',
			kind: 'transport',
			message: `${response.status} from ${path}, and the body was not the envelope`
		};
	}

	if (envelope.stub && envelope.stub.implemented === false) {
		return { state: 'stub', what: envelope.stub.what };
	}
	if (envelope.ok && envelope.data !== undefined && envelope.data !== null) {
		return { state: 'ok', data: envelope.data };
	}
	if (envelope.ok) {
		// ok with a null body and no stub marker: treat as absent, not as empty.
		return { state: 'stub', what: 'the daemon answered with no data' };
	}
	return {
		state: 'failed',
		kind: envelope.error?.kind ?? 'error',
		message: envelope.error?.message ?? 'the daemon refused without saying why'
	};
}

// ---- reads that are backed today -------------------------------------------

export const getPeople = () => request<ApiPerson[]>(`${base()}/people`);
export const getGoals = () => request<ApiGoal[]>(`${base()}/goals`);
export const getCommitments = () => request<ApiCommitment[]>(`${base()}/commitments`);
/**
 * **This read consumes.** `GET /inbox` marks every message it returns as read —
 * verified live: unread went 2 → 1 on a single request. Rendering the page
 * therefore destroys the owner's unread state, and a refresh shows nothing.
 *
 * There is no way around it today. `?as=<actor>` inspects without marking, but
 * it matches `to_actor = <actor>` and the owner's own mail is stored with
 * `to_actor IS NULL`, so `?as=owner` returns nothing at all. A non-consuming
 * read of the owner's inbox does not exist — see `docs/api/MISSING.md` §6.
 *
 * The surface says so rather than pretending, because silently eating the
 * owner's attention queue is exactly the failure this product cannot have.
 */
export const getInbox = () => request<ApiMessage[]>(`${base()}/inbox`);
export const getSpend = () => request<ApiSpend>(`${base()}/spend`);
export const getEvents = (limit = 50) => request<ApiEvent[]>(`${base()}/events?limit=${limit}`);
export const getReceipts = (limit = 50) => request<ApiReceipt[]>(`${base()}/receipts?limit=${limit}`);
export const getStatus = () => request<unknown>(`${base()}/status`);

// ---- reads that are stubs (docs/api/MISSING.md) -----------------------------

export const getAuthority = () => request<unknown>(`${base()}/authority`);
export const getAttention = () => request<unknown>(`${base()}/attention`);
export const getPerson = (actor: string) =>
	request<unknown>(`${base()}/people/${encodeURIComponent(actor)}`);
export const getOrg = () => request<unknown>(`${base()}/org`);
export const getArtifacts = (actor: string) =>
	request<unknown>(`${base()}/artifacts?actor=${encodeURIComponent(actor)}`);

// ---- writes ----------------------------------------------------------------

const post = <T>(path: string, body: unknown) =>
	request<T>(path, { method: 'POST', body: JSON.stringify(body) });

/**
 * Writes return `string | null` — an error message, or null on success — which
 * is the shape every component's callback prop expects. The component states the
 * intent; the caller owns the authority.
 */
async function write(path: string, body: unknown): Promise<string | null> {
	const outcome = await post(path, body);
	if (outcome.state === 'ok') return null;
	if (outcome.state === 'stub') return 'not built yet';
	return `[${outcome.kind}] ${outcome.message}`;
}

/**
 * Create a company. The one call that does not take a company from context —
 * there isn't one yet. Writes the config and stops; `up` starts it.
 */
export const createCompany = (input: {
	name: string;
	model: string;
	mission?: string;
	spend_ceiling_usd?: number;
}) => write('/v1/companies', input);

/** Start the company's computer. Separate from creating it, as the daemon is. */
export const startCompany = (name: string) =>
	write(`/v1/companies/${encodeURIComponent(name)}/up`, { reconcile: false });

export const tell = (body: string) => write(`${base()}/tell`, { body });
export const wake = (reason?: string) => write(`${base()}/wake`, { reason: reason ?? null });
export const sendMessage = (to: string | null, body: string) =>
	write(`${base()}/messages`, { to, body });
export const approveParty = (party: string) => write(`${base()}/approvals`, { party });
export const setCommitmentState = (id: string, state: 'completed' | 'blocked', resolution = '') =>
	write(`${base()}/commitments/${encodeURIComponent(id)}/state`, { state, resolution });
export const hire = (input: {
	name: string;
	task: string;
	repo?: string;
	role?: string;
	model?: string;
}) => write(`${base()}/staff`, input);

/** The live event stream. Returns a closer. */
export function openStream(onEvent: (event: ApiEvent) => void, onError: (message: string) => void) {
	const source = new EventSource(`${base()}/stream`);
	source.onmessage = (message) => {
		try {
			onEvent(JSON.parse(message.data));
		} catch {
			/* a malformed frame is not worth tearing the stream down for */
		}
	};
	source.addEventListener('error', () => {
		onError('the event stream dropped — restlessd may be down');
	});
	return () => source.close();
}

// ---- wire shapes, mirroring docs/api/openapi.yaml ---------------------------

export interface ApiPerson {
	actor_id: string;
	role: string;
	display: string;
	model: string | null;
	spent_usd: number;
	session_running: boolean;
}

export interface ApiGoal {
	id: string;
	title: string;
	body: string;
	created_by: string;
	created_at: string;
	closed_at: string | null;
}

export interface ApiCommitment {
	id: string;
	goal_id: string | null;
	owner_id: string;
	title: string;
	body: string;
	/**
	 * Five states, snake_case on the wire — `CommitmentState` in
	 * restless-orgintel. Not three: an earlier version of this file guessed
	 * `open` and the kanban silently rendered nothing, because no commitment is
	 * ever in a state called that.
	 */
	state: 'proposed' | 'active' | 'blocked' | 'completed' | 'abandoned';
	resolution: string;
	created_at: string;
	updated_at: string;
}

export interface ApiMessage {
	id: number;
	from_actor: string;
	to_actor: string | null;
	body: string;
	created_at: string;
	read_at: string | null;
}

export interface ApiEvent {
	id: number;
	kind: string;
	actor_id: string | null;
	body: unknown;
	created_at: string;
}

export interface ApiReceipt {
	capability: string;
	provider: string;
	party: string;
	actor: string;
	outcome: string;
	idempotency_key: string;
	at: string;
}

export interface ApiSpend {
	accounted_usd: number;
	ceiling_usd: number;
	remaining_usd: number | null;
	poisoned: boolean;
	note: string | null;
	by_actor: { actor: string; role: string | null; model: string; spent_usd: number }[];
}
