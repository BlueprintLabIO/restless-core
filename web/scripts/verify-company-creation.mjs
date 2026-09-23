import { chromium } from 'playwright';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:18789';
const upstream = process.env.RESTLESS_FIXTURE_ORIGIN ?? 'http://127.0.0.1:8788';
const executablePath = process.env.RESTLESS_BROWSER_EXECUTABLE;
assert(executablePath, 'RESTLESS_BROWSER_EXECUTABLE is required');

const browser = await chromium.launch({ executablePath });
let context;
try {
	context = await browser.newContext({ viewport: { width: 1280, height: 850 } });
	const page = await context.newPage();
	const writes = [];
	await context.route('**/api/**', async (route) => {
		const request = route.request();
		const url = new URL(request.url());
		if (request.method() === 'POST' && url.pathname === '/api/companies') {
			writes.push(request.postDataJSON());
			// Keep the button pending long enough to prove a second click cannot duplicate creation.
			await new Promise((resolve) => setTimeout(resolve, 200));
			return route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({ id: 'fixture-unconfigured', model: '' })
			});
		}
		if (request.method() !== 'GET')
			return route.fulfill({ status: 403, contentType: 'application/json', body: '{}' });
		if (url.pathname.endsWith('/activity'))
			return route.fulfill({ contentType: 'text/event-stream', body: '' });
		const response = await context.request.get(`${upstream}${url.pathname}${url.search}`);
		return route.fulfill({
			status: response.status(),
			contentType: response.headers()['content-type'] ?? 'application/json',
			body: await response.text()
		});
	});

	await page.goto(origin, { waitUntil: 'domcontentloaded' });
	const plus = page.getByRole('button', { name: 'Create company', exact: true });
	await plus.waitFor({ state: 'visible' });
	await plus.dispatchEvent('click');
	await plus.dispatchEvent('click');
	await page.waitForURL(/\/fixture-unconfigured\/company\/provider(?:\?|$)/, {
		waitUntil: 'domcontentloaded'
	});
	assert.equal(writes.length, 1, 'one pending create must produce one POST');
	assert.equal(Object.hasOwn(writes[0], 'model'), false, 'creation must omit model');
	assert.equal(writes[0].display_name, 'Untitled company');
	assert.equal(writes[0].mission, '');
	assert.equal(await page.getByRole('dialog').count(), 0, 'creation must not open a dialog');
	console.log('PASS direct model-less creation, provider-first route, no modal, pending-click dedupe');
} finally {
	await context?.unrouteAll({ behavior: 'ignoreErrors' });
	await browser.close();
}
