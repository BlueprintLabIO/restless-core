/* A token that does not exist resolves to nothing, and nothing fails.
 *
 * `var(--text-primary)` with no `--text-primary` anywhere makes the declaration invalid at
 * computed-value time: a colour becomes the inherited one, a border loses its colour, and a
 * `font:` shorthand drops entirely. The reconnecting mark drew as an empty square for this
 * reason (its tokens were out of scope), and two dozen components asked for names that were
 * never defined. So every custom property read without a fallback must be declared somewhere
 * in src: in a stylesheet (`--name:`), a `style:--name` directive, or a string a script sets.
 *
 * A read with a fallback (`var(--knob, 240px)`) is an optional knob and is not policed. */

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const files = [];
function walk(dir) {
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) walk(path);
		else if (/\.(svelte|css|ts|js)$/.test(entry.name)) files.push(path);
	}
}
walk('src');

const declared = new Set();
const reads = [];
for (const file of files) {
	const source = readFileSync(file, 'utf8');
	/* Declarations: `--name:` in CSS or a style string, `style:--name`, '--name' set by script. */
	for (const match of source.matchAll(/(?<![\w(-])--([a-z0-9-]+)\s*[:=]|style:--([a-z0-9-]+)|['"`]--([a-z0-9-]+)['"`]/gi))
		declared.add(match[1] ?? match[2] ?? match[3]);
	for (const match of source.matchAll(/var\(\s*--([a-z0-9-]+)\s*\)/gi)) {
		const line = source.slice(0, match.index).split('\n').length;
		reads.push({ name: match[1], at: `${file}:${line}` });
	}
}

const offences = reads.filter((read) => !declared.has(read.name));
if (offences.length) {
	console.error(
		`\n${offences.length} read(s) of a custom property that nothing declares.\n` +
			`Use a token from src/lib/ui/style/tokens.css, or give the read a fallback if it is an optional knob.\n`
	);
	for (const offence of offences) console.error(`  ${offence.at}  var(--${offence.name})`);
	process.exit(1);
}
console.log(`tokens: all ${reads.length} fallback-free custom property reads resolve to a declaration`);
