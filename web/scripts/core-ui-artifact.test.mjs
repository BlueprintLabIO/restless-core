import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import test from 'node:test';
import {
	CORE_UI_MANIFEST,
	CORE_UI_SCHEMA,
	coreUiReleaseMetadata,
	discoverCanonicalRoutes,
	verifyCoreUiArtifact,
	writeCoreUiArtifact
} from './core-ui-artifact.mjs';

async function put(path, contents = '') {
	await mkdir(dirname(path), { recursive: true });
	await writeFile(path, contents);
}

async function fixture() {
	const projectRoot = await mkdtemp(resolve(tmpdir(), 'restless-core-ui-'));
	const webRoot = resolve(projectRoot, 'web');
	const buildRoot = resolve(webRoot, 'build');
	await put(resolve(projectRoot, 'LICENSE'), 'core license\n');
	await put(resolve(webRoot, 'src/lib/vendor/pixel-agents/NOTICE.md'), 'pixel agents notice\n');
	await put(resolve(webRoot, 'static/vendor/pixel-agents/LICENSE'), 'pixel agents license\n');
	await put(resolve(webRoot, 'src/routes/+page.svelte'), '<h1>Companies</h1>\n');
	await put(resolve(webRoot, 'src/routes/[companyId]/+layout.svelte'), '<slot />\n');
	await put(resolve(webRoot, 'src/routes/[companyId]/+page.svelte'), '<h1>Attention</h1>\n');
	await put(resolve(webRoot, 'src/routes/[companyId]/work/+page.svelte'), '<h1>Work</h1>\n');
	await put(resolve(webRoot, 'src/routes/[companyId]/work/+page 2.svelte'), 'backup\n');
	await put(resolve(webRoot, 'src/routes/(internal)/office-demo/+page.svelte'), 'demo\n');
	await put(resolve(buildRoot, 'index.html'), '<!doctype html>\n');
	await put(resolve(buildRoot, '_app/app.js'), 'console.log("core");\n');
	return { projectRoot, webRoot, buildRoot };
}

test('route inventory contains page entries, not layouts or backup copies', async (context) => {
	const paths = await fixture();
	context.after(() => rm(paths.projectRoot, { recursive: true, force: true }));
	assert.deepEqual(await discoverCanonicalRoutes(resolve(paths.webRoot, 'src/routes')), [
		'/',
		'/[companyId]',
		'/[companyId]/work',
		'/office-demo'
	]);
});

test('artifact identity is deterministic and covers distribution bytes', async (context) => {
	const paths = await fixture();
	context.after(() => rm(paths.projectRoot, { recursive: true, force: true }));

	const first = await writeCoreUiArtifact(paths);
	const firstBytes = await readFile(resolve(paths.buildRoot, CORE_UI_MANIFEST), 'utf8');
	const second = await writeCoreUiArtifact(paths);
	const secondBytes = await readFile(resolve(paths.buildRoot, CORE_UI_MANIFEST), 'utf8');

	assert.equal(first.schema, CORE_UI_SCHEMA);
	assert.deepEqual(second, first);
	assert.equal(secondBytes, firstBytes);
	assert.equal(
		first.routes.sha256,
		'99c08257e7db8ea814d2eb638b6f1d0086ef64ba80d0e5f7a680282737a262c5'
	);
	assert.deepEqual(coreUiReleaseMetadata(first), {
		schema: CORE_UI_SCHEMA,
		artifact_sha256: first.artifact_sha256,
		payload_sha256: first.payload.sha256,
		route_manifest_sha256: first.routes.sha256,
		manifest_path: `/${CORE_UI_MANIFEST}`,
		route_count: first.routes.count
	});
	assert.equal(first.manifest.excluded_from_payload, true);
	assert.equal(
		first.payload.byte_count,
		first.payload.files.reduce((sum, file) => sum + file.size, 0)
	);
	assert.deepEqual(
		first.distribution.files.map((file) => file.path),
		['legal/restless-core/LICENSE', 'legal/pixel-agents/NOTICE.md', 'legal/pixel-agents/LICENSE']
	);
	for (const notice of first.distribution.files) {
		assert.ok(first.payload.files.some((file) => file.path === notice.path));
	}
	assert.ok(!first.payload.files.some((file) => file.path === CORE_UI_MANIFEST));
	assert.equal(
		await readFile(resolve(paths.buildRoot, 'legal/restless-core/LICENSE'), 'utf8'),
		'core license\n'
	);
	assert.deepEqual(await verifyCoreUiArtifact(paths), first);

	await put(resolve(paths.buildRoot, '_app/app.js'), 'console.log("tampered");\n');
	await assert.rejects(
		verifyCoreUiArtifact(paths),
		/manifest does not match the current build bytes and route inventory/
	);

	await put(resolve(paths.buildRoot, '_app/app.js'), 'console.log("changed");\n');
	const changedPayload = await writeCoreUiArtifact(paths);
	assert.notEqual(changedPayload.payload.sha256, first.payload.sha256);
	assert.equal(changedPayload.routes.sha256, first.routes.sha256);
	assert.notEqual(changedPayload.artifact_sha256, first.artifact_sha256);

	await put(resolve(paths.buildRoot, '_app/app.js'), 'console.log("core");\n');
	await put(resolve(paths.webRoot, 'src/routes/[companyId]/people/+page.svelte'), 'people\n');
	const changedRoutes = await writeCoreUiArtifact(paths);
	assert.equal(changedRoutes.payload.sha256, first.payload.sha256);
	assert.notEqual(changedRoutes.routes.sha256, first.routes.sha256);
	assert.notEqual(changedRoutes.artifact_sha256, first.artifact_sha256);

	await put(resolve(paths.projectRoot, 'LICENSE'), 'revised license\n');
	await assert.rejects(
		verifyCoreUiArtifact(paths),
		/distribution files do not match their canonical license\/notice sources/
	);
});

test('artifact writer refuses stale conflict-copy payloads', async (context) => {
	const paths = await fixture();
	context.after(() => rm(paths.projectRoot, { recursive: true, force: true }));
	await put(resolve(paths.buildRoot, 'index 2.html'), 'stale build\n');
	await assert.rejects(
		writeCoreUiArtifact(paths),
		/Core UI build contains a conflict-copy path and is not distributable: index 2\.html/
	);
});

test('the checked-out canonical route inventory remains explicit', async () => {
	const routesRoot = resolve(import.meta.dirname, '../src/routes');
	assert.deepEqual(await discoverCanonicalRoutes(routesRoot), [
		'/',
		'/[companyId]',
		'/[companyId]/authority',
		'/[companyId]/company',
		'/[companyId]/company/actions',
		'/[companyId]/company/authority',
		'/[companyId]/company/computer',
		'/[companyId]/company/decisions',
		'/[companyId]/company/doctor',
		'/[companyId]/company/identity',
		'/[companyId]/company/resources',
		'/[companyId]/people',
		'/[companyId]/work',
		'/[companyId]/work/[workId]',
		'/office-demo'
	]);
});
