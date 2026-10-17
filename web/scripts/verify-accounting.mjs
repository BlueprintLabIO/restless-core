/* Money book and compact rail acceptance; only the explicitly owned test company. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-ta94';
await mkdir(out, { recursive: true });
const api = `${origin}/api/companies/${company}/company/accounting`;
const id = randomUUID();
const input = {
	kind: 'income',
	description: `Camera sale — receipt ${id.slice(0, 4)}`,
	counterparty: 'Test customer',
	category: 'Sales',
	amount_minor: 29900,
	currency: 'AUD',
	status: 'settled',
	occurred_on: '2026-10-17',
	evidence: 'Test receipt INV-104'
};
const save = (version, record) =>
	fetch(`${api}/${id}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json', Origin: origin },
		body: JSON.stringify({ version, record })
	});
let response = await save(0, input);
assert.equal(response.status, 200);
assert.equal((await response.json()).version, 1);
response = await save(0, input);
assert.equal(response.status, 200);
assert.equal((await response.json()).version, 1);
assert.equal((await (await fetch(api)).json()).records.filter((row) => row.id === id).length, 1);
response = await save(0, { ...input, amount_minor: 31000 });
assert.equal(response.status, 409);
response = await save(1, { ...input, amount_minor: 31000 });
assert.equal(response.status, 200);
response = await save(1, { ...input, amount_minor: 32000 });
assert.equal(response.status, 409);
const browser = await chromium.launch();
const results = [];
const require = createRequire(import.meta.url);
for (const [name, viewport] of [
	['desktop', { width: 1440, height: 900 }],
	['phone', { width: 390, height: 844 }]
]) {
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(String(error)));
	try {
		await page.goto(`${origin}/${company}/company/accounting`);
		await page.getByRole('heading', { name: 'Accounting', exact: true }).waitFor();
		await page.getByText(input.description, { exact: true }).waitFor();
		await page.getByRole('button', { name: 'Add record', exact: true }).click();
		const dialog = page.getByRole('dialog');
		await dialog
			.getByLabel('Description', { exact: true })
			.fill(`Hosting subscription ${name} ${id.slice(0, 4)}`);
		await dialog.getByLabel('Amount', { exact: true }).fill('12.29');
		await dialog.getByLabel('Currency', { exact: true }).fill('USD');
		await dialog
			.getByLabel('Receipt or source note')
			.fill('Synthetic test invoice; never enters a real company');
		await dialog.getByRole('button', { name: 'Save record', exact: true }).click();
		await dialog.waitFor({ state: 'hidden' });
		const row = page
			.locator('.record')
			.filter({ hasText: `Hosting subscription ${name} ${id.slice(0, 4)}` });
		await row.locator('summary').click();
		await row.getByRole('button', { name: 'Edit record', exact: true }).click();
		await dialog.getByLabel('Status', { exact: true }).selectOption('void');
		await dialog.getByRole('button', { name: 'Save record', exact: true }).click();
		await dialog.waitFor({ state: 'hidden' });
		await row.getByText('Voided', { exact: true }).waitFor();
		assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
		assert.deepEqual(errors, []);
		await page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
		const violations = await page.evaluate(async () =>
			(
				await window.axe.run(document.body, {
					runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa'] }
				})
			).violations.filter((v) => ['serious', 'critical'].includes(v.impact))
		);
		await writeFile(`${out}/${name}-accessibility.json`, JSON.stringify(violations, null, 2));
		assert.deepEqual(
			violations.map((v) => v.id),
			[]
		);
		await page.screenshot({ path: `${out}/accounting-${name}.png` });
		await page.route(`**/api/companies/${company}/company/accounting`, (route) =>
			route.fulfill({
				status: 503,
				json: { error: 'accounting', message: 'Synthetic source unavailable' }
			})
		);
		await page.getByRole('button', { name: 'Refresh', exact: true }).click();
		await page.getByRole('alert').waitFor();
		assert.equal(
			await page.getByText(input.description, { exact: true }).count(),
			1,
			'Refresh failure preserves books'
		);
		await page.unroute(`**/api/companies/${company}/company/accounting`);
		await page.goto(`${origin}/${company}/company/spend?proof=redirect`);
		await page.getByRole('heading', { name: 'Tokens', exact: true }).waitFor();
		assert(new URL(page.url()).pathname.endsWith('/company/tokens'));
		assert.equal(new URL(page.url()).searchParams.get('proof'), 'redirect');
		assert.equal(await page.getByRole('heading', { name: 'Payments', exact: true }).count(), 0);
		await page.screenshot({ path: `${out}/tokens-${name}.png` });
		results.push({ name, passed: true });
	} catch (error) {
		await page.screenshot({ path: `${out}/${name}-failed.png` });
		results.push({ name, passed: false, error: String(error), pageErrors: errors });
	} finally {
		await context.close();
	}
}
await browser.close();
await writeFile(`${out}/results.json`, JSON.stringify(results, null, 2));
console.log(JSON.stringify(results));
assert(results.every((r) => r.passed));
