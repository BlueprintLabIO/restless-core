import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const URL = 'http://127.0.0.1:8135/index.html';
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

async function arrangeGate(teamSize) {
  return page.evaluate((wantedTeamSize) => {
    const game = window.__game;
    for (const creature of game.creatures) {
      creature.captured = true;
      creature.mesh.visible = false;
      if (creature.blob) creature.blob.visible = false;
    }
    while (game.team.length < wantedTeamSize) {
      const speciesId = game.team.some(member => member.speciesId === 'ripplo') ? 'voltyx' : 'ripplo';
      game.team.push(game._makeTeamMember(speciesId, 5, 2));
    }
    game.hud.setTeam(game.team, game.activeIndex);
    game.player.pos.set(0, 0, -78);
    game.player.pos.y = game.world?.terrainHeight?.(0, -78) || 0;
    return game.team.length;
  }, teamSize);
}

await arrangeGate(1);
await sleep(500);
const gatedText = await page.locator('#hud').innerText();
ok(
  'the cavern gate explains the two-member eligibility gate',
  /prism|warden|cavern|gate/i.test(gatedText)
    && /bond|second|two|team/i.test(gatedText)
    && !/battle prism warden/i.test(gatedText),
  gatedText.replace(/\s+/g, ' ').slice(0, 260),
);

const eligibleSize = await arrangeGate(2);
await sleep(700);
const gateText = await page.locator('#hud').innerText();
ok('the arranged team has two living members', eligibleSize >= 2, String(eligibleSize));
ok('the gate visibly names the Prism Warden', /prism warden/i.test(gateText), gateText.replace(/\s+/g, ' ').slice(0, 260));
ok('the nearby gate prompt offers battle with F', /\bF\b[\s\S]{0,40}(battle|challenge)|(?:battle|challenge)[\s\S]{0,40}\bF\b/i.test(gateText), gateText.replace(/\s+/g, ' ').slice(0, 260));

await page.keyboard.press('KeyF');
await sleep(900);
const started = await page.evaluate(() => window.__game.mode === 'battle' && !!window.__game.battle);
ok('F starts the authored gate battle rather than a wild encounter', started);

let battleText = '';
if (started) battleText = await page.locator('#battle-hud').innerText();
ok('the native battle identifies Prism Warden', started && /prism warden/i.test(battleText), battleText.replace(/\s+/g, ' ').slice(0, 260));
ok('the boss battle visibly begins in Phase 1', started && /phase\s*1/i.test(battleText), battleText.replace(/\s+/g, ' ').slice(0, 260));
ok(
  'normal battle controls remain discoverable',
  started && /WASD/i.test(battleText) && /dodge/i.test(battleText) && /guard/i.test(battleText) && /switch/i.test(battleText),
  battleText.replace(/\s+/g, ' ').slice(-260),
);

let phaseState = { hp: 0, maxHp: 0 };
if (started) {
  phaseState = await page.evaluate(() => {
    const battle = window.__game.battle;
    const active = battle.active();
    active.maxHp = 1000;
    active.hp = 1000;
    active.invuln = 0;
    active.dodgeTime = 0;
    active.statuses = {};
    battle.enemy.ai.state = 'recover';
    battle.enemy.ai.timer = 999;
    battle.enemy.hp = battle.enemy.maxHp * 0.49;
    battle.update(0.02);
    return { hp: active.hp, maxHp: active.maxHp };
  });
  await sleep(250);
}
const phaseTwoText = started ? await page.locator('#battle-hud').innerText() : '';
ok(
  'crossing half health visibly enters Phase 2: Prismatic Surge',
  started && /phase\s*2/i.test(phaseTwoText) && /prismatic surge/i.test(phaseTwoText),
  phaseTwoText.replace(/\s+/g, ' ').slice(0, 320),
);

let pulseSeenAt = null;
let pulseHp = null;
let pulseDamageAt = null;
let pulseEndHp = phaseState.hp;
if (started) {
  for (let sample = 0; sample < 140; sample += 1) {
    const observed = await page.evaluate(() => {
      const battle = window.__game.battle;
      return {
        activeHp: battle?.active()?.hp ?? null,
        log: document.querySelector('#b-log')?.textContent || '',
      };
    });
    if (pulseSeenAt === null && /prism pulse/i.test(observed.log)) {
      pulseSeenAt = Date.now();
      pulseHp = observed.activeHp;
    }
    if (pulseSeenAt !== null && observed.activeHp < pulseHp - 0.001) {
      pulseDamageAt = Date.now();
      pulseEndHp = observed.activeHp;
      break;
    }
    await sleep(100);
  }
}
ok('Phase 2 visibly names the Prism Pulse telegraph', pulseSeenAt !== null);
const telegraphMs = pulseSeenAt !== null && pulseDamageAt !== null ? pulseDamageAt - pulseSeenAt : -1;
ok('Prism Pulse telegraphs before landing', telegraphMs >= 575, `${telegraphMs}ms`);
ok('Prism Pulse damages a vulnerable active creature', pulseDamageAt !== null && pulseEndHp < phaseState.hp, `${phaseState.hp}->${pulseEndHp}`);

