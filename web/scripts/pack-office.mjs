/* Package the company floor (src/lib/office) as a versioned artifact.
 *
 * The public site renders the real office as a scene, the same way it renders the design system:
 * from a release artifact, never from copied source. This packs the office, exactly the modules it
 * reaches (a few model types, the failure sentence and the vendored Pixel Agents engine), and the
 * sprites it loads at runtime. `$lib/` imports are rewritten to relative paths so the package
 * stands alone.
 *
 *   node scripts/pack-office.mjs [--version 0.1.0] [--out dist]
 *
 * The consumer serves `assets/` at `/vendor/pixel-agents/`; the office loads
 * its sprites from /vendor/pixel-agents/assets/. */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, normalize, relative, resolve } from 'node:path';

const args = process.argv.slice(2);
const option = (name, fallback) => {
	const at = args.indexOf(`--${name}`);
	return at >= 0 ? args[at + 1] : fallback;
};

const root = resolve('.');
const lib = join(root, 'src/lib');
const version = option('version', '0.1.0');
const outDir = resolve(option('out', 'dist'));
const stage = join(outDir, 'office');
const ENTRIES = ['office/OfficeCanvas.svelte', 'office/officeDemo.ts', 'office/projection.ts', 'office/officePlan.ts'];

function git(...gitArgs) {
	try {
		return execFileSync('git', gitArgs, { cwd: root, encoding: 'utf8' }).trim();
	} catch {
		return 'unknown';
	}
}

/* Walk the import graph from the entries; anything outside src/lib must be a declared dependency. */
const IMPORT = /(?:import|export)[^'"]*?from\s+['"]([^'"]+)['"]|import\(\s*['"]([^'"]+)['"]\s*\)|^\s*import\s+['"]([^'"]+)['"]/gm;
const EXTERNAL = /^(svelte|@lucide\/svelte)(\/|$)/;
function resolveSpec(from, spec) {
	let path;
	if (spec.startsWith('$lib/')) path = join(lib, spec.slice(5));
	else if (spec.startsWith('.')) path = normalize(join(dirname(join(lib, from)), spec));
	else {
		if (!EXTERNAL.test(spec)) throw new Error(`${from} imports ${spec}, which the office package does not carry`);
		return null;
	}
	const candidates = [path, `${path}.ts`, path.replace(/\.js$/, '.ts'), join(path, 'index.ts')];
	const found = candidates.find((candidate) => existsSync(candidate) && !candidate.endsWith('/'));
	if (!found) throw new Error(`${from}: cannot resolve ${spec}`);
	return relative(lib, found);
}
const files = new Set();
const queue = [...ENTRIES];
while (queue.length) {
	const file = queue.pop();
	if (files.has(file)) continue;
	files.add(file);
	if (!/\.(ts|svelte)$/.test(file)) continue;
	const source = readFileSync(join(lib, file), 'utf8');
	for (const match of source.matchAll(IMPORT)) {
		const next = resolveSpec(file, match[1] ?? match[2] ?? match[3]);
		if (next) queue.push(next);
	}
}

const revision = git('rev-parse', 'HEAD');
const dirty = git('status', '--porcelain', '--', 'src/lib/office', 'src/lib/model', 'src/lib/vendor', 'static/vendor') !== '';

rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });
for (const file of files) {
	const target = join(stage, file);
	mkdirSync(dirname(target), { recursive: true });
	let source = readFileSync(join(lib, file), 'utf8');
	if (/\.(ts|svelte)$/.test(file)) {
		source = source.replace(/(['"])\$lib\/([^'"]+)\1/g, (_, quote, rest) => {
			let path = relative(dirname(file), rest);
			if (!path.startsWith('.')) path = `./${path}`;
			return `${quote}${path}${quote}`;
		});
	}
	writeFileSync(target, source);
}
cpSync(join(lib, 'vendor/pixel-agents/NOTICE.md'), join(stage, 'vendor/pixel-agents/NOTICE.md'));
cpSync(join(root, 'static/vendor/pixel-agents'), join(stage, 'assets'), { recursive: true });

writeFileSync(
	join(stage, 'index.ts'),
	`/* The company floor, rendered from props. See README.md. */
export { default as OfficeCanvas } from './office/OfficeCanvas.svelte';
export { OFFICE_DEMO_MEMBERS, OFFICE_DEMO_PREFERENCES, OFFICE_DEMO_TEAMS } from './office/officeDemo';
export type { OfficeMember, OfficePresence } from './office/projection';
export type { OfficePreferences } from './office/officePlan';
`
);

const pkg = {
	name: '@restless/office',
	version,
	description: 'The Restless company floor: a pixel office rendered from members, teams and preferences.',
	license: 'Apache-2.0',
	type: 'module',
	svelte: './index.ts',
	types: './index.ts',
	exports: { '.': { types: './index.ts', svelte: './index.ts', default: './index.ts' }, './*': './*' },
	peerDependencies: { svelte: '^5.0.0', '@lucide/svelte': '>=1.0.0' },
	restless: { sourceRevision: revision, sourceDirty: dirty, packedAt: new Date().toISOString() }
};
writeFileSync(join(stage, 'package.json'), JSON.stringify(pkg, null, '\t') + '\n');
const licence = join(root, '../LICENSE');
if (existsSync(licence)) cpSync(licence, join(stage, 'LICENSE'));
writeFileSync(
	join(stage, 'README.md'),
	`# @restless/office ${version}

The Restless company floor, as the owner workspace renders it.

\`\`\`svelte
<script>
	import { OfficeCanvas, OFFICE_DEMO_MEMBERS, OFFICE_DEMO_TEAMS, OFFICE_DEMO_PREFERENCES } from '@restless/office';
</script>

<OfficeCanvas members={OFFICE_DEMO_MEMBERS} teams={OFFICE_DEMO_TEAMS} preferences={OFFICE_DEMO_PREFERENCES} explorable={false} />
\`\`\`

- Serve \`assets/\` at \`/vendor/pixel-agents/\`; the office loads its sprites from
  \`/vendor/pixel-agents/assets/\`.
- \`explorable={false}\` renders a scene a page can scroll over; aim it with \`setCamera()\` and pin
  overlays to people with \`actorsOnScreen()\`.
- Needs Svelte 5 and \`@lucide/svelte\`.
- Source revision: ${revision}${dirty ? ' (uncommitted changes)' : ''}.
- Apache 2.0. The office engine is Pixel Agents (MIT, see vendor/pixel-agents/NOTICE.md and assets/LICENSE);
  character art is based on JIK-A-4's MetroCity pack (CC0). The Restless name, marks and visual identity
  are not licensed.
`
);

const listed = [];
(function walk(dir) {
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) walk(path);
		else listed.push(path.slice(stage.length + 1));
	}
})(stage);

const packed = execFileSync('npm', ['pack', '--pack-destination', outDir, '--json'], { cwd: stage, encoding: 'utf8' });
const tarball = JSON.parse(packed)[0].filename;
const tarballPath = join(outDir, tarball);
const sha256 = createHash('sha256').update(readFileSync(tarballPath)).digest('hex');
writeFileSync(`${tarballPath}.sha256`, `${sha256}  ${tarball}\n`);
writeFileSync(
	join(outDir, 'office-manifest.json'),
	JSON.stringify({ name: pkg.name, version, tarball, sha256, sourceRevision: revision, sourceDirty: dirty, files: listed.sort() }, null, '\t') + '\n'
);
console.log(`packed ${tarball} (${listed.length} files) sha256 ${sha256.slice(0, 12)}…${dirty ? '  [uncommitted changes]' : ''}`);
