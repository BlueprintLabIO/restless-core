#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { mkdir, open, readFile } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const FORMAT = 'restless.core.release-bundle.v1';
const SOURCE_REVISION = /^[0-9a-f]{40}$/;
const DIGEST = /^sha256:[0-9a-f]{64}$/;
const OCI_DIGEST = /^(?:[a-z0-9.-]+(?::[0-9]+)?\/)[a-z0-9._/-]+@sha256:[0-9a-f]{64}$/;
const REPOSITORY = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;

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

function safeRelative(root, path, name) {
  const absolute = resolve(path);
  const within = relative(resolve(root), absolute);
  if (isAbsolute(within) || within === '..' || within.startsWith(`..${sep}`) || within === '') {
    fail(`${name} must be a file below the bundle root`);
  }
  return within.split(sep).join('/');
}

async function readCanonical(path, name) {
  const bytes = await readFile(path);
  let value;
  try {
    value = JSON.parse(bytes);
  } catch (error) {
    fail(`${name} is not JSON: ${error.message}`);
  }
  if (!canonical(value).equals(bytes)) fail(`${name} is not canonical two-space JSON`);
  return { bytes, value };
}

async function readSignature(path, name) {
  const bytes = await readFile(path);
  try {
    const value = JSON.parse(bytes);
    if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`${name} must be a JSON object`);
  } catch (error) {
    fail(`${name} is not a Sigstore JSON bundle: ${error.message}`);
  }
  return bytes;
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

async function verifyContractArtifacts(contractPath, contract) {
  if (!Array.isArray(contract.artifacts) || contract.artifacts.length === 0) {
    fail('contract-set artifacts are missing');
  }
  let previous = '';
  for (const artifact of contract.artifacts) {
    exactKeys(artifact, ['id', 'path', 'media_type', 'size_bytes', 'sha256'], `contract artifact ${artifact?.id ?? '?'}`);
    if (typeof artifact.id !== 'string' || artifact.id <= previous) fail('contract-set artifacts must be sorted by id');
    previous = artifact.id;
    match(artifact.sha256, DIGEST, `contract artifact ${artifact.id} digest`);
    if (!Number.isSafeInteger(artifact.size_bytes) || artifact.size_bytes < 1) fail(`contract artifact ${artifact.id} size is invalid`);
    const artifactPath = resolve(dirname(contractPath), artifact.path);
    safeRelative(dirname(contractPath), artifactPath, `contract artifact ${artifact.id}`);
    const bytes = await readFile(artifactPath);
    if (bytes.length !== artifact.size_bytes || sha256(bytes) !== artifact.sha256) {
      fail(`contract artifact ${artifact.id} bytes do not match its manifest`);
    }
  }
}

