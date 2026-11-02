/* Tracked changes (Sprint 109 E4), shared by the documents service and the editor.
 *
 * A suggestion is a `suggestedInsert` or `suggestedDelete` mark on text, carrying its id, author,
 * time and the comment thread it answers. One id is one change: a replacement is a deletion and an
 * insertion under the same id, and a change may span several runs and blocks. Accepting keeps an
 * insertion's words and takes a deletion's out; rejecting does the opposite. Either way the marks
 * go, in one transform, so it reaches every co-editor as one Yjs transaction.
 *
 * SHARED CORE: web/src/lib/model/document-suggestions-core.ts is this file with the editor's
 * imports. web/src/lib/model/document-suggestions.test.ts keeps the two the same below the imports.
 */
import type { Mark, Node as ProseMirrorNode } from 'prosemirror-model';
import { Transform, canJoin } from 'prosemirror-transform';

export interface SuggestionAttrs {
	id: string;
	author: string;
	at: string;
	thread: string | null;
}

export type SuggestionKind = 'insert' | 'delete' | 'replace';

/** One change as people (and agents) read it: who proposed what, and where. */
export interface SuggestionSummary {
	id: string;
	kind: SuggestionKind;
	author: string;
	at: string;
	thread: string | null;
	/** The words proposed, and the words proposed for removal; each at most SUMMARY_CHARACTERS. */
	inserted: string;
	deleted: string;
	/** The first and last place the change covers in the document now. */
	from: number;
	to: number;
	/** The top-level block it starts in. */
	block_id: string | null;
}

export const SUGGESTION_MARK_NAMES = ['suggestedInsert', 'suggestedDelete'] as const;
const SUMMARY_CHARACTERS = 500;

export function isSuggestionMark(mark: Mark): boolean {
	return mark.type.name === 'suggestedInsert' || mark.type.name === 'suggestedDelete';
}

export function suggestionAttrs(mark: Mark): SuggestionAttrs {
	return {
		id: String(mark.attrs.id),
		author: String(mark.attrs.author),
		at: String(mark.attrs.at),
		thread: typeof mark.attrs.thread === 'string' ? mark.attrs.thread : null
	};
}

function clip(value: string): string {
	const characters = [...value];
	return characters.length > SUMMARY_CHARACTERS
		? `${characters.slice(0, SUMMARY_CHARACTERS - 1).join('')}…`
		: value;
}

/** Every open suggestion in the document, in the order they first appear. */
export function suggestionsIn(doc: ProseMirrorNode): SuggestionSummary[] {
	const found = new Map<
		string,
		SuggestionSummary & {
			insertBlock: number;
			deleteBlock: number;
			hasInsert: boolean;
			hasDelete: boolean;
		}
	>();
	doc.descendants((node, pos) => {
		if (!node.isText) return true;
		for (const mark of node.marks) {
			if (!isSuggestionMark(mark)) continue;
			const attrs = suggestionAttrs(mark);
			const $pos = doc.resolve(pos);
			const textblock = $pos.before($pos.depth);
			let entry = found.get(attrs.id);
			if (!entry) {
				const top = $pos.depth >= 1 ? $pos.node(1).attrs.block_id : null;
				entry = {
					...attrs,
					kind: 'insert',
					inserted: '',
					deleted: '',
					from: pos,
					to: pos + node.nodeSize,
					block_id: typeof top === 'string' ? top : null,
					insertBlock: -1,
					deleteBlock: -1,
					hasInsert: false,
					hasDelete: false
				};
				found.set(attrs.id, entry);
			}
			if (attrs.at > entry.at) entry.at = attrs.at;
			entry.to = Math.max(entry.to, pos + node.nodeSize);
			if (mark.type.name === 'suggestedInsert') {
				if (entry.hasInsert && entry.insertBlock !== textblock) entry.inserted += '\n';
				entry.inserted += node.text ?? '';
				entry.insertBlock = textblock;
				entry.hasInsert = true;
			} else {
				if (entry.hasDelete && entry.deleteBlock !== textblock) entry.deleted += '\n';
				entry.deleted += node.text ?? '';
				entry.deleteBlock = textblock;
				entry.hasDelete = true;
			}
		}
		return false;
	});
	return [...found.values()].map(
		({ insertBlock: _i, deleteBlock: _d, hasInsert, hasDelete, ...entry }) => ({
			...entry,
			kind: hasInsert && hasDelete ? 'replace' : hasInsert ? 'insert' : 'delete',
			inserted: clip(entry.inserted),
			deleted: clip(entry.deleted)
		})
	);
}

/** Where a change's words are now, as ranges of text, in document order. */
export function suggestionRanges(doc: ProseMirrorNode, id: string): { from: number; to: number }[] {
	const ranges: { from: number; to: number }[] = [];
	doc.descendants((node, pos) => {
		if (!node.isText) return true;
		if (node.marks.some((mark) => isSuggestionMark(mark) && mark.attrs.id === id)) {
			const last = ranges.at(-1);
			if (last && last.to === pos) last.to = pos + node.nodeSize;
			else ranges.push({ from: pos, to: pos + node.nodeSize });
		}
		return false;
	});
	return ranges;
}

