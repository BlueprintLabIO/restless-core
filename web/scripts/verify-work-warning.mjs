// Browser check against a seeded disposable _test company and its local web server.
// Set RESTLESS_SMOKE_ORIGIN and RESTLESS_SMOKE_COMPANY. This only injects failures in the browser.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(company?.endsWith('_test'));
const browser = await chromium.launch();
try {
	for (const width of [1440, 390]) {
		const page = await browser.newPage({ viewport: { width, height: 900 } });
		const errors = [];
		page.on('pageerror', (e) => errors.push(e.message));
		await page.goto(`${origin}/${company}/work`, { waitUntil: 'domcontentloaded' });
		const board = page.locator('.work-board');
		await board.waitFor({ timeout: 90000 });
		await page.locator('.board-item').first().waitFor({ timeout: 30000 });
		await page.waitForTimeout(6500);
		const before = await board.boundingBox();
		const headerBefore = await page.locator('.page > .head').boundingBox();
		const pattern = (url) => url.pathname === `/api/companies/${company}/attention`;
		await page.route(pattern, (route) =>
			route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'unavailable', message: 'Temporarily unavailable' })
			})
		);
		await page.evaluate((name) => {
			const channel = new BroadcastChannel('restless-attention');
			channel.postMessage(name);
			setTimeout(() => channel.close(), 100);
		}, company);
		const indicator = page.getByRole('button', { name: 'Show Work refresh problem' });
		await indicator.waitFor({ timeout: 60000 });
		const warning = page.locator('.toast.failure').filter({ hasText: 'Showing the last update.' });
		await warning.waitFor({ timeout: 10000 });
		const after = await board.boundingBox();
		const headerAfter = await page.locator('.page > .head').boundingBox();
		assert(Math.abs(before.y - after.y) < 1, `board shifted ${after.y - before.y}px at ${width}`);
		assert(
			Math.abs(headerBefore.height - headerAfter.height) < 1,
			`header changed height at ${width}`
		);
		assert.equal(await page.locator('.body > .failure-notice').count(), 0);
		await warning.getByRole('button', { name: 'Dismiss' }).click();
		await indicator.click();
		await warning.waitFor({ timeout: 5000 });
		await page.unroute(pattern);
		await warning.getByRole('button', { name: 'Retry' }).click();
		await indicator.waitFor({ state: 'detached', timeout: 30000 });
		// If the owner reopens the warning and recovery occurs automatically,
		// the reopened toast must not remain as a stale failure.
		await page.route(pattern, (route) =>
			route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'unavailable', message: 'Temporarily unavailable' })
			})
		);
		await page.evaluate((name) => {
			const channel = new BroadcastChannel('restless-attention');
			channel.postMessage(name);
			setTimeout(() => channel.close(), 100);
		}, company);
		await indicator.waitFor({ timeout: 30000 });
		await warning.waitFor({ timeout: 5000 });
		await warning.getByRole('button', { name: 'Dismiss' }).click();
		await indicator.click();
		await warning.waitFor({ timeout: 5000 });
		await page.unroute(pattern);
		await page.evaluate((name) => {
			const channel = new BroadcastChannel('restless-attention');
			channel.postMessage(name);
			setTimeout(() => channel.close(), 100);
		}, company);
		await indicator.waitFor({ state: 'detached', timeout: 30000 });
		await warning.waitFor({ state: 'detached', timeout: 5000 });
		assert.deepEqual(errors, []);
		console.log(
			JSON.stringify({ width, boardShift: after.y - before.y, retry: true, rediscoverable: true })
		);
		await page.close();
	}
} finally {
	await browser.close();
}
