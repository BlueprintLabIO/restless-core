/* The cockpit places an agent names in its text ("Needs you: Company › Charter") become links to
 * them (S85-07). "Company › X" (or →, >) is linked wherever it appears; a bare section name
 * ("Apps", "Library") only on a "Needs you" line, where it can only mean the place. Code, and
 * text that is already a link, are left alone. */

const COMPANY_SECTIONS: Record<string, string> = {
	charter: 'company/charter',
	'brand & voice': 'company/identity',
	'brand and voice': 'company/identity',
	brand: 'company/identity',
	identity: 'company/identity',
	intelligence: 'company/provider',
	limits: 'company/resources',
	members: 'company/members',
	schedules: 'company/schedules',
	vault: 'company/vault',
	connections: 'company/connections',
	skills: 'company/skills',
	health: 'company/doctor'
};
const SECTIONS: Record<string, string> = {
	apps: 'apps',
	library: 'library',
	work: 'work',
	people: 'people',
	inbox: ''
};

const COMPANY_PLACE = new RegExp(
	`\\bCompany\\s*(?:›|→|->|>)\\s*(${Object.keys(COMPANY_SECTIONS)
		.sort((a, b) => b.length - a.length)
		.map((name) => name.replace(/&/g, '&'))
		.join('|')})\\b`,
	'gi'
);
const NEEDS_YOU = /^(\s*(?:[-*]\s+)?(?:\*\*)?needs you(?:\*\*)?\s*:)/i;
const BARE_PLACE = new RegExp(`\\b(${Object.keys(SECTIONS).join('|')})\\b`, 'gi');

/* Spans already a link, or code: `[text](url)`, `<url>`, `` `code` ``. */
const PROTECTED = /\[[^\]]*\]\([^)]*\)|`[^`]*`|<[^>\s]+>/g;

function linkOutside(
	line: string,
	pattern: RegExp,
	link: (match: RegExpExecArray) => string
): string {
	let out = '';
	let last = 0;
	for (const guard of line.matchAll(PROTECTED)) {
		out += replaceIn(line.slice(last, guard.index), pattern, link) + guard[0];
		last = (guard.index ?? 0) + guard[0].length;
	}
	return out + replaceIn(line.slice(last), pattern, link);
}

function replaceIn(
	text: string,
	pattern: RegExp,
	link: (match: RegExpExecArray) => string
): string {
	pattern.lastIndex = 0;
	return text.replace(pattern, (...args) => {
		const groups = args.slice(0, -2) as string[];
		const match = Object.assign([...groups], {
			index: args[args.length - 2] as number,
			input: text
		}) as unknown as RegExpExecArray;
		return link(match);
	});
}

export function linkCockpitPlaces(text: string, companyRoot: string): string {
	if (!text || !companyRoot) return text;
	let fenced = false;
	return text
		.split('\n')
		.map((line) => {
			if (/^\s{0,3}(```|~~~)/.test(line)) {
				fenced = !fenced;
				return line;
			}
			if (fenced) return line;
			let next = linkOutside(line, COMPANY_PLACE, (match) => {
				const path = COMPANY_SECTIONS[match[1].toLowerCase()];
				return path ? `[${match[0]}](${companyRoot}/${path})` : match[0];
			});
			const needs = NEEDS_YOU.exec(next);
			if (needs) {
				const head = needs[1];
				next =
					head +
					linkOutside(next.slice(head.length), BARE_PLACE, (match) => {
						const path = SECTIONS[match[1].toLowerCase()];
						return `[${match[0]}](${companyRoot}${path ? `/${path}` : ''})`;
					});
			}
			return next;
		})
		.join('\n');
}
