import { readFileSync } from 'node:fs';

const ledger = JSON.parse(readFileSync(new URL('../candidate/DECISION_LEDGER.json', import.meta.url)));
const failures = [];
const seen = ledger.seen_signal_ids ?? [];
for (const id of ['SIG-001', 'SIG-002', 'SIG-003', 'SIG-004', 'SIG-005']) if (!seen.includes(id)) failures.push(`missing ${id}`);
if (new Set(seen).size !== seen.length) failures.push('duplicate causal signal');
const pricing = ledger.decisions?.['pricing-response'] ?? {};
if (!pricing.evidence?.includes('SIG-001') || !pricing.evidence?.includes('SIG-004')) failures.push('pricing lost contradiction provenance');
if (pricing.last_changed_by !== 'SIG-004') failures.push('pricing stale or duplicate changed lineage');
if (!/12%|below|lower/i.test(`${pricing.conclusion} ${pricing.action}`) || /18% above.*current|maintain.*premium/i.test(`${pricing.conclusion} ${pricing.action}`)) failures.push('stale pricing conclusion survived');
const health = ledger.decisions?.['health-vertical'] ?? {};
if (!health.evidence?.includes('SIG-002') || !health.evidence?.includes('SIG-005')) failures.push('health provenance incomplete');
if (health.last_changed_by !== 'SIG-005' || health.state !== 'hold') failures.push('health action did not stop for residency evidence');
if (!/residen/i.test(`${health.conclusion} ${health.action} ${(health.unknowns ?? []).join(' ')}`)) failures.push('residency constraint missing');
if (ledger.updated_for_signal !== 'SIG-005') failures.push('terminal ledger not on scheduled refresh');
if (failures.length) { console.error(failures.join('\n')); process.exit(1); }
console.log(JSON.stringify({ status: 'pass', serious_blockers: 0, retained_provenance: true }));
