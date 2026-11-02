import { createHash } from 'node:crypto';
import type { Node as ProseMirrorNode } from 'prosemirror-model';
import { Transform } from 'prosemirror-transform';
import * as Y from 'yjs';
import { MAX_YJS_STATE_BYTES } from './constants.js';
import { DocumentCodecError, documentSchema, projectionFromState, updateForProjection } from './document-codec.js';
import {
  InvalidSuggestion,
  suggestBlockDeletion,
  suggestBlockEdit,
  suggestedBlock,
  withoutSuggestions,
  type SuggestionAuthor,
} from './suggest-edits.js';
import { resolveSuggestions, suggestionsIn } from './suggestions.js';

type Block = Record<string, unknown>;
export class BodyEditConflict extends Error {}
export class InvalidBodyEdit extends Error {}

function object(value: unknown): Block {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new InvalidBodyEdit('expected an object');
  return value as Block;
}
function exact(value: Block, fields: string[]): void {
  if (Object.keys(value).length !== fields.length || fields.some((field) => !(field in value))) {
    throw new InvalidBodyEdit('unexpected or missing edit field');
  }
}
function exactWithOptional(value: Block, fields: string[], optional: string[]): void {
  if (fields.some((field) => !(field in value)) || Object.keys(value).some((key) => !fields.includes(key) && !optional.includes(key))) {
    throw new InvalidBodyEdit('unexpected or missing edit field');
  }
}
function blockId(value: unknown): string {
  const id = object(object(value).attrs).block_id;
  if (typeof id !== 'string' || !id) throw new InvalidBodyEdit('block needs a stable ID');
  return id;
}
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical((value as Block)[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}
export function blockHash(block: unknown): string {
  return createHash('sha256').update(canonical(block)).digest('hex');
}
/** The live body, each top-level block's guard hash, and the open suggestions in it: who proposed
 * what, where, and which comment thread each answers (Sprint 109 E4). */
export function bodyView(state: Uint8Array): Block {
  const content = projectionFromState(state);
  const blocks = content.content as Block[];
  const suggestions = suggestionsIn(documentSchema.nodeFromJSON(content)).map(
    ({ from: _from, to: _to, ...summary }) => summary,
  );
  return {
    content_json: content,
    blocks: blocks.map((block) => ({ block_id: blockId(block), hash: blockHash(block) })),
    suggestions,
  };
}

/** Who is editing through the body route: the actor Core verified, and when. A suggestion is made
 * in this actor's name; nobody can propose words under someone else's. */
export interface BodyEditAuthor {
  actorId: string;
  now: Date;
}

function guard(operation: Block, blocks: Block[], touched: Set<string>): number {
  if (typeof operation.block_id !== 'string' || typeof operation.expected_hash !== 'string' || !/^[a-f0-9]{64}$/.test(operation.expected_hash)) {
    throw new InvalidBodyEdit('edit needs a block ID and its observed SHA-256 hash');
  }
  if (touched.has(operation.block_id)) throw new InvalidBodyEdit('edit each block at most once per command');
  const index = blocks.findIndex((block) => blockId(block) === operation.block_id);
  if (index < 0 || blockHash(blocks[index]) !== operation.expected_hash) throw new BodyEditConflict('block changed; read it again before editing');
  return index;
}

function thread(operation: Block): string | null {
  if (operation.thread === undefined || operation.thread === null) return null;
  if (typeof operation.thread !== 'string' || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(operation.thread)) {
    throw new InvalidBodyEdit('thread must be a comment thread ID');
  }
  return operation.thread;
}

function node(value: unknown) {
  try {
    const parsed = documentSchema.nodeFromJSON(value);
    parsed.check();
    return parsed;
  } catch { throw new InvalidBodyEdit('invalid block content'); }
}

/** Run a ProseMirror change over the working blocks; untouched blocks keep their exact JSON, so
 * the hashes an agent read for them still hold later in the same command. */
function transformBlocks(projection: Block, blocks: Block[], change: (tr: Transform) => void): Block[] {
  const before = node({ ...projection, content: blocks });
  const tr = new Transform(before);
  try { change(tr); } catch (error) {
    if (error instanceof InvalidSuggestion) throw new InvalidBodyEdit(error.message);
    throw error;
  }
  const kept = new Map<string, { json: Block; node: ProseMirrorNode }>();
  before.forEach((child, _offset, index) => {
    const json = blocks[index]!;
    kept.set(blockId(json), { json, node: child });
  });
  const after: Block[] = [];
  tr.doc.forEach((child) => {
    const id = child.attrs.block_id;
    const same = typeof id === 'string' ? kept.get(id) : undefined;
    after.push(same && same.node.eq(child) ? same.json : (child.toJSON() as Block));
  });
  return after;
}

/** Suggestion marks a block may carry: ones already in the document, or new ones in this actor's
 * own name. */
function ownSuggestions(block: Block, existing: ReadonlySet<string>, actorId: string): void {
  const visit = (value: unknown): void => {
    if (!value || typeof value !== 'object') return;
    const item = value as Block;
    for (const mark of Array.isArray(item.marks) ? item.marks : []) {
      const { type, attrs } = (mark ?? {}) as Block;
      if (type !== 'suggestedInsert' && type !== 'suggestedDelete') continue;
      const { id, author } = (attrs ?? {}) as Block;
      if (author !== actorId && !existing.has(`${String(id)}\u0000${String(author)}`)) {
        throw new InvalidBodyEdit('a suggestion is made in its author\'s name only');
      }
    }
    if (Array.isArray(item.content)) item.content.forEach(visit);
  };
  visit(block);
}

/** Pure preparation: validation and guards run on a clone, never on the shared live document.
 * Core must retain the returned exact update for this command before applying it.
 *
 * Besides insert, replace and delete, an agent can propose instead of change (Sprint 109 E4):
 * `suggest` (a block's new words, marked as insertions and deletions in its name), `suggest_insert`
 * (a new block of words as one insertion), `suggest_delete` (a block's words struck), each with an
 * optional comment `thread` it answers; and `accept` or `reject` a suggestion by id. */
export function prepareBodyEdit(
  state: Uint8Array,
  raw: unknown,
  author: BodyEditAuthor = { actorId: '', now: new Date() },
): { update_base64: string; affected_blocks: string[] } {
  const request = object(raw);
  exact(request, ['operations']);
  if (!Array.isArray(request.operations) || request.operations.length < 1 || request.operations.length > 100) {
    throw new InvalidBodyEdit('expected 1 to 100 block operations');
  }
  const projection = projectionFromState(state);
  let blocks = (projection.content as Block[]).slice();
  const existing = new Set(
    suggestionsIn(node(projection)).map((summary) => `${summary.id}\u0000${summary.author}`),
  );
  const touched = new Set<string>();
  const proposer = (operation: Block): SuggestionAuthor => {
    if (!author.actorId) throw new InvalidBodyEdit('a suggestion needs a verified author');
    return { author: author.actorId, at: author.now.toISOString(), thread: thread(operation) };
  };
  const topPos = (doc: ProseMirrorNode, id: string): number => {
    let found = -1;
    doc.forEach((child, offset) => { if (found < 0 && child.attrs.block_id === id) found = offset; });
    return found;
  };
  for (const rawOperation of request.operations) {
    const operation = object(rawOperation);
    if (operation.op === 'insert') {
      exact(operation, ['op', 'after', 'block']);
      const id = blockId(operation.block);
      if (touched.has(id) || blocks.some((block) => blockId(block) === id)) throw new BodyEditConflict('inserted block already exists');
      let index = -1;
      if (operation.after !== null) {
        if (typeof operation.after !== 'string') throw new InvalidBodyEdit('insert needs an anchor ID or null');
        index = blocks.findIndex((block) => blockId(block) === operation.after);
        if (index < 0) throw new BodyEditConflict('insertion anchor no longer exists');
      }
      ownSuggestions(object(operation.block), existing, author.actorId);
      blocks.splice(index + 1, 0, object(operation.block));
      touched.add(id);
    } else if (operation.op === 'replace' || operation.op === 'delete') {
      exact(operation, operation.op === 'replace' ? ['op', 'block_id', 'expected_hash', 'block'] : ['op', 'block_id', 'expected_hash']);
      const index = guard(operation, blocks, touched);
      if (operation.op === 'delete') blocks.splice(index, 1);
      else {
        if (blockId(operation.block) !== operation.block_id) throw new InvalidBodyEdit('replacement must preserve the block ID');
        ownSuggestions(object(operation.block), existing, author.actorId);
        blocks[index] = object(operation.block);
      }
      touched.add(operation.block_id as string);
    } else if (operation.op === 'suggest' || operation.op === 'suggest_delete') {
      const fields = operation.op === 'suggest' ? ['op', 'block_id', 'expected_hash', 'block'] : ['op', 'block_id', 'expected_hash'];
      exactWithOptional(operation, fields, ['thread']);
      guard(operation, blocks, touched);
      const id = operation.block_id as string;
      const who = proposer(operation);
      if (operation.op === 'suggest' && blockId(operation.block) !== id) throw new InvalidBodyEdit('a suggestion must preserve the block ID');
      const proposed = operation.op === 'suggest' ? node(operation.block) : null;
      let made = 1;
      blocks = transformBlocks(projection, blocks, (tr) => {
        const at = topPos(tr.doc, id);
        if (proposed) made = suggestBlockEdit(tr, at, proposed, who);
        else suggestBlockDeletion(tr, at, who);
      });
      if (!made) throw new InvalidBodyEdit('the suggested block is the same as the current one');
      touched.add(id);
    } else if (operation.op === 'suggest_insert') {
      exactWithOptional(operation, ['op', 'after', 'block'], ['thread']);
      const id = blockId(operation.block);
      if (touched.has(id) || blocks.some((block) => blockId(block) === id)) throw new BodyEditConflict('inserted block already exists');
      let index = -1;
      if (operation.after !== null) {
        if (typeof operation.after !== 'string') throw new InvalidBodyEdit('insert needs an anchor ID or null');
        index = blocks.findIndex((block) => blockId(block) === operation.after);
        if (index < 0) throw new BodyEditConflict('insertion anchor no longer exists');
      }
      let marked: Block;
      try { marked = suggestedBlock(withoutSuggestions(node(operation.block)), proposer(operation)).toJSON() as Block; }
      catch (error) {
        if (error instanceof InvalidSuggestion) throw new InvalidBodyEdit(error.message);
        throw error;
      }
      blocks.splice(index + 1, 0, marked);
      touched.add(id);
    } else if (operation.op === 'accept' || operation.op === 'reject') {
      exact(operation, ['op', 'suggestion']);
      if (typeof operation.suggestion !== 'string' || !/^[A-Za-z0-9_-]{1,64}$/u.test(operation.suggestion)) {
        throw new InvalidBodyEdit('accept and reject name a suggestion by its id');
      }
      const wanted = operation.suggestion;
      const current = suggestionsIn(node({ ...projection, content: blocks })).find((summary) => summary.id === wanted);
      if (!current) throw new BodyEditConflict('the suggestion is no longer open; read the document again');
      blocks = transformBlocks(projection, blocks, (tr) => {
        resolveSuggestions(tr, (attrs) => attrs.id === wanted, operation.op === 'accept');
      });
      if (current.block_id) touched.add(current.block_id);
    } else throw new InvalidBodyEdit('unknown block operation');
  }
  let update: Uint8Array;
  try { update = updateForProjection(state, { ...projection, content: blocks }); }
  catch (error) {
    if (error instanceof DocumentCodecError) throw new InvalidBodyEdit('invalid document content');
    throw error;
  }
  if (update.byteLength > MAX_YJS_STATE_BYTES) throw new InvalidBodyEdit('update is too large');
  return { update_base64: Buffer.from(update).toString('base64'), affected_blocks: [...touched] };
}

/** Validate the full merged projection on a scratch Doc before touching the live one. */
export function checkedBodyUpdate(document: Y.Doc, encoded: unknown): Uint8Array {
  if (typeof encoded !== 'string' || encoded.length > Math.ceil(MAX_YJS_STATE_BYTES / 3) * 4) throw new InvalidBodyEdit('invalid update');
  const update = Buffer.from(encoded, 'base64');
  if (update.length < 2 || update.toString('base64') !== encoded) throw new InvalidBodyEdit('invalid update encoding');
  const candidate = new Y.Doc();
  try {
    Y.applyUpdate(candidate, Y.encodeStateAsUpdate(document));
    try { Y.applyUpdate(candidate, update); } catch { throw new InvalidBodyEdit('invalid Yjs update'); }
    // Unresolved dependencies indicate a delta against a different/obsolete live body.
    if (candidate.store.pendingStructs || candidate.store.pendingDs) throw new BodyEditConflict('update does not belong to the current body');
    try { projectionFromState(Y.encodeStateAsUpdate(candidate)); }
    catch { throw new InvalidBodyEdit('invalid merged document content'); }
    return update;
  } finally { candidate.destroy(); }
}
