import type { InfiniteData, QueryClient } from '@tanstack/query-core';
import type { DocumentListCursor, DocumentListPage } from './documents';

export interface DocumentTarget {
	companyId: string;
	documentId: string;
}

export interface DocumentThreadTarget extends DocumentTarget {
	threadId: string;
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
	documentId: string | null = null
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
}

export function failClosedDocumentRead(
	client: QueryClient,
	error: unknown,
	companyId: string,
	documentId: string | null = null
): boolean {
	if (!isAuthoritativeDocumentFailure(error)) return false;
	const status = (error as { status: number }).status;
	purgeDeniedDocumentState(client, companyId, status === 401 || status === 403 ? null : documentId);
	return true;
}
