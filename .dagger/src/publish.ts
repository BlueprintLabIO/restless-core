import { createHash } from 'node:crypto';
import { dag, Container, Directory, Platform, Secret, ReturnType } from '@dagger.io/dagger';

export const NODE_IMAGE = 'node:24.18.1-alpine3.23@sha256:c2cc26d8f991c2db236ad51a61efee843c482372d6d22570787309d511694110';
export const GRYPE_IMAGE = 'anchore/grype:v0.119.0@sha256:8c2c9234a345577a6d321a4753aa3ee1276d8975c8452d2344a56b57733ecad3';
export const SYFT_IMAGE = 'anchore/syft:v1.54.0@sha256:0356562f495d432056237fbea5cbc2d4839c9c75cd500784a66de2e7cc95ca7c';
export const ORAS_IMAGE = 'ghcr.io/oras-project/oras:v1.2.2@sha256:cd549d80c4aa89638aea5964a3cd8193a6dd8abf939a43b5d562c24dbab08ff1';
export const COSIGN_IMAGE = 'ghcr.io/sigstore/cosign/cosign:v2.4.1@sha256:b03690aa52bfe94054187142fba24dc54137650682810633901767d8a3e15b31';
export const CORE_WORKFLOW = 'BlueprintLabIO/restless-core/.github/workflows/immutable-core-release.yml@refs/heads/main';
export const EXCLUDES = ['**/.git/**', '**/node_modules/**', '**/target/**', '**/build/**', '**/.svelte-kit/**', '**/dist/**', '**/.env', '**/.env.*', '**/__pycache__/**'];
const hash = (value: string) => createHash('sha256').update(value).digest('hex');

async function inspectPlatform(reference: string, platform: string, oras: Container, flags: string[] = []): Promise<void> {
  const manifest = JSON.parse(await oras.withExec(['manifest', 'fetch', ...flags, reference], { useEntrypoint: true }).stdout());
  if (!manifest.config?.digest || manifest.manifests) throw new Error('expected one platform-scoped image manifest');
  const repository = reference.split('@')[0];
  const inspected = oras.withExec(['blob', 'fetch', ...flags, '--output', '/image-config.json', `${repository}@${manifest.config.digest}`],
    { useEntrypoint: true });
  const config = JSON.parse(await inspected.file('/image-config.json').contents());
  if (`${config.os}/${config.architecture}` !== platform) throw new Error(`registry image is not ${platform}`);
}

