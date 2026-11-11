import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const URL = 'http://127.0.0.1:8133/index.html';
const WEBGL_ARGS = [
  '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--ignore-gpu-blocklist', '--no-sandbox', '--enable-webgl', '--disable-setuid-sandbox',
];
const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));
const errors = [];
let step = 0;
function ok(name, condition, extra = '') {
  step += 1;
  const tag = condition ? 'PASS' : 'FAIL';
  if (!condition) process.exitCode = 1;
  console.log(`[${tag}] #${step} ${name}${extra ? ` :: ${extra}` : ''}`);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  headless: true,
  args: WEBGL_ARGS,
});
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
page.on('console', message => {
  if (message.type() === 'error' && !/favicon|Failed to load resource/i.test(message.text())) {
    errors.push(message.text());
  }
});
page.on('pageerror', error => errors.push(String(error)));
page.on('requestfailed', request => {
  if (!/favicon/i.test(request.url())) errors.push(`REQFAIL ${request.url()}`);
});

await page.goto(URL, { waitUntil: 'domcontentloaded' });
await sleep(1200);
await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 20000 });

const introState = await page.evaluate(() => {
  const marker = document.querySelector('#cairn-nav');
  return {
    exists: !!marker,
    hidden: !marker || marker.classList.contains('hidden') || getComputedStyle(marker).display === 'none',
  };
});
ok('cairn navigation does not intrude before starter choice', introState.hidden, JSON.stringify(introState));

await page.click('.starter-card:nth-child(1) .sc-btn');
await page.waitForFunction(() => window.__game.mode === 'play', null, { timeout: 4000 });

const beaconStart = await page.evaluate(() => {
  const game = window.__game;
  const cairn = game.world?.props?.cairn;
  const beacon = game.world?.props?.cairnBeacon;
  if (!cairn || !beacon) return null;
  const cairnWorld = cairn.getWorldPosition(new game.player.pos.constructor());
  const beaconWorld = beacon.getWorldPosition(new game.player.pos.constructor());
  const subtree = [];
  beacon.traverse(object => {
    const materials = (Array.isArray(object.material) ? object.material : [object.material])
      .filter(Boolean)
      .map(material => [material.opacity ?? null, material.emissiveIntensity ?? null]);
    subtree.push([
      object.type,
      object.position.x, object.position.y, object.position.z,
      object.rotation.x, object.rotation.y, object.rotation.z,
      object.scale.x, object.scale.y, object.scale.z,
      object.intensity ?? null,
      materials,
    ]);
  });
  return {
    visible: beacon.visible,
    inScene: !!beacon.parent,
    cairn: [cairnWorld.x, cairnWorld.y, cairnWorld.z],
    beacon: [beaconWorld.x, beaconWorld.y, beaconWorld.z],
    subtree,
  };
});
ok('the existing cairn exposes a rendered beacon review handle', !!beaconStart && beaconStart.inScene && beaconStart.visible, JSON.stringify(beaconStart && { visible: beaconStart.visible, inScene: beaconStart.inScene }));
ok(
  'the beacon is centred above the existing cairn',
  !!beaconStart
    && Math.hypot(beaconStart.beacon[0] - beaconStart.cairn[0], beaconStart.beacon[2] - beaconStart.cairn[2]) < 0.5
    && beaconStart.beacon[1] > beaconStart.cairn[1] + 1,
  beaconStart ? `cairn=${beaconStart.cairn.join(',')} beacon=${beaconStart.beacon.join(',')}` : '',
);

await sleep(900);
const beaconLater = await page.evaluate(() => {
  const beacon = window.__game.world.props.cairnBeacon;
  if (!beacon) return null;
  const subtree = [];
  beacon.traverse(object => {
    const materials = (Array.isArray(object.material) ? object.material : [object.material])
      .filter(Boolean)
      .map(material => [material.opacity ?? null, material.emissiveIntensity ?? null]);
    subtree.push([
      object.type,
      object.position.x, object.position.y, object.position.z,
      object.rotation.x, object.rotation.y, object.rotation.z,
      object.scale.x, object.scale.y, object.scale.z,
      object.intensity ?? null,
      materials,
    ]);
  });
  return subtree;
});
const beaconMotion = beaconStart && beaconLater
  && JSON.stringify(beaconStart.subtree) !== JSON.stringify(beaconLater);
ok('the rendered beacon subtree is softly animated', !!beaconMotion, `objects=${beaconLater?.length ?? 0}`);

async function markerAt(x, z) {
  await page.evaluate(({ x, z }) => {
    const game = window.__game;
    game.player.pos.x = x;
    game.player.pos.z = z;
    game.player.mesh.position.x = x;
    game.player.mesh.position.z = z;
  }, { x, z });
  await sleep(300);
  return page.evaluate(() => {
    const marker = document.querySelector('#cairn-nav');
    if (!marker) return null;
    return {
      text: (marker.getAttribute('aria-label') || marker.textContent || '').replace(/\s+/g, ' ').trim(),
      visible: !marker.classList.contains('hidden') && getComputedStyle(marker).display !== 'none',
    };
  });
}

const north = await markerAt(18, 26);
ok('the exploration HUD exposes a visible accessible Research Cairn marker', !!north && north.visible && /research cairn/i.test(north.text), north?.text || 'missing');
ok('the marker reports 20 metres north using negative-Z north', !!north && /20\s*m/i.test(north.text) && /(?:^|\s)N(?:\s|$)/.test(north.text), north?.text || 'missing');
const northEast = await markerAt(-2, 26);
ok('the marker updates to roughly 28 metres north-east', !!northEast && /28\s*m/i.test(northEast.text) && /(?:^|\s)NE(?:\s|$)/.test(northEast.text), northEast?.text || 'missing');
const west = await markerAt(38, 6);
ok('the marker updates to 20 metres west', !!west && /20\s*m/i.test(west.text) && /(?:^|\s)W(?:\s|$)/.test(west.text), west?.text || 'missing');
const arrived = await markerAt(18, 6);
ok(
  'the marker handles arrival at the cairn',
  !!arrived && (/arrived/i.test(arrived.text) || /(?:^|\s)0\s*m(?:\s|$)/i.test(arrived.text)),
  arrived?.text || 'missing',
);
ok('the marker stays inside the existing exploration HUD', await page.evaluate(() => {
  const marker = document.querySelector('#cairn-nav');
  return !!marker && !!marker.closest('#hud');
}));
ok('no console, page or request errors', errors.length === 0, errors.slice(0, 3).join(' | '));

await browser.close();
console.log(`\nerrors observed: ${errors.length}`);
process.exit(process.exitCode || 0);
