import { ownerRequest } from './setting.svelte';
import { pollEvery } from './connection.svelte';
import { ownerJson } from './failure';
import { bootFetch } from './boot-fetch.ts';
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
	/** The observed Exec admission issue; it does not describe another agent's assigned route. */
	company_issue?: string | null;
	default: { connection: string; model: string } | null;
	/** With no default chosen, the connection serving the company's base model. */
	base_default?: { connection: string; model: string } | null;
	has_connections: boolean | null;
	connections: IntelligenceConnection[];
	agents: IntelligenceAgent[];
};

export { agentRouteState, type AgentRouteState } from './intelligence-route';

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
		// outside this browser: quickly only while a sign-in can be under way (no
		// connection yet, or one still loading), so it never waits a full minute.
		refetchInterval: (query: { state: { data: IntelligenceView | undefined } }) => {
			const view = query.state.data;
			const settling = !view?.has_connections || view.connections.some((row) => !row.loaded);
			return settling ? 15_000 : pollEvery(120_000)();
		}
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
