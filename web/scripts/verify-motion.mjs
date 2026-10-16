// Sprint 84: what only shows in motion. Still screenshots passed while chats opened at the top,
// tool rows flashed open and replies snapped narrower when they finished (14 October report).
//
// Run against a dev stack and a `_test` company filled by scripts/seed-reno-scale.mjs:
//   RESTLESS_TEST_BASE_URL=http://127.0.0.1:14912 RESTLESS_TEST_COMPANY=s73_test \
//     node scripts/verify-motion.mjs [journey ...]
//
// Journeys:
//   open-latest   a long chat opens at its latest message, and so does switching chats and back
//   stream        Exec streams a reply after tool calls: the reply keeps its width when it ends,
//                 and no tool row flashes into view and away within half a second. Needs the
//                 stack pointed at tools/mock-model (RESTLESS_ANTHROPIC_BASE_URL) with a model
//                 connected; it sends one message ("watch me work"), so it writes to the
//                 `_test` company's Exec conversation.
//
// open-latest reads only.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const base = process.env.RESTLESS_SMOKE_ORIGIN ?? process.env.RESTLESS_TEST_BASE_URL;
const company = process.env.RESTLESS_SMOKE_COMPANY ?? process.env.RESTLESS_TEST_COMPANY;
assert(
	base && new URL(base).hostname === '127.0.0.1' && company?.endsWith('_test'),
	'Set an explicit localhost origin and _test company'
);
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

await journey('stream', async (page) => {
	await page.goto(`${base}/${company}/people?person=exec`);
	await page.locator('.exr-msgs').first().waitFor({ timeout: 20_000 });
	await page.waitForTimeout(1500);
	// Watch the chat from before the reply exists: elements that arrive and leave, and the width of
	// the newest Exec message, sampled every 50 ms until it has been still for 3 s.
	await page.evaluate(() => {
		const root = document.querySelector('.exr-msgs');
		const seen = new Map();
		const flashes = [];
		const watcher = new MutationObserver((records) => {
			const now = performance.now();
			for (const record of records) {
				for (const node of record.addedNodes)
					if (node instanceof HTMLElement && /tool|activity|step/i.test(node.className))
						seen.set(node, now);
				for (const node of record.removedNodes)
					if (seen.has(node) && now - seen.get(node) < 500)
						flashes.push(
							`${node.className.toString().slice(0, 60)} ${Math.round(now - seen.get(node))}ms`
						);
			}
		});
		watcher.observe(root, { childList: true, subtree: true });
		window.__motion = { flashes, widths: [] };
		const sample = () => {
			const replies = root.querySelectorAll(
				'[data-message-sender]:not([data-message-sender="owner"])'
			);
			const last = replies[replies.length - 1];
			if (last) window.__motion.widths.push(Math.round(last.getBoundingClientRect().width));
			window.__motion.timer = setTimeout(sample, 50);
		};
		sample();
	});
	const form = new FormData();
	form.set('body', 'Please watch me work and tell me where things stand.');
	form.set('client_command_id', crypto.randomUUID());
	const sent = await fetch(`${base}/api/companies/${company}/actors/exec/conversation`, {
		method: 'POST',
		headers: { Origin: base },
		body: form
	});
	assert(sent.ok, `sending failed: ${sent.status}`);
	// Wait for the reply to land and settle.
	let settled = 0;
	let last = -1;
	for (let i = 0; i < 240 && settled < 60; i++) {
		await page.waitForTimeout(50);
		const widths = await page.evaluate(() => window.__motion.widths.length);
		settled = widths === last ? settled + 1 : 0;
		last = widths;
		const text = await page.locator('.exr-msgs').innerText();
		if (!text.includes('Here is where things stand')) settled = 0;
	}
	const motion = await page.evaluate(() => {
		clearTimeout(window.__motion.timer);
		return window.__motion;
	});
	assert(
		(await page.locator('.exr-msgs').innerText()).includes('Here is where things stand'),
		'the scripted reply never arrived: is the stack pointed at tools/mock-model with a model connected?'
	);
	const tail = motion.widths.slice(-20);
	const finalWidth = tail.at(-1);
	const streamingWidth =
		motion.widths
			.filter((width) => width > 0)
			.slice(-40, -20)
			.at(-1) ?? finalWidth;
	assert.deepEqual(motion.flashes, [], 'tool rows flashed into view and away');
	assert(
		Math.abs((finalWidth ?? 0) - (streamingWidth ?? 0)) <= 4,
		`the reply snapped from ${streamingWidth}px to ${finalWidth}px when it ended`
	);
	return { streamingWidth, finalWidth, samples: motion.widths.length };
});

await browser.close();
console.log(JSON.stringify(report, null, 2));
if (report.some((entry) => !entry.ok)) process.exit(1);
