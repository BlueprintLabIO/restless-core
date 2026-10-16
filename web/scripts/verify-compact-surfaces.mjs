/* Owner acceptance: compact, complete surfaces against a populated *_test company. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-compact-surfaces';
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const results = [];
const require = createRequire(import.meta.url);
for (const [name, viewport] of [
	['desktop', { width: 1440, height: 900 }],
	['phone', { width: 390, height: 844 }]
]) {
	const context = await browser.newContext({
		viewport,
		permissions: ['clipboard-read', 'clipboard-write'],
		reducedMotion: 'reduce'
	});
	const page = await context.newPage();
	page.setDefaultTimeout(20000);
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	async function capture(surface) {
		assert.deepEqual(errors, [], 'No uncaught page errors');
		assert(
			await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
			'No page overflow'
		);
		await page.screenshot({ path: `${out}/${surface}-${name}.png` });
		await page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
		const violations = await page.evaluate(async () =>
			(
				await window.axe.run(document.body, {
					runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa'] }
				})
			).violations.filter((v) => ['serious', 'critical'].includes(v.impact))
		);
		await writeFile(
			`${out}/${surface}-${name}-accessibility.json`,
			JSON.stringify(violations, null, 2)
		);
		assert.deepEqual(
			violations.map((v) => v.id),
			[],
			'No serious/critical automated accessibility failures'
		);
	}
	async function run(label, fn) {
		try {
			await fn();
			results.push({ name: `${label}-${name}`, ok: true });
			console.log('PASS', label, name);
		} catch (error) {
			results.push({
				name: `${label}-${name}`,
				ok: false,
				error: String(error),
				pageErrors: [...errors]
			});
			console.log('FAIL', label, name, String(error));
			await page.screenshot({ path: `${out}/${label}-${name}-failed.png` });
		}
	}
	await run('single-header', async () => {
		for (const actor of ['exec', 'studio-design']) {
			await page.goto(`${origin}/${company}/people?person=${actor}`);
			const header = page.locator('.person-header');
			await header.waitFor({ state: 'visible' });
			assert.equal(await header.count(), 1);
			assert.equal(
				await page.locator('#bridge-exrail .exr-head').count(),
				0,
				'No second toolbar/header'
			);
			const bounds = await header.boundingBox();
			assert(bounds && bounds.height <= 64, 'Conversation identity has a strict height bound');
			const search = header.getByRole('button', { name: /Search this conversation/ });
			await search.click();
			await page
				.getByRole('search', { name: /conversation history/ })
				.waitFor({ state: 'visible' });
			await page.keyboard.press('Escape');
			await header.locator('summary[aria-label^="More for"]').click();
			await page
				.getByRole('button', { name: 'Profile', exact: true })
				.waitFor({ state: 'visible' });
			await page.keyboard.press('Escape');
		}
		await capture('single-header');
	});
	await run('legal-and-notifications', async () => {
		let legalWrites = 0;
		await page.route(`**/api/companies/${company}/company/legal`, (route) => {
			if (route.request().method() === 'PUT') {
				legalWrites++;
				return route.fulfill({
					status: 503,
					json: { error: 'test_unavailable', message: 'Test save refused' }
				});
			}
			return route.continue();
		});
		await page.goto(`${origin}/${company}/company/charter`);
		const legal = page.getByRole('region', { name: 'Legal profile', exact: true });
		await legal.getByLabel('Legal name', { exact: true }).fill('Draft Test Pty Ltd');
		assert.equal(await legal.getByRole('button', { name: /^Add/ }).count(), 0);
		await legal.getByLabel('Form', { exact: true }).fill('Pty Ltd');
		await legal.getByLabel('Jurisdiction', { exact: true }).fill('NSW, Australia');
		await legal.getByLabel('Registration number kind', { exact: true }).fill('Test ID');
		await legal.getByLabel('Registration number', { exact: true }).fill('TEST-123');
		await legal.getByLabel('Business address', { exact: true }).fill('Synthetic test address');
		assert.equal(legalWrites, 0, 'Typing never saves legal declarations');
		await legal.getByRole('button', { name: 'Save', exact: true }).click();
		await legal.getByRole('button', { name: 'Retry', exact: true }).waitFor({ state: 'visible' });
		assert.equal(
			await legal.getByLabel('Legal name', { exact: true }).inputValue(),
			'Draft Test Pty Ltd',
			'A refused save preserves the draft'
		);
		assert.equal(legalWrites, 1);
		await capture('legal');
		await page.unroute(`**/api/companies/${company}/company/legal`);
		let channelWrites = 0;
		page.on('request', (request) => {
			if (request.method() !== 'GET' && /notification-preferences|telegram/.test(request.url()))
				channelWrites++;
		});
		await page.goto(`${origin}/${company}/company/notifications`);
		await page
			.getByRole('heading', { name: 'Your notifications', exact: true })
			.waitFor({ state: 'visible' });
		await page
			.getByRole('switch', { name: 'Browser notifications', exact: true })
			.waitFor({ state: 'visible' });
		await page
			.getByText('Email delivery is available on the hosted app.', { exact: true })
			.waitFor({ state: 'visible' });
		const token = page.getByLabel('Bot token', { exact: true });
		await token.waitFor({ state: 'visible' });
		assert.equal(await token.getAttribute('type'), 'password');
		const field = token.locator('..');
		const bounds = await field.boundingBox();
		assert(bounds && bounds.width <= viewport.width);
		assert.equal(await page.getByRole('button', { name: 'Pair', exact: true }).isEnabled(), false);
		assert.equal(channelWrites, 0, 'Viewing settings never enables a notification channel');
		await capture('notifications');
	});
	await run('know-how-source-and-copy', async () => {
		await page.goto(`${origin}/${company}/apps?view=installed-know-how`);
		const installed = page
			.getByRole('navigation', { name: 'Installed', exact: true })
			.filter({ visible: true });
		await installed
			.getByRole('link', { name: 'Know-how', exact: true })
			.waitFor({ state: 'visible' });
		assert.equal(
			await page.locator('.installed-group > summary').count(),
			0,
			'Installed cannot collapse'
		);
		await page.getByRole('table').getByRole('link', { name: 'Code review', exact: true }).click();
		await page
			.getByRole('heading', { name: 'Code review', exact: true })
			.waitFor({ state: 'visible' });
		await page
			.locator('summary')
			.filter({ hasText: /Instructions/ })
			.click();
		const copy = page.getByRole('button', { name: 'Copy all instructions', exact: true });
		await copy.click();
		assert(
			(await page.evaluate(() => navigator.clipboard.readText())).includes('name: code-review')
		);
		await capture('know-how');
		let installs = 0;
		page.on('request', (request) => {
			if (request.method() === 'POST' && request.url().includes('/app-catalogue/install'))
				installs++;
		});
		await page.goto(`${origin}/${company}/apps/x-claude-api`);
		await page
			.locator('summary')
			.filter({ hasText: /Instructions/ })
			.click();
		const instruction = page.locator('.skill-body');
		await instruction.waitFor({ state: 'visible' });
		assert(
			(await instruction.innerText()).length > 1000,
			'Source instructions render rather than metadata alone'
		);
		await page.getByRole('button', { name: 'Copy all instructions', exact: true }).click();
		assert(
			(await page.evaluate(() => navigator.clipboard.readText())).includes('name: claude-api')
		);
		assert.equal(installs, 0, 'Reading/copying a public source does not install know-how');
		await capture('claude-preview');
	});
	await run('schedule-readability', async () => {
		const now = new Date().toISOString(),
			id = '00000000-0000-4000-8000-000000000092';
		const reason =
			'Continue careful bargain sourcing for the owner’s television, graphics workstation and camera, preserving exact-unit evidence and unresolved checks';
		const schedule = {
			id,
			actor_id: 'exec',
			work_id: null,
			reason,
			fire_at: new Date(Date.now() + 7 * 3600000).toISOString(),
			last_fired_at: now,
			created_at: now,
			interval_seconds: 86400,
			recurrence: 'interval',
			timezone: null,
			local_time: null,
			missed_policy: 'catch_up_once',
			catch_up_grace_seconds: 86400,
			paused_at: null
		};
		const outcomes = Array.from({ length: 10 }, (_, index) => ({
			opportunity_id: `outcome-${index}`,
			state: index ? 'completed' : 'blocked',
			outcome: {
				summary: index ? 'A synthetic completed result.' : 'Synthetic check needs an owner answer.'
			},
			outcome_reason: null,
			created_at: now,
			settled_at: now
		}));
		await page.route(`**/api/companies/${company}/schedules`, (route) =>
			route.fulfill({
				json: {
					schedules: [
						{
							schedule,
							recent_outcomes: outcomes,
							prior_responsibility_outcomes: [],
							testable: true
						}
					]
				}
			})
		);
		await page.route(`**/api/companies/${company}/schedules/${id}/history`, (route) =>
			route.fulfill({
				json: {
					occurrences: Array.from({ length: 20 }, (_, index) => ({
						scheduled_for: new Date(Date.now() - index * 86400000).toISOString(),
						disposition: index === 3 ? 'skipped' : 'fired',
						recovered_at: null,
						detail: null,
						superseded_count: 0
					}))
				}
			})
		);
		await page.goto(`${origin}/${company}/company/schedules`);
		await page.getByRole('button', { name: reason, exact: true }).click();
		await page
			.locator('.notice-text')
			.getByText('Synthetic check needs an owner answer.', { exact: true })
			.waitFor({ state: 'visible' });
		assert.equal(
			await page.locator('.runs li:visible').count(),
			0,
			'Older histories do not overwhelm the current result'
		);
		await capture('schedule');
		await page.locator('.result-history summary').click();
		await page.locator('.result-history li').first().waitFor({ state: 'visible' });
		assert.equal(await page.locator('.result-history li:visible').count(), 10);
		await page.locator('.history-settings summary').click();
		await page.getByRole('button', { name: 'Recover', exact: true }).waitFor({ state: 'visible' });
		assert.equal(
			(await page.getByText('Dispatched', { exact: true }).count()) > 0,
			true,
			'Dispatch is not labelled as successful completion'
		);
		await page.unroute(`**/api/companies/${company}/schedules`);
		await page.unroute(`**/api/companies/${company}/schedules/${id}/history`);
	});
	await run('work-controls', async () => {
		await page.goto(`${origin}/${company}/work?view=board`);
		const controls = page.locator('.secondary-actions');
		await controls.waitFor({ state: 'visible' });
		await controls.getByRole('radio', { name: 'Board', exact: true }).waitFor({ state: 'visible' });
		await capture('work-controls');
	});
	await context.close();
}
await browser.close();
await writeFile(`${out}/results.json`, JSON.stringify(results, null, 2));
if (results.some((result) => !result.ok)) process.exitCode = 1;
