import type {
	IntelligenceAgent,
	IntelligenceConnection,
	IntelligenceView
} from './intelligence.svelte';

export type AgentRouteState =
	'checking' | 'ready' | 'starting' | 'needs_connection' | 'unavailable';

/** The listed connection a saved choice names, or none when it cannot be used now (signed out,
 * key gone, no longer granted). The one rule the Models list, its editor and the chip read, so
 * they never disagree about whether a choice works. A `direct:` choice is its provider's one key. */
export function listedConnection(
	view: IntelligenceView,
	id: string
): IntelligenceConnection | undefined {
	const listed = view.connections.find((row) => row.id === id);
	if (listed || !id.startsWith('direct:')) return listed;
	const provider = id.slice('direct:'.length);
	return view.connections.find((row) => row.kind === 'direct' && row.provider === provider);
}

/** The listed connection that serves a saved choice now. A choice on one of the owner's accounts
 * that the company no longer uses (granting another account for the same provider replaced it)
 * thinks on the company's current account for that provider, exactly as the daemon routes it
 * (`model_connections::provider_fallback`, Sprint 104 A5). Matching the choice's id alone called
 * such an agent "Model unavailable" while its turns ran (3 Nov 2026). A plain key is no fallback:
 * the daemon refuses then, and so does this. */
export function servingConnection(
	view: IntelligenceView,
	id: string
): IntelligenceConnection | undefined {
	const listed = listedConnection(view, id);
	if (listed) return listed;
	const harness = /^account-harness:([^:]+):/.exec(id)?.[1];
	if (!harness && !id.startsWith('account:')) return undefined;
	const provider = routeProvider(id);
	if (!provider) return undefined;
	const current = harness ? `account-harness:${harness}:${provider}@` : `account:${provider}@`;
	return view.connections.find((row) => row.id.startsWith(current));
}

/** A saved choice whose connection is known to be unusable now. While the connections are still
 * being read (`has_connections` null) nothing is called unavailable, so nothing flashes. */
export function connectionUnavailable(view: IntelligenceView, id: string | null | undefined) {
	return !!id && view.has_connections !== null && !servingConnection(view, id);
}

/** The provider a route serves, as `CompanyConfig::for_agent` reads it; null for a harness. */
export function routeProvider(connection: string): string | null {
	if (connection.startsWith('direct:')) return connection.slice('direct:'.length) || null;
	const account = connection.startsWith('account-harness:')
		? connection.split(':').slice(2).join(':')
		: connection.startsWith('account:')
			? connection.slice('account:'.length)
			: '';
	return account.split('@')[0] || null;
}

/** Whether an agent's assignment pins it to a model of its own, as the engine decides
 * (`pins_own_model`): a model or provider other than the company default's. The company's main
 * model on another connection of the same provider (a second Claude login, to spread a plan's
 * limits) only chooses where the agent thinks; the everyday model still runs there. */
export function pinsOwnModel(
	assignment: { connection: string; model: string } | null | undefined,
	companyDefault: { connection: string; model: string } | null | undefined
): boolean {
	if (!assignment) return false;
	if (!companyDefault) return true;
	const bare = (model: string) => model.trim().split('/').pop() ?? '';
	const own = routeProvider(assignment.connection);
	return (
		bare(assignment.model) !== bare(companyDefault.model) ||
		!own ||
		own !== routeProvider(companyDefault.connection)
	);
}

/** The model a choice runs, written as the server reports `effective_model`. */
function effectiveModel(choice: { connection: string; model: string }) {
	const provider = routeProvider(choice.connection);
	return provider && !choice.model.startsWith(`${provider}/`)
		? `${provider}/${choice.model}`
		: choice.model;
}

/** The view as it reads once a model choice is saved: shown at once, rolled back if the save
 * fails. Agents on the company default follow a new default, as the server's do. */
export function withAssignment(
	view: IntelligenceView,
	actor: string,
	choice: { connection: string; model: string } | null
): IntelligenceView {
	if (actor === 'default') {
		if (!choice) return { ...view, default: null };
		const before = view.default ?? view.base_default ?? null;
		const was = before ? effectiveModel(before) : null;
		return {
			...view,
			default: choice,
			agents: view.agents.map((agent) =>
				!agent.assignment && (!was || agent.effective_model === was)
					? { ...agent, effective_model: effectiveModel(choice) }
					: agent
			)
		};
	}
	const runs = choice ?? view.default ?? view.base_default ?? null;
	return {
		...view,
		agents: view.agents.map((agent) =>
			agent.id === actor
				? {
						...agent,
						assignment: choice,
						effective_model: runs ? effectiveModel(runs) : agent.effective_model
					}
				: agent
		)
	};
}

/** A connection that was serving stays "loaded" through a model gateway reload, for up to
 * `grace`. Adding a provider reloads the gateway, which takes every connection down for seconds;
 * read as-is, every chip, the rail and the Models list flashed "Checking model…" and back after
 * a switch (S109). A connection that never loaded, or stays down past the grace, shows as it is.
 * `since` remembers when each held connection was first seen down. */
export function holdLoaded(
	previous: IntelligenceView | null | undefined,
	next: IntelligenceView,
	since: Map<string, number>,
	now = Date.now(),
	grace = 60_000
): IntelligenceView {
	let held = false;
	for (const id of [...since.keys()])
		if (!next.connections.some((row) => row.id === id)) since.delete(id);
	const connections = next.connections.map((row) => {
		if (row.loaded) {
			since.delete(row.id);
			return row;
		}
		const was = previous?.connections.find((old) => old.id === row.id);
		if (!was?.loaded) return row;
		const first = since.get(row.id) ?? now;
		since.set(row.id, first);
		if (now - first >= grace) return row;
		held = true;
		return { ...row, loaded: true };
	});
	return held ? { ...next, connections } : next;
}

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
	let connection = selected ? servingConnection(view, selected.connection) : undefined;
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

/** The one action a route that cannot reply offers, the same words on the rail, the composer
 * and Doctor. A route that can reply offers none. */
export function routeActionLabel(state: AgentRouteState): string | null {
	if (state === 'needs_connection') return 'Connect a model';
	if (state === 'unavailable') return 'Check model connection';
	return null;
}

/** Whether an actor's model can reply now. `progressing` is live evidence that it is: one of the
 * actor's turns is thinking, acting or replying at this moment. That beats anything read from
 * the configured routes, which are a projection and can lag; a person whose turn is running is
 * never shown as waiting on a model (3 Nov 2026). */
export function agentRouteState(
	view: IntelligenceView | null,
	actorId: string,
	progressing = false
): AgentRouteState {
	if (progressing) return 'ready';
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
