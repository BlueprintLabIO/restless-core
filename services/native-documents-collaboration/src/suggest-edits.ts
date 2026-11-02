/* How an agent proposes changes instead of making them (Sprint 109 E4). An agent sends the block
 * as it would like it; the service compares it with the block as it is, word by word, and marks
 * the difference as suggestions in the agent's name, for a person to accept or reject in place.
 * Words someone else proposed stay theirs; words this agent proposed and now drops simply go. */
import { randomUUID } from 'node:crypto';
import { Fragment, Mark, type Node as ProseMirrorNode, type Schema } from 'prosemirror-model';
import { Transform } from 'prosemirror-transform';
import { isSuggestionMark, type SuggestionAttrs } from './suggestions.js';

/** A suggestion that cannot be made as asked; the request is wrong, not the document. */
export class InvalidSuggestion extends Error {}

export interface SuggestionAuthor {
	author: string;
	at: string;
	thread: string | null;
}

const LCS_CELLS = 4_000_000;

function suggestionType(schema: Schema, name: 'suggestedInsert' | 'suggestedDelete') {
	const type = schema.marks[name];
	if (!type) throw new InvalidSuggestion('this document schema has no suggestions');
	return type;
}

function cleanMarks(marks: readonly Mark[]): Mark[] {
	return marks.filter((mark) => !isSuggestionMark(mark));
}

/** The same node without any suggestion marks: what was proposed, never whose. */
export function withoutSuggestions(node: ProseMirrorNode): ProseMirrorNode {
	if (node.isText) return node.mark(cleanMarks(node.marks));
	if (node.isLeaf) return node;
	const children: ProseMirrorNode[] = [];
	node.content.forEach((child) => children.push(withoutSuggestions(child)));
	return node.copy(Fragment.fromArray(children));
}

/** The shape of a block with its words left out: two blocks with one shape differ only in words. */
function shape(node: ProseMirrorNode): string {
	if (node.isTextblock) return `${node.type.name}${JSON.stringify(node.attrs)}`;
	const children: string[] = [];
	node.content.forEach((child) => children.push(shape(child)));
	return `${node.type.name}${JSON.stringify(node.attrs)}[${children.join(',')}]`;
}

function textblocks(node: ProseMirrorNode, offset: number): { node: ProseMirrorNode; pos: number }[] {
	const found: { node: ProseMirrorNode; pos: number }[] = [];
	if (node.isTextblock) return [{ node, pos: offset }];
	node.descendants((child, pos) => {
		if (!child.isTextblock) return true;
		found.push({ node: child, pos: offset + 1 + pos });
		return false;
	});
	return found;
}

interface Token {
	key: string;
	text: string;
	marks: readonly Mark[];
	atom: boolean;
	from: number;
	to: number;
}

const WORDS = /\s+|[\p{L}\p{N}\p{M}_'’]+|[^\s\p{L}\p{N}\p{M}_'’]/gu;

function marksKey(marks: readonly Mark[]): string {
	return JSON.stringify(cleanMarks(marks).map((mark) => mark.toJSON()));
}

function tokens(block: ProseMirrorNode, contentStart: number): Token[] {
	const result: Token[] = [];
	block.forEach((child, offset) => {
		const from = contentStart + offset;
		if (!child.isText) {
			result.push({
				key: `atom:${JSON.stringify(child.toJSON())}`,
				text: '',
				marks: child.marks,
				atom: true,
				from,
				to: from + child.nodeSize
			});
			return;
		}
		/* Words already struck are on their way out: a proposal says how the block should read, so
		 * it is compared with the words still standing, and the struck ones stay struck. */
		if (child.marks.some((mark) => mark.type.name === 'suggestedDelete')) return;
		const key = marksKey(child.marks);
		/* ProseMirror counts text in UTF-16 units, as JavaScript strings index it. */
		for (const match of (child.text ?? '').matchAll(WORDS)) {
			const word = match[0];
			const index = match.index ?? 0;
			result.push({
				key: `${word}\u0000${key}`,
				text: word,
				marks: child.marks,
				atom: false,
				from: from + index,
				to: from + index + word.length
			});
		}
	});
	return result;
}

interface Hunk {
	oldStart: number;
	oldEnd: number;
	newStart: number;
	newEnd: number;
}

