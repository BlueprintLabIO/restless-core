/* Browser check against an explicitly owned local pressure fixture. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { access, writeFile } from 'node:fs/promises';
const [origin, company, ready, recovered] = process.argv.slice(2);
assert(['127.0.0.1', 'localhost'].includes(new URL(origin).hostname));
assert(company.endsWith('_test'));
const browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
const pages = [];
try {
  for (const width of [1440, 390]) {
    const page = await browser.newPage({viewport: {width, height: 900}, reducedMotion: 'reduce'});
    pages.push(page);
    await page.goto(`${origin}/${encodeURIComponent(company)}/library`, {waitUntil: 'domcontentloaded'});
    await page.getByText('Files could not be read. Try again.', {exact: true}).waitFor({state: 'visible', timeout: 30000});
    const row = page.getByRole('link', {name: /Folders/});
    await row.waitFor({state: 'visible', timeout: 30000});
    assert.equal((await row.innerText()).replace(/\s/g, ''), 'Folders—');
    await row.click();
    await page.locator('.item-title').getByText('isolation/visible.txt', {exact: true}).waitFor({state: 'visible', timeout: 30000});
  }
  await writeFile(ready, 'ready');
  let available = false;
  for (let i = 0; i < 180; i++) {
    if (await access(recovered).then(() => true, () => false)) { available = true; break; }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  assert(available, 'test pressure did not recover');
  for (const page of pages) {
    await page.getByRole('button', {name: 'Try again', exact: true}).click();
    await page.getByText('Files could not be read. Try again.', {exact: true}).waitFor({state: 'hidden', timeout: 30000});
    await page.locator('.item-title').getByText('isolation/visible.txt', {exact: true}).waitFor({state: 'visible'});
    console.log(JSON.stringify({width: page.viewportSize().width, cachedFilesVisible: true, pressureWarned: true, retried: true}));
  }
} finally { await browser.close(); }