export async function createCoreReleaseBundle({
  bundleRoot,
  releaseManifestPath,
  contractSetPath,
  releaseSignaturePath,
  contractSignaturePath,
  repository,
  workflowRef,
  outputPath,
}) {
  const root = resolve(bundleRoot);
  match(repository, REPOSITORY, 'repository');
  const workflowIdentity = `${repository}/.github/workflows/immutable-core-release.yml@`;
  if (typeof workflowRef !== 'string' || !workflowRef.startsWith(workflowIdentity)
      || !workflowRef.slice(workflowIdentity.length).startsWith('refs/')) {
    fail('workflow ref must be the immutable Core release workflow identity at a Git ref');
  }
  const releaseRelative = safeRelative(root, releaseManifestPath, 'release manifest');
  const contractRelative = safeRelative(root, contractSetPath, 'contract-set manifest');
  const releaseSignatureRelative = safeRelative(root, releaseSignaturePath, 'release-manifest signature');
  const contractSignatureRelative = safeRelative(root, contractSignaturePath, 'contract-set signature');
  const outputRelative = safeRelative(root, outputPath, 'bundle index');
  if (outputRelative !== 'core-release-bundle.json') fail('bundle index path must be core-release-bundle.json');

  const [{ bytes: releaseBytes, value: release }, { bytes: contractBytes, value: contract }, releaseSignature, contractSignature] =
    await Promise.all([
      readCanonical(releaseManifestPath, 'release manifest'),
      readCanonical(contractSetPath, 'contract-set manifest'),
      readSignature(releaseSignaturePath, 'release-manifest signature'),
      readSignature(contractSignaturePath, 'contract-set signature'),
    ]);
  exactKeys(release, ['manifest_version', 'core_version', 'source_revision', 'images', 'contracts', 'deployment'], 'release manifest');
  if (release.manifest_version !== 1) fail('release manifest version is unsupported');
  match(release.source_revision, SOURCE_REVISION, 'release source revision');
  exactKeys(release.images, ['account_plane', 'company_runtime', 'native_documents'], 'release images');
  for (const [name, image] of Object.entries(release.images)) match(image, OCI_DIGEST, `release image ${name}`);
  const expectedImages = {
    account_plane: 'ghcr.io/blueprintlabio/restless-account-plane@',
    company_runtime: 'ghcr.io/blueprintlabio/restless-company-runtime@',
    native_documents: 'ghcr.io/blueprintlabio/restless-native-documents-collaboration@',
  };
  for (const [name, prefix] of Object.entries(expectedImages)) {
    if (!release.images[name].startsWith(prefix)) fail(`release image ${name} is not from its canonical repository`);
  }
  exactKeys(contract, ['format', 'contract_set', 'release', 'capabilities', 'artifacts'], 'contract-set manifest');
  if (contract.format !== 'restless.core.contract-set.v1' || contract.contract_set !== 'company-collaboration') {
    fail('contract-set identity is unsupported');
  }
  const contractDigest = sha256(contractBytes);
  if (release.contracts?.collaboration?.manifest_digest !== contractDigest) {
    fail('release manifest does not bind the exact contract-set bytes');
  }
  const expectedContractRelease = {
    core_version: release.core_version,
    source_revision: release.source_revision,
    images: release.images,
    contracts: {
      api: release.contracts.api,
      identity_assertion: release.contracts.identity_assertion,
      schema: release.contracts.schema,
    },
    deployment: release.deployment,
  };
  if (JSON.stringify(contract.release) !== JSON.stringify(expectedContractRelease)) {
    fail('contract-set release tuple differs from the release manifest');
  }
  await verifyContractArtifacts(contractSetPath, contract);
  const descriptor = contract.artifacts.find(({ id }) =>
    id === 'restless.core.native-documents-collaboration.deployment');
  if (!descriptor) fail('contract set has no native Documents hosting descriptor');
  const descriptorValue = (await readCanonical(resolve(dirname(contractSetPath), descriptor.path), 'native Documents descriptor')).value;
  if (descriptorValue.image !== release.images.native_documents) {
    fail('native Documents descriptor image differs from the release manifest');
  }

  const index = {
    format: FORMAT,
    source: { repository, revision: release.source_revision },
    workflow: { ref: workflowRef },
    images: release.images,
    release_manifest: {
      path: releaseRelative,
      sha256: sha256(releaseBytes),
      signature: { path: releaseSignatureRelative, sha256: sha256(releaseSignature) },
    },
    contract_set: {
      path: contractRelative,
      sha256: contractDigest,
      signature: { path: contractSignatureRelative, sha256: sha256(contractSignature) },
    },
  };
  const bytes = canonical(index);
  await writeImmutable(outputPath, bytes);
  return Object.freeze({ indexPath: resolve(outputPath), indexDigest: sha256(bytes) });
}

function argumentsFrom(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (!name?.startsWith('--') || value === undefined) fail('arguments must be --name value pairs');
    if (Object.hasOwn(values, name)) fail(`duplicate argument ${name}`);
    values[name] = value;
  }
  const names = ['--bundle-root', '--release-manifest', '--contract-set', '--release-signature', '--contract-signature', '--repository', '--workflow-ref', '--output'];
  for (const name of Object.keys(values)) if (!names.includes(name)) fail(`unsupported argument ${name}`);
  for (const name of names) if (!values[name]) fail(`${name} is required`);
  return values;
}

async function main() {
  const args = argumentsFrom(process.argv.slice(2));
  const result = await createCoreReleaseBundle({
    bundleRoot: args['--bundle-root'],
    releaseManifestPath: args['--release-manifest'],
    contractSetPath: args['--contract-set'],
    releaseSignaturePath: args['--release-signature'],
    contractSignaturePath: args['--contract-signature'],
    repository: args['--repository'],
    workflowRef: args['--workflow-ref'],
    outputPath: args['--output'],
  });
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
