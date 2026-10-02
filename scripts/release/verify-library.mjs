import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const [root, kind, revision, day, expectedInput] = process.argv.slice(2);
assert.ok(['ui', 'office', 'issuer'].includes(kind));
const release = JSON.parse(readFileSync(join(root, 'library-release.json')));
assert.equal(release.format, 'restless.core.library.v1');
assert.equal(release.component, kind);
assert.equal(release.source.repository, 'BlueprintLabIO/restless-core');
assert.equal(release.source.revision, revision);
assert.equal(release.vulnerability_scan.period, day);
assert.equal(release.vulnerability_scan.high_critical, 0);
assert.equal(release.name, `@restless/${kind}`);
if (expectedInput !== undefined) {
  assert.match(expectedInput, /^[0-9a-f]{64}$/);
  assert.equal(release.input_sha256, expectedInput, 'library does not prove the requested inputs');
}
const sha = file => {
  assert.ok(!file.startsWith('/') && !file.split('/').includes('..') && !file.includes('\\'));
  return createHash('sha256').update(readFileSync(join(root, file))).digest('hex');
};
assert.equal(sha(release.archive.file), release.archive.sha256);
const required = [`${kind}-manifest.json`, `${release.archive.file}.sha256`, 'qualification.txt',
  'dependencies/package.json', 'dependencies/package-lock.json', 'npm-audit.json', 'sbom.spdx.json', 'grype.json'];
assert.deepEqual(Object.keys(release.evidence).sort(), required.sort());
for (const [file, digest] of Object.entries(release.evidence)) assert.equal(sha(file), digest, `changed evidence: ${file}`);
const manifest = JSON.parse(readFileSync(join(root, `${kind}-manifest.json`)));
assert.equal(manifest.sourceRevision, revision);
assert.equal(manifest.sourceDirty, false);
assert.equal(manifest.qualificationOnly, undefined);
assert.equal(manifest.sha256, release.archive.sha256);
assert.equal(manifest.tarball, release.archive.file);
assert.equal(manifest.name, release.name);
assert.equal(manifest.version, release.version);
console.log(`PASS exact ${kind} library payload and evidence from ${revision}`);
