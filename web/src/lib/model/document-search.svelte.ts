import { onDestroy } from 'svelte';
import { useQueryClient } from '@tanstack/svelte-query';
import { failClosedDocumentRead } from './document-cache';
import { failureSentence } from './failure';
import { searchDocuments } from './documents';
import { documentSearchController, type DocumentSearchState } from './document-search-controller';

/** Search results and snippets stay in memory under the verified principal. */
export function documentSearch(scope: {
	company: () => string;
	partition: () => string;
	authorized: () => boolean;
}) {
	const client = useQueryClient();
	let query = $state(''),
		refreshToken = $state(0);
	const target = () => ({
		company: scope.company(),
		partition: scope.partition(),
		authorized: scope.authorized(),
		query
	});
	let state = $state<DocumentSearchState>({
		scope: target(),
		hits: [],
		failure: '',
		loading: false,
		loadingMore: false,
		nextOffset: null
	});
	const controller = documentSearchController({
		scope: target,
		read: searchDocuments,
		denied: (cause, company) => failClosedDocumentRead(client, cause, company),
		message: (cause) => failureSentence(cause, 'Document search is unavailable.'),
		changed: (value) => (state = value)
	});
	$effect(() => {
		target();
		refreshToken;
		controller.reset();
		return () => controller.cancel();
	});
	onDestroy(() => controller.dispose());
	// Reading state tracks publication; the controller also checks scope synchronously
	// before a pending Svelte effect can run after an identity or query change.
	const view = () => {
		state;
		return controller.state;
	};
	return {
		get query() {
			return query;
		},
		get hits() {
			return view().hits;
		},
		get failure() {
			return view().failure;
		},
		get loading() {
			return view().loading;
		},
		get loadingMore() {
			return view().loadingMore;
		},
		get hasMore() {
			return view().nextOffset !== null;
		},
		setQuery(value: string) {
			query = value;
		},
		refresh() {
			refreshToken++;
		},
		loadMore: controller.loadMore,
		denyDocument: controller.denyDocument,
		admitDocument: controller.admitDocument
	};
}
