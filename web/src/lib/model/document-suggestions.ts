/* Suggesting (Sprint 109 E4): edits that wait for someone to accept them, as in Google Docs.
 *
 * In suggesting mode every word typed is marked as a suggested insertion in the writer's name, and
 * every word deleted stays in place, struck through, as a suggested deletion. Anyone who may edit
 * accepts or rejects each change, or all of them, in one transaction that reaches every co-editor
 * as one Yjs update. Paragraph breaks, formatting, pictures, mentions and line breaks are not
 * tracked: in suggesting mode a break or a format applies directly, and a picture, mention or line
 * break someone deletes stays (suggest its removal in words, or switch to editing).
 *
 * The marks and how they settle are shared with the documents service and agents
 * (document-suggestions-core.ts); this module adds the editor's side: the marks as Tiptap draws
 * them, the mode, and turning a writer's own transactions into suggestions. */
import { Extension, Mark, type Editor } from '@tiptap/core';
import { isChangeOrigin } from '@tiptap/extension-collaboration';
import {
	Fragment,
	Slice,
	type Mark as ProseMirrorMark,
	type MarkType,
	type Node as ProseMirrorNode
} from '@tiptap/pm/model';
import {
	Plugin,
	PluginKey,
	TextSelection,
	type EditorState,
	type Transaction
} from '@tiptap/pm/state';
import { Mapping, ReplaceStep } from '@tiptap/pm/transform';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import {
	isSuggestionMark,
	resolveSuggestions,
	suggestionAttrs,
	suggestionRanges,
	suggestionsIn,
	type SuggestionAttrs,
	type SuggestionSummary
} from './document-suggestions-core.ts';
import { caretHue } from './page-faces.ts';

export {
	resolveSuggestions,
	suggestionRanges,
	suggestionsIn,
	type SuggestionAttrs,
	type SuggestionSummary
} from './document-suggestions-core.ts';

/** Who is suggesting, and the comment thread their changes answer, if any. */
export interface SuggestingMode {
	actorId: string;
	thread: string | null;
}

export interface SuggestionsState {
	/** Null while editing directly. */
	mode: SuggestingMode | null;
	/** The change whose card is hovered or open; its words are marked more strongly. */
	active: string | null;
	decorations: DecorationSet;
}

type SuggestionsMeta =
	{ mode: SuggestingMode | null } | { active: string | null } | { settled: true };

export const suggestionsKey = new PluginKey<SuggestionsState>('documentSuggestions');

function suggestionMark(name: 'suggestedInsert' | 'suggestedDelete') {
	return Mark.create({
		name,
		inclusive: false,
		addAttributes: () => ({
			id: { default: null, rendered: false },
			author: { default: null, rendered: false },
			at: { default: null, rendered: false },
			thread: { default: null, rendered: false }
		}),
		/* Never read from a paste: a suggestion is made here, in its writer's name, or not at all. */
		parseHTML: () => [],
		renderHTML: ({ mark }) => {
			const attrs = suggestionAttrs(mark as unknown as ProseMirrorMark);
			return [
				name === 'suggestedInsert' ? 'ins' : 'del',
				{
					class: 'suggestion',
					'data-suggestion': attrs.id,
					'data-author': attrs.author,
					datetime: attrs.at,
					...(attrs.thread ? { 'data-thread': attrs.thread } : {}),
					style: `--suggestion-hue: ${caretHue(attrs.author)}`
				},
				0
			];
		}
	});
}

export const SuggestedInsert = suggestionMark('suggestedInsert');
export const SuggestedDelete = suggestionMark('suggestedDelete');

/** The marks a writer's own suggestion takes next to `pos`, so typing on extends one change. */
function neighbour(
	doc: ProseMirrorNode,
	pos: number,
	type: MarkType,
	mode: SuggestingMode,
	side: -1 | 1
): SuggestionAttrs | null {
	const $pos = doc.resolve(Math.max(0, Math.min(pos, doc.content.size)));
	const node = side < 0 ? $pos.nodeBefore : $pos.nodeAfter;
	if (!node?.isText) return null;
	const mark = node.marks.find(
		(candidate) =>
			candidate.type === type &&
			candidate.attrs.author === mode.actorId &&
			(candidate.attrs.thread ?? null) === mode.thread
	);
	return mark ? suggestionAttrs(mark) : null;
}

