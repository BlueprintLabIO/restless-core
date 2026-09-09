#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { open, mkdir, readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const CONTRACT_SET_FORMAT = 'restless.core.contract-set.v1';
export const CONTRACT_SET_NAME = 'company-collaboration';
export const NATIVE_DOCUMENTS_CAPABILITY = 'native-documents-collaboration.v1';
export const NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT =
  'restless.core.native-documents-collaboration.deployment';
export const NATIVE_DOCUMENTS_HEALTH_ARTIFACT =
  'restless.core.native-documents-collaboration.health';
export const NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT =
  'restless.core.native-documents-collaboration.protocol';
export const NATIVE_DOCUMENTS_TOKEN_ARTIFACT =
  'restless.core.native-documents-collaboration.token';

const SOURCE_REVISION = /^[0-9a-f]{40}$/;
const OCI_DIGEST = /^(?:[a-z0-9.-]+(?::[0-9]+)?\/)[a-z0-9._/-]+@sha256:[0-9a-f]{64}$/;
const scriptRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const capabilities = Object.freeze([
  'mentions.commands.v1',
  'mentions.events.v1',
  'mentions.queries.v1',
  NATIVE_DOCUMENTS_CAPABILITY,
  'rooms.commands.v1',
  'rooms.event-replay.v1',
  'rooms.event-stream.v1',
  'rooms.queries.v1',
  'rooms.threads.v1',
]);

const artifactSources = Object.freeze([
  Object.freeze({
    id: 'restless.core.collaboration.events',
    path: 'contracts/collaboration.events.schema.json',
    source: 'contracts/company-collaboration/v1/collaboration.events.schema.json',
    media_type: 'application/schema+json',
  }),
  Object.freeze({
    id: 'restless.core.collaboration.openapi',
    path: 'contracts/collaboration.openapi.json',
    source: 'contracts/company-collaboration/v1/collaboration.openapi.json',
    media_type: 'application/vnd.oai.openapi+json',
  }),
  Object.freeze({
    id: NATIVE_DOCUMENTS_HEALTH_ARTIFACT,
    path: 'contracts/native-documents/health.schema.json',
    source:
      'contracts/company-collaboration/v1/native-documents-collaboration.health.schema.json',
    media_type: 'application/schema+json',
  }),
  Object.freeze({
    id: NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT,
    path: 'contracts/native-documents/protocol.schema.json',
    source:
      'contracts/company-collaboration/v1/native-documents-collaboration.protocol.schema.json',
    media_type: 'application/schema+json',
  }),
  Object.freeze({
    id: NATIVE_DOCUMENTS_TOKEN_ARTIFACT,
    path: 'contracts/native-documents/token.schema.json',
    source:
      'contracts/company-collaboration/v1/native-documents-collaboration.token.schema.json',
    media_type: 'application/schema+json',
  }),
]);

function nativeDocumentsDescriptor(image) {
  requireMatch(image, OCI_DIGEST, 'native Documents image');
  return {
    format: 'restless.core.native-documents-hosting.v1',
    capability: {
      id: 'native_documents_collaboration',
      version: 1,
    },
    image,
    protocol: {
      version: 1,
      schema_version: 1,
      artifact_id: NATIVE_DOCUMENTS_PROTOCOL_ARTIFACT,
    },
    routing: {
      internal_listen_port: 6688,
      reserved_route_prefix:
        '/api/companies/{company_id}/documents/{document_id}/collaboration',
    },
    health: {
      liveness_path: '/internal/v1/native-documents/live',
      readiness_path: '/internal/v1/native-documents/ready',
      artifact_id: NATIVE_DOCUMENTS_HEALTH_ARTIFACT,
    },
    storage: {
      mount_path: '/var/lib/restless/native-documents',
      class: 'cell-durable',
      credential_file: '/run/secrets/native_documents_store',
      scope: 'document-content-only',
    },
    token: {
      issuer_template: 'https://{plane_hostname}',
      jwks_path: '/.well-known/restless-native-documents-jwks.json',
      audience: 'restless-native-documents-collaboration',
      minimum_ttl_seconds: 15,
      maximum_ttl_seconds: 120,
      artifact_id: NATIVE_DOCUMENTS_TOKEN_ARTIFACT,
    },
    availability: {
      always_on: true,
      runtime_dependency: 'none',
      instance_scope: 'company-cell',
      public_exposure: 'owner-plane-proxy-only',
    },
  };
}

