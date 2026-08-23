import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const URL = 'http://127.0.0.1:8133/index.html';
const WEBGL_ARGS = [
  '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--ignore-gpu-blocklist', '--no-sandbox', '--enable-webgl', '--disable-setuid-sandbox',
];
const errors = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const TESTS = [
  'unchanged starter experience loads',
  'Prism exposes a serialisable native snapshot',
  'Basin exposes a visible discoverable entrance',
  'ordinary R interaction enters the cavern',
  'cavern separates Basin-only actors',
  'cavern has a bounded room and differentiated prism roles',
  'initial cavern objective explains the Volt bridge gate',
  'non-Volt interaction cannot power the bridge',
  'Volt interaction powers and opens the bridge',
  'powered meaning persists in text and objective advances',
  'authored Nullix exists beyond the gate',
  'authored Nullix uses the existing battle path',
  'battle resolution returns to the powered cavern',
  'return portal restores the Basin loop',
  're-entry preserves bridge completion for the page session',
  'cavern guidance remains inside a phone viewport',
  'reload restores unchanged intro and fresh expedition state',
  'live journey produced no browser errors',
];
let step = 0;

function ok(name, condition, detail = '') {
  step += 1;
  const tag = condition ? 'PASS' : 'FAIL';
  if (!condition) process.exitCode = 1;
  console.log(`[${tag}] #${step} ${name}${detail ? ` :: ${detail}` : ''}`);
}

function watch(page) {
  page.on('console', message => {
    if (message.type() === 'error' && !/favicon|Failed to load resource/i.test(message.text())) {
      errors.push(`console:${message.text()}`);
    }
  });
  page.on('pageerror', error => errors.push(`pageerror:${String(error)}`));
  page.on('requestfailed', request => {
    if (!/favicon/i.test(request.url())) errors.push(`request:${request.url()}`);
  });
}

async function snapshot(page) {
  return page.evaluate(() => window.__cosmon?.prismSnapshot?.() || null);
}

async function positionAt(page, target) {
  await page.evaluate(where => {
    const game = window.__game;
    const exposed = window.__cosmon?.prismSnapshot?.()?.positions?.[where];
    const point = exposed || (where === 'entrance'
      ? game?.prism?.entrancePosition || game?.prismEntrancePosition || { x: 0, z: -82 }
      : where === 'console'
        ? game?.prism?.consolePosition || game?.prismConsolePosition
        : where === 'encounter'
          ? game?.prism?.encounterPosition || game?.prismEncounterPosition
          : game?.prism?.returnPosition || game?.prismReturnPosition);
    if (!game?.player || !point) throw new Error(`runtime position absent: ${where}`);
    game.player.pos.x = Number(point.x);
    game.player.pos.z = Number(point.z);
    game.player.mesh.position.x = Number(point.x);
    game.player.mesh.position.z = Number(point.z);
  }, target);
  await sleep(120);
}

async function press(page, key) {
  await page.keyboard.press(key);
  await sleep(160);
}

const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: WEBGL_ARGS });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
watch(page);

