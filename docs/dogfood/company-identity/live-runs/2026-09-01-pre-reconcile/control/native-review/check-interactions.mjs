import { chromium } from 'file:///usr/local/lib/node_modules/playwright/index.mjs';
import { writeFile } from 'node:fs/promises';

const browser = await chromium.launch({ headless: true, executablePath: '/usr/bin/chromium' });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
const consoleErrors = [];
page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
await page.goto('http://127.0.0.1:8777/review-gallery.html', { waitUntil: 'networkidle' });
await page.keyboard.press('Tab');
const firstFocus = await page.evaluate(() => {
  const element = document.activeElement;
  const style = getComputedStyle(element);
  return { text: element?.textContent?.trim(), href: element?.getAttribute('href'), outlineStyle: style.outlineStyle, outlineWidth: style.outlineWidth, outlineColor: style.outlineColor };
});
await page.keyboard.press('Enter');
await page.waitForTimeout(100);
const skipResult = await page.evaluate(() => ({ hash: location.hash, targetExists: Boolean(document.querySelector(location.hash)) }));
const blogLink = page.locator('a[href="#blog"]').first();
await blogLink.focus();
const navFocus = await blogLink.evaluate(element => { const style = getComputedStyle(element); return { outlineStyle: style.outlineStyle, outlineWidth: style.outlineWidth, outlineColor: style.outlineColor }; });
await blogLink.click();
await page.waitForTimeout(100);
const fragmentResult = await page.evaluate(() => ({ hash: location.hash, heading: document.querySelector('#blog h2')?.textContent?.trim() }));
const deterministic = await page.evaluate(() => ({
  horizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
  animationCount: document.getAnimations().length,
  reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
  fragmentLinks: [...document.querySelectorAll('a[href^="#"]')].map(link => link.getAttribute('href')),
  missingFragments: [...document.querySelectorAll('a[href^="#"]')].map(link => link.getAttribute('href')).filter(href => !document.querySelector(href))
}));
await browser.close();
const result = {
  checkedAt: new Date().toISOString(),
  method: 'Playwright Chromium, native keyboard input, 1440x1000, reduced-motion emulation',
  url: 'http://127.0.0.1:8777/review-gallery.html',
  firstFocus,
  skipResult,
  navFocus,
  fragmentResult,
  deterministic,
  consoleErrors
};
await writeFile('/company/outputs/identity-evaluation/native-review/interaction-check.json', JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result, null, 2));
if (firstFocus.href !== '#package' || firstFocus.outlineStyle === 'none' || skipResult.hash !== '#package' || !skipResult.targetExists || navFocus.outlineStyle === 'none' || fragmentResult.hash !== '#blog' || deterministic.horizontalOverflow || !deterministic.reducedMotion || deterministic.animationCount || deterministic.missingFragments.length || consoleErrors.length) process.exitCode = 1;
