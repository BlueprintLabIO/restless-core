import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { normalizePackageModes, packageProvenance } from '../../web/scripts/package-provenance.mjs';
import { issuerSources } from './library-inputs.mjs';

const [kind, revision, epoch, destination] = process.argv.slice(2);
if (!['ui', 'office', 'issuer'].includes(kind)) throw new Error('choose ui, office or issuer');
const root = resolve('.');
const out = resolve(destination);
const args = ['--source-revision', revision, '--source-date-epoch', epoch];
const { packedAt, ...provenance } = packageProvenance(args, root, []);
mkdirSync(out, { recursive: true });
let manifest;
if (kind !== 'issuer') {
  execFileSync('node', [`scripts/pack-${kind}.mjs`, ...args, '--out', out], {
    cwd: join(root, 'web'), stdio: 'inherit',
  });
  manifest = JSON.parse(readFileSync(join(out, `${kind}-manifest.json`)));
} else {
  const stage = join(out, 'issuer');
  mkdirSync(stage, { recursive: true });
  cpSync(join(root, 'services/identity/library'), stage, { recursive: true });
  for (const file of issuerSources) {
    cpSync(join(root, 'services/identity/src', file), join(stage, file));
  }
  cpSync(join(root, 'LICENSE'), join(stage, 'LICENSE'));
  const pkg = JSON.parse(readFileSync(join(stage, 'package.json')));
  pkg.restless = { ...provenance, packedAt };
  writeFileSync(join(stage, 'package.json'), JSON.stringify(pkg, null, 2) + '\n');
  normalizePackageModes(stage);
  const packed = JSON.parse(execFileSync('npm', ['pack', '--pack-destination', out, '--json'], {
    cwd: stage, encoding: 'utf8',
  }))[0];
  const sha256 = createHash('sha256').update(readFileSync(join(out, packed.filename))).digest('hex');
  manifest = { name: pkg.name, version: pkg.version, tarball: packed.filename, sha256,
    ...provenance, files: packed.files.map(file => file.path).sort() };
  writeFileSync(join(out, 'issuer-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  writeFileSync(join(out, `${packed.filename}.sha256`), `${sha256}  ${packed.filename}\n`);
}
if (manifest.sourceRevision !== revision || manifest.sourceDirty !== false || manifest.qualificationOnly) {
  throw new Error('library output does not bind clean release provenance');
}
console.log(`Packed ${manifest.name} ${manifest.version} from ${revision}`);
