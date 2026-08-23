import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const URL = 'http://127.0.0.1:8133/index.html';
const WEBGL_ARGS = [
  '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--ignore-gpu-blocklist', '--no-sandbox', '--enable-webgl', '--disable-setuid-sandbox',
];
const EXPECTED_SPECIES = [
  'lumipod', 'brambell', 'voltyx', 'mistlyn', 'pebblet', 'cinderling',
  'ripplo', 'froslite', 'nullix', 'mendray', 'thornox', 'crystawk',
  'verdantree', 'infernox', 'pyrelisk', 'stormkin', 'tidalfin', 'zephyral',
].sort();
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const errors = [];
let step = 0;

function ok(name, condition, extra = '') {
  step += 1;
  const tag = condition ? 'PASS' : 'FAIL';
  if (!condition) process.exitCode = 1;
  console.log(`[${tag}] #${step} ${name}${extra ? ` :: ${extra}` : ''}`);
}

function sorted(values) { return [...values].sort(); }
function sameValues(a, b) { return JSON.stringify(sorted(a)) === JSON.stringify(sorted(b)); }
function sameOrder(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
function absolute(href) {
  try { return new URL(href, URL).href; } catch { return null; }
}

function watch(page, label) {
  page.on('console', message => {
    if (message.type() === 'error' && !/favicon|Failed to load resource/i.test(message.text())) {
      errors.push(`${label}:console:${message.text()}`);
    }
  });
  page.on('pageerror', error => errors.push(`${label}:pageerror:${String(error)}`));
  page.on('requestfailed', request => {
    if (!/favicon/i.test(request.url())) errors.push(`${label}:request:${request.url()}`);
  });
}

function fillSuiteRemainder(names, startedAt, error) {
  const produced = step - startedAt;
  for (const name of names.slice(produced)) ok(name, false, `suite error: ${String(error)}`);
}

async function setControl(page, selector, value) {
  return page.locator(selector).first().evaluate((element, next) => {
    element.value = String(next);
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
    return element.value;
  }, value);
}

async function selectSemantic(page, selector, wanted) {
  return page.locator(selector).first().evaluate((element, target) => {
    const option = [...element.options].find(item =>
      item.value.toLowerCase() === target.toLowerCase() || item.textContent.trim().toLowerCase() === target.toLowerCase());
    if (!option) throw new Error(`option ${target} absent`);
    element.value = option.value;
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
    return option.value;
  }, wanted);
}

async function resetAtlas(page) {
  await setControl(page, '[data-atlas-search]', '');
  await page.locator('[data-atlas-filter]').evaluateAll(elements => {
    for (const element of elements) {
      const option = [...element.options].find(item => item.value === '' || /^(all|any)$/i.test(item.value));
      if (option) element.value = option.value;
      element.dispatchEvent(new Event('input', { bubbles: true }));
      element.dispatchEvent(new Event('change', { bubbles: true }));
    }
  });
  await sleep(80);
}

async function speciesCards(page) {
  return page.locator('[data-atlas-card]').evaluateAll(elements =>
    elements.filter(element => getComputedStyle(element).display !== 'none' && !element.hidden)
      .map(element => element.getAttribute('data-atlas-card'))
      .filter(Boolean));
}

async function squadState(page) {
  return page.locator('[data-workshop-slot]').evaluateAll(elements => elements.map(element => ({
    speciesId: element.getAttribute('data-species-id') || element.dataset.speciesId || '',
    level: Number(element.querySelector('[data-workshop-level]')?.value),
    bond: Number(element.querySelector('[data-workshop-bond]')?.value),
    ult: Number(element.querySelector('[data-workshop-ult]')?.value),
  })));
}

async function commonToolChecks(page, expectedTitle) {
  const common = await page.evaluate(() => {
    const root = document.querySelector('[data-tool-root]');
    const title = document.querySelector('[data-tool-title]');
    const back = document.querySelector('[data-return-to-survey]');
    return {
      root: !!root,
      title: title?.textContent?.trim() || '',
      heading: !!document.querySelector('h1'),
      backHref: back?.getAttribute('href') || '',
      backName: back?.textContent?.trim() || back?.getAttribute('aria-label') || '',
    };
  });
  ok(`${expectedTitle} exposes a native semantic root and title`, common.root && common.heading &&
    common.title.toLowerCase().includes(expectedTitle.toLowerCase()), JSON.stringify(common));
  ok(`${expectedTitle} exposes a named return to the survey`, !!absolute(common.backHref) &&
    common.backName.length > 2, JSON.stringify(common));
}

async function labelledControls(page) {
  return page.locator('button,input,select,textarea,a[href]').evaluateAll(elements => elements.filter(element => {
    if (element.hidden || getComputedStyle(element).display === 'none') return false;
    const id = element.id;
    const explicit = id && document.querySelector(`label[for="${CSS.escape(id)}"]`);
    const nested = element.closest('label');
    const name = element.getAttribute('aria-label') || element.getAttribute('aria-labelledby') ||
      element.getAttribute('title') || element.textContent?.trim();
    return !explicit && !nested && !name;
  }).map(element => element.outerHTML.slice(0, 180)));
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  headless: true,
  args: WEBGL_ARGS,
});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
watch(page, 'main');

