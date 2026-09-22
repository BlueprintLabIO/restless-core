import { createInfiniteQuery, createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { InfiniteData, QueryClient } from '@tanstack/svelte-query';
import {
	failClosedDocumentRead,
	isAuthoritativeDocumentFailure,
	type DocumentTarget,
	type DocumentThreadTarget
} from './document-cache';
import {
	getDocument,
	getDocumentComments,
	getDocumentCommentThreads,
	getDocumentRevisionProposal,
	getDocumentRevisionProposals,
	getDocumentReviews,
	getDocumentVersion,
	getDocuments,
	getDocumentVersions,
	type DocumentCommentPage,
	type DocumentCommentRow,
	type DocumentCommentThreadCreated,
	type DocumentCommentThreadPage,
	type DocumentCommentThreadRow,
	type DocumentCommentThreadView,
	type DocumentListCursor,
	type DocumentListPage,
	type DocumentPageCursor,
	type DocumentReadView,
	type DocumentRevisionProposalPage,
	type DocumentRevisionProposalRow,
	type DocumentRevisionResolution,
	type DocumentReviewPage,
	type DocumentReviewResolution,
	type DocumentReviewView,
	type DocumentVersionCursor,
	type DocumentVersionPage,
	type DocumentVersionView
} from './documents';
import { getCompanyPrincipal, type CompanyPrincipal } from './query-persistence';

const DOCUMENT_STALE_MS = 5_000;
const DOCUMENT_RETAIN_MS = 10 * 60_000;
const DOCUMENT_REFRESH_MS = 15_000;

type QueryId = string | (() => string);

function queryId(value: QueryId): string {
	return typeof value === 'function' ? value() : value;
}

export const documentQueryKeys = {
	principal: (company: string) => ['document-principal', company] as const,
	list: (company: string) => ['documents', company] as const,
	detail: (company: string, document: string) => ['document', company, document] as const,
	versions: (company: string, document: string) =>
		['document-versions', company, document] as const,
	version: (company: string, document: string, version: string) =>
		['document-version', company, document, version] as const,
	commentThreads: (company: string, document: string) =>
		['document-comment-threads', company, document] as const,
	comments: (company: string, document: string, thread: string) =>
		['document-comments', company, document, thread] as const,
	reviews: (company: string, document: string) => ['document-reviews', company, document] as const,
	proposals: (company: string, document: string) =>
		['document-proposals', company, document] as const,
	proposal: (company: string, document: string, proposal: string) =>
		['document-proposal', company, document, proposal] as const
};

function sourceStatus(query: {
	data?: unknown;
	isPending: boolean;
	isError: boolean;
	error?: unknown;
}): 'unknown' | 'live' | 'stale' {
	if (isAuthoritativeDocumentFailure(query.error)) return 'unknown';
	if (query.isPending && !query.data) return 'unknown';
	return query.isError ? (query.data ? 'stale' : 'unknown') : 'live';
}

function readableData<T>(query: { data?: T; error?: unknown }): T | undefined {
	return isAuthoritativeDocumentFailure(query.error) ? undefined : query.data;
}

function keyPart(queryKey: readonly unknown[], index: number): string {
	const value = queryKey[index];
	return typeof value === 'string' ? value : '';
}

function retryDocumentRead(failureCount: number, error: unknown): boolean {
	return !isAuthoritativeDocumentFailure(error) && failureCount < 1;
}

async function guardedDocumentRead<T>(
	client: QueryClient,
	companyId: string,
	documentId: string | null,
	read: () => Promise<T>
): Promise<T> {
	try {
		return await read();
	} catch (error) {
		failClosedDocumentRead(client, error, companyId, documentId);
		throw error;
	}
}

function cursorOrUndefined<T>(cursor: T | null): T | undefined {
	return cursor ?? undefined;
}

function uniqueById<T extends { id: string }>(rows: T[]): T[] {
	return [...new Map(rows.map((row) => [row.id, row])).values()];
}

export function documentsQuery(companyId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.list(queryId(companyId)),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			return guardedDocumentRead(client, company, null, () =>
				getDocuments(company, pageParam, 30, false, signal)
			);
		},
		enabled: Boolean(queryId(companyId)),
		initialPageParam: null as DocumentListCursor | null,
		getNextPageParam: (page: DocumentListPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get documents() {
			const data = readableData(query);
			return data
				? uniqueById(data.pages.flatMap((page) => page.items).map((item) => item.document))
				: [];
		},
		get summaries() {
			const data = readableData(query);
			return data
				? uniqueById(
						data.pages
							.flatMap((page) => page.items)
							.map((item) => ({
								...item,
								id: item.document.id
							}))
					).map(({ id: _id, ...item }) => item)
				: [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch()
	};
}

export function documentPrincipalQuery(companyId: QueryId) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: documentQueryKeys.principal(queryId(companyId)),
		queryFn: ({ queryKey, signal }) => {
			const company = keyPart(queryKey, 1);
			return guardedDocumentRead(client, company, null, () => getCompanyPrincipal(company, signal));
		},
		enabled: Boolean(queryId(companyId)),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get principal() {
			return (readableData(query) as CompanyPrincipal | undefined) ?? null;
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => query.refetch()
	};
}

export function documentQuery(companyId: QueryId, documentId: QueryId) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: documentQueryKeys.detail(queryId(companyId), queryId(documentId)),
		queryFn: ({ queryKey, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			return guardedDocumentRead(client, company, document, () =>
				getDocument(company, document, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId)),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get view() {
			return (readableData(query) as DocumentReadView | undefined) ?? null;
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => query.refetch(),
		accept(target: DocumentTarget, view: DocumentReadView): void {
			client.setQueryData(documentQueryKeys.detail(target.companyId, target.documentId), view);
			void invalidateDocumentList(client, target.companyId);
		}
	};
}

export function documentVersionsQuery(companyId: QueryId, documentId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.versions(queryId(companyId), queryId(documentId)),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentVersions(company, document, pageParam, 25, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId)),
		initialPageParam: null as DocumentVersionCursor | null,
		getNextPageParam: (page: DocumentVersionPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		retry: retryDocumentRead
	}));
	return {
		get versions() {
			const data = readableData(query);
			return data ? uniqueById(data.pages.flatMap((page) => page.items)) : [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch()
	};
}

export function documentVersionQuery(companyId: QueryId, documentId: QueryId, versionId: QueryId) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: documentQueryKeys.version(
			queryId(companyId),
			queryId(documentId),
			queryId(versionId)
		),
		queryFn: ({ queryKey, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			const version = keyPart(queryKey, 3);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentVersion(company, document, version, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId) && queryId(versionId)),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		retry: retryDocumentRead
	}));
	return {
		get version() {
			return (readableData(query) as DocumentVersionView | undefined) ?? null;
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		refresh: () => query.refetch()
	};
}

