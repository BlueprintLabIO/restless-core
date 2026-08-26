import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { normalizeMenu } from './menu.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const good = JSON.parse(await readFile(path.join(directory, 'input/menu-source.json'), 'utf8'));
const goodResult = normalizeMenu(good);
assert.equal(goodResult.valid, true);
assert.equal(goodResult.menu.items.find(item => item.id === 'main-pasta').allergens, null);

const conflicting = JSON.parse(await readFile(path.join(directory, 'input/conflicting-menu-source.json'), 'utf8'));
const conflictingResult = normalizeMenu(conflicting);
assert.equal(conflictingResult.valid, false);
assert(conflictingResult.errors.some(error => error.reason === 'source values conflict and require human resolution'));
console.log('thymelake menu fixture tests passed');
