/* What changed between two versions of a document, block by block, as a reader would say it:
 * this paragraph was added, that one removed, this one reworded (and which words). Blocks keep
 * their identity across versions, so a moved or edited block is matched to itself. */
import { documentText, type DocumentContent, type DocumentNode } from './documents.ts';

export type WordChange = { op: 'same' | 'added' | 'removed'; text: string };

export type BlockChange =
	| { kind: 'added' | 'removed'; id: string; text: string }
	| { kind: 'changed'; id: string; words: WordChange[] };

const LABELS: Record<string, string> = {
	image: 'An image',
	horizontalRule: 'A divider',
	table: 'A table'
};

function blockText(node: DocumentNode): string {
	const text = documentText(node).trim();
	return text || LABELS[node.type] || '';
}

function blocks(content: DocumentContent | null): { id: string; node: DocumentNode }[] {
	return (content?.content ?? []).map((node, index) => {
		const id = node.attrs?.block_id;
		return { id: typeof id === 'string' && id ? id : `#${index}`, node };
	});
}

/** Beyond this many words a side, a reworded block reads as replaced rather than diffed. */
const MAX_DIFF_WORDS = 400;

/** The words of `after` against `before`: kept, added or removed, in reading order. */
export function wordChanges(before: string, after: string): WordChange[] {
	const left = before.split(/(\s+)/u).filter(Boolean);
	const right = after.split(/(\s+)/u).filter(Boolean);
	if (left.length > MAX_DIFF_WORDS || right.length > MAX_DIFF_WORDS) {
		return [
			{ op: 'removed', text: before },
			{ op: 'added', text: after }
		].filter((part) => part.text) as WordChange[];
	}
	/* Longest common subsequence over words and the spaces between them. */
	const rows = left.length + 1;
	const cols = right.length + 1;
	const table = new Uint16Array(rows * cols);
	for (let i = left.length - 1; i >= 0; i -= 1)
		for (let j = right.length - 1; j >= 0; j -= 1)
			table[i * cols + j] =
				left[i] === right[j]
					? table[(i + 1) * cols + j + 1] + 1
					: Math.max(table[(i + 1) * cols + j], table[i * cols + j + 1]);
	const parts: WordChange[] = [];
	const push = (op: WordChange['op'], text: string) => {
		const last = parts.at(-1);
		if (last?.op === op) last.text += text;
		else parts.push({ op, text });
	};
	let i = 0;
	let j = 0;
	while (i < left.length && j < right.length) {
		if (left[i] === right[j]) {
			push('same', left[i]);
			i += 1;
			j += 1;
		} else if (table[(i + 1) * cols + j] >= table[i * cols + j + 1]) push('removed', left[i++]);
		else push('added', right[j++]);
	}
	while (i < left.length) push('removed', left[i++]);
	while (j < right.length) push('added', right[j++]);
	/* A change that is only spacing is not one. */
	return parts.map((part) =>
		part.op !== 'same' && !part.text.trim() ? { ...part, op: 'same' as const } : part
	);
}

/** The blocks that differ, in the order the newer version reads, with removed blocks where they
 * stood. Unchanged blocks are left out. */
export function versionChanges(
	before: DocumentContent | null,
	after: DocumentContent
): BlockChange[] {
	const old = blocks(before);
	const next = blocks(after);
	const oldIndex = new Map(old.map((block, index) => [block.id, index]));
	const kept = new Set(next.map((block) => block.id));
	const changes: BlockChange[] = [];
	let cursor = 0;
	const removedUpTo = (end: number) => {
		for (; cursor < end; cursor += 1) {
			const block = old[cursor];
			if (!kept.has(block.id)) {
				const text = blockText(block.node);
				if (text) changes.push({ kind: 'removed', id: block.id, text });
			}
		}
	};
	for (const block of next) {
		const at = oldIndex.get(block.id);
		const text = blockText(block.node);
		if (at === undefined) {
			if (text) changes.push({ kind: 'added', id: block.id, text });
			continue;
		}
		removedUpTo(at);
		cursor = Math.max(cursor, at + 1);
		const previous = blockText(old[at].node);
		if (previous !== text)
			changes.push({ kind: 'changed', id: block.id, words: wordChanges(previous, text) });
	}
	removedUpTo(old.length);
	return changes;
}
