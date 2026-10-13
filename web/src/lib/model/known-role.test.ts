import assert from 'node:assert/strict';
import test from 'node:test';
import { PLANE_OWNER_KEY, knownPlaneOwner, rememberRole, roleKey } from './known-role.ts';

const store = new Map<string, string>();
(globalThis as { localStorage?: unknown }).localStorage = {
	getItem: (key: string) => store.get(key) ?? null,
	setItem: (key: string, value: string) => void store.set(key, value),
	removeItem: (key: string) => void store.delete(key)
};

test('a member is remembered as a member, and never as the plane owner', () => {
	store.clear();
	rememberRole('reno', 'member');
	assert.equal(store.get(roleKey('reno')), 'member');
	assert.equal(knownPlaneOwner(), false);
});

test('owning any company here makes this browser the plane owner, and a member visit keeps it', () => {
	store.clear();
	rememberRole('acme', 'owner');
	rememberRole('reno', 'member');
	assert.equal(store.get(PLANE_OWNER_KEY), '1');
	assert.equal(knownPlaneOwner(), true);
});

test('without storage nothing is known', () => {
	const saved = (globalThis as { localStorage?: unknown }).localStorage;
	(globalThis as { localStorage?: unknown }).localStorage = {
		getItem: () => {
			throw new Error('blocked');
		},
		setItem: () => {
			throw new Error('blocked');
		}
	};
	rememberRole('acme', 'owner');
	assert.equal(knownPlaneOwner(), false);
	(globalThis as { localStorage?: unknown }).localStorage = saved;
});
