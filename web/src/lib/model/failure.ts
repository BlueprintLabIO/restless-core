/**
 * One reading of every owner-API failure.
 *
 * Requests fail in a small, enumerable number of ways, so the kind is computed
 * from the HTTP status and transport, once, here. What the owner reads is then
 * chosen from the kind rather than from whatever text an intermediary returned:
 * "502 Bad Gateway" and "Failed to fetch" never reach a screen.
 *
 * The daemon's own `message` is written for the owner and is kept whenever the
 * failure is about the request (rejected, conflict, access, missing,
 * unavailable). A 5xx message may describe internals, so it moves behind a
 * detail tooltip instead of becoming the headline.
 *
 * Dependency-free on purpose: node's type-stripping test runner imports the
 * model modules that use it, so they import it as './failure.ts'.
 */

export type FailureKind =
	| 'offline'
	| 'unreachable'
	| 'signed_out'
	| 'access'
	| 'missing'
	| 'conflict'
	| 'busy'
	| 'rejected'
	| 'unavailable'
	| 'server'
	| 'local';

export type OwnerFailure = Error & {
	status?: number;
	code?: string | null;
	/** The daemon's JSON message, when it sent one. Absent for proxy or transport text. */
	serverMessage?: string | null;
};

export interface FailureView {
	kind: FailureKind;
	title: string;
	/** One owner-facing sentence. */
	detail: string;
	/** Supporting text for a tooltip, never the headline. */
	technical: string | null;
	/** Whether trying the same request again can reasonably succeed. */
	retryable: boolean;
}

/** Build the error for a non-OK owner-API response. Reads the body once. */
export async function responseFailure(response: Response): Promise<OwnerFailure> {
	let serverMessage: string | null = null;
	let code: string | null = null;
	try {
		const body = (await response.json()) as {
			message?: unknown;
			error?: { message?: unknown } | unknown;
			code?: unknown;
		};
		const nested = (body.error as { message?: unknown } | null)?.message;
		const said = typeof body.message === 'string' ? body.message : nested;
		if (typeof said === 'string' && said.trim()) serverMessage = said;
		if (typeof body.code === 'string') code = body.code;
	} catch {
		// A proxy or intermediary answered. The status is still honest.
	}
	const message = serverMessage ?? `${response.status} ${response.statusText}`.trim();
	return Object.assign(new Error(message), { status: response.status, code, serverMessage });
}

