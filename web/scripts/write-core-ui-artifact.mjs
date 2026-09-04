import { resolve } from 'node:path';
import { writeCoreUiArtifact } from './core-ui-artifact.mjs';

const webRoot = resolve(import.meta.dirname, '..');
const projectRoot = resolve(webRoot, '..');
const buildRoot = resolve(webRoot, 'build');
const manifest = await writeCoreUiArtifact({ projectRoot, webRoot, buildRoot });

process.stdout.write(
	`Core UI artifact ${manifest.artifact_sha256} covers ${manifest.payload.file_count} files, ` +
		`${manifest.payload.byte_count} bytes, and ${manifest.routes.count} routes.\n`
);
