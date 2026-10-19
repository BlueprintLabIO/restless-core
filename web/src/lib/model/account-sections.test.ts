import assert from 'node:assert/strict';
import test from 'node:test';
import { ACCOUNT_SECTIONS, accountsHref, legacyAccountSection } from './account-sections.ts';

test('every section has one address under /account/settings', () => {
	for (const section of Object.values(ACCOUNT_SECTIONS)) {
		assert.equal(section.href, `/account/settings/${section.id}`);
	}
});

/* Inbox items stored before the move still link `/account/connections?company=…&request=…`. */
test('each earlier account address lands on the section it became', () => {
	const cases: [string, string][] = [
		['', '/account/settings/accounts'],
		['settings', '/account/settings/accounts'],
		['connections', '/account/settings/accounts'],
		['ai-apps', '/account/settings/accounts'],
		['settings/connections/', '/account/settings/accounts'],
		['appearance', '/account/settings/appearance'],
		['settings/computers', '/account/settings/computers'],
		['appliance', '/account/settings/this-computer']
	];
	for (const [rest, href] of cases) assert.equal(legacyAccountSection(rest)?.href, href, rest);
	assert.equal(legacyAccountSection('settings/billing'), null);
	assert.equal(legacyAccountSection('anything-else'), null);
});

test('a deep link into Accounts keeps its query', () => {
	assert.equal(
		accountsHref({ grant: 'company_x', connection: 'c1' }),
		'/account/settings/accounts?grant=company_x&connection=c1'
	);
	assert.equal(accountsHref(), '/account/settings/accounts');
});
