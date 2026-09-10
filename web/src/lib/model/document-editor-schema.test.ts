import assert from 'node:assert/strict';
import test from 'node:test';
import { getSchema } from '@tiptap/core';
import * as Y from 'yjs';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
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
