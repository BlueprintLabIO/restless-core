import assert from 'node:assert/strict';
import test from 'node:test';
import { COMPANY_PAGES, companyAlias, companyPageAlias } from './company-pages.ts';

/* 6 Nov: `/company/health` showed "This page does not exist" while the sidebar's Health row
 * went to `/company/doctor`. A page's label or key names it as well as its path does. */
test('a Company page named by its label or key lands on its path', () => {
	assert.equal(companyPageAlias('health')?.path, '/doctor');
	assert.equal(companyPageAlias('Health')?.path, '/doctor');
	assert.equal(companyPageAlias('brand')?.path, '/identity');
	assert.equal(companyPageAlias('intelligence')?.path, '/provider');
	assert.equal(companyPageAlias('limits')?.path, '/resources');
	assert.equal(companyPageAlias('weekly-review')?.path, '/week');
	assert.equal(companyPageAlias('Team results')?.path, '/team-results');
	assert.equal(companyPageAlias('nonsense'), undefined);
	/* The overview is the area itself, never an alias of a page beneath it. */
	assert.equal(companyPageAlias('overview'), undefined);
});

test('every alias lands on a page the sidebar lists', () => {
	for (const page of COMPANY_PAGES.filter((candidate) => candidate.path)) {
		assert.equal(companyPageAlias(page.key)?.path ?? page.path, page.path, page.key);
	}
});

test('a company address typed by a surface or page name lands where it lives', () => {
	assert.equal(companyAlias('inbox'), '');
	assert.equal(companyAlias('settings'), '/company');
	assert.equal(companyAlias('health'), '/company/doctor');
	assert.equal(companyAlias('doctor'), '/company/doctor');
	assert.equal(companyAlias('vault'), '/company/vault');
	assert.equal(companyAlias('100%'), undefined);
	assert.equal(companyAlias('nonsense'), undefined);
});
