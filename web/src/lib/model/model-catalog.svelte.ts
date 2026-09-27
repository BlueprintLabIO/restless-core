import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { MODEL_PRESETS } from './model-presets';
import { parseCatalog, readSnapshot, type CatalogSnapshot } from './model-catalog';
const KEY = 'restless-model-catalog-v2';
const CATALOG_QUERY_KEY = ['model-catalog'];
type RuntimeModels = { models: { id: string; name: string; default?: boolean }[] };
async function fetchCatalog(force = false): Promise<CatalogSnapshot> {
	const response = await fetch(`/api/model-catalog${force ? '?refresh=true' : ''}`, {
		cache: 'no-store',
		signal: AbortSignal.timeout(60000)
	});
	if (!response.ok) throw new Error('Catalog refresh unavailable');
	const raw = await response.text();
	if (raw.length > 12000100) throw new Error('Catalog is too large');
	const payload = JSON.parse(raw);
	if (!Number.isFinite(payload.updatedAt)) throw new Error('Catalog refresh was invalid');
	const snapshot = { updatedAt: payload.updatedAt, providers: parseCatalog(payload.catalog) };
	try {
		localStorage.setItem(KEY, JSON.stringify(snapshot));
	} catch {
		/* Keep the in-memory catalog. */
	}
	return snapshot;
}
type AccountConnection = { provider: string; kind: string; status?: string };
/* Which providers this account has signed in to. A provider's model list is
 * only asked for once it is connected: asking first only produces a 404 the
 * browser logs as an error on every settings page. */
function accountConnections(enabled: () => boolean) {
	return createQuery(() => ({
		queryKey: ['account-connections'],
		enabled: enabled(),
		staleTime: 30_000,
		retry: false,
		queryFn: async (): Promise<AccountConnection[]> => {
			const response = await fetch('/api/connections', { cache: 'no-store' });
			if (!response.ok) throw new Error('Account connections unavailable');
			return ((await response.json()) as { connections?: AccountConnection[] }).connections ?? [];
		}
	}));
}
function connectedModels(
	provider: 'openai-codex' | 'anthropic',
	enabled: () => boolean,
	connections: { data?: AccountConnection[] }
) {
	const signedIn = () =>
		connections.data?.some((row) => row.kind === 'oauth' && row.provider === provider) ?? false;
	return createQuery(() => ({
		queryKey: ['connected-models', provider, signedIn()],
		enabled: enabled() && signedIn(),
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
/** `enabled` lets a surface that only sometimes shows model choices skip the
 * catalog fetch (about 75 KiB) until it needs it. */
export function modelCatalog(enabled: () => boolean = () => true) {
	const queryClient = useQueryClient();
	let manualPending = $state(false);
	let manualFailed = $state(false);
	const connections = accountConnections(enabled);
	const codex = connectedModels('openai-codex', enabled, connections);
	const claude = connectedModels('anthropic', enabled, connections);
	/* Invalidation, unlike refetch, leaves a disabled (not signed-in) list alone. */
	const refreshSignedIn = async () => {
		await connections.refetch();
		await queryClient.invalidateQueries({ queryKey: ['connected-models'] });
	};
	const connected = (provider: string, kind: 'api_key' | 'oauth' = 'api_key') =>
		provider === 'openai-codex'
			? codex
			: provider === 'anthropic' && kind === 'oauth'
				? claude
				: undefined;
	let initial: CatalogSnapshot | undefined;
	if (typeof localStorage !== 'undefined') {
		try {
			initial = readSnapshot(localStorage.getItem(KEY));
		} catch {
			/* Storage can be disabled. */
		}
	}
	const query = createQuery(() => ({
		queryKey: CATALOG_QUERY_KEY,
		enabled: enabled(),
		initialData: initial,
		initialDataUpdatedAt: initial?.updatedAt,
		staleTime: 3600000,
		refetchInterval: 3600000,
		retry: false,
		queryFn: () => fetchCatalog()
	}));
	return {
		get providers() {
			return query.data?.providers ?? MODEL_PRESETS;
		},
		get updatedAt() {
			return query.data?.updatedAt;
		},
		get pending() {
			return query.isFetching || manualPending;
		},
		get failed() {
			return !!query.error || manualFailed;
		},
		refresh: async () => {
			manualPending = true;
			try {
				const [snapshot] = await Promise.all([fetchCatalog(true), refreshSignedIn()]);
				queryClient.setQueryData(CATALOG_QUERY_KEY, snapshot);
				manualFailed = false;
			} catch {
				manualFailed = true;
			} finally {
				manualPending = false;
			}
		},
		refreshConnected: refreshSignedIn,
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
