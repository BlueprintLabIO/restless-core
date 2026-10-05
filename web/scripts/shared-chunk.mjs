/* A few large chunks for what company pages share, instead of dozens of small ones.
 *
 * Left to itself the bundler makes a chunk per distinct set of importing pages:
 * about eighty files on a company page. Behind Core's plain HTTP/1.1 gateway
 * each file waits its turn on one of a browser's six connections, and on a
 * slow phone link that queue, not bytes or CPU, set when a page first painted.
 *
 * - `shell`: what the root and company layouts statically need.
 * - `pages`: light code at least two company pages statically need.
 * - `markdown`: the Markdown renderer and the conversation pieces built on it.
 *
 * The editors, the spreadsheet and anything that statically imports them stay
 * behind their own pages and dynamic imports. A static import scan finds those
 * importers, so a new import of a heavy package cannot quietly put it on every
 * page. */

import fs from 'node:fs';
import path from 'node:path';

const EDITOR = /^(@odoo\/|@tiptap\/|@hocuspocus\/|yjs$|@dagrejs\/)/;
const MARKDOWN = /^svelte-streamdown$/;
const STATIC_IMPORT =
	/(?:^|[\s;])(?:import|export)\s+(type\s+)?(?:[^'";]*?\sfrom\s*)?['"]([^'"]+)['"]/g;
const EXTENSIONS = [
	'',
	'.ts',
	'.js',
	'.svelte',
	'.svelte.ts',
	'.svelte.js',
	'/index.ts',
	'/index.js'
];

/**
 * @param {string} dir
 * @param {string[]} [found]
 * @returns {string[]}
 */
function sourceFiles(dir, found = []) {
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) sourceFiles(full, found);
		else if (/\.(svelte|ts|js)$/.test(entry.name) && !/\.test\.ts$/.test(entry.name))
			found.push(full);
	}
	return found;
}

/**
 * @param {string} from
 * @param {string} specifier
 * @param {string} lib
 */
function resolve(from, specifier, lib) {
	let base;
	if (specifier.startsWith('$lib/')) base = path.join(lib, specifier.slice(5));
	else if (specifier === '$lib') base = lib;
	else if (specifier.startsWith('.')) base = path.resolve(path.dirname(from), specifier);
	else return null;
	for (const extension of EXTENSIONS) {
		const candidate = base + extension;
		if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
	}
	return null;
}

/** @typedef {{ imports: Map<string, Set<string>>, editor: Set<string>, markdown: Set<string> }} Graph */

/**
 * Static imports between source files, and which files import a heavy package themselves.
 * @param {string} src
 * @returns {Graph}
 */
function importGraph(src) {
	const lib = path.join(src, 'lib');
	/** @type {Map<string, Set<string>>} */
	const imports = new Map();
	/** @type {Set<string>} */
	const editor = new Set();
	/** @type {Set<string>} */
	const markdown = new Set();
	for (const file of sourceFiles(src)) {
		/** @type {Set<string>} */
		const targets = new Set();
		for (const match of fs.readFileSync(file, 'utf8').matchAll(STATIC_IMPORT)) {
			if (match[1]) continue; // a type-only import is erased from the build
			const specifier = match[2].replace(/\?.*$/, '');
			if (EDITOR.test(specifier)) editor.add(file);
			if (MARKDOWN.test(specifier)) markdown.add(file);
			const target = resolve(file, specifier, lib);
			if (target) targets.add(target);
		}
		imports.set(file, targets);
	}
	return { imports, editor, markdown };
}

/**
 * Files that statically import one of `seeds`, directly or through other source.
 * @param {Graph} graph
 * @param {Set<string>} seeds
 */
function importersOf(graph, seeds) {
	const found = new Set(seeds);
	let grew = true;
	while (grew) {
		grew = false;
		for (const [file, targets] of graph.imports) {
			if (found.has(file)) continue;
			if ([...targets].some((target) => found.has(target))) {
				found.add(file);
				grew = true;
			}
		}
	}
	return found;
}

/**
 * Source outside routes reachable from `starts` without passing through `stop`.
 * @param {Graph} graph
 * @param {string[]} starts
 * @param {Set<string>} stop
 */
function reachable(graph, starts, stop) {
	/** @type {Set<string>} */
	const found = new Set();
	const pending = [...starts];
	while (pending.length) {
		const file = /** @type {string} */ (pending.pop());
		if (found.has(file) || stop.has(file)) continue;
		found.add(file);
		pending.push(...(graph.imports.get(file) ?? []));
	}
	const routes = `${path.sep}routes${path.sep}`;
	return new Set([...found].filter((file) => !file.includes(routes)).map(path.normalize));
}

/** @param {string} root */
export function chunkGroups(root) {
	const src = path.join(root, 'src');
	const graph = importGraph(src);
	const editor = importersOf(graph, graph.editor);
	const markdown = importersOf(graph, graph.markdown);
	const company = path.join(src, 'routes', '[companyId]');
	const pages = [...graph.imports.keys()].filter(
		(file) => file.startsWith(company) && /\+(page|layout)\.svelte$/.test(file)
	);
	const layouts = [
		path.join(src, 'routes', '+layout.svelte'),
		path.join(company, '+layout.svelte')
	];
	const shell = reachable(graph, layouts, new Set([...editor, ...markdown]));
	const light = reachable(graph, pages, new Set([...editor, ...markdown]));
	const rendered = reachable(
		graph,
		pages
			.flatMap((file) => [...(graph.imports.get(file) ?? [])])
			.filter((file) => markdown.has(file)),
		editor
	);
	/** @param {Set<string>} set */
	const has = (set) => (/** @type {string} */ id) =>
		set.has(path.normalize(id.replace(/\?.*$/, '')));
	return [
		{ name: 'shell', priority: 3, test: has(shell) },
		{ name: 'pages', priority: 2, minShareCount: 2, test: has(light) },
		{ name: 'markdown', priority: 1, minShareCount: 2, test: has(rendered) }
	];
}
