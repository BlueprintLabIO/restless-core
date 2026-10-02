import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [root, kind, revision, day] = process.argv.slice(2);
assert.ok(['ui', 'office', 'issuer'].includes(kind));
assert.match(revision, /^[0-9a-f]{40}$/);
assert.equal(day, new Date().toISOString().slice(0, 10));
const manifest = JSON.parse(readFileSync(join(root, `${kind}-manifest.json`)));
assert.equal(manifest.sourceRevision, revision);
assert.equal(manifest.sourceDirty, false);
assert.equal(manifest.qualificationOnly, undefined);
const sha = name => createHash('sha256').update(readFileSync(join(root, name))).digest('hex');
assert.equal(sha(manifest.tarball), manifest.sha256);
const scan = JSON.parse(readFileSync(join(root, 'grype.json')));
assert.equal(scan.source?.type, 'sbom');
assert.equal((scan.matches ?? []).filter(match => ['High', 'Critical'].includes(match.vulnerability?.severity)).length, 0);
const sbom = JSON.parse(readFileSync(join(root, 'sbom.spdx.json')));
const lock = JSON.parse(readFileSync(join(root, 'dependencies/package-lock.json')));
for (const [path, pkg] of Object.entries(lock.packages)) {
  if (!path.includes('node_modules/') || pkg.dev) continue;
  const name = pkg.name ?? path.slice(path.lastIndexOf('node_modules/') + 'node_modules/'.length);
  assert.ok(sbom.packages.some(item => item.name === name && item.versionInfo === pkg.version),
    `SBOM omitted runtime dependency ${name} ${pkg.version}`);
}
const evidence = [`${kind}-manifest.json`, `${manifest.tarball}.sha256`, 'qualification.txt',
  'dependencies/package.json', 'dependencies/package-lock.json', 'npm-audit.json', 'sbom.spdx.json', 'grype.json'];
const release = { format: 'restless.core.library.v1', component: kind,
  source: { repository: 'BlueprintLabIO/restless-core', revision },
  name: manifest.name, version: manifest.version,
  archive: { file: manifest.tarball, sha256: manifest.sha256 },
  vulnerability_scan: { period: day, high_critical: 0 },
  evidence: Object.fromEntries(evidence.map(file => [file, sha(file)])) };
writeFileSync(join(root, 'library-release.json'), JSON.stringify(release, null, 2) + '\n');
console.log(`Qualified ${manifest.name} ${manifest.version}: archive, consumer, SBOM and High/Critical scan`);
