import assert from 'node:assert/strict';
import test from 'node:test';
import {
	fallbackAccounts,
	fallbackRoute,
	routeSentence,
	type FallbackConnection
} from './model-failover.ts';

/* Shapes as GET /intelligence lists them (owner.rs intelligence_view). */
const claude: FallbackConnection = {
	id: 'account:anthropic@acct-1',
	provider: 'anthropic',
	kind: 'direct',
	account_kind: 'oauth',
	label: 'Work Claude',
	loaded: true
};
const chatgpt: FallbackConnection = {
	id: 'account:openai-codex@acct-2',
	provider: 'openai-codex',
	kind: 'direct',
	account_kind: 'oauth',
	label: 'ChatGPT',
	loaded: true
};
const claudeHarness: FallbackConnection = {
	id: 'account-harness:claude-agent:anthropic@acct-1',
	provider: 'claude-agent',
	account_provider: 'anthropic',
	kind: 'harness',
	label: 'Work Claude'
};
const ownKey: FallbackConnection = { id: 'direct:groq', provider: 'groq', kind: 'direct' };
const included: FallbackConnection = {
	id: 'direct:deepseek',
	provider: 'deepseek',
	kind: 'direct',
	included: true
};

test('M5: a fallback names the account that pays for it before it is saved', () => {
	const all = [claude, claudeHarness, chatgpt, included];
	const route = fallbackRoute('anthropic/claude-sonnet-4-6', all);
	assert.equal(route.state, 'billed');
	assert.equal(route.state === 'billed' && route.connection.id, claude.id);
	assert.equal(routeSentence(route, all), 'Bills your Anthropic · Work Claude account');
	assert.equal(
		routeSentence(fallbackRoute('deepseek/deepseek-v4', all), all),
		'Uses the included AI in your plan'
	);
	assert.equal(
		routeSentence(fallbackRoute('groq/llama-5', [ownKey]), [ownKey]),
		"Bills this company's own Groq key"
	);
});

test('M5: a harness sign-in never counts as the billing route for a gateway fallback', () => {
	const route = fallbackRoute('anthropic/claude-sonnet-4-6', [claudeHarness]);
	assert.equal(route.state, 'none');
	assert.match(routeSentence(route, [claudeHarness]), /^Not usable/);
});

test('M5: a provider without a working connection here is reported unusable, not billed', () => {
	// An inaccessible account (signed out, or not granted) is absent from the view.
	assert.equal(fallbackRoute('openai/gpt-6', [claude, chatgpt]).state, 'none');
	assert.equal(fallbackRoute('no-slash', [claude]).state, 'none');
	assert.deepEqual(fallbackAccounts([]), []);
});

test('M5: two connections for one provider are ambiguous and cannot be chosen', () => {
	const second = { ...claude, id: 'account:anthropic@acct-9', label: 'Personal Claude' };
	const all = [claude, second, chatgpt];
	const route = fallbackRoute('anthropic/claude-sonnet-4-6', all);
	assert.equal(route.state, 'ambiguous');
	assert.match(routeSentence(route, all), /unclear which would pay/);
	assert.deepEqual(
		fallbackAccounts(all).map((row) => row.id),
		[chatgpt.id]
	);
});

test('M5: owner accounts are offered first, then company keys, then included AI', () => {
	assert.deepEqual(
		fallbackAccounts([included, ownKey, claudeHarness, chatgpt, claude]).map((row) => row.id),
		[chatgpt.id, claude.id, ownKey.id, included.id]
	);
});
