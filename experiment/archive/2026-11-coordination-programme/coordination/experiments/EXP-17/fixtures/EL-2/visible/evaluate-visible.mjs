import { existsSync, readFileSync } from 'node:fs';

const path = new URL('./DECISION_LEDGER.json', import.meta.url);
if (!existsSync(path)) throw new Error('DECISION_LEDGER.json is missing');
const ledger = JSON.parse(readFileSync(path));
if (ledger.schema !== 'exp17.el2.ledger.v1') throw new Error('wrong schema');
if (!Array.isArray(ledger.seen_signal_ids) || new Set(ledger.seen_signal_ids).size !== ledger.seen_signal_ids.length) throw new Error('signal identities must be unique');
const keys = Object.keys(ledger.decisions ?? {}).sort();
if (JSON.stringify(keys) !== JSON.stringify(['health-vertical', 'partner-channel', 'pricing-response'])) throw new Error(`wrong decision keys: ${keys}`);
for (const [key, decision] of Object.entries(ledger.decisions)) {
  if (!['act', 'hold', 'investigate'].includes(decision.state)) throw new Error(`${key}: invalid state`);
  for (const field of ['conclusion', 'action', 'evidence', 'unknowns', 'last_changed_by']) if (!(field in decision)) throw new Error(`${key}: missing ${field}`);
  if (!Array.isArray(decision.evidence) || !Array.isArray(decision.unknowns)) throw new Error(`${key}: evidence/unknowns must be arrays`);
}
console.log(JSON.stringify({ status: 'pass', signals: ledger.seen_signal_ids.length, decisions: keys.length }));
