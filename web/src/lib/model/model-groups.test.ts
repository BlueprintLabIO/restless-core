import assert from 'node:assert/strict';
import test from 'node:test';
import { defaultChoice, filterModels, release, splitModels } from './model-groups.ts';

const choices = (names: string[]) =>
	names.map((name) => ({ id: name.toLowerCase().replaceAll(' ', '-'), name }));

const anthropic = choices([
	'Claude Haiku 5.5',
	'Claude Sonnet 5.5',
	'Claude Opus 5.5',
	'Claude Fable 5.1',
	'Claude Opus 5',
	'Claude Sonnet 5',
	'Claude Fable 5',
	'Claude Opus 4.8',
	'Claude Opus 4.7'
]);

test('the five newest releases stay up front and the rest fold away', () => {
	const { latest, older } = splitModels(anthropic);
	assert.deepEqual(
		latest.map((m) => m.name),
		['Claude Haiku 5.5', 'Claude Sonnet 5.5', 'Claude Opus 5.5', 'Claude Fable 5.1', 'Claude Opus 5']
	);
	assert.deepEqual(
		older.map((m) => m.name),
		['Claude Sonnet 5', 'Claude Fable 5', 'Claude Opus 4.8', 'Claude Opus 4.7']
	);
});

/* The owner saw "Claude Opus 4.5 (latest)" listed among the newest beside 5.5 (2026-10-19): the
 * trailing "(latest)" hid its version, so it counted as a family of its own. */
test('a "(latest)" alias is ranked by its version, not shown as the newest', () => {
	const catalogue = choices([
		'Claude Haiku 5.5',
		'Claude Sonnet 5.5',
		'Claude Opus 5.5',
		'Claude Fable 5.1',
		'Claude Sonnet 5',
		'Claude Opus 4.5 (latest)',
		'Claude Haiku 4.5 (latest)',
		'Claude Sonnet 4.5 (latest)',
		'Claude Opus 4.5'
	]);
	const { latest, older } = splitModels(catalogue, 'claude-sonnet-5');
	assert.deepEqual(
		latest.map((m) => m.name),
		['Claude Haiku 5.5', 'Claude Sonnet 5.5', 'Claude Opus 5.5', 'Claude Fable 5.1', 'Claude Sonnet 5']
	);
	assert.ok(!latest.some((m) => /latest/.test(m.name)));
	assert.ok(older.some((m) => m.name === 'Claude Opus 4.5'));
	assert.ok(!older.some((m) => m.name === 'Claude Opus 4.5 (latest)'), 'an alias of a listed model is left out');
	assert.ok(older.some((m) => m.name === 'Claude Haiku 4.5 (latest)'), 'an alias with no plain twin stays');
});

test('versions are read wherever they sit, and dates are not versions', () => {
	assert.equal(release('GPT-5.5 mini').version, 5.005);
	assert.equal(release('Claude Opus 4.5 (latest)').version, 4.005);
	assert.equal(release('claude-opus-4-5-20251101').version, 4);
	assert.equal(release('Gemini 2.5 Pro').base, 'Gemini 2.5 Pro');
	assert.equal(release('o3').version, 3);
});

test('the model in use is never folded away', () => {
	const { latest } = splitModels(anthropic, 'claude-opus-4.7');
	assert.ok(latest.some((m) => m.id === 'claude-opus-4.7'));
	assert.equal(latest.length, 6);
});

test('filtering matches every word against name and id', () => {
	assert.deepEqual(
		filterModels(anthropic, 'opus 4').map((m) => m.name),
		['Claude Opus 4.8', 'Claude Opus 4.7']
	);
	assert.equal(filterModels(anthropic, '').length, anthropic.length);
});

test('a new connection starts on the newest everyday tier, not the flagship', () => {
	assert.equal(defaultChoice(anthropic)?.name, 'Claude Sonnet 5.5');
	const openai = [
		{ id: 'gpt-6-sol', name: 'GPT-6 Sol', default: true },
		{ id: 'gpt-5.6-terra', name: 'GPT-5.6 Terra' }
	];
	assert.equal(defaultChoice(openai)?.id, 'gpt-5.6-terra');
	assert.equal(defaultChoice([{ id: 'x-large', name: 'X Large', default: true }, { id: 'x-small', name: 'X Small' }])?.id, 'x-large');
});

test("one release listed under two word orders, or twice, shows once", () => {
	const listed = [
		{ id: "claude-sonnet-4-20250514", name: "Claude Sonnet 4" },
		{ id: "claude-4-sonnet", name: "Claude 4 Sonnet" },
		{ id: "claude-sonnet-4-5", name: "Claude Sonnet 4.5" },
		{ id: "claude-sonnet-4-5-20250929", name: "Claude Sonnet 4.5" }
	];
	const { latest, older } = splitModels(listed);
	assert.deepEqual([...latest, ...older].map((m) => m.name), ["Claude Sonnet 4.5", "Claude Sonnet 4"]);
});
