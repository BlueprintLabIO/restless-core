import { createQuery } from '@tanstack/svelte-query';
import { MODEL_PRESETS } from './model-presets';
import { parseCatalog, readSnapshot, type CatalogSnapshot } from './model-catalog';
const KEY = 'restless-model-catalog-v2';
type RuntimeModels = { models: { id: string; name: string; default?: boolean }[] };
function connectedModels(provider: 'openai-codex' | 'anthropic') {
	return createQuery(() => ({
		queryKey: ['connected-models', provider],
		staleTime: 3600000,
		refetchInterval: 3600000,
		retry: false,
		queryFn: async (): Promise<RuntimeModels> => {
			const response = await fetch(`/api/connections/models/${provider}`, { cache: 'no-store' });
			if (!response.ok) throw new Error('Connected model list unavailable');
			return response.json();
		}
	}));
}
export function modelCatalog() {
	const codex = connectedModels('openai-codex');
	const claude = connectedModels('anthropic');
	const connected = (provider: string, kind: 'api_key' | 'oauth' = 'api_key') =>
		provider === 'openai-codex' ? codex : provider === 'anthropic' && kind === 'oauth' ? claude : undefined;
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
		refresh: () => Promise.all([query.refetch(), codex.refetch(), claude.refetch()]),
		refreshConnected: () => Promise.all([codex.refetch(), claude.refetch()]),
		source(provider: string, kind: 'api_key' | 'oauth' = 'api_key') {
			if (connected(provider, kind)?.data?.models?.length) return 'connected';
			if (provider === 'openai-codex') return 'bundled';
			return query.data?.providers.find((p) => p.id === provider) ? 'public' : 'bundled';
		},
		models(provider: string, kind: 'api_key' | 'oauth' = 'api_key') {
			const discovered = connected(provider, kind)?.data?.models;
			if (discovered?.length) return discovered;
			return (query.data?.providers ?? MODEL_PRESETS).find((p) => p.id === provider)?.models ?? [];
		}
	};
}
