import assert from 'node:assert/strict';
import test from 'node:test';
import { getSchema } from '@tiptap/core';
import { EditorState, TextSelection } from '@tiptap/pm/state';
import * as Y from 'yjs';
import * as documentEditorSchema from './document-editor-schema.ts';

test('the browser editor schema is the exact sidecar-compatible node and mark set', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	assert.equal(documentEditorSchema.DOCUMENT_COLLABORATION_FRAGMENT, 'default');
	assert.deepEqual(Object.keys(schema.nodes).sort(), [
		'blockquote',
		'bulletList',
		'codeBlock',
		'doc',
		'hardBreak',
		'heading',
		'horizontalRule',
		'image',
		'listItem',
		'mention',
		'orderedList',
		'paragraph',
		'reference',
		'table',
		'tableCell',
		'tableHeader',
		'tableRow',
		'taskItem',
		'taskList',
		'text'
	]);
	assert.deepEqual(Object.keys(schema.marks).sort(), [
		'bold',
		'code',
		'highlight',
		'italic',
		'link',
		'strike',
		'textColor',
		'underline'
	]);
});

/* The sidecar refuses a whole body for one attribute outside these lists, and every later store
 * with it (services/native-documents-collaboration/src/document-codec.ts validateAttributes). */
const SIDECAR_ATTRIBUTES: Record<string, string[]> = {
	heading: ['align', 'block_id', 'level'],
	paragraph: ['align', 'block_id'],
	orderedList: ['block_id', 'start'],
	taskItem: ['block_id', 'checked'],
	codeBlock: ['block_id', 'language'],
	mention: ['actor_id', 'label'],
	reference: ['id', 'kind', 'label'],
	image: ['alt', 'attachment_id', 'block_id', 'width'],
	doc: [],
	text: [],
	hardBreak: []
};

test('no browser node or mark can carry an attribute the sidecar refuses', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	for (const [name, type] of Object.entries(schema.nodes)) {
		assert.deepEqual(
			Object.keys(type.spec.attrs ?? {}).sort(),
			SIDECAR_ATTRIBUTES[name] ?? ['block_id'],
			`${name} attributes`
		);
	}
	for (const [name, type] of Object.entries(schema.marks)) {
		assert.deepEqual(
			Object.keys(type.spec.attrs ?? {}).sort(),
			name === 'link'
				? ['href', 'title']
				: ['highlight', 'textColor'].includes(name)
					? ['color']
					: [],
			`${name} attributes`
		);
	}
});

function element(attributes: Record<string, string>, text = '') {
	return {
		getAttribute: (name: string) => attributes[name] ?? null,
		textContent: text
	} as unknown as HTMLElement;
}

test('pasted links, mentions and references the sidecar would refuse are not parsed as such', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true, undefined, {
			companyId: 'acme'
		})
	);
	const linkRule = schema.marks.link.spec.parseDOM?.[0];
	const parseLink = (attributes: Record<string, string>) =>
		linkRule?.getAttrs?.(element(attributes) as never);
	for (const href of ['javascript:alert(1)', 'tel:+15551234', '#', 'notes/plan', 'file:///etc']) {
		assert.equal(parseLink({ href }), false, href);
	}
	assert.equal(parseLink({ href: `https://example.com/${'a'.repeat(2_100)}` }), false);
	const kept = parseLink({ href: ' https://example.com/a ', title: '   ' });
	assert.equal(kept && kept.href, 'https://example.com/a');
	assert.equal(kept && kept.title, undefined, 'an empty title is dropped, not stored');

	const mentionRule = schema.nodes.mention.spec.parseDOM?.[0];
	assert.equal(mentionRule?.getAttrs?.(element({}, '@Exec') as never), false);
	assert.deepEqual(
		mentionRule?.getAttrs?.(element({ 'data-actor-id': 'exec' }, '@Exec') as never),
		{
			actor_id: 'exec',
			label: 'Exec'
		}
	);
	const referenceRule = schema.nodes.reference.spec.parseDOM?.[0];
	assert.equal(
		referenceRule?.getAttrs?.(
			element({ 'data-reference-kind': 'website', 'data-reference-id': 'x' }, 'X') as never
		),
		false
	);
	assert.deepEqual(
		referenceRule?.getAttrs?.(
			element({ 'data-reference-kind': 'artifact', 'data-reference-id': 'a-1' }, 'Deck') as never
		),
		{ kind: 'artifact', id: 'a-1', label: 'Deck' }
	);
});