await page.goto(URL, { waitUntil: 'domcontentloaded' });
await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 20000 });
await sleep(400);

const seedState = await page.evaluate(async () => {
  const { SPECIES } = await import('./js/creatures.js');
  return {
    mode: window.__game?.mode,
    starterCards: document.querySelectorAll('.starter-card').length,
    species: Object.keys(SPECIES).sort(),
    canvas: !!document.querySelector('#game-root canvas'),
    links: [...document.querySelectorAll('[data-companion-tool]')].map(element => ({
      tool: element.getAttribute('data-companion-tool'),
      href: element.getAttribute('href'),
      name: element.textContent?.trim() || element.getAttribute('aria-label') || '',
    })),
  };
});
ok('the unchanged starter experience loads in intro mode', seedState.mode === 'intro' &&
  seedState.starterCards === 3 && seedState.canvas, JSON.stringify(seedState));
ok('the exact seed roster still contains 18 unique species', sameValues(seedState.species, EXPECTED_SPECIES),
  seedState.species.join(','));

const atlasLink = seedState.links.find(link => link.tool === 'atlas');
const workshopLink = seedState.links.find(link => link.tool === 'workshop');
ok('the starter experience links a clearly named Field Atlas', !!atlasLink?.href &&
  /atlas/i.test(atlasLink.name), JSON.stringify(atlasLink));
ok('the starter experience links a clearly named Squad Workshop', !!workshopLink?.href &&
  /(squad|workshop)/i.test(workshopLink.name), JSON.stringify(workshopLink));
ok('both companion links stay inside the reviewable product origin',
  !!absolute(atlasLink?.href) && !!absolute(workshopLink?.href) &&
  new URL(absolute(atlasLink.href)).origin === new URL(URL).origin &&
  new URL(absolute(workshopLink.href)).origin === new URL(URL).origin,
  JSON.stringify({ atlas: atlasLink?.href, workshop: workshopLink?.href }));

const atlasTests = [
  'Field Atlas exposes a native semantic root and title',
  'Field Atlas exposes a named return to the survey',
  'Atlas shows every exact species once',
  'Atlas result count matches the visible roster',
  'Atlas exposes all four combinable filters',
  'Atlas search finds an ability name, not only a species name',
  'Atlas combines search and element filtering',
  'Atlas tier filter identifies exactly six evolved forms',
  'Atlas presents a calm explicit no-result state',
  'Atlas detail shows the complete Cinderling identity',
  'Atlas detail shows all three Cinderling abilities',
  'Atlas shows both live branching evolution targets and requirements',
  'Atlas compares two distinct complete species',
  'Atlas derives both elemental matchup directions',
  'Atlas never creates a false same-species comparison',
  'Atlas controls have accessible names',
  'Atlas remains within a phone-width viewport',
  'Field Atlas returns through its native survey link',
];

