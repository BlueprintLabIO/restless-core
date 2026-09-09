import assert from 'node:assert/strict';
import test from 'node:test';
import { QueryClient } from '@tanstack/query-core';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as persistence from './query-persistence.ts';
import type { PersistedQueryEnvelope, SafeQueryStore } from './query-persistence';

const {
	QUERY_CACHE_MAX_AGE_MS,
	QUERY_CACHE_MAX_ATTENTION_ITEMS,
	QUERY_CACHE_MAX_ENTRIES,
	QUERY_CACHE_MAX_TOTAL_BYTES,
	QUERY_CACHE_MAX_ROOM_MESSAGES,
	QUERY_CACHE_SCHEMA,
	buildPersistedEnvelope,
	companyForPersistableQuery,
	getCompanyPrincipal,
	parsePersistedEnvelope,
	projectQueryForPersistence,
	queryCacheRecordId,
	shouldHydratePersistedEntry,
	startCompanyQueryPersistence
} = persistence;

const NOW = Date.parse('2026-09-09T02:00:00Z');

function attentionView() {
	return {
		company: { id: 'acme', name: 'Acme', mission: 'Ship useful work.', model: 'model' },
		sourceHealth: {
			orgintel: 'available',
			authority: 'available',
			runtime: 'running',
			browser: 'available'
		},
		workGraph: { secret: 'runtime command and output' },
		items: [
			{
				id: 'authority-item',
				source: { plane: 'authority', kind: 'approval', reference: 'approval-1', party: 'bank' },
				category: 'approval',
				title: 'Approve transfer',
				whatHappened: 'A payment is waiting.',
				whyItMatters: 'Money moves.',
				recommendation: 'Approve.',
				requestedAction: 'Approve transfer.',
				ifNoAction: 'Nothing moves.',
				briefStatus: 'current',
				evidence: [],
				reviewSources: [],
				actions: [],
				canContinue: false,
				createdAt: '2026-09-09T01:00:00Z'
			},
			{
				id: 'runtime-item',
				source: { plane: 'runtime', kind: 'attach', reference: 'runtime-1' },
				category: 'review',
				title: 'Runtime result',
				whatHappened: 'A runtime produced data.',
				whyItMatters: 'It may contain live environment state.',
				recommendation: 'Inspect live.',
				requestedAction: 'Open runtime.',
				ifNoAction: 'Nothing changes.',
				briefStatus: 'current',
				evidence: [],
				reviewSources: [],
				actions: [],
				canContinue: false,
				createdAt: '2026-09-09T01:00:00Z'
			},
			{
				id: 'review-item',
				workId: 'work-1',
				source: { plane: 'orgintel', kind: 'review', reference: 'handoff-1' },
				category: 'review',
				title: 'Review the result',
				whatHappened: 'The result is ready.',
				whyItMatters: 'It closes the Work.',
				recommendation: 'Inspect it.',
				requestedAction: 'Accept or revise.',
				ifNoAction: 'Work waits.',
				briefStatus: 'current',
				evidence: [{ label: 'secret', kind: 'runtime', content: 'runtime-secret' }],
				reviewSources: [
					{
						label: 'provider',
						provider: 'provider',
						reference: 'ref',
						verification: 'verified',
						content: 'provider-secret',
						observedAt: '2026-09-09T01:00:00Z'
					}
				],
				runtimeAttach: { company: 'acme', generation: 'secret-generation' },
				reviewTarget: {
					company: 'acme',
					generation: 'generation',
					uri: 'file:///company/secret',
					status: 'available',
					kind: 'runtime-text',
					label: 'Result',
					content: 'artifact-secret'
				},
				actions: [
					{
						id: 'inspect',
						label: 'Inspect',
						role: 'inspect',
						consequence: 'Opens the result.',
						nextState: 'open',
						href: 'https://secret.example'
					}
				],
				canContinue: true,
				createdAt: '2026-09-09T01:00:00Z'
			}
		],
		continuations: [],
		refreshedAt: '2026-09-09T01:00:00Z'
	};
}

function roomMessage(id: number, room = 'general') {
	return {
		id,
		room_id: room,
		from_actor: id % 2 ? 'alice' : 'exec',
		to_actor: null,
		body: `Message ${id}`,
		outcome_standard: null,
		parent_message_id: null,
		thread_root_message_id: null,
		client_command_id: `command-${id}`,
		created_at: '2026-09-09T01:00:00Z',
		legacy_read_at: '2026-09-09T01:00:00Z'
	};
}

