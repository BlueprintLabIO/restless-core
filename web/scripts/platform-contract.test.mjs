import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { parsePlatformContext } from '../src/lib/platform/contracts.ts';

function context(overrides = {}) {
	return {
		schemaVersion: 1,
		mode: 'cloud_fleet',
		identity: { userId: 'user_one', displayName: 'Owner One', role: 'owner' },
		scope: { kind: 'owner' },
		capabilities: ['company.open', 'account.sign_out'],
		navigation: { portfolioHref: '/account', signOutHref: '/account?/signOut' },
		companies: [
			{
				id: 'org_one',
				name: 'One Company',
				mission: '',
				model: '',
				spend_ceiling_usd: null,
				runtime_status: 'running',
				lifecycle_status: 'active',
				role: 'owner'
			}
		],
		projections: {
			org_one: {
				attentionCount: null,
				nextProof: null,
				nextProofDetail: 'Open the company to load live Work.',
				spendAccounted: null
			}
		},
		...overrides
	};
}

test('accepts the complete canonical Cloud fleet projection', () => {
	assert.deepEqual(parsePlatformContext(context()), context());
});

test('requires a complete projection for every Cloud company', () => {
	assert.throws(() => parsePlatformContext(context({ projections: {} })), /incompatible/);
});

test('refuses duplicate capabilities, unsafe navigation and unknown runtime state', () => {
	assert.throws(
		() => parsePlatformContext(context({ capabilities: ['company.open', 'company.open'] })),
		/incompatible/
	);
	assert.throws(
		() => parsePlatformContext(context({ navigation: { portfolioHref: 'https://evil.test' } })),
		/incompatible/
	);
	const invalid = context();
	invalid.companies[0].runtime_status = 'probably-running';
	assert.throws(() => parsePlatformContext(invalid), /incompatible/);
});

test('does not permit Fleet portfolio data in company or self-hosted mode', () => {
	assert.throws(() => parsePlatformContext(context({ mode: 'cloud_company' })), /incompatible/);
	const selfHosted = context({
		mode: 'self_hosted',
		companies: undefined,
		projections: undefined
	});
	assert.equal(parsePlatformContext(selfHosted).mode, 'self_hosted');
});

test('leaves the Core client router for platform-owned account surfaces', async () => {
	const portfolio = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
	assert.match(portfolio, /href=\{supportHref\} data-sveltekit-reload/);
	assert.match(portfolio, /href="\/account" data-sveltekit-reload/);
});
