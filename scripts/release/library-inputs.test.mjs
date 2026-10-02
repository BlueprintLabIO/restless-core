import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { affectedLibraries, libraryInputs, libraryKinds } from './library-inputs.mjs';

const actual = fileURLToPath(new URL('../../', import.meta.url));
const files = [...new Set(libraryKinds.flatMap(kind => libraryInputs(actual, kind).files.map(file => file.path)))];
function snapshot() {
  const root = mkdtempSync(join(tmpdir(), 'restless-library-inputs-'));
  for (const path of files) {
    mkdirSync(dirname(join(root, path)), {recursive: true});
    cpSync(join(actual, path), join(root, path));
  }
  return root;
}
const change = (root, path) => writeFileSync(join(root, path), readFileSync(join(root, path), 'utf8') + '\n// release-input qualification\n');

test('real source edits invalidate only their packaged dependants', () => {
  const before = snapshot();
  try {
    for (const [path, expected] of [
      ['web/src/lib/ui/time.ts', ['ui']],
      ['web/src/lib/model/cockpit.ts', ['office']],
      ['services/identity/src/issuer.mjs', ['issuer']],
      ['web/package-lock.json', ['ui', 'office']],
      ['.dagger/src/libraries.ts', libraryKinds],
    ]) {
      const current = snapshot();
      try { change(current, path); assert.deepEqual(affectedLibraries(current, before), expected, path); }
      finally { rmSync(current, {recursive: true, force: true}); }
    }
  } finally { rmSync(before, {recursive: true, force: true}); }
});

test('unchanged, unrelated and removed-file inputs are compared correctly', () => {
  const before = snapshot(), current = snapshot();
  try {
    assert.deepEqual(affectedLibraries(current, before), []);
    mkdirSync(join(current, 'src'), {recursive: true});
    writeFileSync(join(current, 'src/runtime.rs'), '// unrelated Rust source');
    writeFileSync(join(current, 'README.md'), 'unrelated documentation');
    assert.deepEqual(affectedLibraries(current, before), []);
    rmSync(join(current, 'web/src/lib/ui/time.ts'));
    assert.deepEqual(affectedLibraries(current, before), ['ui']);
  } finally { for (const root of [before, current]) rmSync(root, {recursive: true, force: true}); }
});

test('symlink inputs are refused rather than read outside the source', () => {
  const root = snapshot();
  try {
    const path = join(root, 'web/src/lib/ui/time.ts');
    rmSync(path); symlinkSync('/etc/passwd', path);
    assert.throws(() => libraryInputs(root, 'ui'), /cannot be a symlink/);
  } finally { rmSync(root, {recursive: true, force: true}); }
});
