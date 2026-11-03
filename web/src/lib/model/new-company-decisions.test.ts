import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decisionChoice, decisionSentence } from './new-company-decisions.ts';

test('a decision default reads in the words its choices use', () => {
	assert.equal(decisionChoice(null), 'watching');
	assert.equal(decisionSentence(undefined), 'Included, watching only');
	assert.equal(decisionChoice('included'), 'included');
	assert.equal(decisionSentence('included'), 'Included');
	assert.equal(decisionChoice('typesafe/jev-1.14'), 'openrouter');
	assert.match(decisionSentence('typesafe/jev-1.14'), /on OpenRouter$/);
});
