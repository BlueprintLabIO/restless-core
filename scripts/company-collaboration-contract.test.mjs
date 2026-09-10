import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import {
  CONTRACT_SET_FORMAT,
  CONTRACT_SET_NAME,
  NATIVE_DOCUMENTS_CAPABILITY,
  NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT,
  NATIVE_DOCUMENTS_HEALTH_ARTIFACT,
  NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT,
  NATIVE_DOCUMENTS_TOKEN_ARTIFACT,
  createCompanyCollaborationContractSet,
  readCoreReleaseTuple,
} from './create-company-collaboration-contract-set.mjs';
import { createCoreReleaseManifest } from './create-core-release-manifest.mjs';
import { createCoreReleaseBundle } from './create-core-release-bundle.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const sourceRevision = '1'.repeat(40);
const accountPlaneImage = `ghcr.io/blueprintlabio/restless-account-plane@sha256:${'a'.repeat(64)}`;
const companyRuntimeImage = `ghcr.io/blueprintlabio/restless-company-runtime@sha256:${'b'.repeat(64)}`;
const nativeDocumentsImage = `ghcr.io/blueprintlabio/restless-native-documents-collaboration@sha256:${'c'.repeat(64)}`;

function digest(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

async function generatedFixture(t) {
  const outputRoot = await mkdtemp(join(tmpdir(), 'restless-core-contract-'));
  t.after(() => rm(outputRoot, { recursive: true, force: true }));
  const release = await readCoreReleaseTuple({
    sourceRoot: root,
    sourceRevision,
    accountPlaneImage,
    companyRuntimeImage,
    nativeDocumentsImage,
  });
  const contractSet = await createCompanyCollaborationContractSet({
    sourceRoot: root,
    outputRoot,
    release,
  });
  return { outputRoot, release, contractSet };
}

test('the Core contract set is canonical, immutable and release-complete', async (t) => {
  const fixture = await generatedFixture(t);
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
  assert.equal(manifest.release.images.native_documents, nativeDocumentsImage);
  assert.deepEqual(manifest.capabilities, [...manifest.capabilities].sort());
  assert.ok(manifest.capabilities.includes(NATIVE_DOCUMENTS_CAPABILITY));
  assert.deepEqual(
    manifest.artifacts.map((artifact) => artifact.id),
    [
      'restless.core.collaboration.events',
      'restless.core.collaboration.openapi',
      NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT,
      NATIVE_DOCUMENTS_HEALTH_ARTIFACT,
      NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT,
      NATIVE_DOCUMENTS_TOKEN_ARTIFACT,
    ],
  );
  const descriptorArtifact = manifest.artifacts.find(
    ({ id }) => id === NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT,
  );
  const descriptor = JSON.parse(
    await readFile(join(fixture.contractSet.bundleRoot, descriptorArtifact.path)),
  );
  assert.equal(descriptor.image, nativeDocumentsImage);
  assert.equal(
    descriptor.routing.reserved_route_prefix,
    '/api/companies/{company_id}/documents/{document_id}/collaboration',
  );
  assert.equal(descriptor.availability.instance_scope, 'company-cell');
  assert.equal(descriptor.availability.public_exposure, 'owner-plane-proxy-only');
  assert.equal(descriptor.availability.runtime_dependency, 'none');
  assert.equal(descriptor.storage.scope, 'document-content-only');
  assert.equal(
    descriptor.token.jwks_path,
    '/.well-known/restless-native-documents-jwks.json',
  );
  assert.ok(descriptor.token.maximum_ttl_seconds <= 300);
  assert.deepEqual(
    [
      descriptor.protocol.artifact_id,
      descriptor.health.artifact_id,
      descriptor.token.artifact_id,
    ].sort(),
    [
      NATIVE_DOCUMENTS_HEALTH_ARTIFACT,
      NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT,
      NATIVE_DOCUMENTS_TOKEN_ARTIFACT,
    ].sort(),
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

test('the release manifest can only bind an existing verified contract set', async (t) => {
  const fixture = await generatedFixture(t);
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

test('the signed bundle index binds exact release, contract and native Documents bytes', async (t) => {
  const fixture = await generatedFixture(t);
  const release = await createCoreReleaseManifest({
    contractSetManifestPath: fixture.contractSet.manifestPath,
    outputRoot: fixture.outputRoot,
  });
  const releaseSignature = join(fixture.outputRoot, 'release-manifest.sigstore.json');
  const contractSignature = join(fixture.outputRoot, 'contract-set.sigstore.json');
  await writeFile(releaseSignature, '{"mediaType":"application/vnd.dev.sigstore.bundle+json;version=0.3"}\n');
  await writeFile(contractSignature, '{"mediaType":"application/vnd.dev.sigstore.bundle+json;version=0.3"}\n');
  const outputPath = join(fixture.outputRoot, 'core-release-bundle.json');
  const created = await createCoreReleaseBundle({
    bundleRoot: fixture.outputRoot,
    releaseManifestPath: release.manifestPath,
    contractSetPath: fixture.contractSet.manifestPath,
    releaseSignaturePath: releaseSignature,
    contractSignaturePath: contractSignature,
    repository: 'BlueprintLabIO/restless-core',
    workflowRef: 'BlueprintLabIO/restless-core/.github/workflows/immutable-core-release.yml@refs/heads/dev',
    outputPath,
  });
  const bytes = await readFile(outputPath);
  const index = JSON.parse(bytes);
  assert.equal(created.indexDigest, digest(bytes));
  assert.equal(index.source.revision, sourceRevision);
  assert.deepEqual(index.images, fixture.release.images);
  assert.equal(index.release_manifest.sha256, release.manifestDigest);
  assert.equal(index.contract_set.sha256, fixture.contractSet.manifestDigest);

  const changedRelease = JSON.parse(await readFile(release.manifestPath));
  changedRelease.images.native_documents =
    `ghcr.io/blueprintlabio/restless-native-documents@sha256:${'9'.repeat(64)}`;
  await writeFile(release.manifestPath, `${JSON.stringify(changedRelease, null, 2)}\n`);
  await assert.rejects(
    createCoreReleaseBundle({
      bundleRoot: fixture.outputRoot,
      releaseManifestPath: release.manifestPath,
      contractSetPath: fixture.contractSet.manifestPath,
      releaseSignaturePath: releaseSignature,
      contractSignaturePath: contractSignature,
      repository: 'BlueprintLabIO/restless-core',
      workflowRef: 'BlueprintLabIO/restless-core/.github/workflows/immutable-core-release.yml@refs/heads/dev',
      outputPath,
    }),
    /not from its canonical repository|release tuple differs|descriptor image differs/,
  );
});

test('artifact tampering prevents release-manifest creation', async (t) => {
  const fixture = await generatedFixture(t);
  const manifest = JSON.parse(await readFile(fixture.contractSet.manifestPath));
  const artifactPath = join(
    fixture.contractSet.bundleRoot,
    manifest.artifacts.find(({ id }) => id === NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT).path,
  );
  await writeFile(artifactPath, '{}\n');
  await assert.rejects(
    createCoreReleaseManifest({
      contractSetManifestPath: fixture.contractSet.manifestPath,
      outputRoot: fixture.outputRoot,
    }),
    /bytes do not match/,
  );
});

test('native Documents hosting requires one immutable image and refuses bundle tampering', async (t) => {
  const fixture = await generatedFixture(t);
  const missingNativeDocuments = structuredClone(fixture.release);
  delete missingNativeDocuments.images.native_documents;
  await assert.rejects(
    createCompanyCollaborationContractSet({
      sourceRoot: root,
      outputRoot: fixture.outputRoot,
      release: missingNativeDocuments,
    }),
    /native Documents image is invalid/,
  );
  const mutableNativeDocuments = structuredClone(fixture.release);
  mutableNativeDocuments.images.native_documents =
    'ghcr.io/blueprintlabio/restless-native-documents:latest';
  await assert.rejects(
    createCompanyCollaborationContractSet({
      sourceRoot: root,
      outputRoot: fixture.outputRoot,
      release: mutableNativeDocuments,
    }),
    /native Documents image is invalid/,
  );

  const manifest = JSON.parse(await readFile(fixture.contractSet.manifestPath));
  const descriptorArtifact = manifest.artifacts.find(
    ({ id }) => id === NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT,
  );
  await writeFile(join(fixture.contractSet.bundleRoot, descriptorArtifact.path), '{}\n');
  await assert.rejects(
    createCompanyCollaborationContractSet({
      sourceRoot: root,
      outputRoot: fixture.outputRoot,
      release: fixture.release,
    }),
    /immutable release artifact already exists with different bytes/,
  );
});

test('the CLI requires the native Documents image before release generation', () => {
  const result = spawnSync(
    process.execPath,
    [
      join(root, 'scripts/create-company-collaboration-contract-set.mjs'),
      '--output',
      '/tmp/restless-contract-cli-missing-image',
      '--account-plane-image',
      accountPlaneImage,
      '--company-runtime-image',
      companyRuntimeImage,
    ],
    { encoding: 'utf8' },
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /--native-documents-image is required/);
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
    'RESTLESS_PUBLISHED_SERVICE_PROVIDER: cloud-http',
    'RESTLESS_PUBLISHED_SERVICE_PROVIDER_URL: ${RESTLESS_PUBLISHED_SERVICE_PROVIDER_URL}',
    'RESTLESS_PUBLISHED_SERVICE_PROVIDER_TOKEN: ${RESTLESS_PUBLISHED_SERVICE_PROVIDER_TOKEN}',
    'INFISICAL_API_URL: ${INFISICAL_API_URL}',
    'INFISICAL_PROJECT_ID: ${INFISICAL_PROJECT_ID}',
    'INFISICAL_ENVIRONMENT: ${INFISICAL_ENVIRONMENT}',
    'INFISICAL_UNIVERSAL_AUTH_CLIENT_ID: ${INFISICAL_UNIVERSAL_AUTH_CLIENT_ID}',
    'INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET: ${INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET}',
    'name: restless-plane-{{PLANE_ID}}_database',
    'environment: RESTLESS_COMPANY_BOOTSTRAP_TOKEN',
    'environment: RESTLESS_RUNTIME_BOOTSTRAP_TOKEN',
    '- source: company_bootstrap_token',
    'target: company_bootstrap_token',
    '- source: runtime_bootstrap_token',
    'target: runtime_bootstrap_token',
    'mode: 0400',
  ]) assert.match(compose, new RegExp(required.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.equal(compose.match(/mode: 0400/g)?.length, 4);
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

test('the release workflow preserves one branch-neutral, signed digest handoff', async () => {
  const workflow = await readFile(join(root, '.github/workflows/immutable-core-release.yml'), 'utf8');
  assert.match(workflow, /on:\n  workflow_dispatch:/);
  assert.doesNotMatch(workflow, /branches:/);
  assert.equal(workflow.match(/ref: \$\{\{ github\.sha \}\}/g)?.length, 2);
  assert.equal(workflow.match(/provenance: mode=max/g)?.length, 3);
  assert.equal(workflow.match(/sbom: true/g)?.length, 3);
  for (const required of [
    'cosign sign --yes "$ACCOUNT_PLANE_IMAGE"',
    'cosign sign --yes "$COMPANY_RUNTIME_IMAGE"',
    'cosign sign --yes "$NATIVE_DOCUMENTS_IMAGE"',
    'release-manifest.sigstore.json',
    'contract-set.sigstore.json',
    'core-release-bundle.sigstore.json',
    'node scripts/create-core-release-bundle.mjs',
    'docker buildx imagetools inspect --raw "$image"',
    'oras push "${RELEASE_REPOSITORY}:${GITHUB_SHA}"',
    'reference="${RELEASE_REPOSITORY}@${digest}"',
    'format:"restless.core.release-handoff.v1"',
    'core-release-handoff.sigstore.json',
  ]) assert.ok(workflow.includes(required), `release workflow is missing ${required}`);
  assert.ok(workflow.indexOf('node scripts/create-core-release-bundle.mjs')
    < workflow.indexOf('tar --sort=name'));
  assert.ok(workflow.indexOf('cosign sign --yes "$reference"')
    < workflow.indexOf('format:"restless.core.release-handoff.v1"'));
});
