import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const [origin, company] = process.argv.slice(2);
assert(['127.0.0.1', 'localhost'].includes(new URL(origin).hostname) && company.endsWith('_test'));
const browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
try {
  for (const width of [1440, 390]) {
    const page = await browser.newPage({viewport: {width, height: 900}, reducedMotion: 'reduce'});
    try {
      await page.goto(`${origin}/${company}/company/resources`, {waitUntil: 'domcontentloaded'});
      const spend = page.locator('#spend').first();
      await spend.waitFor({state: 'visible', timeout: 30000});
      await spend.getByRole('button', {name: /Requests already in progress can add charges beyond the limit/}).waitFor({state: 'attached'});
      await spend.getByRole('button', {name: 'Edit limit'}).click();
      await page.getByRole('button', {name: 'Requests already in progress can add charges beyond this limit.'}).waitFor({state: 'visible'});
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      console.log(JSON.stringify({width, warningInEditor: true, helpAvailable: true}));
    } finally { await page.close(); }
  }
} finally { await browser.close(); }
