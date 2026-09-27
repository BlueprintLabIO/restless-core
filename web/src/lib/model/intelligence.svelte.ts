import { ownerJson } from './failure';
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
export type IntelligenceConnection = {
	id: string;
	provider: string;
	label?: string;
	account_provider?: string;
	account_kind?: 'api_key' | 'oauth';
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
	thinking_effort?: string;
	harness: string;
};
export type IntelligenceView = {
	revision: string;
	default: { connection: string; model: string } | null;
	has_connections: boolean | null;
	connections: IntelligenceConnection[];
	agents: IntelligenceAgent[];
};

export type AgentRouteState =
	'checking' | 'ready' | 'starting' | 'needs_connection' | 'unavailable';

/** The selected route for this agent, distinct from whether OrgIntel can
 * address the agent or whether a model call will ultimately succeed. */
export function agentRouteState(view: IntelligenceView | null, actorId: string): AgentRouteState {
	if (!view || view.has_connections === null) return 'checking';
	const agent = view.agents.find((row) => row.id === actorId);
	if (!agent) return 'checking';
	const selected = agent.assignment ?? view.default;
	let connection = selected
		? view.connections.find((row) => row.id === selected.connection)
		: undefined;
	if (!connection && selected?.connection.startsWith('direct:')) {
		const provider = selected.connection.slice('direct:'.length);
		connection = view.connections.find((row) => row.kind === 'direct' && row.provider === provider);
	}
	if (!connection && !selected) {
		const provider = agent.effective_model.split('/')[0];
		connection = agent.effective_model.startsWith('native-')
			? view.connections.find((row) => row.id === `harness:${agent.harness}`)
			: view.connections.find((row) => row.kind === 'direct' && row.provider === provider);
	}
	if (connection) return connection.loaded ? 'ready' : 'starting';
	return view.has_connections ? 'unavailable' : 'needs_connection';
}

export function intelligenceQuery(company: string, enabled: () => boolean = () => true) {
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: ['intelligence', company],
		queryFn: async ({ signal }) => {
			const r = await fetch(`/api/companies/${encodeURIComponent(company)}/intelligence`, {
				signal
			});
			return ownerJson<IntelligenceView>(r);
		},
		enabled: enabled(),
		staleTime: 5000,
		// Events handle known changes. Poll for CLI and broker changes made
		// outside this browser, without making a sign-in wait a full minute.
		refetchInterval: 15_000
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
