/* Owner Library count on a disposable local plane; does not touch production. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const [origin, company, expected] = process.argv.slice(2);
assert(['127.0.0.1', 'localhost'].includes(new URL(origin).hostname));
assert(company.endsWith('_test'));
assert(['2000', '2000+'].includes(expected));
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
try {
  for (const width of [1440, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 }, reducedMotion: 'reduce' });
    try {
      await page.goto(`${origin}/${encodeURIComponent(company)}/library`, { waitUntil: 'domcontentloaded' });
      const row = page.getByRole('link', { name: /Folders/ });
      await row.waitFor({ state: 'visible', timeout: 30000 });
      const text = (await row.innerText()).replace(/\s/g, '');
      assert.equal(text, `Folders${expected}`, `unexpected folder count at ${width}: ${text}`);
      if (expected.endsWith('+')) {
        const title = width === 1440 ? await row.locator('.sidebar-reading').getAttribute('title') : await row.getAttribute('title');
        assert.match(title ?? '', /older files are not listed/);
      }
      await row.click();
      await page.waitForURL(url => url.searchParams.get('show') === 'folders');
      await page.getByRole('heading', { name: 'Folders', exact: true }).waitFor();
      assert.equal((await page.locator('.head-count').innerText()).trim(), expected);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await page.screenshot({ path: `/tmp/s109-library-limit-${width}-${expected.replace('+', 'plus')}.png` });
      console.log(JSON.stringify({ width, count: expected, openedFolders: true }));
    } finally { await page.close(); }
  }
} finally { await browser.close(); }
