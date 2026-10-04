/* The composer's `/`, `$`, `#` and `@` menu, kept pure so its matching rules
 * are testable. `/` opens only at the very start of a message, so paths and
 * fractions in ordinary prose never pop a menu; `$` (skills), `#` (Work and
 * Goals) and `@` (people) open at any word boundary. */

import type { ComposerOption } from '$lib/model/skills';

export type ComposerTriggerChar = '/' | '$' | '#' | '@';
export type ComposerTrigger = {
	trigger: ComposerTriggerChar;
	query: string;
	start: number;
	end: number;
};

export function composerTrigger(value: string, caret: number): ComposerTrigger | null {
	const before = value.slice(0, caret);
	const slash = before.match(/^\/([a-z0-9-]*)$/i);
	if (slash) return { trigger: '/', query: slash[1].toLowerCase(), start: 0, end: caret };
	const word = before.match(/(^|\s)([$#@])([\p{L}\p{N}-]*)$/iu);
	if (word) {
		const start = caret - word[3].length - 1;
		return {
			trigger: word[2] as ComposerTriggerChar,
			query: word[3].toLowerCase(),
			start,
			end: caret
		};
	}
	return null;
}

export function filterComposerOptions(
	options: ComposerOption[],
	trigger: ComposerTriggerChar,
	query: string,
	selected: string[] = []
): ComposerOption[] {
	const kinds: Record<ComposerTriggerChar, ComposerOption['kind'][]> = {
		'/': ['command', 'skill'],
		$: ['skill'],
		'#': ['reference'],
		'@': ['person']
	};
	return options
		.filter((option) => kinds[trigger].includes(option.kind))
		.filter((option) => !(option.kind === 'skill' && selected.includes(option.name)))
		.filter(
			(option) =>
				!query ||
				option.name.includes(query) ||
				option.label.toLowerCase().includes(query) ||
				option.description.toLowerCase().includes(query)
		)
		.sort((left, right) => {
			const rank = (option: ComposerOption) =>
				(option.name.startsWith(query) ? 0 : 2) + (option.kind === 'command' ? 0 : 1);
			return rank(left) - rank(right) || left.name.localeCompare(right.name);
		})
		.slice(0, 8);
}
