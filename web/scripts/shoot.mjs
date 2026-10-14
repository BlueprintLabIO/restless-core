// Screenshot cockpit paths on a dev stack for review in a hidden or covered desktop session:
//   node scripts/shoot.mjs <out-dir> <path> [<path> …]   (paths are relative to the company)
// Env: RESTLESS_TEST_BASE_URL, RESTLESS_TEST_COMPANY (a `_test` company), WIDTH, HEIGHT, SCHEME.
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:14912';
const company = process.env.RESTLESS_TEST_COMPANY ?? 's73_test';
if (!company.endsWith('_test')) throw new Error('only a _test company');
const [out, ...paths] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch();
const context = await browser.newContext({
	viewport: { width: Number(process.env.WIDTH ?? 1440), height: Number(process.env.HEIGHT ?? 900) },
	colorScheme: process.env.SCHEME === 'dark' ? 'dark' : 'light'
});
const page = await context.newPage();
const errors = [];
page.on('pageerror', (error) => errors.push(String(error)));
for (const [index, path] of paths.entries()) {
	await page.goto(`${base}/${company}${path}`);
	await page.waitForTimeout(Number(process.env.SETTLE ?? 3500));
	const name = `${String(index).padStart(2, '0')}-${path.replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '') || 'home'}.png`;
	await page.screenshot({ path: `${out}/${name}` });
	console.log(`${out}/${name}`);
}
if (errors.length) console.log('page errors:', errors);
await browser.close();
