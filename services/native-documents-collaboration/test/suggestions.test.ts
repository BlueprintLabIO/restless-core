import assert from 'node:assert/strict';
import test from 'node:test';
import { Transform } from 'prosemirror-transform';
import * as Y from 'yjs';
import { blockHash, bodyView, checkedBodyUpdate, InvalidBodyEdit, prepareBodyEdit } from '../src/body-edits.js';
import {
  DocumentCodecError,
  documentSchema,
  PROSEMIRROR_FRAGMENT_NAME,
  projectionFromState,
  stateFromProjection,
} from '../src/document-codec.js';
import { diffTokens } from '../src/suggest-edits.js';
import { resolveSuggestions, suggestionsIn } from '../src/suggestions.js';

type Json = Record<string, unknown>;
const AGENT = 'agent-exec';
const OWNER = 'owner';
const THREAD = '8a6f2c1e-1b2a-4c3d-9e8f-0a1b2c3d4e5f';
const NOW = new Date('2026-11-03T09:30:00.000Z');
const by = (actorId: string) => ({ actorId, now: NOW });
const text = (value: string, marks?: Json[]) => ({ type: 'text', text: value, ...(marks ? { marks } : {}) });
const paragraph = (id: string, ...content: Json[]) => ({ type: 'paragraph', attrs: { block_id: id }, content });
const suggestion = (id: string, author = AGENT) => ({ id, author, at: '2026-11-03T09:00:00.000Z', thread: null });

function live(content: Json[]): Y.Doc {
  const doc = new Y.Doc();
  Y.applyUpdate(doc, stateFromProjection({ type: 'doc', content }));
  return doc;
}
function blocks(doc: Y.Doc): Json[] {
  return projectionFromState(Y.encodeStateAsUpdate(doc)).content as Json[];
}
function words(block: Json | undefined): string {
  return ((block?.content ?? []) as Json[]).map((node) => String(node.text ?? '')).join('');
}
/** The text as it would read if every open suggestion were accepted. */
function accepted(doc: Y.Doc): string[] {
  const tr = new Transform(documentSchema.nodeFromJSON(projectionFromState(Y.encodeStateAsUpdate(doc))));
  resolveSuggestions(tr, () => true, true);
  return (tr.doc.toJSON().content as Json[]).map(words);
}
function rejected(doc: Y.Doc): string[] {
  const tr = new Transform(documentSchema.nodeFromJSON(projectionFromState(Y.encodeStateAsUpdate(doc))));
  resolveSuggestions(tr, () => true, false);
  return (tr.doc.toJSON().content as Json[]).map(words);
}
function apply(doc: Y.Doc, operations: Json[], actor = AGENT): void {
  const prepared = prepareBodyEdit(Y.encodeStateAsUpdate(doc), { operations }, by(actor));
  Y.applyUpdate(doc, checkedBodyUpdate(doc, prepared.update_base64));
}
function yText(doc: Y.Doc, index: number): Y.XmlText {
  return (doc.getXmlFragment(PROSEMIRROR_FRAGMENT_NAME).get(index) as Y.XmlElement).get(0) as Y.XmlText;
}

test('the codec keeps suggestion marks, in code too, and refuses malformed ones', () => {
  const body = {
    type: 'doc',
    content: [
      paragraph('a', text('Ship '), text('Friday', [{ type: 'suggestedDelete', attrs: suggestion('s1') }]),
        text('Monday', [{ type: 'bold' }, { type: 'suggestedInsert', attrs: { ...suggestion('s1'), thread: THREAD } }])),
      paragraph('b', text('npm ci', [{ type: 'code' }, { type: 'suggestedInsert', attrs: suggestion('s2') }])),
      { type: 'codeBlock', attrs: { block_id: 'c', language: null }, content: [text('let a = 1;', [{ type: 'suggestedDelete', attrs: suggestion('s3') }])] },
    ],
  };
  const round = projectionFromState(stateFromProjection(body));
  assert.deepEqual(suggestionsIn(documentSchema.nodeFromJSON(round)).map(({ id, kind, inserted, deleted, thread }) => ({ id, kind, inserted, deleted, thread })), [
    { id: 's1', kind: 'replace', inserted: 'Monday', deleted: 'Friday', thread: null },
    { id: 's2', kind: 'insert', inserted: 'npm ci', deleted: '', thread: null },
    { id: 's3', kind: 'delete', inserted: '', deleted: 'let a = 1;', thread: null },
  ]);
  for (const attrs of [
    { author: AGENT, at: '2026-11-03T09:00:00Z' },
    { id: 'bad id', author: AGENT, at: '2026-11-03T09:00:00Z' },
    { id: 's1', author: '', at: '2026-11-03T09:00:00Z' },
    { id: 's1', author: AGENT, at: 'Tuesday' },
    { id: 's1', author: AGENT, at: '2026-11-03T09:00Z' },
    { id: 's1', author: AGENT, at: '2026-11-03T09:00:00Z', thread: 'nope' },
    { id: 's1', author: AGENT, at: '2026-11-03T09:00:00Z', colour: 'red' },
  ]) {
    assert.throws(
      () => stateFromProjection({ type: 'doc', content: [paragraph('a', text('x', [{ type: 'suggestedInsert', attrs }]))] }),
      DocumentCodecError,
      JSON.stringify(attrs),
    );
  }
});

