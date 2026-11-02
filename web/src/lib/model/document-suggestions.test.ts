import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { getSchema } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { EditorState, TextSelection, type Transaction } from '@tiptap/pm/state';
import * as Y from 'yjs';
import {
	createDocumentEditorExtensions,
	projectDocumentContent
} from './document-editor-schema.ts';
import {
	documentSuggestions,
	resolveSuggestions,
	suggestionAtSelection,
	suggestionSentence,
	suggestionsKey,
	suggestionsPlugin,
	type SuggestingMode
} from './document-suggestions.ts';

const ALICE: SuggestingMode = { actorId: 'alice', thread: null };

function schema() {
	const document = new Y.Doc();
	const built = getSchema(createDocumentEditorExtensions(document, {} as never, true));
	document.destroy();
	return built;
}
const SCHEMA = schema();

function start(paragraphs: string[], mode: SuggestingMode | null = ALICE): EditorState {
	const doc = SCHEMA.nodeFromJSON({
		type: 'doc',
		content: paragraphs.map((text, index) => ({
			type: 'paragraph',
			attrs: { block_id: `p${index}` },
			content: text ? [{ type: 'text', text }] : []
		}))
	});
	const state = EditorState.create({ schema: SCHEMA, doc, plugins: [suggestionsPlugin()] });
	return state.apply(state.tr.setMeta(suggestionsKey, { mode }));
}

/** A position inside paragraph `index`, `offset` characters in. */
function at(state: EditorState, index: number, offset: number): number {
	let found = -1;
	state.doc.forEach((node, pos, i) => {
		if (i === index) found = pos + 1 + offset;
	});
	return found;
}
function apply(state: EditorState, change: (tr: Transaction) => Transaction): EditorState {
	return state.applyTransaction(change(state.tr)).state;
}
function caretAt(state: EditorState, pos: number): EditorState {
	return apply(state, (tr) => tr.setSelection(TextSelection.create(tr.doc, pos)));
}
function type(state: EditorState, text: string): EditorState {
	for (const character of text) {
		state = apply(state, (tr) => tr.insertText(character));
	}
	return state;
}
function backspace(state: EditorState, times = 1): EditorState {
	for (let index = 0; index < times; index += 1) {
		const { from } = state.selection;
		state = apply(state, (tr) => tr.delete(from - 1, from));
	}
	return state;
}
/** The paragraph's runs as [text, marks] with suggestion marks named by kind and author. */
function runs(node: ProseMirrorNode): [string, string][] {
	const out: [string, string][] = [];
	node.forEach((child) => {
		const marks = child.marks
			.map((mark) =>
				mark.type.name === 'suggestedInsert'
					? `+${mark.attrs.author}`
					: mark.type.name === 'suggestedDelete'
						? `-${mark.attrs.author}`
						: mark.type.name
			)
			.join(' ');
		out.push([child.text ?? '', marks]);
	});
	return out;
}
function settle(state: EditorState, accept: boolean): EditorState {
	return apply(state, (tr) => {
		resolveSuggestions(tr, () => true, accept);
		return tr.setMeta(suggestionsKey, { settled: true });
	});
}
const texts = (state: EditorState) => {
	const out: string[] = [];
	state.doc.forEach((node) => out.push(node.textContent));
	return out;
};

