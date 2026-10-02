import type { DocumentSearchHit, DocumentSearchPage } from './documents';

export interface DocumentSearchScope {
	company: string;
	partition: string;
	authorized: boolean;
	query: string;
}
export interface DocumentSearchState {
	scope: DocumentSearchScope;
	hits: DocumentSearchHit[];
	failure: string;
	loading: boolean;
	loadingMore: boolean;
	nextOffset: number | null;
}

/** One cancellable, ephemeral native-document search and its explicit next page. */
export function documentSearchController(options: {
	scope: () => DocumentSearchScope;
	read: (
		company: string,
		query: string,
		offset: number,
		signal: AbortSignal
	) => Promise<DocumentSearchPage>;
	denied: (cause: unknown, company: string) => boolean;
	message: (cause: unknown) => string;
	changed: (state: DocumentSearchState) => void;
	debounceMs?: number;
}) {
	let generation = 0,
		disposed = false;
	let controller: AbortController | null = null;
	let timer: ReturnType<typeof setTimeout> | undefined;
	let deniedDocuments = new Set<string>();
	let state: DocumentSearchState = {
		scope: options.scope(),
		hits: [],
		failure: '',
		loading: false,
		loadingMore: false,
		nextOffset: null
	};
	const sameScope = (a: DocumentSearchScope, b: DocumentSearchScope) =>
		a.company === b.company &&
		a.partition === b.partition &&
		a.authorized === b.authorized &&
		a.query.trim() === b.query.trim();
	const current = (captured: DocumentSearchScope, version: number) =>
		!disposed &&
		captured.authorized &&
		Boolean(captured.partition) &&
		version === generation &&
		sameScope(captured, options.scope());
	function publish(patch: Partial<DocumentSearchState>) {
		state = { ...state, ...patch };
		options.changed(state);
	}
	function cancel() {
		generation++;
		clearTimeout(timer);
		controller?.abort();
		controller = null;
	}
	async function read(
		captured: DocumentSearchScope,
		version: number,
		offset: number,
		signal: AbortSignal
	) {
		try {
			const page = await options.read(captured.company, captured.query.trim(), offset, signal);
			if (!current(captured, version) || signal.aborted) return;
			const hits = [
				...new Map(
					[...(offset ? state.hits : []), ...page.items].map((hit) => [hit.document_id, hit])
				).values()
			].filter((hit) => !deniedDocuments.has(hit.document_id));
			publish({ hits, nextOffset: page.next_offset, failure: '' });
		} catch (cause) {
			if (!current(captured, version) || signal.aborted) return;
			const denied = options.denied(cause, captured.company);
			publish({
				...(denied ? { hits: [], nextOffset: null } : {}),
				failure: options.message(cause)
			});
		} finally {
			if (current(captured, version)) publish({ loading: false, loadingMore: false });
		}
	}
	function reset() {
		cancel();
		const scope = options.scope();
		// Tombstones block this generation's stale responses. A fresh request
		// rechecks access server-side, so an explicitly re-granted document can return.
		deniedDocuments = new Set();
		publish({ scope, hits: [], nextOffset: null, failure: '', loading: false, loadingMore: false });
		if (disposed || !scope.query.trim() || !scope.partition || !scope.authorized) return;
		if (new TextEncoder().encode(scope.query.trim()).length > 256) {
			publish({ failure: 'Search is too long. Try a shorter phrase.' });
			return;
		}
		publish({ loading: true });
		const abort = new AbortController();
		controller = abort;
		const version = generation;
		timer = setTimeout(() => {
			void read(scope, version, 0, abort.signal);
		}, options.debounceMs ?? 180);
	}
	return {
		get state() {
			if (disposed || !sameScope(state.scope, options.scope()))
				return {
					...state,
					hits: [],
					nextOffset: null,
					failure: '',
					loading: false,
					loadingMore: false
				};
			return state;
		},
		reset,
		cancel,
		dispose() {
			disposed = true;
			cancel();
		},
		loadMore() {
			if (
				!current(state.scope, generation) ||
				state.nextOffset === null ||
				state.loading ||
				state.loadingMore ||
				!controller
			)
				return;
			publish({ loadingMore: true });
			void read(state.scope, generation, state.nextOffset!, controller.signal);
		},
		denyDocument(id: string) {
			deniedDocuments.add(id);
			publish({ hits: state.hits.filter((hit) => hit.document_id !== id) });
		},
		admitDocument(id: string) {
			if (deniedDocuments.delete(id)) reset();
		}
	};
}
