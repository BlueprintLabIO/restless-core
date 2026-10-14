// Sprint 84: what only shows in motion. Still screenshots passed while chats opened at the top,
// tool rows flashed open and replies snapped narrower when they finished (14 October report).
//
// Run against a dev stack and a `_test` company filled by scripts/seed-reno-scale.mjs:
//   RESTLESS_TEST_BASE_URL=http://127.0.0.1:14912 RESTLESS_TEST_COMPANY=s73_test \
//     node scripts/verify-motion.mjs [journey ...]
//
// Journeys:
//   open-latest   a long chat opens at its latest message, and so does switching chats and back
//
// It reads only.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:14912';
const company = process.env.RESTLESS_TEST_COMPANY ?? 's73_test';
assert(company.endsWith('_test'), 'only a _test company is read');
const wanted = process.argv.slice(2);
const run = (name) => wanted.length === 0 || wanted.includes(name);

const browser = await chromium.launch(
	process.env.PW_CHROMIUM ? { executablePath: process.env.PW_CHROMIUM } : {}
);
const report = [];

async function journey(name, body) {
	if (!run(name)) return;
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(String(error)));
	const started = Date.now();
	try {
		const detail = await body(page);
		assert.deepEqual(errors, [], `${name}: page errors`);
		report.push({ journey: name, ok: true, ms: Date.now() - started, ...detail });
	} catch (error) {
		report.push({ journey: name, ok: false, error: String(error?.message ?? error) });
	} finally {
		await context.close();
	}
}

/* How far the chat's scroller sits from its end, once it has stopped moving. */
async function distanceFromEnd(page) {
	const scroller = page.locator('.exr-msgs').first();
	await scroller.waitFor({ timeout: 20_000 });
	let last = -1;
	for (let i = 0; i < 20; i++) {
		const now = await scroller.evaluate((node) => ({
			gap: Math.round(node.scrollHeight - node.clientHeight - node.scrollTop),
			height: node.scrollHeight,
			view: node.clientHeight
		}));
		if (now.height === last && now.height > now.view) return now;
		last = now.height;
		await page.waitForTimeout(250);
	}
	return scroller.evaluate((node) => ({
		gap: Math.round(node.scrollHeight - node.clientHeight - node.scrollTop),
		height: node.scrollHeight,
		view: node.clientHeight
	}));
}

await journey('open-latest', async (page) => {
	await page.goto(`${base}/${company}/people?person=exec`);
	const first = await distanceFromEnd(page);
	assert(first.height > first.view, 'the seeded Exec thread is longer than the screen');
	assert(first.gap <= 48, `opening the Exec chat leaves ${first.gap}px below`);
	// Scroll up as if reading, switch to another chat, then come back.
	await page.locator('.exr-msgs').evaluate((node) => (node.scrollTop = 0));
	await page.locator(`a[href="/${company}/people?person=studio-direction"]`).first().click();
	await page.waitForTimeout(1500);
	await page.locator(`a[href="/${company}/people?person=exec"]`).first().click();
	const back = await distanceFromEnd(page);
	assert(back.gap <= 48, `coming back to the Exec chat leaves ${back.gap}px below`);
	return { first: first.gap, back: back.gap };
});

await browser.close();
console.log(JSON.stringify(report, null, 2));
if (report.some((entry) => !entry.ok)) process.exit(1);
