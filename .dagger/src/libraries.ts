import { randomUUID } from 'node:crypto';
import { dag, Container, Directory, Secret, ReturnType } from '@dagger.io/dagger';
import { assertScanPeriod, NODE_IMAGE, SYFT_IMAGE, GRYPE_IMAGE, ORAS_IMAGE, COSIGN_IMAGE, registryConfig, tool, EXCLUDES } from './publish.js';

export const LIBRARY_WORKFLOW = 'BlueprintLabIO/restless-core/.github/workflows/ui-artifact-release.yml@refs/heads/main';
/* Packages published before main became the release line were signed from dev (now locked);
 * reuse still trusts them. New packages are sealed only from main. */
const REUSABLE_LIBRARY_IDENTITY =
  '^https://github\\.com/BlueprintLabIO/restless-core/\\.github/workflows/ui-artifact-release\\.yml@refs/heads/(main|dev)$';
const REPOSITORY = 'ghcr.io/blueprintlabio/restless-core-release';
const checkKind = (kind: string) => {
  if (!['ui', 'office', 'issuer'].includes(kind)) throw new Error('choose ui, office or issuer');
};

function descriptor(oras: Container, reference: string): Container {
  // Mutable discovery tags must be observed now. In particular, an earlier
  // expected 404 must not remain cached after successful publication.
  return oras.withEnvVariable('RESTLESS_REGISTRY_OBSERVATION', randomUUID())
    .withExec(['manifest', 'fetch', '--descriptor', reference], {useEntrypoint: true, expect: ReturnType.Any});
}

async function libraryInput(source: Directory, kind: string): Promise<{input_sha256: string}> {
  checkKind(kind);
  const context = dag.container().from(NODE_IMAGE)
    .withFile('/src/LICENSE', source.file('LICENSE'))
    .withFile('/src/dagger.json', source.file('dagger.json'))
    .withDirectory('/src/web', source.directory('web'), {exclude: EXCLUDES})
    .withDirectory('/src/services/identity', source.directory('services/identity'), {exclude: EXCLUDES})
    .withDirectory('/src/scripts/release', source.directory('scripts/release'))
    .withDirectory('/src/.dagger', source.directory('.dagger'), {
      include: ['package.json', 'tsconfig.json', 'src/index.ts', 'src/libraries.ts', 'src/publish.ts'],
    })
    .withFile('/src/.github/workflows/ui-artifact-release.yml', source.file('.github/workflows/ui-artifact-release.yml'))
    .withWorkdir('/src');
  return JSON.parse(await context.withExec(['node', 'scripts/release/library-inputs.mjs', '.', kind]).stdout());
}

/** Reuse only a signed, fully qualified input marker from the current scan day.
 * A failed/unsigned earlier publication cannot suppress the next real build. */
export async function reuseLibrary(source: Directory, kind: string, scanPeriod: string,
  username: string, password: Secret): Promise<Directory | null> {
  assertScanPeriod(scanPeriod, 'library requires the current UTC scan day');
  const input = await libraryInput(source, kind);
  const config = await registryConfig(username, password);
  const oras = tool(ORAS_IMAGE, config);
  const tag = `${REPOSITORY}:${kind}-inputs-${input.input_sha256}-${scanPeriod}`;
  const lookup = descriptor(oras, tag);
  if (await lookup.exitCode() !== 0) {
    const error = await lookup.stderr();
    if (/(?:manifest[ _]unknown|name[ _]unknown|: not found|: 404)/i.test(error)) return null;
    throw new Error(`library input lookup failed: ${error}`);
  }
  const reference = `${REPOSITORY}@${JSON.parse(await lookup.stdout()).digest}`;
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error('library reuse requires an OCI digest');
  await tool(COSIGN_IMAGE, config).withExec(['verify', '--certificate-identity-regexp', REUSABLE_LIBRARY_IDENTITY,
    '--certificate-oidc-issuer', 'https://token.actions.githubusercontent.com', reference], {useEntrypoint: true}).sync();
  const payload = oras.withExec(['pull', reference, '--output', '/payload'], {useEntrypoint: true}).directory('/payload');
  const release = JSON.parse(await payload.file('library-release.json').contents());
  if (!/^[0-9a-f]{40}$/.test(release.source?.revision ?? '')) throw new Error('reused library has no exact Core source revision');
  await dag.container().from(NODE_IMAGE).withDirectory('/payload', payload)
    .withFile('/verify-library.mjs', source.file('scripts/release/verify-library.mjs'))
    .withExec(['node', '/verify-library.mjs', '/payload', kind, release.source.revision, scanPeriod, input.input_sha256]).sync();
  console.log(`reused signed ${kind} inputs ${input.input_sha256}: ${reference}; source ${release.source.revision}`);
  return payload.withNewFile('library-receipt.json', JSON.stringify({reference, component: kind,
    source_revision: release.source.revision, scan_period: scanPeriod, input_sha256: input.input_sha256, reused: true}, null, 2) + '\n');
}