test('an agent suggests new words in its own name; the owner accepts or rejects them in place', () => {
  const doc = live([paragraph('a', text('We ship on Friday to all customers.')), paragraph('b', text('Notes.'))]);
  try {
    const observed = bodyView(Y.encodeStateAsUpdate(doc));
    assert.deepEqual(observed.suggestions, []);
    apply(doc, [{
      op: 'suggest', block_id: 'a', thread: THREAD,
      expected_hash: blockHash(blocks(doc)[0]),
      block: paragraph('a', text('We ship on Monday to every customer.')),
    }]);
    const view = bodyView(Y.encodeStateAsUpdate(doc));
    const made = view.suggestions as Json[];
    assert.equal(made.length, 2, 'two changed stretches make two suggestions');
    for (const item of made) {
      assert.equal(item.author, AGENT);
      assert.equal(item.at, NOW.toISOString());
      assert.equal(item.thread, THREAD);
      assert.equal(item.block_id, 'a');
      assert.equal(item.kind, 'replace');
    }
    assert.deepEqual(made.map((item) => [item.deleted, item.inserted]), [['Friday', 'Monday'], ['all customers', 'every customer']]);
    // Untouched words are untouched; the agreed text is still the old one until someone accepts.
    assert.deepEqual(rejected(doc), ['We ship on Friday to all customers.', 'Notes.']);
    assert.deepEqual(accepted(doc), ['We ship on Monday to every customer.', 'Notes.']);

    apply(doc, [{ op: 'accept', suggestion: made[0]!.id }], OWNER);
    apply(doc, [{ op: 'reject', suggestion: made[1]!.id }], OWNER);
    assert.deepEqual(blocks(doc).map(words), ['We ship on Monday to all customers.', 'Notes.']);
    assert.deepEqual(bodyView(Y.encodeStateAsUpdate(doc)).suggestions, []);
    assert.throws(() => apply(doc, [{ op: 'accept', suggestion: made[0]!.id }], OWNER), /no longer open/);
  } finally { doc.destroy(); }
});

test('suggested blocks: a new paragraph and a struck one go or stay as a whole', () => {
  const doc = live([paragraph('a', text('Keep this.')), paragraph('b', text('Cut this.')), paragraph('c', text('End.'))]);
  try {
    apply(doc, [
      { op: 'suggest_insert', after: 'a', block: paragraph('n', text('A new idea.')) },
      { op: 'suggest_delete', block_id: 'b', expected_hash: blockHash(blocks(doc)[1]) },
    ]);
    assert.deepEqual(blocks(doc).map(words), ['Keep this.', 'A new idea.', 'Cut this.', 'End.']);
    assert.deepEqual(accepted(doc), ['Keep this.', 'A new idea.', 'End.']);
    assert.deepEqual(rejected(doc), ['Keep this.', 'Cut this.', 'End.']);
    // Pictures, mentions and line breaks are never suggested.
    assert.throws(() => apply(doc, [{ op: 'suggest_insert', after: 'a', block: paragraph('m', text('Hi '), { type: 'mention', attrs: { actor_id: 'bob', label: 'Bob' } }) }]), InvalidBodyEdit);
  } finally { doc.destroy(); }
});

