import type { IntelligenceView } from './intelligence.svelte';

export type AgentRouteState =
	'checking' | 'ready' | 'starting' | 'needs_connection' | 'unavailable';

/** The selected route for this agent, distinct from whether OrgIntel can
 * address the agent or whether a model call will ultimately succeed. */
export function agentRouteState(view: IntelligenceView | null, actorId: string): AgentRouteState {
	if (!view) return 'checking';
	if (actorId === 'exec' && view.company_issue)
		return view.has_connections === false ? 'needs_connection' : 'unavailable';
	if (view.has_connections === null) return 'checking';
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
