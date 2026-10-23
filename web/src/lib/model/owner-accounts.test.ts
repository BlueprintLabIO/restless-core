import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	DEFAULT_ABILITIES,
	EMPTY_LIMITS,
	grantSummary,
	kindPhrase,
	limitsSummary,
	money,
	presetOf,
	useRefLine,
	emptyAccessGrant,
	useSentence
} from './owner-accounts.ts';

test('a new grant is permissive: read and draft allowed, act tells, spend asks, admin off', () => {
	assert.deepEqual(DEFAULT_ABILITIES, {
		read: 'allow',
		draft: 'allow',
		act: 'tell',
		spend: 'ask',
		admin: 'off'
	});
	assert.equal(presetOf(DEFAULT_ABILITIES), 'standard');
});

test('a grant reads as one plain line', () => {
	assert.equal(
		grantSummary({ abilities: DEFAULT_ABILITIES, spend_cap: null }),
		'Read · Draft · Act, tells you · Spend after asking'
	);
	assert.equal(
		grantSummary({
			abilities: { ...DEFAULT_ABILITIES, spend: 'ask' },
			spend_cap: { amount_micros: 50_000_000, period: 'week' }
		}),
		'Read · Draft · Act, tells you · Spend up to $50 a week, then asks'
	);
	assert.equal(
		grantSummary({
			abilities: { read: 'off', draft: 'off', act: 'off', spend: 'off', admin: 'off' },
			spend_cap: null
		}),
		'No access'
	);
});

test('presets are recognised, and anything else is custom', () => {
	assert.equal(
		presetOf({ read: 'allow', draft: 'off', act: 'off', spend: 'off', admin: 'off' }),
		'read'
	);
	assert.equal(presetOf({ ...DEFAULT_ABILITIES, admin: 'ask' }), null);
});

test('limits read in plain words, and none means nothing to show', () => {
	assert.deepEqual(limitsSummary(EMPTY_LIMITS, 'everyone'), []);
	assert.deepEqual(
		limitsSummary(
			{
				...EMPTY_LIMITS,
				scope: ['restless-core'],
				actions_per_day: 20,
				expires_at: '2026-12-01T00:00:00Z'
			},
			{ actors: ['Bianca'] }
		),
		['Only restless-core', 'Up to 20 actions a day', 'Until 2026-12-01', 'Only Bianca']
	);
	assert.equal(money(12_500_000), '$12.50');
});

test('a use reads as one sentence naming who asked, and leaves out what the page already says', () => {
	const use = {
		actor: 'actor_exec',
		actor_display: 'Exec',
		directed_by: 'actor_bianca',
		directed_by_display: 'Bianca',
		action: 'listed',
		target: 'Canon AE-1'
	};
	assert.equal(
		useSentence(use, { account: 'eBay', company: 'Sydney Resale' }),
		'Exec, at Bianca’s request, listed “Canon AE-1”, using eBay in Sydney Resale.'
	);
	assert.equal(useSentence(use), 'Exec, at Bianca’s request, listed “Canon AE-1”.');
	assert.equal(
		useSentence({ actor: 'Exec', action: 'orders.read', target: null }),
		'Exec orders read.'
	);
	assert.equal(
		useSentence({
			actor: 'Exec',
			directed_by_display: 'Exec',
			action: 'read orders',
			target: null
		}),
		'Exec read orders.'
	);
});

test('a use on an Inbox item or a message is one short line', () => {
	assert.equal(
		useRefLine({
			account: { id: 'a', label: 'eBay', kind: 'website_login', provider: 'ebay.com.au' },
			grant_id: 'g',
			company: 'c',
			tier: 'spend',
			action: 'bought postage',
			target: null,
			actor_display: 'Exec',
			directed_by_display: 'Bianca',
			cost_micros: 12_500_000
		}),
		'eBay · Spend · Exec, for Bianca · $12.50'
	);
});

test('a kind reads as a phrase in a sentence', () => {
	assert.equal(kindPhrase('api'), 'an API key');
	assert.equal(kindPhrase('website_login'), 'a website login');
	assert.equal(kindPhrase('github'), 'a GitHub');
	assert.equal(kindPhrase('ai_provider'), 'an AI account');
});

test('M1: a signed-in account with no company offers the grant, preselecting a lone company', () => {
	const account = { health: 'signed_in' as const, frozen: false, grants: [] };
	assert.deepEqual(emptyAccessGrant(account, [{ id: 'acme', name: 'Acme' }]), {
		label: 'Give Acme access',
		company: 'acme'
	});
	assert.deepEqual(
		emptyAccessGrant(account, [
			{ id: 'acme', name: 'Acme' },
			{ id: 'beta', name: 'Beta' }
		]),
		{ label: 'Give a company access', company: '' }
	);
	assert.equal(emptyAccessGrant(account, []), null);
	assert.equal(emptyAccessGrant({ ...account, health: 'expired' }, [{ id: 'a', name: 'A' }]), null);
	assert.equal(emptyAccessGrant({ ...account, frozen: true }, [{ id: 'a', name: 'A' }]), null);
});
