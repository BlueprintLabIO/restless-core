import assert from 'node:assert/strict';
import test from 'node:test';
import type { IntelligenceView } from './intelligence.svelte.ts';
import type { ReusableConnection } from './provider.ts';
import {
	accountName,
	includedRoute,
	offerSentence,
	setupAccountOffer,
	startingModel,
	usableAccountConnections
} from './setup-model-offer.ts';

const view = (included: boolean): IntelligenceView =>
	({
		revision: 'r1',
		default: null,
		base_default: { connection: 'direct:litellm', model: 'litellm/deepseek-v4.1-flash' },
		has_connections: true,
		connections: [
			{ id: 'direct:litellm', provider: 'litellm', kind: 'direct', included, loaded: true }
		],
		agents: [{ id: 'exec', name: 'Exec', role: 'exec', assignment: null, effective_model: '' }]
	}) as unknown as IntelligenceView;

const row = (over: Partial<ReusableConnection>): ReusableConnection =>
	({ id: 'c', label: '', provider: 'openai', kind: 'api_key', status: 'present', companies: [], ...over }) as ReusableConnection;
const chatgpt = row({ id: 'chatgpt', provider: 'openai-codex', kind: 'oauth', label: 'me@x' });
const claude = row({ id: 'claude', provider: 'anthropic', kind: 'oauth' });
const key = row({ id: 'key', provider: 'openai', label: 'Work key' });
const relayKey = row({ id: 'relay', provider: 'litellm', label: 'Gateway' });
const broken = row({ id: 'broken', provider: 'anthropic', kind: 'oauth', status: 'invalid' });

const offer = (over: Partial<Parameters<typeof setupAccountOffer>[0]> = {}) =>
	setupAccountOffer({
		view: view(true),
		connections: [key, broken, chatgpt, relayKey, claude],
		includedProviders: ['litellm'],
		chosen: false,
		interviewStarted: false,
		...over
	});

test('M6: a company on the included AI is offered the owner’s working accounts, sign-ins first', () => {
	assert.deepEqual(
		offer().map((r) => r.id),
		['chatgpt', 'claude', 'key']
	);
	assert.equal(
		offerSentence(offer().map((r) => accountName(r))),
		'ChatGPT, Claude and Work key'
	);
});

test('M6: no offer when Exec already thinks with the owner’s own account', () => {
	assert.equal(includedRoute(view(false)), null);
	assert.deepEqual(offer({ view: view(false) }), []);
	assert.deepEqual(offer({ view: null }), []);
});

test('M6: the offer never interrupts a choice made or an interview under way', () => {
	assert.deepEqual(offer({ chosen: true }), []);
	assert.deepEqual(offer({ interviewStarted: true }), []);
});

test('M6: broken sign-ins and providers the included AI serves are never offered', () => {
	const ids = usableAccountConnections([broken, relayKey, chatgpt], ['litellm']).map((r) => r.id);
	assert.deepEqual(ids, ['chatgpt']);
});

test('M6: the starting model is written provider/model from the everyday choice', () => {
	const models = [
		{ id: 'gpt-5.6-sol', name: 'Sol' },
		{ id: 'gpt-5.6-terra', name: 'Terra', default: true }
	];
	assert.equal(startingModel('openai-codex', models as never), 'openai-codex/gpt-5.6-terra');
	assert.equal(startingModel('openai-codex', []), '');
});
