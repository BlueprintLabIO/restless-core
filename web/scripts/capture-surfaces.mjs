/* Owner-surface capture for visual review: every primary route at desktop,
 * tablet and phone widths, with overflow and page-error probes. It reads only;
 * point it at a `_test` company on a running restless-dev stack. */
import { chromium } from 'playwright';
import fs from 'node:fs/promises';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !company?.endsWith('_test'))
	throw new Error('set RESTLESS_SMOKE_ORIGIN and a _test RESTLESS_SMOKE_COMPANY');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-surfaces';
const only = process.env.ROUTES?.split(',');
const widths = (process.env.WIDTHS ?? '1440,1024,390').split(',').map(Number);
const reduced = process.env.REDUCED === '1';

const routes = [
	['home', '/'],
	['attention', `/${company}`],
	['work', `/${company}/work`],
	['work-documents', `/${company}/work/documents`],
	['people', `/${company}/people`],
	['people-rooms', `/${company}/people/rooms`],
	['authority', `/${company}/authority`],
	['company', `/${company}/company`],
	['company-identity', `/${company}/company/identity`],
	['company-provider', `/${company}/company/provider`],
	['company-members', `/${company}/company/members`],
	['company-resources', `/${company}/company/resources`],
	['company-schedules', `/${company}/company/schedules`],
	['company-skills', `/${company}/company/skills`],
	['company-harnesses', `/${company}/company/harnesses`],
	['company-vault', `/${company}/company/vault`],
	['company-decisions', `/${company}/company/decisions`],
	['company-actions', `/${company}/company/actions`],
	['company-doctor', `/${company}/company/doctor`],
	['company-computer', `/${company}/company/computer`],
	['company-authority', `/${company}/company/authority`],
	['account', '/account/settings'],
	['connections', '/account/settings/connections']
].filter(([name]) => !only || only.includes(name));

await fs.mkdir(out, { recursive: true });
const browser = await chromium.launch(
	process.env.RESTLESS_BROWSER_EXECUTABLE
		? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
		: {}
);
const report = [];
try {
	for (const width of widths) {
		const context = await browser.newContext({
			viewport: { width, height: width < 600 ? 844 : 900 },
			deviceScaleFactor: width < 600 ? 2 : 1,
			reducedMotion: reduced ? 'reduce' : 'no-preference'
		});
		for (const [name, path] of routes) {
			const page = await context.newPage();
			const errors = [];
			page.on('pageerror', (e) => errors.push(e.message));
			page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
			await page
				.goto(origin + path, { waitUntil: 'networkidle' })
				.catch((e) => errors.push(String(e)));
			await page.waitForTimeout(700);
			const overflow = await page.evaluate(() => {
				const doc = document.documentElement;
				const wide = [...document.querySelectorAll('body *')]
					.filter((el) => {
						const r = el.getBoundingClientRect();
						return (
							r.width > 0 &&
							r.right > doc.clientWidth + 1 &&
							getComputedStyle(el).position !== 'fixed'
						);
					})
					.slice(0, 5)
					.map((el) => `${el.tagName.toLowerCase()}.${[...el.classList].join('.')}`);
				return { scroll: doc.scrollWidth > doc.clientWidth + 1, wide };
			});
			const file = `${out}/${name}-${width}${reduced ? '-rm' : ''}.png`;
			await page.screenshot({ path: file });
			report.push({ name, width, url: page.url().replace(origin, ''), errors, overflow });
			await page.close();
		}
		await context.close();
	}
} finally {
	await browser.close();
}
await fs.writeFile(`${out}/report.json`, JSON.stringify(report, null, 2));
for (const r of report) {
	/* Clipped off-canvas panes (a closed rail) are wide but invisible; only a
	 * page that actually scrolls sideways is a defect worth naming. */
	const flags = [
		r.overflow.scroll && `H-SCROLL ${r.overflow.wide.join(' ')}`,
		r.errors.length && `errors:${r.errors.length}`
	].filter(Boolean);
	console.log(
		`${r.name.padEnd(18)} ${String(r.width).padEnd(5)} ${r.url.padEnd(40)} ${flags.join(' ') || 'ok'}`
	);
}
