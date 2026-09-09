import type { InfiniteData, QueryClient } from '@tanstack/query-core';
import type { DocumentListCursor, DocumentListPage } from './documents';

export interface DocumentTarget {
	companyId: string;
	documentId: string;
}

export interface DocumentThreadTarget extends DocumentTarget {
	threadId: string;
}

interface StorageLike {
	readonly length: number;
	getItem(key: string): string | null;
	key(index: number): string | null;
	removeItem(key: string): void;
}

const DOCUMENT_QUERY_FAMILIES = new Set([
	'document-principal',
	'document',
	'document-versions',
	'document-version',
	'document-comment-threads',
	'document-comments',
	'document-reviews',
	'document-proposals',
	'document-proposal'
]);

const deniedCompanies = new Set<string>();
const deniedDocuments = new Set<string>();
const companyDenialGeneration = new Map<string, number>();
const documentDenialGeneration = new Map<string, number>();

const deniedDocumentKey = (companyId: string, documentId: string): string =>
	`${companyId}\u0000${documentId}`;

export interface DocumentAccessToken {
	companyGeneration: number;
	documentGeneration: number;
}

const draftPrefix = (companyId: string): string =>
	`restless:document-draft:v2:${encodeURIComponent(companyId)}:`;

export function documentDraftStorageKey(
	companyId: string,
	actorId: string,
	cachePartition: string,
	documentId: string
): string {
	return `${draftPrefix(companyId)}${encodeURIComponent(actorId)}:${encodeURIComponent(cachePartition)}:${encodeURIComponent(documentId)}`;
}

export function isAuthoritativeDocumentFailure(error: unknown): boolean {
	const status =
		typeof error === 'object' && error !== null && 'status' in error
			? (error as { status?: unknown }).status
			: undefined;
	return status === 401 || status === 403 || status === 404;
}

export function sameDocumentTarget(
	target: DocumentTarget,
	companyId: string,
	documentId: string
): boolean {
	return target.companyId === companyId && target.documentId === documentId;
}

export function documentAccessToken(
	companyId: string,
	documentId: string | null = null
): DocumentAccessToken {
	return {
		companyGeneration: companyDenialGeneration.get(companyId) ?? 0,
		documentGeneration: documentId
			? (documentDenialGeneration.get(deniedDocumentKey(companyId, documentId)) ?? 0)
			: 0
	};
}

export function allowDocumentDraftPersistence(
	companyId: string,
	documentId: string | null,
	token: DocumentAccessToken
): boolean {
	const current = documentAccessToken(companyId, documentId);
	if (
		current.companyGeneration !== token.companyGeneration ||
		current.documentGeneration !== token.documentGeneration
	) {
		return false;
	}
	deniedCompanies.delete(companyId);
	if (documentId) deniedDocuments.delete(deniedDocumentKey(companyId, documentId));
	return true;
}

export function canPersistDocumentDraft(companyId: string, documentId: string): boolean {
	return (
		!deniedCompanies.has(companyId) &&
		!deniedDocuments.has(deniedDocumentKey(companyId, documentId))
	);
}

export function purgeDocumentDraftStorage(
	companyId: string,
	documentId: string | null = null,
	storage: StorageLike | null = typeof localStorage === 'undefined' ? null : localStorage
): void {
	if (!storage) return;
	const prefix = draftPrefix(companyId);
	const legacyPrefix = `restless:document-draft:${companyId}:`;
	const suffix = documentId ? `:${encodeURIComponent(documentId)}` : '';
	for (let index = storage.length - 1; index >= 0; index -= 1) {
		const key = storage.key(index);
		if (!key) continue;
		const current = key.startsWith(prefix) && (!suffix || key.endsWith(suffix));
		const legacy = key.startsWith(legacyPrefix) && (!documentId || key.endsWith(`:${documentId}`));
		if (current || legacy) storage.removeItem(key);
	}
}

function documentQueryForCompany(queryKey: readonly unknown[], companyId: string): boolean {
	return (
		typeof queryKey[0] === 'string' &&
		(DOCUMENT_QUERY_FAMILIES.has(queryKey[0]) || queryKey[0] === 'documents') &&
		queryKey[1] === companyId
	);
}

export function purgeDeniedDocumentState(
	client: QueryClient,
	companyId: string,
	documentId: string | null = null,
	storage?: StorageLike | null
): void {
	if (documentId) {
		client.setQueryData<InfiniteData<DocumentListPage, DocumentListCursor | null>>(
			['documents', companyId],
			(current) =>
				current
					? {
							...current,
							pages: current.pages.map((page) => ({
								...page,
								items: page.items.filter((item) => item.document.id !== documentId)
							}))
						}
					: current
		);
		client.removeQueries({
			predicate: (query) =>
				documentQueryForCompany(query.queryKey, companyId) &&
				query.queryKey[0] !== 'documents' &&
				query.queryKey[2] === documentId
		});
	} else {
		client.removeQueries({
			predicate: (query) => documentQueryForCompany(query.queryKey, companyId)
		});
	}
	purgeDocumentDraftStorage(companyId, documentId, storage);
}

export function failClosedDocumentRead(
	client: QueryClient,
	error: unknown,
	companyId: string,
	documentId: string | null = null,
	storage?: StorageLike | null
): boolean {
	if (!isAuthoritativeDocumentFailure(error)) return false;
	const status = (error as { status: number }).status;
	if (status === 401 || status === 403) {
		deniedCompanies.add(companyId);
		companyDenialGeneration.set(companyId, (companyDenialGeneration.get(companyId) ?? 0) + 1);
	} else if (documentId) {
		const key = deniedDocumentKey(companyId, documentId);
		deniedDocuments.add(key);
		documentDenialGeneration.set(key, (documentDenialGeneration.get(key) ?? 0) + 1);
	}
	purgeDeniedDocumentState(
		client,
		companyId,
		status === 401 || status === 403 ? null : documentId,
		storage
	);
	return true;
}