function roomPage(room = 'general', count = 2) {
	return {
		pages: [
			{
				messages: Array.from({ length: count }, (_, index) => roomMessage(index + 1, room)),
				mentions: [
					{
						id: 'mention-1',
						room_id: room,
						message_id: 1,
						thread_root_message_id: 1,
						mentioned_actor_id: 'exec',
						kind: 'exec',
						work_id: null,
						why_this_actor: 'Needs judgement.',
						expected_response: null,
						recommendation: null,
						alternatives: { secret: 'alternative-secret' },
						evidence: { secret: 'evidence-secret' },
						uncertainty: null,
						affected_scope: null,
						deadline_at: null,
						fallback: null,
						independent_work_can_continue: true,
						created_event_id: 1,
						resolution_message_id: null,
						resolved_event_id: null,
						cancelled_event_id: null,
						cancelled_by: null,
						cancellation_reason: null,
						created_at: '2026-09-09T01:00:00Z',
						resolved_at: null,
						cancelled_at: null
					}
				],
				next_before_message_id: 1,
				has_more: true
			},
			{
				messages: [{ ...roomMessage(-1, room), body: 'older-page-secret' }],
				mentions: [],
				next_before_message_id: null,
				has_more: false
			}
		],
		pageParams: [null, 1]
	};
}

test('cache keys are exact, company-scoped, and exclude sensitive query families', () => {
	assert.equal(companyForPersistableQuery(['attention', 'acme']), 'acme');
	assert.equal(companyForPersistableQuery(['room-messages', 'acme', 'general']), 'acme');
	for (const key of [
		['cockpit', 'acme'],
		['company', 'acme', { probeCredentials: false }],
		['conversation', 'acme', 'exec'],
		['room-thread', 'acme', 'general', 1],
		['browser-status', 'acme'],
		['attention', 'acme', 'extra']
	]) {
		assert.equal(companyForPersistableQuery(key), null);
	}
	assert.notEqual(queryCacheRecordId('acme', 'alice-v1'), queryCacheRecordId('acme', 'bob-v1'));
	assert.notEqual(queryCacheRecordId('acme', 'alice-v1'), queryCacheRecordId('other', 'alice-v1'));
});

test('Attention persistence is a bounded summary without Authority or Runtime payloads', () => {
	const projected = projectQueryForPersistence(['attention', 'acme'], attentionView(), 'acme');
	assert.ok(projected);
	const value = projected as ReturnType<typeof attentionView>;
	assert.equal(value.workGraph, null);
	assert.equal(value.sourceHealth.runtime, 'unknown');
	assert.equal(value.sourceHealth.browser, 'unknown');
	assert.deepEqual(
		value.items.map((item) => item.id),
		['review-item']
	);
	assert.deepEqual(value.items[0].evidence, []);
	assert.deepEqual(value.items[0].reviewSources, []);
	assert.equal('runtimeAttach' in value.items[0], false);
	assert.equal('reviewTarget' in value.items[0], false);
	assert.deepEqual(value.items[0].actions, []);
	const serialized = JSON.stringify(projected);
	for (const secret of [
		'authority-item',
		'runtime-item',
		'runtime-secret',
		'provider-secret',
		'artifact-secret',
		'secret-generation',
		'https://secret.example'
	]) {
		assert.equal(serialized.includes(secret), false);
	}
});

test('Room persistence keeps only the latest bounded page and redacts mutation and evidence data', () => {
	const projected = projectQueryForPersistence(
		['room-messages', 'acme', 'general'],
		roomPage('general', 60),
		'acme'
	) as ReturnType<typeof roomPage>;
	assert.equal(projected.pages.length, 1);
	assert.equal(projected.pages[0].messages.length, QUERY_CACHE_MAX_ROOM_MESSAGES);
	assert.equal(projected.pages[0].messages[0].id, 11);
	assert.equal(projected.pages[0].messages.at(-1)?.id, 60);
	assert.equal(projected.pages[0].messages[0].client_command_id, null);
	assert.equal(projected.pages[0].messages[0].legacy_read_at, null);
	assert.equal(projected.pages[0].mentions.length, 0);
	assert.equal(projected.pages[0].next_before_message_id, 11);
	assert.deepEqual(projected.pageParams, [null]);
	assert.equal(projected.pages[0].has_more, true);
	const serialized = JSON.stringify(projected);
	assert.equal(serialized.includes('older-page-secret'), false);
	assert.equal(serialized.includes('command-'), false);
	assert.equal(serialized.includes('evidence-secret'), false);
});