/** Build and exercise an exact package without importing or building Rust images. */
export async function library(source: Directory, kind: string, revision: string, epoch: string, scanPeriod: string): Promise<Directory> {
  checkKind(kind);
  if (!/^[0-9a-f]{40}$/.test(revision) || !/^\d+$/.test(epoch)) throw new Error('library requires exact checkout revision/epoch');
  assertScanPeriod(scanPeriod, 'library requires the current UTC scan day');
  const input = await libraryInput(source, kind);
  let project = dag.container().from(NODE_IMAGE)
    .withEnvVariable('NODE_OPTIONS', '--no-network-family-autoselection')
    .withMountedCache('/root/.npm', dag.cacheVolume('restless-core-npm-v1'))
    .withFile('/src/web/package.json', source.file('web/package.json'))
    .withFile('/src/web/package-lock.json', source.file('web/package-lock.json'));
  if (kind !== 'issuer') project = project.withExec(['npm', '--prefix', '/src/web', 'ci', '--no-audit', '--no-fund']);
  project = project.withFile('/src/LICENSE', source.file('LICENSE'))
    .withDirectory('/src/web', source.directory('web'), { exclude: EXCLUDES })
    .withDirectory('/src/services/identity', source.directory('services/identity'), { exclude: EXCLUDES })
    .withDirectory('/src/scripts/release', source.directory('scripts/release')).withWorkdir('/src');
  if (kind !== 'issuer') project = project.withExec(['npm', '--prefix', 'web', 'run', 'check']);
  const checked = project.withExec(['node', 'scripts/release/pack-library.mjs', kind, revision, epoch, '/artifact'])
    .withExec(['node', 'scripts/release/check-library.mjs', kind, '/artifact']);
  const payload = checked.directory('/artifact').filter({
    include: ['*.tgz', '*.sha256', '*-manifest.json', 'dependencies/**', 'qualification.txt', 'npm-audit.json'],
  });
  const sbom = dag.container().from(SYFT_IMAGE).withDirectory('/scan', payload)
    .withExec(['dir:/scan', '--output', 'spdx-json'], { useEntrypoint: true, redirectStdout: '/sbom.spdx.json' });
  const scanned = dag.container().from(GRYPE_IMAGE).withFile('/scan/sbom.spdx.json', sbom.file('/sbom.spdx.json'))
    .withMountedCache('/cache', dag.cacheVolume('restless-core-grype-v0.119.0'))
    .withEnvVariable('GRYPE_DB_CACHE_DIR', '/cache').withEnvVariable('GRYPE_CHECK_FOR_APP_UPDATE', 'false')
    .withEnvVariable('RESTLESS_SCAN_PERIOD', scanPeriod)
    .withExec(['sbom:/scan/sbom.spdx.json', '--fail-on', 'high', '--output', 'json',
      '--file', '/grype.json'], { useEntrypoint: true, expect: ReturnType.Any });
  if (await scanned.exitCode() !== 0) {
    const report = JSON.parse(await scanned.file('/grype.json').contents());
    const findings = (report.matches ?? []).filter((m: any) => ['High', 'Critical'].includes(m.vulnerability?.severity))
      .slice(0, 15).map((m: any) => `${m.artifact.name} ${m.artifact.version}: ${m.vulnerability.id}`).join('\n');
    throw new Error(`${kind}: library scan failed\n${findings}\n${await scanned.stderr()}`);
  }
  const complete = checked.withDirectory('/artifact', payload)
    .withFile('/artifact/sbom.spdx.json', sbom.file('/sbom.spdx.json')).withFile('/artifact/grype.json', scanned.file('/grype.json'))
    .withExec(['node', 'scripts/release/library-release.mjs', '/artifact', kind, revision, scanPeriod, input.input_sha256]);
  return complete.directory('/artifact').filter({
    include: ['*.tgz', '*.sha256', '*-manifest.json', 'dependencies/**', 'qualification.txt', 'npm-audit.json',
      'sbom.spdx.json', 'grype.json', 'library-release.json'],
  });
}