let noRepeat = false;
if (started && pulseDamageAt !== null) {
  const afterPulse = await page.evaluate(() => {
    const battle = window.__game.battle;
    const active = battle.active();
    active.statuses = {};
    battle.enemy.ai.state = 'recover';
    battle.enemy.ai.timer = 999;
    battle.enemy.pos.set(100, battle.enemy.pos.y, 100);
    return active.hp;
  });
  await sleep(450);
  const laterHp = await page.evaluate(() => window.__game.battle?.active()?.hp ?? -1);
  noRepeat = Math.abs(laterHp - afterPulse) < 0.001;
}
ok('the transition pulse lands only once', noRepeat);

let bondBlocked = false;
let bondFeedback = '';
if (started) {
  await page.evaluate(() => {
    const battle = window.__game.battle;
    battle.enemy.hp = Math.min(battle.enemy.hp, battle.enemy.maxHp * 0.25);
    battle.update(0.02);
  });
  await page.keyboard.press('KeyB');
  await sleep(150);
  const observed = await page.evaluate(() => ({
    stillBattle: window.__game.mode === 'battle' && !!window.__game.battle && !window.__game.battle.done,
    log: document.querySelector('#b-log')?.textContent || '',
  }));
  bondBlocked = observed.stillBattle;
  bondFeedback = observed.log;
}
ok('B cannot replace or resolve the boss encounter', bondBlocked);
ok('the rejected bond action explains the authored-boss rule', /cannot|can.?t|warden|boss|bound/i.test(bondFeedback), bondFeedback);

let victory = null;
if (started && bondBlocked) {
  victory = await page.evaluate(() => {
    const game = window.__game;
    const battle = game.battle;
    const before = game.team.map(member => ({
      bond: member.bond || 1, xp: member.xp || 0, level: member.level || 1,
    }));
    battle.enemy.invuln = 0;
    battle.enemy.dodgeTime = 0;
    battle.enemy.hp = 1;
    battle._dealDamage(battle.active(), battle.enemy, {
      name: 'Evaluator Finish', kind: 'melee', element: 'Flame', power: 999999,
    });
    return { before };
  });
  await sleep(500);
  victory = await page.evaluate((state) => {
    const game = window.__game;
    return {
      ...state,
      mode: game.mode,
      flag: game.flags.prismUnlocked,
      after: game.team.map(member => ({
        bond: member.bond || 1, xp: member.xp || 0, level: member.level || 1,
      })),
      hud: document.querySelector('#hud')?.textContent || '',
    };
  }, victory);
}
ok('defeat returns cleanly to the basin and unlocks progression', victory?.mode === 'play' && victory?.flag === true, JSON.stringify(victory));
const singleReward = !!victory && victory.after.every((member, index) =>
  member.bond === Math.min(5, victory.before[index].bond + 1)
    && (member.level > victory.before[index].level || member.xp > victory.before[index].xp)
);
ok('existing victory XP and bond growth are granted exactly once', singleReward, JSON.stringify(victory));
ok('the owner-facing HUD announces the unlocked cavern signal', /cavern|unlock|prism/i.test(victory?.hud || ''), (victory?.hud || '').replace(/\s+/g, ' ').slice(0, 300));

await arrangeGate(2);
await sleep(500);
const postVictoryGate = await page.locator('#hud').innerText();
ok(
  'the defeated gate encounter is non-repeatable for the session',
  !!victory && victory.flag === true && !/battle prism warden|challenge prism warden/i.test(postVictoryGate),
  postVictoryGate.replace(/\s+/g, ' ').slice(0, 260),
);

const wildReady = await page.evaluate(() => {
  const game = window.__game;
  const wild = game.creatures[0];
  if (!wild) return false;
  wild.captured = false;
  wild.mesh.visible = true;
  if (wild.blob) wild.blob.visible = true;
  game.player.pos.x = wild.pos.x + 2;
  game.player.pos.z = wild.pos.z + 2;
  return true;
});
ok('a wild creature remains available after boss resolution', wildReady);
await page.keyboard.press('KeyF');
await sleep(500);
const wildBattle = await page.evaluate(() => ({
  mode: window.__game.mode,
  hasBattle: !!window.__game.battle,
  text: document.querySelector('#battle-hud')?.textContent || '',
}));
ok('ordinary wild battle still starts after the boss is gone', wildBattle.mode === 'battle' && wildBattle.hasBattle && !/prism warden/i.test(wildBattle.text), wildBattle.text.replace(/\s+/g, ' ').slice(0, 260));
if (wildBattle.hasBattle) await page.keyboard.press('Escape');
await sleep(200);
ok('ordinary wild battle still exits cleanly', await page.evaluate(() => window.__game.mode === 'play'));
ok('no console, page or request errors', errors.length === 0, errors.slice(0, 3).join(' | '));

await browser.close();
console.log(`\nerrors observed: ${errors.length}`);
process.exit(process.exitCode || 0);