/** What a deletion leaves in place: everyone's words struck, the writer's own suggested words
 * gone, and anything that is not words (a mention, a picture) kept as it was. */
function struck(
	fragment: Fragment,
	strike: ProseMirrorMark,
	insertType: MarkType,
	deleteType: MarkType,
	author: string
): { fragment: Fragment; kept: boolean } {
	const children: ProseMirrorNode[] = [];
	let kept = false;
	fragment.forEach((node) => {
		if (node.isText) {
			const insert = node.marks.find((mark) => mark.type === insertType);
			if (insert && insert.attrs.author === author) return;
			kept = true;
			children.push(
				node.marks.some((mark) => mark.type === deleteType)
					? node
					: node.mark(strike.addToSet(node.marks))
			);
		} else if (node.isLeaf) {
			kept = true;
			children.push(node);
		} else {
			const inner = struck(node.content, strike, insertType, deleteType, author);
			kept ||= inner.kept;
			children.push(node.copy(inner.fragment));
		}
	});
	return { fragment: Fragment.fromArray(children), kept };
}

/**
 * Turn the writer's own transactions into suggestions: what they typed is marked as theirs, and
 * what they deleted is put back, struck. Returns the transaction to append, or null.
 */
export function trackSuggestions(
	transactions: readonly Transaction[],
	oldState: EditorState,
	newState: EditorState,
	mode: SuggestingMode,
	now: () => string = () => new Date().toISOString(),
	newId: () => string = () => crypto.randomUUID()
): Transaction | null {
	const schema = newState.schema;
	const insertType = schema.marks.suggestedInsert;
	const deleteType = schema.marks.suggestedDelete;
	if (!insertType || !deleteType) return null;
	const maps = transactions.flatMap((tr) => tr.mapping.maps);
	const out = newState.tr;
	let caret: number | null = null;
	let offset = 0;
	for (const [trIndex, tr] of transactions.entries()) {
		const tracked =
			tr.docChanged &&
			!isChangeOrigin(tr) &&
			!tr.getMeta(suggestionsKey) &&
			!tr.getMeta('appendedTransaction');
		tr.steps.forEach((step, index) => {
			if (!tracked || !(step instanceof ReplaceStep)) return;
			const { from, to, slice } = step as unknown as { from: number; to: number; slice: Slice };
			const before = tr.docs[index];
			const rest = new Mapping(maps.slice(offset + index + 1));
			const place = (pos: number, assoc: number) => out.mapping.map(rest.map(pos, assoc), assoc);
			const start = place(from, 1);
			const end = Math.max(start, place(from + slice.size, -1));
			const at = now();

			/* One change per stretch of typing: next to the writer's own insertion or deletion, the
			 * new words join it; a replacement's deletion and insertion share one id. */
			const insertNear =
				neighbour(out.doc, start, insertType, mode, -1) ??
				neighbour(out.doc, end, insertType, mode, 1);
			const deleteNear =
				neighbour(out.doc, start, deleteType, mode, -1) ??
				neighbour(out.doc, end, deleteType, mode, 1);
			const id = insertNear?.id ?? deleteNear?.id ?? newId();
			const fresh = { id, author: mode.actorId, at, thread: mode.thread };

			if (end > start) {
				const mark = insertType.create(insertNear ?? fresh);
				const pieces: { from: number; to: number }[] = [];
				out.doc.nodesBetween(start, end, (node, pos) => {
					if (!node.isText) return true;
					pieces.push({ from: Math.max(start, pos), to: Math.min(end, pos + node.nodeSize) });
					return false;
				});
				for (const piece of pieces) {
					out.removeMark(piece.from, piece.to, deleteType);
					out.addMark(piece.from, piece.to, mark);
				}
			}

			if (to > from && before) {
				const removed = before.slice(from, to);
				const strike = deleteType.create(deleteNear ?? fresh);
				const kept = struck(removed.content, strike, insertType, deleteType, mode.actorId);
				if (kept.kept) {
					const size = out.doc.content.size;
					out.replace(start, start, new Slice(kept.fragment, removed.openStart, removed.openEnd));
					const restored = out.doc.content.size - size;
					/* Backspace leaves the caret before the struck words, so the next Backspace
					 * reaches the word before them; any other deletion leaves it after. */
					const selection = trIndex === 0 && index === 0 ? oldState.selection : null;
					if (slice.size === 0) {
						caret = selection?.empty && selection.head === to ? start : start + restored;
					}
				}
			}
		});
		offset += tr.steps.length;
	}
	if (!out.steps.length) return null;
	if (caret !== null) {
		out.setSelection(TextSelection.near(out.doc.resolve(Math.min(caret, out.doc.content.size))));
	}
	return out.setMeta(suggestionsKey, { settled: true } satisfies SuggestionsMeta);
}

