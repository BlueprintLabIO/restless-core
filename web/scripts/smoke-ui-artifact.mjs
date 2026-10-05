/* Prove the artifact works the way a consumer uses it.
 *
 * Packs @restless/ui, installs the tarball into an empty project that has never seen this repository,
 * server-renders real views from it with Vite, and checks the output and the stylesheet. A tarball that
 * merely packs is not evidence; one that renders in a clean project is. */

import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const externalAt = process.argv.indexOf('--artifact-dir');
const dist = resolve(externalAt < 0 ? 'dist' : process.argv[externalAt + 1]);
const qualificationOnly = process.argv.includes('--qualification');
if (externalAt < 0)
	execFileSync(
		'node',
		['scripts/pack-ui.mjs', '--out', dist, ...(qualificationOnly ? ['--qualification'] : [])],
		{ stdio: 'inherit' }
	);
const manifest = JSON.parse(readFileSync(join(dist, 'ui-manifest.json'), 'utf8'));
if (qualificationOnly) {
	assert.equal(manifest.qualificationOnly, true);
	assert.equal(manifest.sourceRevision, null);
	assert.equal(manifest.sourceDirty, null);
} else {
	assert.match(manifest.sourceRevision, /^[0-9a-f]{40}$/);
	assert.equal(manifest.qualificationOnly, undefined);
}
const tarball = join(dist, manifest.tarball);

const dir = mkdtempSync(join(tmpdir(), 'restless-ui-smoke-'));
const run = (cmd, args) => execFileSync(cmd, args, { cwd: dir, stdio: 'pipe', encoding: 'utf8' });
let failed = false;

try {
	writeFileSync(
		join(dir, 'package.json'),
		JSON.stringify(
			{
				name: 'restless-ui-smoke',
				private: true,
				type: 'module',
				dependencies: { '@restless/ui': `file:${tarball}`, svelte: '^5.56.1' },
				devDependencies: { vite: '^8.0.16', '@sveltejs/vite-plugin-svelte': '^7.1.2' }
			},
			null,
			2
		)
	);
	run('npm', ['install', '--no-audit', '--no-fund']);

	writeFileSync(
		join(dir, 'Page.svelte'),
		`<script>
	import { Wordmark, WorkBoard, OutcomeFolio, HoldApprove, SemanticMark, AccountShell, CompanyPortfolio, STUDIO_BOARD, STUDIO_FOLIO } from '@restless/ui';
const companies = [
	{ id: 'local-ready', name: 'Local Company', status: 'running', tone: 'presence', focus: 'A real company focus', next: 'Review a prepared result', attentionCount: 0, entry: { href: '/local-ready' } },
	{ id: 'cloud-ready', name: 'Hosted Company', status: 'Ready', tone: 'presence', attentionCount: null, entry: { formAction: '?/enterCockpit', fields: { organization_id: 'cloud-ready' } } },
	{ id: 'pending-company', name: 'Pending Company', status: 'Setting up', next: 'Preparing its private workspace', attentionCount: null }
];
</script>
<AccountShell><CompanyPortfolio {companies} /></AccountShell>
<div class="bridge-tokens">
	<Wordmark />
	<SemanticMark meaning="attention" />
	<WorkBoard columns={STUDIO_BOARD} />
	<OutcomeFolio title={STUDIO_FOLIO.title} whatHappened={STUDIO_FOLIO.whatHappened} whyItMatters={STUDIO_FOLIO.whyItMatters}>
		{#snippet decision()}<HoldApprove label="Approve and publish" />{/snippet}
	</OutcomeFolio>
</div>
`
	);
	writeFileSync(
		join(dir, 'entry.js'),
		`import '@restless/ui/style.css';
import { render } from 'svelte/server';
import Page from './Page.svelte';
console.log(render(Page, {}).body);
`
	);
	writeFileSync(
		join(dir, 'vite.config.js'),
		`import { svelte } from '@sveltejs/vite-plugin-svelte';
export default {
	plugins: [svelte()],
	ssr: { noExternal: ['@restless/ui'] },
	build: { ssr: 'entry.js', outDir: 'out', emptyOutDir: true }
};
`
	);
	run('npx', ['vite', 'build']);
	const html = run('node', ['out/entry.js']);

	/* The stylesheet export must resolve through the package's exports map, and every file it
	 * imports must exist. (An SSR build drops CSS, so this is checked on the installed package.) */
	const entryCss = createRequire(join(dir, 'x.js')).resolve('@restless/ui/style.css');
	const imports = [...readFileSync(entryCss, 'utf8').matchAll(/@import '(\.\/[^']+)'/g)].map(
		(match) => match[1]
	);
	const css = imports.map((file) => readFileSync(join(dirname(entryCss), file), 'utf8')).join('\n');

	const expect = [
		[html, 'Recently landed', 'work board column'],
		[html, 'Approve the playable demo', 'outcome folio title'],
		[html, 'Approve and publish', 'hold-to-approve'],
		[html, 'class="bridge-tokens"', 'token scope'],
		[html, 'aria-label="Account navigation"', 'shared account header'],
		[html, 'href="/local-ready"', 'self-hosted adapter navigation'],
		[html, 'action="?/enterCockpit"', 'hosted native POST entry'],
		[html, 'name="organization_id" value="cloud-ready"', 'entry identity supplied by the adapter'],
		[html, 'Focus unavailable', 'unknown focus remains unknown'],
		[html, 'Nothing now', 'observed zero attention'],
		[css, '--intent-direction', 'tokens stylesheet'],
		[String(imports.length), '4', 'the four design stylesheets, and not the document reset'],
		[String(imports.includes('./base.css')), 'false', 'style.css leaves the host document alone']
	];
	const hosted =
		html.split('data-company-id="cloud-ready"')[1]?.split('data-company-id="pending-company"')[0] ??
		'';
	const pending = html.split('data-company-id="pending-company"')[1]?.split('</section>')[0] ?? '';
	const hostedAttention =
		hosted.match(/<span class="[^"]*\bportfolio-attention\b[^"]*">([\s\S]*?)<\/span>/)?.[1] ?? '';
	expect.push(
		[hostedAttention, '—', 'unknown attention remains unknown in its own row'],
		[String(hosted.includes('Nothing now')), 'false', 'hosted unknown attention never claims zero'],
		[
			String(/class="[^"]*\bportfolio-entry\b/.test(pending)),
			'false',
			'pending company has no invented entry'
		],
		[
			String(pending.includes('portfolio-entry-form')),
			'false',
			'pending company cannot submit entry'
		]
	);
	for (const [haystack, needle, what] of expect) {
		const ok = haystack.includes(needle);
		console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
		if (!ok) failed = true;
	}
} catch (error) {
	failed = true;
	console.error(error.stdout?.toString() ?? '', error.stderr?.toString() ?? '', error.message);
} finally {
	rmSync(dir, { recursive: true, force: true });
}

if (failed) process.exit(1);
console.log(`artifact ${manifest.tarball} renders in a clean project`);
