import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const [kind, destination] = process.argv.slice(2);
assert.ok(['ui', 'office', 'issuer'].includes(kind));
const out = resolve(destination);
const manifest = JSON.parse(readFileSync(join(out, `${kind}-manifest.json`)));
const archive = join(out, manifest.tarball);
assert.match(manifest.sourceRevision, /^[0-9a-f]{40}$/);
assert.equal(manifest.sourceDirty, false);
assert.equal(manifest.qualificationOnly, undefined);
assert.equal(createHash('sha256').update(readFileSync(archive)).digest('hex'), manifest.sha256);
const pkg = JSON.parse(execFileSync('tar', ['-xOf', archive, 'package/package.json'], { encoding: 'utf8' }));
assert.equal(pkg.name, `@restless/${kind}`);
assert.equal(pkg.version, manifest.version);
assert.equal(pkg.restless.sourceRevision, manifest.sourceRevision);
assert.equal(pkg.restless.sourceDirty, false);
assert.equal(pkg.restless.qualificationOnly, undefined);

if (kind === 'ui') execFileSync('node', ['scripts/smoke-ui-artifact.mjs', '--artifact-dir', out], {
  cwd: resolve('web'), stdio: 'inherit',
});

const consumer = mkdtempSync(join(tmpdir(), `restless-${kind}-consumer-`));
try {
  const lock = JSON.parse(readFileSync('web/package-lock.json'));
  const dependencies = { [pkg.name]: `file:${archive}` };
  if (kind !== 'issuer') {
    dependencies.svelte = lock.packages['node_modules/svelte'].version;
    dependencies['@lucide/svelte'] = lock.packages['node_modules/@lucide/svelte'].version;
  }
  writeFileSync(join(consumer, 'package.json'), JSON.stringify({ name: 'restless-library-check',
    private: true, type: 'module', dependencies }, null, 2) + '\n');
  const run = (cmd, args) => execFileSync(cmd, args, { cwd: consumer, encoding: 'utf8', stdio: 'pipe' });
  run('npm', ['install', '--ignore-scripts', '--no-audit', '--no-fund']);
  mkdirSync(join(out, 'dependencies'), { recursive: true });
  for (const file of ['package.json', 'package-lock.json']) cpSync(join(consumer, file), join(out, 'dependencies', file));
  let audit;
  try { audit = run('npm', ['audit', '--omit=dev', '--audit-level=high', '--json']); }
  catch (error) {
    if (error.stdout) writeFileSync(join(out, 'npm-audit.json'), error.stdout);
    throw new Error(`${kind}: runtime dependency audit failed`, { cause: error });
  }
  writeFileSync(join(out, 'npm-audit.json'), audit);
  if (kind === 'issuer') {
    run('node', ['--input-type=module', '-e', 'import assert from "node:assert/strict"; '
      + 'import {createIssuer} from "@restless/issuer"; '
      + 'import {createSqlMembershipStore} from "@restless/issuer/membership-sql"; '
      + 'import {coreRequest} from "@restless/issuer/core-request"; '
      + 'for(const value of [createIssuer,createSqlMembershipStore,coreRequest]) assert.equal(typeof value,"function");']);
  }
  if (kind === 'office') {
    const tools = ['vite', '@sveltejs/vite-plugin-svelte'];
    run('npm', ['install', '--save-dev', '--ignore-scripts', '--no-audit', '--no-fund',
      ...tools.map(name => `${name}@${lock.packages[`node_modules/${name}`].version}`)]);
    writeFileSync(join(consumer, 'Office.svelte'), '<script>import {OfficeCanvas,OFFICE_DEMO_MEMBERS,OFFICE_DEMO_TEAMS,OFFICE_DEMO_PREFERENCES} from "@restless/office";</script>\n'
      + '<OfficeCanvas members={OFFICE_DEMO_MEMBERS} teams={OFFICE_DEMO_TEAMS} preferences={OFFICE_DEMO_PREFERENCES} explorable={false}/>\n');
    writeFileSync(join(consumer, 'entry.js'), 'import {render} from "svelte/server";import Office from "./Office.svelte";console.log(render(Office).body);\n');
    writeFileSync(join(consumer, 'vite.config.js'), 'import {svelte} from "@sveltejs/vite-plugin-svelte";export default {plugins:[svelte()],ssr:{noExternal:["@restless/office"]},build:{ssr:"entry.js",outDir:"out"}};\n');
    run('npx', ['vite', 'build']);
    const html = run('node', ['out/entry.js']);
    assert.match(html, /<canvas\b/);
    const sprites = manifest.files.filter(file => file.endsWith('.png'));
    assert.ok(sprites.length > 0, 'office must carry its sprites');
    for (const file of sprites) {
      const bytes = readFileSync(join(consumer, 'node_modules/@restless/office', file));
      assert.equal(bytes.subarray(1, 4).toString(), 'PNG');
    }
  }
  writeFileSync(join(out, 'qualification.txt'), `PASS ${pkg.name} ${pkg.version}: exact archive/provenance, empty consumer, runtime dependency audit${kind === 'office' ? ', rendered canvas and packaged sprites' : ''}\n`);
  console.log(readFileSync(join(out, 'qualification.txt'), 'utf8').trim());
} finally { rmSync(consumer, { recursive: true, force: true }); }
