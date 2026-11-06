import assert from 'node:assert/strict';
import test from 'node:test';
import { HOLDS_AT_MOST, holdsReveal, surfaceOf } from './first-paint-hold.ts';

const read = (key: unknown[], data: unknown, status: string, fetchStatus: string) => ({
	queryKey: key,
	state: { data, status, fetchStatus }
});

/* The multiplayer smoke saw a member's cockpit reveal with the persisted Inbox, then add the clear
 * pill and the floor when the live read landed: a placeholder must hold the reveal too. */
test('a persisted attention placeholder holds the reveal while its live read is in flight', () => {
	const persisted = { sourceHealth: { authority: 'unknown' }, items: [] };
	assert.equal(
		holdsReveal(read(['attention', 'acme'], persisted, 'success', 'fetching'), 'acme'),
		true
	);
	assert.equal(
		holdsReveal(read(['attention', 'acme'], persisted, 'success', 'idle'), 'acme'),
		false
	);
	const live = { sourceHealth: { authority: 'available' }, items: [] };
	assert.equal(
		holdsReveal(read(['attention', 'acme'], live, 'success', 'fetching'), 'acme'),
		false
	);
});

test('first answers hold the reveal only for critical reads of this company or the account', () => {
	assert.equal(
		holdsReveal(read(['attention', 'acme'], undefined, 'pending', 'fetching'), 'acme'),
		true
	);
	assert.equal(
		holdsReveal(read(['attention', 'other'], undefined, 'pending', 'fetching'), 'acme'),
		false
	);
	assert.equal(holdsReveal(read(['companies'], undefined, 'pending', 'fetching'), 'acme'), true);
	assert.equal(
		holdsReveal(read(['for-you', 'acme'], undefined, 'pending', 'fetching'), 'acme'),
		false
	);
	assert.equal(
		holdsReveal(read(['attention', 'acme'], undefined, 'pending', 'idle'), 'acme'),
		false
	);
});

test('a failed placeholder read reveals its recovery state while background retry continues', () => {
	const persisted = { sourceHealth: { authority: 'unknown' }, items: [] };
	assert.equal(
		holdsReveal(read(['attention', 'acme'], persisted, 'error', 'fetching'), 'acme'),
		false
	);
	assert.equal(
		holdsReveal(read(['attention', 'acme'], undefined, 'error', 'fetching'), 'acme'),
		false
	);
});

/* 3 Nov walkthrough: a direct load of Resources or the Library waited on the Inbox's own reads. */
test('a read only the Inbox shows holds the reveal on the Inbox and nowhere else', () => {
	const digest = read(['direction-digest', 'acme'], undefined, 'pending', 'fetching');
	assert.equal(holdsReveal(digest, 'acme', 'inbox'), true);
	assert.equal(holdsReveal(digest, 'acme', 'library'), false);
	assert.equal(holdsReveal(digest, 'acme', 'company'), false);
	const shell = read(['attention', 'acme'], undefined, 'pending', 'fetching');
	assert.equal(holdsReveal(shell, 'acme', 'library'), true, 'the shell still waits for its badge');
	const graph = read(['work-graph', 'acme'], undefined, 'pending', 'fetching');
	assert.equal(holdsReveal(graph, 'acme', 'work'), true, 'Work waits for its graph');
	assert.equal(holdsReveal(graph, 'acme', 'library'), false);
	assert.equal(surfaceOf('/acme', 'acme'), 'inbox');
	assert.equal(surfaceOf('/acme/', 'acme'), 'inbox');
	assert.equal(surfaceOf('/acme/company/resources', 'acme'), 'company');
});

/* 6 Nov: another company's Health stayed a blank frame for 8–10 s while its company read ran the
 * live checks. A slow read holds the reveal only for its limit; the others still hold it. */
test('a slow read holds the reveal only until its limit', () => {
	const company = read(['company', 'acme', false], undefined, 'pending', 'fetching');
	const cockpit = read(['cockpit', 'acme'], undefined, 'pending', 'fetching');
	const limit = HOLDS_AT_MOST.company.ms;
	assert.ok(limit > 1200, 'longer than the zero-jank gate holds a slow read');
	assert.equal(holdsReveal(company, 'acme', 'company', 0), true);
	assert.equal(holdsReveal(company, 'acme', 'company', limit - 1), true);
	assert.equal(holdsReveal(company, 'acme', 'company', limit), false);
	assert.equal(holdsReveal(cockpit, 'acme', 'company', limit * 10), true);
	/* The Inbox detail lays out from the company read, so there it keeps holding. */
	assert.equal(holdsReveal(company, 'acme', 'inbox', limit * 10), true);
});
