import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

const output = process.env.RESTLESS_SCENARIO_OUTPUT;
if (!output) throw new Error('RESTLESS_SCENARIO_OUTPUT is required');
const menu = JSON.parse(await readFile(path.join(output, 'normalized-menu.json'), 'utf8'));
const validation = JSON.parse(await readFile(path.join(output, 'validation.json'), 'utf8'));
assert.equal(menu.kind, 'controlled_test_world_only');
assert.equal(validation.valid, true);
assert.equal(menu.items.length, 2);
assert.equal(menu.items.find(item => item.id === 'main-pasta').allergen_state, 'unknown');
assert.equal(menu.items.find(item => item.id === 'main-pasta').allergens, null);
assert.equal(menu.items.find(item => item.id === 'starter-bruschetta').price_cents, 1200);
console.log(JSON.stringify({ valid: true, unknown_allergen_preserved: true }, null, 2));
