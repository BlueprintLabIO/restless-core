import { existsSync, readFileSync } from 'node:fs';

const accounts = JSON.parse(readFileSync(new URL('./accounts.json', import.meta.url)));
const failures = [];
for (const account of accounts) {
  const path = new URL(`./briefs/${account.account_id}.json`, import.meta.url);
  if (!existsSync(path)) { failures.push(`${account.account_id}: missing brief`); continue; }
  let brief;
  try { brief = JSON.parse(readFileSync(path)); } catch { failures.push(`${account.account_id}: invalid JSON`); continue; }
  for (const field of ['account_id', 'state', 'confidence', 'evidence', 'risk_or_opportunity', 'next_action', 'owner', 'unknowns']) {
    if (!(field in brief)) failures.push(`${account.account_id}: missing ${field}`);
  }
  if (brief.account_id !== account.account_id) failures.push(`${account.account_id}: wrong account_id`);
  if (!['stable', 'risk', 'opportunity', 'hold'].includes(brief.state)) failures.push(`${account.account_id}: invalid state`);
  if (!['high', 'medium', 'low'].includes(brief.confidence)) failures.push(`${account.account_id}: invalid confidence`);
  const allowed = new Set(account.sources.map((source) => source.source_id));
  if (!Array.isArray(brief.evidence) || brief.evidence.length < 2 || brief.evidence.some((id) => !allowed.has(id))) failures.push(`${account.account_id}: evidence not isolated/complete`);
  if (!Array.isArray(brief.unknowns)) failures.push(`${account.account_id}: unknowns must be array`);
}
if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log(JSON.stringify({ status: 'pass', briefs: accounts.length }));
