/* The pane heading's decisions, as pure functions.
 *
 * A pane heading has three jobs it may or may not be doing at once: naming the region,
 * carrying an on-demand explanation, and expanding to a fuller page. Which shape it takes
 * is a decision, not a rendering detail, so it lives here where a test can reach it without
 * a DOM — the same split `composer-keys.ts` uses for the composer's key contract. */

export type PaneHeaderShape = 'plain' | 'link' | 'action';

/**
 * Which shape the heading takes.
 *
 * `href` and `action` are mutually exclusive and `action` wins. An expand chevron and a
 * button in the same row would mean nesting interactive content inside an `<a>`, which is
 * invalid HTML and unreachable by keyboard in practice — so a pane that has its own control
 * keeps the control and forgoes the chevron. Callers must not pass both; this is the
 * defensive resolution, not a supported combination.
 */
export function paneHeaderShape(options: {
	href?: string | null;
	hasAction?: boolean;
}): PaneHeaderShape {
	if (options.hasAction) return 'action';
	if (options.href) return 'link';
	return 'plain';
}

/**
 * The accessible name for the expand affordance.
 *
 * The whole header row is the hit target, so the link's accessible name has to say where it
 * goes on its own — "Standing authority" alone would announce as a heading that mysteriously
 * navigates. The title is lower-cased only when it is not already an acronym or proper-noun
 * shape, so "Ops" and "AI spend" survive intact.
 */
export function expandLabel(title: string): string {
	const trimmed = title.trim();
	if (!trimmed) return 'Open in full';
	const firstWord = trimmed.split(/\s+/)[0];
	const keepsCase = firstWord !== firstWord.toLowerCase() && firstWord !== capitalize(firstWord);
	const named = keepsCase ? trimmed : lowerFirst(trimmed);
	return `Open ${named} in full`;
}

function capitalize(word: string): string {
	return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
}

function lowerFirst(text: string): string {
	return text.charAt(0).toLowerCase() + text.slice(1);
}
