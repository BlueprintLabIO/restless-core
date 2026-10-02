#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { readCoreReleaseTuple, createCompanyCollaborationContractSet } from '../create-company-collaboration-contract-set.mjs';
import { createCoreReleaseManifest } from '../create-core-release-manifest.mjs';

const [sourceRoot, outputRoot, revision, platform] = process.argv.slice(2);
const images = {};
for (const [component, key] of [['account-plane', 'account_plane'], ['company-runtime', 'company_runtime'], ['native-documents-collaboration', 'native_documents']]) {
  const record = JSON.parse(await readFile(join(outputRoot, 'images', `${component}.json`)));
  if (record.source_revision !== revision || record.platform !== platform) throw new Error(`${component}: qualification scope differs from the release`);
  for (const [folder, field] of [['scans', 'scan_sha256'], ['sboms', 'sbom_sha256']]) {
    const bytes = await readFile(join(outputRoot, folder, `${component}.json`));
    if (createHash('sha256').update(bytes).digest('hex') !== record[field]) throw new Error(`${component}: qualification bytes differ`);
  }
  images[key] = record.reference;
}
const release = await readCoreReleaseTuple({ sourceRoot, sourceRevision: revision,
  accountPlaneImage: images.account_plane, companyRuntimeImage: images.company_runtime, nativeDocumentsImage: images.native_documents });
const contract = await createCompanyCollaborationContractSet({ sourceRoot, outputRoot, release });
const manifest = await createCoreReleaseManifest({ contractSetManifestPath: contract.manifestPath, outputRoot, platforms: [platform] });
await writeFile(join(outputRoot, 'assembly.json'), `${JSON.stringify({ revision, platform, images,
  release_manifest: manifest.manifestPath.slice(outputRoot.length + 1), contract_manifest: contract.manifestPath.slice(outputRoot.length + 1),
  release_manifest_digest: manifest.manifestDigest }, null, 2)}\n`);
console.log(`Core v2 composition assembled for ${platform}`);