if (!atlasLink?.href) {
  for (const name of atlasTests) ok(name, false, 'Atlas launch link absent');
} else {
  const atlasStartedAt = step;
  try {
  await page.goto(absolute(atlasLink.href), { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-tool-root]', { timeout: 10000 }).catch(() => {});
  await commonToolChecks(page, 'Field Atlas');

  const allCards = await speciesCards(page);
  ok('Atlas shows every exact species once', allCards.length === 18 &&
    new Set(allCards).size === 18 && sameValues(allCards, EXPECTED_SPECIES), allCards.join(','));
  const initialCount = await page.locator('[data-atlas-count]').first().textContent().catch(() => '');
  ok('Atlas result count matches the visible roster', /18/.test(initialCount || ''), initialCount || 'missing');
  const filterKinds = await page.locator('[data-atlas-filter]').evaluateAll(elements =>
    elements.map(element => element.getAttribute('data-atlas-filter')));
  ok('Atlas exposes all four combinable filters', sameValues(filterKinds,
    ['element', 'role', 'tier', 'temperament']), filterKinds.join(','));

  await setControl(page, '[data-atlas-search]', 'Flame Burst');
  await sleep(80);
  let filtered = await speciesCards(page);
  ok('Atlas search finds an ability name, not only a species name',
    filtered.length === 1 && filtered[0] === 'cinderling', filtered.join(','));

  await selectSemantic(page, '[data-atlas-filter="element"]', 'Tide');
  await sleep(80);
  filtered = await speciesCards(page);
  ok('Atlas combines search and element filtering', filtered.length === 0, filtered.join(','));

  await resetAtlas(page);
  await selectSemantic(page, '[data-atlas-filter="tier"]', 'evolved');
  await sleep(80);
  filtered = await speciesCards(page);
  ok('Atlas tier filter identifies exactly six evolved forms', filtered.length === 6 &&
    sameValues(filtered, ['verdantree', 'infernox', 'pyrelisk', 'stormkin', 'tidalfin', 'zephyral']),
    filtered.join(','));

  await resetAtlas(page);
  await setControl(page, '[data-atlas-search]', 'no-species-has-this-signal');
  await sleep(80);
  const noResult = await page.evaluate(() => ({
    cards: [...document.querySelectorAll('[data-atlas-card]')].filter(element =>
      getComputedStyle(element).display !== 'none' && !element.hidden).length,
    text: document.querySelector('[data-tool-root]')?.textContent || '',
    count: document.querySelector('[data-atlas-count]')?.textContent || '',
  }));
  ok('Atlas presents a calm explicit no-result state', noResult.cards === 0 && /0/.test(noResult.count) &&
    /(no result|none found|try another|no species)/i.test(noResult.text), JSON.stringify(noResult));

  await resetAtlas(page);
  await page.locator('[data-atlas-card="cinderling"]').first().click();
  await sleep(80);
  const detail = await page.evaluate(() => {
    const root = document.querySelector('[data-atlas-detail]');
    const fields = Object.fromEntries([...document.querySelectorAll('[data-atlas-detail-field]')]
      .map(element => [element.getAttribute('data-atlas-detail-field'), element.textContent?.trim() || '']));
    const abilities = [...document.querySelectorAll('[data-atlas-ability]')].map(element => element.textContent || '');
    const evolutions = [...document.querySelectorAll('[data-atlas-evolution-target]')].map(element => ({
      id: element.getAttribute('data-atlas-evolution-target'),
      text: element.textContent || '',
    }));
    return { exists: !!root, text: root?.textContent || '', fields, abilities, evolutions };
  });
  ok('Atlas detail shows the complete Cinderling identity', detail.exists &&
    ['name', 'element', 'role', 'temperament', 'tier', 'growth', 'blurb'].every(key => detail.fields[key]) &&
    /Cinderling/i.test(detail.fields.name) && /Flame/i.test(detail.fields.element) &&
    /Striker/i.test(detail.fields.role), JSON.stringify(detail.fields));
  ok('Atlas detail shows all three Cinderling abilities', detail.abilities.length === 3 &&
    ['Ember Strike', 'Flame Burst', 'Blaze Rush'].every(name => detail.abilities.some(text => text.includes(name))),
    detail.abilities.join(' | '));
  const evoText = detail.evolutions.map(item => `${item.id}:${item.text}`).join(' | ');
  ok('Atlas shows both live branching evolution targets and requirements',
    detail.evolutions.some(item => item.id === 'infernox') &&
    detail.evolutions.some(item => item.id === 'pyrelisk') && /7/.test(evoText) &&
    /bond/i.test(evoText) && /(6|signature|use)/i.test(evoText), evoText);

  await page.locator('[data-atlas-compare-add="cinderling"]').first().click();
  await page.locator('[data-atlas-compare-add="ripplo"]').first().click();
  await sleep(80);
  const comparison = await page.evaluate(() => ({
    a: document.querySelector('[data-atlas-compare-slot="a"]')?.textContent || '',
    b: document.querySelector('[data-atlas-compare-slot="b"]')?.textContent || '',
    ab: document.querySelector('[data-atlas-matchup="a-to-b"]')?.textContent || '',
    ba: document.querySelector('[data-atlas-matchup="b-to-a"]')?.textContent || '',
  }));
  ok('Atlas compares two distinct complete species', /Cinderling/i.test(comparison.a) &&
    /Ripplo/i.test(comparison.b) && /Ember Strike/i.test(comparison.a) && /Tide Bolt/i.test(comparison.b),
    JSON.stringify(comparison));
  ok('Atlas derives both elemental matchup directions', /0[.,]62|0\.62/.test(comparison.ab) &&
    /1[.,]6|1\.6/.test(comparison.ba), JSON.stringify(comparison));
  await page.locator('[data-atlas-compare-add="ripplo"]').first().click();
  await sleep(50);
  const compareIds = await page.locator('[data-atlas-compare-slot]').evaluateAll(elements =>
    elements.map(element => element.getAttribute('data-species-id') || element.dataset.speciesId || element.textContent));
  ok('Atlas never creates a false same-species comparison', compareIds.length === 2 &&
    compareIds[0] !== compareIds[1], compareIds.join(','));

  const atlasUnlabelled = await labelledControls(page);
  ok('Atlas controls have accessible names', atlasUnlabelled.length === 0, atlasUnlabelled.join(' | '));
  await page.setViewportSize({ width: 390, height: 844 });
  await sleep(80);
  const atlasMobile = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    rootWidth: document.querySelector('[data-tool-root]')?.getBoundingClientRect().width || 0,
  }));
  ok('Atlas remains within a phone-width viewport', atlasMobile.scrollWidth <= atlasMobile.innerWidth + 2 &&
    atlasMobile.rootWidth <= atlasMobile.innerWidth + 2, JSON.stringify(atlasMobile));
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.locator('[data-return-to-survey]').first().click();
  await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 10000 });
  ok('Field Atlas returns through its native survey link', page.url().startsWith(new URL(URL).origin) &&
    await page.locator('.starter-card').count() === 3, page.url());
  } catch (error) {
    fillSuiteRemainder(atlasTests, atlasStartedAt, error);
  }
}