/**
 * Editing directly: words typed inside someone's suggestion are the writer's own, not part of the
 * suggestion (ProseMirror would otherwise carry the surrounding mark onto them, and a struck word
 * would swallow what was typed into it).
 */
export function plainTyping(
	transactions: readonly Transaction[],
	newState: EditorState
): Transaction | null {
	const maps = transactions.flatMap((tr) => tr.mapping.maps);
	const out = newState.tr;
	let offset = 0;
	for (const tr of transactions) {
		const typed =
			tr.docChanged &&
			!isChangeOrigin(tr) &&
			!tr.getMeta(suggestionsKey) &&
			!tr.getMeta('appendedTransaction') &&
			!tr.getMeta('uiEvent');
		tr.steps.forEach((step, index) => {
			if (!typed || !(step instanceof ReplaceStep)) return;
			const { from, slice } = step as unknown as { from: number; slice: Slice };
			if (!slice.size || slice.openStart || slice.openEnd) return;
			let words = true;
			slice.content.forEach((node) => (words &&= node.isText));
			if (!words) return;
			const rest = new Mapping(maps.slice(offset + index + 1));
			const start = out.mapping.map(rest.map(from, 1), 1);
			const end = Math.max(start, out.mapping.map(rest.map(from + slice.size, -1), -1));
			for (const name of ['suggestedInsert', 'suggestedDelete'] as const) {
				const type = newState.schema.marks[name];
				if (type && out.doc.rangeHasMark(start, end, type)) out.removeMark(start, end, type);
			}
		});
		offset += tr.steps.length;
	}
	return out.steps.length
		? out.setMeta(suggestionsKey, { settled: true } satisfies SuggestionsMeta)
		: null;
}

function activeDecorations(doc: ProseMirrorNode, active: string | null): DecorationSet {
	if (!active) return DecorationSet.empty;
	return DecorationSet.create(
		doc,
		suggestionRanges(doc, active).map((range) =>
			Decoration.inline(range.from, range.to, { class: 'active-suggestion' })
		)
	);
}

export function suggestionsPlugin(): Plugin<SuggestionsState> {
	return new Plugin<SuggestionsState>({
		key: suggestionsKey,
		state: {
			init: () => ({ mode: null, active: null, decorations: DecorationSet.empty }),
			apply: (tr, value, _old, state) => {
				const meta = tr.getMeta(suggestionsKey) as SuggestionsMeta | undefined;
				let next = value;
				if (meta && 'mode' in meta) next = { ...next, mode: meta.mode };
				if (meta && 'active' in meta) {
					next = {
						...next,
						active: meta.active,
						decorations: activeDecorations(state.doc, meta.active)
					};
				} else if (tr.docChanged && next.active) {
					next = { ...next, decorations: activeDecorations(state.doc, next.active) };
				}
				return next;
			}
		},
		appendTransaction: (transactions, oldState, newState) => {
			if (!transactions.some((tr) => tr.docChanged)) return null;
			const mode = suggestionsKey.getState(newState)?.mode;
			return mode
				? trackSuggestions(transactions, oldState, newState, mode)
				: plainTyping(transactions, newState);
		},
		props: {
			decorations: (state) => suggestionsKey.getState(state)?.decorations,
			attributes: (state): Record<string, string> =>
				suggestionsKey.getState(state)?.mode ? { class: 'suggesting' } : {},
			/* Copied words read as the change would leave them: struck words stay behind and
			 * proposed ones come plain. A paste is never someone's suggestion. */
			transformCopied: (slice) =>
				new Slice(asIfAccepted(slice.content), slice.openStart, slice.openEnd)
		}
	});
}

