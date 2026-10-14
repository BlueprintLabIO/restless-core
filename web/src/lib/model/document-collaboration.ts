import type { DocumentContent, DocumentNode } from './documents.ts';

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

/* Edits not yet acknowledged by the documents service are kept on this device too, so closing the
 * tab while the connection is down does not lose them. Only the difference from what the service
 * last acknowledged is kept: applying it again later merges as an ordinary CRDT update and can
 * never duplicate what the service already holds. */
const PENDING_PREFIX = 'restless:document-pending:';
const MAX_PENDING_BYTES = 1_000_000;

function pendingKey(companyUuid: string, documentId: string, actorId: string): string {
	return `${PENDING_PREFIX}${companyUuid}:${documentId}:${actorId}`;
}

export function savePendingDocumentEdits(
	companyUuid: string,
	documentId: string,
	actorId: string,
	update: Uint8Array
): void {
	try {
		const key = pendingKey(companyUuid, documentId, actorId);
		if (update.byteLength < 3 || update.byteLength > MAX_PENDING_BYTES) {
			localStorage.removeItem(key);
			return;
		}
		let binary = '';
		for (const byte of update) binary += String.fromCharCode(byte);
		localStorage.setItem(key, btoa(binary));
	} catch {
		/* No storage here (a private window, a full quota): the edits stay in this tab. */
	}
}

export function loadPendingDocumentEdits(
	companyUuid: string,
	documentId: string,
	actorId: string
): Uint8Array | null {
	try {
		const stored = localStorage.getItem(pendingKey(companyUuid, documentId, actorId));
		if (!stored) return null;
		const binary = atob(stored);
		return Uint8Array.from(binary, (character) => character.charCodeAt(0));
	} catch {
		return null;
	}
}

export function clearPendingDocumentEdits(
	companyUuid: string,
	documentId: string,
	actorId: string
): void {
	try {
		localStorage.removeItem(pendingKey(companyUuid, documentId, actorId));
	} catch {
		/* Nothing was kept. */
	}
}

/* When someone restores an earlier version, the body is replaced and edits this device had not
 * yet synced belong to a body that no longer exists. They are carried over block by block, by
 * each top-level block's id: a block changed or added here is put back into the restored body
 * (in place when the restored body has it, else after the block it followed), and a block
 * removed here is removed there too. Everything else is the restored version. */
export interface StrandedBlockEdits {
	/** Changed or added blocks, in the order they stood here, each with the block before it. */
	blocks: { node: DocumentNode; after: string | null }[];
	removed: string[];
}

function blockIdOf(node: DocumentNode | undefined): string | null {
	const id = node?.attrs?.block_id;
	return typeof id === 'string' && id ? id : null;
}

export function strandedBlockEdits(
	base: DocumentContent,
	local: DocumentContent
): StrandedBlockEdits {
	const before = new Map<string, string>();
	for (const node of base.content) {
		const id = blockIdOf(node);
		if (id) before.set(id, JSON.stringify(node));
	}
	const blocks: StrandedBlockEdits['blocks'] = [];
	const present = new Set<string>();
	let after: string | null = null;
	for (const node of local.content) {
		const id = blockIdOf(node);
		if (!id) continue;
		present.add(id);
		if (before.get(id) !== JSON.stringify(node)) blocks.push({ node, after });
		after = id;
	}
	return { blocks, removed: [...before.keys()].filter((id) => !present.has(id)) };
}

/** The restored body with the stranded edits put back, and how many of them changed it. */
export function reapplyBlockEdits(
	restored: DocumentContent,
	edits: StrandedBlockEdits
): { content: DocumentContent; applied: number } {
	const removed = new Set(edits.removed);
	let applied = 0;
	const blocks = restored.content.filter((node) => {
		const gone = removed.has(blockIdOf(node) ?? '');
		if (gone) applied++;
		return !gone;
	});
	const indexOf = (id: string) => blocks.findIndex((node) => blockIdOf(node) === id);
	let lastPlaced = -1;
	for (const { node, after } of edits.blocks) {
		const at = indexOf(blockIdOf(node)!);
		if (at >= 0) {
			if (JSON.stringify(blocks[at]) !== JSON.stringify(node)) {
				blocks[at] = node;
				applied++;
			}
			lastPlaced = at;
			continue;
		}
		const anchor = after === null ? -1 : indexOf(after);
		const place =
			anchor >= 0 || after === null ? anchor + 1 : lastPlaced >= 0 ? lastPlaced + 1 : blocks.length;
		blocks.splice(place, 0, node);
		lastPlaced = place;
		applied++;
	}
	if (!blocks.length) blocks.push({ type: 'paragraph' });
	return { content: { ...restored, content: blocks }, applied };
}
