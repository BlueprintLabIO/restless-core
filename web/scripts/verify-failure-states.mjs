/* Owner-facing failure states against a running restless-dev stack.
 *
 * The API is cut at the browser's network layer, which is exactly what the
 * cockpit sees when the daemon is down: fetch() rejects, or a proxy answers
 * without the daemon's JSON. It checks that the owner reads plain copy, never
 * transport text; that a lost connection is one app-level banner while panes
 * keep their data; and that recovery is announced and refreshes the page.
 * Reads only. Point it at a `_test` company. */
import { chromium } from 'playwright';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !company?.endsWith('_test'))
	throw new Error('set RESTLESS_SMOKE_ORIGIN and a _test RESTLESS_SMOKE_COMPANY');

const RAW = /failed to fetch|bad gateway|gateway timeout|internal server error|\b50[0-9]\b|sqlx/i;
const failures = [];
const check = (ok, label) => {
	console.log(`${ok ? 'PASS' : 'FAIL'}  ${label}`);
	if (!ok) failures.push(label);
};
const text = (page) => page.locator('body').innerText();

const browser = await chromium.launch(
	process.env.RESTLESS_BROWSER_EXECUTABLE
		? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
		: {}
);
try {
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));

	// 1. Lost connection: the page keeps its data and one banner speaks for it.
	await page.goto(`${origin}/${company}/work`);
	// A cold dev server compiles on first load; allow for it.
	await page.getByRole('heading', { name: 'Goals' }).waitFor({ timeout: 90_000 });
	const before = await text(page);
	await page.route('**/api/**', (route) => route.abort('connectionrefused'));
	await page.evaluate(() => window.dispatchEvent(new Event('focus')));
	const banner = page.getByText('Connection lost. Reconnecting…');
	await banner.waitFor({ timeout: 20_000 });
	check(true, 'a lost connection shows one app-level banner');
	const during = await text(page);
	check(!RAW.test(during), 'no transport text is shown while disconnected');
	check(
		during.includes('Goals') && before.split('\n').length - during.split('\n').length < 10,
		'Work keeps its last data while disconnected'
	);
	check(
		(await page.locator('.failure-notice').count()) === 0,
		'panes with data stay quiet during an outage'
	);
	check(
		(await page.locator('.cockpit-error:visible').count()) === 0,
		'no empty notice frame is left behind'
	);

	// 2. Recovery is announced once, then the banner leaves.
	await page.unroute('**/api/**');
	await page.getByRole('button', { name: 'Retry now' }).click();
	await page.getByText('Reconnected.').waitFor({ timeout: 15_000 });
	check(true, 'recovery is announced');
	await banner.waitFor({ state: 'detached', timeout: 5_000 });
	check(true, 'the outage banner is removed after recovery');

	// 3. One endpoint failing while the rest answer, on a page that already
	// shows its data, is reported where it failed and keeps that data.
	const warm = await context.newPage();
	warm.on('pageerror', (error) => errors.push(error.message));
	await warm.goto(`${origin}/${company}/work`);
	await warm.getByRole('heading', { name: 'Goals' }).waitFor({ timeout: 60_000 });
	await warm.waitForTimeout(6_000); // past staleTime, so focus refetches
	await warm.route(`**/api/companies/${company}/attention`, (route) =>
		route.fulfill({ status: 502, contentType: 'text/plain', body: 'Bad Gateway' })
	);
	await warm.evaluate(() => window.dispatchEvent(new Event('visibilitychange')));
	await warm.evaluate(() => window.dispatchEvent(new Event('focus')));
	await warm.getByText('Showing the last update.').waitFor({ timeout: 60_000 });
	check(
		(await warm.getByText('Design the storefront identity').count()) > 0,
		'"Showing the last update" appears only over data that is still shown'
	);
	check(!RAW.test(await text(warm)), 'that notice carries no transport text');
	await warm.close();

	// 4. A proxy answering for a dead daemon, before anything is cached, stops
	// the company opening with plain copy rather than the proxy's text.
	const coldContext = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const cold = await coldContext.newPage();
	cold.on('pageerror', (error) => errors.push(error.message));
	await cold.route(`**/api/companies/${company}/attention`, (route) =>
		route.fulfill({ status: 502, contentType: 'text/plain', body: 'Bad Gateway' })
	);
	await cold.goto(`${origin}/${company}`);
	const startup = cold.getByRole('alertdialog');
	await startup.waitFor({ timeout: 60_000 });
	check(
		(await startup.innerText()).includes('Reconnecting automatically'),
		'a proxy 502 on first load reads as a lost connection'
	);
	check(!RAW.test(await text(cold)), 'a proxy 502 never shows its status or body');
	await coldContext.close();

	// 5. A daemon 500 keeps internals out of the copy and points to Doctor.
	const brokenContext = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const broken = await brokenContext.newPage();
	broken.on('pageerror', (error) => errors.push(error.message));
	await broken.route(`**/api/companies/${company}/attention`, (route) =>
		route.fulfill({
			status: 500,
			contentType: 'application/json',
			body: JSON.stringify({ error: 'api', message: 'sqlx: pool timed out' })
		})
	);
	await broken.goto(`${origin}/${company}/work`);
	await broken.getByRole('alertdialog').waitFor({ timeout: 60_000 });
	check(!RAW.test(await text(broken)), 'a daemon 500 keeps internals out of the copy');
	check(
		(await broken.getByRole('alertdialog').innerText()).includes('open Doctor'),
		'a daemon 500 says what to do next'
	);
	check(
		await broken.getByRole('button', { name: 'Try again' }).isVisible(),
		'a failed load offers a retry'
	);
	await brokenContext.close();

	check(errors.length === 0, `no page errors${errors.length ? `: ${errors.join(' | ')}` : ''}`);
	await context.close();
} finally {
	await browser.close();
}
if (failures.length) {
	console.error(`${failures.length} failure-state check(s) failed`);
	process.exit(1);
}
