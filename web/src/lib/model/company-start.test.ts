import assert from 'node:assert/strict';
import test from 'node:test';
import {
	accountGrantFix,
	emptyAccessStep,
	type AccountConnectionSummary
} from './company-start.ts';

const codex = (over: Partial<AccountConnectionSummary> = {}): AccountConnectionSummary => ({
	id: 'conn-codex',
	label: 'ChatGPT',
	provider: 'openai-codex',
	kind: 'oauth',
	status: 'present',
	companies: [],
	...over
});
const claude: AccountConnectionSummary = {
	id: 'conn-claude',
	label: 'Claude',
	provider: 'anthropic',
	kind: 'oauth',
	status: 'present',
	companies: []
};
const reconnectCodex = 'Reconnect ChatGPT / Codex in Company -> Intelligence.';

test('M1: a working sign-in no company uses offers the grant, never another sign-in', () => {
	const step = emptyAccessStep(codex(), [{ id: 'acme', name: 'Acme' }], [codex()]);
	assert.deepEqual(step, {
		sentence: 'Signed in. No company uses it yet.',
		action: 'Give Acme access',
		companyId: 'acme'
	});
	assert.doesNotMatch(`${step?.sentence} ${step?.action}`, /sign in|reconnect/i);
});

test('M1: the company waiting on exactly this connection is preselected among several', () => {
	const companies = [
		{ id: 'alpha', name: 'Alpha' },
		{ id: 'waiting', name: 'Waiting Co', unstartable_reason: reconnectCodex },
		{
			id: 'other',
			name: 'Other',
			unstartable_reason: 'Reconnect Claude in Company -> Intelligence.'
		}
	];
	const step = emptyAccessStep(codex(), companies, [codex(), claude]);
	assert.equal(step?.companyId, 'waiting');
	assert.equal(step?.action, 'Give Waiting Co access');
	// The Claude row is not claimed by the company waiting on Codex.
	assert.equal(emptyAccessStep(claude, companies.slice(0, 2), [codex(), claude])?.companyId, null);
});

test('M1: with several companies and none waiting, the owner chooses; nothing is preselected', () => {
	const step = emptyAccessStep(
		codex(),
		[
			{ id: 'a', name: 'A' },
			{ id: 'b', name: 'B' }
		],
		[codex()]
	);
	assert.deepEqual(step, {
		sentence: 'Signed in. No company uses it yet.',
		action: 'Choose a company',
		companyId: null
	});
});

test('M1: no next step for a broken sign-in, an already granted one, or no companies to grant', () => {
	const companies = [{ id: 'acme', name: 'Acme' }];
	assert.equal(emptyAccessStep(codex({ status: 'invalid' }), companies, []), null);
	assert.equal(emptyAccessStep(codex({ status: 'absent' }), companies, []), null);
	assert.equal(emptyAccessStep(codex({ companies: [{ id: 'acme' }] }), companies, []), null);
	assert.deepEqual(emptyAccessStep(codex(), [], [codex()]), {
		sentence: 'Signed in. No company to give it to yet.',
		action: null,
		companyId: null
	});
});

test('M1: an API key that works is not described as a sign-in', () => {
	const key = codex({ id: 'key', provider: 'openai', kind: 'api_key' });
	assert.equal(
		emptyAccessStep(key, [{ id: 'acme', name: 'Acme' }], [key])?.sentence,
		'Key works. No company uses it yet.'
	);
});

test('M1: Fix setup reference still links the grant with the company and connection preselected', () => {
	const fix = accountGrantFix(reconnectCodex, 'acme', [codex()]);
	assert.equal(fix?.href, '/account/settings/accounts?grant=acme&connection=conn-codex');
	assert.equal(fix?.label, 'Use your ChatGPT / Codex sign-in');
});
