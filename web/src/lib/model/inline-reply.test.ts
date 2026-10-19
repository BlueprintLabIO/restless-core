import assert from 'node:assert/strict';
import test from 'node:test';
import { quickAnswers, replyBody } from './inline-reply.ts';

test('a question with two choices offers both as answers', () => {
	assert.deepEqual(quickAnswers('Needs you: quote S&I at A$1,000 for five pages, or A$500 for the single page?'), [
		'A$1,000 for five pages',
		'A$500 for the single page'
	]);
	assert.deepEqual(quickAnswers('Needs you: announce 3 November or 10 November?'), ['3 November', '10 November']);
	assert.deepEqual(
		quickAnswers('do you want me to ask the lead to update the offers pack for S&I at A$500, or leave it until you’re ready to send?'),
		['Ask the lead to update the offers pack for S&I at A$500', 'Leave it until you’re ready to send']
	);
});

test('an open question offers nothing to click', () => {
	assert.deepEqual(quickAnswers('What should the launch post say about pricing?'), []);
});

test('the reply quotes what it answers', () => {
	assert.equal(replyBody('Quote A$500 or A$1,000?', 'A$500'), '> Quote A$500 or A$1,000?\n\nA$500');
});