test('envelopes enforce item, byte, age, schema and partition bounds', () => {
	const snapshots = Array.from({ length: QUERY_CACHE_MAX_ENTRIES + 5 }, (_, index) => ({
		queryKey: ['room-messages', 'acme', `room-${index}`],
		data: roomPage(`room-${index}`),
		dataUpdatedAt: NOW - index
	}));
	const envelope = buildPersistedEnvelope('acme', 'partition-a', snapshots, NOW);
	assert.equal(envelope.entries.length, QUERY_CACHE_MAX_ENTRIES);
	assert.ok(
		new TextEncoder().encode(JSON.stringify(envelope)).byteLength <= QUERY_CACHE_MAX_TOTAL_BYTES
	);
	assert.ok(parsePersistedEnvelope(envelope, 'acme', 'partition-a', NOW));

	const oversizedAttention = attentionView();
	oversizedAttention.items = Array.from(
		{ length: QUERY_CACHE_MAX_ATTENTION_ITEMS + 10 },
		(_, index) => ({
			...attentionView().items.at(-1)!,
			id: `review-${index}`,
			whatHappened: 'x'.repeat(8_000),
			whyItMatters: 'x'.repeat(8_000),
			recommendation: 'x'.repeat(8_000),
			requestedAction: 'x'.repeat(8_000),
			ifNoAction: 'x'.repeat(8_000)
		})
	);
	assert.equal(
		buildPersistedEnvelope(
			'acme',
			'partition-a',
			[{ queryKey: ['attention', 'acme'], data: oversizedAttention, dataUpdatedAt: NOW }],
			NOW
		).entries.length,
		0
	);

	assert.equal(
		parsePersistedEnvelope(
			{ ...envelope, schema: QUERY_CACHE_SCHEMA + 1 },
			'acme',
			'partition-a',
			NOW
		),
		null
	);
	assert.equal(
		parsePersistedEnvelope(
			{
				...envelope,
				entries: envelope.entries.map((entry, index) =>
					index === 0 ? { ...entry, dataUpdatedAt: NOW - QUERY_CACHE_MAX_AGE_MS - 1 } : entry
				)
			},
			'acme',
			'partition-a',
			NOW
		),
		null
	);
	assert.equal(parsePersistedEnvelope(envelope, 'acme', 'partition-b', NOW), null);
	assert.equal(
		parsePersistedEnvelope(
			{ ...envelope, savedAt: NOW - QUERY_CACHE_MAX_AGE_MS - 1 },
			'acme',
			'partition-a',
			NOW
		),
		null
	);
	assert.equal(
		parsePersistedEnvelope(
			{
				...envelope,
				entries: [
					...envelope.entries,
					{ queryKey: ['conversation', 'acme', 'exec'], data: { secret: true }, dataUpdatedAt: NOW }
				]
			},
			'acme',
			'partition-a',
			NOW
		),
		null
	);
	assert.equal(parsePersistedEnvelope('{broken json', 'acme', 'partition-a', NOW), null);
});

test('hydration never replaces equally fresh or newer live data', () => {
	assert.equal(shouldHydratePersistedEntry(undefined, 10), true);
	assert.equal(shouldHydratePersistedEntry(9, 10), true);
	assert.equal(shouldHydratePersistedEntry(10, 10), false);
	assert.equal(shouldHydratePersistedEntry(11, 10), false);
});

