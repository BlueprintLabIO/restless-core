import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:18789';
const upstream = 'http://127.0.0.1:8788';
const source = process.env.RESTLESS_FIXTURE_COMPANY;
assert(source);
const company = 'handoff_instructions_ui_test';
const browser = await chromium.launch({ executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE });
let context;
try {
	context = await browser.newContext({ viewport: { width: 1440, height: 950 } });
	const original = await (
		await context.request.get(`${upstream}/api/companies/${source}/attention`)
	).json();
	const template = original.items.find((i) => i.source.kind === 'owner_handoff');
	assert(template);
	let ready = false;
	const errors = [];
	await context.route('**/api/**', async (route) => {
		const request = route.request(),
			url = new URL(request.url());
		if (request.method() !== 'GET') return route.fulfill({ status: 403, json: {} });
		if (url.pathname.endsWith('/attention'))
			return route.fulfill({
				json: {
					...original,
					items: [
						{
							...template,
							id: 'test-preparation',
							category: 'human_step',
							source: { plane: 'orgintel', kind: 'owner_handoff', reference: 'test-preparation' },
							title: ready ? 'GitHub sign-in is ready' : 'GitHub sign-in timed out',
							preparing: !ready,
							requested_action: ready
								? 'Open https://github.com/login/device and enter TEST-1234.'
								: 'Preparing a fresh GitHub sign-in code at your request',
							responsible_actor: { id: 'cloud-engineering', display: 'Dobby', role: 'engineer' },
							runtime_attach: null,
							deadline: ready ? 'Expected expiry: test deadline' : null,
							actions: [],
							evidence: []
						}
					]
				}
			});
		if (url.pathname.endsWith('/activity'))
			return route.fulfill({ contentType: 'text/event-stream', body: '' });
		try {
			const real = `${upstream}${url.pathname.replaceAll(company, source)}${url.search}`;
			const response = await context.request.get(real);
			return route.fulfill({
				status: response.status(),
				contentType: response.headers()['content-type'] ?? 'application/json',
				body: (await response.text()).replaceAll(source, company)
			});
		} catch {
			return route.fulfill({ status: 503, json: {} });
		}
	});
	const people = await context.newPage(),
		attention = await context.newPage();
	for (const page of [people, attention])
		page.on('pageerror', (error) => errors.push(error.message));
	await people.goto(`${origin}/${company}/people?person=hosting-engineering`);
	await attention.goto(`${origin}/${company}?item=test-preparation`);
	for (const page of [people, attention]) {
		const card = page.locator('[data-attention-id="test-preparation"]');
		await card.getByRole('status').waitFor();
		assert.match(await card.innerText(), /Nothing to do yet/);
		assert.equal(
			await page.getByRole('heading', { name: 'GitHub sign-in timed out', exact: true }).count(),
			0
		);
		assert.equal(await card.getByRole('link', { name: 'Open github.com', exact: true }).count(), 0);
	}
	assert.match(
		await people.locator('[data-attention-id="test-preparation"] header').innerText(),
		/Preparing your next step/
	);
	ready = true;
	// Observe normal polling update both already-open surfaces, without owner chat or reload.
	for (const page of [people, attention]) {
		const card = page.locator('[data-attention-id="test-preparation"]');
		const link = card.getByRole('link', { name: 'Open github.com', exact: true });
		await link.waitFor({ timeout: 20000 });
		assert.equal(await link.getAttribute('href'), 'https://github.com/login/device');
		assert.equal(await link.getAttribute('target'), '_blank');
		assert.match(await card.innerText(), /TEST-1234/);
		assert.equal(await card.getByRole('status').count(), 0);
	}
	await people
		.locator('[data-attention-id="test-preparation"]')
		.getByText('Expected expiry: test deadline', { exact: true })
		.waitFor();
	await people.setViewportSize({ width: 390, height: 850 });
	assert(await people.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
	assert.deepEqual(errors, []);
	console.log(
		'PASS preparing instructions/title, no premature action, polling to live link/code on People and Attention, visible deadline, narrow bounds'
	);
} finally {
	await context?.unrouteAll({ behavior: 'ignoreErrors' });
	await browser.close();
}
