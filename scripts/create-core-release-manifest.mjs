#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { open, mkdir, readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  CONTRACT_SET_FORMAT,
  CONTRACT_SET_NAME,
} from './create-company-collaboration-contract-set.mjs';

const SHA256 = /^sha256:[0-9a-f]{64}$/;
const SOURCE_REVISION = /^[0-9a-f]{40}$/;
const OCI_DIGEST = /^(?:[a-z0-9.-]+(?::[0-9]+)?\/)[a-z0-9._/-]+@sha256:[0-9a-f]{64}$/;

function fail(message) {
  throw new Error(message);
}

function sha256(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

function canonical(value) {
  return Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
}

function exactKeys(value, keys, name) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`${name} must be an object`);
  const expected = new Set(keys);
  for (const key of Object.keys(value)) if (!expected.has(key)) fail(`${name}.${key} is unsupported`);
  for (const key of keys) if (!Object.hasOwn(value, key)) fail(`${name}.${key} is required`);
}

function match(value, expression, name) {
  if (typeof value !== 'string' || !expression.test(value)) fail(`${name} is invalid`);
}

function positive(value, name) {
  if (!Number.isSafeInteger(value) || value < 1) fail(`${name} must be a positive integer`);
}

async function writeImmutable(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  let handle;
  try {
    handle = await open(path, 'wx', 0o644);
    await handle.writeFile(bytes);
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    if (!(await readFile(path)).equals(bytes)) fail(`immutable release artifact already exists with different bytes: ${path}`);
  } finally {
    await handle?.close();
  }
}

async function readCanonical(path, name) {
  const bytes = await readFile(path);
  let value;
  try {
    value = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    fail(`${name} is not JSON: ${error.message}`);
  }
  if (!canonical(value).equals(bytes)) fail(`${name} is not canonical two-space JSON`);
  return { bytes, value };
}

function validateRelease(release) {
  exactKeys(release, ['core_version', 'source_revision', 'images', 'contracts', 'deployment'], 'contract-set release');
  match(release.core_version, /^[A-Za-z0-9._+-]{1,64}$/, 'contract-set release core_version');
  match(release.source_revision, SOURCE_REVISION, 'contract-set release source_revision');
  exactKeys(release.images, ['account_plane', 'company_runtime'], 'contract-set release images');
  match(release.images.account_plane, OCI_DIGEST, 'contract-set release account_plane image');
  match(release.images.company_runtime, OCI_DIGEST, 'contract-set release company_runtime image');
  exactKeys(release.contracts, ['api', 'identity_assertion', 'schema'], 'contract-set release contracts');
  positive(release.contracts.api, 'contract-set release API contract');
  positive(release.contracts.identity_assertion, 'contract-set release identity contract');
  positive(release.contracts.schema, 'contract-set release schema contract');
  exactKeys(release.deployment, ['account_plane_compose_sha256'], 'contract-set release deployment');
  match(release.deployment.account_plane_compose_sha256, /^[0-9a-f]{64}$/, 'contract-set release Compose digest');
}

async function validateArtifacts(contractSetPath, manifest) {
  if (!Array.isArray(manifest.capabilities) || manifest.capabilities.length === 0) fail('contract-set capabilities are missing');
  if ([...manifest.capabilities].sort().some((value, index) => value !== manifest.capabilities[index])) fail('contract-set capabilities must be sorted');
  if (!Array.isArray(manifest.artifacts) || manifest.artifacts.length === 0) fail('contract-set artifacts are missing');
  let previous = '';
  for (const artifact of manifest.artifacts) {
    exactKeys(artifact, ['id', 'path', 'media_type', 'size_bytes', 'sha256'], `contract artifact ${artifact.id ?? '?'}`);
    if (typeof artifact.id !== 'string' || artifact.id <= previous) fail('contract-set artifacts must be sorted by id');
    previous = artifact.id;
    if (typeof artifact.path !== 'string' || artifact.path.startsWith('/') || artifact.path.split('/').some((part) => !part || part === '.' || part === '..')) fail(`contract artifact ${artifact.id} has an unsafe path`);
    positive(artifact.size_bytes, `contract artifact ${artifact.id} size`);
    match(artifact.sha256, SHA256, `contract artifact ${artifact.id} digest`);
    const bytes = await readFile(join(dirname(contractSetPath), artifact.path));
    if (bytes.length !== artifact.size_bytes || sha256(bytes) !== artifact.sha256) fail(`contract artifact ${artifact.id} bytes do not match its manifest`);
  }
}

export async function createCoreReleaseManifest({ contractSetManifestPath, outputRoot }) {
  if (!contractSetManifestPath || !outputRoot) fail('contract-set manifest path and output root are required');
  const { bytes: contractSetBytes, value: contractSet } = await readCanonical(contractSetManifestPath, 'contract-set manifest');
  exactKeys(contractSet, ['format', 'contract_set', 'release', 'capabilities', 'artifacts'], 'contract-set manifest');
  if (contractSet.format !== CONTRACT_SET_FORMAT || contractSet.contract_set !== CONTRACT_SET_NAME) fail('contract-set manifest identity is unsupported');
  validateRelease(contractSet.release);
  await validateArtifacts(contractSetManifestPath, contractSet);
  const contractSetDigest = sha256(contractSetBytes);
  const release = contractSet.release;
  const manifest = {
    manifest_version: 1,
    core_version: release.core_version,
    source_revision: release.source_revision,
    images: release.images,
    contracts: {
      ...release.contracts,
      collaboration: {
        format: CONTRACT_SET_FORMAT,
        contract_set: CONTRACT_SET_NAME,
        manifest_digest: contractSetDigest,
      },
    },
    deployment: release.deployment,
  };
  const manifestBytes = canonical(manifest);
  const manifestDigest = sha256(manifestBytes);
  const releaseRoot = join(resolve(outputRoot), 'releases', manifestDigest.slice('sha256:'.length));
  const manifestPath = join(releaseRoot, 'release-manifest.json');
  await writeImmutable(manifestPath, manifestBytes);
  return Object.freeze({ releaseRoot, manifestPath, manifestDigest, contractSetDigest });
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
  const allowed = new Set(['--contract-set-manifest', '--output']);
  for (const name of Object.keys(values)) if (!allowed.has(name)) fail(`unsupported argument ${name}`);
  for (const name of allowed) if (!values[name]) fail(`${name} is required`);
  return values;
}

async function main() {
  const args = cliArguments(process.argv.slice(2));
  const created = await createCoreReleaseManifest({
    contractSetManifestPath: resolve(args['--contract-set-manifest']),
    outputRoot: resolve(args['--output']),
  });
  process.stdout.write(`${JSON.stringify(created)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
