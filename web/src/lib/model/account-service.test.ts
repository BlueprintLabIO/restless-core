import assert from 'node:assert/strict';
import test from 'node:test';
import { accountServicePath } from './account-service.ts';

test('every account page leaves the cockpit for the account service; company pages stay', () => {
	for (const path of [
		'/account',
		'/account/settings',
		'/account/settings/accounts',
		'/account/settings/appearance',
		'/account/company/abc/compute',
		'/account/verify-email'
	]) {
		assert.equal(accountServicePath(path), true, path);
	}
	for (const path of ['/', '/company_x/company/provider', '/accounting', '/accounts']) {
		assert.equal(accountServicePath(path), false, path);
	}
});
