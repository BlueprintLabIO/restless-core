import { test } from 'node:test';
import assert from 'node:assert/strict';
import { agentRouteState } from './intelligence-route.ts';
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
