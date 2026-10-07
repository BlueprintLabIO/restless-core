import { test } from 'node:test';
import assert from 'node:assert/strict';
import { answerIndex, excerptOf, openQuestionIndex } from './open-question.ts';

const ask = (need: string) => ({
	from: 'exec',
	text: 'Two things to confirm.',
	intent: { ownerNeed: need }
});
const said = (text: string) => ({ from: 'you', text });
const answer = (need: string, text: string) => said(`> Exec: ${excerptOf(need)}\n\n${text}`);
const NEED = 'Confirm A$4,800 or a different price, and give the booking email or link';

test('an ordinary message leaves the question open', () => {
	const thread = [ask(NEED), said('This is not an answer: is the VM alive?')];
	assert.equal(openQuestionIndex(thread, 'Exec'), 0);
	assert.equal(answerIndex(thread, 'Exec', 0), -1);
});

test('an agent reply in between leaves it open too', () => {
	const thread = [ask(NEED), said('ping'), { from: 'exec', text: 'Still waiting on price.' }];
	assert.equal(openQuestionIndex(thread, 'Exec'), 0);
});

test('a reply quoting the question answers it', () => {
	const thread = [ask(NEED), said('ping'), answer(NEED, 'Price: A$4,800')];
	assert.equal(openQuestionIndex(thread, 'Exec'), -1);
	assert.equal(answerIndex(thread, 'Exec', 0), 2);
});

test('a newer question supersedes the older one', () => {
	const thread = [ask(NEED), said('ping'), ask('Which day suits?')];
	assert.equal(openQuestionIndex(thread, 'Exec'), 2);
	assert.equal(answerIndex(thread, 'Exec', 0), -1);
});
