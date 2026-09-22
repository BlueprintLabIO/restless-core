import { createHash } from 'node:crypto';
import * as Y from 'yjs';
import { MAX_YJS_STATE_BYTES } from './constants.js';
import { DocumentCodecError, projectionFromState, updateForProjection } from './document-codec.js';

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
export function bodyView(state: Uint8Array): Block {
  const content = projectionFromState(state);
  const blocks = content.content as Block[];
  return { content_json: content, blocks: blocks.map((block) => ({ block_id: blockId(block), hash: blockHash(block) })) };
}

/** Pure preparation: validation and guards run on a clone, never on the shared live document.
 * Core must retain the returned exact update for this command before applying it. */
export function prepareBodyEdit(state: Uint8Array, raw: unknown): { update_base64: string; affected_blocks: string[] } {
  const request = object(raw);
  exact(request, ['operations']);
  if (!Array.isArray(request.operations) || request.operations.length < 1 || request.operations.length > 100) {
    throw new InvalidBodyEdit('expected 1 to 100 block operations');
  }
  const projection = projectionFromState(state);
  const blocks = (projection.content as Block[]).slice();
  const touched = new Set<string>();
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
      blocks.splice(index + 1, 0, object(operation.block));
      touched.add(id);
    } else if (operation.op === 'replace' || operation.op === 'delete') {
      exact(operation, operation.op === 'replace' ? ['op', 'block_id', 'expected_hash', 'block'] : ['op', 'block_id', 'expected_hash']);
      if (typeof operation.block_id !== 'string' || typeof operation.expected_hash !== 'string' || !/^[a-f0-9]{64}$/.test(operation.expected_hash)) {
        throw new InvalidBodyEdit('edit needs a block ID and its observed SHA-256 hash');
      }
      if (touched.has(operation.block_id)) throw new InvalidBodyEdit('edit each block at most once per command');
      const index = blocks.findIndex((block) => blockId(block) === operation.block_id);
      if (index < 0 || blockHash(blocks[index]) !== operation.expected_hash) throw new BodyEditConflict('block changed; read it again before editing');
      if (operation.op === 'delete') blocks.splice(index, 1);
      else {
        if (blockId(operation.block) !== operation.block_id) throw new InvalidBodyEdit('replacement must preserve the block ID');
        blocks[index] = object(operation.block);
      }
      touched.add(operation.block_id);
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
