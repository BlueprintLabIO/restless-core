import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  headless: true,
  args: [
    '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--ignore-gpu-blocklist', '--no-sandbox', '--enable-webgl', '--disable-setuid-sandbox',
  ],
});
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
const errors = [];
page.on('pageerror', error => errors.push(String(error)));
page.on('console', message => {
  if (message.type() === 'error' && !/favicon|Failed to load resource/i.test(message.text())) {
    errors.push(message.text());
  }
});

await page.goto('http://127.0.0.1:8134/index.html', { waitUntil: 'domcontentloaded' });
await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 20000 });
await page.click('.starter-card:nth-child(1) .sc-btn');
await page.waitForFunction(() => window.__game.mode === 'play', null, { timeout: 4000 });
await page.waitForTimeout(500);

const observed = await page.evaluate(() => {
  const game = window.__game;
  const cairn = game.world?.props?.cairn;
  const beacon = game.world?.props?.cairnBeacon;
  if (!cairn || !beacon) return null;
  const Vector3 = game.player.pos.constructor;
  const cairnWorld = cairn.getWorldPosition(new Vector3());
  const rendered = [];
  beacon.traverse(object => {
    if (!object.isMesh || object.visible === false) return;
    const position = object.getWorldPosition(new Vector3());
    rendered.push([position.x, position.y, position.z]);
  });
  return {
    cairn: [cairnWorld.x, cairnWorld.y, cairnWorld.z],
    rendered,
  };
});

const centredAbove = !!observed && observed.rendered.length > 0
  && observed.rendered.some(([x, y, z]) => (
    Math.hypot(x - observed.cairn[0], z - observed.cairn[2]) < 0.5
    && y > observed.cairn[1] + 1
  ));
console.log(`${centredAbove ? '[PASS]' : '[FAIL]'} rendered beacon geometry is centred above the existing cairn :: ${JSON.stringify(observed)}`);
console.log(`${errors.length === 0 ? '[PASS]' : '[FAIL]'} no browser errors :: ${errors.slice(0, 3).join(' | ')}`);

await browser.close();
process.exit(centredAbove && errors.length === 0 ? 0 : 1);