/* A block a whole change empties goes with its words: a list item, quote or list it leaves empty
 * goes too. A table cell, the first paragraph of a list item with more in it, and the document's
 * last block stay, empty. */
const EMPTIED_CONTAINERS = new Set([
	'listItem',
	'taskItem',
	'blockquote',
	'bulletList',
	'orderedList',
	'taskList'
]);

function deleteEmptiedBlock(tr: Transform, pos: number): void {
	const node = tr.doc.nodeAt(pos);
	if (!node || !node.isTextblock || node.content.size) return;
	const $pos = tr.doc.resolve(pos);
	let depth = $pos.depth;
	let from = pos;
	let to = pos + node.nodeSize;
	while (depth > 0) {
		const parent = $pos.node(depth);
		if (!EMPTIED_CONTAINERS.has(parent.type.name)) return;
		if (parent.childCount !== 1) {
			const first = $pos.index(depth) === 0;
			if (first && (parent.type.name === 'listItem' || parent.type.name === 'taskItem')) return;
			break;
		}
		from = $pos.before(depth);
		to = $pos.after(depth);
		depth -= 1;
	}
	if (depth === 0 && tr.doc.childCount === 1 && from === 0 && to === tr.doc.content.size) return;
	tr.delete(from, to);
}

/**
 * Accept or reject every suggestion `select` picks, in `tr`. Returns the ids it settled.
 *
 * A deletion that runs from the end of one paragraph into the next joins them when accepted, as
 * the person deleting meant; a block the change empties goes (deleteEmptiedBlock).
 */
export function resolveSuggestions(
	tr: Transform,
	select: (attrs: SuggestionAttrs) => boolean,
	accept: boolean
): string[] {
	const start = tr.steps.length;
	const doc = tr.doc;
	const ids = new Set<string>();
	interface Segment {
		from: number;
		to: number;
		removeText: boolean;
		removedBy: string;
		marks: Mark[];
		block: number;
		blockEnd: number;
		blockStart: number;
	}
	const segments: Segment[] = [];
	const removedPerBlock = new Map<number, number>();
	doc.descendants((node, pos) => {
		if (!node.isText) return true;
		const chosen = node.marks.filter(
			(mark) => isSuggestionMark(mark) && select(suggestionAttrs(mark))
		);
		if (!chosen.length) return false;
		for (const mark of chosen) ids.add(String(mark.attrs.id));
		const remover = chosen.find((mark) => (mark.type.name === 'suggestedInsert') !== accept);
		const $pos = doc.resolve(pos);
		const block = $pos.before($pos.depth);
		segments.push({
			from: pos,
			to: pos + node.nodeSize,
			removeText: Boolean(remover),
			removedBy: remover ? String(remover.attrs.id) : '',
			marks: chosen,
			block,
			blockStart: $pos.start($pos.depth),
			blockEnd: $pos.end($pos.depth)
		});
		if (remover) removedPerBlock.set(block, (removedPerBlock.get(block) ?? 0) + node.nodeSize);
		return false;
	});
	if (!segments.length) return [];

	const emptied = new Set<number>();
	for (const [block, removed] of removedPerBlock) {
		const node = doc.nodeAt(block);
		if (node && node.content.size > 0 && removed === node.content.size) emptied.add(block);
	}
	/* Paragraph breaks a deletion crossed: the end of one block and the start of the next, both
	 * taken out by the same change, in blocks side by side that both keep other words. */
	const joinsAcross = (left: Segment, right: Segment): boolean => {
		if (left.block === right.block || left.removedBy !== right.removedBy) return false;
		if (left.to !== left.blockEnd || right.from !== right.blockStart) return false;
		if (emptied.has(left.block) || emptied.has(right.block)) return false;
		const leftNode = doc.nodeAt(left.block);
		return Boolean(leftNode && left.block + leftNode.nodeSize === right.block);
	};
	const joins: number[] = [];
	let previous: Segment | undefined;
	for (const segment of segments) {
		if (!segment.removeText) continue;
		if (previous && joinsAcross(previous, segment)) joins.push(segment.block);
		previous = segment;
	}

	for (const segment of segments.toReversed()) {
		if (segment.removeText) tr.delete(segment.from, segment.to);
		else for (const mark of segment.marks) tr.removeMark(segment.from, segment.to, mark);
	}
	const mapping = () => tr.mapping.slice(start);
	for (const boundary of joins.toReversed()) {
		const at = mapping().map(boundary);
		if (canJoin(tr.doc, at)) tr.join(at);
	}
	for (const block of [...emptied].sort((left, right) => right - left)) {
		deleteEmptiedBlock(tr, mapping().map(block));
	}
	return [...ids];
}
