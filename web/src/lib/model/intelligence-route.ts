import type {
	IntelligenceAgent,
	IntelligenceConnection,
	IntelligenceView
} from './intelligence.svelte';

export type AgentRouteState =
	'checking' | 'ready' | 'starting' | 'needs_connection' | 'unavailable';

/** The selected route for this agent, distinct from whether OrgIntel can
 * address the agent or whether a model call will ultimately succeed. */
export function agentConnection(
	view: IntelligenceView,
	agent: IntelligenceAgent
): IntelligenceConnection | undefined {
	// With no explicit assignment/default, the agent's saved preference may
	// use a different provider from the company's base model. The daemon uses
	// that effective model for this agent, so resolve its provider below.
	const selected = agent.assignment ?? view.default ?? null;
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
	return connection;
}

/** The picker must edit the same connection and model the agent will use. */
export function agentPickerSelection(view: IntelligenceView, agent: IntelligenceAgent) {
	const connection = agentConnection(view, agent);
	if (!connection) return null;
	const effective = agent.effective_model;
	const model = effective.includes('/') ? effective.slice(effective.indexOf('/') + 1) : effective;
	return { connection: connection.id, model };
}

export function agentRouteState(view: IntelligenceView | null, actorId: string): AgentRouteState {
	if (!view) return 'checking';
	if (actorId === 'exec' && view.company_issue)
		return view.has_connections === false ? 'needs_connection' : 'unavailable';
	if (view.has_connections === null) return 'checking';
	const agent = view.agents.find((row) => row.id === actorId);
	if (!agent) return 'checking';
	const connection = agentConnection(view, agent);
	if (connection) return connection.loaded ? 'ready' : 'starting';
	return view.has_connections ? 'unavailable' : 'needs_connection';
}
