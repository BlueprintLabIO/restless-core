import assert from 'node:assert/strict';
import test from 'node:test';
import * as Y from 'yjs';
import { bodyView, blockHash, BodyEditConflict, checkedBodyUpdate, prepareBodyEdit } from '../src/body-edits.js';
import { PROSEMIRROR_FRAGMENT_NAME, projectionFromState, stateFromProjection } from '../src/document-codec.js';

const paragraph = (id: string, text: string) => ({ type: 'paragraph', attrs: { block_id: id }, content: [{ type: 'text', text }] });
const seed = () => stateFromProjection({ type: 'doc', content: [paragraph('a', 'Opening'), paragraph('b', 'Human notes')] });
function live(state = seed()): Y.Doc { const doc = new Y.Doc(); Y.applyUpdate(doc, state); return doc; }
function blocks(doc: Y.Doc): Record<string, unknown>[] { return projectionFromState(Y.encodeStateAsUpdate(doc)).content as Record<string, unknown>[]; }
function text(doc: Y.Doc, index: number): Y.XmlText {
  return (doc.getXmlFragment(PROSEMIRROR_FRAGMENT_NAME).get(index) as Y.XmlElement).get(0) as Y.XmlText;
}

test('guarded rich block edits preserve another human block and survive replay after further typing', () => {
  const document = live();
  try {
    const initial = Y.encodeStateAsUpdate(document);
    const observed = blocks(document);
    const prepared = prepareBodyEdit(initial, { operations: [{ op: 'replace', block_id: 'a', expected_hash: blockHash(observed[0]), block: paragraph('a', 'Refined opening') }] });
    assert.deepEqual(Y.encodeStateAsUpdate(document), initial, 'preparation mutated the shared document');
    text(document, 1).insert('Human notes'.length, ' while the agent works');
    Y.applyUpdate(document, checkedBodyUpdate(document, prepared.update_base64));
    assert.deepEqual(blocks(document), [paragraph('a', 'Refined opening'), paragraph('b', 'Human notes while the agent works')]);
    text(document, 0).insert('Refined opening'.length, ' with a human addition');
    const afterHuman = Y.encodeStateAsUpdate(document);
    Y.applyUpdate(document, checkedBodyUpdate(document, prepared.update_base64));
    assert.deepEqual(Y.encodeStateAsUpdate(document), afterHuman, 'retry overwrote a subsequent human edit');
  } finally { document.destroy(); }
});

test('same-block concurrent text uses CRDT character edits, not block replacement', () => {
  const document = live();
  try {
    const prepared = prepareBodyEdit(Y.encodeStateAsUpdate(document), { operations: [{ op: 'replace', block_id: 'a', expected_hash: blockHash(blocks(document)[0]), block: paragraph('a', 'Better opening') }] });
    text(document, 0).insert('Opening'.length, ' — human addition');
    Y.applyUpdate(document, checkedBodyUpdate(document, prepared.update_base64));
    const output = (blocks(document)[0]?.content as { text: string }[]).map((node) => node.text).join('');
    assert.match(output, /human addition/);
    assert.match(output, /Better opening/);
  } finally { document.destroy(); }
});

test('stale guards fail without mutation and an insert/delete batch is idempotent', () => {
  const document = live();
  try {
    const oldHash = blockHash(blocks(document)[0]);
    text(document, 0).insert(0, 'Human ');
    const before = Y.encodeStateAsUpdate(document);
    assert.throws(() => prepareBodyEdit(before, { operations: [{ op: 'delete', block_id: 'a', expected_hash: oldHash }] }), BodyEditConflict);
    assert.deepEqual(Y.encodeStateAsUpdate(document), before);
    const prepared = prepareBodyEdit(before, { operations: [
      { op: 'insert', after: 'a', block: { type: 'heading', attrs: { block_id: 'c', level: 2 }, content: [{ type: 'text', text: 'Shared plan' }] } },
      { op: 'delete', block_id: 'b', expected_hash: blockHash(blocks(document)[1]) },
    ] });
    Y.applyUpdate(document, checkedBodyUpdate(document, prepared.update_base64));
    const after = Y.encodeStateAsUpdate(document);
    assert.equal(blocks(document).length, 2);
    Y.applyUpdate(document, checkedBodyUpdate(document, prepared.update_base64));
    assert.deepEqual(Y.encodeStateAsUpdate(document), after);
    assert.throws(() => prepareBodyEdit(after, { operations: [{ op: 'insert', after: null, block: paragraph('c', 'duplicate') }] }), BodyEditConflict);
  } finally { document.destroy(); }
});

test('the complete prospective document is validated before any live mutation', () => {
  const document = live(); const malicious = new Y.Doc();
  try {
    const before = Y.encodeStateAsUpdate(document);
    assert.throws(() => prepareBodyEdit(before, { operations: [{ op: 'insert', after: 'a', block: { type: 'script', attrs: { block_id: 'unsafe' } } }] }));
    malicious.getMap('authority').set('role', 'owner');
    assert.throws(() => checkedBodyUpdate(document, Buffer.from(Y.encodeStateAsUpdate(malicious)).toString('base64')));
    assert.throws(() => checkedBodyUpdate(document, 'not-base64'));
    assert.deepEqual(Y.encodeStateAsUpdate(document), before);
    const view = bodyView(before);
    assert.equal((view.blocks as unknown[]).length, 2);
  } finally { document.destroy(); malicious.destroy(); }
});
