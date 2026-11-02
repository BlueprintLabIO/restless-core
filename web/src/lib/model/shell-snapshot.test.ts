import assert from 'node:assert/strict';
import test from 'node:test';
import {
	isPane,
	shellFor,
	shellPanel,
	shellSection,
	withSection,
	type ShellSnapshot
} from './shell-snapshot.ts';

const paint = {
	backgroundColor: 'rgb(250, 251, 254)',
	borderRadius: '6px',
	borderWidth: '1px',
	borderStyle: 'solid',
	borderColor: 'rgba(76, 88, 117, 0.18)',
	boxShadow: 'none'
};

test('a pane is kept as insets from the window edges, so a taller window still fits it', () => {
	/* Reno's Work pane and Exec rail at 1062 × 588 (2 Nov). */
	const viewport = { width: 1062, height: 588 };
	assert.deepEqual(shellPanel({ x: 699, y: 60, width: 355, height: 520 }, paint, viewport), {
		t: 60,
		l: 699,
		r: 8,
		b: 8,
		bg: 'rgb(250, 251, 254)',
		radius: '6px',
		bw: '1px',
		bs: 'solid',
		bc: 'rgba(76, 88, 117, 0.18)',
		shadow: 'none'
	});
	assert.equal(shellPanel({ x: 1100, y: 60, width: 300, height: 500 }, paint, viewport), null);
	assert.equal(shellPanel({ x: 0, y: 0, width: 0, height: 0 }, paint, viewport), null);
});

test('only a filled surface of pane size counts as a pane', () => {
	assert.equal(isPane({ x: 0, y: 0, width: 687, height: 520 }, paint), true);
	assert.equal(
		isPane({ x: 0, y: 0, width: 687, height: 520 }, { backgroundColor: 'rgba(0, 0, 0, 0)' }),
		false
	);
	assert.equal(isPane({ x: 0, y: 0, width: 687, height: 40 }, paint), false);
});

test('each section keeps its own layout, newest last, and the oldest give way', () => {
	const shot = (width: number): ShellSnapshot => ({ w: width, theme: 'light', bg: '', panels: [] });
	assert.equal(shellSection('/acme/work'), 'work');
	assert.equal(shellSection('/acme/company/charter'), 'company');
	assert.equal(shellSection('/acme'), '');
	let stored = withSection(null, 'work', shot(1));
	stored = withSection(stored, '', shot(2));
	stored = withSection(stored, 'work', shot(3));
	assert.equal(stored.last, 'work');
	assert.deepEqual(Object.keys(stored.sections), ['', 'work']);
	for (let index = 0; index < 20; index++) stored = withSection(stored, `s${index}`, shot(index));
	assert.equal(Object.keys(stored.sections).length, 12);
	assert.equal(stored.sections.s19.w, 19);
	/* A record from another version starts over. */
	assert.deepEqual(Object.keys(withSection({ ...stored, v: 0 }, 'work', shot(1)).sections), [
		'work'
	]);
});

test('a snapshot is drawn only at the width and theme it was taken at', () => {
	const stored = withSection(null, 'work', { w: 1440, theme: 'light', bg: '', panels: [] });
	assert.ok(shellFor(stored, '/acme/work', 1440, 'light'));
	assert.ok(shellFor(stored, '/acme/library', 1440, 'light'));
	assert.equal(shellFor(stored, '/acme/work', 1439, 'light'), null);
	assert.equal(shellFor(stored, '/acme/work', 1440, 'dark'), null);
	assert.equal(shellFor(null, '/acme/work', 1440, 'light'), null);
});
