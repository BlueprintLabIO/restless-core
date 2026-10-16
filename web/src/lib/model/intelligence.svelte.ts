import { ownerRequest } from './setting.svelte';
import { ownerJson } from './failure';
import { bootFetch } from './boot-fetch';
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
export type IntelligenceConnection = {
	id: string;
	provider: string;
	label?: string;
	account_provider?: string;
	account_kind?: 'api_key' | 'oauth';
	kind: 'direct' | 'harness';
	/** The plan's included AI rather than a key the owner added. */
	included?: boolean;
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
	/** With no default chosen, the connection serving the company's base model. */
	base_default?: { connection: string; model: string } | null;
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
	const selected = agent.assignment ?? view.default ?? view.base_default ?? null;
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
			const r = await bootFetch(`/api/companies/${encodeURIComponent(company)}/intelligence`, {
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

/** Choose the model an agent (or the company default) uses from its next session. */
/** How hard every agent of the company thinks: one company setting. */
export const EFFORTS = [
	{ value: 'low', label: 'Low', hint: 'Fastest and cheapest' },
	{ value: 'medium', label: 'Medium', hint: 'Balanced' },
	{ value: 'high', label: 'High', hint: 'More careful, slower' },
	{ value: 'xhigh', label: 'Extra high', hint: 'For hard judgement' },
	{ value: 'max', label: 'Max', hint: 'Slowest and most expensive' }
] as const;

export function setEffort(company: string, body: { effort: string; revision: string }) {
	return ownerRequest<unknown>(
		`/api/companies/${encodeURIComponent(company)}/intelligence-effort`,
		{ method: 'PUT', json: body }
	);
}

export function assignModel(
	company: string,
	actor: string,
	body: { connection: string; model: string; reset: boolean; revision: string }
) {
	return ownerRequest<unknown>(
		`/api/companies/${encodeURIComponent(company)}/intelligence/${encodeURIComponent(actor)}`,
		{ method: 'PUT', json: body }
	);
}
