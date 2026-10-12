import assert from 'node:assert/strict';
import test from 'node:test';
import * as access from './company-access.ts';
import type { CompanyPrincipal } from './query-persistence';

const {
	collaboratorHome,
	companyShellTabs,
	hasOwnerSurfaceAccess,
	mayManageCompanySetup,
	mayOpenCompanyRoute
} = access;

function principal(membership_role: CompanyPrincipal['membership_role']): CompanyPrincipal {
	return { actor_id: 'human-1', membership_role, cache_partition: 'partition-1' };
}

test('only the authenticated owner membership receives owner surfaces', () => {
	assert.equal(hasOwnerSurfaceAccess(principal('owner')), true);
	assert.equal(hasOwnerSurfaceAccess(principal('admin')), false);
	assert.equal(hasOwnerSurfaceAccess(principal('member')), false);
	assert.equal(hasOwnerSurfaceAccess(null), false);
});

test('members see Work, Library, People and Company, never the Inbox or Apps', () => {
	const tabs = companyShellTabs('acme', '/acme/people/rooms', principal('member'));
	assert.deepEqual(
		tabs.map(({ key, href, on }) => ({ key, href, on })),
		[
			{ key: 'attention', href: '/acme', on: false },
			{ key: 'work', href: '/acme/work', on: false },
			{ key: 'library', href: '/acme/library', on: false },
			{ key: 'people', href: '/acme/people', on: true },
			{ key: 'company', href: '/acme/company', on: false }
		]
	);
	assert.equal(
		tabs.some((tab) => tab.key === 'apps'),
		false
	);
});

test('owners and admins manage company setup; members read it', () => {
	assert.equal(mayManageCompanySetup(principal('owner')), true);
	assert.equal(mayManageCompanySetup(principal('admin')), true);
	assert.equal(mayManageCompanySetup(principal('member')), false);
	assert.equal(mayManageCompanySetup(null), false);
});

test('navigation stays empty until a principal has been authenticated', () => {
	assert.deepEqual(companyShellTabs('acme', '/acme', null), []);
});

test('owner navigation keeps the six canonical company areas', () => {
	const tabs = companyShellTabs('acme', '/acme/company/computer', principal('owner'), 3);
	assert.deepEqual(
		tabs.map(({ key, on, badge }) => ({ key, on, badge })),
		[
			{ key: 'attention', on: false, badge: 3 },
			{ key: 'work', on: false, badge: undefined },
			{ key: 'library', on: false, badge: undefined },
			{ key: 'people', on: false, badge: undefined },
			{ key: 'apps', on: false, badge: undefined },
			{ key: 'company', on: true, badge: undefined }
		]
	);
	assert.equal(companyShellTabs('acme', '/acme/apps/c-linear', principal('owner'))[4].on, true);
});

test('members open the shared areas and the Company sections that describe it', () => {
	const member = principal('member');
	for (const pathname of [
		'/acme/people',
		'/acme/people/rooms',
		'/acme/work',
		'/acme/work/work-1',
		'/acme/library/documents',
		'/acme/company',
		'/acme/company/charter',
		'/acme/company/identity',
		'/acme/company/members',
		'/acme/company/schedules',
		'/acme'
	]) {
		assert.equal(mayOpenCompanyRoute('acme', pathname, member), true, pathname);
	}
	for (const pathname of [
		'/acme/apps',
		'/acme/company/authority',
		'/acme/company/computer',
		'/acme/company/vault',
		'/acme/company/provider',
		'/acme/company/resources'
	]) {
		assert.equal(mayOpenCompanyRoute('acme', pathname, member), false, pathname);
	}
	assert.equal(mayOpenCompanyRoute('acme', '/other/people/rooms', member), false);
	assert.equal(mayOpenCompanyRoute('acme', '/acme/company/authority', principal('owner')), true);
	assert.equal(collaboratorHome('acme group'), '/acme%20group/work');
});
