import { dag, Directory, Secret, ReturnType } from '@dagger.io/dagger';
import { COSIGN_HTTP, COSIGN_IMAGE, ORAS_HTTP, ORAS_IMAGE, RELEASE_BUILDER, RELEASE_KEY, STAGING_REGISTRY, node, publicReference,
  registryConfig, timed, timingsFile, tool, withRetry, type Timing } from './publish.js';

const REPOSITORY = `${STAGING_REGISTRY}/blueprintlabio/restless-core-release`;
/** A signing call normally takes seconds; one that hangs fails here and is retried, not after cosign's 3 minutes. */
const COSIGN_TIMEOUT = ['--timeout', '90s'];

/**
 * Seal exact images, SLSA/SPDX evidence and the existing Core bundle contract with the release key
 * (ADR 0010 in restless-cloud): images are signed and attested in staging, where the pipeline
 * pushed them, and the release names their public copies, which the mirror puts in R2 under the
 * same digests. Signatures are recorded in Rekor.
 */
export async function sealRelease(source: Directory, published: Directory, revision: string, platform: string,
  username: string, password: Secret, signingKey: Secret, signingPassword: Secret): Promise<Directory> {
  // Publish timings are run evidence for the summary, never part of the sealed composition.
  const staged = published.withoutDirectory('timings');
  const config = await registryConfig(username, password);
  const identity = ['--key', '/keys/signing.pub'];
  const key = ['--key', 'env://COSIGN_PRIVATE_KEY'];
  const cosign = tool(COSIGN_IMAGE, config).withDirectory('/work', staged)
    .withFile('/keys/signing.pub', source.file(RELEASE_KEY))
    .withSecretVariable('COSIGN_PRIVATE_KEY', signingKey).withSecretVariable('COSIGN_PASSWORD', signingPassword);
  const timings: Timing[] = [];
  // Each attempt is its own Dagger operation, so a retry runs again instead of replaying a cached failure.
  // Re-signing an image that an interrupted attempt already signed only adds an equivalent signature.
  const attempt = (n: number) => cosign.withEnvVariable('RESTLESS_SEAL_ATTEMPT', String(n));
  const components = ['runtime-tools', 'account-plane', 'company-runtime', 'native-documents-collaboration', 'cockpit'];
  // The images are independent: sign, attest and verify them concurrently.
  await timed(timings, 'sign and attest images', () => Promise.all(components.map(async (component) => {
    const record = JSON.parse(await staged.file(`images/${component}.json`).contents());
    await timed(timings, `sign ${component}`, () => withRetry(`${component}: seal`, (n) => attempt(n)
      .withExec(['sign', '--yes', ...key, ...COSIGN_HTTP, ...COSIGN_TIMEOUT, record.reference], { useEntrypoint: true })
      .withExec(['verify', ...identity, ...COSIGN_HTTP, record.reference], { useEntrypoint: true })
      .withExec(['attest', '--yes', ...key, ...COSIGN_HTTP, ...COSIGN_TIMEOUT, '--type', 'https://slsa.dev/provenance/v1', '--predicate', `/work/provenance/${component}.json`, record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-provenance-signature` })
      .withExec(['verify-attestation', ...identity, ...COSIGN_HTTP, '--type', 'https://slsa.dev/provenance/v1', record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-verified-provenance` })
      .withExec(['attest', '--yes', ...key, ...COSIGN_HTTP, ...COSIGN_TIMEOUT, '--type', 'spdxjson', '--predicate', `/work/sboms/${component}.json`, record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-sbom-signature` })
      .withExec(['verify-attestation', ...identity, ...COSIGN_HTTP, '--type', 'spdxjson', record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-verified-sbom` }).sync()));
  })));
  // The release names the public copies: same digests, the address the world pulls from.
  let artifacts = staged;
  for (const component of [...components, ...(await staged.directory('images').entries())
    .map((name) => name.replace(/\.json$/, '')).filter((name) => !components.includes(name))]) {
    const record = JSON.parse(await staged.file(`images/${component}.json`).contents());
    artifacts = artifacts.withNewFile(`images/${component}.json`,
      `${JSON.stringify({ ...record, reference: publicReference(record.reference) }, null, 2)}\n`);
  }
  const assembled = node(source).withDirectory('/work', artifacts)
    .withExec(['node', 'scripts/release/assemble.mjs', '/src', '/work', revision, platform]);
  const assembly = JSON.parse(await assembled.file('/work/assembly.json').contents());
  const tag = `${REPOSITORY}:${revision}-${platform.split('/')[1]}`;
  const oras = tool(ORAS_IMAGE, config);
  const lookup = oras.withExec(['manifest', 'fetch', ...ORAS_HTTP, '--descriptor', tag], { useEntrypoint: true, expect: ReturnType.Any });
  let payload: Directory;
  let reference: string;
  if (await lookup.exitCode() === 0) {
    const descriptor = JSON.parse(await lookup.stdout());
    reference = `${REPOSITORY}@${descriptor.digest}`;
    const pulled = oras.withExec(['pull', ...ORAS_HTTP, reference, '--output', '/download'], { useEntrypoint: true });
    const unpacked = node(source).withExec(['apk', 'add', '--no-cache', 'tar'])
      .withDirectory('/download', pulled.directory('/download')).withNewFile('/work/.keep', '')
      .withExec(['tar', '-xzf', '/download/core-release-bundle.tar.gz', '--no-same-owner', '--no-same-permissions', '-C', '/work']);
    payload = unpacked.directory('/work');
    const existing = JSON.parse(await payload.file('assembly.json').contents());
    if (existing.revision !== revision || existing.platform !== platform
      || existing.release_manifest_digest !== assembly.release_manifest_digest
      || JSON.stringify(existing.images) !== JSON.stringify(assembly.images)) {
      throw new Error('existing Core bundle differs from the qualified composition; never overwrite it');
    }
    for (const field of ['release_manifest', 'contract_manifest']) {
      if (await payload.file(existing[field]).contents() !== await assembled.file(`/work/${assembly[field]}`).contents()) {
        throw new Error(`existing Core bundle ${field} differs from the qualified bytes`);
      }
    }
    let verified = cosign.withDirectory('/existing', payload);
    for (const [document, signature] of [[existing.release_manifest, 'release-manifest.sigstore.json'],
      [existing.contract_manifest, 'contract-set.sigstore.json'], ['core-release-bundle.json', 'core-release-bundle.sigstore.json']]) {
      verified = verified.withExec(['verify-blob', '--bundle', `/existing/${signature}`, ...identity, `/existing/${document}`], { useEntrypoint: true });
    }
    await timed(timings, 'verify existing bundle', () => withRetry('existing bundle: verify', () => verified.sync()));
  } else {
    const error = await lookup.stderr();
    if (!/(?:manifest[ _]unknown|name[ _]unknown|: not found|: 404)/i.test(error)) throw new Error(`Core bundle registry lookup failed: ${error}`);
    const documents = await timed(timings, 'sign release documents', () => withRetry('release documents: seal', async (n) => {
      const signed = attempt(n).withDirectory('/work', assembled.directory('/work'))
        .withExec(['sign-blob', '--yes', ...key, ...COSIGN_TIMEOUT, '--bundle', '/work/release-manifest.sigstore.json', `/work/${assembly.release_manifest}`],
          { useEntrypoint: true, redirectStdout: '/tmp/release-signature' })
        .withExec(['verify-blob', '--bundle', '/work/release-manifest.sigstore.json', ...identity, `/work/${assembly.release_manifest}`], { useEntrypoint: true })
        .withExec(['sign-blob', '--yes', ...key, ...COSIGN_TIMEOUT, '--bundle', '/work/contract-set.sigstore.json', `/work/${assembly.contract_manifest}`],
          { useEntrypoint: true, redirectStdout: '/tmp/contract-signature' })
        .withExec(['verify-blob', '--bundle', '/work/contract-set.sigstore.json', ...identity, `/work/${assembly.contract_manifest}`], { useEntrypoint: true });
      await signed.sync();
      return signed;
    }));
    const indexed = node(source).withDirectory('/work', documents.directory('/work'))
      .withExec(['node', 'scripts/create-core-release-bundle.mjs', '--bundle-root', '/work',
        '--release-manifest', `/work/${assembly.release_manifest}`, '--contract-set', `/work/${assembly.contract_manifest}`,
        '--release-signature', '/work/release-manifest.sigstore.json', '--contract-signature', '/work/contract-set.sigstore.json',
        '--repository', 'BlueprintLabIO/restless-core', '--workflow-ref', RELEASE_BUILDER, '--output', '/work/core-release-bundle.json']);
    const signedIndex = await timed(timings, 'sign bundle index', () => withRetry('bundle index: seal', async (n) => {
      const signed = attempt(n).withDirectory('/work', indexed.directory('/work'))
        .withExec(['sign-blob', '--yes', ...key, ...COSIGN_TIMEOUT, '--bundle', '/work/core-release-bundle.sigstore.json', '/work/core-release-bundle.json'],
          { useEntrypoint: true, redirectStdout: '/tmp/index-signature' })
        .withExec(['verify-blob', '--bundle', '/work/core-release-bundle.sigstore.json', ...identity, '/work/core-release-bundle.json'], { useEntrypoint: true });
      await signed.sync();
      return signed;
    }));
    payload = signedIndex.directory('/work');
    const archive = node(source).withExec(['apk', 'add', '--no-cache', 'tar'])
      .withDirectory('/work', payload).withExec(['tar', '--sort=name', '--mtime=UTC 1970-01-01', '--owner=0', '--group=0', '--numeric-owner',
        '-czf', '/core-release-bundle.tar.gz', '-C', '/work', '.']);
    await timed(timings, 'push bundle', () => withRetry('bundle: push', (n) => oras.withEnvVariable('RESTLESS_SEAL_ATTEMPT', String(n))
      .withFile('/upload/core-release-bundle.tar.gz', archive.file('/core-release-bundle.tar.gz')).withWorkdir('/upload')
      .withExec(['push', ...ORAS_HTTP, tag, '--artifact-type', 'application/vnd.restless.core.release-bundle.v1',
        'core-release-bundle.tar.gz:application/vnd.restless.core.release-bundle.v1+tar+gzip'], { useEntrypoint: true }).sync()));
    const descriptor = JSON.parse(await oras.withExec(['manifest', 'fetch', ...ORAS_HTTP, '--descriptor', tag], { useEntrypoint: true }).stdout());
    reference = `${REPOSITORY}@${descriptor.digest}`;
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error('Core publication requires an immutable OCI bundle');
  const handoff = { format: 'restless.core.release-handoff.v1', source: { repository: 'BlueprintLabIO/restless-core', revision },
    workflow: { ref: RELEASE_BUILDER }, bundle: publicReference(reference), release_manifest_digest: assembly.release_manifest_digest };
  const complete = await timed(timings, 'sign bundle and handoff', () => withRetry('bundle: seal', async (n) => {
    const signed = attempt(n).withDirectory('/work', payload)
      .withExec(['sign', '--yes', ...key, ...COSIGN_HTTP, ...COSIGN_TIMEOUT, reference], { useEntrypoint: true })
      .withExec(['verify', ...identity, ...COSIGN_HTTP, reference], { useEntrypoint: true })
      .withNewFile('/work/core-release-handoff.json', `${JSON.stringify(handoff, null, 2)}\n`)
      .withExec(['sign-blob', '--yes', ...key, ...COSIGN_TIMEOUT, '--bundle', '/work/core-release-handoff.sigstore.json', '/work/core-release-handoff.json'],
        { useEntrypoint: true, redirectStdout: '/tmp/handoff-signature' })
      .withExec(['verify-blob', '--bundle', '/work/core-release-handoff.sigstore.json', ...identity, '/work/core-release-handoff.json'], { useEntrypoint: true });
    await signed.sync();
    return signed;
  }));
  console.log(`sealed Core ${platform} release: ${publicReference(reference)}`);
  return complete.directory('/work').withNewFile('timings/seal.json', timingsFile(timings));
}