const workshopTests = [
  'Squad Workshop exposes a native semantic root and title',
  'Squad Workshop exposes a named return to the survey',
  'Workshop offers every exact species once',
  'Workshop builds an ordered four-member squad',
  'Workshop rejects a duplicate atomically',
  'Workshop rejects a fifth member atomically',
  'Workshop reorders members explicitly',
  'Workshop forecasts the bond evolution from live values',
  'Workshop forecasts the signature-use evolution from live values',
  'Workshop exposes live roles, elements, strengths, risks and recommendations',
  'Workshop analysis changes with composition',
  'Workshop saves a named squad',
  'Workshop persists a named save across reload',
  'Workshop reloads exact order and per-slot values',
  'Workshop exports versioned exact JSON',
  'Workshop imports an exact valid exchange atomically',
  'Workshop rejects malformed JSON atomically',
  'Workshop rejects unknown species atomically',
  'Workshop rejects duplicate species atomically',
  'Workshop rejects out-of-range values atomically',
  'Workshop rejects more than four members atomically',
  'Workshop deletes a named save',
  'Workshop controls have accessible names',
  'Workshop remains within a phone-width viewport',
  'Squad Workshop returns through its native survey link',
];

if (!workshopLink?.href) {
  for (const name of workshopTests) ok(name, false, 'Workshop launch link absent');
} else {
  const workshopStartedAt = step;
  try {
  await page.goto(absolute(workshopLink.href), { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-tool-root]', { timeout: 10000 }).catch(() => {});
  await page.evaluate(() => {
    for (const key of Object.keys(localStorage)) {
      if (/(workshop|squad)/i.test(key)) localStorage.removeItem(key);
    }
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await commonToolChecks(page, 'Squad Workshop');

  const offered = await page.locator('[data-workshop-species]').evaluateAll(elements =>
    elements.map(element => element.getAttribute('data-workshop-species')).filter(Boolean));
  ok('Workshop offers every exact species once', offered.length === 18 && new Set(offered).size === 18 &&
    sameValues(offered, EXPECTED_SPECIES), offered.join(','));

  for (const id of ['cinderling', 'ripplo', 'voltyx', 'brambell']) {
    await page.locator(`[data-workshop-add="${id}"]`).first().click();
  }
  await sleep(80);
  let squad = await squadState(page);
  const teamCount = await page.locator('[data-workshop-team-count]').first().textContent().catch(() => '');
  ok('Workshop builds an ordered four-member squad', sameOrder(squad.map(item => item.speciesId),
    ['cinderling', 'ripplo', 'voltyx', 'brambell']) && /4/.test(teamCount || ''), JSON.stringify(squad));

  await page.locator('[data-workshop-add="cinderling"]').first().click();
  await sleep(40);
  let afterInvalid = await squadState(page);
  let workshopError = await page.locator('[data-workshop-error]').first().textContent().catch(() => '');
  ok('Workshop rejects a duplicate atomically', sameOrder(afterInvalid.map(item => item.speciesId),
    squad.map(item => item.speciesId)) && /(duplicate|already|distinct)/i.test(workshopError || ''), workshopError || 'missing');

  await page.locator('[data-workshop-add="pebblet"]').first().click();
  await sleep(40);
  afterInvalid = await squadState(page);
  workshopError = await page.locator('[data-workshop-error]').first().textContent().catch(() => '');
  ok('Workshop rejects a fifth member atomically', sameOrder(afterInvalid.map(item => item.speciesId),
    squad.map(item => item.speciesId)) && /(four|4|full|maximum)/i.test(workshopError || ''), workshopError || 'missing');

  await page.locator('[data-workshop-slot]').first().locator('[data-workshop-move="down"]').click();
  await sleep(40);
  squad = await squadState(page);
  ok('Workshop reorders members explicitly', sameOrder(squad.map(item => item.speciesId),
    ['ripplo', 'cinderling', 'voltyx', 'brambell']), JSON.stringify(squad));

  const cinderSlot = page.locator('[data-workshop-slot][data-species-id="cinderling"]');
  await setControl(page, '[data-workshop-slot][data-species-id="cinderling"] [data-workshop-level]', 7);
  await setControl(page, '[data-workshop-slot][data-species-id="cinderling"] [data-workshop-bond]', 4);
  await setControl(page, '[data-workshop-slot][data-species-id="cinderling"] [data-workshop-ult]', 0);
  await sleep(60);
  let forecast = await cinderSlot.locator('[data-workshop-forecast]').textContent().catch(() => '');
  ok('Workshop forecasts the bond evolution from live values', /Infernox/i.test(forecast || ''), forecast || 'missing');

  await setControl(page, '[data-workshop-slot][data-species-id="cinderling"] [data-workshop-bond]', 1);
  await setControl(page, '[data-workshop-slot][data-species-id="cinderling"] [data-workshop-ult]', 6);
  await sleep(60);
  forecast = await cinderSlot.locator('[data-workshop-forecast]').textContent().catch(() => '');
  ok('Workshop forecasts the signature-use evolution from live values', /Pyrelisk/i.test(forecast || ''), forecast || 'missing');

  const analysisBefore = await page.locator('[data-workshop-analysis]').evaluateAll(elements =>
    Object.fromEntries(elements.map(element => [element.getAttribute('data-workshop-analysis'), element.textContent?.trim() || ''])));
  ok('Workshop exposes live roles, elements, strengths, risks and recommendations',
    ['roles', 'elements', 'strengths', 'risks', 'recommendations'].every(key => analysisBefore[key]?.length > 2),
    JSON.stringify(analysisBefore));
  await page.locator('[data-workshop-slot]').last().locator('[data-workshop-remove]').click();
  await sleep(60);
  const analysisAfter = await page.locator('[data-workshop-analysis]').evaluateAll(elements =>
    Object.fromEntries(elements.map(element => [element.getAttribute('data-workshop-analysis'), element.textContent?.trim() || ''])));
  ok('Workshop analysis changes with composition', JSON.stringify(analysisBefore) !== JSON.stringify(analysisAfter),
    JSON.stringify({ before: analysisBefore, after: analysisAfter }));
  await page.locator('[data-workshop-add="brambell"]').first().click();

  await setControl(page, '[data-workshop-save-name]', 'Aurora Four');
  await page.locator('[data-workshop-save]').first().click();
  await sleep(60);
  let savedText = await page.locator('[data-workshop-saved]').allTextContents();
  ok('Workshop saves a named squad', savedText.some(text => /Aurora Four/i.test(text)), savedText.join(' | '));
  const expectedSaved = await squadState(page);

  await page.reload({ waitUntil: 'domcontentloaded' });
  await sleep(80);
  const savedEntry = page.locator('[data-workshop-saved]').filter({ hasText: 'Aurora Four' }).first();
  savedText = await page.locator('[data-workshop-saved]').allTextContents();
  ok('Workshop persists a named save across reload', savedText.some(text => /Aurora Four/i.test(text)), savedText.join(' | '));
  await savedEntry.locator('[data-workshop-load]').click();
  await sleep(60);
  squad = await squadState(page);
  ok('Workshop reloads exact order and per-slot values', JSON.stringify(squad) === JSON.stringify(expectedSaved),
    JSON.stringify({ expectedSaved, squad }));

  await page.locator('[data-workshop-export]').first().click();
  await sleep(50);
  const exportedText = await page.locator('[data-workshop-json]').first().inputValue().catch(async () =>
    page.locator('[data-workshop-json]').first().textContent());
  let exported = null;
  try { exported = JSON.parse(exportedText); } catch {}
  const exportedMembers = Array.isArray(exported?.members) ? exported.members : [];
  const exportedIds = exportedMembers.map(item => item.speciesId);
  ok('Workshop exports versioned exact JSON', exported?.version === 1 &&
    sameOrder(exportedIds, expectedSaved.map(item => item.speciesId)) &&
    exportedMembers.every((item, index) => Number(item.level) === expectedSaved[index].level &&
      Number(item.bond) === expectedSaved[index].bond && Number(item.ultUses) === expectedSaved[index].ult),
    exportedText || 'missing');

  while (await page.locator('[data-workshop-slot]').count()) {
    await page.locator('[data-workshop-slot]').first().locator('[data-workshop-remove]').click();
  }
  await setControl(page, '[data-workshop-json]', exportedText || '');
  await page.locator('[data-workshop-import]').first().click();
  await sleep(60);
  squad = await squadState(page);
  ok('Workshop imports an exact valid exchange atomically', JSON.stringify(squad) === JSON.stringify(expectedSaved),
    JSON.stringify(squad));

  async function invalidImport(name, value, errorPattern) {
    const before = await squadState(page);
    await setControl(page, '[data-workshop-json]', value);
    await page.locator('[data-workshop-import]').first().click();
    await sleep(40);
    const after = await squadState(page);
    const message = await page.locator('[data-workshop-error]').first().textContent().catch(() => '');
    ok(name, JSON.stringify(before) === JSON.stringify(after) && errorPattern.test(message || ''),
      JSON.stringify({ message, before, after }));
  }

  await invalidImport('Workshop rejects malformed JSON atomically', '{broken', /(invalid|json|parse)/i);
  if (exported?.version === 1 && Array.isArray(exported.members)) {
    const unknown = structuredClone(exported);
    unknown.members[0].speciesId = 'not-a-real-species';
    await invalidImport('Workshop rejects unknown species atomically', JSON.stringify(unknown), /(unknown|species|invalid)/i);

    const duplicate = structuredClone(exported);
    duplicate.members[1] = structuredClone(duplicate.members[0]);
    await invalidImport('Workshop rejects duplicate species atomically', JSON.stringify(duplicate), /(duplicate|distinct|already|invalid)/i);

    const range = structuredClone(exported);
    range.members[0].level = 99;
    await invalidImport('Workshop rejects out-of-range values atomically', JSON.stringify(range), /(range|level|invalid|1|12)/i);

    const tooMany = structuredClone(exported);
    tooMany.members.push({ ...structuredClone(tooMany.members[0]), speciesId: 'pebblet' });
    await invalidImport('Workshop rejects more than four members atomically', JSON.stringify(tooMany), /(four|4|maximum|invalid|member)/i);
  } else {
    for (const name of [
      'Workshop rejects unknown species atomically',
      'Workshop rejects duplicate species atomically',
      'Workshop rejects out-of-range values atomically',
      'Workshop rejects more than four members atomically',
    ]) ok(name, false, 'valid export schema unavailable');
  }

  await savedEntry.locator('[data-workshop-delete]').click().catch(async () => {
    await page.locator('[data-workshop-saved]').filter({ hasText: 'Aurora Four' }).first()
      .locator('[data-workshop-delete]').click();
  });
  await sleep(50);
  savedText = await page.locator('[data-workshop-saved]').allTextContents();
  ok('Workshop deletes a named save', !savedText.some(text => /Aurora Four/i.test(text)), savedText.join(' | '));

  const workshopUnlabelled = await labelledControls(page);
  ok('Workshop controls have accessible names', workshopUnlabelled.length === 0, workshopUnlabelled.join(' | '));
  await page.setViewportSize({ width: 390, height: 844 });
  await sleep(80);
  const workshopMobile = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    rootWidth: document.querySelector('[data-tool-root]')?.getBoundingClientRect().width || 0,
  }));
  ok('Workshop remains within a phone-width viewport', workshopMobile.scrollWidth <= workshopMobile.innerWidth + 2 &&
    workshopMobile.rootWidth <= workshopMobile.innerWidth + 2, JSON.stringify(workshopMobile));
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.locator('[data-return-to-survey]').first().click();
  await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 10000 });
  ok('Squad Workshop returns through its native survey link', page.url().startsWith(new URL(URL).origin) &&
    await page.locator('.starter-card').count() === 3, page.url());
  } catch (error) {
    fillSuiteRemainder(workshopTests, workshopStartedAt, error);
  }
}

