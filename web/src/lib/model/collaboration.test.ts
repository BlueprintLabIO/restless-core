import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as collaboration from './collaboration.ts';

const {
	COLLABORATION_SCHEMA_VERSION,
	collaborationMatchesPrincipal,
	getCollaborationBootstrap,
	parseCollaborationBootstrap
} = collaboration;

function fixture(companyId = 'acme') {
	return {
		schema_version: COLLABORATION_SCHEMA_VERSION,
		principal: {
			actor_id: 'alice',
			membership_role: 'member',
			cache_partition: 'partition-alice-v1'
		},
		company: { id: companyId, company_id: null, name: 'Acme', mission: 'Ship together.' },
		people: [
			{
				actor_id: 'alice',
				kind: 'human',
				actor_class: 'human',
				role: 'Collaborator',
				display: 'Alice',
				team_id: null,
				session_running: false
			}
		],
		teams: [],
		goals: [],
		work_graph: {
			work: [
				{
					id: 'work-1',
					goal_id: null,
					owner_id: 'alice',
					title: 'Launch the demo',
					outcome: 'A playable demo.',
					status: 'active',
					resolution: '',
					priority: 10,
					expected_artifact: 'A public URL',
					revision: 1,
					updated_at: '2026-09-09T00:00:00Z'
				}
			],
			edges: [],
			attempts: [],
			artifacts: []
		},
		attention: { handoffs: [], mentions: [] },
		source_health: { orgintel: 'available' },
		refreshed_at: '2026-09-09T00:00:00Z'
	};
}

test('parses the explicit member collaboration schema', () => {
	const parsed = parseCollaborationBootstrap(fixture());
	assert.equal(parsed.principal.actor_id, 'alice');
	assert.equal(parsed.work_graph.work[0].title, 'Launch the demo');
});

test('fails closed for unknown schema and membership values', () => {
	assert.throws(
		() => parseCollaborationBootstrap({ ...fixture(), schema_version: 'owner-cockpit.v1' }),
		/response is invalid/
	);
	assert.throws(
		() =>
			parseCollaborationBootstrap({
				...fixture(),
				principal: { ...fixture().principal, membership_role: 'operator' }
			}),
		/response is invalid/
	);
});

test('binds cached collaboration data to the exact authenticated principal', () => {
	const view = parseCollaborationBootstrap(fixture());
	assert.equal(collaborationMatchesPrincipal(view, view.principal), true);
	assert.equal(
		collaborationMatchesPrincipal(view, { ...view.principal, actor_id: 'mallory' }),
		false
	);
	assert.equal(
		collaborationMatchesPrincipal(view, { ...view.principal, cache_partition: 'new-session' }),
		false
	);
});

test('fetches only the member-safe bootstrap with no-store credentials', async () => {
	const previousFetch = globalThis.fetch;
	const requests: Array<[RequestInfo | URL, RequestInit | undefined]> = [];
	globalThis.fetch = async (input, init) => {
		requests.push([input, init]);
		return new Response(JSON.stringify(fixture('acme/blue')), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		});
	};
	try {
		const value = await getCollaborationBootstrap('acme/blue');
		assert.equal(value.company.id, 'acme/blue');
		assert.equal(requests[0]?.[0], '/api/companies/acme%2Fblue/collaboration/bootstrap');
		assert.equal(requests[0]?.[1]?.credentials, 'same-origin');
		assert.equal(requests[0]?.[1]?.cache, 'no-store');
	} finally {
		globalThis.fetch = previousFetch;
	}
});
