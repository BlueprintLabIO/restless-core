const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

export type DocumentCollaborationAccess = 'read' | 'write';
export type DocumentCollaborationState =
	'connecting' | 'synced' | 'reconnecting' | 'degraded' | 'read-only';

export type DocumentCollaborationEvent =
	'start' | 'socket-connecting' | 'socket-disconnected' | 'synced-write' | 'synced-read' | 'failed';

export interface DocumentCollaborationGrant {
	token: string;
	token_type: 'Bearer';
	access: DocumentCollaborationAccess;
	expires_at: string;
}

export class DocumentCollaborationError extends Error {
	readonly status: number | null;
	readonly code: string;

	constructor(code: string, message: string, status: number | null = null) {
		super(message);
		this.name = 'DocumentCollaborationError';
		this.code = code;
		this.status = status;
	}
}

function invalid(message: string): never {
	throw new DocumentCollaborationError('invalid_document_collaboration', message);
}

function record(value: unknown): Record<string, unknown> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) {
		invalid('The collaboration-token response is invalid.');
	}
	return value as Record<string, unknown>;
}

export function canonicalDocumentUuid(value: string, label: string): string {
	if (!CANONICAL_UUID.test(value) || value === '00000000-0000-0000-0000-000000000000') {
		invalid(`${label} must be a canonical non-nil UUID.`);
	}
	return value;
}

export function documentCollaborationName(companyUuid: string, documentUuid: string): string {
	return `${canonicalDocumentUuid(companyUuid, 'Company identity')}:${canonicalDocumentUuid(
		documentUuid,
		'Document identity'
	)}`;
}

/** Build the one browser-reachable route. It deliberately cannot select another host. */
export function documentCollaborationWebSocketUrl(
	origin: string,
	companyUuid: string,
	documentUuid: string
): string {
	let url: URL;
	try {
		url = new URL(origin);
	} catch {
		invalid('The owner-plane origin is invalid.');
	}
	if (url.protocol !== 'http:' && url.protocol !== 'https:') {
		invalid('Document collaboration requires an HTTP owner-plane origin.');
	}
	if (url.username || url.password) invalid('The owner-plane origin must not contain credentials.');
	url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
	url.pathname = `/api/companies/${canonicalDocumentUuid(
		companyUuid,
		'Company identity'
	)}/documents/${canonicalDocumentUuid(documentUuid, 'Document identity')}/collaboration`;
	url.search = '';
	url.hash = '';
	return url.toString();
}

export function transitionDocumentCollaborationState(
	current: DocumentCollaborationState,
	event: DocumentCollaborationEvent
): DocumentCollaborationState {
	switch (event) {
		case 'start':
			return 'connecting';
		case 'socket-connecting':
			return current === 'synced' || current === 'read-only' || current === 'reconnecting'
				? 'reconnecting'
				: 'connecting';
		case 'socket-disconnected':
			return current === 'degraded'
				? 'degraded'
				: current === 'connecting'
					? 'connecting'
					: 'reconnecting';
		case 'synced-write':
			return 'synced';
		case 'synced-read':
			return 'read-only';
		case 'failed':
			return 'degraded';
	}
}

export function documentCollaborationStateLabel(state: DocumentCollaborationState): string {
	switch (state) {
		case 'connecting':
			return 'Connecting';
		case 'synced':
			return 'Synced with collaborators';
		case 'reconnecting':
			return 'Reconnecting';
		case 'degraded':
			return 'Collaboration unavailable';
		case 'read-only':
			return 'Read only';
	}
}

async function responseFailure(response: Response): Promise<DocumentCollaborationError> {
	let message = `${response.status} ${response.statusText}`;
	let code = 'document_collaboration_unavailable';
	try {
		const body = record(await response.json());
		if (typeof body.message === 'string' && body.message) message = body.message;
		if (typeof body.error === 'string' && body.error) code = body.error;
		if (typeof body.code === 'string' && body.code) code = body.code;
	} catch {
		// Intermediaries are allowed to return non-JSON failures; retain the status.
	}
	return new DocumentCollaborationError(code, message, response.status);
}

export async function requestDocumentCollaborationToken(
	companyHandle: string,
	documentUuid: string,
	signal?: AbortSignal,
	now = Date.now()
): Promise<DocumentCollaborationGrant> {
	if (!companyHandle.trim() || companyHandle.length > 256) {
		invalid('The company route handle is invalid.');
	}
	canonicalDocumentUuid(documentUuid, 'Document identity');
	const response = await fetch(
		`/api/companies/${encodeURIComponent(companyHandle)}/documents/${encodeURIComponent(
			documentUuid
		)}/collaboration/token`,
		{
			method: 'POST',
			credentials: 'same-origin',
			cache: 'no-store',
			signal
		}
	);
	if (!response.ok) throw await responseFailure(response);
	const body = record(await response.json());
	if (
		typeof body.token !== 'string' ||
		body.token.length < 1 ||
		body.token.length > 16_384 ||
		body.token_type !== 'Bearer' ||
		(body.access !== 'read' && body.access !== 'write') ||
		typeof body.expires_at !== 'string'
	) {
		invalid('The collaboration-token response is invalid.');
	}
	const expiresAt = Date.parse(body.expires_at);
	if (!Number.isFinite(expiresAt) || expiresAt <= now) {
		invalid('The collaboration token is already expired.');
	}
	return {
		token: body.token,
		token_type: 'Bearer',
		access: body.access,
		expires_at: body.expires_at
	};
}
