import assert from 'node:assert/strict';
import test from 'node:test';
import { sheetChanges } from './sheet-diff.ts';

const book = (sheets: { id: string; name: string; cells: Record<string, unknown> }[]) => ({
	sheets
});

test('changed, added and cleared cells read down the sheet', () => {
	const before = book([{ id: 's1', name: 'Budget', cells: { B2: '40', A1: 'Item', C10: 'x' } }]);
	const after = book([
		{ id: 's1', name: 'Budget', cells: { A1: 'Item', B2: '45', A3: '=SUM(B1:B2)' } }
	]);
	assert.deepEqual(sheetChanges(before, after), {
		tabs: [],
		cells: [
			{ tab: 'Budget', cell: 'B2', before: '40', after: '45' },
			{ tab: 'Budget', cell: 'A3', before: '', after: '=SUM(B1:B2)' },
			{ tab: 'Budget', cell: 'C10', before: 'x', after: '' }
		],
		more: 0
	});
});

test('tabs are matched by id: renamed, added and removed', () => {
	const before = book([
		{ id: 's1', name: 'Sheet1', cells: {} },
		{ id: 's2', name: 'Old', cells: { A1: 'gone' } }
	]);
	const after = book([
		{ id: 's1', name: 'Plan', cells: {} },
		{ id: 's3', name: 'New', cells: { A1: { content: 'hi' } } }
	]);
	const changes = sheetChanges(before, after);
	assert.deepEqual(changes.tabs, [
		{ tab: 'Plan', kind: 'renamed', from: 'Sheet1' },
		{ tab: 'New', kind: 'added' },
		{ tab: 'Old', kind: 'removed' }
	]);
	assert.deepEqual(
		changes.cells.map((change) => [change.tab, change.cell, change.before, change.after]),
		[
			['New', 'A1', '', 'hi'],
			['Old', 'A1', 'gone', '']
		]
	);
});

test('a long list says how many more changed', () => {
	const cells = Object.fromEntries(Array.from({ length: 30 }, (_, i) => [`A${i + 1}`, String(i)]));
	const changes = sheetChanges(
		book([{ id: 's', name: 'S', cells: {} }]),
		book([{ id: 's', name: 'S', cells }]),
		10
	);
	assert.equal(changes.cells.length, 10);
	assert.equal(changes.more, 20);
	assert.equal(changes.cells[0].cell, 'A1');
	assert.equal(changes.cells[9].cell, 'A10');
});

test('the first version is everything in it', () => {
	const changes = sheetChanges(null, book([{ id: 's', name: 'S', cells: { A1: 'a' } }]));
	assert.deepEqual(changes.tabs, [{ tab: 'S', kind: 'added' }]);
	assert.equal(changes.cells.length, 1);
});
