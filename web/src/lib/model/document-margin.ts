/* Comments in the margin (Sprint 109 E4): each one sits beside the words it is about, pinned to
 * them by two Yjs relative positions so the mark follows the words through everyone's edits, and
 * the cards stack down the margin without covering each other. */
import type { EditorState } from '@tiptap/pm/state';
import {
	absolutePositionToRelativePosition,
	relativePositionToAbsolutePosition,
	ySyncPluginKey
} from '@tiptap/y-tiptap';
import * as Y from 'yjs';

/** Where a margin comment's words start and end, as Core stores it: base64 relative positions. */
export interface CommentRange {
	from: string;
	to: string;
}

/** The live document a range is read against: the shared Yjs type and its ProseMirror mapping. */
export interface CommentRangeBinding {
	doc: Y.Doc;
	type: Y.XmlFragment;
	/** y-prosemirror's own mapping, from shared types to the nodes drawn from them. */
	mapping: Map<any, any>;
}

/** The collaboration binding behind an editor state; null when the editor is not collaborative
 * or has not drawn the shared document yet. */
export function commentRangeBinding(state: EditorState): CommentRangeBinding | null {
	const sync = ySyncPluginKey.getState(state) as
		| {
				doc?: Y.Doc;
				type?: Y.XmlFragment;
				binding?: { mapping?: Map<unknown, unknown> } | null;
		  }
		| undefined;
	if (!sync?.doc || !sync.type || !sync.binding?.mapping?.size) return null;
	return { doc: sync.doc, type: sync.type, mapping: sync.binding.mapping };
}

function toBase64(bytes: Uint8Array): string {
	let text = '';
	for (const byte of bytes) text += String.fromCharCode(byte);
	return btoa(text);
}

function fromBase64(value: string): Uint8Array | null {
	try {
		const text = atob(value);
		const bytes = new Uint8Array(text.length);
		for (let index = 0; index < text.length; index += 1) bytes[index] = text.charCodeAt(index);
		return bytes;
	} catch {
		return null;
	}
}

/** The words between two editor positions as a range that survives edits; null when the
 * positions cannot be pinned (no live document, or an empty selection). */
export function encodeCommentRange(
	binding: CommentRangeBinding | null,
	from: number,
	to: number
): CommentRange | null {
	if (!binding || to <= from) return null;
	try {
		const start = absolutePositionToRelativePosition(from, binding.type, binding.mapping);
		const end = absolutePositionToRelativePosition(to, binding.type, binding.mapping);
		const encoded = {
			from: toBase64(Y.encodeRelativePosition(start)),
			to: toBase64(Y.encodeRelativePosition(end))
		};
		/* Core keeps a range to 256 characters a side; a longer one is simply not pinned. */
		return encoded.from.length <= 256 && encoded.to.length <= 256 ? encoded : null;
	} catch {
		return null;
	}
}

/** Where a stored range's words are now; null when they were deleted or cannot be read here
 * (another document, or a view without the live document). */
export function resolveCommentRange(
	binding: CommentRangeBinding | null,
	range: CommentRange | null | undefined
): { from: number; to: number } | null {
	if (!binding || !range) return null;
	const start = fromBase64(range.from);
	const end = fromBase64(range.to);
	if (!start || !end) return null;
	try {
		const from = relativePositionToAbsolutePosition(
			binding.doc,
			binding.type,
			Y.decodeRelativePosition(start),
			binding.mapping
		);
		const to = relativePositionToAbsolutePosition(
			binding.doc,
			binding.type,
			Y.decodeRelativePosition(end),
			binding.mapping
		);
		if (from === null || to === null || to <= from) return null;
		return { from, to };
	} catch {
		return null;
	}
}

/** A card waiting for a place in the margin: the height of the words it is about, and its own. */
export interface MarginNote {
	id: string;
	anchor: number;
	height: number;
}

/** Places the cards down the margin, each as near its words as it can be without covering
 * another. The active card sits exactly beside its words; the cards above it move up out of its
 * way and the cards below it move down, as Google Docs does. Answers each card's top. */
export function stackMarginNotes(
	notes: readonly MarginNote[],
	options: { gap?: number; active?: string | null; top?: number } = {}
): Map<string, number> {
	const gap = options.gap ?? 8;
	const floor = options.top ?? 0;
	const ordered = notes.toSorted(
		(left, right) => left.anchor - right.anchor || left.id.localeCompare(right.id)
	);
	const placed = new Map<string, number>();
	if (!ordered.length) return placed;
	const pivot = Math.max(
		0,
		ordered.findIndex((note) => note.id === options.active)
	);
	const pinned = options.active && ordered[pivot]?.id === options.active;
	/* Below the pivot (or from the first card), each card goes down as far as the one above it
	 * needs. */
	const start = pinned ? pivot : 0;
	let next = floor;
	for (let index = start; index < ordered.length; index += 1) {
		const note = ordered[index];
		const top =
			index === start && pinned ? Math.max(floor, note.anchor) : Math.max(next, note.anchor);
		placed.set(note.id, top);
		next = top + note.height + gap;
	}
	if (!pinned) return placed;
	/* Above the pivot, each card goes up as far as the one below it needs. */
	let limit = placed.get(ordered[pivot].id) ?? floor;
	for (let index = pivot - 1; index >= 0; index -= 1) {
		const note = ordered[index];
		const top = Math.min(note.anchor, limit - gap - note.height);
		placed.set(note.id, top);
		limit = top;
	}
	/* Pushed above the top of the page: everything moves down by as much, still in order. */
	const highest = placed.get(ordered[0].id) ?? floor;
	if (highest < floor) {
		const shift = floor - highest;
		for (const [id, top] of placed) placed.set(id, top + shift);
	}
	return placed;
}

/** The height a margin card takes before anyone opens it: rows never grow, so this is fixed. */
export const MARGIN_CARD_HEIGHT = 84;