test('typed links become only links the sidecar keeps', () => {
	assert.equal(documentEditorSchema.linkFromInput('example.com/a'), 'https://example.com/a');
	assert.equal(documentEditorSchema.linkFromInput('HTTPS://Example.com'), 'https://Example.com');
	assert.equal(documentEditorSchema.linkFromInput('pat@example.com'), 'mailto:pat@example.com');
	assert.equal(documentEditorSchema.linkFromInput('/acme/work/1'), '/acme/work/1');
	assert.equal(documentEditorSchema.linkFromInput('javascript:alert(1)'), null);
	assert.equal(documentEditorSchema.linkFromInput('two words'), null);
	assert.equal(
		documentEditorSchema.documentReferenceHref('acme', 'artifact', 'a/1'),
		'/acme/library?file=a%2F1'
	);
	assert.equal(documentEditorSchema.documentReferenceHref('acme', 'decision', 'd-1'), null);
});

test('named-version projection preserves stable blocks and strips generic Tiptap attributes', () => {
	assert.deepEqual(
		documentEditorSchema.projectDocumentContent({
			type: 'doc',
			content: [
				{
					type: 'paragraph',
					attrs: { block_id: 'opening', textAlign: null },
					content: [
						{
							type: 'text',
							text: 'Evidence',
							marks: [
								{
									type: 'link',
									attrs: {
										href: 'https://restless.run',
										title: null,
										target: '_blank',
										rel: 'noopener noreferrer'
									}
								}
							]
						}
					]
				}
			]
		}),
		{
			type: 'doc',
			content: [
				{
					type: 'paragraph',
					attrs: { block_id: 'opening' },
					content: [
						{
							type: 'text',
							text: 'Evidence',
							marks: [{ type: 'link', attrs: { href: 'https://restless.run' } }]
						}
					]
				}
			]
		}
	);
});

test('named-version projection fails closed without stable top-level identity', () => {
	assert.throws(
		() =>
			documentEditorSchema.projectDocumentContent({
				type: 'doc',
				content: [{ type: 'paragraph', content: [{ type: 'text', text: 'No identity' }] }]
			}),
		/stable identity/u
	);
});

test('named-version projection fails closed on duplicate block identities', () => {
	assert.throws(
		() =>
			documentEditorSchema.projectDocumentContent({
				type: 'doc',
				content: [
					{ type: 'paragraph', attrs: { block_id: 'same' } },
					{ type: 'heading', attrs: { block_id: 'same', level: 2 } }
				]
			}),
		/duplicated/u
	);
});

test('named-version projection retains mentions, references, tasks, and table shape', () => {
	const content = documentEditorSchema.projectDocumentContent({
		type: 'doc',
		content: [
			{
				type: 'taskList',
				attrs: { block_id: 'tasks' },
				content: [
					{
						type: 'taskItem',
						attrs: { checked: true, block_id: 'task-1' },
						content: [
							{
								type: 'paragraph',
								attrs: { block_id: 'task-copy' },
								content: [
									{ type: 'mention', attrs: { actor_id: 'lead', label: 'Lead' } },
									{ type: 'text', text: ' owns ' },
									{
										type: 'reference',
										attrs: { kind: 'work', id: 'work-1', label: 'the launch' }
									}
								]
							}
						]
					}
				]
			}
		]
	});
	assert.equal(content.content[0]?.attrs?.block_id, 'tasks');
	assert.equal(content.content[0]?.content?.[0]?.attrs?.checked, true);
	assert.equal(content.content[0]?.content?.[0]?.content?.[0]?.content?.[0]?.type, 'mention');
});