export function documentCommentThreadsQuery(companyId: QueryId, documentId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.commentThreads(queryId(companyId), queryId(documentId)),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentCommentThreads(company, document, pageParam, 50, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId)),
		initialPageParam: null as DocumentPageCursor | null,
		getNextPageParam: (page: DocumentCommentThreadPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get threads() {
			const data = readableData(query);
			return data
				? uniqueById(
						data.pages
							.flatMap((page) => page.items)
							.map((item) => ({
								...item,
								id: item.thread.id
							}))
					).map(({ id: _id, ...item }) => item)
				: [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch(),
		acceptCreated(target: DocumentTarget, result: DocumentCommentThreadCreated): void {
			prependInfiniteItem(
				client,
				documentQueryKeys.commentThreads(target.companyId, target.documentId),
				result.thread,
				(item) => item.thread.id
			);
			client.setQueryData<InfiniteData<DocumentCommentPage, DocumentPageCursor | null>>(
				documentQueryKeys.comments(target.companyId, target.documentId, result.thread.thread.id),
				{
					pages: [{ items: [result.first_comment], next_cursor: null }],
					pageParams: [null]
				}
			);
		},
		acceptResolved(target: DocumentTarget, result: DocumentCommentThreadRow): void {
			client.setQueryData<InfiniteData<DocumentCommentThreadPage, DocumentPageCursor | null>>(
				documentQueryKeys.commentThreads(target.companyId, target.documentId),
				(current) => {
					if (!current) return current;
					return {
						...current,
						pages: current.pages.map((page) => ({
							...page,
							items: page.items.map((item) =>
								item.thread.id === result.id ? { ...item, thread: result } : item
							)
						}))
					};
				}
			);
		}
	};
}

export function documentCommentsQuery(companyId: QueryId, documentId: QueryId, threadId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.comments(
			queryId(companyId),
			queryId(documentId),
			queryId(threadId)
		),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			const thread = keyPart(queryKey, 3);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentComments(company, document, thread, pageParam, 100, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId) && queryId(threadId)),
		initialPageParam: null as DocumentPageCursor | null,
		getNextPageParam: (page: DocumentCommentPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get comments() {
			const data = readableData(query);
			return data ? uniqueById(data.pages.flatMap((page) => page.items)) : [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		accept(target: DocumentThreadTarget, comment: DocumentCommentRow): void {
			prependInfiniteItem(
				client,
				documentQueryKeys.comments(target.companyId, target.documentId, target.threadId),
				comment,
				(item) => item.id
			);
		}
	};
}

export function documentReviewsQuery(companyId: QueryId, documentId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.reviews(queryId(companyId), queryId(documentId)),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentReviews(company, document, pageParam, 50, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId)),
		initialPageParam: null as DocumentPageCursor | null,
		getNextPageParam: (page: DocumentReviewPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get reviews() {
			const data = readableData(query);
			return data
				? uniqueById(data.pages.flatMap((page) => page.items).map((item) => item.review))
				: [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		accept(target: DocumentTarget, review: DocumentReviewView): void {
			prependInfiniteItem(
				client,
				documentQueryKeys.reviews(target.companyId, target.documentId),
				review,
				(item) => item.review.id
			);
		},
		acceptResolution(target: DocumentTarget, result: DocumentReviewResolution): void {
			replaceInfiniteItem(
				client,
				documentQueryKeys.reviews(target.companyId, target.documentId),
				{ review: result.review, work_dependency: result.work_dependency },
				(item) => item.review.id
			);
		}
	};
}

export function documentProposalsQuery(companyId: QueryId, documentId: QueryId) {
	const client = useQueryClient();
	const query = createInfiniteQuery(() => ({
		queryKey: documentQueryKeys.proposals(queryId(companyId), queryId(documentId)),
		queryFn: ({ queryKey, pageParam, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentRevisionProposals(company, document, pageParam, 50, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId)),
		initialPageParam: null as DocumentPageCursor | null,
		getNextPageParam: (page: DocumentRevisionProposalPage) => cursorOrUndefined(page.next_cursor),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		refetchInterval: DOCUMENT_REFRESH_MS,
		refetchIntervalInBackground: true,
		retry: retryDocumentRead
	}));
	return {
		get proposals() {
			const data = readableData(query);
			return data ? uniqueById(data.pages.flatMap((page) => page.items)) : [];
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		},
		get hasMore() {
			return query.hasNextPage;
		},
		get loadingMore() {
			return query.isFetchingNextPage;
		},
		loadMore: () => query.fetchNextPage(),
		refresh: () => query.refetch(),
		acceptResolution(target: DocumentTarget, result: DocumentRevisionResolution): void {
			replaceInfiniteItem(
				client,
				documentQueryKeys.proposals(target.companyId, target.documentId),
				result.proposal,
				(item) => item.id
			);
			client.setQueryData(
				documentQueryKeys.proposal(target.companyId, target.documentId, result.proposal.id),
				result.proposal
			);
		}
	};
}

export function documentProposalQuery(
	companyId: QueryId,
	documentId: QueryId,
	proposalId: QueryId
) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: documentQueryKeys.proposal(
			queryId(companyId),
			queryId(documentId),
			queryId(proposalId)
		),
		queryFn: ({ queryKey, signal }) => {
			const company = keyPart(queryKey, 1);
			const document = keyPart(queryKey, 2);
			const proposal = keyPart(queryKey, 3);
			return guardedDocumentRead(client, company, document, () =>
				getDocumentRevisionProposal(company, document, proposal, signal)
			);
		},
		enabled: Boolean(queryId(companyId) && queryId(documentId) && queryId(proposalId)),
		staleTime: DOCUMENT_STALE_MS,
		gcTime: DOCUMENT_RETAIN_MS,
		retry: retryDocumentRead
	}));
	return {
		get proposal() {
			return (readableData(query) as DocumentRevisionProposalRow | undefined) ?? null;
		},
		get status() {
			return sourceStatus(query);
		},
		get failure() {
			return (query.error as (Error & { status?: number }) | null) ?? null;
		}
	};
}

