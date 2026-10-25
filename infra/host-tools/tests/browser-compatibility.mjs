// Run against a disposable browser, with no owner profile or external website.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const require = createRequire(process.env.BROKER_PACKAGE_ROOT
  ? resolve(process.env.BROKER_PACKAGE_ROOT, 'package.json')
  : new URL('../package.json', import.meta.url));
const puppeteer = require('puppeteer-core');
const html = '<title>Broker compatibility</title><input id="value"><button id="add" onclick="this.textContent=String(Number(this.textContent)+1)">0</button>';
const fixture = 'data:text/html,'+encodeURIComponent(html);
let browser;
let child;
let bidi;
try {
  browser = await puppeteer.launch({
    executablePath: process.env.BROWSER_TEST_EXECUTABLE ?? '/usr/bin/chromium',
    headless: true,
    args: process.env.BROWSER_TEST_NO_SANDBOX === '1' ? ['--no-sandbox'] : []
  });
  child = browser.process();
  console.log('Browser:', await browser.version());
  const page = await browser.newPage();
  await page.goto(fixture);
  await page.type('#value', 'retained input');
  await page.click('#add');
  assert.equal(await page.$eval('#add', (button) => button.textContent), '1');
  await page.setViewport({ width: 390, height: 844 });
  const screenshot = Buffer.from(await page.screenshot());
  assert.equal(screenshot.readUInt32BE(16), 390);
  assert.equal(screenshot.readUInt32BE(20), 844);
  const first = await browser.createBrowserContext();
  const second = await browser.createBrowserContext();
  await first.setCookie({ name:'scope', value:'first', domain:'example.test', path:'/' });
  assert.equal((await first.cookies()).length, 1);
  assert.equal((await second.cookies()).length, 0);
  await first.close();
  await second.close();
  const endpoint = browser.wsEndpoint();
  await browser.disconnect();
  browser = await puppeteer.connect({ browserWSEndpoint:endpoint });
  const retained = (await browser.pages()).find((candidate) => candidate.url()===fixture);
  assert.ok(retained, 'reconnection retains the tab');
  assert.equal(await retained.$eval('#value', (input) => input.value), 'retained input');
  console.log('CDP typing, click, phone screenshot, isolated cookies and reconnect passed');

  // Exercise chromium-bidi's repaired Zod dependency through the actual mapper.
  bidi = await puppeteer.connect({ browserWSEndpoint:endpoint, protocol:'webDriverBiDi' });
  const mapped = await bidi.newPage();
  await mapped.goto(fixture);
  await mapped.click('#add');
  assert.equal(await mapped.$eval('#add', (button) => button.textContent), '1');
  await mapped.close();
  console.log('WebDriver BiDi mapping and interaction passed');
} finally {
  await bidi?.disconnect().catch(() => {});
  await browser?.close().catch(() => {});
  if (child && child.exitCode === null) child.kill('SIGTERM');
}
