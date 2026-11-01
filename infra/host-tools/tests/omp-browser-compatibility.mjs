import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

const moduleUrl = process.env.BROKER_PACKAGE_ROOT
  ? pathToFileURL(resolve(process.env.BROKER_PACKAGE_ROOT, 'node_modules/@oh-my-pi/pi-coding-agent/src/tools/browser/launch.ts'))
  : new URL('../node_modules/@oh-my-pi/pi-coding-agent/src/tools/browser/launch.ts', import.meta.url);
const { launchHeadlessBrowser, removeUserDataDir } = await import(moduleUrl.href);
let result;
try {
  result = await launchHeadlessBrowser({ headless:true, viewport:{ width:390,height:844,deviceScaleFactor:1 } });
  const page = await result.browser.newPage();
  await page.goto('data:text/html,<title>OMP browser</title><button onclick="this.textContent=\'ready\'">Start</button>');
  await page.click('button');
  assert.equal(await page.$eval('button', (button) => button.textContent), 'ready');
  assert.equal(await page.title(), 'OMP browser');
  console.log('OMP browser launch, profile management and interaction passed');
} finally {
  await result?.browser.close();
  if (result?.userDataDir) await removeUserDataDir(result.userDataDir);
}