/** Upload once by discovery tag, and return the exact immutable payload on retries. */
export async function publishLibrary(payload: Directory, kind: string, revision: string, scanPeriod: string,
  username: string, password: Secret, source: Directory): Promise<Directory> {
  checkKind(kind);
  const qualified = JSON.parse(await payload.file('library-release.json').contents());
  if (qualified.component !== kind || qualified.source?.revision !== revision || qualified.vulnerability_scan?.period !== scanPeriod) {
    throw new Error('library publication does not bind the qualified component/source/day');
  }
  const config = await registryConfig(username, password);
  const oras = tool(ORAS_IMAGE, config);
  const tag = `${REPOSITORY}:${kind}-${revision}-${scanPeriod}`;
  const lookup = descriptor(oras, tag);
  let reference: string;
  let actual: Directory;
  if (await lookup.exitCode() === 0) {
    reference = `${REPOSITORY}@${JSON.parse(await lookup.stdout()).digest}`;
    actual = oras.withExec(['pull', reference, '--output', '/payload'], { useEntrypoint: true }).directory('/payload');
    const old = JSON.parse(await actual.file('library-release.json').contents());
    if (old.component !== kind || old.source?.revision !== revision || old.archive?.sha256 !== qualified.archive.sha256
      || old.evidence?.['dependencies/package-lock.json'] !== qualified.evidence['dependencies/package-lock.json']
      || old.vulnerability_scan?.period !== scanPeriod) throw new Error('existing library differs; never overwrite it');
  } else {
    const error = await lookup.stderr();
    if (!/(?:manifest[ _]unknown|name[ _]unknown|: not found|: 404)/i.test(error)) throw new Error(`library registry lookup failed: ${error}`);
    const files = [qualified.archive.file, ...Object.keys(qualified.evidence), 'library-release.json'];
    const upload = oras.withDirectory('/upload', payload).withWorkdir('/upload');
    await upload.withExec(['push', tag, '--artifact-type', `application/vnd.restless.core.${kind}.v1`,
      ...files.map(file => `${file}:application/octet-stream`)], { useEntrypoint: true }).sync();
    reference = `${REPOSITORY}@${JSON.parse(await descriptor(oras, tag).stdout()).digest}`;
    actual = oras.withExec(['pull', reference, '--output', '/payload'], { useEntrypoint: true }).directory('/payload');
    const roundTrip = JSON.parse(await actual.file('library-release.json').contents());
    if (JSON.stringify(roundTrip) !== JSON.stringify(qualified)) throw new Error('library registry round-trip changed the release');
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error('library publication must return an OCI digest');
  await dag.container().from(NODE_IMAGE).withDirectory('/payload', actual)
    .withFile('/verify-library.mjs', source.file('scripts/release/verify-library.mjs'))
    .withExec(['node', '/verify-library.mjs', '/payload', kind, revision, scanPeriod]).sync();
  return actual.withNewFile('library-receipt.json', JSON.stringify({ reference, component: kind,
    source_revision: revision, scan_period: scanPeriod, input_sha256: qualified.input_sha256, reused: false }, null, 2) + '\n');
}

/** Seal the exact library with the established UI workflow's main OIDC identity. */
export async function sealLibrary(source: Directory, artifacts: Directory, username: string, password: Secret,
  oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string): Promise<Directory> {
  if (workflowRef !== LIBRARY_WORKFLOW || !/^https:\/\/[^/]+\.actions\.githubusercontent\.com\//.test(oidcRequestUrl)) {
    throw new Error('library sealing requires the main library workflow and its GitHub OIDC endpoint');
  }
  const receipt = JSON.parse(await artifacts.file('library-receipt.json').contents());
  const release = JSON.parse(await artifacts.file('library-release.json').contents());
  if (!/^[0-9a-f]{64}$/.test(release.input_sha256 ?? '') || receipt.input_sha256 !== release.input_sha256) {
    throw new Error('library receipt does not bind exact qualified inputs');
  }
  if (receipt.component !== release.component || receipt.source_revision !== release.source?.revision
    || !new RegExp(`^${REPOSITORY.replaceAll('.', '\\.')}@sha256:[0-9a-f]{64}$`).test(receipt.reference)) {
    throw new Error('library receipt does not bind the expected release');
  }
  await dag.container().from(NODE_IMAGE).withDirectory('/payload', artifacts)
    .withFile('/verify-library.mjs', source.file('scripts/release/verify-library.mjs'))
    .withExec(['node', '/verify-library.mjs', '/payload', receipt.component, receipt.source_revision, receipt.scan_period]).sync();
  const config = await registryConfig(username, password);
  const identity = ['--certificate-identity', `https://github.com/${LIBRARY_WORKFLOW}`,
    '--certificate-oidc-issuer', 'https://token.actions.githubusercontent.com'];
  const sealed = tool(COSIGN_IMAGE, config).withDirectory('/work', artifacts)
    .withEnvVariable('GITHUB_ACTIONS', 'true').withEnvVariable('CI', 'true')
    .withEnvVariable('ACTIONS_ID_TOKEN_REQUEST_URL', oidcRequestUrl)
    .withSecretVariable('ACTIONS_ID_TOKEN_REQUEST_TOKEN', oidcRequestToken)
    .withExec(['sign', '--yes', receipt.reference], { useEntrypoint: true })
    .withExec(['verify', ...identity, receipt.reference], { useEntrypoint: true })
    .withExec(['sign-blob', '--yes', '--bundle', '/work/library-receipt.sigstore.json', '/work/library-receipt.json'],
      { useEntrypoint: true, redirectStdout: '/tmp/library-receipt-signature' })
    .withExec(['verify-blob', '--bundle', '/work/library-receipt.sigstore.json', ...identity, '/work/library-receipt.json'], { useEntrypoint: true });
  await sealed.sync();
  // Publish the reusable input alias only after all qualification and signing
  // finished. Existing aliases are immutable and must identify these same bytes.
  const oras = tool(ORAS_IMAGE, config);
  const tag = `${REPOSITORY}:${release.component}-inputs-${release.input_sha256}-${release.vulnerability_scan.period}`;
  const lookup = descriptor(oras, tag);
  if (await lookup.exitCode() === 0) {
    if (`${REPOSITORY}@${JSON.parse(await lookup.stdout()).digest}` !== receipt.reference) throw new Error('existing library input alias differs; never overwrite it');
  } else {
    const error = await lookup.stderr();
    if (!/(?:manifest[ _]unknown|name[ _]unknown|: not found|: 404)/i.test(error)) throw new Error(`library alias lookup failed: ${error}`);
    await oras.withExec(['tag', receipt.reference, tag.slice(`${REPOSITORY}:`.length)], {useEntrypoint: true}).sync();
    if (`${REPOSITORY}@${JSON.parse(await descriptor(oras, tag).stdout()).digest}`
      !== receipt.reference) throw new Error('library input alias changed its digest');
  }
  console.log(`sealed ${receipt.component}: ${receipt.reference}`);
  return sealed.directory('/work');
}
