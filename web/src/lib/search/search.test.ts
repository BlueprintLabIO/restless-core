import assert from 'node:assert/strict';
import test from 'node:test';
import { keep, markdownText, matches, occurrences, rank } from './search.ts';

const models = ['Claude Haiku 5.5', 'Claude Opus 5.5', 'Claude Sonnet 5.5', 'Claude Fable 5.1'];

test('lists match every typed word at a word start, in any order, folding case and accents', () => {
	assert.deepEqual(
		rank(models, 'sonnet 5', (m) => m),
		['Claude Sonnet 5.5']
	);
	assert.deepEqual(
		rank(models, 'OPUS claude', (m) => m),
		['Claude Opus 5.5']
	);
	assert.ok(matches('relief gu', ['Integrate with Relief Guild hook']));
	assert.ok(matches('guild relief', ['Integrate with Relief Guild hook']));
	assert.ok(matches('cafe', ['Le Café du Port']));
	// A word start, not anywhere in a word.
	assert.ok(!matches('uild', ['Relief Guild hook']));
});

test('a longer word may carry one typo', () => {
	assert.ok(matches('sonet', ['Claude Sonnet 5.5']));
	assert.ok(!matches('xyzzy', ['Claude Sonnet 5.5']));
});

test('an empty query keeps everything in its own order; keep never reorders', () => {
	assert.deepEqual(
		rank(models, '  ', (m) => m),
		models
	);
	assert.deepEqual(
		keep(models, 'claude 5.5', (m) => m),
		['Claude Haiku 5.5', 'Claude Opus 5.5', 'Claude Sonnet 5.5']
	);
	assert.ok(matches('', ['anything']));
});

test('rows match on any of their fields', () => {
	const rows = [
		{ title: 'Write the deck', owner: 'Ada Lovelace' },
		{ title: 'Price the stock', owner: null }
	];
	assert.deepEqual(
		keep(rows, 'ada', (row) => [row.title, row.owner]).map((row) => row.title),
		['Write the deck']
	);
});

test('find counts every occurrence and points into the original text', () => {
	const text = 'Do you want the Relief Guild hook? Integrate with relief  guild, or without.';
	const found = occurrences(text, 'relief gu');
	assert.equal(found.length, 2);
	assert.equal(text.slice(...found[0]), 'Relief Gu');
	assert.equal(text.slice(...found[1]), 'relief  gu');
	const accented = 'Le Café, then the café again';
	assert.deepEqual(
		occurrences(accented, 'cafe').map((range) => accented.slice(...range)),
		['Café', 'café']
	);
	assert.deepEqual(occurrences(text, ' '), []);
});

test('markdown reads as it shows', () => {
	assert.equal(
		markdownText('**Needs you:** the [Relief Guild](https://x) hook, `500g`'),
		'Needs you: the Relief Guild hook, 500g'
	);
	assert.equal(markdownText('- item one\n## Heading'), 'item one\nHeading');
	assert.equal(markdownText('an _emphasis_ word'), 'an emphasis word');
});
