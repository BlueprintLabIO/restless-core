import assert from 'node:assert/strict';
import test from 'node:test';
import { accountServicePath } from './account-service.ts';

test("the account service's pages leave the cockpit; the plane's own sections stay", () => {
	for (const path of [
		'/account',
		'/account/settings',
		'/account/company/abc/compute',
		'/account/verify-email'
	]) {
		assert.equal(accountServicePath(path), true, path);
	}
	for (const path of [
		'/account/connections',
		'/account/ai-apps',
		'/account/appearance',
		'/account/entry',
		'/account/settings/connections',
		'/',
		'/company_x/company/provider',
		'/accounting'
	]) {
		assert.equal(accountServicePath(path), false, path);
	}
});
