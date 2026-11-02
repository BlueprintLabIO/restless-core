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
test('an avatar never holds a digit: a repeated character keeps its number in its name only', () => {
	assert.equal(initials('Eames 2'), 'Ea');
	assert.equal(initials('Ebenezer Scrooge 20'), 'Eb');
	assert.equal(initials('R2-D2'), 'Rd');
	for (const name of ['Eeyore 2', 'Echo 3', 'Q*bert', 'Wall-E 2'])
		assert.doesNotMatch(initials(name), /\d/);
});
test('character punctuation and a single-letter name remain readable', () => {
	assert.equal(initials('Q*bert'), 'Qb');
	assert.equal(initials('Q'), 'Q');
	assert.equal(initials('Xûr'), 'Xû');
});
test('empty labels and the executive keep their existing presentation', () => {
	assert.equal(initials(''), '');
	assert.equal(initials('  The Exec '), 'Ex');
	assert.equal(initials('Owner'), 'Ow');
});
