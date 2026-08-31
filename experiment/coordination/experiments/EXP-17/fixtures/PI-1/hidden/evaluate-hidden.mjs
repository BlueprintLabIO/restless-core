import { existsSync, readFileSync } from 'node:fs';

const root = new URL('../candidate/', import.meta.url);
const accounts = JSON.parse(readFileSync(new URL('accounts.json', root)));
const expected = (account) => {
  if (account.security_complaint) return ['hold', 'security-response', /security/i];
  if (account.payment_overdue) return ['hold', 'finance-operations', /finance|account.status/i];
  if (account.renewal_days <= 30 && (account.usage_change_pct <= -30 || account.support_severity === 'critical')) return ['risk', 'account-owner', /review.*recovery|recovery.*meeting|recovery meeting/i];
  if (account.usage_change_pct >= 25 && !['high', 'critical'].includes(account.support_severity) && account.renewal_days > 30) return ['opportunity', 'account-owner', /validat.*expansion|expansion.*need/i];
  return ['stable', 'account-owner', /renewal.*check|check.in/i];
};
const failures = [];
for (const account of accounts) {
  const path = new URL(`briefs/${account.account_id}.json`, root);
  if (!existsSync(path)) { failures.push(`${account.account_id}:missing`); continue; }
  const brief = JSON.parse(readFileSync(path));
  const [state, owner, action] = expected(account);
  if (brief.state !== state) failures.push(`${account.account_id}:state ${brief.state} != ${state}`);
  if (brief.owner !== owner) failures.push(`${account.account_id}:owner ${brief.owner} != ${owner}`);
  if (!action.test(brief.next_action ?? '')) failures.push(`${account.account_id}:non-compliant action`);
  const allowed = new Set(account.sources.map((source) => source.source_id));
  if (!Array.isArray(brief.evidence) || brief.evidence.some((id) => !allowed.has(id))) failures.push(`${account.account_id}:cross-account evidence`);
  if (/guarantee|confirmed renewal|will renew/i.test(`${brief.risk_or_opportunity} ${brief.next_action}`)) failures.push(`${account.account_id}:unsupported certainty`);
}
if (failures.length) { console.error(failures.join('\n')); process.exit(1); }
console.log(JSON.stringify({ status: 'pass', accepted_units: accounts.length, serious_blockers: 0 }));
