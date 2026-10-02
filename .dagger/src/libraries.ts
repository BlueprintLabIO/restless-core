import { dag, Directory, Secret, ReturnType } from '@dagger.io/dagger';
import { NODE_IMAGE, SYFT_IMAGE, GRYPE_IMAGE, ORAS_IMAGE, COSIGN_IMAGE, registryConfig, tool, EXCLUDES } from './publish.js';

export const LIBRARY_WORKFLOW = 'BlueprintLabIO/restless-core/.github/workflows/ui-artifact-release.yml@refs/heads/dev';
const REPOSITORY = 'ghcr.io/blueprintlabio/restless-core-release';
const checkKind = (kind: string) => {
  if (!['ui', 'office', 'issuer'].includes(kind)) throw new Error('choose ui, office or issuer');
};

/** Build and exercise an exact package without importing or building Rust images. */
export async function library(source: Directory, kind: string, revision: string, epoch: string, scanPeriod: string): Promise<Directory> {
  checkKind(kind);
  if (!/^[0-9a-f]{40}$/.test(revision) || !/^\d+$/.test(epoch)) throw new Error('library requires exact checkout revision/epoch');
  if (scanPeriod !== new Date().toISOString().slice(0, 10)) throw new Error('library requires the current UTC scan day');
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
    .withExec(['sbom:/scan/sbom.spdx.json', '--fail-on', 'high', '--output', 'json'],
      { useEntrypoint: true, redirectStdout: '/grype.json', expect: ReturnType.Any });
  if (await scanned.exitCode() !== 0) {
    const report = JSON.parse(await scanned.file('/grype.json').contents());
    const findings = (report.matches ?? []).filter((m: any) => ['High', 'Critical'].includes(m.vulnerability?.severity))
      .slice(0, 15).map((m: any) => `${m.artifact.name} ${m.artifact.version}: ${m.vulnerability.id}`).join('\n');
    throw new Error(`${kind}: library scan failed\n${findings}\n${await scanned.stderr()}`);
  }
  const complete = checked.withDirectory('/artifact', payload)
    .withFile('/artifact/sbom.spdx.json', sbom.file('/sbom.spdx.json')).withFile('/artifact/grype.json', scanned.file('/grype.json'))
    .withExec(['node', 'scripts/release/library-release.mjs', '/artifact', kind, revision, scanPeriod]);
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
  const lookup = oras.withExec(['manifest', 'fetch', '--descriptor', tag], { useEntrypoint: true, expect: ReturnType.Any });
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
    reference = `${REPOSITORY}@${JSON.parse(await oras.withExec(['manifest', 'fetch', '--descriptor', tag], { useEntrypoint: true }).stdout()).digest}`;
    actual = oras.withExec(['pull', reference, '--output', '/payload'], { useEntrypoint: true }).directory('/payload');
    const roundTrip = JSON.parse(await actual.file('library-release.json').contents());
    if (JSON.stringify(roundTrip) !== JSON.stringify(qualified)) throw new Error('library registry round-trip changed the release');
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error('library publication must return an OCI digest');
  await dag.container().from(NODE_IMAGE).withDirectory('/payload', actual)
    .withFile('/verify-library.mjs', source.file('scripts/release/verify-library.mjs'))
    .withExec(['node', '/verify-library.mjs', '/payload', kind, revision, scanPeriod]).sync();
  return actual.withNewFile('library-receipt.json', JSON.stringify({ reference, component: kind,
    source_revision: revision, scan_period: scanPeriod }, null, 2) + '\n');
}

/** Seal the exact library with the established UI workflow's dev OIDC identity. */
export async function sealLibrary(source: Directory, artifacts: Directory, username: string, password: Secret,
  oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string): Promise<Directory> {
  if (workflowRef !== LIBRARY_WORKFLOW || !/^https:\/\/[^/]+\.actions\.githubusercontent\.com\//.test(oidcRequestUrl)) {
    throw new Error('library sealing requires the dev library workflow and its GitHub OIDC endpoint');
  }
  const receipt = JSON.parse(await artifacts.file('library-receipt.json').contents());
  const release = JSON.parse(await artifacts.file('library-release.json').contents());
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
  console.log(`sealed ${receipt.component}: ${receipt.reference}`);
  return sealed.directory('/work');
}
