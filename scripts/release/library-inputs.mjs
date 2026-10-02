import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { officeSourceFiles, sourceFiles } from '../../web/scripts/package-inputs.mjs';

export const libraryKinds = ['ui', 'office', 'issuer'];
export const issuerSources = ['issuer.mjs', 'membership-sql.mjs', 'core-request.mjs'];
const shared = ['LICENSE', 'dagger.json', '.dagger/package.json', '.dagger/tsconfig.json',
  '.dagger/src/index.ts', '.dagger/src/libraries.ts', '.dagger/src/publish.ts',
  '.github/workflows/ui-artifact-release.yml', 'scripts/release/pack-library.mjs',
  'scripts/release/check-library.mjs', 'scripts/release/library-release.mjs',
  'scripts/release/verify-library.mjs', 'scripts/release/install-dagger.sh', 'scripts/release/library-inputs.mjs',
  'web/scripts/package-provenance.mjs', 'web/scripts/package-inputs.mjs'];
const frontend = ['web/package.json', 'web/package-lock.json', 'web/svelte.config.js', 'web/tsconfig.json',
  'web/scripts/package-inputs.mjs'];

/** Identity of actual packaged sources plus their build/qualification recipes. */
export function libraryInputs(root, kind) {
  if (!libraryKinds.includes(kind)) throw new Error('choose ui, office or issuer');
  root = resolve(root);
  const web = join(root, 'web');
  const paths = [...shared];
  if (kind === 'ui') paths.push(...frontend, 'web/scripts/ui-package.json', 'web/scripts/pack-ui.mjs',
    'web/scripts/check-ui-boundary.mjs', 'web/scripts/smoke-ui-artifact.mjs',
    ...sourceFiles(join(web, 'src/lib/ui')).map(path => relative(root, path)));
  if (kind === 'office') paths.push(...frontend, 'web/scripts/office-package.json', 'web/scripts/pack-office.mjs',
    'web/src/lib/vendor/pixel-agents/NOTICE.md',
    ...officeSourceFiles(web).map(path => `web/src/lib/${path}`),
    ...sourceFiles(join(web, 'static/vendor/pixel-agents')).map(path => relative(root, path)));
  if (kind === 'issuer') paths.push(...issuerSources.map(file => `services/identity/src/${file}`),
    ...sourceFiles(join(root, 'services/identity/library')).map(path => relative(root, path)));
  const files = [...new Set(paths)].sort().map(path => {
    const normalized = relative(root, resolve(root, path));
    if (normalized === '..' || normalized.startsWith('../')) throw new Error(`library input escapes source root: ${path}`);
    let stat;
    try { stat = lstatSync(join(root, normalized)); }
    catch (error) { throw new Error(`missing library input: ${path}`, {cause: error}); }
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`library input must be a regular file: ${path}`);
    return {path: normalized, sha256: createHash('sha256').update(readFileSync(join(root, normalized))).digest('hex')};
  });
  const identity = {format: 'restless.core.library-inputs.v1', component: kind, files};
  return {...identity, input_sha256: createHash('sha256').update(JSON.stringify(identity)).digest('hex')};
}

export function affectedLibraries(current, previous) {
  return libraryKinds.filter(kind => libraryInputs(current, kind).input_sha256 !== libraryInputs(previous, kind).input_sha256);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [root = '.', kind] = process.argv.slice(2);
  console.log(JSON.stringify(kind ? libraryInputs(root, kind) : libraryKinds.map(kind => libraryInputs(root, kind)), null, 2));
}
