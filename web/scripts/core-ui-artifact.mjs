import { createHash } from 'node:crypto';
import { copyFile, lstat, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve, sep } from 'node:path';

export const CORE_UI_MANIFEST = 'core-ui-manifest.json';
export const CORE_UI_SCHEMA = 'restless.core-ui-artifact/v1';

const DISTRIBUTION_FILES = [
	{
		kind: 'license',
		subject: 'Restless Core',
		source: 'LICENSE',
		target: 'legal/restless-core/LICENSE'
	},
	{
		kind: 'notice',
		subject: 'Pixel Agents and bundled artwork',
		source: 'web/src/lib/vendor/pixel-agents/NOTICE.md',
		target: 'legal/pixel-agents/NOTICE.md'
	},
	{
		kind: 'license',
		subject: 'Pixel Agents',
		source: 'web/static/vendor/pixel-agents/LICENSE',
		target: 'legal/pixel-agents/LICENSE'
	}
];

function portablePath(path) {
	return path.split(sep).join('/');
}

function lexical(left, right) {
	return left < right ? -1 : left > right ? 1 : 0;
}

function isConflictCopyPath(path) {
	return path.split('/').some((segment) => / \d+(?:\.[^/]*)?$/.test(segment));
}

async function regularFiles(root, current = root) {
	const files = [];
	for (const entry of (await readdir(current, { withFileTypes: true })).sort((a, b) =>
		lexical(a.name, b.name)
	)) {
		const path = resolve(current, entry.name);
		if (entry.isSymbolicLink()) {
			throw new Error(
				`Core UI artifacts may not contain symbolic links: ${portablePath(relative(root, path))}`
			);
		}
		if (entry.isDirectory()) files.push(...(await regularFiles(root, path)));
		else if (entry.isFile()) files.push(path);
		else {
			throw new Error(
				`Core UI artifacts may contain only directories and regular files: ${portablePath(relative(root, path))}`
			);
		}
	}
	return files;
}

/**
 * Inventory the actual Svelte page entries. Layouts and near-miss backup files
 * are deliberately absent: this list describes browser-addressable routes,
 * not every source file under the route tree.
 */
export async function discoverCanonicalRoutes(routesRoot) {
	const routeSources = (await regularFiles(routesRoot)).filter((path) =>
		/^\+page(?:@[^.]*)?\.svelte$/.test(path.slice(path.lastIndexOf(sep) + 1))
	);
	const routes = new Set();
	for (const source of routeSources) {
		const directory = portablePath(relative(routesRoot, dirname(source)));
		const segments = directory
			.split('/')
			.filter((segment) => segment && !(segment.startsWith('(') && segment.endsWith(')')));
		routes.add(segments.length ? `/${segments.join('/')}` : '/');
	}
	return [...routes].sort();
}

async function installDistributionFiles(projectRoot, buildRoot) {
	for (const file of DISTRIBUTION_FILES) {
		const source = resolve(projectRoot, file.source);
		const target = resolve(buildRoot, file.target);
		await mkdir(dirname(target), { recursive: true });
		await copyFile(source, target);
	}
}

async function distributionFilesMatch(projectRoot, buildRoot) {
	for (const file of DISTRIBUTION_FILES) {
		const source = await readFile(resolve(projectRoot, file.source));
		const target = await readFile(resolve(buildRoot, file.target)).catch(() => null);
		if (!target || !source.equals(target)) return false;
	}
	return true;
}

function aggregatePayload(files) {
	const digest = createHash('sha256');
	digest.update('restless.core-ui-payload/v1\0');
	for (const file of files) {
		digest.update(file.path);
		digest.update('\0');
		digest.update(String(file.size));
		digest.update('\0');
		digest.update(file.sha256);
		digest.update('\0');
	}
	return digest.digest('hex');
}

function routeManifestIdentity(routes) {
	const digest = createHash('sha256');
	digest.update('restless.core-ui-routes/v1\0');
	for (const route of routes) {
		digest.update(route);
		digest.update('\0');
	}
	return digest.digest('hex');
}

function artifactIdentity(payloadSha256, routes) {
	const digest = createHash('sha256');
	digest.update(`${CORE_UI_SCHEMA}\0${payloadSha256}\0`);
	for (const route of routes) {
		digest.update(route);
		digest.update('\0');
	}
	return digest.digest('hex');
}

