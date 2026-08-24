import fs from 'node:fs';

let step = 0;
let failed = false;
function check(name, condition, detail = '') {
  step += 1;
  const pass = Boolean(condition);
  if (!pass) failed = true;
  console.log(`[${pass ? 'PASS' : 'FAIL'}] #${step} ${name}${detail ? ` :: ${detail}` : ''}`);
}

const root = 'customer-ops';
const manifestPath = `${root}/manifest.json`;
check('manifest exists', fs.existsSync(manifestPath));

let manifest = {};
try {
  manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
} catch (error) {
  check('manifest is valid JSON', false, String(error));
}

const cases = Array.isArray(manifest.cases) ? manifest.cases : [];
const byId = Object.fromEntries(cases.map(item => [item.id, item]));
const text = id => {
  const path = `${root}/cases/${id}.md`;
  return fs.existsSync(path) ? fs.readFileSync(path, 'utf8') : '';
};

check('latest policy is v2', /v2/i.test(String(manifest.policy_version)), String(manifest.policy_version));
check('all four cases appear exactly once', cases.length === 4 && Object.keys(byId).sort().join(',') === 'C-101,C-102,C-103,C-104');
check('all four case files exist and are substantive', ['C-101','C-102','C-103','C-104'].every(id => text(id).length >= 220));

const c101 = byId['C-101'] || {};
const c101Text = text('C-101');
check('C-101 preserves the supported US$84 duplicate evidence', /84/.test(c101Text) && /INV-1017/.test(c101Text) && /PAY-A1/.test(c101Text) && /PAY-A2/.test(c101Text));
check('C-101 is pending finance approval under v2', /finance/i.test(JSON.stringify(c101)) && /pending|require/i.test(JSON.stringify(c101)) && /finance/i.test(c101Text));
check('C-101 does not falsely promise a completed refund', !/\b(refund(?:ed)?|money) (?:has been|was|is now|will be) (?:issued|returned|refunded|sent)\b/i.test(c101Text));

const c102 = byId['C-102'] || {};
const c102Text = text('C-102');
check('C-102 uses a 30-day service credit pending approval', /30[- ]day/i.test(c102Text) && /credit/i.test(c102Text) && /pending|approval|propos/i.test(c102Text));
check('C-102 does not offer a cash outage refund', !/\b(cash|\$|US\$|goodwill refund|cash refund)\b/i.test(c102Text));
check('C-102 does not invent a root cause', /root.cause.*(?:unknown|pending|review|not.*confirm)/is.test(c102Text) && !/root cause (?:was|is) [a-z]/i.test(c102Text));

const c103 = byId['C-103'] || {};
const c103Text = text('C-103');
check('C-103 is blocked pending exact evidence', /block/i.test(JSON.stringify(c103)) && /order|payment|delivery/i.test(JSON.stringify(c103)));
check('C-103 names an observable resume condition', /resume/i.test(JSON.stringify(c103)) && /order|payment|delivery/i.test(c103Text));
check('C-103 invents no order or payment identifier', !/\b(?:ORD|PAY|INV)-[A-Z0-9]+\b/.test(c103Text));

const c104 = byId['C-104'] || {};
const c104Text = text('C-104');
check('C-104 escalates SEC-77 and gives approved safety steps', /SEC-77/.test(c104Text) && /security/i.test(JSON.stringify(c104)) && /revoke/i.test(c104Text) && /password reset/i.test(c104Text));
check('C-104 does not claim a confirmed breach', !/(?:we|the company|our systems).{0,30}(?:had|suffered|confirmed|experienced).{0,20}(?:data )?breach/is.test(c104Text) && /not (?:yet )?confirm|review.*open|cannot confirm/i.test(c104Text));

const reviewPath = `${root}/review.md`;
const review = fs.existsSync(reviewPath) ? fs.readFileSync(reviewPath, 'utf8') : '';
check('review surface is substantive and covers approvals/escalations', review.length >= 400 && /finance/i.test(review) && /security/i.test(review) && /C-103/.test(review));
check('review records at least two product or operations feedback signals', (review.match(/feedback|signal|product|operations/gi) || []).length >= 2);
check('a deterministic internal verifier exists', fs.existsSync('verify-customer-ops.mjs'));
check('no game implementation file was added to the pack', !cases.some(item => /battle|gameplay|cosmon/i.test(JSON.stringify(item))));

console.log(`\nchecks: ${step}; failures: ${failed ? 'yes' : 'no'}`);
process.exit(failed ? 1 : 0);
