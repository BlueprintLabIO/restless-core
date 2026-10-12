import assert from 'node:assert/strict';
import test from 'node:test';

import {
	caretColor,
	documentFaces,
	sheetFaces,
	visibleFaces,
	type PageFace
} from './page-faces.ts';

const names: Record<string, string> = { bea: 'Bea', exec: 'The Exec', staff: 'Rowan' };
const nameOf = (actorId: string) => names[actorId] ?? 'Someone';

test('a document shows one face per other actor, editing if any of their tabs is', () => {
	const states = new Map<number, unknown>([
		[1, { user: { actor_id: 'me' }, cursor: { anchor: 1, head: 1 }, editing: true }],
		[2, { user: { actor_id: 'bea' } }],
		[3, { user: { actor_id: 'bea' }, editing: true }],
		[4, { user: { actor_id: 'exec' } }],
		[5, {}],
		[6, { user: { actor_id: '' } }]
	]);
	assert.deepEqual(documentFaces(states, 'me', nameOf), [
		{ actorId: 'bea', name: 'Bea', editing: true, color: caretColor('bea') },
		{ actorId: 'exec', name: 'The Exec', editing: false, color: caretColor('exec') }
	]);
});

test('a document face is viewing unless the service marked it editing', () => {
	const states = new Map<number, unknown>([[2, { user: { actor_id: 'bea' }, editing: 'yes' }]]);
	assert.equal(documentFaces(states, 'me', nameOf)[0]?.editing, false);
});

test('a sheet face takes its session selection colour, else the caret colour', () => {
	const faces = sheetFaces(
		[
			{ actor_id: 'bea', name: 'Bea', client_ids: ['c1', 'c2'], editing: false },
			{ actor_id: 'staff', name: 'Rowan', client_ids: [], editing: true },
			{ name: 'No actor' },
			'junk'
		],
		(clientId) => (clientId === 'c2' ? '#ff0000' : undefined)
	);
	assert.deepEqual(faces, [
		{ actorId: 'bea', name: 'Bea', editing: false, color: '#ff0000' },
		{ actorId: 'staff', name: 'Rowan', editing: true, color: caretColor('staff') }
	]);
	assert.deepEqual(sheetFaces(undefined), []);
});

test('never more than five circles: five faces show, six fold into four and +2', () => {
	const face = (n: number): PageFace => ({
		actorId: `a${n}`,
		name: `A${n}`,
		editing: false,
		color: ''
	});
	const five = Array.from({ length: 5 }, (_, n) => face(n));
	assert.deepEqual(visibleFaces(five), { shown: five, rest: [] });
	const six = [...five, face(5)];
	const { shown, rest } = visibleFaces(six);
	assert.equal(shown.length, 4);
	assert.deepEqual(
		rest.map((f) => f.actorId),
		['a4', 'a5']
	);
});