test('the editor and the documents service share one suggestions core', () => {
	const below = (path: string) =>
		readFileSync(new URL(path, import.meta.url), 'utf8')
			.split('\n')
			.filter(
				(line) =>
					!line.startsWith('import ') &&
					!line.startsWith(' * SHARED CORE') &&
					!/^ \* (imports|service's imports)\./u.test(line)
			)
			.join('\n');
	assert.equal(
		below('./document-suggestions-core.ts'),
		below('../../../../services/native-documents-collaboration/src/suggestions.ts')
	);
});

test('typing while suggesting is one insertion in the writer’s name, not the document’s words', () => {
	let state = start(['Ship on Friday.']);
	state = caretAt(state, at(state, 0, 'Ship on Friday'.length));
	state = type(state, ' at noon');
	assert.deepEqual(runs(state.doc.child(0)), [
		['Ship on Friday', ''],
		[' at noon', '+alice'],
		['.', '']
	]);
	const [only] = documentSuggestions(state);
	assert.equal(only?.kind, 'insert');
	assert.equal(only?.author, 'alice');
	assert.equal(suggestionSentence(only!), 'Add “at noon”');
	assert.equal(suggestionAtSelection(state), only?.id);
	assert.deepEqual(texts(settle(state, false)), ['Ship on Friday.']);
	assert.deepEqual(texts(settle(state, true)), ['Ship on Friday at noon.']);
});

test('Backspace strikes words and walks left; typing then makes one replacement', () => {
	let state = start(['Ship on Friday.']);
	state = caretAt(state, at(state, 0, 'Ship on Friday'.length));
	state = backspace(state, 6);
	assert.deepEqual(runs(state.doc.child(0)), [
		['Ship on ', ''],
		['Friday', '-alice'],
		['.', '']
	]);
	assert.equal(
		state.selection.from,
		at(state, 0, 'Ship on '.length),
		'the caret waits before the struck word'
	);
	assert.equal(documentSuggestions(state).length, 1, 'six Backspaces are one deletion');
	state = type(state, 'Monday');
	const [replacement] = documentSuggestions(state);
	assert.equal(documentSuggestions(state).length, 1);
	assert.equal(replacement?.kind, 'replace');
	assert.equal(suggestionSentence(replacement!), 'Replace “Friday” with “Monday”');
	assert.deepEqual(texts(settle(state, true)), ['Ship on Monday.']);
	assert.deepEqual(texts(settle(state, false)), ['Ship on Friday.']);
});

test('typing over a selection strikes it and adds the new words after it', () => {
	let state = start(['Ship on Friday.']);
	const from = at(state, 0, 'Ship on '.length);
	state = apply(state, (tr) =>
		tr.setSelection(TextSelection.create(tr.doc, from, from + 'Friday'.length))
	);
	state = type(state, 'Monday');
	assert.deepEqual(runs(state.doc.child(0)), [
		['Ship on ', ''],
		['Friday', '-alice'],
		['Monday', '+alice'],
		['.', '']
	]);
	assert.equal(state.selection.from, at(state, 0, 'Ship on FridayMonday'.length));
	assert.equal(documentSuggestions(state).length, 1);
});

test('deleting your own suggested words takes them out; another person’s are struck', () => {
	let state = start(['Plan.']);
	state = caretAt(state, at(state, 0, 'Plan'.length));
	state = type(state, ' A');
	state = backspace(state, 2);
	assert.deepEqual(texts(state), ['Plan.'], 'own words simply go');
	assert.equal(documentSuggestions(state).length, 0);

	state = type(state, ' B');
	const bob = apply(state, (tr) =>
		tr.setMeta(suggestionsKey, { mode: { actorId: 'bob', thread: null } })
	);
	const struck = backspace(bob, 1);
	assert.deepEqual(runs(struck.doc.child(0)), [
		['Plan', ''],
		[' ', '+alice'],
		['B', '+alice -bob'],
		['.', '']
	]);
});

test('a deletion across paragraphs keeps both, struck; accepting it joins them', () => {
	let state = start(['First paragraph', 'Second paragraph']);
	const from = at(state, 0, 'First '.length);
	const to = at(state, 1, 'Second'.length);
	state = apply(state, (tr) => tr.setSelection(TextSelection.create(tr.doc, from, to)));
	state = apply(state, (tr) => tr.deleteSelection());
	assert.equal(state.doc.childCount, 2, 'the paragraphs stay apart until accepted');
	assert.deepEqual(texts(state), ['First paragraph', 'Second paragraph']);
	assert.deepEqual(
		state.doc.child(1).attrs.block_id,
		'p1',
		'the second paragraph keeps its identity'
	);
	assert.equal(documentSuggestions(state).length, 1);
	assert.deepEqual(texts(settle(state, true)), ['First  paragraph']);
	assert.deepEqual(texts(settle(state, false)), ['First paragraph', 'Second paragraph']);
});

test('a new paragraph of suggested words goes as a whole when rejected', () => {
	let state = start(['Only line']);
	state = caretAt(state, at(state, 0, 'Only line'.length));
	state = apply(state, (tr) => tr.split(tr.selection.from));
	state = type(state, 'Proposed line');
	assert.deepEqual(texts(state), ['Only line', 'Proposed line']);
	assert.deepEqual(texts(settle(state, false)), ['Only line']);
	assert.deepEqual(texts(settle(state, true)), ['Only line', 'Proposed line']);
});

test('suggestions can be proposed in code as well', () => {
	let state = EditorState.create({
		schema: SCHEMA,
		doc: SCHEMA.nodeFromJSON({
			type: 'doc',
			content: [
				{
					type: 'codeBlock',
					attrs: { block_id: 'c' },
					content: [{ type: 'text', text: 'let a;' }]
				},
				{
					type: 'paragraph',
					attrs: { block_id: 'p' },
					content: [{ type: 'text', text: 'run npm', marks: [{ type: 'code' }] }]
				}
			]
		}),
		plugins: [suggestionsPlugin()]
	});
	state = state.apply(state.tr.setMeta(suggestionsKey, { mode: ALICE }));
	state = caretAt(state, 1 + 'let a'.length);
	state = type(state, ' = 1');
	state = caretAt(state, state.doc.child(0).nodeSize + 1 + 'run npm'.length);
	state = type(state, ' ci');
	assert.deepEqual(runs(state.doc.child(0)), [
		['let a', ''],
		[' = 1', '+alice'],
		[';', '']
	]);
	assert.deepEqual(runs(state.doc.child(1)), [
		['run npm', 'code'],
		[' ci', 'code +alice']
	]);
});

test('editing directly inside someone’s suggestion types plain words', () => {
	let state = start(['Ship on Friday.']);
	state = caretAt(state, at(state, 0, 'Ship on Friday'.length));
	state = backspace(state, 6);
	state = apply(state, (tr) => tr.setMeta(suggestionsKey, { mode: null }));
	state = caretAt(state, at(state, 0, 'Ship on Fri'.length));
	state = type(state, 'X');
	assert.deepEqual(runs(state.doc.child(0)), [
		['Ship on ', ''],
		['Fri', '-alice'],
		['X', ''],
		['day', '-alice'],
		['.', '']
	]);
	assert.deepEqual(texts(settle(state, true)), ['Ship on X.']);
});

test('a named version keeps a suggestion’s author, time and thread', () => {
	const projected = projectDocumentContent({
		type: 'doc',
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: 'p', align: null },
				content: [
					{
						type: 'text',
						text: 'new',
						marks: [
							{
								type: 'suggestedInsert',
								attrs: {
									id: 's1',
									author: 'alice',
									at: '2026-11-03T09:00:00.000Z',
									thread: null
								}
							}
						]
					}
				]
			}
		]
	});
	assert.deepEqual(projected.content[0]?.content?.[0]?.marks, [
		{
			type: 'suggestedInsert',
			attrs: { id: 's1', author: 'alice', at: '2026-11-03T09:00:00.000Z' }
		}
	]);
	assert.throws(() =>
		projectDocumentContent({
			type: 'doc',
			content: [
				{
					type: 'paragraph',
					attrs: { block_id: 'p' },
					content: [
						{ type: 'text', text: 'x', marks: [{ type: 'suggestedDelete', attrs: { id: 's1' } }] }
					]
				}
			]
		})
	);
});

