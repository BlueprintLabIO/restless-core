import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { normalizeMenu } from './menu.mjs';

const good = JSON.parse(await readFile('input/menu-source.json', 'utf8'));
const goodResult = normalizeMenu(good);
assert.equal(goodResult.valid, true);
assert.equal(goodResult.menu.items.find(item => item.id === 'main-pasta').allergens, null);

const conflicting = JSON.parse(await readFile('input/conflicting-menu-source.json', 'utf8'));
const conflictingResult = normalizeMenu(conflicting);
assert.equal(conflictingResult.valid, false);
assert(conflictingResult.errors.some(error => error.reason === 'source values conflict and require human resolution'));
console.log('thymelake menu fixture tests passed');
