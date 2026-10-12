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
	assert.deepEqual(Object.keys(schema.marks).sort(), ['bold', 'code', 'italic', 'link', 'strike']);
});

/* The sidecar refuses a whole body for one attribute outside these lists, and every later store
 * with it (services/native-documents-collaboration/src/document-codec.ts validateAttributes). */
const SIDECAR_ATTRIBUTES: Record<string, string[]> = {
	heading: ['block_id', 'level'],
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
			name === 'link' ? ['href', 'title'] : [],
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
			() => documentEditorSchema.projectDocumentContent({ type: 'doc', content: [{ type: 'image', attrs }] }),
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
	assert.equal(documentEditorSchema.nextSlashMenuState(at('/ h', 4), open, 1), null, 'a space first');
	assert.equal(documentEditorSchema.nextSlashMenuState(at('/head', 10), open, 1), null, 'caret left');
	assert.equal(documentEditorSchema.nextSlashMenuState(at('/head', 6), null, 1), null, 'never opened');
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
