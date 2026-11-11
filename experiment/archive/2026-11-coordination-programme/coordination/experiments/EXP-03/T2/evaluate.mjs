import fs from 'node:fs';

let step = 0;
let failed = false;
function check(name, condition, detail = '') {
  step += 1;
  const pass = Boolean(condition);
  if (!pass) failed = true;
  console.log(`[${pass ? 'PASS' : 'FAIL'}] #${step} ${name}${detail ? ` :: ${detail}` : ''}`);
}

const required = [
  'marketing/manifest.json',
  'marketing/strategy.md',
  'marketing/claims-register.md',
  'marketing/assets/steam-page.md',
  'marketing/assets/vertical-video.md',
  'marketing/measurement.md',
  'marketing/review.html',
  'verify-marketing.mjs',
];
check('all required files exist', required.every(path => fs.existsSync(path)));

let manifest = {};
let validJson = false;
try {
  manifest = JSON.parse(fs.readFileSync('marketing/manifest.json', 'utf8'));
  validJson = true;
} catch {
  // Fixed-count failure is emitted below.
}
check('manifest is valid JSON', validJson);
check('exact schema and fictional company identity', manifest.schema_version === 'exp03-t2-v1' && manifest.company?.id === 'cosmon_test' && manifest.company?.fictional === true);
check('campaign carries the exact single CTA', manifest.campaign?.cta === 'Join the closed desktop-browser playtest waitlist.' && ['goal', 'audience', 'offer'].every(key => String(manifest.campaign?.[key] || '').length >= 20));

const evidenceIds = new Set([
  ...Array.from({length: 8}, (_, i) => `P-0${i + 1}`),
  ...Array.from({length: 7}, (_, i) => `I-0${i + 1}`),
]);
const claims = Array.isArray(manifest.claims) ? manifest.claims : [];
const claimIds = claims.map(claim => claim.id);
check('claims are unique, supported and source-bound', claims.length >= 6 && new Set(claimIds).size === claims.length && claims.every(claim => typeof claim.id === 'string' && String(claim.text || '').length >= 12 && claim.status === 'supported' && Array.isArray(claim.evidence_ids) && claim.evidence_ids.length > 0 && claim.evidence_ids.every(id => evidenceIds.has(id))));

const prohibited = JSON.stringify(manifest.prohibited_claims || []).toLowerCase();
check('prohibited register covers every supplied boundary', ['multiplayer', 'planet', 'biome', '60', 'release', 'performance', 'conversion'].every(term => prohibited.includes(term)));

const assets = Array.isArray(manifest.assets) ? manifest.assets : [];
const expectedPaths = ['marketing/assets/steam-page.md', 'marketing/assets/vertical-video.md'];
check('exactly the two frozen channel assets are registered', assets.length === 2 && assets.map(asset => asset.path).sort().join('|') === expectedPaths.sort().join('|') && new Set(assets.map(asset => asset.id)).size === 2);
check('every asset uses the exact CTA and resolves supported claims', assets.length === 2 && assets.every(asset => asset.cta === manifest.campaign?.cta && Array.isArray(asset.claim_ids) && asset.claim_ids.length > 0 && asset.claim_ids.every(id => claimIds.includes(id))));

const read = path => fs.existsSync(path) ? fs.readFileSync(path, 'utf8') : '';
const strategy = read('marketing/strategy.md');
const register = read('marketing/claims-register.md');
const steam = read('marketing/assets/steam-page.md');
const video = read('marketing/assets/vertical-video.md');
const measurement = read('marketing/measurement.md');
const review = read('marketing/review.html');
const publicCopy = `${steam}\n${video}`;

check('strategy is substantive and connects evidence, channels, risks and non-goals', strategy.length >= 1400 && /position/i.test(strategy) && /I-0[1-7]/.test(strategy) && /sequence/i.test(strategy) && /risk/i.test(strategy) && /non-goal/i.test(strategy));
check('claims register separates product facts from research signals', register.length >= 900 && /P-0[1-8]/.test(register) && /I-0[1-7]/.test(register) && /prohibit|unsupported/i.test(register) && /signal|interview/i.test(register));
check('Steam asset is channel-native closed-playtest copy', steam.length >= 700 && /headline/i.test(steam) && /short description/i.test(steam) && /feature/i.test(steam) && /closed.*playtest/is.test(steam) && steam.includes('[PLAYTEST_WAITLIST_URL]'));
const timecodes = [...video.matchAll(/(?:^|\n)\s*(\d{1,2})(?:\s*[-–]\s*|:)(\d{1,2})\s*(?:s|sec|seconds)?/gi)];
check('vertical video is a timed 45-second proof storyboard', video.length >= 900 && /45[- ]second/i.test(video) && timecodes.length >= 5 && /shot|visual/i.test(video) && /on-screen|voice|spoken/i.test(video) && video.includes('[PLAYTEST_WAITLIST_URL]'));

const forbiddenPublic = [
  /\bMMO(?:RPG)?\b/i,
  /massively multiplayer/i,
  /\bopen world\b/i,
  /(?:multiple|many|several) playable planets?/i,
  /(?:multiple|many|several) playable biomes?/i,
  /\b60\s*[-–]\s*90\s*(?:minute|min)/i,
  /\b(?:available|download|buy|play) now\b/i,
  /\bPok[eé]mon\b/i,
];
check('public assets contain none of the frozen unsupported claims', steam.length > 0 && video.length > 0 && forbiddenPublic.every(pattern => !pattern.test(publicCopy)));
check('public assets state current-build truth and the shared CTA', /browser/i.test(publicCopy) && /creature/i.test(publicCopy) && /element|bond|evol/i.test(publicCopy) && steam.includes('Join the closed desktop-browser playtest waitlist.') && video.includes('Join the closed desktop-browser playtest waitlist.'));

const measurementObject = manifest.measurement || {};
const requiredEvents = ['landing_view', 'playtest_cta_click', 'waitlist_complete', 'qualified_desktop_confirm', 'playtest_start'];
check('machine-readable measurement contract has unobserved baseline and full funnel', measurementObject.baseline_status === 'unobserved' && typeof measurementObject.primary_metric === 'string' && requiredEvents.every(event => JSON.stringify(measurementObject.events || []).includes(event)) && Array.isArray(measurementObject.decision_rules) && measurementObject.decision_rules.length >= 3);
check('measurement prose is denominator-aware and precommitted', measurement.length >= 1000 && requiredEvents.every(event => measurement.includes(event)) && /denominator/i.test(measurement) && /100 qualified landing views/i.test(measurement) && /14 days/i.test(measurement) && /continue/i.test(measurement) && /change/i.test(measurement) && /stop/i.test(measurement) && /unobserved|no baseline/i.test(measurement));

check('executive review is self-contained and substantive', review.length >= 2200 && /<!doctype html>/i.test(review) && /strategy/i.test(review) && /claims/i.test(review) && /steam/i.test(review) && /vertical/i.test(review) && /measurement/i.test(review) && !/<script\b/i.test(review) && !/(?:src|href)\s*=\s*["']https?:/i.test(review));
check('nothing was represented as published', manifest.nothing_published === true && /unpublished|nothing (?:was|is|has been) published/i.test(`${strategy}\n${review}`));
check('deterministic internal verifier exists', fs.existsSync('verify-marketing.mjs') && read('verify-marketing.mjs').length >= 800);

console.log(`\nchecks: ${step}; failures: ${failed ? 'yes' : 'no'}`);
process.exit(failed ? 1 : 0);
