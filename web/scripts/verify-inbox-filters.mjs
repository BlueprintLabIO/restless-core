/* Real filtered Inbox journey. A temporary dismissal is always restored; never use owner data. */
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !['127.0.0.1', 'localhost'].includes(new URL(origin).hostname) || !company?.endsWith('_test'))
	throw new Error('Use a local smoke origin and an isolated _test company');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-inbox-filters';
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
const page = await context.newPage();
const base = `${origin}/${encodeURIComponent(company)}`;
const api = `${origin}/api/companies/${encodeURIComponent(company)}`;
let dismissed;
const findings = [];
async function post(path, data) {
	const response = await context.request.post(api + path, { data, headers: { Origin: origin } });
	assert(response.ok(), `${path}: ${response.status()}`);
	return response;
}
async function filter(value) {
	await page.locator('.inbox-head summary[aria-label^="Filter Inbox:"]').click();
	await page.locator('.action-menu-panel').getByRole('button', { name: value, exact: true }).click();
	await page.waitForURL(url => value === 'Open' ? !url.searchParams.has('filter') : url.searchParams.get('filter') === value.toLowerCase());
}
try {
	const response = await context.request.get(api + '/attention');
	assert(response.ok());
	const view = await response.json();
	const candidate = view.items.find(item => item.source?.plane === 'orgintel' && !item.preparing);
	assert(candidate, 'Seed at least one OrgIntel Inbox item');
	await post('/attention/dismiss', { item_id: candidate.id });
	dismissed = candidate;
	const skippedResponse = await context.request.get(api + '/attention/dismissed');
	assert(skippedResponse.ok());
	const skippedRecord = (await skippedResponse.json()).items.find(item => item.item_id === candidate.id);
	assert(skippedRecord, 'The dismissed request is restorable');
	dismissed = { ...candidate, title: skippedRecord.title };
	for (const width of [1440, 390]) {
		await page.setViewportSize({ width, height: 900 });
		await page.goto(base + '?filter=skipped');
		const skipped = page.getByRole('region', { name: 'Skipped items', exact: true });
		await skipped.getByText(dismissed.title, { exact: true }).waitFor({ timeout: 20000 });
		assert.equal(await page.locator('.set-aside summary').count(), 0, 'No old skipped drawer');
		assert(await skipped.getByRole('button', { name: 'Restore', exact: true }).count());
		await page.screenshot({ path: `${out}/skipped-${width}.png`, fullPage: true });
		await filter('Done');
		await page.getByRole('region', { name: 'Done items', exact: true }).waitFor();
		await page.getByText('Reading completed decisions…', { exact: true }).waitFor({ state: 'hidden', timeout: 20000 });
		assert.equal(await page.locator('.inbox-rows .item-hit').count(), 0, 'Done does not show open decisions');
		await filter('All');
		await skipped.getByText(dismissed.title, { exact: true }).waitFor();
		assert(await page.getByRole('region', { name: 'Done items', exact: true }).isVisible());
		await filter('Open');
		assert.equal(await page.getByRole('region', { name: 'Skipped items', exact: true }).count(), 0);
		assert.equal(await page.getByRole('region', { name: 'Done items', exact: true }).count(), 0);
		await page.reload();
		await page.locator('.inbox-head summary[aria-label="Filter Inbox: Open"]').waitFor({ timeout: 20000 });
		findings.push({ width, filters: ['Open', 'Skipped', 'Done', 'All'], passed: true });
	}
	await page.goto(base + '?filter=skipped');
	const row = page.locator('.set-aside-row').filter({ hasText: dismissed.title });
	await row.getByRole('button', { name: 'Restore', exact: true }).click();
	await row.waitFor({ state: 'hidden' });
	const restored = await context.request.get(api + '/attention');
	assert((await restored.json()).items.some(item => item.id === dismissed.id), 'Restore returns the original request');
	await page.route('**/attention/history', route => route.fulfill({
		status: 503, contentType: 'application/json',
		body: JSON.stringify({ error: 'unavailable', message: 'History is temporarily unavailable.' })
	}));
	await page.goto(base + '?filter=done');
	await page.locator('.inbox-history .failure-notice').waitFor({ timeout: 20000 });
	assert.equal(await page.getByText('No completed decisions.', { exact: true }).count(), 0,
		'A failed history read is never a claim that there are no decisions');
	findings.push({ historyFailure: 'explained without a false empty state', passed: true });
	console.log('PASS Inbox filters at desktop and phone widths, reload, real restore, and unavailable history');
} finally {
	if (dismissed) await post('/attention/restore', { item_id: dismissed.id });
	await writeFile(`${out}/report.json`, JSON.stringify(findings, null, 2));
	await browser.close();
}
