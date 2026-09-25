import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';

// All mutations and attention state are intercepted in this browser-only company.
// GET fixtures come from the local company; no live owner decisions are submitted.
const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:8788';
const source = process.env.RESTLESS_FIXTURE_COMPANY;
assert(source, 'RESTLESS_FIXTURE_COMPANY is required');
const company = 'shared_attention_ui_test';
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-shared-attention';
await fs.mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE });
let context;
try {
	context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
	const original = await (
		await context.request.get(`${origin}/api/companies/${source}/attention`)
	).json();
	const template = original.items.find((i) => i.source.kind === 'owner_handoff');
	assert(template);
	const lead = 'hosting-engineering',
		member = 'cloud-engineering';
	const decision = {
		...structuredClone(template),
		id: 'test-decision',
		title: 'Choose the hosting approach',
		source: { plane: 'orgintel', kind: 'owner_handoff', reference: 'test-decision' },
		responsible_actor: { id: member, display: 'Dobby', role: 'engineer' },
		actions: [
			{
				id: 'record-decision',
				label: 'Record decision',
				role: 'decision',
				consequence: 'Returns your answer to the team.',
				next_state: 'Work resumes.'
			}
		]
	};
	const approval = {
		...structuredClone(decision),
		id: 'test-approval',
		title: 'First contact: example.test',
		category: 'approval',
		source: {
			plane: 'authority',
			kind: 'approval_required',
			reference: 'test-approval',
			party: 'example.test'
		},
		requested_action: 'Allow first contact with example.test.',
		responsible_actor: null,
		runtime_attach: null,
		work_id: null,
		brief_author: { id: member, display: 'Dobby', role: 'engineer' },
		actions: [
			{
				id: 'grant',
				label: 'Grant first contact',
				role: 'decision',
				consequence: 'Allows contact with example.test.',
				next_state: 'The company can retry.'
			},
			{
				id: 'decline',
				label: 'Decline',
				role: 'decision',
				consequence: 'Declines contact.',
				next_state: 'The request closes.'
			}
		]
	};
	const unrelated = {
		...structuredClone(decision),
		id: 'test-unrelated',
		title: 'Another team request',
		work_id: null,
		runtime_attach: null,
		responsible_actor: { id: 'venture-strategy', display: 'Alice', role: 'strategy' }
	};
	let items = [decision, approval, unrelated];
	let failGrant = true,
		decisions = 0,
		grants = 0;
	const errors = [];
	await context.route('**/api/**', async (route) => {
		const req = route.request(),
			url = new URL(req.url());
		if (req.method() !== 'GET') {
			if (url.pathname.endsWith('/handoffs/test-decision/decision')) {
				assert.deepEqual(req.postDataJSON(), {
					resolution: 'Proceed with the smaller deployment.'
				});
				decisions++;
				items = items.filter((i) => i.id !== 'test-decision');
				return route.fulfill({ json: { ok: true } });
			}
			if (url.pathname.endsWith('/approvals/grant')) {
				assert.deepEqual(req.postDataJSON(), { party: 'example.test' });
				grants++;
				if (failGrant) {
					failGrant = false;
					return route.fulfill({
						status: 503,
						json: { message: 'Test service temporarily unavailable' }
					});
				}
				items = items.filter((i) => i.id !== 'test-approval');
				return route.fulfill({ json: { ok: true } });
			}
			return route.fulfill({
				status: 403,
				json: { message: 'Writes disabled in UI verification' }
			});
		}
		if (url.pathname.endsWith('/attention'))
			return route.fulfill({
				json: { ...original, company: { ...original.company, id: company }, items }
			});
		if (url.pathname.endsWith('/principal'))
			return route.fulfill({
				json: { actor_id: 'owner', membership_role: 'owner', cache_partition: 'ui-test' }
			});
		if (url.pathname.endsWith('/activity'))
			return route.fulfill({ contentType: 'text/event-stream', body: '' });
		const mapped = req.url().replaceAll(company, source);
		const response = await context.request.get(mapped);
		const body = (await response.text()).replaceAll(source, company);
		return route.fulfill({
			status: response.status(),
			contentType: response.headers()['content-type'] ?? 'application/json',
			body
		});
	});
	const people = await context.newPage(),
		attention = await context.newPage();
	for (const page of [people, attention]) page.on('pageerror', (e) => errors.push(e.message));
	await people.goto(`${origin}/${company}/people?person=${lead}`);
	await attention.goto(`${origin}/${company}?item=test-decision`);
	const pCard = people.locator('[data-attention-id="test-decision"]');
	const aCard = attention.locator('[data-attention-id="test-decision"]');
	await pCard.waitFor();
	await aCard.waitFor();
	assert.equal(await people.locator('[data-attention-id="test-unrelated"]').count(), 0);
	assert.equal(await people.locator('[data-attention-id="test-approval"]').count(), 1);
	await pCard.getByLabel('Your decision').fill('Proceed with the smaller deployment.');
	await pCard.getByRole('button', { name: 'Record decision', exact: true }).click();
	await pCard.waitFor({ state: 'detached' });
	await aCard.waitFor({ state: 'detached', timeout: 5000 });
	assert.equal(decisions, 1);
	await attention.goto(`${origin}/${company}?item=test-approval`);
	let grant = attention.getByRole('button', { name: 'Hold to grant first contact', exact: true });
	await grant.focus();
	await attention.keyboard.down('Space');
	await attention.waitForTimeout(1200);
	await attention.keyboard.up('Space');
	await attention
		.getByRole('alert')
		.filter({ hasText: 'Test service temporarily unavailable' })
		.waitFor();
	assert.equal(await people.locator('[data-attention-id="test-approval"]').count(), 1);
	grant = attention.getByRole('button', { name: 'Hold to grant first contact', exact: true });
	await grant.focus();
	await attention.keyboard.down('Space');
	await attention.waitForTimeout(1200);
	await attention.keyboard.up('Space');
	await people
		.locator('[data-attention-id="test-approval"]')
		.waitFor({ state: 'detached', timeout: 5000 });
	assert.equal(grants, 2);
	console.log(
		'PASS team scope, same decision in both views, cross-tab sync in both directions, failed approval retry, keyboard hold'
	);
	// Restore browser fixtures for visual checks only.
	items = [decision, approval, unrelated];
	for (const width of [1440, 390, 320]) {
		await people.setViewportSize({ width, height: 1000 });
		await people.goto(`${origin}/${company}/people?person=${lead}`);
		await people.locator('[data-attention-id="test-approval"]').waitFor();
		await people.locator('[data-attention-id="test-decision"]').scrollIntoViewIfNeeded();
		assert(
			await people.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
			`People overflow at ${width}`
		);
		await people.screenshot({ path: `${out}/people-${width}.png` });
		// Phones open on the stacked board; the map key belongs to the map lens.
		await people.goto(`${origin}/${company}/work?lens=map`);
		await people.getByRole('heading', { name: 'Company work', exact: true }).waitFor();
		assert.equal(await people.locator('.work-stage-head a').count(), 1);
		assert.equal(await people.locator('.work-stage-head button').count(), 2);
		await people.getByText('Map key', { exact: true }).click();
		if (width <= 760)
			assert(
				await people
					.getByRole('navigation', { name: 'Work resources' })
					.getByRole('link', { name: 'Documents', exact: true })
					.isVisible()
			);
		assert(
			await people.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
			`Work overflow at ${width}`
		);
		await people.screenshot({ path: `${out}/work-${width}.png` });
		await people.goto(`${origin}/${company}/company/doctor`);
		await people
			.locator('.doctor-diagnostics')
			.getByRole('button', { name: 'Recheck', exact: true })
			.waitFor();
		assert.equal(
			await people
				.locator('.company-page-head')
				.getByRole('button', { name: 'Recheck', exact: true })
				.count(),
			0
		);
		assert.equal(await people.locator('.company-spine-head .info-tip').count(), 0);
		await people.screenshot({ path: `${out}/doctor-${width}.png` });
	}
	await people.goto(`${origin}/${company}/company/resources`);
	await people.getByRole('heading', { name: 'Model spend', exact: true }).waitFor();
	assert.equal(await people.getByRole('combobox', { name: 'Company outcome standard' }).count(), 0);
	assert.deepEqual(errors, []);
	console.log(
		'PASS desktop/mobile Work toolbar, shared cards, Doctor placement, company setting removal; no browser errors'
	);
} finally {
	await context?.unrouteAll({ behavior: 'ignoreErrors' });
	await browser.close();
}