function fail(message) {
  throw new Error(message);
}

function sha256(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

function canonical(value) {
  return Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
}

function parseCanonical(bytes, name) {
  let value;
  try {
    value = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    fail(`${name} is not JSON: ${error.message}`);
  }
  if (!canonical(value).equals(bytes)) fail(`${name} is not canonical two-space JSON`);
  return value;
}

function exactKeys(value, keys, name) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`${name} must be an object`);
  const expected = new Set(keys);
  for (const key of Object.keys(value)) if (!expected.has(key)) fail(`${name}.${key} is unsupported`);
  for (const key of keys) if (!Object.hasOwn(value, key)) fail(`${name}.${key} is required`);
}

function requireMatch(value, pattern, name) {
  if (typeof value !== 'string' || !pattern.test(value)) fail(`${name} is invalid`);
  return value;
}

function positiveInteger(value, name) {
  if (!Number.isSafeInteger(value) || value < 1) fail(`${name} must be a positive integer`);
  return value;
}

async function writeImmutable(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  let handle;
  try {
    handle = await open(path, 'wx', 0o644);
    await handle.writeFile(bytes);
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    const existing = await readFile(path);
    if (!existing.equals(bytes)) fail(`immutable release artifact already exists with different bytes: ${path}`);
  } finally {
    await handle?.close();
  }
}

function sourceNumber(source, expression, name) {
  const match = source.match(expression);
  if (!match) fail(`could not read ${name} from its canonical source`);
  return positiveInteger(Number(match[1]), name);
}

function sourceString(source, expression, name) {
  const match = source.match(expression);
  if (!match) fail(`could not read ${name} from its canonical source`);
  return match[1];
}

export async function readCoreReleaseTuple({
  sourceRoot = scriptRoot,
  sourceRevision,
  accountPlaneImage,
  companyRuntimeImage,
}) {
  requireMatch(sourceRevision, SOURCE_REVISION, 'source revision');
  requireMatch(accountPlaneImage, OCI_DIGEST, 'account-plane image');
  requireMatch(companyRuntimeImage, OCI_DIGEST, 'company Runtime image');
  const [cargo, release, entry, compose] = await Promise.all([
    readFile(join(sourceRoot, 'Cargo.toml'), 'utf8'),
    readFile(join(sourceRoot, 'crates/restlessd/src/release.rs'), 'utf8'),
    readFile(join(sourceRoot, 'crates/restlessd/src/entry.rs'), 'utf8'),
    readFile(join(sourceRoot, 'infra/account-plane/cloud-compose.template.yaml')),
  ]);
  return Object.freeze({
    core_version: sourceString(cargo, /^version\s*=\s*"([^"]+)"/m, 'Core version'),
    source_revision: sourceRevision,
    images: Object.freeze({
      account_plane: accountPlaneImage,
      company_runtime: companyRuntimeImage,
    }),
    contracts: Object.freeze({
      api: sourceNumber(release, /API_CONTRACT_VERSION:\s*u32\s*=\s*(\d+)/, 'API contract version'),
      identity_assertion: sourceNumber(entry, /ASSERTION_CONTRACT_VERSION:\s*u32\s*=\s*(\d+)/, 'identity assertion contract version'),
      schema: sourceNumber(release, /SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/, 'schema version'),
    }),
    deployment: Object.freeze({
      account_plane_compose_sha256: sha256(compose).slice('sha256:'.length),
    }),
  });
}

