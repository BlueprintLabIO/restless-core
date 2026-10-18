import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	DEFAULT_ABILITIES,
	EMPTY_LIMITS,
	grantSummary,
	limitsSummary,
	money,
	presetOf
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
