import { resolve } from 'node:path';
import { coreUiReleaseMetadata, verifyCoreUiArtifact } from './core-ui-artifact.mjs';

const webRoot = resolve(import.meta.dirname, '..');
const projectRoot = resolve(webRoot, '..');
const buildRoot = resolve(webRoot, 'build');
const manifest = await verifyCoreUiArtifact({ projectRoot, webRoot, buildRoot });

if (process.argv.includes('--json')) {
	process.stdout.write(JSON.stringify(coreUiReleaseMetadata(manifest)));
} else {
	process.stdout.write(
		`Core UI artifact ${manifest.artifact_sha256} matches ${manifest.payload.file_count} files and ` +
			`${manifest.routes.count} routes.\n`
	);
}
