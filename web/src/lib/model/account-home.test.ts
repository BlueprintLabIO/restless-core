import assert from 'node:assert/strict';
import test from 'node:test';
import { accountHomeDestination } from './account-home.ts';

const hosted = { hosted: true, home_url: 'https://app.restless.run/account' };
test('an explicit company return takes precedence over hosted Home', () => {
	assert.equal(
		accountHomeDestination(new URL('https://app.restless.run/?next=%2Freno%2Fpeople'), hosted),
		'/reno/people'
	);
});
test('All companies reaches the account issuer and preserves Start a company', () => {
	assert.equal(accountHomeDestination(new URL('https://plane.test/'), hosted), hosted.home_url);
	assert.equal(
		accountHomeDestination(new URL('https://plane.test/?start=1'), hosted),
		hosted.home_url + '?start=1'
	);
});
test('Home never redirects to itself or follows an external return', () => {
	assert.equal(accountHomeDestination(new URL('https://app.restless.run/account'), hosted), null);
	assert.equal(
		accountHomeDestination(new URL('https://plane.test/?next=%2F%2Fevil.test'), hosted),
		hosted.home_url
	);
	assert.equal(accountHomeDestination(new URL('https://local.test/'), { hosted: false }), null);
});
