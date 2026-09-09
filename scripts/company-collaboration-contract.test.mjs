import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import {
  CONTRACT_SET_FORMAT,
  CONTRACT_SET_NAME,
  createCompanyCollaborationContractSet,
  readCoreReleaseTuple,
} from './create-company-collaboration-contract-set.mjs';
import { createCoreReleaseManifest } from './create-core-release-manifest.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const sourceRevision = '1'.repeat(40);
const accountPlaneImage = `ghcr.io/blueprintlabio/restless-account-plane@sha256:${'a'.repeat(64)}`;
const companyRuntimeImage = `ghcr.io/blueprintlabio/restless-company-runtime@sha256:${'b'.repeat(64)}`;

function digest(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

async function generatedFixture() {
  const outputRoot = await mkdtemp(join(tmpdir(), 'restless-core-contract-'));
  const release = await readCoreReleaseTuple({
    sourceRoot: root,
    sourceRevision,
    accountPlaneImage,
    companyRuntimeImage,
  });
  const contractSet = await createCompanyCollaborationContractSet({
    sourceRoot: root,
    outputRoot,
    release,
  });
  return { outputRoot, release, contractSet };
}

test('the Core contract set is canonical, immutable and release-complete', async () => {
  const fixture = await generatedFixture();
  const bytes = await readFile(fixture.contractSet.manifestPath);
  const manifest = JSON.parse(bytes);
  assert.deepEqual(Object.keys(manifest), ['format', 'contract_set', 'release', 'capabilities', 'artifacts']);
  assert.equal(manifest.format, CONTRACT_SET_FORMAT);
  assert.equal(manifest.contract_set, CONTRACT_SET_NAME);
  assert.deepEqual(Object.keys(manifest.release), ['core_version', 'source_revision', 'images', 'contracts', 'deployment']);
  assert.equal(Object.hasOwn(manifest.release, 'release_manifest_digest'), false);
  assert.equal(manifest.release.source_revision, sourceRevision);
  assert.equal(manifest.release.images.account_plane, accountPlaneImage);
  assert.equal(manifest.release.images.company_runtime, companyRuntimeImage);
  assert.deepEqual(manifest.capabilities, [...manifest.capabilities].sort());
  assert.deepEqual(
    manifest.artifacts.map((artifact) => artifact.id),
    ['restless.core.collaboration.events', 'restless.core.collaboration.openapi'],
  );
  assert.equal(fixture.contractSet.manifestDigest, digest(bytes));
  assert.equal(bytes.toString('utf8'), `${JSON.stringify(manifest, null, 2)}\n`);

  const replay = await createCompanyCollaborationContractSet({
    sourceRoot: root,
    outputRoot: fixture.outputRoot,
    release: fixture.release,
  });
  assert.deepEqual(replay, fixture.contractSet);
});

test('the release manifest can only bind an existing verified contract set', async () => {
  const fixture = await generatedFixture();
  const created = await createCoreReleaseManifest({
    contractSetManifestPath: fixture.contractSet.manifestPath,
    outputRoot: fixture.outputRoot,
  });
  const bytes = await readFile(created.manifestPath);
  const manifest = JSON.parse(bytes);
  assert.deepEqual(Object.keys(manifest), [
    'manifest_version',
    'core_version',
    'source_revision',
    'images',
    'contracts',
    'deployment',
  ]);
  assert.equal(manifest.manifest_version, 1);
  assert.deepEqual(manifest.contracts.collaboration, {
    format: CONTRACT_SET_FORMAT,
    contract_set: CONTRACT_SET_NAME,
    manifest_digest: fixture.contractSet.manifestDigest,
  });
  assert.equal(created.manifestDigest, digest(bytes));
  assert.equal(created.contractSetDigest, fixture.contractSet.manifestDigest);

  const replay = await createCoreReleaseManifest({
    contractSetManifestPath: fixture.contractSet.manifestPath,
    outputRoot: fixture.outputRoot,
  });
  assert.deepEqual(replay, created);
});

test('artifact tampering prevents release-manifest creation', async () => {
  const fixture = await generatedFixture();
  const manifest = JSON.parse(await readFile(fixture.contractSet.manifestPath));
  const artifactPath = join(fixture.contractSet.bundleRoot, manifest.artifacts[0].path);
  await writeFile(artifactPath, '{}\n');
  await assert.rejects(
    createCoreReleaseManifest({
      contractSetManifestPath: fixture.contractSet.manifestPath,
      outputRoot: fixture.outputRoot,
    }),
    /bytes do not match/,
  );
});

test('the OpenAPI bootstrap shapes are exact 13-field input and 14-field receipt', async () => {
  const openapi = JSON.parse(await readFile(join(root, 'contracts/company-collaboration/v1/collaboration.openapi.json')));
  assert.deepEqual(Object.keys(openapi.paths).sort(), [
    '/api/companies/{company}/mentions',
    '/api/companies/{company}/rooms',
    '/api/companies/{company}/rooms/{room}/events',
    '/api/companies/{company}/rooms/{room}/events/live',
    '/api/companies/{company}/rooms/{room}/messages',
    '/api/companies/{company}/rooms/{room}/messages/{parent}/replies',
    '/api/companies/{company}/rooms/{room}/participants',
    '/api/companies/{company}/rooms/{room}/participants/{actor}',
    '/api/companies/{company}/rooms/{room}/read-cursor',
    '/api/companies/{company}/rooms/{room}/threads/{message}',
    '/internal/v1/companies/bootstrap',
  ]);
  const request = openapi.components.schemas.CompanyBootstrapRequest;
  const receipt = openapi.components.schemas.CompanyBootstrapReadyReceipt;
  assert.equal(request.additionalProperties, false);
  assert.equal(request.required.length, 13);
  assert.equal(Object.keys(request.properties).length, 13);
  assert.equal(receipt.additionalProperties, false);
  assert.equal(receipt.required.length, 14);
  assert.equal(Object.keys(receipt.properties).length, 14);
  assert.equal(receipt.properties.status.const, 'ready');
});

test('the account-plane Compose contract mounts distinct bootstrap audiences', async () => {
  const compose = await readFile(join(root, 'infra/account-plane/cloud-compose.template.yaml'), 'utf8');
  for (const required of [
    'RESTLESS_ACCOUNT_PLANE_IMAGE: "{{ACCOUNT_PLANE_IMAGE}}"',
    'RESTLESS_COMPANY_BOOTSTRAP_TOKEN_FILE: /run/secrets/company_bootstrap_token',
    'RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE: /run/secrets/runtime_bootstrap_token',
    'environment: RESTLESS_COMPANY_BOOTSTRAP_TOKEN',
    'environment: RESTLESS_RUNTIME_BOOTSTRAP_TOKEN',
    '- source: company_bootstrap_token',
    'target: company_bootstrap_token',
    '- source: runtime_bootstrap_token',
    'target: runtime_bootstrap_token',
    'mode: 0400',
  ]) assert.match(compose, new RegExp(required.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.equal(compose.match(/mode: 0400/g)?.length, 2);
  assert.doesNotMatch(compose, /RESTLESS_CORE_RELEASE/);
});

test('mutable image references are not accepted as release inputs', async () => {
  await assert.rejects(
    readCoreReleaseTuple({
      sourceRoot: root,
      sourceRevision,
      accountPlaneImage: 'ghcr.io/blueprintlabio/restless-account-plane:latest',
      companyRuntimeImage,
    }),
    /account-plane image is invalid/,
  );
});
