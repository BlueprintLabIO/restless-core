import assert from 'node:assert/strict';
import test from 'node:test';
import {
	abilitySummary,
	accountInput,
	blankAccountForm,
	grantDraft,
	grantInput,
	missingForAccount
} from './account-forms.ts';
import { DEFAULT_ABILITIES } from './owner-accounts.ts';

test('a site an agent named fills in where that kind of account takes it', () => {
	assert.equal(blankAccountForm('website_login', 'ebay.com.au/signin').url, 'ebay.com.au/signin');
	assert.equal(blankAccountForm('api', 'api.lumen.test').baseUrl, 'api.lumen.test');
	assert.equal(blankAccountForm('github', 'github.com').url, '');
});

test('the form says what it still needs, one thing at a time', () => {
	const form = blankAccountForm('website_login', 'ebay.com.au');
	assert.equal(missingForAccount('website_login', form), 'Add the username or email.');
	form.username = 'maya';
	assert.equal(missingForAccount('website_login', form), 'Add the password.');
	form.password = 'x';
	assert.equal(missingForAccount('website_login', form), '');
	const api = {
		...blankAccountForm('api', 'api.lumen.test'),
		secret: 'k',
		auth: 'header' as const
	};
	assert.equal(missingForAccount('api', api), 'Add the header name.');
});

test('a website login goes to the plane with a full address and its domain', () => {
	const input = accountInput('website_login', {
		...blankAccountForm('website_login', 'ebay.com.au/signin'),
		username: ' maya ',
		password: 'pw'
	});
	assert.equal(input.label, 'ebay.com.au');
	assert.deepEqual(input.credential, {
		kind: 'password',
		sign_in_url: 'https://ebay.com.au/signin',
		domains: ['ebay.com.au'],
		username: 'maya',
		password: 'pw'
	});
});

test('a grant draft round-trips the limits and only caps spend that is allowed', () => {
	const draft = grantDraft(null, 'reno', DEFAULT_ABILITIES);
	assert.equal(draft.company, 'reno');
	draft.capOn = true;
	draft.capDollars = '25';
	draft.scope = 'renostudio.au, , godaddy.com';
	const input = grantInput(draft, 'Australia/Sydney');
	assert.deepEqual(input.spend_cap, { amount_micros: 25_000_000, period: 'week' });
	assert.deepEqual(input.limits.scope, ['renostudio.au', 'godaddy.com']);
	assert.equal(input.who, 'everyone');
	draft.abilities = { ...draft.abilities, spend: 'off' };
	assert.equal(grantInput(draft).spend_cap, null);
});

test('an access level reads as one sentence', () => {
	assert.equal(
		abilitySummary(DEFAULT_ABILITIES),
		'Read and Draft freely, Act, telling you, asks before Spend, no Admin.'
	);
});
