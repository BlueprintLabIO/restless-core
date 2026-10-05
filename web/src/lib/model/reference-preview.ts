/* What a chip in a message points at, in a line or two, so hovering a
 * reference answers "which one is that?" without leaving the conversation.
 * The company layout provides it from data it already holds. */
import { getContext, setContext } from 'svelte';

export type ReferencePreview = { title: string; lines: string[] };
type Resolve = (href: string) => ReferencePreview | null;

const KEY = Symbol('reference-preview');

export function provideReferencePreview(resolve: Resolve) {
	setContext(KEY, resolve);
}

export function referencePreview(): Resolve | null {
	return getContext<Resolve | undefined>(KEY) ?? null;
}
