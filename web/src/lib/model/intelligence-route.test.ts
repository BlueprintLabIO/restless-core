import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	agentPickerSelection,
	agentRouteState,
	connectionUnavailable,
	holdLoaded,
	listedConnection,
	pinsOwnModel,
	routeActionLabel,
	routeProvider,
	withAssignment
} from './intelligence-route.ts';
import type { IntelligenceView } from './intelligence.svelte';

const connected: IntelligenceView = {
	revision: '1',
	default: { connection: 'direct:anthropic', model: 'claude-opus-5-5' },
	has_connections: true,
	company_issue: 'The Exec model was not admitted',
	connections: [{ id: 'direct:anthropic', provider: 'anthropic', kind: 'direct', loaded: true }],
	agents: ['exec', 'eames'].map((id) => ({
		id,
		name: id,
		role: 'agent',
		assignment: null,
		effective_model: 'anthropic/claude-opus-5-5',
		harness: 'restless-managed'
	}))
};

test('an Exec admission failure does not disable a lead’s independently available route', () => {
	assert.equal(agentRouteState(connected, 'exec'), 'unavailable');
	assert.equal(agentRouteState(connected, 'eames'), 'ready');
});

test('a selected model is not proof of a usable connection', () => {
	assert.equal(agentRouteState({ ...connected, connections: [] }, 'eames'), 'unavailable');
	assert.equal(
		agentRouteState({ ...connected, connections: [], has_connections: false }, 'exec'),
		'needs_connection'
	);
	assert.equal(agentRouteState(null, 'eames'), 'checking');
	assert.equal(
		agentRouteState(
			{ ...connected, connections: connected.connections.map((c) => ({ ...c, loaded: false })) },
			'eames'
		),
		'starting'
	);
});

test('an agent without an explicit assignment uses its actual model provider, not the base model provider', () => {
	const view: IntelligenceView = {
		...connected,
		company_issue: null,
		default: null,
		base_default: { connection: 'direct:anthropic', model: 'claude-opus-5-5' },
		agents: connected.agents.map((agent) =>
			agent.id === 'eames' ? { ...agent, effective_model: 'openai/gpt-example' } : agent
		)
	};
	assert.equal(agentRouteState(view, 'eames'), 'unavailable');
	assert.equal(
		agentRouteState(
			{
				...view,
				connections: [{ id: 'direct:openai', provider: 'openai', kind: 'direct', loaded: true }]
			},
			'eames'
		),
		'ready'
	);
	const eames = view.agents.find((agent) => agent.id === 'eames')!;
	assert.equal(agentPickerSelection(view, eames), null);
	const withOpenAI: IntelligenceView = {
		...view,
		connections: [
			{ id: 'account:openai@fixture', provider: 'openai', kind: 'direct', loaded: true }
		]
	};
	assert.deepEqual(agentPickerSelection(withOpenAI, eames), {
		connection: 'account:openai@fixture',
		model: 'gpt-example'
	});
});

test('rail, composer and Doctor name the same action for a route', () => {
	assert.equal(routeActionLabel('needs_connection'), 'Connect a model');
	assert.equal(routeActionLabel('unavailable'), 'Check model connection');
	for (const state of ['ready', 'starting', 'checking'] as const)
		assert.equal(routeActionLabel(state), null);
	assert.equal(routeActionLabel(agentRouteState(connected, 'exec')), 'Check model connection');
});

const ACCOUNT = '0123456789abcdef0123456789abcdef';

test('a default on a connection that is no longer listed reads as unavailable, not as empty (owner, 31 Oct)', () => {
	const view: IntelligenceView = {
		...connected,
		company_issue: null,
		default: { connection: `account:openai-codex@${ACCOUNT}`, model: 'gpt-6-sol' }
	};
	assert.equal(listedConnection(view, view.default!.connection), undefined);
	assert.equal(connectionUnavailable(view, view.default!.connection), true);
	// A direct choice is its provider's one key; a key under another id still serves it.
	assert.equal(connectionUnavailable(view, 'direct:anthropic'), false);
	// While the connections are still being read, nothing is called unavailable.
	assert.equal(
		connectionUnavailable({ ...view, has_connections: null }, view.default!.connection),
		false
	);
	assert.equal(connectionUnavailable(view, null), false);
	assert.equal(routeProvider(`account-harness:claude-agent:anthropic@${ACCOUNT}`), 'anthropic');
	assert.equal(routeProvider(`account:openai-codex@${ACCOUNT}`), 'openai-codex');
	assert.equal(routeProvider('harness:codex'), null);
});

const OTHER = 'fedcba9876543210fedcba9876543210';

