import fs from 'node:fs';

let step = 0;
let failed = false;
function check(name, condition, detail = '') {
  step += 1;
  const pass = Boolean(condition);
  if (!pass) failed = true;
  console.log(`[${pass ? 'PASS' : 'FAIL'}] #${step} ${name}${detail ? ` :: ${detail}` : ''}`);
}

const sourceIds = [
  ...Array.from({ length: 5 }, (_, index) => `A-0${index + 1}`),
  ...Array.from({ length: 5 }, (_, index) => `B-0${index + 1}`),
];
const sourceSet = new Set(sourceIds);
const required = [
  'research-decision/manifest.json',
  'research-decision/decision.md',
  'research-decision/evidence-map.md',
  'research-decision/review.html',
  'verify-research-decision.mjs',
];
const read = path => fs.existsSync(path) ? fs.readFileSync(path, 'utf8') : '';

check('all required decision files exist', required.every(path => fs.existsSync(path)));

let manifest = {};
let validJson = false;
try {
  manifest = JSON.parse(read('research-decision/manifest.json'));
  validJson = true;
} catch {
  // Fixed-count failure below.
}
check('manifest is valid JSON', validJson);
check('schema and fictional company identity are exact', manifest.schema_version === 'exp03-t5-v1' && manifest.company?.id === 'cosmon_test' && manifest.company?.fictional === true && manifest.nothing_external === true);

const decision = manifest.decision || {};
check('decision is explicit, falsifiable and actionable', ['primary_bet', 'recommendation', 'rationale', 'rejected_or_deferred', 'confidence', 'falsifier', 'next_action'].every(key => String(decision[key] || '').length >= 20));

const claims = Array.isArray(manifest.claims) ? manifest.claims : [];
const claimIds = claims.map(claim => claim.id);
check('claims are unique and epistemically typed', claims.length >= 8 && new Set(claimIds).size === claims.length && claims.every(claim => typeof claim.id === 'string' && String(claim.text || '').length >= 15 && ['observation', 'estimate', 'judgement', 'assumption', 'unknown'].includes(claim.kind)));
check('every machine-readable claim cites only supplied sources', claims.every(claim => Array.isArray(claim.source_ids) && claim.source_ids.length > 0 && claim.source_ids.every(source => sourceSet.has(source))));
check('claims use both independent evidence regions', claims.some(claim => claim.source_ids.some(source => source.startsWith('A-'))) && claims.some(claim => claim.source_ids.some(source => source.startsWith('B-'))));

const options = Array.isArray(manifest.options) ? manifest.options : [];
check('both frozen options are compared exactly once', options.length === 2 && [...options.map(option => option.id)].sort().join('|') === 'A|B');
check('each option covers benefits, risks, learning and feasibility', options.every(option => ['benefits', 'risks', 'learning_value', 'feasibility'].every(key => Array.isArray(option[key]) ? option[key].length > 0 : String(option[key] || '').length >= 15)));

const contradictions = Array.isArray(manifest.contradictions) ? manifest.contradictions : [];
check('at least one cross-region contradiction is resolved', contradictions.length > 0 && contradictions.every(item => String(item.tension || '').length >= 20 && String(item.resolution || '').length >= 20 && Array.isArray(item.source_ids) && item.source_ids.some(source => source.startsWith('A-')) && item.source_ids.some(source => source.startsWith('B-')) && item.source_ids.every(source => sourceSet.has(source))));
check('uncertainty and decision gates remain explicit', Array.isArray(manifest.uncertainties) && manifest.uncertainties.length >= 3 && Array.isArray(manifest.decision_gates) && manifest.decision_gates.length >= 2);

const memo = read('research-decision/decision.md');
const evidenceMap = read('research-decision/evidence-map.md');
const review = read('research-decision/review.html');
check('decision memo is substantive and cites both evidence regions', memo.length >= 900 && /\[A-0[1-5]\]/.test(memo) && /\[B-0[1-5]\]/.test(memo) && /funnel/i.test(memo) && /falsif|reverse/i.test(memo) && /next action/i.test(memo));
check('funnel preserves supplied stage counts', ['120', '96', '72', '54', '31', '18', '6'].every(value => memo.includes(value)));
check('memo does not turn the interview counts into population percentages', !/(?:eleven|11|nine|9|ten|10|four|4)\s+(?:of\s+16\s+)?(?:participants|interviewees)?.{0,20}\b(?:25|56|62|63|69)\s*%/i.test(memo));
check('memo does not claim causal or real-market validation', !/\bcaused (?:higher |improved )?(?:retention|conversion|demand|revenue)\b/i.test(memo) && !/real[- ]world demand (?:is|was) (?:validated|proven)/i.test(memo));

check('evidence map covers every supplied source', evidenceMap.length >= 900 && sourceIds.every(id => evidenceMap.includes(id)) && /limitation/i.test(evidenceMap));
check('owner review is self-contained and script-free', review.length >= 900 && /<!doctype html>/i.test(review) && /option a/i.test(review) && /option b/i.test(review) && /uncertainty/i.test(review) && /next action/i.test(review) && !/<script\b/i.test(review) && !/(?:src|href)\s*=\s*["']https?:/i.test(review));
check('deterministic internal verifier exists', read('verify-research-decision.mjs').length >= 800);

console.log(`\nchecks: ${step}; failures: ${failed ? 'yes' : 'no'}`);
process.exit(failed ? 1 : 0);
