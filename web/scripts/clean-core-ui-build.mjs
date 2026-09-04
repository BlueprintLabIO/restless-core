import { rm } from 'node:fs/promises';
import { resolve } from 'node:path';

// adapter-static does not promise to remove payloads left by an earlier build.
// An immutable carrier must start from an empty output root or stale files can
// become signed, shipped bytes. These two paths are generated state beneath
// this package; source routes and user backup files are deliberately untouched.
const webRoot = resolve(import.meta.dirname, '..');
for (const directory of ['build', '.svelte-kit']) {
	await rm(resolve(webRoot, directory), { recursive: true, force: true });
}