function asIfAccepted(fragment: Fragment): Fragment {
	const children: ProseMirrorNode[] = [];
	fragment.forEach((node) => {
		if (node.isText) {
			if (node.marks.some((mark) => mark.type.name === 'suggestedDelete')) return;
			children.push(node.mark(node.marks.filter((mark) => !isSuggestionMark(mark))));
		} else children.push(node.isLeaf ? node : node.copy(asIfAccepted(node.content)));
	});
	return Fragment.fromArray(children);
}

export const suggestionsExtension = Extension.create({
	name: 'documentSuggestions',
	addProseMirrorPlugins: () => [suggestionsPlugin()]
});

/** Suggest (with the writer and the thread the changes answer) or edit directly (null). */
export function setSuggestingMode(editor: Editor, mode: SuggestingMode | null): void {
	if (editor.isDestroyed) return;
	const current = suggestionsKey.getState(editor.state)?.mode ?? null;
	if (current?.actorId === mode?.actorId && current?.thread === mode?.thread) return;
	editor.view.dispatch(editor.state.tr.setMeta(suggestionsKey, { mode } satisfies SuggestionsMeta));
}

export function suggestingMode(state: EditorState): SuggestingMode | null {
	return suggestionsKey.getState(state)?.mode ?? null;
}

export function setActiveSuggestion(editor: Editor, active: string | null): void {
	if (editor.isDestroyed || (suggestionsKey.getState(editor.state)?.active ?? null) === active)
		return;
	editor.view.dispatch(
		editor.state.tr.setMeta(suggestionsKey, { active } satisfies SuggestionsMeta)
	);
}

/* The list is read on every transaction; the same document reads the same list. */
const listed = new WeakMap<ProseMirrorNode, SuggestionSummary[]>();
export function documentSuggestions(state: EditorState): SuggestionSummary[] {
	let found = listed.get(state.doc);
	if (!found) listed.set(state.doc, (found = suggestionsIn(state.doc)));
	return found;
}

/**
 * Accept (keep an insertion, take a deletion out) or reject every change `select` picks, in one
 * transaction. Returns how many changes were settled.
 */
export function settleSuggestions(
	editor: Editor,
	select: (attrs: SuggestionAttrs) => boolean,
	accept: boolean
): number {
	if (editor.isDestroyed || !editor.isEditable) return 0;
	const tr = editor.state.tr;
	const settled = resolveSuggestions(tr, select, accept);
	if (!settled.length) return 0;
	editor.view.dispatch(tr.setMeta(suggestionsKey, { settled: true } satisfies SuggestionsMeta));
	return settled.length;
}

/** The change at the caret, if the caret is in (or at the edge of) suggested words. */
export function suggestionAtSelection(state: EditorState): string | null {
	const { $from, empty } = state.selection;
	const marks = [
		...($from.nodeAfter?.marks ?? []),
		...(empty ? ($from.nodeBefore?.marks ?? []) : [])
	];
	const mark = marks.find(isSuggestionMark);
	return mark ? String(mark.attrs.id) : null;
}

/** How a change reads in a card: "Add “…”", "Delete “…”", or "Replace “…” with “…”". */
export function suggestionSentence(
	summary: Pick<SuggestionSummary, 'kind' | 'inserted' | 'deleted'>
): string {
	const quote = (value: string) => `“${value.replace(/\s+/gu, ' ').trim()}”`;
	if (summary.kind === 'replace')
		return `Replace ${quote(summary.deleted)} with ${quote(summary.inserted)}`;
	return summary.kind === 'insert'
		? `Add ${quote(summary.inserted)}`
		: `Delete ${quote(summary.deleted)}`;
}

/** Whether a saved body holds any open suggestion: a download then offers the agreed text too. */
export function contentHasSuggestions(value: unknown): boolean {
	if (!value || typeof value !== 'object') return false;
	const node = value as { marks?: unknown; content?: unknown };
	if (
		Array.isArray(node.marks) &&
		node.marks.some(
			(mark) =>
				(mark as { type?: unknown })?.type === 'suggestedInsert' ||
				(mark as { type?: unknown })?.type === 'suggestedDelete'
		)
	)
		return true;
	return Array.isArray(node.content) && node.content.some(contentHasSuggestions);
}
