import assert from 'node:assert/strict';
import test from 'node:test';
import { connectionOf, freshness, meterTone, resetsIn } from './account-usage-view.ts';

const now = Date.parse('2026-10-19T01:00:00Z');

test('freshness says how old the number is', () => {
	assert.equal(freshness('2026-10-19T00:59:50Z', now), 'as of just now');
	assert.equal(freshness('2026-10-19T00:56:00Z', now), 'as of 4 min ago');
	assert.equal(freshness('2026-10-18T22:00:00Z', now), 'as of 3 h ago');
});

test('the reset reads as time left', () => {
	assert.equal(resetsIn('2026-10-19T03:41:00Z', now), 'resets in 2 h 41 min');
	assert.equal(resetsIn('2026-10-19T01:20:00Z', now), 'resets in 20 min');
	assert.equal(resetsIn('2026-10-22T01:00:00Z', now), 'resets in 3 days');
	assert.equal(resetsIn(null, now), null);
});

test('a meter warns at 80 and is full at 100', () => {
	assert.equal(meterTone(42), 'ok');
	assert.equal(meterTone(80), 'warn');
	assert.equal(meterTone(100), 'full');
});

test('an account names its connection', () => {
	assert.equal(connectionOf('openai-codex@2f9aea62'), '2f9aea62');
});