const CHART = '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4';

test('an image keeps only the company file it names, and a page shows it from that company', () => {
	assert.deepEqual(
		documentEditorSchema.projectDocumentContent({
			type: 'doc',
			content: [
				{
					type: 'image',
					attrs: { block_id: 'chart', attachment_id: CHART, alt: 'Q3', width: null, src: 'x' }
				}
			]
		}).content[0],
		{ type: 'image', attrs: { block_id: 'chart', attachment_id: CHART, alt: 'Q3' } }
	);
	for (const attrs of [
		{ block_id: 'x', attachment_id: '../secrets' },
		{ block_id: 'x', attachment_id: CHART.toUpperCase() },
		{ block_id: 'x', attachment_id: CHART, width: 9_000 }
	]) {
		assert.throws(
			() =>
				documentEditorSchema.projectDocumentContent({
					type: 'doc',
					content: [{ type: 'image', attrs }]
				}),
			/image/u,
			JSON.stringify(attrs)
		);
	}
	assert.equal(
		documentEditorSchema.withDocumentImages(
			`<figure data-block-id="c"><img data-attachment-id="${CHART}" alt="Q3"></figure><img data-attachment-id="nope" alt="">`,
			'acme co'
		),
		`<figure data-block-id="c"><img src="/api/companies/acme%20co/attachments/${CHART}?inline=1" loading="lazy" data-attachment-id="${CHART}" alt="Q3"></figure><img data-attachment-id="nope" alt="">`
	);
	assert.equal(documentEditorSchema.imageAltFromName('Q3 chart.png'), 'Q3 chart');
});

test('the "/" menu follows its slash and closes when the slash goes or the words stop naming a block', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	const at = (text: string, caret: number) => {
		const doc = schema.node('doc', null, [
			schema.node('paragraph', { block_id: 'a' }, text ? [schema.text(text)] : []),
			schema.node('paragraph', { block_id: 'b' }, [schema.text('Elsewhere')])
		]);
		return EditorState.create({ doc, selection: TextSelection.create(doc, caret) });
	};
	const open = { from: 1, query: '' };
	assert.deepEqual(documentEditorSchema.nextSlashMenuState(at('/head', 6), open, 1), {
		from: 1,
		query: 'head'
	});
	assert.equal(documentEditorSchema.nextSlashMenuState(at('', 1), open, 1), null, 'slash deleted');
	assert.equal(
		documentEditorSchema.nextSlashMenuState(at('/ h', 4), open, 1),
		null,
		'a space first'
	);
	assert.equal(
		documentEditorSchema.nextSlashMenuState(at('/head', 10), open, 1),
		null,
		'caret left'
	);
	assert.equal(
		documentEditorSchema.nextSlashMenuState(at('/head', 6), null, 1),
		null,
		'never opened'
	);
});

test('"/" opens the menu at the start of a line or after a space, never mid-word or in code', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	const at = (text: string, caret: number, type = 'paragraph') => {
		const block = schema.node(type, { block_id: 'a' }, text ? [schema.text(text)] : []);
		const doc = schema.node('doc', null, [block]);
		return EditorState.create({ doc, selection: TextSelection.create(doc, caret) });
	};
	assert.ok(documentEditorSchema.slashOpensMenu(at('', 1)), 'an empty line');
	assert.ok(documentEditorSchema.slashOpensMenu(at('Plan ', 6)), 'after a space');
	assert.ok(documentEditorSchema.slashOpensMenu(at('Plan', 1)), 'the start of a line of text');
	assert.ok(documentEditorSchema.slashOpensMenu(at('Plan', 1, 'heading')), 'a heading');
	assert.ok(!documentEditorSchema.slashOpensMenu(at('and', 4)), 'mid-word: and/or');
	assert.ok(!documentEditorSchema.slashOpensMenu(at('', 1, 'codeBlock')), 'code');
});

