import { dag, Directory, Secret, ReturnType } from '@dagger.io/dagger';
import { CORE_WORKFLOW, COSIGN_IMAGE, ORAS_IMAGE, node, registryConfig, tool } from './publish.js';

const REPOSITORY = 'ghcr.io/blueprintlabio/restless-core-release';

/** Seal exact images, SLSA/SPDX evidence and the existing Core bundle contract. */
export async function sealRelease(source: Directory, artifacts: Directory, revision: string, platform: string,
  username: string, password: Secret, oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string): Promise<Directory> {
  if (workflowRef !== CORE_WORKFLOW || !/^https:\/\/[^/]+\.actions\.githubusercontent\.com\//.test(oidcRequestUrl)) {
    throw new Error('Core sealing requires the trusted dev workflow and its GitHub OIDC endpoint');
  }
  const config = await registryConfig(username, password);
  const identity = ['--certificate-identity', `https://github.com/${CORE_WORKFLOW}`,
    '--certificate-oidc-issuer', 'https://token.actions.githubusercontent.com'];
  let cosign = tool(COSIGN_IMAGE, config).withDirectory('/work', artifacts)
    .withEnvVariable('GITHUB_ACTIONS', 'true').withEnvVariable('CI', 'true')
    .withEnvVariable('ACTIONS_ID_TOKEN_REQUEST_URL', oidcRequestUrl)
    .withSecretVariable('ACTIONS_ID_TOKEN_REQUEST_TOKEN', oidcRequestToken);
  const components = ['runtime-tools', 'account-plane', 'company-runtime', 'native-documents-collaboration'];
  for (const component of components) {
    const record = JSON.parse(await artifacts.file(`images/${component}.json`).contents());
    cosign = cosign.withExec(['sign', '--yes', record.reference], { useEntrypoint: true })
      .withExec(['verify', ...identity, record.reference], { useEntrypoint: true })
      .withExec(['attest', '--yes', '--type', 'https://slsa.dev/provenance/v1', '--predicate', `/work/provenance/${component}.json`, record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-provenance-signature` })
      .withExec(['verify-attestation', ...identity, '--type', 'https://slsa.dev/provenance/v1', record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-verified-provenance` })
      .withExec(['attest', '--yes', '--type', 'spdxjson', '--predicate', `/work/sboms/${component}.json`, record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-sbom-signature` })
      .withExec(['verify-attestation', ...identity, '--type', 'spdxjson', record.reference],
        { useEntrypoint: true, redirectStdout: `/tmp/${component}-verified-sbom` });
  }
  await cosign.sync();
  const assembled = node(source).withDirectory('/work', artifacts)
    .withExec(['node', 'scripts/release/assemble.mjs', '/src', '/work', revision, platform]);
  const assembly = JSON.parse(await assembled.file('/work/assembly.json').contents());
  const tag = `${REPOSITORY}:${revision}-${platform.split('/')[1]}`;
  const oras = tool(ORAS_IMAGE, config);
  const lookup = oras.withExec(['manifest', 'fetch', '--descriptor', tag], { useEntrypoint: true, expect: ReturnType.Any });
  let payload: Directory;
  let reference: string;
  if (await lookup.exitCode() === 0) {
    const descriptor = JSON.parse(await lookup.stdout());
    reference = `${REPOSITORY}@${descriptor.digest}`;
    const pulled = oras.withExec(['pull', reference, '--output', '/download'], { useEntrypoint: true });
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
    await verified.sync();
  } else {
    const error = await lookup.stderr();
    if (!/(?:manifest[ _]unknown|name[ _]unknown|: not found|: 404)/i.test(error)) throw new Error(`Core bundle registry lookup failed: ${error}`);
    const documents = cosign.withDirectory('/work', assembled.directory('/work'))
      .withExec(['sign-blob', '--yes', '--bundle', '/work/release-manifest.sigstore.json', `/work/${assembly.release_manifest}`],
        { useEntrypoint: true, redirectStdout: '/tmp/release-signature' })
      .withExec(['verify-blob', '--bundle', '/work/release-manifest.sigstore.json', ...identity, `/work/${assembly.release_manifest}`], { useEntrypoint: true })
      .withExec(['sign-blob', '--yes', '--bundle', '/work/contract-set.sigstore.json', `/work/${assembly.contract_manifest}`],
        { useEntrypoint: true, redirectStdout: '/tmp/contract-signature' })
      .withExec(['verify-blob', '--bundle', '/work/contract-set.sigstore.json', ...identity, `/work/${assembly.contract_manifest}`], { useEntrypoint: true });
    const indexed = node(source).withDirectory('/work', documents.directory('/work'))
      .withExec(['node', 'scripts/create-core-release-bundle.mjs', '--bundle-root', '/work',
        '--release-manifest', `/work/${assembly.release_manifest}`, '--contract-set', `/work/${assembly.contract_manifest}`,
        '--release-signature', '/work/release-manifest.sigstore.json', '--contract-signature', '/work/contract-set.sigstore.json',
        '--repository', 'BlueprintLabIO/restless-core', '--workflow-ref', workflowRef, '--output', '/work/core-release-bundle.json']);
    const signedIndex = cosign.withDirectory('/work', indexed.directory('/work'))
      .withExec(['sign-blob', '--yes', '--bundle', '/work/core-release-bundle.sigstore.json', '/work/core-release-bundle.json'],
        { useEntrypoint: true, redirectStdout: '/tmp/index-signature' })
      .withExec(['verify-blob', '--bundle', '/work/core-release-bundle.sigstore.json', ...identity, '/work/core-release-bundle.json'], { useEntrypoint: true });
    payload = signedIndex.directory('/work');
    const archive = node(source).withExec(['apk', 'add', '--no-cache', 'tar'])
      .withDirectory('/work', payload).withExec(['tar', '--sort=name', '--mtime=UTC 1970-01-01', '--owner=0', '--group=0', '--numeric-owner',
        '-czf', '/core-release-bundle.tar.gz', '-C', '/work', '.']);
    await oras.withFile('/upload/core-release-bundle.tar.gz', archive.file('/core-release-bundle.tar.gz')).withWorkdir('/upload')
      .withExec(['push', tag, '--artifact-type', 'application/vnd.restless.core.release-bundle.v1',
        'core-release-bundle.tar.gz:application/vnd.restless.core.release-bundle.v1+tar+gzip'], { useEntrypoint: true }).sync();
    const descriptor = JSON.parse(await oras.withExec(['manifest', 'fetch', '--descriptor', tag], { useEntrypoint: true }).stdout());
    reference = `${REPOSITORY}@${descriptor.digest}`;
  }
  if (!/^sha256:[0-9a-f]{64}$/.test(reference.split('@')[1])) throw new Error('Core publication requires an immutable OCI bundle');
  const handoff = { format: 'restless.core.release-handoff.v1', source: { repository: 'BlueprintLabIO/restless-core', revision },
    workflow: { ref: workflowRef }, bundle: reference, release_manifest_digest: assembly.release_manifest_digest };
  const complete = cosign.withDirectory('/work', payload)
    .withExec(['sign', '--yes', reference], { useEntrypoint: true }).withExec(['verify', ...identity, reference], { useEntrypoint: true })
    .withNewFile('/work/core-release-handoff.json', `${JSON.stringify(handoff, null, 2)}\n`)
    .withExec(['sign-blob', '--yes', '--bundle', '/work/core-release-handoff.sigstore.json', '/work/core-release-handoff.json'],
      { useEntrypoint: true, redirectStdout: '/tmp/handoff-signature' })
    .withExec(['verify-blob', '--bundle', '/work/core-release-handoff.sigstore.json', ...identity, '/work/core-release-handoff.json'], { useEntrypoint: true });
  await complete.sync();
  console.log(`sealed Core ${platform} release: ${reference}`);
  return complete.directory('/work');
}
