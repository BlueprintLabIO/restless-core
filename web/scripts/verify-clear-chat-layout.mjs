// Read-only live projections with browser-only empty/unavailable Attention states.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:8788';
const source = process.env.RESTLESS_FIXTURE_COMPANY;
assert(source, 'RESTLESS_FIXTURE_COMPANY required');
const company = 'clear_chat_layout_test';
const browser = await chromium.launch({ executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE });
try {
	const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
	const original = await (
		await context.request.get(`${origin}/api/companies/${source}/attention`)
	).json();
	assert(original.items.length, 'Need a real nonempty projection as the comparison');
	const widths = {};
	for (const mode of ['pending', 'clear', 'unavailable']) {
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', (e) => errors.push(e.message));
		await page.route('**/api/**', async (route) => {
			const request = route.request(),
				url = new URL(request.url());
			if (request.method() !== 'GET') return route.fulfill({ status: 409, json: {} });
			if (url.pathname.endsWith('/attention'))
				return route.fulfill({
					status: mode === 'unavailable' ? 503 : 200,
					json: { ...original, items: mode === 'clear' ? [] : original.items }
				});
			if (url.pathname.endsWith('/activity'))
				return route.fulfill({ contentType: 'text/event-stream', body: ': fixture\n\n' });
			try {
				const response = await context.request.get(url.toString().replaceAll(company, source));
				return await route.fulfill({ response });
			} catch {
				return route.fulfill({ status: 503, json: {} }).catch(() => {});
			}
		});
		await page.goto(`${origin}/${company}`);
		const seam = page.getByRole('separator', { name: 'Resize Exec pane', exact: true });
		await seam.waitFor();
		if (mode === 'clear')
			await page.getByRole('button', { name: 'All clear', exact: true }).waitFor();
		await page.waitForTimeout(300);
		widths[mode] = Number(await seam.getAttribute('aria-valuenow'));
		if (mode !== 'clear') assert(widths[mode] <= 400, `${mode} should retain normal chat width`);
		else assert(widths[mode] >= 800, 'All-clear chat should take most of the workspace');
		assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
		assert.deepEqual(errors, []);
		await page.unrouteAll({ behavior: 'ignoreErrors' });
		await page.close();
	}
	console.log(
		'PASS pending and unavailable retain compact chat; confirmed All clear expands chat:',
		widths
	);
} finally {
	await browser.close();
}
