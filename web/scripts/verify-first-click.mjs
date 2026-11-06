// 6 Nov: arriving on Company › Health and clicking "Rebuild company computer" at once did nothing;
// the second click opened the confirm. A surface change crossfades (routes/+layout.svelte), and
// while that view transition runs Chrome hit-tests the whole page as the root element, so a press
// on anything that had just appeared reached nothing. lib/transition.ts now ends the crossfade on
// a press and hands the click to what it was aimed at.
//
// Each journey arrives by a client navigation, as the owner does, and acts the moment the
// control is on screen, at 1440 and 390 wide:
//   rebuild   the sidebar's Health row, then "Rebuild company computer": the confirm opens on the
//             first click (cancelled, never confirmed). The company read is answered with a
//             failing image check so the action is there on any stack.
//   recheck   the same arrival, then "Check again": the check starts on the first click.
//   link      Health, then at once a sidebar row: the first click navigates.
//
//   RESTLESS_TEST_BASE_URL=http://127.0.0.1:13534 RESTLESS_TEST_COMPANY=s73_test \
//     node scripts/verify-first-click.mjs [journey ...]
//
// Reads only: nothing is confirmed.
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
const health = `/${company}/company/doctor`;

const browser = await chromium.launch(
	process.env.PW_CHROMIUM ? { executablePath: process.env.PW_CHROMIUM } : {}
);
const report = [];

/* The company read with a failing image check and its rebuild action. */
async function failingImage(page) {
	await page.route(
		(url) => url.pathname === `/api/companies/${company}/company`,
		async (route) => {
			const response = await route.fetch();
			const body = await response.json();
			const doctor = body?.computer?.doctor;
			if (doctor) {
				doctor.status = 'degraded';
				doctor.checks = [
					{
						id: 'image',
						label: 'Company services',
						status: 'degraded',
						source: 'runtime',
						summary: 'One or more supervised services are not healthy.'
					},
					...doctor.checks.filter((check) => check.id !== 'image')
				];
				doctor.actions = [
					{
						id: 'reconcile',
						label: 'Rebuild company computer',
						consequence: 'Rebuilds the company computer from its image.',
						confirmation: 'Rebuild the company computer?'
					}
				];
			}
			await route.fulfill({ response, json: body });
		}
	);
}

/* Counts the crossfades that ran, so a pass shows the click really landed during one. */
const countTransitions = () => {
	window.__transitions = 0;
	const start = document.startViewTransition?.bind(document);
	if (start)
		document.startViewTransition = (argument) => {
			window.__transitions++;
			return start(argument);
		};
};

/* One press as a pointer makes it, at the control's centre, the moment it is visible. */
async function pressAtOnce(page, locator) {
	await locator.waitFor({ state: 'visible', timeout: 20_000 });
	const box = await locator.boundingBox();
	assert(box, 'the control has no box');
	await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
}

async function arrive(page, width, from, rowHref) {
	await page.goto(`${base}${from}`);
	await page.waitForSelector('.company-browser-link-capture:not(.first-paint-pending)', {
		timeout: 60_000
	});
	await page.waitForTimeout(600);
	await page.locator(`a[href="${rowHref}"]`).filter({ visible: true }).first().click();
}

async function journey(name, width, body) {
	if (!run(name)) return;
	const context = await browser.newContext({
		viewport: { width, height: width < 600 ? 844 : 900 }
	});
	const page = await context.newPage();
	await page.addInitScript(countTransitions);
	const errors = [];
	page.on('pageerror', (error) => errors.push(String(error)));
	try {
		const detail = await body(page);
		assert.deepEqual(errors, [], `${name}: page errors`);
		report.push({ journey: name, width, ok: true, ...detail });
	} catch (error) {
		report.push({ journey: name, width, ok: false, error: String(error?.message ?? error) });
	} finally {
		await context.close();
	}
}

for (const width of [1440, 390]) {
	await journey('rebuild', width, async (page) => {
		await failingImage(page);
		await arrive(page, width, `/${company}/company`, health);
		await pressAtOnce(page, page.getByRole('button', { name: 'Rebuild company computer' }));
		const opened = await page
			.waitForSelector('dialog.confirm[open]', { timeout: 1500 })
			.then(() => true)
			.catch(() => false);
		assert(opened, 'the first click did not open the confirm');
		await page.getByRole('button', { name: 'Cancel' }).click();
		return { transitions: await page.evaluate(() => window.__transitions) };
	});

	await journey('recheck', width, async (page) => {
		await arrive(page, width, `/${company}/company`, health);
		const button = page.getByRole('button', { name: 'Check again' });
		const reads = [];
		page.on('request', (request) => {
			if (new URL(request.url()).pathname === `/api/companies/${company}/company`)
				reads.push(Date.now());
		});
		await button.first().waitFor({ state: 'visible', timeout: 20_000 });
		const before = reads.length;
		await pressAtOnce(page, button.first());
		await page.waitForTimeout(1200);
		assert(reads.length > before, 'the first click did not check again');
		return { transitions: await page.evaluate(() => window.__transitions) };
	});

	await journey('link', width, async (page) => {
		await arrive(page, width, `/${company}/company`, health);
		const back =
			width < 600
				? page.locator(`a.company-back[href="/${company}/company"]`)
				: page.locator(`a[href="/${company}/company/vault"]`).filter({ visible: true });
		await page.getByRole('heading', { name: 'Health' }).first().waitFor({ timeout: 20_000 });
		await pressAtOnce(page, back.first());
		await page.waitForURL((url) => url.pathname !== health, { timeout: 2000 });
		return { landed: new URL(page.url()).pathname };
	});
}

await browser.close();
console.log(JSON.stringify(report, null, 2));
if (report.some((entry) => !entry.ok)) process.exit(1);
