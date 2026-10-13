import assert from 'node:assert/strict';
import test from 'node:test';
import { reenterHref } from './reenter.ts';

function memory() {
	const values = new Map<string, string>();
	return {
		getItem: (key: string) => values.get(key) ?? null,
		setItem: (key: string, value: string) => void values.set(key, value)
	};
}

const page = { hostname: 'app.restless.run', pathname: '/company_ab/work', search: '?goal=g1' };

test('a lapsed hosted session goes back through the account and lands on the same page', () => {
	assert.equal(
		reenterHref(page, 1_000, memory()),
		'/account?return=%2Fcompany_ab%2Fwork%3Fgoal%3Dg1'
	);
});

test('it tries once a minute, so a real sign-out shows the page instead of looping', () => {
	const store = memory();
	assert.ok(reenterHref(page, 1_000, store));
	assert.equal(reenterHref(page, 30_000, store), null);
	assert.ok(reenterHref(page, 62_000, store));
});

test('a local appliance has no account to ask', () => {
	for (const hostname of ['localhost', '127.0.0.1', '[::1]', 'groom.localhost'])
		assert.equal(reenterHref({ ...page, hostname }, 1_000, memory()), null);
});
