/**
 * The composer's keyboard and sizing rules, kept out of the component so they can be
 * tested without a DOM — the same split `desk-map.ts` uses for its view derivations.
 *
 * Enter sends, Shift+Enter makes a newline, Cmd/Ctrl+Enter also sends. Both composers
 * this replaces disagreed about every one of those.
 */

/** The subset of a KeyboardEvent these rules read. */
export interface ComposerKey {
	key: string;
	shiftKey?: boolean;
	/** True while an IME is mid-composition — Enter is committing a candidate, not sending. */
	isComposing?: boolean;
}

export type ComposerAction =
	/** Submit the form. The caller must `preventDefault()` first. */
	| 'send'
	/** Let the textarea insert a line break. */
	| 'newline'
	/** Not our key; leave the event alone. */
	| 'ignore';

/**
 * What a keystroke means in the composer.
 *
 * `composing` is passed separately as well as read off the event because some browsers
 * fire keydown for the IME commit without setting `isComposing`; the component tracks
 * compositionstart/end to cover them. Sending mid-composition swallows the candidate word.
 */
export function composerKeyAction(event: ComposerKey, composing = false): ComposerAction {
	if (event.key !== 'Enter') return 'ignore';
	if (composing || event.isComposing) return 'newline';
	if (event.shiftKey) return 'newline';
	return 'send';
}

/** The textarea's auto-grow height: content height, capped. */
export function nextComposerHeight(scrollHeight: number, max: number): number {
	return Math.min(scrollHeight, max);
}
