import assert from 'node:assert/strict';
import test from 'node:test';
import { holdsReveal } from './first-paint-hold.ts';

const read = (key: unknown[], data: unknown, status: string, fetchStatus: string) => ({
	queryKey: key,
	state: { data, status, fetchStatus }
});

/* The multiplayer smoke saw a member's cockpit reveal with the persisted Inbox, then add the clear
 * pill and the floor when the live read landed: a placeholder must hold the reveal too. */
test('a persisted attention placeholder holds the reveal while its live read is in flight', () => {
	const persisted = { sourceHealth: { authority: 'unknown' }, items: [] };
	assert.equal(holdsReveal(read(['attention', 'acme'], persisted, 'success', 'fetching'), 'acme'), true);
	assert.equal(holdsReveal(read(['attention', 'acme'], persisted, 'success', 'idle'), 'acme'), false);
	const live = { sourceHealth: { authority: 'available' }, items: [] };
	assert.equal(holdsReveal(read(['attention', 'acme'], live, 'success', 'fetching'), 'acme'), false);
});

test('first answers hold the reveal only for critical reads of this company or the account', () => {
	assert.equal(holdsReveal(read(['attention', 'acme'], undefined, 'pending', 'fetching'), 'acme'), true);
	assert.equal(holdsReveal(read(['attention', 'other'], undefined, 'pending', 'fetching'), 'acme'), false);
	assert.equal(holdsReveal(read(['companies'], undefined, 'pending', 'fetching'), 'acme'), true);
	assert.equal(holdsReveal(read(['for-you', 'acme'], undefined, 'pending', 'fetching'), 'acme'), false);
	assert.equal(holdsReveal(read(['attention', 'acme'], undefined, 'pending', 'idle'), 'acme'), false);
});
