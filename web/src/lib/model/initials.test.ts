import assert from 'node:assert/strict';
import test from 'node:test';
import { initials } from './initials.ts';

test('fictional team characters retain their ordered prefix in multiword names', () => {
	assert.deepEqual(['Eames', 'Ebenezer Scrooge', 'Echo', 'Edna Mode', 'Eeyore'].map(initials), [
		'Ea',
		'Eb',
		'Ec',
		'Ed',
		'Ee'
	]);
});
test('repeated characters stay distinct after a name pool fills', () => {
	assert.equal(initials('Eames 2'), 'Ea2');
	assert.equal(initials('Ebenezer Scrooge 20'), 'Eb20');
	assert.notEqual(initials('Eeyore'), initials('Eeyore 2'));
});
test('character punctuation and a single-letter name remain readable', () => {
	assert.equal(initials('Q*bert'), 'Qb');
	assert.equal(initials('Q'), 'Q');
	assert.equal(initials('R2-D2'), 'R2');
	assert.equal(initials('Xûr'), 'Xû');
});
test('empty labels and the executive keep their existing presentation', () => {
	assert.equal(initials(''), '');
	assert.equal(initials('  The Exec '), 'Ex');
	assert.equal(initials('Owner'), 'Ow');
});
