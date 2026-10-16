/* Partial goal writes must retain source truth and every available Undo.
 * Faults are browser-local; successful requests still reach the real disposable daemon. */
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || new URL(origin).hostname !== '127.0.0.1' || !company?.endsWith('_test'))
	throw new Error('Use a localhost origin and disposable _test company');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-goal-recovery';
await mkdir(out, { recursive: true });
const base = origin + '/api/companies/' + company;
const report = [];
const browser = await chromium.launch();
const read = async () => {
	const response = await fetch(base + '/goals');
	assert(response.ok);
	return (await response.json()).goals;
};
const fixtures = (await read()).filter((goal) => /^Goal archive recovery [AB]$/.test(goal.title));
assert.equal(fixtures.length, 2, 'Seed with scripts/seed-inbox-actions.mjs');
fixtures.sort((a, b) => a.title.localeCompare(b.title));
const [a, b] = fixtures;
async function reset() {
	for (const goal of fixtures) {
		const response = await fetch(base + '/goals/' + goal.id + '/reopen', {
			method: 'POST',
			headers: { Origin: origin, 'Content-Type': 'application/json' },
			body: '{}'
		});
		assert(response.ok);
	}
}
try {
	for (const width of [1440, 390]) {
		await reset();
		const context = await browser.newContext({
			viewport: { width, height: 844 },
			reducedMotion: 'reduce'
		});
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', (error) => errors.push(error.message));
		let mode = 'partial-close';
		const attempts = [];
		await page.route('**/goals/*/close', async (route) => {
			const id = new URL(route.request().url()).pathname.split('/').at(-2);
			attempts.push({ id, action: 'close', mode });
			if (id === b.id && mode === 'partial-close')
				return route.fulfill({
					status: 503,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Fixture unavailable' })
				});
			if (id === b.id && mode === 'lost-response') {
				const response = await route.fetch();
				assert(response.ok());
				return route.abort('connectionreset');
			}
			return route.continue();
		});
		await page.route('**/goals/*/reopen', async (route) => {
			const id = new URL(route.request().url()).pathname.split('/').at(-2);
			attempts.push({ id, action: 'reopen', mode });
			if (id === b.id && mode === 'partial-undo')
				return route.fulfill({
					status: 503,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Fixture unavailable' })
				});
			return route.continue();
		});
		await page.goto(origin + '/' + company + '/work', { waitUntil: 'networkidle' });
		const begin = async () => {
			await page.getByRole('button', { name: 'Clear done', exact: true }).click();
			await page
				.getByRole('dialog')
				.getByRole('button', { name: 'Clear and archive', exact: true })
				.click();
		};
		const archived = async (id) => Boolean((await read()).find((goal) => goal.id === id).closed_at);
		const waitState = async (id, closed) => {
			for (let attempt = 0; attempt < 50; attempt++) {
				if ((await archived(id)) === closed) return;
				await new Promise((resolve) => setTimeout(resolve, 100));
			}
			assert.fail('Goal did not reach expected state');
		};
		const toast = page.locator('.toast').filter({ hasText: 'could not be archived' }).last();
		await begin();
		await toast.waitFor();
		assert(await archived(a.id));
		assert.equal(await archived(b.id), false);
		assert.equal(await toast.getByRole('button', { name: 'Undo', exact: true }).count(), 1);
		await page.locator('section[aria-label="Done"]').waitFor({ state: 'visible' });
		const box = await toast.boundingBox();
		assert(box.x >= 0 && box.x + box.width <= width + 1);
		const controls = await toast.locator('button').evaluateAll((nodes) =>
			nodes.map((node) => {
				const r = node.getBoundingClientRect();
				return { x: r.left, right: r.right, top: r.top, bottom: r.bottom };
			})
		);
		assert(controls.every((r) => r.x >= 0 && r.right <= width + 1 && r.bottom <= 844));
		const labels = await toast.locator('button.undo').evaluateAll((nodes) =>
			nodes.map((node) => {
				const range = document.createRange();
				range.selectNodeContents(node);
				return {
					text: node.textContent,
					lines: new Set([...range.getClientRects()].map((r) => r.top)).size
				};
			})
		);
		assert(
			labels.every((label) => label.lines === 1),
			'Undo and Retry stay on one line on a phone'
		);
		await page.screenshot({ path: out + '/partial-close-' + width + '.png' });
		await toast.getByRole('button', { name: 'Undo', exact: true }).click();
		await waitState(a.id, false);
		assert.equal(await archived(a.id), false);
		assert.equal(await archived(b.id), false);
		report.push({ width, scenario: 'Partial close retains Undo and visible completed work' });

		await begin();
		await toast.waitFor();
		mode = 'healthy';
		const before = attempts.length;
		await toast.getByRole('button', { name: 'Retry', exact: true }).click();
		const success = page.locator('.toast').filter({ hasText: 'goals archived' }).last();
		await success.waitFor();
		assert((await archived(a.id)) && (await archived(b.id)));
		assert.deepEqual(
			attempts
				.slice(before)
				.filter((x) => x.action === 'close')
				.map((x) => x.id),
			[b.id],
			'Retry only failed archive'
		);
		mode = 'partial-undo';
		await success.getByRole('button', { name: 'Undo', exact: true }).click();
		const undoFailed = page.getByRole('alert').filter({ hasText: 'goal is still archived.' });
		await undoFailed.waitFor();
		assert.equal(await archived(a.id), false);
		assert(await archived(b.id));
		mode = 'healthy';
		const undoBefore = attempts.length;
		await undoFailed.getByRole('button', { name: 'Retry', exact: true }).click();
		await waitState(b.id, false);
		assert.equal(await archived(b.id), false);
		assert.deepEqual(
			attempts
				.slice(undoBefore)
				.filter((x) => x.action === 'reopen')
				.map((x) => x.id),
			[b.id],
			'Retry only failed Undo'
		);
		report.push({
			width,
			scenario: 'Retry preserves earlier successes; Undo retry targets only failed reopen'
		});

		mode = 'lost-response';
		await begin();
		const reconciled = page.locator('.toast').filter({ hasText: 'goals archived' }).last();
		await reconciled.waitFor();
		assert((await archived(a.id)) && (await archived(b.id)));
		await reconciled.getByRole('button', { name: 'Undo', exact: true }).click();
		await waitState(b.id, false);
		assert.equal(await archived(a.id), false);
		assert.equal(await archived(b.id), false);
		report.push({ width, scenario: 'Lost committed response is reconciled and remains undoable' });
		mode = 'healthy';
		await begin();
		await page.locator('.toast').filter({ hasText: 'goals archived' }).last().waitFor();
		await page.goto(origin + '/' + company + '/work?goal=' + a.id, { waitUntil: 'networkidle' });
		await page.getByRole('heading', { name: a.title, exact: true }).waitFor();
		await page.locator('section[aria-label="Done"]').waitFor({ state: 'visible' });
		assert.equal(
			await page.getByText('No work under this goal yet', { exact: true }).count(),
			0,
			'An archived goal keeps its completed Work visible after Clear done'
		);
		assert.equal(
			await page.getByRole('button', { name: 'Clear done', exact: true }).count(),
			0,
			'Archived history does not offer Clear done'
		);
		report.push({ width, scenario: 'Archived goal retains its completed Work after Clear done' });
		assert.deepEqual(errors, []);
		await context.close();
	}
} finally {
	await reset();
	await writeFile(out + '/report.json', JSON.stringify(report, null, 2));
	await browser.close();
}
console.log(JSON.stringify(report, null, 2));
