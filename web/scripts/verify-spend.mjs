/* Company Spend acceptance. Synthetic records live only in the owned *_test stack. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const output = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-s93-spend';
await mkdir(output, { recursive: true });
const api = `${origin}/api/companies/${company}/company/spend/report`;
const read = async (params = '') => {
	const response = await fetch(api + params);
	assert.equal(response.status, 200);
	return response.json();
};
const first = await read('?days=7');
assert.equal(first.totals.calls, 207);
assert.equal(first.totals.tokens, 246000);
assert.equal(first.totals.calls_without_tokens, 2);
assert.equal(first.totals.recorded_usd, 0);
assert(first.totals.estimated_usd > 0, 'A real catalogue estimate is available');
assert.equal(first.activity.length, 100);
const second = await read(`?days=7&before=${encodeURIComponent(first.next_before)}`);
assert.equal(second.activity.length, 100);
assert.equal(new Set([...first.activity, ...second.activity].map((row) => row.id)).size, 200);
const filtered = await read('?days=7&actor=reviewer');
assert.equal(filtered.totals.calls, 102);
assert(filtered.activity.every((row) => row.actor === 'reviewer'));
const require = createRequire(import.meta.url);
const browser = await chromium.launch();
const results = [];
for (const [name, viewport] of [
	['desktop', { width: 1440, height: 900 }],
	['phone', { width: 390, height: 844 }]
]) {
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	try {
		await page.goto(`${origin}/${company}/company/spend`);
		await page.getByRole('heading', { name: 'Spend', exact: true }).waitFor();
		await page.getByText('246,000+', { exact: true }).waitFor();
		assert.equal(await page.getByRole('heading', { name: 'Spend', exact: true }).count(), 1);
		await page.getByRole('button', { name: 'reviewer', exact: true }).click();
		await page.getByText('122,400', { exact: true }).first().waitFor();
		await page.getByRole('button', { name: 'All employees ×', exact: true }).click();
		await page.getByText('246,000+', { exact: true }).waitFor();
		await page.locator('.spend-call summary').first().click();
		await page.getByText('Not recorded as a separate charge', { exact: true }).first().waitFor();
		await page.getByRole('button', { name: 'Show earlier', exact: true }).click();
		await page.waitForFunction(() => document.querySelectorAll('.spend-call').length === 200);
		await page.getByRole('button', { name: 'Refresh', exact: true }).scrollIntoViewIfNeeded();
		await page.route(`**/api/companies/${company}/company/spend/report?**`, (route) =>
			route.fulfill({
				status: 503,
				json: { error: 'spend', message: 'Synthetic source unavailable' }
			})
		);
		await page.getByRole('button', { name: 'Refresh', exact: true }).click();
		await page.getByRole('alert').waitFor();
		assert.equal(
			await page.getByText('246,000+', { exact: true }).count(),
			1,
			'Failed refresh preserves the existing report'
		);
		await page.unroute(`**/api/companies/${company}/company/spend/report?**`);
		await page.getByRole('button', { name: 'Refresh', exact: true }).click();
		await page.getByRole('alert').waitFor({ state: 'hidden' });
		await page.locator('.page').evaluate((element) => element.scrollTop); // observation only
		assert.deepEqual(errors, []);
		assert(
			await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
			'No page overflow'
		);
		await page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
		const violations = await page.evaluate(async () =>
			(
				await window.axe.run(document.body, {
					runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa'] }
				})
			).violations.filter((v) => ['serious', 'critical'].includes(v.impact))
		);
		await writeFile(`${output}/${name}-accessibility.json`, JSON.stringify(violations, null, 2));
		assert.deepEqual(
			violations.map((v) => v.id),
			[]
		);
		await page.screenshot({ path: `${output}/${name}.png` });
		const empty = {
			...first,
			totals: Object.fromEntries(
				Object.keys(first.totals).map((key) => [
					key,
					key === 'estimated_usd' || key === 'provider_reported_usd' ? null : 0
				])
			),
			by_actor: [],
			by_model: [],
			by_day: [],
			activity: [],
			next_before: null,
			payments: [],
			payment_totals: []
		};
		await page.route(`**/api/companies/${company}/company/spend/report?**`, (route) =>
			route.fulfill({ json: empty })
		);
		await page.getByLabel('Period', { exact: true }).selectOption('30');
		await page.getByText('No recorded model activity in this period', { exact: true }).waitFor();
		assert.equal(
			await page.locator('.spend-call').count(),
			0,
			'An empty filter never retains the previous activity'
		);
		await page.unroute(`**/api/companies/${company}/company/spend/report?**`);
		await page.route(`**/api/companies/${company}/company/spend/report?**`, (route) =>
			route.fulfill({ json: { ...first, payments_status: 'unavailable' } })
		);
		await page.getByLabel('Period', { exact: true }).selectOption('1');
		await page.getByText('Payment records could not be read', { exact: true }).waitFor();
		await page.getByText('246,000+', { exact: true }).waitFor();
		results.push({ name, passed: true });
	} catch (error) {
		await page.screenshot({ path: `${output}/${name}-failed.png` });
		results.push({ name, passed: false, error: String(error), pageErrors: errors });
	} finally {
		await context.close();
	}
}
await browser.close();
await writeFile(`${output}/results.json`, JSON.stringify({ api: 'passed', results }, null, 2));
console.log(JSON.stringify(results));
assert(results.every((result) => result.passed));
