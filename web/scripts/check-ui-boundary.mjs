/* The design-system boundary stays honest only while something fails when it is crossed.
 *
 * Everything under src/lib/ui renders from props and is packed into a versioned artifact that
 * Restless Cloud consumes without copying Core source (scripts/pack-ui.mjs). That only works if a
 * file in here can be lifted out of the app unchanged, so it may import:
 *
 *   - relative paths that stay inside src/lib/ui;
 *   - `svelte` and `svelte/*`;
 *   - the few packages listed in ALLOWED_PACKAGES.
 *
 * It may not reach for `$lib`, `$app`, the query client, the model layer or the network. */

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';

const ROOT = resolve('src/lib/ui');
const ALLOWED_PACKAGES = new Set(['svelte', '@lucide/svelte']);
const FORBIDDEN_CALLS = /\b(fetch|XMLHttpRequest|WebSocket|localStorage|sessionStorage)\s*\(/;

const IMPORT = /(?:import|export)\s+(?:type\s+)?(?:[^'"]*?\s+from\s+)?['"]([^'"]+)['"]|import\(\s*['"]([^'"]+)['"]\s*\)/g;

const offences = [];

function packageName(specifier) {
	if (specifier.startsWith('@')) return specifier.split('/').slice(0, 2).join('/');
	return specifier.split('/')[0];
}

function scan(file) {
	const source = readFileSync(file, 'utf8');
	const isCss = file.endsWith('.css');
	for (const match of source.matchAll(IMPORT)) {
		const specifier = match[1] ?? match[2];
		if (!specifier || isCss) continue;
		const at = `${relative(process.cwd(), file)}  ${specifier}`;
		if (specifier.startsWith('.')) {
			const target = resolve(dirname(file), specifier);
			if (target !== ROOT && !target.startsWith(ROOT + sep)) offences.push(`${at}  (escapes src/lib/ui)`);
		} else if (specifier.startsWith('$')) {
			offences.push(`${at}  (app alias; use a relative path inside ui)`);
		} else if (!ALLOWED_PACKAGES.has(packageName(specifier))) {
			offences.push(`${at}  (package not allowed in the UI artifact)`);
		}
	}
	if (!isCss && FORBIDDEN_CALLS.test(source.replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, ''))) {
		offences.push(`${relative(process.cwd(), file)}  (network or storage call: views render from props)`);
	}
}

function walk(dir) {
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) walk(path);
		else if (/\.(svelte|ts|css)$/.test(entry.name) && !/\.test\.ts$/.test(entry.name)) scan(path);
	}
}

walk(ROOT);

if (offences.length) {
	console.error(`\n${offences.length} import(s) cross the src/lib/ui boundary.\n`);
	for (const offence of offences) console.error('  ' + offence);
	console.error('\nMove the dependency into src/lib/ui, pass it in as a prop, or keep the file outside ui.\n');
	process.exit(1);
}
console.log('ui boundary: src/lib/ui imports only itself, svelte and allowed packages');