test('an agent whose account was replaced thinks on the company’s current one, as the daemon routes it (3 Nov)', () => {
	// Reno, 3 Nov: Ebenezer was assigned to the first ChatGPT account; granting a second one for
	// the same provider took the company's binding. The daemon keeps him thinking on the second
	// (provider_fallback), and the cockpit called him "Model unavailable" while his turns ran.
	const view: IntelligenceView = {
		...connected,
		company_issue: null,
		connections: [
			{
				id: `account:openai-codex@${OTHER}`,
				provider: 'openai-codex',
				kind: 'direct',
				loaded: true
			},
			{
				id: `account-harness:codex:openai-codex@${OTHER}`,
				provider: 'codex',
				account_provider: 'openai-codex',
				kind: 'harness',
				loaded: true
			}
		],
		agents: [
			{
				id: 'ebenezer',
				name: 'Ebenezer',
				role: 'designer',
				assignment: { connection: `account:openai-codex@${ACCOUNT}`, model: 'gpt-6-sol' },
				effective_model: 'openai-codex/gpt-6-sol',
				harness: 'restless-managed'
			},
			{
				id: 'cratchit',
				name: 'Cratchit',
				role: 'writer',
				assignment: {
					connection: `account-harness:codex:openai-codex@${ACCOUNT}`,
					model: 'gpt-6-sol'
				},
				effective_model: 'openai-codex/gpt-6-sol',
				harness: 'codex'
			}
		]
	};
	assert.equal(agentRouteState(view, 'ebenezer'), 'ready');
	assert.equal(agentRouteState(view, 'cratchit'), 'ready');
	assert.equal(connectionUnavailable(view, `account:openai-codex@${ACCOUNT}`), false);
	// The picker edits the connection that actually serves him.
	assert.deepEqual(agentPickerSelection(view, view.agents[0]), {
		connection: `account:openai-codex@${OTHER}`,
		model: 'gpt-6-sol'
	});
	// The company's current binding is a plain key, not an account: the daemon refuses, so does this.
	const keyed: IntelligenceView = {
		...view,
		connections: [
			{ id: 'direct:openai-codex', provider: 'openai-codex', kind: 'direct', loaded: true }
		]
	};
	assert.equal(agentRouteState(keyed, 'ebenezer'), 'unavailable');
	// No account for that provider at all: genuinely unavailable, and the button still shows.
	const gone: IntelligenceView = { ...view, connections: connected.connections };
	assert.equal(agentRouteState(gone, 'ebenezer'), 'unavailable');
	assert.equal(routeActionLabel(agentRouteState(gone, 'ebenezer')), 'Check model connection');
});

test('a turn in progress is proof the model replies: never "unavailable" while it runs (3 Nov)', () => {
	const gone: IntelligenceView = { ...connected, company_issue: null, connections: [] };
	assert.equal(agentRouteState(gone, 'eames'), 'unavailable');
	assert.equal(agentRouteState(gone, 'eames', true), 'ready');
	assert.equal(agentRouteState(connected, 'exec', true), 'ready');
	assert.equal(routeActionLabel(agentRouteState(gone, 'eames', true)), null);
});

test('a new company default shows at once for agents on the default, not for agents with their own', () => {
	const view: IntelligenceView = {
		...connected,
		agents: [
			...connected.agents,
			{
				id: 'pinned',
				name: 'pinned',
				role: 'agent',
				assignment: { connection: 'direct:anthropic', model: 'claude-haiku-5-5' },
				effective_model: 'anthropic/claude-haiku-5-5',
				harness: 'restless-managed'
			}
		]
	};
	const next = withAssignment(view, 'default', {
		connection: `account:openai-codex@${ACCOUNT}`,
		model: 'gpt-6-sol'
	});
	assert.deepEqual(next.default, {
		connection: `account:openai-codex@${ACCOUNT}`,
		model: 'gpt-6-sol'
	});
	assert.deepEqual(
		next.agents.map((agent) => agent.effective_model),
		['openai-codex/gpt-6-sol', 'openai-codex/gpt-6-sol', 'anthropic/claude-haiku-5-5']
	);
	// Back on the company model: the agent runs the default again.
	const reset = withAssignment(next, 'pinned', null);
	assert.equal(reset.agents[2].assignment, null);
	assert.equal(reset.agents[2].effective_model, 'openai-codex/gpt-6-sol');
	// The view handed in is left as it was, for the rollback.
	assert.equal(view.default?.model, 'claude-opus-5-5');
});

test('a gateway reload does not flash a serving connection to "starting"; a new one shows as it is', () => {
	const since = new Map<string, number>();
	const down: IntelligenceView = {
		...connected,
		connections: [
			{ ...connected.connections[0], loaded: false },
			{ id: 'direct:openai', provider: 'openai', kind: 'direct', loaded: false }
		]
	};
	const held = holdLoaded(connected, down, since, 1_000);
	assert.deepEqual(
		held.connections.map((row) => row.loaded),
		[true, false]
	);
	assert.equal(agentRouteState(held, 'eames'), 'ready');
	// Still down after the grace: it is shown as it is.
	const later = holdLoaded(held, down, since, 61_000);
	assert.equal(later.connections[0].loaded, false);
	// Back up: the hold is forgotten.
	holdLoaded(later, connected, since, 62_000);
	assert.equal(since.size, 0);
});

test('the main model on a second login of the same provider is not a model of its own', () => {
	const company = {
		connection: 'account:anthropic@e69f625023c149158e3b9d79ce9f7d63',
		model: 'claude-opus-5-5'
	};
	const secondLogin = 'account:anthropic@dcbf955c63a14e58958cf9efa0d6f97a';
	// Reno's Staff: Opus on a second Claude login spreads the plan's limits.
	assert.equal(pinsOwnModel({ connection: secondLogin, model: 'claude-opus-5-5' }, company), false);
	assert.equal(
		pinsOwnModel({ connection: secondLogin, model: 'anthropic/claude-opus-5-5' }, company),
		false
	);
	// Another model, another provider or a harness is the owner's exact choice.
	assert.equal(pinsOwnModel({ connection: secondLogin, model: 'claude-haiku-5-5' }, company), true);
	assert.equal(
		pinsOwnModel({ connection: 'direct:openai', model: 'claude-opus-5-5' }, company),
		true
	);
	assert.equal(
		pinsOwnModel({ connection: 'harness:claude-agent', model: 'claude-opus-5-5' }, company),
		true
	);
	// No company default to compare: any assignment pins; no assignment never does.
	assert.equal(pinsOwnModel({ connection: secondLogin, model: 'claude-opus-5-5' }, null), true);
	assert.equal(pinsOwnModel(null, company), false);
});
