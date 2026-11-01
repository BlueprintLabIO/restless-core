/* Regression for the installed Aris failure: an inline computer must not hide Inbox navigation. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
const item = process.env.INBOX_DESKTOP_ITEM_ID;
if (!origin || !['127.0.0.1','localhost'].includes(new URL(origin).hostname) || !company?.endsWith('_test') || !item)
	throw new Error('Use a local _test company and a real prepared-computer fixture item');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-inline-computer-inbox';
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ headless: true });
try {
	const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
	await page.goto(`${origin}/${company}?filter=all&item=${encodeURIComponent(item)}`);
	await page.locator('.attention-focus .inline-computer').waitFor({ timeout: 20000 });
	const filter = page.locator('.inbox-head summary[aria-label="Filter Inbox: All"]');
	await filter.waitFor();
	const index = await page.locator('.attention-index').boundingBox();
	assert(index?.width >= 250, 'The prepared computer does not collapse the sidebar');
	await page.screenshot({ path: `${out}/computer-and-inbox.png`, fullPage: true });
	await filter.click();
	await page.getByRole('button', { name: 'Skipped', exact: true }).click();
	await page.getByRole('region', { name: 'Skipped items', exact: true }).waitFor();
	await page.screenshot({ path: `${out}/desktop.png`, fullPage: true });
	await page.setViewportSize({ width: 390, height: 844 });
	await page.goto(`${origin}/${company}?filter=all`);
	await filter.waitFor();
	await filter.click();
	await page.getByRole('button', { name: 'Done', exact: true }).click();
	await page.getByRole('region', { name: 'Done items', exact: true }).waitFor();
	await page.screenshot({ path: `${out}/phone.png`, fullPage: true });
	console.log('PASS real inline-computer card leaves the desktop filter usable; phone All/Done navigation works');
} finally { await browser.close(); }