test('a suggestion is only ever in its author\'s name, and structure changes are refused', () => {
  const doc = live([paragraph('a', text('Opening.'))]);
  try {
    const hash = blockHash(blocks(doc)[0]);
    const forged = paragraph('a', text('Opening.'), text(' Forged.', [{ type: 'suggestedInsert', attrs: suggestion('f1', OWNER) }]));
    assert.throws(() => apply(doc, [{ op: 'replace', block_id: 'a', expected_hash: hash, block: forged }]), /author's name/);
    // Its own marks through replace are allowed.
    const own = paragraph('a', text('Opening.'), text(' Mine.', [{ type: 'suggestedInsert', attrs: suggestion('m1', AGENT) }]));
    assert.doesNotThrow(() => prepareBodyEdit(Y.encodeStateAsUpdate(doc), { operations: [{ op: 'replace', block_id: 'a', expected_hash: hash, block: own }] }, by(AGENT)));
    // Without a verified actor there is nobody to attribute a suggestion to.
    assert.throws(() => prepareBodyEdit(Y.encodeStateAsUpdate(doc), { operations: [{ op: 'suggest_delete', block_id: 'a', expected_hash: hash }] }), InvalidBodyEdit);
    assert.throws(() => apply(doc, [{ op: 'suggest', block_id: 'a', expected_hash: hash, block: { type: 'heading', attrs: { block_id: 'a', level: 2 }, content: [text('Opening.')] } }]), /structure/);
    assert.throws(() => apply(doc, [{ op: 'suggest', block_id: 'a', expected_hash: hash, block: paragraph('a', text('Opening.')) }]), /same as the current/);
    assert.throws(() => apply(doc, [{ op: 'suggest', block_id: 'a', expected_hash: hash, thread: 'x', block: paragraph('a', text('Better.')) }]), /thread/);
  } finally { doc.destroy(); }
});

test('an agent revising its own suggestion drops its own words rather than striking them', () => {
  const doc = live([paragraph('a', text('Ship Friday.'))]);
  try {
    apply(doc, [{ op: 'suggest', block_id: 'a', expected_hash: blockHash(blocks(doc)[0]), block: paragraph('a', text('Ship Monday.')) }]);
    // A proposal says how the block should read; struck words are already on their way out.
    apply(doc, [{ op: 'suggest', block_id: 'a', expected_hash: blockHash(blocks(doc)[0]), block: paragraph('a', text('Ship Tuesday.')) }]);
    assert.ok(!words(blocks(doc)[0]).includes('Monday'), 'its own withdrawn words are gone, not struck');
    assert.equal(words(blocks(doc)[0]), 'Ship FridayTuesday.');
    assert.deepEqual(accepted(doc), ['Ship Tuesday.']);
    assert.deepEqual(rejected(doc), ['Ship Friday.']);
  } finally { doc.destroy(); }
});

test('concurrent typing survives a suggestion and an acceptance, and every copy converges', () => {
  const owner = live([paragraph('a', text('Ship on Friday.')), paragraph('b', text('Second.'))]);
  const member = new Y.Doc();
  try {
    Y.applyUpdate(member, Y.encodeStateAsUpdate(owner));
    // The agent prepares against the owner's copy while the member types in the same paragraph.
    const prepared = prepareBodyEdit(Y.encodeStateAsUpdate(owner), { operations: [{
      op: 'suggest', block_id: 'a', expected_hash: blockHash(blocks(owner)[0]),
      block: paragraph('a', text('Ship on Monday.')),
    }] }, by(AGENT));
    yText(member, 0).insert(0, 'Please ');
    Y.applyUpdate(owner, checkedBodyUpdate(owner, prepared.update_base64));
    Y.applyUpdate(owner, Y.encodeStateAsUpdate(member));
    Y.applyUpdate(member, Y.encodeStateAsUpdate(owner));
    assert.deepEqual(blocks(owner), blocks(member));
    assert.deepEqual(accepted(owner), ['Please Ship on Monday.', 'Second.']);

    // The owner accepts while the member types inside the struck word and after the new one.
    const id = String((bodyView(Y.encodeStateAsUpdate(owner)).suggestions as Json[])[0]!.id);
    const accept = prepareBodyEdit(Y.encodeStateAsUpdate(owner), { operations: [{ op: 'accept', suggestion: id }] }, by(OWNER));
    const memberText = yText(member, 0);
    memberText.insert(memberText.length, ' Thanks!');
    memberText.insert((memberText.toString().replace(/<[^>]*>/gu, '')).indexOf('Friday') + 3, 'X');
    Y.applyUpdate(owner, checkedBodyUpdate(owner, accept.update_base64));
    Y.applyUpdate(member, Y.encodeStateAsUpdate(owner));
    Y.applyUpdate(owner, Y.encodeStateAsUpdate(member));
    assert.deepEqual(blocks(owner), blocks(member), 'both copies converge');
    // The struck word went; what the member typed inside it while it went stays.
    assert.equal(words(blocks(owner)[0]), 'Please Ship on XMonday. Thanks!');
    assert.deepEqual(bodyView(Y.encodeStateAsUpdate(owner)).suggestions, []);
  } finally { owner.destroy(); member.destroy(); }
});

test('two people suggesting at once keep two suggestions, each in its own name', () => {
  const left = live([paragraph('a', text('One two three.'))]);
  const right = new Y.Doc();
  try {
    Y.applyUpdate(right, Y.encodeStateAsUpdate(left));
    const first = prepareBodyEdit(Y.encodeStateAsUpdate(left), { operations: [{ op: 'suggest', block_id: 'a', expected_hash: blockHash(blocks(left)[0]), block: paragraph('a', text('One 2 three.')) }] }, by('alice'));
    const second = prepareBodyEdit(Y.encodeStateAsUpdate(right), { operations: [{ op: 'suggest', block_id: 'a', expected_hash: blockHash(blocks(right)[0]), block: paragraph('a', text('One two 3.')) }] }, by('bob'));
    Y.applyUpdate(left, checkedBodyUpdate(left, first.update_base64));
    Y.applyUpdate(right, checkedBodyUpdate(right, second.update_base64));
    Y.applyUpdate(left, Y.encodeStateAsUpdate(right));
    Y.applyUpdate(right, Y.encodeStateAsUpdate(left));
    assert.deepEqual(blocks(left), blocks(right));
    const open = suggestionsIn(documentSchema.nodeFromJSON(projectionFromState(Y.encodeStateAsUpdate(left))));
    assert.deepEqual(open.map((item) => [item.author, item.deleted, item.inserted]), [['alice', 'two', '2'], ['bob', 'three', '3']]);
    assert.deepEqual(accepted(left), ['One 2 3.']);
  } finally { left.destroy(); right.destroy(); }
});

test('accepting a deletion across paragraphs joins them; one that empties a list item takes it out', () => {
  const at = suggestion('d1');
  const struck = (value: string) => text(value, [{ type: 'suggestedDelete', attrs: at }]);
  const doc = documentSchema.nodeFromJSON({
    type: 'doc',
    content: [
      paragraph('a', text('Keep '), struck('the end')),
      paragraph('b', struck('Start of'), text(' the next')),
      { type: 'bulletList', attrs: { block_id: 'l' }, content: [
        { type: 'listItem', attrs: { block_id: 'i1' }, content: [paragraph('p1', text('Stays'))] },
        { type: 'listItem', attrs: { block_id: 'i2' }, content: [paragraph('p2', text('Goes', [{ type: 'suggestedDelete', attrs: suggestion('d2') }]))] },
      ] },
    ],
  });
  const tr = new Transform(doc);
  assert.deepEqual(resolveSuggestions(tr, () => true, true).sort(), ['d1', 'd2']);
  const result = tr.doc.toJSON().content as Json[];
  assert.equal(result.length, 2);
  assert.equal(words(result[0]), 'Keep  the next');
  const items = (result[1]!.content as Json[]);
  assert.equal(items.length, 1, 'the emptied list item went');
  // Rejecting instead keeps everything, unmarked.
  const back = new Transform(doc);
  resolveSuggestions(back, () => true, false);
  assert.equal(suggestionsIn(back.doc).length, 0);
  assert.equal(back.doc.childCount, 3);
});

test('the word diff keeps common words and groups each changed stretch', () => {
  assert.deepEqual(diffTokens(['a', ' ', 'b', ' ', 'c'], ['a', ' ', 'x', ' ', 'c']), [{ oldStart: 2, oldEnd: 3, newStart: 2, newEnd: 3 }]);
  assert.deepEqual(diffTokens(['a'], ['a']), []);
  assert.deepEqual(diffTokens([], ['a']), [{ oldStart: 0, oldEnd: 0, newStart: 0, newEnd: 1 }]);
  assert.deepEqual(diffTokens(['a', 'b', 'c', 'd'], ['a', 'c', 'd', 'e']), [
    { oldStart: 1, oldEnd: 2, newStart: 1, newEnd: 1 },
    { oldStart: 4, oldEnd: 4, newStart: 3, newEnd: 4 },
  ]);
});
