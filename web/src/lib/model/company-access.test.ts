import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as access from './company-access.ts';
import type { CompanyPrincipal } from './query-persistence';

const { collaboratorHome, companyShellTabs, hasOwnerSurfaceAccess, mayOpenCompanyRoute } = access;

function principal(membership_role: CompanyPrincipal['membership_role']): CompanyPrincipal {
	return { actor_id: 'human-1', membership_role, cache_partition: 'partition-1' };
}

test('only the authenticated owner membership receives owner surfaces', () => {
	assert.equal(hasOwnerSurfaceAccess(principal('owner')), true);
	assert.equal(hasOwnerSurfaceAccess(principal('admin')), false);
	assert.equal(hasOwnerSurfaceAccess(principal('member')), false);
	assert.equal(hasOwnerSurfaceAccess(null), false);
});

test('collaborator navigation contains no owner-only destinations', () => {
	const tabs = companyShellTabs('acme', '/acme/people/rooms', principal('member'));
	assert.deepEqual(
		tabs.map(({ key, href, on }) => ({ key, href, on })),
		[
			{ key: 'work', href: '/acme/work', on: false },
			{ key: 'people', href: '/acme/people', on: true }
		]
	);
	assert.equal(
		tabs.some((tab) => tab.key === 'company' || tab.key === 'attention'),
		false
	);
});

test('navigation stays empty until a principal has been authenticated', () => {
	assert.deepEqual(companyShellTabs('acme', '/acme', null), []);
});

test('owner navigation keeps the four canonical company areas', () => {
	const tabs = companyShellTabs('acme', '/acme/company/computer', principal('owner'), 3);
	assert.deepEqual(
		tabs.map(({ key, on, badge }) => ({ key, on, badge })),
		[
			{ key: 'attention', on: false, badge: 3 },
			{ key: 'work', on: false, badge: undefined },
			{ key: 'people', on: false, badge: undefined },
			{ key: 'company', on: true, badge: undefined }
		]
	);
});

test('non-owner route admission matches only collaboration route families', () => {
	const member = principal('member');
	for (const pathname of [
		'/acme/people',
		'/acme/people/rooms',
		'/acme/work',
		'/acme/work/work-1',
		'/acme/work/documents'
	]) {
		assert.equal(mayOpenCompanyRoute('acme', pathname, member), true, pathname);
	}
	for (const pathname of [
		'/acme',
		'/acme/company',
		'/acme/company/authority',
		'/acme/company/computer'
	]) {
		assert.equal(mayOpenCompanyRoute('acme', pathname, member), false, pathname);
	}
	assert.equal(mayOpenCompanyRoute('acme', '/other/people/rooms', member), false);
	assert.equal(mayOpenCompanyRoute('acme', '/acme/company/authority', principal('owner')), true);
	assert.equal(collaboratorHome('acme group'), '/acme%20group/work');
});
