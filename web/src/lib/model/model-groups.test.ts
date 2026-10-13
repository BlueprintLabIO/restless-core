import assert from 'node:assert/strict';
import test from 'node:test';
import { filterModels, splitModels } from './model-groups.ts';

const anthropic = [
	'Claude Haiku 5.5',
	'Claude Sonnet 5.5',
	'Claude Opus 5.5',
	'Claude Fable 5.1',
	'Claude Opus 5',
	'Claude Sonnet 5',
	'Claude Fable 5',
	'Claude Opus 4.8',
	'Claude Opus 4.7'
].map((name) => ({ id: name.toLowerCase().replaceAll(' ', '-'), name }));

test('the newest of each family stays up front and older versions fold away', () => {
	const { latest, older } = splitModels(anthropic);
	assert.deepEqual(
		latest.map((m) => m.name),
		['Claude Haiku 5.5', 'Claude Sonnet 5.5', 'Claude Opus 5.5', 'Claude Fable 5.1']
	);
	assert.equal(older.length, 5);
});

test('the model in use is never folded away', () => {
	const { latest } = splitModels(anthropic, 'claude-opus-4.7');
	assert.ok(latest.some((m) => m.id === 'claude-opus-4.7'));
});

test('filtering matches every word against name and id', () => {
	assert.deepEqual(
		filterModels(anthropic, 'opus 4').map((m) => m.name),
		['Claude Opus 4.8', 'Claude Opus 4.7']
	);
	assert.equal(filterModels(anthropic, '').length, anthropic.length);
});
