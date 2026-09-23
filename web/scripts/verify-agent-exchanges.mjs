// Read-only live smoke plus browser-only error/empty/history fixtures. No company writes.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const source = process.env.RESTLESS_FIXTURE_COMPANY;
assert(source, 'RESTLESS_FIXTURE_COMPANY required');
const base = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:8788';
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-agent-exchanges';
await fs.mkdir(out, { recursive: true });
const browser = await chromium.launch({
	executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE,
	headless: true
});
let closing = false;
try {
	const context = await browser.newContext();
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (e) => errors.push(e.message));
	const result = await context.request.get(
		`${base}/api/companies/${source}/actors/hosting-engineering/exchanges`
	);
	assert.equal(result.status(), 200);
	const actual = await result.json();
	assert(
		actual.messages.some((m) => m.from_actor === 'exec' || m.to_actor === 'exec'),
		'real Exec/lead exchange'
	);
	assert(
		actual.messages.every((m) => m.to_actor && m.to_actor !== 'owner' && m.from_actor !== 'owner')
	);
	for (const width of [1440, 390, 320]) {
		await page.setViewportSize({ width, height: 900 });
		await page.goto(`${base}/${source}/people?person=hosting-engineering`);
		const box = page.locator('.agent-exchanges');
		await box.locator('> summary').click();
		await box.locator('.exchange').first().waitFor();
		await box.locator('.exchange > summary').first().click();
		const bounds = await box.boundingBox();
		assert(bounds.x >= 0 && bounds.x + bounds.width <= width + 1, `exchanges fit ${width}`);
		const overflow = await page
			.locator('.people-talk')
			.evaluate((e) => e.scrollWidth > e.clientWidth + 1);
		assert(!overflow, `chat overflow at ${width}`);
		const composer = await page.locator('.talk-composer').boundingBox();
		assert(composer && composer.y + composer.height <= 901, 'composer remains visible');
		await page.screenshot({ path: `${out}/exchanges-${width}.png` });
	}
	const company = 'agent_exchanges_ui_test';
	let mode = 'error',
		calls = 0;
	await page.route('**/api/**', async (route) => {
		try {
			const req = route.request(),
				url = new URL(req.url());
			assert.equal(req.method(), 'GET', 'read-only verification');
			if (url.pathname.endsWith('/exchanges')) {
				calls++;
				if (mode === 'error')
					return route.fulfill({ status: 503, json: { message: 'Unavailable' } });
				if (mode === 'empty')
					return route.fulfill({ json: { messages: [], names: {}, next_before: null } });
				const older = url.searchParams.has('before');
				return route.fulfill({
					json: {
						names: { exec: 'Exec', 'hosting-engineering': 'Daria' },
						next_before: older ? null : 2,
						messages: [
							{
								id: older ? 1 : 2,
								from_actor: older ? 'hosting-engineering' : 'exec',
								to_actor: older ? 'exec' : 'hosting-engineering',
								created_at: '2026-09-22T03:00:00Z',
								body: older ? 'Older recorded exchange' : 'Current recorded exchange'
							}
						]
					}
				});
			}
			if (url.pathname.includes('/activity'))
				return route.fulfill({ body: ': fixture\n\n', contentType: 'text/event-stream' });
			const response = await route.fetch({ url: req.url().replace(company, source) });
			await route.fulfill({ response, body: (await response.text()).replaceAll(source, company) });
		} catch (e) {
			if (!closing) throw e;
		}
	});
	await page.goto(`${base}/${company}/people?person=hosting-engineering`);
	const card = page.locator('.agent-exchanges');
	await card.locator('> summary').focus();
	await page.keyboard.press('Enter');
	await card.getByRole('alert').waitFor();
	mode = 'empty';
	await card.getByRole('button', { name: 'Retry' }).click();
	await card.getByText('No internal messages recorded for this person yet.').waitFor();
	mode = 'history';
	await page.reload();
	await card.locator('> summary').click();
	await card.getByRole('button', { name: 'Older exchanges' }).click();
	await card.getByText('Older recorded exchange', { exact: true }).waitFor();
	assert.equal(await card.locator('.exchange').count(), 2);
	assert.equal(await card.getByRole('button', { name: 'Older exchanges' }).count(), 0);
	assert.deepEqual(errors, []);
	console.log(
		`PASS real Exec/lead history; desktop/mobile layout; keyboard; error/retry; empty; pagination (${calls} fixture reads); no company writes`
	);
} finally {
	closing = true;
	for (const c of browser.contexts())
		for (const p of c.pages()) await p.unrouteAll({ behavior: 'ignoreErrors' });
	await browser.close();
}