async function describePayload(buildRoot) {
	const described = [];
	for (const path of await regularFiles(buildRoot)) {
		const relativePath = portablePath(relative(buildRoot, path));
		if (relativePath === CORE_UI_MANIFEST) continue;
		if (isConflictCopyPath(relativePath)) {
			throw new Error(
				`Core UI build contains a conflict-copy path and is not distributable: ${relativePath}`
			);
		}
		const bytes = await readFile(path);
		described.push({
			path: relativePath,
			size: bytes.byteLength,
			sha256: createHash('sha256').update(bytes).digest('hex')
		});
	}
	return described.sort((a, b) => lexical(a.path, b.path));
}

async function describeCoreUiArtifact({ projectRoot, webRoot, buildRoot }) {
	const entrypoint = resolve(buildRoot, 'index.html');
	if (!(await lstat(entrypoint).catch(() => null))?.isFile()) {
		throw new Error(`Core UI build has no index.html: ${entrypoint}`);
	}

	const files = await describePayload(buildRoot);
	const routes = await discoverCanonicalRoutes(resolve(webRoot, 'src/routes'));
	if (routes.length === 0) throw new Error('Core UI route inventory is empty');
	const byteCount = files.reduce((total, file) => total + file.size, 0);
	const payloadSha256 = aggregatePayload(files);
	return {
		schema: CORE_UI_SCHEMA,
		artifact_sha256: artifactIdentity(payloadSha256, routes),
		entrypoint: 'index.html',
		manifest: {
			path: CORE_UI_MANIFEST,
			excluded_from_payload: true
		},
		payload: {
			sha256: payloadSha256,
			file_count: files.length,
			byte_count: byteCount,
			files
		},
		routes: {
			format: 'sveltekit-url-pattern',
			sha256: routeManifestIdentity(routes),
			count: routes.length,
			items: routes
		},
		distribution: {
			files: DISTRIBUTION_FILES.map(({ kind, subject, target }) => ({
				kind,
				subject,
				path: target
			}))
		}
	};
}

/**
 * Materialise the distribution files and write the detached identity inside
 * the build. The manifest names itself as excluded because no file can contain
 * its own cryptographic digest; every other byte in the directory is covered.
 */
export async function writeCoreUiArtifact({ projectRoot, webRoot, buildRoot }) {
	await rm(resolve(buildRoot, CORE_UI_MANIFEST), { force: true });
	await installDistributionFiles(projectRoot, buildRoot);
	const manifest = await describeCoreUiArtifact({ projectRoot, webRoot, buildRoot });

	await writeFile(
		resolve(buildRoot, CORE_UI_MANIFEST),
		`${JSON.stringify(manifest, null, 2)}\n`,
		'utf8'
	);
	return manifest;
}

export function coreUiReleaseMetadata(manifest) {
	return {
		schema: manifest.schema,
		artifact_sha256: manifest.artifact_sha256,
		payload_sha256: manifest.payload.sha256,
		route_manifest_sha256: manifest.routes.sha256,
		manifest_path: `/${manifest.manifest.path}`,
		route_count: manifest.routes.count
	};
}

/** Refuse a stale or hand-edited identity before it enters a release manifest. */
export async function verifyCoreUiArtifact({ projectRoot, webRoot, buildRoot }) {
	const manifestPath = resolve(buildRoot, CORE_UI_MANIFEST);
	const raw = await readFile(manifestPath, 'utf8').catch(() => null);
	if (raw === null) throw new Error(`Core UI manifest is missing: ${manifestPath}`);
	let recorded;
	try {
		recorded = JSON.parse(raw);
	} catch {
		throw new Error(`Core UI manifest is not valid JSON: ${manifestPath}`);
	}
	if (!(await distributionFilesMatch(projectRoot, buildRoot))) {
		throw new Error(
			'Core UI distribution files do not match their canonical license/notice sources'
		);
	}
	const observed = await describeCoreUiArtifact({ projectRoot, webRoot, buildRoot });
	if (JSON.stringify(recorded) !== JSON.stringify(observed)) {
		throw new Error('Core UI manifest does not match the current build bytes and route inventory');
	}
	return recorded;
}
