import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const serviceRoot = new URL('../', import.meta.url);

test('the production container is pinned, compiled, non-root, healthchecked, and starts the real sidecar', async () => {
  const [dockerfile, dockerignore, manifest, lockfile] = await Promise.all([
    readFile(new URL('Dockerfile', serviceRoot), 'utf8'),
    readFile(new URL('.dockerignore', serviceRoot), 'utf8'),
    readFile(new URL('package.json', serviceRoot), 'utf8'),
    readFile(new URL('package-lock.json', serviceRoot), 'utf8'),
  ]);
  assert.equal((dockerfile.match(/FROM node:24-bookworm-slim/g) ?? []).length, 2);
  assert.match(dockerfile, /RUN npm ci --ignore-scripts/u);
  assert.match(dockerfile, /RUN npm run build/u);
  assert.match(dockerfile, /npm prune --omit=dev --ignore-scripts/u);
  assert.match(dockerfile, /USER 10001:10001/u);
  assert.match(dockerfile, /HEALTHCHECK[\s\S]+dist\/healthcheck\.js/u);
  assert.match(dockerfile, /CMD \["node", "dist\/main\.js"\]/u);
  assert.doesNotMatch(dockerfile, /--platform=/u, 'the image must not pin a host architecture');
  assert.match(dockerignore, /^\*$/mu);
  assert.match(dockerignore, /^!src\/\*\*$/mu);

  const packageJson = JSON.parse(manifest) as { engines?: { node?: string }; scripts?: { start?: string } };
  const packageLock = JSON.parse(lockfile) as { packages?: Record<string, { version?: string }> };
  assert.equal(packageJson.engines?.node, '>=22');
  assert.equal(packageJson.scripts?.start, 'node dist/main.js');
  assert.equal(packageLock.packages?.['node_modules/@hocuspocus/server']?.version, '4.6.0');
});