export async function createCompanyCollaborationContractSet({
  sourceRoot = scriptRoot,
  outputRoot,
  release,
  nativeDocumentsImage,
}) {
  if (!outputRoot) fail('output root is required');
  const descriptorBytes = canonical(nativeDocumentsDescriptor(nativeDocumentsImage));
  exactKeys(release, ['core_version', 'source_revision', 'images', 'contracts', 'deployment'], 'release');
  requireMatch(release.core_version, /^[A-Za-z0-9._+-]{1,64}$/, 'release.core_version');
  requireMatch(release.source_revision, SOURCE_REVISION, 'release.source_revision');
  exactKeys(release.images, ['account_plane', 'company_runtime'], 'release.images');
  requireMatch(release.images.account_plane, OCI_DIGEST, 'release.images.account_plane');
  requireMatch(release.images.company_runtime, OCI_DIGEST, 'release.images.company_runtime');
  exactKeys(release.contracts, ['api', 'identity_assertion', 'schema'], 'release.contracts');
  positiveInteger(release.contracts.api, 'release.contracts.api');
  positiveInteger(release.contracts.identity_assertion, 'release.contracts.identity_assertion');
  positiveInteger(release.contracts.schema, 'release.contracts.schema');
  exactKeys(release.deployment, ['account_plane_compose_sha256'], 'release.deployment');
  requireMatch(release.deployment.account_plane_compose_sha256, /^[0-9a-f]{64}$/, 'release.deployment.account_plane_compose_sha256');
  const canonicalRelease = {
    core_version: release.core_version,
    source_revision: release.source_revision,
    images: {
      account_plane: release.images.account_plane,
      company_runtime: release.images.company_runtime,
    },
    contracts: {
      api: release.contracts.api,
      identity_assertion: release.contracts.identity_assertion,
      schema: release.contracts.schema,
    },
    deployment: {
      account_plane_compose_sha256: release.deployment.account_plane_compose_sha256,
    },
  };

  const artifacts = [
    {
      id: NATIVE_DOCUMENTS_DESCRIPTOR_ARTIFACT,
      path: 'contracts/native-documents/hosting.json',
      media_type: 'application/json',
      size_bytes: descriptorBytes.length,
      sha256: sha256(descriptorBytes),
    },
  ];
  const artifactBytes = new Map([
    ['contracts/native-documents/hosting.json', descriptorBytes],
  ]);
  for (const source of artifactSources) {
    const bytes = await readFile(join(sourceRoot, source.source));
    parseCanonical(bytes, source.source);
    artifactBytes.set(source.path, bytes);
    artifacts.push({
      id: source.id,
      path: source.path,
      media_type: source.media_type,
      size_bytes: bytes.length,
      sha256: sha256(bytes),
    });
  }
  artifacts.sort((left, right) => (left.id < right.id ? -1 : left.id > right.id ? 1 : 0));
  const manifest = {
    format: CONTRACT_SET_FORMAT,
    contract_set: CONTRACT_SET_NAME,
    release: canonicalRelease,
    capabilities: [...capabilities],
    artifacts,
  };
  const manifestBytes = canonical(manifest);
  const manifestDigest = sha256(manifestBytes);
  const bundleRoot = join(resolve(outputRoot), CONTRACT_SET_NAME, manifestDigest.slice('sha256:'.length));
  for (const [path, bytes] of artifactBytes) await writeImmutable(join(bundleRoot, path), bytes);
  const manifestPath = join(bundleRoot, 'contract-set.json');
  await writeImmutable(manifestPath, manifestBytes);
  return Object.freeze({ bundleRoot, manifestPath, manifestDigest });
}

function cliArguments(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (!name?.startsWith('--') || value === undefined) fail('arguments must be --name value pairs');
    if (Object.hasOwn(values, name)) fail(`duplicate argument ${name}`);
    values[name] = value;
  }
  const allowed = new Set([
    '--output',
    '--account-plane-image',
    '--company-runtime-image',
    '--native-documents-image',
  ]);
  for (const name of Object.keys(values)) if (!allowed.has(name)) fail(`unsupported argument ${name}`);
  for (const name of allowed) if (!values[name]) fail(`${name} is required`);
  return values;
}

function exactCleanRevision(root) {
  let revision;
  let status;
  try {
    revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
    status = execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' });
  } catch (error) {
    fail(`release contract generation requires a Git checkout: ${error.message}`);
  }
  requireMatch(revision, SOURCE_REVISION, 'Git revision');
  if (status !== '') fail('release contract generation refuses a dirty or untracked working tree');
  return revision;
}

async function main() {
  const args = cliArguments(process.argv.slice(2));
  requireMatch(args['--native-documents-image'], OCI_DIGEST, 'native Documents image');
  const release = await readCoreReleaseTuple({
    sourceRoot: scriptRoot,
    sourceRevision: exactCleanRevision(scriptRoot),
    accountPlaneImage: args['--account-plane-image'],
    companyRuntimeImage: args['--company-runtime-image'],
  });
  const created = await createCompanyCollaborationContractSet({
    sourceRoot: scriptRoot,
    outputRoot: args['--output'],
    release,
    nativeDocumentsImage: args['--native-documents-image'],
  });
  process.stdout.write(`${JSON.stringify(created)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
