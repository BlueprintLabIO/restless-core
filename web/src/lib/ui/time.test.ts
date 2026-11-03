import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatTimeLeft } from './time.ts';

test('time left keeps one width as it counts down, and never goes below zero', () => {
	assert.equal(formatTimeLeft(120_000), '2:00');
	assert.equal(formatTimeLeft(119_001), '2:00');
	assert.equal(formatTimeLeft(30_000), '0:30');
	assert.equal(formatTimeLeft(9_400), '0:10');
	assert.equal(formatTimeLeft(1), '0:01');
	assert.equal(formatTimeLeft(0), '0:00');
	assert.equal(formatTimeLeft(-5_000), '0:00');
	for (const ms of [120_000, 61_000, 30_000, 5_000, 0])
		assert.equal(formatTimeLeft(ms).length, 4, `${ms}`);
});
