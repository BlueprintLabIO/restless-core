/* Reading a small text file in place: a CSV or TSV as rows, JSON indented, anything else as it
 * is. The cockpit shows these itself because a browser downloads CSV and frames Markdown raw. */

export type TextFormat = 'markdown' | 'csv' | 'tsv' | 'json' | 'text';

/** The format a file name reads as, or null when it is not a text file the cockpit reads. */
export function textFormat(name: string): TextFormat | null {
	const extension = name.split(/[?#]/)[0].match(/\.([a-z0-9]+)$/i)?.[1]?.toLowerCase();
	switch (extension) {
		case 'md':
		case 'markdown':
			return 'markdown';
		case 'csv':
			return 'csv';
		case 'tsv':
			return 'tsv';
		case 'json':
			return 'json';
		case 'txt':
		case 'log':
			return 'text';
		default:
			return null;
	}
}

/** Rows of a CSV (RFC 4180 quoting) or TSV, at most `limit` of them. */
export function parseDelimited(
	text: string,
	delimiter: ',' | '\t',
	limit = 2000
): { rows: string[][]; truncated: boolean } {
	const rows: string[][] = [];
	let row: string[] = [];
	let field = '';
	let quoted = false;
	for (let index = 0; index < text.length; index += 1) {
		const char = text[index];
		if (quoted) {
			if (char === '"' && text[index + 1] === '"') {
				field += '"';
				index += 1;
			} else if (char === '"') quoted = false;
			else field += char;
			continue;
		}
		if (char === '"' && field === '') quoted = true;
		else if (char === delimiter) {
			row.push(field);
			field = '';
		} else if (char === '\n' || char === '\r') {
			if (char === '\r' && text[index + 1] === '\n') index += 1;
			row.push(field);
			field = '';
			rows.push(row);
			row = [];
			if (rows.length >= limit) return { rows, truncated: index + 1 < text.length };
		} else field += char;
	}
	if (field !== '' || row.length) {
		row.push(field);
		rows.push(row);
	}
	return { rows, truncated: false };
}

/** JSON indented for reading, or the text as written when it does not parse. */
export function readableJson(text: string): string {
	try {
		return JSON.stringify(JSON.parse(text), null, 2);
	} catch {
		return text;
	}
}
