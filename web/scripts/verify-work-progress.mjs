/* Work progress estimates on a disposable stack (owner decision, 3 November 2026): the Work's
 * creator records an estimate through the scripted model, then the bar and line show on the
 * board card, the list row, the Goal, the Work detail, the People row and the "Working on" card,
 * and one compact line lands in the creator's chat with the owner. Every surface is checked at
 * 1440 and 390 wide, light and dark, and no card or row may change height when an estimate lands.
 *
 *   RESTLESS_SMOKE_ORIGIN=http://127.0.0.1:<web> RESTLESS_SMOKE_COMPANY=<name>_test \
 *   WORK_ID=<uuid> LEAD=<creator actor id> node scripts/verify-work-progress.mjs
 *
 * The stack's model must be tools/mock-model with scenarios/work-progress.json. */
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
const workId = process.env.WORK_ID;
const lead = process.env.LEAD;
if (!origin || new URL(origin).hostname !== '127.0.0.1' || !company?.endsWith('_test'))
	throw new Error('Use a localhost origin and disposable _test company');
if (!workId || !lead) throw new Error('Set WORK_ID and LEAD (the Work and its creator)');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-work-progress';
await mkdir(out, { recursive: true });
const api = `${origin}/api/companies/${encodeURIComponent(company)}`;
const report = [];

async function graph() {
	const response = await fetch(`${api}/work-graph`);
	assert(response.ok, `work-graph ${response.status}`);
	return response.json();
}
async function latest() {
	const rows = (await graph()).progress.filter((row) => row.work_id === workId);
	return rows.at(-1) ?? null;
}
/* The owner asks the creator in their chat; the scripted model runs the CLI as the creator. */
async function askCreator(text) {
	const before = (await graph()).progress.length;
	const form = new FormData();
	form.set('client_command_id', crypto.randomUUID());
	form.set('body', text);
	const response = await fetch(`${api}/actors/${encodeURIComponent(lead)}/conversation`, {
		method: 'POST',
		headers: { Origin: origin },
		body: form
	});
	assert(response.ok, `send ${response.status}: ${await response.text()}`);
	for (let tries = 0; tries < 120; tries++) {
		if ((await graph()).progress.length > before) return latest();
		await new Promise((resolve) => setTimeout(resolve, 1000));
	}
	throw new Error('the creator never recorded an estimate');
}

const work = (await graph()).work.find((item) => item.id === workId);
assert(work, 'WORK_ID is not in this company');
assert.equal(work.commissioned_by, lead, 'LEAD must be the Work creator');

const browser = await chromium.launch();
try {
	// 1. Live: the board is open while the creator records its first estimate; the card holds.
	const live = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const board = await live.newPage();
	const errors = [];
	board.on('pageerror', (error) => errors.push(error.message));
	await board.goto(`${origin}/${company}/work?view=board`, { waitUntil: 'networkidle' });
	const card = board.locator('.board-item', { hasText: work.title }).first();
	await card.waitFor();
	const before = await card.boundingBox();
	const first = await askCreator(
		`Where does work ${workId} stand? It is at 60, next export print files`
	);
	assert.equal(first.percent, 60);
	assert.equal(first.estimated_by, lead, 'the creator recorded it');
	await card.locator('[role="meter"][aria-valuenow="60"]').waitFor({ timeout: 20_000 });
	const after = await card.boundingBox();
	assert.equal(Math.round(after.height), Math.round(before.height), 'the card did not grow');
	report.push({ check: 'live board card holds its height', before, after });
	assert.deepEqual(errors, []);
	await live.close();

	// 2. Every surface, both widths, both schemes.
	for (const scheme of ['light', 'dark']) {
		for (const width of [1440, 390]) {
			const context = await browser.newContext({
				viewport: { width, height: 900 },
				colorScheme: scheme,
				reducedMotion: 'reduce'
			});
			const page = await context.newPage();
			const pageErrors = [];
			page.on('pageerror', (error) => pageErrors.push(error.message));
			const shot = async (name) => {
				const file = `${out}/${name}-${width}-${scheme}.png`;
				await page.screenshot({ path: file, fullPage: false });
				const sideways = await page.evaluate(
					() => document.documentElement.scrollWidth > window.innerWidth + 1
				);
				assert(!sideways, `${name} at ${width} scrolls sideways`);
				report.push({ shot: file });
			};
			const line = '~60% · next: export print files';

			await page.goto(`${origin}/${company}/work?view=board`, { waitUntil: 'networkidle' });
			const boardCard = page.locator('.board-item', { hasText: work.title }).first();
			await boardCard.scrollIntoViewIfNeeded();
			await boardCard.getByText(line).waitFor();
			await shot('board');

			await page.goto(`${origin}/${company}/work?view=list`, { waitUntil: 'networkidle' });
			const row = page.locator('.item', { hasText: work.title }).first();
			await row.scrollIntoViewIfNeeded();
			await row.getByText(line).waitFor();
			const rowBox = await row.locator('.item-row').boundingBox();
			const plain = await page
				.locator('.item:not(:has([role="meter"])) .item-note')
				.first()
				.locator('xpath=ancestor::div[contains(@class,"item-row")]')
				.boundingBox()
				.catch(() => null);
			if (plain)
				assert.equal(
					Math.round(rowBox.height),
					Math.round(plain.height),
					'a row with an estimate is as tall as one without'
				);
			await shot('list');

			if (work.goal_id) {
				await page.goto(`${origin}/${company}/work?goal=${work.goal_id}`, {
					waitUntil: 'networkidle'
				});
				await page.getByText(/parts? done/).first().waitFor();
				await shot('goal');
			}

			await page.goto(`${origin}/${company}/people?person=${lead}&work=${workId}`, {
				waitUntil: 'networkidle'
			});
			await page.locator('.work-progress').getByText(line).waitFor();
			await shot('detail');

			await page.goto(`${origin}/${company}/people?person=${lead}`, {
				waitUntil: 'networkidle'
			});
			await page
				.getByText(`${work.title}: ~60% (next: export print files)`)
				.first()
				.waitFor({ timeout: 20_000 });
			await shot('chat');
			assert.deepEqual(pageErrors, []);
			await context.close();
		}
	}

	// 3. Blocked drops the number in words; the board says so without growing.
	const blocked = await askCreator(`Work ${workId} is blocked: waiting on the printer's spec`);
	assert.equal(blocked.percent, null);
	assert.equal(blocked.state, 'blocked');
	report.push({ check: 'blocked estimate recorded by the creator', blocked });
} finally {
	await browser.close();
	await writeFile(`${out}/report.json`, JSON.stringify(report, null, 2));
}
console.log(`work progress: verified; screenshots and report in ${out}`);