/** Where two token lists differ, as replaced stretches, by longest common subsequence. */
export function diffTokens(before: readonly string[], after: readonly string[]): Hunk[] {
	let prefix = 0;
	while (prefix < before.length && prefix < after.length && before[prefix] === after[prefix]) prefix += 1;
	let suffix = 0;
	while (
		suffix < before.length - prefix &&
		suffix < after.length - prefix &&
		before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
	)
		suffix += 1;
	const left = before.slice(prefix, before.length - suffix);
	const right = after.slice(prefix, after.length - suffix);
	if (!left.length && !right.length) return [];
	if (!left.length || !right.length || left.length * right.length > LCS_CELLS) {
		return [{ oldStart: prefix, oldEnd: prefix + left.length, newStart: prefix, newEnd: prefix + right.length }];
	}
	const width = right.length + 1;
	const table = new Uint32Array((left.length + 1) * width);
	const cell = (row: number, column: number) => table[row * width + column] ?? 0;
	for (let i = left.length - 1; i >= 0; i -= 1) {
		for (let j = right.length - 1; j >= 0; j -= 1) {
			table[i * width + j] =
				left[i] === right[j] ? cell(i + 1, j + 1) + 1 : Math.max(cell(i + 1, j), cell(i, j + 1));
		}
	}
	const hunks: Hunk[] = [];
	let i = 0;
	let j = 0;
	let open: Hunk | null = null;
	const close = () => {
		if (open) hunks.push(open);
		open = null;
	};
	while (i < left.length || j < right.length) {
		if (i < left.length && j < right.length && left[i] === right[j]) {
			close();
			i += 1;
			j += 1;
		} else {
			open ??= { oldStart: prefix + i, oldEnd: prefix + i, newStart: prefix + j, newEnd: prefix + j };
			/* A word taken out reads before the word put in. */
			if (j < right.length && (i === left.length || cell(i, j + 1) > cell(i + 1, j))) {
				j += 1;
				open.newEnd = prefix + j;
			} else {
				i += 1;
				open.oldEnd = prefix + i;
			}
		}
	}
	close();
	return hunks;
}

function attrsFor(author: SuggestionAuthor, id: string): SuggestionAttrs {
	return { id, author: author.author, at: author.at, thread: author.thread };
}

/** Strike `[from, to)` as a suggested deletion; this author's own pending words simply go. */
function strike(tr: Transform, from: number, to: number, attrs: SuggestionAttrs): void {
	const schema = tr.doc.type.schema;
	const pieces: { from: number; to: number; own: boolean; struck: boolean }[] = [];
	tr.doc.nodesBetween(from, to, (node, pos) => {
		if (!node.isText) return true;
		const start = Math.max(from, pos);
		const end = Math.min(to, pos + node.nodeSize);
		const insert = node.marks.find((mark) => mark.type.name === 'suggestedInsert');
		pieces.push({
			from: start,
			to: end,
			own: insert?.attrs.author === attrs.author,
			struck: node.marks.some((mark) => mark.type.name === 'suggestedDelete')
		});
		return false;
	});
	for (const piece of pieces.toReversed()) {
		if (piece.own) tr.delete(piece.from, piece.to);
		else if (!piece.struck) tr.addMark(piece.from, piece.to, suggestionType(schema, 'suggestedDelete').create(attrs));
	}
}

/**
 * Propose `proposed` in place of the top-level block at `blockPos`: the same structure, other
 * words. Each changed stretch becomes one suggestion, deletion before insertion as people read a
 * replacement. Returns the number of suggestions made.
 */