/** Throw the classified error for a non-OK response, otherwise parse its JSON. */
export async function ownerJson<T>(response: Response): Promise<T> {
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

function isTransportFailure(error: unknown): boolean {
	// fetch() rejects with a TypeError when no response arrived at all. Each
	// engine words it once; any other TypeError is a bug, not a network fault.
	if (error instanceof TypeError)
		return /failed to fetch|networkerror|load failed|network connection was lost/i.test(
			error.message
		);
	return error instanceof DOMException && error.name === 'TimeoutError';
}

export function failureKind(error: unknown): FailureKind {
	if (typeof navigator !== 'undefined' && navigator.onLine === false) return 'offline';
	if (isTransportFailure(error)) return 'unreachable';
	const failure = error as OwnerFailure | null;
	const status = typeof failure?.status === 'number' ? failure.status : null;
	// A plain Error was written by cockpit code for the owner. Anything else
	// without a status (a SyntaxError from a bad body, a TypeError) is a fault.
	if (status === null) return error instanceof Error && error.constructor === Error ? 'local' : 'server';
	if (status === 401) return 'signed_out';
	if (status === 403) return 'access';
	if (status === 404 || status === 410) return 'missing';
	if (status === 409 || status === 412) return 'conflict';
	if (status === 408 || status === 425 || status === 429) return 'busy';
	if (status >= 400 && status < 500) return 'rejected';
	// Without a JSON body the answer came from a proxy, not the daemon: the
	// daemon itself is down or restarting.
	if ((status === 502 || status === 503 || status === 504) && !failure?.serverMessage)
		return 'unreachable';
	if (status === 503) return 'unavailable';
	return 'server';
}

/** Whether an automatic retry can help. Used by the query client. */
export function isRetryable(error: unknown): boolean {
	const kind = failureKind(error);
	return kind === 'unreachable' || kind === 'busy' || kind === 'server' || kind === 'unavailable';
}

/**
 * The owner-facing reading of a failure.
 * `subject` names what failed to load or save ("Attention", "the charter").
 */
export function describeFailure(error: unknown, subject?: string): FailureView {
	const kind = failureKind(error);
	const failure = error as OwnerFailure | null;
	const said = failure?.serverMessage?.trim() || null;
	const local = error instanceof Error && !isTransportFailure(error) ? error.message : null;
	const what = subject ? `Couldn't load ${subject}` : null;
	switch (kind) {
		case 'offline':
			return {
				kind,
				title: "You're offline",
				detail: 'This will refresh when your connection returns.',
				technical: null,
				retryable: true
			};
		case 'unreachable':
			return {
				kind,
				title: 'Connection lost',
				detail: 'Reconnecting automatically.',
				technical: failure?.status ? `HTTP ${failure.status}` : null,
				retryable: true
			};
		case 'signed_out':
			return {
				kind,
				title: 'Signed out',
				detail: said ?? 'Sign in again to continue.',
				technical: null,
				retryable: false
			};
		case 'access':
			return {
				kind,
				title: 'No access',
				detail: said ?? 'Your membership doesn’t include this.',
				technical: null,
				retryable: false
			};
		case 'missing':
			return {
				kind,
				title: subject ? `${capitalise(subject)} not found` : 'Not found',
				detail: said ?? 'It may have been moved or removed.',
				technical: null,
				retryable: false
			};
		case 'conflict':
			return {
				kind,
				title: 'Changed elsewhere',
				detail: said ?? 'Refresh to see the latest, then try again.',
				technical: null,
				retryable: true
			};
		case 'busy':
			return {
				kind,
				title: 'Busy right now',
				detail: said ?? 'Wait a moment, then try again.',
				technical: null,
				retryable: true
			};
		case 'rejected':
			return {
				kind,
				title: what ?? 'Couldn’t do that',
				detail: said ?? 'The request wasn’t accepted.',
				technical: failure?.code ?? null,
				retryable: false
			};
		case 'unavailable':
			return {
				kind,
				title: what ?? 'Temporarily unavailable',
				detail: said ?? 'Try again shortly.',
				technical: null,
				retryable: true
			};
		case 'server':
			return {
				kind,
				title: what ?? 'Something went wrong',
				detail: 'Try again. If it keeps failing, open Doctor.',
				technical: said ?? (failure?.status ? `HTTP ${failure.status}` : null),
				retryable: true
			};
		case 'local':
			return {
				kind,
				title: what ?? 'Something went wrong',
				detail: local ?? 'Try again.',
				technical: null,
				retryable: true
			};
	}
}

/**
 * One sentence for a failed action, where only a line fits (a form, a
 * composer). `fallback` says what did not happen: "The charter was not saved."
 */
export function failureSentence(error: unknown, fallback = 'That didn’t work.'): string {
	const view = describeFailure(error);
	const retry = (sentence: string) =>
		/try again\.?$/i.test(sentence) ? sentence : `${sentence} Try again.`;
	switch (view.kind) {
		case 'offline':
			return `${fallback} You're offline.`;
		case 'unreachable':
			return `${fallback} The connection was lost; try again in a moment.`;
		case 'local':
			return view.detail === 'Try again.' ? fallback : view.detail;
		case 'server':
			return retry(fallback);
		case 'unavailable':
		case 'busy':
			return (error as OwnerFailure).serverMessage ? view.detail : retry(fallback);
		default:
			return view.detail;
	}
}

function capitalise(value: string): string {
	return value.charAt(0).toUpperCase() + value.slice(1);
}
