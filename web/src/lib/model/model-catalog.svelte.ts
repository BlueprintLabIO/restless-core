import { createQuery } from '@tanstack/svelte-query';
import { MODEL_PRESETS } from './model-presets';
import { parseCatalog, readSnapshot, type CatalogSnapshot } from './model-catalog';
const KEY = 'restless-model-catalog-v1';
export function modelCatalog() {
	let initial: CatalogSnapshot | undefined;
	if (typeof localStorage !== 'undefined') {
		try {
			initial = readSnapshot(localStorage.getItem(KEY));
		} catch {
			/* Storage can be disabled. */
		}
	}
	const query = createQuery(() => ({
		queryKey: ['model-catalog'],
		initialData: initial,
		initialDataUpdatedAt: initial?.updatedAt,
		staleTime: 3600000,
		refetchInterval: 3600000,
		retry: false,
		queryFn: async () => {
			const response = await fetch('https://models.dev/api.json', {
				credentials: 'omit',
				referrerPolicy: 'no-referrer',
				signal: AbortSignal.timeout(15000)
			});
			if (!response.ok) throw new Error('Catalog refresh unavailable');
			const raw = await response.text();
			if (raw.length > 12000000) throw new Error('Catalog is too large');
			const snapshot = { updatedAt: Date.now(), providers: parseCatalog(JSON.parse(raw)) };
			try {
				localStorage.setItem(KEY, JSON.stringify(snapshot));
			} catch {
				/* Keep the in-memory catalog. */
			}
			return snapshot;
		}
	}));
	return {
		get providers() {
			return query.data?.providers ?? MODEL_PRESETS;
		},
		get updatedAt() {
			return query.data?.updatedAt;
		},
		get pending() {
			return query.isFetching;
		},
		get failed() {
			return !!query.error;
		},
		refresh: () => query.refetch(),
		models(provider: string) {
			return (query.data?.providers ?? MODEL_PRESETS).find((p) => p.id === provider)?.models ?? [];
		}
	};
}