test('a paragraph stays the block the editor fills an empty place with', (context) => {
	/* An image declared ahead of the paragraph once became the default block: deleting text or
	 * turning a block left an empty image behind. */
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	assert.equal(schema.topNodeType.contentMatch.defaultType?.name, 'paragraph');
	assert.equal(schema.nodes.blockquote.contentMatch.defaultType?.name, 'paragraph');
});

test('a split block keeps its id on the part that is still that block', (context) => {
	const document = new Y.Doc();
	context.after(() => document.destroy());
	const schema = getSchema(
		documentEditorSchema.createDocumentEditorExtensions(document, {} as never, true)
	);
	const paragraph = (id: string, text: string) =>
		schema.nodes.paragraph.create({ block_id: id }, text ? schema.text(text) : undefined);
	const ids = (state: EditorState) => {
		const found: [string, string][] = [];
		state.doc.forEach((node) => found.push([node.attrs.block_id, node.textContent]));
		return found;
	};
	const split = (blocks: ReturnType<typeof paragraph>[], at: (state: EditorState) => number) => {
		let state = EditorState.create({
			schema,
			doc: schema.nodes.doc.create(null, blocks),
			plugins: [documentEditorSchema.blockIdentityPlugin()]
		});
		state = state.applyTransaction(state.tr.split(at(state))).state;
		return ids(state);
	};
	// Enter at the end of a line: the line keeps its id; the new empty line is new.
	const end = split([paragraph('a', 'Shared plan.')], () => 13);
	assert.deepEqual(end[0], ['a', 'Shared plan.']);
	assert.notEqual(end[1][0], 'a');
	// Enter on an empty line: that line keeps its id; the new one below is new.
	const empty = split([paragraph('a', 'Shared plan.'), paragraph('b', '')], () => 15);
	assert.deepEqual(empty.slice(0, 2), [
		['a', 'Shared plan.'],
		['b', '']
	]);
	assert.ok(!['a', 'b'].includes(empty[2][0]));
	// Enter at the start of a line: the text keeps its id; the empty line above is new.
	const start = split([paragraph('a', 'Shared plan.')], () => 1);
	assert.deepEqual(start[1], ['a', 'Shared plan.']);
	assert.notEqual(start[0][0], 'a');
});

test('colours and alignment keep only the named palette the sidecar keeps', () => {
	const { documentTextColorFromCss, documentHighlightFromCss, projectDocumentContent } =
		documentEditorSchema;
	assert.equal(documentTextColorFromCss('#000000'), null, 'a page’s ordinary black is no colour');
	assert.equal(documentTextColorFromCss('rgb(255, 255, 255)'), null);
	assert.equal(documentTextColorFromCss('#e01b1b'), 'red');
	assert.equal(documentTextColorFromCss('#1155cc'), 'blue');
	assert.equal(documentHighlightFromCss('#ffff00'), 'yellow');
	assert.equal(documentHighlightFromCss('transparent'), null);
	assert.equal(documentHighlightFromCss('#ffffff'), null);
	const body = (marks: unknown[], attrs: Record<string, unknown> = {}) => ({
		type: 'doc',
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: 'p', ...attrs },
				content: [{ type: 'text', text: 'Due Friday', marks }]
			}
		]
	});
	const kept = projectDocumentContent(
		body(
			[
				{ type: 'underline' },
				{ type: 'highlight', attrs: { color: 'yellow' } },
				{ type: 'textColor', attrs: { color: 'red' } }
			],
			{ align: 'center' }
		) as never
	);
	assert.deepEqual(kept.content[0].attrs, { block_id: 'p', align: 'center' });
	assert.throws(() =>
		projectDocumentContent(body([{ type: 'highlight', attrs: { color: '#ff0' } }]) as never)
	);
	assert.throws(() => projectDocumentContent(body([], { align: 'left' }) as never));
});
