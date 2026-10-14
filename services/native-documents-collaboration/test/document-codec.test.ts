import assert from 'node:assert/strict';
import test from 'node:test';

import * as Y from 'yjs';

import {
  DocumentCodecError,
  PROSEMIRROR_FRAGMENT_NAME,
  projectionFromState,
  stateFromProjection,
} from '../src/document-codec.js';

const projection = {
  type: 'doc',
  content: [
    {
      type: 'heading',
      attrs: { block_id: 'heading', level: 2, align: 'center' },
      content: [{ type: 'text', text: 'A shared plan', marks: [{ type: 'bold' }] }],
    },
    {
      type: 'paragraph',
      attrs: { block_id: 'opening' },
      content: [
        { type: 'mention', attrs: { actor_id: 'research-lead', label: 'Research Lead' } },
        { type: 'text', text: ' linked ', marks: [{ type: 'italic' }] },
        { type: 'reference', attrs: { kind: 'work', id: 'work-1', label: 'the experiment' } },
        { type: 'text', text: 'due Friday', marks: [{ type: 'underline' }, { type: 'highlight', attrs: { color: 'yellow' } }, { type: 'textColor', attrs: { color: 'red' } }] },
        { type: 'hardBreak' },
        { type: 'text', text: 'Evidence', marks: [{ type: 'link', attrs: { href: '/evidence', title: 'Evidence' } }] },
      ],
    },
    {
      type: 'bulletList', attrs: { block_id: 'bullets' }, content: [
        { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Observe' }] }] },
      ],
    },
    {
      type: 'orderedList', attrs: { block_id: 'steps', start: 2 }, content: [
        { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Decide' }] }] },
      ],
    },
    {
      type: 'taskList', attrs: { block_id: 'tasks' }, content: [
        { type: 'taskItem', attrs: { checked: false }, content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Ship' }] }] },
      ],
    },
    {
      type: 'blockquote', attrs: { block_id: 'quote' }, content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Proof before confidence.', marks: [{ type: 'strike' }] }] },
      ],
    },
    { type: 'codeBlock', attrs: { block_id: 'code', language: 'typescript' }, content: [{ type: 'text', text: 'const proof = true' }] },
    { type: 'horizontalRule', attrs: { block_id: 'rule' } },
    { type: 'image', attrs: { block_id: 'chart', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', alt: 'Q3 chart', width: 320, caption: 'Revenue by month' } },
    {
      type: 'table', attrs: { block_id: 'table' }, content: [{
        type: 'tableRow', content: [
          { type: 'tableHeader', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Claim', marks: [{ type: 'code' }] }] }] },
          { type: 'tableCell', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Observed' }] }] },
        ],
      }],
    },
  ],
};

test('the document codec round-trips the released rich-text schema through the Tiptap default fragment', () => {
  const state = stateFromProjection(projection);
  assert.ok(state.byteLength >= 2);
  const restored = projectionFromState(state);
  assert.equal(restored.type, 'doc');
  assert.equal((restored.content as unknown[]).length, projection.content.length);

  const document = new Y.Doc();
  Y.applyUpdate(document, state);
  assert.deepEqual([...document.share.keys()], [PROSEMIRROR_FRAGMENT_NAME]);
  assert.ok(document.getXmlFragment(PROSEMIRROR_FRAGMENT_NAME).length > 0);
  document.destroy();
});

test('an image keeps only the company file it names', () => {
  const restored = projectionFromState(stateFromProjection(projection));
  assert.deepEqual((restored.content as Array<{ type: string }>).find((node) => node.type === 'image'), {
    type: 'image',
    attrs: { block_id: 'chart', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', alt: 'Q3 chart', width: 320, caption: 'Revenue by month' },
  });
  for (const attrs of [
    { block_id: 'x', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', src: 'https://elsewhere.test/a.png' },
    { block_id: 'x', attachment_id: '../../secrets' },
    { block_id: 'x', attachment_id: '0B7D9C1E-3F4A-4B5C-8D6E-7F8091A2B3C4' },
    { block_id: 'x', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', width: 9_000 },
    { block_id: 'x', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', caption: ' ' },
    { block_id: 'x', attachment_id: '0b7d9c1e-3f4a-4b5c-8d6e-7f8091a2b3c4', caption: 'x'.repeat(301) },
  ]) {
    assert.throws(
      () => stateFromProjection({ type: 'doc', content: [{ type: 'image', attrs }] }),
      DocumentCodecError,
      JSON.stringify(attrs),
    );
  }
});

test('the document codec rejects malformed projections and hidden shared Yjs types', () => {
  assert.throws(
    () => stateFromProjection({
      type: 'doc',
      content: [
        { type: 'paragraph', attrs: { block_id: 'repeated' } },
        { type: 'paragraph', attrs: { block_id: 'repeated' } },
      ],
    }),
    DocumentCodecError,
  );
  assert.throws(
    () => stateFromProjection({
      type: 'doc',
      content: [{
        type: 'paragraph',
        attrs: { block_id: 'unsafe-link' },
        content: [{ type: 'text', text: 'unsafe', marks: [{ type: 'link', attrs: { href: 'javascript:alert(1)' } }] }],
      }],
    }),
    DocumentCodecError,
  );

  const hidden = new Y.Doc();
  hidden.getMap('hidden').set('content', 'not part of the editor projection');
  const hiddenState = Y.encodeStateAsUpdate(hidden);
  hidden.destroy();
  assert.throws(() => projectionFromState(hiddenState), DocumentCodecError);
});

test('colours and alignment are names from the company palette', () => {
  const restored = projectionFromState(stateFromProjection(projection));
  assert.deepEqual((restored.content as Array<{ attrs: unknown }>)[0]?.attrs, { block_id: 'heading', level: 2, align: 'center' });
  const text = (marks: unknown[], attrs: Record<string, unknown> = { block_id: 'p' }) => ({
    type: 'doc',
    content: [{ type: 'paragraph', attrs, content: [{ type: 'text', text: 'x', marks }] }],
  });
  for (const body of [
    text([{ type: 'highlight', attrs: { color: '#ffff00' } }]),
    text([{ type: 'textColor', attrs: { color: 'red', shade: 2 } }]),
    text([{ type: 'underline', attrs: { style: 'double' } }]),
    text([], { block_id: 'p', align: 'left' }),
  ]) {
    assert.throws(() => stateFromProjection(body), DocumentCodecError, JSON.stringify(body));
  }
});
