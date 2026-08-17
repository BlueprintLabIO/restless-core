/* The type ramp regrows unless something stops it.
 *
 * Before the ramp existed this design system had 24 distinct font sizes, 13 of
 * them fractional and 64 declarations below 10px. Not one of those was a
 * decision anybody defended — they accumulated because nothing failed when the
 * 25th appeared. Discipline alone loses to a deadline, so the check is here.
 *
 * Sizes are chosen in tokens.css. Everywhere else asks for a step by name. */

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const DESIGN = 'src/lib/design';
const TOKENS = 'tokens.css';

/* An absolute size in `font-size:` or the `font:` shorthand.
 *
 * `em` and `%` are deliberately allowed: they are relative to whichever step
 * they inherit, so Markdown setting its <h3> at 1.08em stays on the ramp
 * wherever the ramp puts it. A px is a new size; an em is a proportion. */
const RAW_SIZE = /(?:font-size|font)\s*:[^;{}]*?(?<![\w-])(\d+(?:\.\d+)?)(px|pt|rem)\b/g;

const offences = [];

function scan(file, source) {
	for (const match of source.matchAll(RAW_SIZE)) {
		/* line-height in the `font:` shorthand is written `13px/1.4`; the size is
		 * what we police, and it has already been captured separately. */
		const before = source.slice(Math.max(0, match.index), match.index + match[0].length);
		if (/\/\s*$/.test(before.slice(0, before.lastIndexOf(match[1])))) continue;
		const line = source.slice(0, match.index).split('\n').length;
		offences.push(`${file}:${line}  ${match[0].replace(/\s+/g, ' ').trim()}`);
	}
}

for (const entry of readdirSync(DESIGN)) {
	if (!entry.endsWith('.css') || entry === TOKENS) continue;
	scan(join(DESIGN, entry), readFileSync(join(DESIGN, entry), 'utf8'));
}

/* Components must not hand-roll type either. */
function walk(dir) {
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) walk(path);
		else if (entry.name.endsWith('.svelte')) scan(path, readFileSync(path, 'utf8'));
	}
}
walk('src/lib');
walk('src/routes');

if (offences.length) {
	console.error(
		`\n${offences.length} raw font size(s) outside ${DESIGN}/${TOKENS}.\n` +
			`Use a step — --t-label, --t-body, --t-head, --t-title, --t-hero — or add one there\n` +
			`with a reason. A size one pixel from its neighbour is a weight or an ink change.\n`
	);
	for (const offence of offences) console.error('  ' + offence);
	process.exit(1);
}

console.log('type: no raw font sizes outside the ramp');
