// Evaluate a snippet on a cockpit path of a dev stack, for layout debugging:
//   node scripts/probe.mjs <path> '<js expression returning JSON>'
import { chromium } from 'playwright';
const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:14912';
const company = process.env.RESTLESS_TEST_COMPANY ?? 's73_test';
if (!company.endsWith('_test')) throw new Error('only a _test company');
const [path, expression] = process.argv.slice(2);
const browser = await chromium.launch();
const page = await browser.newPage({
	viewport: { width: Number(process.env.WIDTH ?? 1440), height: Number(process.env.HEIGHT ?? 900) }
});
await page.goto(`${base}/${company}${path}`);
await page.waitForTimeout(Number(process.env.SETTLE ?? 3500));
console.log(JSON.stringify(await page.evaluate(expression), null, 1));
await browser.close();
