/* Owner Library count on a disposable local plane; does not touch production. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
const [origin, company, expected, container] = process.argv.slice(2);
assert(['127.0.0.1', 'localhost'].includes(new URL(origin).hostname));
assert(company.endsWith('_test'));
assert(['2000', '2000+', 'unsupported', 'changed', 'unavailable'].includes(expected));
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
try {
  for (const width of [1440, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 }, reducedMotion: 'reduce' });
    try {
      if (expected === 'unavailable') {
        let failures = 0;
        await page.route('**/library/outputs**', route => {
          if (failures++ < 2) return route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'library', message: 'Files could not be read. Try again.' }) });
          return route.continue();
        });
      }
      await page.goto(`${origin}/${encodeURIComponent(company)}/library`, { waitUntil: 'domcontentloaded' });
      if (expected === 'unavailable') {
        const notice = page.getByText('Files could not be read. Try again.', { exact: true });
        await notice.waitFor({ state: 'visible', timeout: 30000 });
        const row = page.getByRole('link', { name: /Folders/ });
        await row.waitFor({ state: 'visible', timeout: 30000 });
        assert.equal((await row.innerText()).replace(/\s/g, ''), 'Folders—');
        await page.getByRole('button', { name: 'Try again', exact: true }).click();
        await notice.waitFor({ state: 'visible', timeout: 30000 });
        await page.getByRole('button', { name: 'Try again', exact: true }).click();
        await notice.waitFor({ state: 'hidden', timeout: 30000 });
        await page.waitForFunction(() => [...document.querySelectorAll('a')].some(a => a.textContent?.replace(/\s/g, '') === 'Folders2000'), { timeout: 30000 });
        console.log(JSON.stringify({ width, unavailableWarned: true, repeatedRetryRecovered: true }));
        continue;
      }
      if (expected === 'unsupported') {
        await page.getByText('Some file names cannot be displayed.', { exact: true }).waitFor({ state: 'visible', timeout: 30000 });
        const row = page.getByRole('link', { name: /Folders/ });
        await row.waitFor({ state: 'visible', timeout: 30000 });
        assert.equal((await row.innerText()).replace(/\s/g, ''), 'Folders2000+');
        const title = width === 1440 ? await row.locator('.sidebar-reading').getAttribute('title') : await row.getAttribute('title');
        assert.match(title ?? '', /Some file names cannot be displayed/);
        console.log(JSON.stringify({ width, unsupportedNameReported: true }));
        continue;
      }
      if (expected === 'changed') {
        const row = page.getByRole('link', { name: /Folders/ });
        await row.waitFor({ state: 'visible', timeout: 30000 });
        await row.click();
        await page.getByRole('heading', { name: 'Folders', exact: true }).waitFor();
        execFileSync('docker', ['exec', '-u', 'company', container, 'sh', '-c', `touch /company/outputs/limit-test/browser-change-${width}`]);
        await page.getByRole('button', { name: 'Load older files' }).click();
        await page.getByText('Files changed. Refresh the list to continue.', { exact: true }).waitFor({ state: 'visible', timeout: 30000 });
        await page.getByRole('button', { name: 'Refresh', exact: true }).click();
        await page.getByRole('button', { name: 'Load older files' }).waitFor({ state: 'visible', timeout: 30000 });
        await page.getByText('Files changed. Refresh the list to continue.', { exact: true }).waitFor({ state: 'hidden', timeout: 30000 });
        console.log(JSON.stringify({ width, staleCursorWarning: true, refreshed: true }));
        continue;
      }
      const row = page.getByRole('link', { name: /Folders/ });
      await row.waitFor({ state: 'visible', timeout: 30000 });
      const text = (await row.innerText()).replace(/\s/g, '');
      assert.equal(text, `Folders${expected}`, `unexpected folder count at ${width}: ${text}`);
      if (expected.endsWith('+')) {
        const title = width === 1440 ? await row.locator('.sidebar-reading').getAttribute('title') : await row.getAttribute('title');
        assert.match(title ?? '', /more older files are available/);
      }
      // Exercise visibility events without leaving the local test page.
      if (width === 1440 && expected === '2000') {
        let reads = 0;
        page.on('request', request => { if (request.url().includes('/library/outputs')) reads++; });
        await page.evaluate(() => {
          Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
          document.dispatchEvent(new Event('visibilitychange'));
        });
        await page.waitForTimeout(200);
        assert.equal(reads, 0, 'hidden page refreshed outputs');
        const refreshed = page.waitForRequest(request => request.url().includes('/library/outputs'));
        await page.evaluate(() => {
          Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' });
          document.dispatchEvent(new Event('visibilitychange'));
        });
        await refreshed;
      }
      await row.click();
      await page.waitForURL(url => url.searchParams.get('show') === 'folders');
      await page.getByRole('heading', { name: 'Folders', exact: true }).waitFor();
      assert.equal((await page.locator('.head-count').innerText()).trim(), expected);
      if (expected.endsWith('+')) {
        const search = page.locator('input[placeholder="Search files"]:visible');
        await search.fill('no-such-file-in-this-test');
        await page.getByText('No matches in loaded files', { exact: true }).waitFor();
        await page.getByText('Load older files to continue searching.', { exact: true }).waitFor({ state: 'attached' });
        await search.fill('');
        await page.getByRole('button', { name: 'Load older files' }).click();
        await page.waitForFunction(() => document.querySelector('.head-count')?.textContent?.trim() === '2001', { timeout: 30000 });
        await page.getByRole('button', { name: 'Load older files' }).waitFor({ state: 'hidden', timeout: 30000 });
      }
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await page.screenshot({ path: `/tmp/s109-library-limit-${width}-${expected.replace('+', 'plus')}.png` });
      console.log(JSON.stringify({ width, count: expected, openedFolders: true }));
    } finally { await page.close(); }
  }
} finally { await browser.close(); }
