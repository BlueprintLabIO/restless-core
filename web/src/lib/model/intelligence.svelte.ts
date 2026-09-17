import { createQuery, useQueryClient } from '@tanstack/svelte-query';
export type IntelligenceConnection = {
	id: string;
	provider: string;
	kind: 'direct' | 'harness';
	model?: string;
	models?: { id: string; name: string; default?: boolean }[] | null;
	loaded: boolean;
};
export type IntelligenceAgent = {
	id: string;
	name: string;
	role: string;
	assignment: { connection: string; model: string } | null;
	effective_model: string;
	harness: string;
};
export type IntelligenceView = {
	revision: string;
	default: { connection: string; model: string } | null;
	has_connections: boolean | null;
	connections: IntelligenceConnection[];
	agents: IntelligenceAgent[];
};
export function intelligenceQuery(company: string, enabled: () => boolean = () => true) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: ['intelligence', company],
		queryFn: async ({ signal }) => {
			const r = await fetch(`/api/companies/${encodeURIComponent(company)}/intelligence`, {
				signal
			});
			if (!r.ok) throw new Error('Could not check intelligence connections.');
			return (await r.json()) as IntelligenceView;
		},
		enabled: enabled(),
		staleTime: 5000,
		refetchInterval: 10000,
		retry: 1
	}));
	return {
		get view() {
			return query.data ?? null;
		},
		get error() {
			return query.error;
		},
		refresh: () => client.invalidateQueries({ queryKey: ['intelligence', company] })
	};
}
