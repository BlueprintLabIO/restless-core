// Live projection/ticket check, then browser-only fixtures; no owner decisions or sign-ins.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs/promises';

const source = process.env.RESTLESS_FIXTURE_COMPANY;
const itemId = process.env.RESTLESS_HANDOFF_ITEM;
assert(source && itemId, 'Set RESTLESS_FIXTURE_COMPANY and RESTLESS_HANDOFF_ITEM');
const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:8788';
const output = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-handoff-browser';
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE });
try {
	const context = await browser.newContext();
	const response = await context.request.get(`${origin}/api/companies/${source}/attention`);
	assert.equal(response.status(), 200);
	const original = await response.json();
	const live = original.items.find((item) => item.id === itemId);
	assert(live, 'The reported handoff still exists');
	assert(!live.runtime_attach, 'Unprepared identity step must not expose the desktop');
	assert(!live.actions.some((a) => a.id === 'open-outcome'), 'No misleading prepared-browser action');
	const denied = await context.request.post(`${origin}/api/companies/${source}/browser/ticket`, {
		headers: { origin },
		data: { item_id: itemId, client_id: randomUUID() }
	});
	assert.equal(denied.status(), 409, 'Stale desktop URL must be rejected by the server');
	assert.match((await denied.text()), /no live runtime attachment/);
	const company = 'handoff_browser_ui_test';
	let ready = false;
	let desktopRequests = 0;
	const errors = [];
	await context.route('**/api/**', async (route) => {
		const request = route.request();
		const url = new URL(request.url());
		if (url.pathname.endsWith('/browser/ticket')) desktopRequests++;
		if (request.method() !== 'GET') return route.fulfill({ status: 409, json: {} });
		if (url.pathname.endsWith('/attention')) {
			const item = structuredClone(live);
			item.title = ready ? 'Test GitHub sign-in' : 'Test sign-in preparation';
			item.preparing = !ready;
			item.actions = ready ? [{ id: 'open-external-human-step', label: 'Open github.com in normal browser',
				href: 'https://github.com/login/device', role: 'human_step', consequence: 'Open the test destination.',
				next_state: 'Await provider confirmation.' }] : [];
			return route.fulfill({ json: { ...original, items: [item] } });
		}
		const real = url.toString().replace(company, source);
		try { return route.fulfill({ response: await context.request.get(real) }); }
		catch { return route.fulfill({ status: 503, json: {} }); }
	});
	for (const width of [1440, 390]) {
		const page = await context.newPage();
		await page.setViewportSize({ width, height: 900 });
		page.on('pageerror', (error) => errors.push(error.message));
		await page.goto(`${origin}/${company}?item=${encodeURIComponent(itemId)}&computer=${encodeURIComponent(itemId)}`);
		await page.waitForURL((url) => !url.searchParams.has('computer'));
		await page.getByText('Test sign-in preparation', { exact: true }).first().waitFor();
		assert.equal(await page.locator('iframe[src*="desktop"]').count(), 0);
		assert(await page.getByText('Preparing:', { exact: true }).count() > 0);
		assert.equal(await page.getByText('Open prepared browser', { exact: true }).count(), 0);
		await page.screenshot({ path: `${output}/pending-${width}.png` });
		await page.close();
	}
	ready = true;
	const page = await context.newPage();
	page.on('pageerror', (error) => errors.push(error.message));
	await page.goto(`${origin}/${company}?item=${encodeURIComponent(itemId)}`);
	const link = page.getByRole('link', { name: 'Open github.com in normal browser', exact: true });
	await link.waitFor();
	assert.equal(await link.getAttribute('href'), 'https://github.com/login/device');
	assert.equal(await link.getAttribute('target'), '_blank');
	assert.equal(desktopRequests, 0, 'Neither state requests a company desktop');
	assert.deepEqual(errors, []);
	console.log('PASS: pending sign-in has no desktop attachment, stale tickets are denied, old URLs recover, and a prepared URL opens a browser tab.');
} finally {
	await browser.close();
}
