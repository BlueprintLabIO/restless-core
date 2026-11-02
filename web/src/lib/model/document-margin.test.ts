import assert from 'node:assert/strict';
import test from 'node:test';
import { getSchema } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { initProseMirrorDoc, prosemirrorJSONToYXmlFragment } from '@tiptap/y-tiptap';
import * as Y from 'yjs';
import { createDocumentEditorExtensions } from './document-editor-schema.ts';
import {
	encodeCommentRange,
	resolveCommentRange,
	stackMarginNotes,
	type CommentRangeBinding
} from './document-margin.ts';

function live(document: Y.Doc, fragment: Y.XmlFragment) {
	const schema = getSchema(createDocumentEditorExtensions(document, {} as never, true));
	const { doc, mapping } = initProseMirrorDoc(fragment, schema) as {
		doc: ProseMirrorNode;
		mapping: CommentRangeBinding['mapping'];
	};
	return { doc, binding: { doc: document, type: fragment, mapping } satisfies CommentRangeBinding };
}

function wordsAt(doc: ProseMirrorNode, word: string): { from: number; to: number } {
	let found: { from: number; to: number } | null = null;
	doc.descendants((node, pos) => {
		const at = node.isText ? (node.text ?? '').indexOf(word) : -1;
		if (!found && at >= 0) found = { from: pos + at, to: pos + at + word.length };
	});
	assert.ok(found, `"${word}" is in the document`);
	return found;
}

test('a margin comment range follows its words through edits made elsewhere', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const fragment = document.getXmlFragment('default');
	const schema = getSchema(createDocumentEditorExtensions(document, {} as never, true));
	prosemirrorJSONToYXmlFragment(
		schema,
		{
			type: 'doc',
			content: [
				{
					type: 'paragraph',
					attrs: { block_id: 'a' },
					content: [{ type: 'text', text: 'Pricing goes out' }]
				},
				{
					type: 'paragraph',
					attrs: { block_id: 'b' },
					content: [{ type: 'text', text: 'Ship the price list on Friday' }]
				}
			]
		},
		fragment
	);
	const before = live(document, fragment);
	const words = wordsAt(before.doc, 'Friday');
	const range = encodeCommentRange(before.binding, words.from, words.to);
	assert.ok(range, 'the words can be pinned');
	assert.match(range.from, /^[A-Za-z0-9+/]+=*$/u);
	assert.ok(range.from.length <= 256 && range.to.length <= 256);

	/* Someone else types above it, and before it in the same line, through Yjs. */
	const other = new Y.Doc();
	context.after(() => other.destroy());
	Y.applyUpdate(other, Y.encodeStateAsUpdate(document));
	const [first, second] = other.getXmlFragment('default').toArray() as Y.XmlElement[];
	(first.get(0) as Y.XmlText).insert(0, 'New: ');
	(second.get(0) as Y.XmlText).insert(0, 'Please ');
	Y.applyUpdate(document, Y.encodeStateAsUpdate(other));

	const after = live(document, fragment);
	const moved = resolveCommentRange(after.binding, range);
	assert.ok(moved, 'the range still resolves');
	assert.equal(after.doc.textBetween(moved.from, moved.to), 'Friday');
	assert.notDeepEqual(moved, words, 'the words moved, and the range with them');

	/* The words deleted: nothing to point at, so the page falls back to the block. */
	const text = second.get(0) as Y.XmlText;
	const otherText = text.toString();
	text.delete(otherText.indexOf('Friday'), 'Friday'.length);
	Y.applyUpdate(document, Y.encodeStateAsUpdate(other));
	assert.equal(resolveCommentRange(live(document, fragment).binding, range), null);
});

test('a range is not pinned without a live document, and a damaged one never resolves', () => {
	assert.equal(encodeCommentRange(null, 1, 4), null);
	assert.equal(resolveCommentRange(null, { from: 'AAAA', to: 'AAAA' }), null);
	const document = new Y.Doc();
	const fragment = document.getXmlFragment('default');
	const binding = { doc: document, type: fragment, mapping: new Map() };
	assert.equal(encodeCommentRange(binding, 4, 4), null, 'an empty selection is not a range');
	assert.equal(resolveCommentRange(binding, { from: '!!', to: '%%' }), null);
	document.destroy();
});

test('margin cards sit beside their words and never cover each other', () => {
	const notes = [
		{ id: 'c', anchor: 300, height: 84 },
		{ id: 'a', anchor: 100, height: 84 },
		{ id: 'b', anchor: 120, height: 84 }
	];
	const tops = stackMarginNotes(notes, { gap: 8 });
	assert.equal(tops.get('a'), 100, 'the first card is level with its words');
	assert.equal(tops.get('b'), 192, 'the next card goes just below it');
	assert.equal(tops.get('c'), 300, 'a card with room stays beside its words');
	const order = ['a', 'b', 'c'];
	for (let index = 1; index < order.length; index += 1) {
		const above = tops.get(order[index - 1]) ?? 0;
		assert.ok((tops.get(order[index]) ?? 0) >= above + 84 + 8, 'no overlap');
	}
});

test('the opened card sits level with its words and the others move out of its way', () => {
	const notes = [
		{ id: 'a', anchor: 100, height: 84 },
		{ id: 'b', anchor: 120, height: 200 },
		{ id: 'c', anchor: 150, height: 84 }
	];
	const tops = stackMarginNotes(notes, { gap: 8, active: 'b' });
	assert.equal(tops.get('b'), 120);
	assert.equal(tops.get('a'), 120 - 8 - 84, 'the card above moves up');
	assert.equal(tops.get('c'), 120 + 200 + 8, 'the card below moves down');

	/* Moving up past the top of the page shifts everything down, still in order. */
	const crowded = stackMarginNotes(
		[
			{ id: 'a', anchor: 0, height: 84 },
			{ id: 'b', anchor: 10, height: 84 }
		],
		{ gap: 8, active: 'b' }
	);
	assert.equal(crowded.get('a'), 0);
	assert.equal(crowded.get('b'), 92);

	/* An active card not in the margin changes nothing. */
	assert.deepEqual(
		[...stackMarginNotes(notes, { gap: 8, active: 'gone' })],
		[...stackMarginNotes(notes, { gap: 8 })]
	);
	assert.equal(stackMarginNotes([], {}).size, 0);
});
