/* Package src/lib/ui as a versioned artifact.
 *
 * Restless Cloud consumes Core as immutable release artifacts and never copies its source. This is the
 * artifact for the design system: a tarball of exactly the files under src/lib/ui, a checksum, and a
 * manifest that records which source revision it came from.
 *
 *   node scripts/pack-ui.mjs [--version 0.1.0] [--out dist]
 *   --qualification allows a source-only consumer check without Git metadata. Its artifact is
 *   explicitly unversioned and must never be published.
 *
 * The boundary check runs first, because an artifact that still reaches into the app cannot work. */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { normalizePackageModes, packageProvenance } from './package-provenance.mjs';

const args = process.argv.slice(2);
const qualificationOnly = args.includes('--qualification');
const option = (name, fallback) => {
	const at = args.indexOf(`--${name}`);
	return at >= 0 ? args[at + 1] : fallback;
};

const root = resolve('.');
const source = join(root, 'src/lib/ui');
const template = JSON.parse(readFileSync(join(root, 'scripts/ui-package.json'), 'utf8'));
const version = option('version', template.version);
const outDir = resolve(option('out', 'dist'));
const stage = join(outDir, 'ui');

execFileSync('node', ['scripts/check-ui-boundary.mjs'], { stdio: 'inherit' });

const { packedAt, ...provenance } = packageProvenance(args, root, ['src/lib/ui', 'scripts/ui-package.json',
	'scripts/pack-ui.mjs', 'scripts/package-provenance.mjs', 'scripts/check-ui-boundary.mjs', '../LICENSE']);
const revision = provenance.sourceRevision;
const dirty = provenance.sourceDirty;

rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });
cpSync(source, stage, {
	recursive: true,
	filter: (path) => !/\.test\.ts$/.test(path)
});

const pkg = {
	...template,
	version,
	restless: { ...provenance, packedAt }
};
writeFileSync(join(stage, 'package.json'), JSON.stringify(pkg, null, '\t') + '\n');

const licence = join(root, '../LICENSE');
if (existsSync(licence)) cpSync(licence, join(stage, 'LICENSE'));

writeFileSync(
	join(stage, 'README.md'),
	`# @restless/ui ${version}

The Restless design system: tokens, pixel marks and the views the owner workspace is made of.
Everything renders from props; nothing reaches the network or a company.

\`\`\`svelte
<script>
	import '@restless/ui/style.css';
	import { WorkBoard, STUDIO_BOARD } from '@restless/ui';
</script>

<div class="bridge-tokens">
	<WorkBoard columns={STUDIO_BOARD} />
</div>
\`\`\`

- Needs Svelte 5. Fonts are yours to load: Inter (variable, opsz), IBM Plex Mono and Silkscreen.
- Put \`bridge-tokens\` on an ancestor. It carries tokens and type without the cockpit's fixed frame.
- ${qualificationOnly ? 'Unversioned qualification artifact. Do not publish.' : `Source revision: ${revision}${dirty ? ' (uncommitted packaging inputs)' : ''}.`}
- Apache 2.0. The Restless name, marks and visual identity are not licensed.
`
);

normalizePackageModes(stage);
const files = [];
(function walk(dir) {
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) walk(path);
		else files.push(path.slice(stage.length + 1));
	}
})(stage);

const packed = execFileSync('npm', ['pack', '--pack-destination', outDir, '--json'], {
	cwd: stage,
	encoding: 'utf8'
});
const tarball = JSON.parse(packed)[0].filename;
const tarballPath = join(outDir, tarball);
const sha256 = createHash('sha256').update(readFileSync(tarballPath)).digest('hex');
writeFileSync(`${tarballPath}.sha256`, `${sha256}  ${tarball}\n`);
writeFileSync(
	join(outDir, 'ui-manifest.json'),
	JSON.stringify({ name: pkg.name, version, tarball, sha256, ...provenance, files }, null, '\t') + '\n'
);

console.log(`packed ${tarball} (${files.length} files) sha256 ${sha256.slice(0, 12)}…${qualificationOnly ? '  [qualification only; do not publish]' : dirty ? '  [uncommitted packaging inputs]' : ''}`);