test('a Word copy carries suggestions as tracked changes, or the agreed text only', async () => {
	const { documentDocx } = await import('./document-docx.ts');
	const { unzipSync, strFromU8 } = await import('fflate');
	const suggestion = { id: 's1', author: 'alice', at: '2026-11-03T09:00:00.000Z', thread: null };
	const content = {
		type: 'doc' as const,
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: 'p' },
				content: [
					{ type: 'text', text: 'Ship on ' },
					{ type: 'text', text: 'Friday', marks: [{ type: 'suggestedDelete', attrs: suggestion }] },
					{
						type: 'text',
						text: 'Monday',
						marks: [{ type: 'bold' }, { type: 'suggestedInsert', attrs: suggestion }]
					}
				]
			}
		]
	};
	const xml = async (options: Parameters<typeof documentDocx>[2]) =>
		strFromU8(
			unzipSync(new Uint8Array(await documentDocx('Plan', content, options).arrayBuffer()))[
				'word/document.xml'
			]!
		);
	const tracked = await xml({ nameOf: (id) => (id === 'alice' ? 'Alice Ng' : id) });
	assert.match(
		tracked,
		/<w:del w:id="1" w:author="Alice Ng" w:date="2026-11-03T09:00:00Z"><w:r><w:delText xml:space="preserve">Friday<\/w:delText><\/w:r><\/w:del>/u
	);
	assert.match(
		tracked,
		/<w:ins w:id="2" w:author="Alice Ng" w:date="2026-11-03T09:00:00Z"><w:r><w:rPr><w:b\/><\/w:rPr><w:t xml:space="preserve">Monday<\/w:t><\/w:r><\/w:ins>/u
	);
	const agreed = await xml({ suggestions: 'agreed' });
	assert.match(agreed, /Ship on <\/w:t><\/w:r><w:r><w:t xml:space="preserve">Friday<\/w:t>/u);
	assert.doesNotMatch(agreed, /Monday|w:ins|w:del/u);
});
