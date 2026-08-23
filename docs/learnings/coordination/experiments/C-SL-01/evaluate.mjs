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
await page.click('.starter-card:nth-child(1) .sc-btn');
await page.waitForFunction(() => window.__game.mode === 'play', null, { timeout: 4000 });

const positioned = await page.evaluate(() => {
  const game = window.__game;
  const wild = game.creatures.find(creature => !creature.captured);
  if (!wild) return false;
  game.player.pos.x = wild.pos.x + 2;
  game.player.pos.z = wild.pos.z + 2;
  return true;
});
ok('a wild creature is available for the native battle', positioned);
await page.keyboard.press('KeyF');
await page.waitForFunction(() => window.__game.mode === 'battle' && window.__game.battle, null, { timeout: 4000 });

const help = await page.locator('#b-help').innerText();
ok('battle help makes late or perfect guarding discoverable', /perfect|late|tim/i.test(help), help);

async function arrangeWindup(seconds, energy = 20) {
  return page.evaluate(({ seconds, energy }) => {
    const battle = window.__game.battle;
    const active = battle.active();
    const enemy = battle.enemy;
    active.hp = active.maxHp;
    active.energy = energy;
    active.invuln = 0;
    active.dodgeTime = 0;
    active.dead = false;
    active.popIn = 1;
    active.statuses = {};
    active.guarding = false;
    battle.keys.ShiftLeft = false;
    battle.keys.ShiftRight = false;
    battle.rng = () => 0.5;
    enemy.popIn = 1;
    enemy.energy = 100;
    enemy.pos.copy(active.pos);
    enemy.pos.z += 1.5;
    enemy.mesh.position.copy(enemy.pos);
    const ability = {
      name: 'Evaluator Strike', kind: 'melee', element: enemy.species.element,
      power: 30, range: 3, arc: 1.4,
    };
    enemy.ai.state = 'windup';
    enemy.ai.chosen = ability;
    enemy.ai.windup = seconds;
    battle._clearTelegraph();
    battle._beginTelegraph(enemy, ability, 0.45);
    return { hp: active.hp, energy: active.energy };
  }, { seconds, energy });
}

// Holding before the 180 ms window must stay an ordinary guard.
const earlyStart = await arrangeWindup(0.45);
await page.keyboard.down('ShiftLeft');
await page.evaluate(() => window.__game.battle.update(0.25));
await page.evaluate(() => window.__game.battle.update(0.21));
const early = await page.evaluate(() => {
  const battle = window.__game.battle;
  const active = battle.active();
  return { hp: active.hp, energy: active.energy, log: battle._logLine || '' };
});
await page.keyboard.up('ShiftLeft');
ok('an early held guard still takes reduced non-zero damage', early.hp < earlyStart.hp && early.hp > 0, `${earlyStart.hp}->${early.hp}`);
ok('ordinary guard grants no 18-energy Perfect Guard reward', early.energy - earlyStart.energy < 18, `${earlyStart.energy}->${early.energy}`);

const naturalLateEnergyGain = await page.evaluate(() => {
  const battle = window.__game.battle;
  const active = battle.active();
  active.energy = 20;
  battle.keys.ShiftLeft = false;
  battle.keys.ShiftRight = false;
  battle.enemy.ai.state = 'recover';
  battle.enemy.ai.timer = 10;
  const before = active.energy;
  battle.update(0.15);
  return active.energy - before;
});

// A fresh Shift press inside the final 180 ms must negate exactly the telegraphed hit.
const lateStart = await arrangeWindup(0.14);
await page.keyboard.down('ShiftRight');
await page.evaluate(() => window.__game.battle.update(0.15));
const late = await page.evaluate(() => {
  const battle = window.__game.battle;
  const active = battle.active();
  return { hp: active.hp, energy: active.energy, log: battle._logLine || '' };
});
ok('late fresh guard negates the telegraphed hit', late.hp === lateStart.hp, `${lateStart.hp}->${late.hp}`);
ok(
  'Perfect Guard grants exactly 18 energy beyond ordinary regeneration',
  Math.abs((late.energy - lateStart.energy - naturalLateEnergyGain) - 18) < 0.01,
  `${lateStart.energy}->${late.energy}; natural=${naturalLateEnergyGain}`,
);
ok('Perfect Guard produces visible battle feedback', /perfect guard/i.test(late.log), late.log);

// The reward cannot leak to a second unrelated hit while Shift remains held.
const second = await page.evaluate(() => {
  const battle = window.__game.battle;
  const active = battle.active();
  const enemy = battle.enemy;
  const before = { hp: active.hp, energy: active.energy };
  battle._dealDamage(enemy, active, {
    name: 'Follow-up', kind: 'melee', element: enemy.species.element, power: 30,
  });
  return { before, hp: active.hp, energy: active.energy, log: battle._logLine || '' };
});
await page.keyboard.up('ShiftRight');
ok('Perfect Guard does not leak to an unrelated later hit', second.hp < second.before.hp, `${second.before.hp}->${second.hp}`);
ok('one telegraph grants its energy reward at most once', second.energy === second.before.energy, `${second.before.energy}->${second.energy}`);

await page.keyboard.press('Escape');
await page.waitForFunction(() => window.__game.mode === 'play', null, { timeout: 4000 });
ok('battle still exits cleanly to the overworld', await page.evaluate(() => window.__game.mode === 'play'));
ok('no console, page or request errors', errors.length === 0, errors.slice(0, 3).join(' | '));

await browser.close();
console.log(`\nerrors observed: ${errors.length}`);
process.exit(process.exitCode || 0);
