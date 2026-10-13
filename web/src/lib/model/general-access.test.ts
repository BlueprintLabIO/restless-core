import assert from 'node:assert/strict';
import test from 'node:test';
import { GENERAL_ACCESS_OPTIONS, matchPeople } from './general-access.ts';

test('documents and sheets offer the same four levels', () => {
	assert.deepEqual(
		GENERAL_ACCESS_OPTIONS.map((option) => option.value),
		['private', 'view', 'comment', 'edit']
	);
});

test('adding by name finds people and agents, never the system or anyone already added', () => {
	const people = [
		{ actor_id: 'alex', display: 'Alex Moreno', kind: 'human' },
		{ actor_id: 'exec', display: 'Exec', kind: 'exec' },
		{ actor_id: 'research-analyst', display: 'Research Analyst', kind: 'staff' },
		{ actor_id: 'daemon', display: 'The daemon', kind: 'system' },
		{ actor_id: 'blair', display: 'Blair Exeter', kind: 'human' }
	];
	assert.deepEqual(matchPeople(people, '', new Set()), []);
	assert.deepEqual(
		matchPeople(people, 'ex', new Set()).map((person) => person.actor_id),
		['exec', 'alex', 'blair'],
		'a name that starts with the query comes first'
	);
	assert.deepEqual(
		matchPeople(people, 'the', new Set()).map((person) => person.actor_id),
		[]
	);
	assert.deepEqual(
		matchPeople(people, 'ana', new Set(['research-analyst'])).map((person) => person.actor_id),
		[]
	);
	assert.deepEqual(
		matchPeople(people, 'research', new Set()).map((person) => person.actor_id),
		['research-analyst']
	);
});
