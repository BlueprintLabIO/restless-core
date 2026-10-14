import assert from 'node:assert/strict';
import test from 'node:test';
import { paragraphChanges, proposalChanges, versionChanges, wordChanges } from './document-diff.ts';
import type { DocumentContent } from './documents.ts';

const doc = (...blocks: [string, string][]): DocumentContent => ({
	type: 'doc',
	content: blocks.map(([id, text]) => ({
		type: 'paragraph',
		attrs: { block_id: id },
		content: text ? [{ type: 'text', text }] : undefined
	}))
});

test('a reworded block names the words that changed, and nothing else', () => {
	assert.deepEqual(wordChanges('Price is 10 dollars', 'Price is 12 dollars'), [
		{ op: 'same', text: 'Price is ' },
		{ op: 'removed', text: '10' },
		{ op: 'added', text: '12' },
		{ op: 'same', text: ' dollars' }
	]);
});

test('blocks are matched by identity: added, removed where it stood, and reworded', () => {
	const before = doc(['a', 'Intro'], ['b', 'Old plan'], ['c', 'Price is 10'], ['d', 'Close']);
	const after = doc(['a', 'Intro'], ['c', 'Price is 12'], ['n', 'A new line'], ['d', 'Close']);
	const changes = versionChanges(before, after);
	assert.deepEqual(
		changes.map((change) => [change.kind, change.id]),
		[
			['removed', 'b'],
			['changed', 'c'],
			['added', 'n']
		]
	);
});

test('a moved block that kept its words is not a change, and the first version is all new', () => {
	const before = doc(['a', 'One'], ['b', 'Two']);
	const after = doc(['b', 'Two'], ['a', 'One']);
	assert.deepEqual(versionChanges(before, after), []);
	assert.deepEqual(
		versionChanges(null, before).map((change) => change.kind),
		['added', 'added']
	);
});

test('a one-block proposal reads as that block reworded, nothing else', () => {
	const current = doc(['a', 'Intro'], ['b', 'Price is 10'], ['c', 'Close']);
	const proposed = { type: 'paragraph', content: [{ type: 'text', text: 'Price is 12' }] };
	const changes = proposalChanges(current, proposed, 'b');
	assert.deepEqual(
		changes.map((change) => [change.kind, change.id]),
		[['changed', 'b']]
	);
});

test('a charter compares paragraph by paragraph: kept ones are left out, a reworded one names its words', () => {
	const before = '# Acme\n\nWe bake bread.\n\nOpen daily.\n\nNo delivery.';
	const after =
		'# Acme\n\nWe bake sourdough bread.\n\nOpen daily.\n\nDelivery on Fridays.\n\nA new rule.';
	assert.deepEqual(
		paragraphChanges(before, after).map((change) =>
			change.kind === 'changed'
				? ['changed', change.words.filter((part) => part.op !== 'same').map((part) => part.text)]
				: [change.kind, change.text]
		),
		[
			['changed', ['sourdough ']],
			['changed', ['No', 'Delivery', 'delivery.', 'on Fridays.']],
			['added', 'A new rule.']
		]
	);
});