test('principal lookup uses the current no-store company boundary and validates its shape', async () => {
	const original = globalThis.fetch;
	let observed = { url: '', cache: '', credentials: '' };
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		observed = {
			url: String(input),
			cache: String(init?.cache),
			credentials: String(init?.credentials)
		};
		return new Response(
			JSON.stringify({ actor_id: 'alice', membership_role: 'member', cache_partition: 'opaque-a' }),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;
	try {
		assert.deepEqual(await getCompanyPrincipal('acme group'), {
			actor_id: 'alice',
			membership_role: 'member',
			cache_partition: 'opaque-a'
		});
		assert.deepEqual(observed, {
			url: '/api/companies/acme%20group/principal',
			cache: 'no-store',
			credentials: 'same-origin'
		});
	} finally {
		globalThis.fetch = original;
	}
});

class MemoryStore implements SafeQueryStore {
	values = new Map<string, PersistedQueryEnvelope>();
	clearCount = 0;
	async load(id: string) {
		return this.values.get(id);
	}
	async save(envelope: PersistedQueryEnvelope) {
		this.values.set(envelope.id, envelope);
	}
	async remove(id: string) {
		this.values.delete(id);
	}
	async clearCompany(company: string, keepId?: string) {
		this.clearCount += 1;
		for (const [id, value] of this.values) {
			if (value.company === company && id !== keepId) this.values.delete(id);
		}
	}
}

test('a changed or revoked principal purges affected in-memory and persisted company state', async () => {
	const original = globalThis.fetch;
	let actor = 'alice';
	let role = 'member';
	let partition = 'partition-a';
	let status = 200;
	let invalid = false;
	globalThis.fetch = (async () =>
		status === 200
			? new Response(
					JSON.stringify(
						invalid
							? { actor_id: 'alice', membership_role: 'member' }
							: {
									actor_id: actor,
									membership_role: role,
									cache_partition: partition
								}
					),
					{ status: 200, headers: { 'content-type': 'application/json' } }
				)
			: new Response('', { status })) as typeof fetch;
	const client = new QueryClient();
	const store = new MemoryStore();
	const controller = startCompanyQueryPersistence(client, 'acme', store);
	try {
		await controller.verify();
		client.setQueryData(['attention', 'acme'], attentionView());
		client.setQueryData(['attention', 'other'], { safe: true });
		partition = 'partition-b';
		await controller.verify();
		assert.equal(client.getQueryData(['attention', 'acme']), undefined);
		assert.deepEqual(client.getQueryData(['attention', 'other']), { safe: true });

		client.setQueryData(['attention', 'acme'], attentionView());
		actor = 'bob';
		await controller.verify();
		assert.equal(client.getQueryData(['attention', 'acme']), undefined);

		client.setQueryData(['attention', 'acme'], attentionView());
		role = 'owner';
		await controller.verify();
		assert.equal(client.getQueryData(['attention', 'acme']), undefined);

		client.setQueryData(['attention', 'acme'], attentionView());
		invalid = true;
		await controller.verify();
		assert.equal(client.getQueryData(['attention', 'acme']), undefined);

		client.setQueryData(['attention', 'acme'], attentionView());
		invalid = false;
		status = 404;
		await controller.verify();
		assert.equal(client.getQueryData(['attention', 'acme']), undefined);
		assert.ok(store.clearCount >= 2);
	} finally {
		controller.stop();
		globalThis.fetch = original;
		client.clear();
	}
});

test('verified hydration loads absent data but never clobbers a newer live response', async () => {
	const original = globalThis.fetch;
	globalThis.fetch = (async () =>
		new Response(
			JSON.stringify({
				actor_id: 'alice',
				membership_role: 'member',
				cache_partition: 'partition-a'
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		)) as typeof fetch;
	const now = Date.now();
	const cached = attentionView();
	cached.company.name = 'Cached';
	const envelope = buildPersistedEnvelope(
		'acme',
		'partition-a',
		[{ queryKey: ['attention', 'acme'], data: cached, dataUpdatedAt: now - 1_000 }],
		now
	);
	const store = new MemoryStore();
	store.values.set(envelope.id, envelope);
	const client = new QueryClient();
	const controller = startCompanyQueryPersistence(client, 'acme', store);
	try {
		await controller.verify();
		assert.equal(
			(client.getQueryData(['attention', 'acme']) as ReturnType<typeof attentionView>).company.name,
			'Cached'
		);

		const live = attentionView();
		live.company.name = 'Live';
		client.setQueryData(['attention', 'acme'], live, { updatedAt: now + 1_000 });
		store.values.set(envelope.id, envelope);
		await controller.verify();
		assert.equal(
			(client.getQueryData(['attention', 'acme']) as ReturnType<typeof attentionView>).company.name,
			'Live'
		);
	} finally {
		controller.stop();
		globalThis.fetch = original;
		client.clear();
	}
});