try {
  await page.goto(URL, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 20000 });
  await sleep(350);
  const intro = await page.evaluate(() => ({
    mode: window.__game.mode,
    starters: document.querySelectorAll('.starter-card').length,
    canvas: !!document.querySelector('canvas'),
  }));
  ok('unchanged starter experience loads', intro.mode === 'intro' && intro.starters === 3 && intro.canvas,
    JSON.stringify(intro));

  await page.locator('.starter-card').first().click();
  await sleep(250);
  let state = await snapshot(page);
  ok('Prism exposes a serialisable native snapshot', !!state && state.biome === 'basin', JSON.stringify(state));
  ok('Basin exposes a visible discoverable entrance', state?.entrance?.present && state?.entrance?.visible &&
    /prism/i.test(`${state.objective} ${state.status}`), JSON.stringify(state));

  await positionAt(page, 'entrance');
  await press(page, 'r');
  state = await snapshot(page);
  ok('ordinary R interaction enters the cavern', state?.biome === 'cavern' && state?.room?.visible,
    JSON.stringify(state));
  ok('cavern separates Basin-only actors', state?.basinActorsVisible === false, JSON.stringify(state));
  ok('cavern has a bounded room and differentiated prism roles',
    Number(state?.room?.enclosingForms) >= 4 && Number(state?.room?.crystals) >= 6 &&
    Number(state?.room?.materialRoles) >= 3 && state?.entrance?.present && state?.returnPortal?.present,
    JSON.stringify(state?.room));
  ok('initial cavern objective explains the Volt bridge gate', /volt/i.test(state?.objective || '') &&
    /(bridge|cross|path)/i.test(state?.objective || '') && !state?.bridge?.powered,
    `${state?.objective} | ${state?.status}`);

  await positionAt(page, 'console');
  await press(page, 'r');
  state = await snapshot(page);
  ok('non-Volt interaction cannot power the bridge', !state?.bridge?.powered &&
    /volt/i.test(`${state?.objective} ${state?.status}`), `${state?.objective} | ${state?.status}`);

  await page.evaluate(async () => {
    const game = window.__game;
    const { SPECIES } = await import('./js/creatures.js');
    const index = game.team.findIndex(member => SPECIES[member.speciesId]?.element === 'Volt');
    if (index >= 0) game._switchActive(index);
    else {
      game.team.push(game._makeTeamMember('voltyx', 5, 3));
      game._switchActive(game.team.length - 1);
    }
  });
  await press(page, 'r');
  state = await snapshot(page);
  ok('Volt interaction powers and opens the bridge', state?.bridge?.powered && state?.bridge?.traversable,
    JSON.stringify(state?.bridge));
  ok('powered meaning persists in text and objective advances', /(powered|stable|online|restored)/i.test(
    `${state?.bridge?.text || ''} ${state?.status || ''}`) && /(nullix|encounter|signal|beyond)/i.test(state?.objective || ''),
    `${state?.objective} | ${state?.status} | ${state?.bridge?.text}`);
  ok('authored Nullix exists beyond the gate', state?.encounter?.speciesId === 'nullix' &&
    state?.encounter?.authored === true && state?.encounter?.beyondGate === true,
    JSON.stringify(state?.encounter));

  await positionAt(page, 'encounter');
  await press(page, 'f');
  const battle = await page.evaluate(() => ({
    mode: window.__game.mode,
    enemy: window.__game.battle?.enemy?.species?.id || window.__game.battleEntity?.speciesId || '',
    battleHud: !!document.querySelector('#battle-hud'),
  }));
  ok('authored Nullix uses the existing battle path', battle.mode === 'battle' && battle.enemy === 'nullix' && battle.battleHud,
    JSON.stringify(battle));
  await press(page, 'Escape');
  state = await snapshot(page);
  ok('battle resolution returns to the powered cavern', state?.biome === 'cavern' && state?.bridge?.powered,
    JSON.stringify(state));

  await positionAt(page, 'return');
  await press(page, 'r');
  state = await snapshot(page);
  ok('return portal restores the Basin loop', state?.biome === 'basin' && state?.basinActorsVisible === true &&
    /basin|survey|explore/i.test(state?.objective || ''), JSON.stringify(state));

  await positionAt(page, 'entrance');
  await press(page, 'r');
  state = await snapshot(page);
  ok('re-entry preserves bridge completion for the page session', state?.biome === 'cavern' &&
    state?.bridge?.powered && state?.bridge?.traversable, JSON.stringify(state));

  await page.setViewportSize({ width: 390, height: 844 });
  await sleep(180);
  const mobile = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    objective: document.querySelector('#objective')?.getBoundingClientRect().toJSON(),
    status: document.querySelector('#status')?.getBoundingClientRect().toJSON(),
    prompt: document.querySelector('#prompt:not(.hidden)')?.getBoundingClientRect().toJSON() || null,
  }));
  const inside = box => !box || (box.left >= -2 && box.right <= mobile.innerWidth + 2 && box.top >= -2 && box.bottom <= 844 + 2);
  ok('cavern guidance remains inside a phone viewport', mobile.scrollWidth <= mobile.innerWidth + 2 &&
    inside(mobile.objective) && inside(mobile.status) && inside(mobile.prompt), JSON.stringify(mobile));

  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 10000 });
  const reset = await page.evaluate(() => ({ mode: window.__game.mode, biome: window.__game.biome,
    starters: document.querySelectorAll('.starter-card').length }));
  ok('reload restores unchanged intro and fresh expedition state', reset.mode === 'intro' && reset.biome === 'basin' &&
    reset.starters === 3, JSON.stringify(reset));
} catch (error) {
  errors.push(`suite:${String(error)}`);
  for (const name of TESTS.slice(step, -1)) ok(name, false, `suite error: ${String(error)}`);
}

ok('live journey produced no browser errors', errors.length === 0, errors.join(' | '));
console.log(`errors observed: ${errors.length}`);
await browser.close();