function prependInfiniteItem<T>(
	client: QueryClient,
	key: readonly unknown[],
	item: T,
	idOf: (item: T) => string
): void {
	client.setQueryData<
		InfiniteData<{ items: T[]; next_cursor: DocumentPageCursor | null }, DocumentPageCursor | null>
	>(key, (current) => {
		if (!current) return { pages: [{ items: [item], next_cursor: null }], pageParams: [null] };
		const pages = current.pages.map((page) => ({ ...page, items: [...page.items] }));
		pages[0].items = [item, ...pages[0].items.filter((row) => idOf(row) !== idOf(item))];
		return { ...current, pages };
	});
}

function replaceInfiniteItem<T>(
	client: QueryClient,
	key: readonly unknown[],
	item: T,
	idOf: (item: T) => string
): void {
	client.setQueryData<
		InfiniteData<{ items: T[]; next_cursor: DocumentPageCursor | null }, DocumentPageCursor | null>
	>(key, (current) => {
		if (!current) return current;
		const target = idOf(item);
		return {
			...current,
			pages: current.pages.map((page) => ({
				...page,
				items: page.items.map((row) => (idOf(row) === target ? item : row))
			}))
		};
	});
}

export function invalidateDocumentList(client: QueryClient, companyId: string): Promise<void> {
	return client.invalidateQueries({ queryKey: documentQueryKeys.list(companyId) });
}

export async function invalidateDocument(
	client: QueryClient,
	companyId: string,
	documentId: string
): Promise<void> {
	await Promise.all([
		client.invalidateQueries({ queryKey: documentQueryKeys.detail(companyId, documentId) }),
		client.invalidateQueries({ queryKey: documentQueryKeys.versions(companyId, documentId) }),
		client.invalidateQueries({ queryKey: documentQueryKeys.commentThreads(companyId, documentId) }),
		client.invalidateQueries({ queryKey: documentQueryKeys.reviews(companyId, documentId) }),
		client.invalidateQueries({ queryKey: documentQueryKeys.proposals(companyId, documentId) }),
		invalidateDocumentList(client, companyId)
	]);
}