/** Exercise the pinned ORAS commands against a real isolated image registry. */
export async function verifyImageInspection(): Promise<void> {
  const registry = dag.container().from('registry:3.0.0@sha256:6c5666b861f3505b116bb9aa9b25175e71210414bd010d92035ff64018f9457e')
    .withExposedPort(5000).asService();
  try {
    const image = dag.container().from(NODE_IMAGE);
    // Loopback discovery permits HTTP for this isolated test service. Clients
    // use its service binding; production keeps the canonical TLS GHCR origin.
    const published = await image.publish('localhost:5000/core-image-check:qualified', { registryService: registry });
    const reference = published.replace(/^localhost:5000\//, 'registry:5000/');
    const oras = dag.container().from(ORAS_IMAGE).withServiceBinding('registry', registry);
    await inspectPlatform(reference, await image.platform(), oras, ['--plain-http']);
    let rejected = false;
    try { await inspectPlatform(reference, 'linux/invalid', oras, ['--plain-http']); }
    catch (error) {
      if (!String(error).includes('registry image is not linux/invalid')) throw error;
      rejected = true;
    }
    if (!rejected) throw new Error('image inspection accepted a mismatched platform');
    console.log('Actual registry image/config inspection passed; mismatched platform rejected');
  } finally { await registry.stop(); }
}

async function inputIdentity(context: Directory, platform: string): Promise<string> {
  return hash(JSON.stringify({ recipe: 'restless-core-dagger-0.21.10-v1', platform,
    source: await context.withTimestamps(0).digest() }));
}

/** A fresh checkout's timestamps must not invalidate an interrupted release. */
export async function verifyBuildInputs(): Promise<void> {
  const context = dag.directory().withNewFile('Dockerfile', 'FROM scratch\n');
  const original = await inputIdentity(context.withTimestamps(100), 'linux/amd64');
  if (original !== await inputIdentity(context.withTimestamps(200), 'linux/amd64')
    || original === await inputIdentity(context.withNewFile('Dockerfile', 'FROM scratch\nLABEL changed=true\n'), 'linux/amd64')
    || original === await inputIdentity(context, 'linux/arm64')) {
    throw new Error('Core publication inputs must ignore timestamps and bind bytes/platform');
  }
}

export async function registryConfig(username: string, password: Secret): Promise<Secret> {
  const auth = Buffer.from(`${username}:${await password.plaintext()}`).toString('base64');
  return dag.setSecret('restless-core-registry-config', JSON.stringify({ auths: { 'ghcr.io': { auth } } }));
}

export function tool(image: string, config: Secret): Container {
  return dag.container().from(image).withUser('0').withMountedSecret('/auth/config.json', config)
    .withEnvVariable('DOCKER_CONFIG', '/auth').withEnvVariable('HOME', '/tmp');
}

export function node(source: Directory): Container {
  return dag.container().from(NODE_IMAGE).withDirectory('/src', source, { exclude: EXCLUDES }).withWorkdir('/src');
}

function scanner(image: string, prefix: string, username: string, password: Secret): Container {
  return dag.container().from(image).withEnvVariable(`${prefix}_REGISTRY_AUTH_AUTHORITY`, 'ghcr.io')
    .withEnvVariable(`${prefix}_REGISTRY_AUTH_USERNAME`, username).withSecretVariable(`${prefix}_REGISTRY_AUTH_PASSWORD`, password);
}

/**
 * Push to GHCR, retrying a dropped connection. Already uploaded blobs are
 * skipped on the next attempt, so a retry only resends what was interrupted.
 */
async function publishWithRetry(image: Container, tag: string): Promise<string> {
  for (let attempt = 1; ; attempt += 1) {
    try {
      return await image.publish(tag);
    } catch (error) {
      const transient = /(?:timed out|timeout|connection reset|broken pipe|unexpected EOF|network is unreachable|: 5\d\d)/i;
      if (attempt >= 3 || !transient.test(String(error))) throw error;
      console.log(`${tag}: push interrupted (attempt ${attempt}), retrying`);
    }
  }
}

/** Reuse a partial upload, then qualify the exact image before it can be sealed. */
export async function publishImage(component: string, revision: string, platform: Platform, context: Directory,
  factory: () => Promise<Container>, verify: (image: Container) => Promise<void>, username: string, password: Secret,
  scanPeriod: string): Promise<Directory> {
  if (scanPeriod !== new Date().toISOString().slice(0, 10)) throw new Error('Core scans must use the current UTC vulnerability refresh day');
  const repository = `ghcr.io/blueprintlabio/restless-${component}`;
  const input = await inputIdentity(context, platform);
  const tag = `${repository}:${component === 'runtime-tools' ? `inputs-${input}` : revision}-${platform.split('/')[1]}`;
  let image: Container;
  let reference: string;
  let reused = false;
  try {
    image = dag.container({ platform }).withRegistryAuth('ghcr.io', username, password).from(tag);
    reference = `${repository}@${(await image.imageRef()).split('@')[1]}`;
    reused = true;
  } catch (error) {
    if (!/(?:manifest unknown|manifest_unknown|name_unknown|: not found|: 404)/i.test(String(error))) throw error;
    image = (await factory()).withLabel('io.restless.build-input-sha256', input).withLabel('io.restless.component', component)
      .withLabel('org.opencontainers.image.revision', revision).withRegistryAuth('ghcr.io', username, password);
    reference = `${repository}@${(await publishWithRetry(image, tag)).split('@')[1]}`;
    image = dag.container({ platform }).withRegistryAuth('ghcr.io', username, password).from(reference);
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error(`${component}: registry returned a mutable reference`);
  const [actualInput, actualComponent, actualRevision, actualSource] = await Promise.all([
    image.label('io.restless.build-input-sha256'), image.label('io.restless.component'),
    image.label('org.opencontainers.image.revision'), image.label('org.opencontainers.image.source'),
  ]);
  if (actualInput !== input || actualComponent !== component || actualSource !== 'https://github.com/BlueprintLabIO/restless-core'
    || !/^[0-9a-f]{40}$/.test(actualRevision) || (component !== 'runtime-tools' && actualRevision !== revision)) {
    throw new Error(`${component}: existing artifact does not bind the requested inputs/source`);
  }
  await verify(image);
  const config = await registryConfig(username, password);
  // Inspect actual registry config, not a requested SDK platform hint.
  await inspectPlatform(reference, platform, tool(ORAS_IMAGE, config));
  // Catalogue the exact image once. Include producer CycloneDX records: newer
  // Chrome binaries no longer match Syft's binary-string version classifier.
  // Grype consumes this same inventory rather than recataloguing the image.
  const sbom = scanner(SYFT_IMAGE, 'SYFT', username, password).withNewFile('/reports/.keep', '')
    .withExec([`registry:${reference}`, '--override-default-catalogers', 'image',
      '--override-default-catalogers', 'sbom-cataloger', '--output', 'syft-json=/reports/inventory.json',
      '--output', 'spdx-json=/reports/sbom.json'], { useEntrypoint: true });
  if (['runtime-tools', 'company-runtime'].includes(component)) {
    const inventory = JSON.parse(await sbom.file('/reports/inventory.json').contents());
    const browser = JSON.parse(await image.file('/usr/local/share/restless/browser.cdx.json').contents()).components?.[0];
    const expectedCpe = `cpe:2.3:a:google:chrome:${browser?.version}:*:*:*:*:*:*:*`;
    const found = (inventory.artifacts ?? []).filter((p: any) => p.name === 'chrome');
    if (browser?.name !== 'chrome' || !/^\d+\.\d+\.\d+\.\d+$/.test(browser.version)
      || browser.cpe !== expectedCpe || found.some((p: any) => p.version !== browser.version)
      || !found.some((p: any) => p.cpes?.some((c: any) => c.cpe === expectedCpe))) {
      throw new Error(`${component}: software inventory does not cover the installed Chrome version and CPE`);
    }
  }
  const scan = scanner(GRYPE_IMAGE, 'GRYPE', username, password)
    .withMountedCache('/cache', dag.cacheVolume('restless-core-grype-v0.119.0'))
    .withEnvVariable('GRYPE_DB_CACHE_DIR', '/cache').withEnvVariable('GRYPE_CHECK_FOR_APP_UPDATE', 'false')
    .withEnvVariable('RESTLESS_SCAN_PERIOD', scanPeriod)
    .withFile('/reports/inventory.json', sbom.file('/reports/inventory.json'))
    // Ask Grype to own the report file. Dagger's progress stream can still
    // mirror redirected stdout, which previously inflated one Actions log by
    // tens of megabytes and obscured diagnosis of an unrelated runner hang.
    .withExec(['sbom:/reports/inventory.json', '--fail-on', 'high', '--output', 'json',
      '--file', '/reports/scan.json'], { useEntrypoint: true, expect: ReturnType.Any });
  const status = await scan.exitCode();
  if (status !== 0) {
    let findings = '';
    if (status === 2) {
      const report = JSON.parse(await scan.file('/reports/scan.json').contents());
      findings = (report.matches ?? []).filter((m: any) => ['High', 'Critical'].includes(m.vulnerability?.severity)).slice(0, 20)
        .map((m: any) => `${m.artifact.name} ${m.artifact.version}: ${m.vulnerability.id} (${m.vulnerability.severity}); fix: ${(m.vulnerability.fix?.versions ?? []).join(', ') || 'unavailable'}`).join('\n');
    }
    throw new Error(`${component}: exact Core artifact scan failed (exit ${status})\n${findings}\n${await scan.stderr()}`);
  }
  const [scanBytes, sbomBytes] = await Promise.all([scan.file('/reports/scan.json').contents(), sbom.file('/reports/sbom.json').contents()]);
  const provenance = { buildDefinition: { buildType: 'https://github.com/BlueprintLabIO/restless-core/tree/main/.dagger',
    externalParameters: { component, platform, source: { repository: 'BlueprintLabIO/restless-core', revision: actualRevision }, input_sha256: input },
    resolvedDependencies: [{ uri: `git+https://github.com/BlueprintLabIO/restless-core@${actualRevision}`, digest: { gitCommit: actualRevision } }] },
    runDetails: { builder: { id: `https://github.com/${CORE_WORKFLOW}` } } };
  console.log(`${component}: ${reused ? 'reused' : 'built'} ${reference}; ${platform}; scan passed`);
  return dag.directory().withNewFile(`images/${component}.json`, `${JSON.stringify({ component, reference, platform,
    source_revision: actualRevision, input_sha256: input, scan_sha256: hash(scanBytes), sbom_sha256: hash(sbomBytes) }, null, 2)}\n`)
    .withFile(`scans/${component}.json`, scan.file('/reports/scan.json')).withFile(`sboms/${component}.json`, sbom.file('/reports/sbom.json'))
    .withNewFile(`provenance/${component}.json`, `${JSON.stringify(provenance, null, 2)}\n`);
}