export function suggestBlockEdit(
	tr: Transform,
	blockPos: number,
	proposed: ProseMirrorNode,
	author: SuggestionAuthor
): number {
	const current = tr.doc.nodeAt(blockPos);
	if (!current) throw new InvalidSuggestion('the block is not in the document');
	const wanted = withoutSuggestions(proposed);
	if (shape(withoutSuggestions(current)) !== shape(wanted)) {
		throw new InvalidSuggestion(
			'a suggestion changes words only; keep the block structure and attributes, or use replace'
		);
	}
	const schema: Schema = tr.doc.type.schema;
	const proposedBlocks = textblocks(wanted, 0);
	const pairs = textblocks(current, blockPos).map((old, index) => ({
		old,
		next: proposedBlocks[index]!.node
	}));
	let made = 0;
	for (const { old, next } of pairs.toReversed()) {
		const before = tokens(old.node, old.pos + 1);
		const after = tokens(next, 0);
		/* Changes a space apart read as one: "all customers" to "every customer" is one suggestion. */
		const hunks: Hunk[] = [];
		for (const hunk of diffTokens(
			before.map((token) => token.key),
			after.map((token) => token.key)
		)) {
			const last = hunks.at(-1);
			const gap = last ? before.slice(last.oldEnd, hunk.oldStart) : [];
			if (last && gap.length && gap.every((token) => !token.atom && !token.text.trim())) {
				last.oldEnd = hunk.oldEnd;
				last.newEnd = hunk.newEnd;
			} else hunks.push({ ...hunk });
		}
		for (const hunk of hunks.toReversed()) {
			const removed = before.slice(hunk.oldStart, hunk.oldEnd);
			const added = after.slice(hunk.newStart, hunk.newEnd);
			if (removed.some((token) => token.atom) || added.some((token) => token.atom)) {
				throw new InvalidSuggestion(
					'mentions, references and line breaks are kept as they are in a suggestion'
				);
			}
			const attrs = attrsFor(author, randomUUID());
			const at =
				hunk.oldEnd > hunk.oldStart
					? before[hunk.oldEnd - 1]!.to
					: hunk.oldStart < before.length
						? before[hunk.oldStart]!.from
						: old.pos + 1 + old.node.content.size;
			if (added.length) {
				const insertMark = suggestionType(schema, 'suggestedInsert').create(attrs);
				const nodes: ProseMirrorNode[] = [];
				for (const token of added) {
					const marks = insertMark.addToSet(cleanMarks(token.marks));
					const last = nodes.at(-1);
					if (last && Mark.sameSet(last.marks, marks)) {
						nodes[nodes.length - 1] = schema.text((last.text ?? '') + token.text, marks);
					} else nodes.push(schema.text(token.text, marks));
				}
				tr.insert(at, Fragment.fromArray(nodes));
			}
			if (removed.length) strike(tr, removed[0]!.from, removed.at(-1)!.to, attrs);
			made += 1;
		}
	}
	return made;
}

/** A new block as one suggested insertion: words only. */
export function suggestedBlock(block: ProseMirrorNode, author: SuggestionAuthor): ProseMirrorNode {
	const attrs = attrsFor(author, randomUUID());
	const mark = suggestionType(block.type.schema, 'suggestedInsert').create(attrs);
	let words = 0;
	const marked = (node: ProseMirrorNode): ProseMirrorNode => {
		if (node.isText) {
			words += 1;
			return node.mark(mark.addToSet(cleanMarks(node.marks)));
		}
		if (node.isInline || (node.isLeaf && !node.isTextblock)) {
			throw new InvalidSuggestion(
				'a suggested block holds words only; pictures, files, rules, mentions and line breaks are added directly'
			);
		}
		const children: ProseMirrorNode[] = [];
		node.content.forEach((child) => children.push(marked(child)));
		return node.copy(Fragment.fromArray(children));
	};
	const result = marked(block);
	if (!words) throw new InvalidSuggestion('a suggested block needs words');
	return result;
}

/** Strike a whole block's words as one suggested deletion; accepting it takes the block out. */
export function suggestBlockDeletion(tr: Transform, blockPos: number, author: SuggestionAuthor): void {
	const block = tr.doc.nodeAt(blockPos);
	if (!block) throw new InvalidSuggestion('the block is not in the document');
	let words = 0;
	block.descendants((node) => {
		if (node.isText) words += 1;
		else if (node.isInline || (node.isLeaf && !node.isTextblock)) {
			throw new InvalidSuggestion(
				'only a block of words can be suggested for deletion; delete pictures, files, rules and mentions directly'
			);
		}
		return true;
	});
	if (!words) throw new InvalidSuggestion('the block has no words to strike');
	strike(tr, blockPos + 1, blockPos + block.nodeSize - 1, attrsFor(author, randomUUID()));
}