const coreTests = [
  'the original matchup and branching-evolution rules remain executable',
  'the normal survey still starts after companion-tool use',
  'the original live battle still starts against a nearby wild creature',
  'the original battle still exits cleanly to exploration',
];
const coreStartedAt = step;
try {
  await page.goto(URL, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => window.__game && document.querySelector('#intro'), null, { timeout: 20000 });
  const pureRules = await page.evaluate(async () => {
    const { elementMult } = await import('./js/config.js');
    const { evolutionTarget } = await import('./js/evolution.js');
    return {
      tideVsFlame: elementMult('Tide', 'Flame'),
      flameVsTide: elementMult('Flame', 'Tide'),
      bond: evolutionTarget({ speciesId: 'cinderling', level: 7, bond: 4, ultUses: 0 }),
      signature: evolutionTarget({ speciesId: 'cinderling', level: 7, bond: 1, ultUses: 6 }),
    };
  });
  ok('the original matchup and branching-evolution rules remain executable',
    pureRules.tideVsFlame === 1.6 && pureRules.flameVsTide === 0.62 &&
    pureRules.bond === 'infernox' && pureRules.signature === 'pyrelisk', JSON.stringify(pureRules));

  await page.click('.starter-card:nth-child(1) .sc-btn');
  await page.waitForFunction(() => window.__game?.mode === 'play', null, { timeout: 4000 });
  const finalGame = await page.evaluate(() => ({
    mode: window.__game?.mode,
    team: window.__game?.team?.length,
    world: !!window.__game?.world,
    creatures: window.__game?.creatures?.length,
  }));
  ok('the normal survey still starts after companion-tool use', finalGame.mode === 'play' &&
    finalGame.team === 1 && finalGame.world && finalGame.creatures >= 5, JSON.stringify(finalGame));

  const prey = await page.evaluate(() => {
    const game = window.__game;
    const entity = game.creatures.find(item => !item.captured);
    if (!entity) return null;
    game.player.pos.x = entity.pos.x;
    game.player.pos.z = entity.pos.z - 3;
    game.player.mesh.position.x = game.player.pos.x;
    game.player.mesh.position.z = game.player.pos.z;
    return entity.speciesId;
  });
  await page.keyboard.press('KeyF');
  await page.waitForFunction(() => window.__game?.mode === 'battle' && window.__game?.battle, null, { timeout: 4000 });
  await page.waitForFunction(() => document.querySelectorAll('#b-bar .ab-slot').length === 4, null, { timeout: 4000 });
  const battle = await page.evaluate(() => ({
    mode: window.__game.mode,
    enemy: window.__game.battle?.enemy?.species?.id,
    players: window.__game.battle?.players?.length,
    abilitySlots: document.querySelectorAll('#b-bar .ab-slot').length,
  }));
  ok('the original live battle still starts against a nearby wild creature', !!prey && battle.mode === 'battle' &&
    battle.enemy === prey && battle.players >= 1 && battle.abilitySlots === 4, JSON.stringify({ prey, battle }));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => window.__game?.mode === 'play', null, { timeout: 4000 });
  ok('the original battle still exits cleanly to exploration',
    await page.evaluate(() => window.__game?.mode === 'play' && !window.__game?.battle));
} catch (error) {
  fillSuiteRemainder(coreTests, coreStartedAt, error);
}
ok('the complete native review produced zero browser/runtime errors', errors.length === 0, errors.join(' | '));

await browser.close();
