import { Schema, type MarkSpec, type NodeSpec } from 'prosemirror-model';
import { initProseMirrorDoc, prosemirrorJSONToYDoc, updateYFragment, yDocToProsemirrorJSON } from 'y-prosemirror';
import * as Y from 'yjs';

import { MAX_YJS_STATE_BYTES } from './constants.js';

export const PROSEMIRROR_FRAGMENT_NAME = 'default';

const MAX_DOCUMENT_BYTES = 1_000_000;
const MAX_DOCUMENT_DEPTH = 64;
const MAX_DOCUMENT_NODES = 20_000;
const MAX_DOCUMENT_TEXT_CHARACTERS = 500_000;

type JsonObject = Record<string, unknown>;

const blockId = { block_id: { default: null } } as const;

const nodes: Record<string, NodeSpec> = {
  doc: { content: 'block*' },
  paragraph: { attrs: { block_id: { default: null }, align: { default: null } }, content: 'inline*', group: 'block' },
  heading: {
    attrs: { block_id: { default: null }, level: { default: 1 }, align: { default: null } },
    content: 'inline*',
    defining: true,
    group: 'block',
  },
  bulletList: { attrs: blockId, content: 'listItem+', group: 'block' },
  orderedList: {
    attrs: { block_id: { default: null }, start: { default: 1 } },
    content: 'listItem+',
    group: 'block',
  },
  taskList: { attrs: blockId, content: 'taskItem+', group: 'block' },
  listItem: { attrs: blockId, content: 'block+' },
  taskItem: {
    attrs: { block_id: { default: null }, checked: { default: false } },
    content: 'block+',
  },
  blockquote: { attrs: blockId, content: 'block+', defining: true, group: 'block' },
  codeBlock: {
    attrs: { block_id: { default: null }, language: { default: null } },
    code: true,
    content: 'text*',
    defining: true,
    group: 'block',
    marks: '',
  },
  horizontalRule: { attrs: blockId, group: 'block' },
  image: {
    attrs: {
      block_id: { default: null },
      attachment_id: {},
      alt: { default: null },
      width: { default: null },
      caption: { default: null },
    },
    atom: true,
    group: 'block',
  },
  file: {
    attrs: { block_id: { default: null }, attachment_id: {}, name: {} },
    atom: true,
    group: 'block',
  },
  table: { attrs: blockId, content: 'tableRow+', group: 'block', isolating: true },
  tableRow: { attrs: blockId, content: '(tableCell | tableHeader)+' },
  tableCell: { attrs: { block_id: { default: null }, width: { default: null } }, content: 'block+', isolating: true },
  tableHeader: { attrs: { block_id: { default: null }, width: { default: null } }, content: 'block+', isolating: true },
  text: { group: 'inline' },
  hardBreak: { inline: true, group: 'inline', selectable: false },
  mention: {
    attrs: { actor_id: {}, label: {} },
    atom: true,
    inline: true,
    group: 'inline',
  },
  reference: {
    attrs: { id: {}, kind: {}, label: {} },
    atom: true,
    inline: true,
    group: 'inline',
  },
};

const marks: Record<string, MarkSpec> = {
  bold: {},
  italic: {},
  underline: {},
  strike: {},
  highlight: { attrs: { color: {} } },
  textColor: { attrs: { color: {} } },
  code: { code: true, excludes: '_' },
  link: {
    attrs: { href: {}, title: { default: null } },
    inclusive: false,
  },
};

const documentSchema = new Schema({ nodes, marks });

export class DocumentCodecError extends Error {
  constructor() {
    super('native Documents structured content is invalid');
    this.name = 'DocumentCodecError';
  }
}

/** Which cap a body passed: its JSON size, its node count, or its text length. */
export type DocumentSizeMeasure = 'bytes' | 'nodes' | 'characters';

/** A body that is well formed but past a size cap. Editors are told which cap, so the owner reads
 * "too large" rather than "holds something documents can't keep". */
export class DocumentTooLargeError extends DocumentCodecError {
  constructor(readonly measure: DocumentSizeMeasure) {
    super();
    this.name = 'DocumentTooLargeError';
  }
}

function invalid(): never {
  throw new DocumentCodecError();
}

function tooLarge(measure: DocumentSizeMeasure): never {
  throw new DocumentTooLargeError(measure);
}

function object(value: unknown): JsonObject {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) invalid();
  return value as JsonObject;
}

function exactKeys(value: JsonObject, allowed: readonly string[]): void {
  if (Object.keys(value).some((key) => !allowed.includes(key))) invalid();
}

function boundedString(value: unknown, maximum: number): string {
  if (
    typeof value !== 'string' ||
    value.length < 1 ||
    value.length > maximum ||
    value.trim().length < 1 ||
    value.includes('\0')
  ) {
    invalid();
  }
  return value;
}

function safeHref(value: string): boolean {
  if ([...value].some((character) => /\s|\p{Cc}/u.test(character))) return false;
  if (value.startsWith('https://') || value.startsWith('http://')) {
    try {
      const parsed = new URL(value);
      return parsed.protocol === 'https:' || parsed.protocol === 'http:';
    } catch {
      return false;
    }
  }
  if (value.startsWith('mailto:')) {
    const address = value.slice('mailto:'.length);
    const separator = address.indexOf('@');
    return separator > 0 && separator < address.length - 1;
  }
  return (value.startsWith('/') && !value.startsWith('//') && !value.includes('\\')) ||
    (value.startsWith('#') && value.length > 1);
}

interface ValidationState {
  nodes: number;
  textCharacters: number;
  readonly blockIds: Set<string>;
}

const MARK_TYPES = ['bold', 'italic', 'underline', 'strike', 'code', 'link', 'highlight', 'textColor'];
// Colours and alignments are names from a small palette; web/src/lib/model/document-editor-schema.ts.
const TEXT_COLORS = ['gray', 'red', 'orange', 'green', 'blue', 'purple'];
const HIGHLIGHTS = ['yellow', 'green', 'blue', 'pink', 'purple'];
const ALIGNMENTS = ['center', 'right', 'justify'];

function validateMarks(value: unknown): void {
  if (value === undefined) return;
  if (!Array.isArray(value)) invalid();
  const seen = new Set<string>();
  for (const candidate of value) {
    const mark = object(candidate);
    exactKeys(mark, ['type', 'attrs']);
    const type = mark.type;
    if (typeof type !== 'string' || !MARK_TYPES.includes(type) || seen.has(type)) {
      invalid();
    }
    seen.add(type);
    const attrs = mark.attrs === undefined ? {} : object(mark.attrs);
    if (type === 'highlight' || type === 'textColor') {
      exactKeys(attrs, ['color']);
      const palette = type === 'highlight' ? HIGHLIGHTS : TEXT_COLORS;
      if (typeof attrs.color !== 'string' || !palette.includes(attrs.color)) invalid();
      continue;
    }
    if (type !== 'link') {
      if (Object.keys(attrs).length !== 0) invalid();
      continue;
    }
    exactKeys(attrs, ['href', 'title']);
    const href = boundedString(attrs.href, 2_048);
    if (!safeHref(href)) invalid();
    if (attrs.title !== undefined && attrs.title !== null) boundedString(attrs.title, 300);
  }
}

const ATTACHMENT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;

function validateAttributes(type: string, value: unknown, state: ValidationState): void {
  const attrs = value === undefined ? {} : object(value);
  const allowed = (() => {
    switch (type) {
      case 'heading': return ['block_id', 'level', 'align'];
      case 'paragraph': return ['block_id', 'align'];
      case 'orderedList': return ['block_id', 'start'];
      case 'taskItem': return ['block_id', 'checked'];
      case 'codeBlock': return ['block_id', 'language'];
      case 'mention': return ['actor_id', 'label'];
      case 'reference': return ['id', 'kind', 'label'];
      case 'image': return ['block_id', 'attachment_id', 'alt', 'width', 'caption'];
      case 'file': return ['block_id', 'attachment_id', 'name'];
      case 'bulletList':
      case 'taskList':
      case 'listItem':
      case 'blockquote':
      case 'horizontalRule':
      case 'table':
      case 'tableRow':
        return ['block_id'];
      case 'tableCell':
      case 'tableHeader':
        return ['block_id', 'width'];
      default:
        return [];
    }
  })();
  exactKeys(attrs, allowed);

  if (attrs.block_id !== undefined) {
    const id = boundedString(attrs.block_id, 128);
    if (!/^[A-Za-z0-9_.:-]+$/u.test(id) || state.blockIds.has(id)) invalid();
    state.blockIds.add(id);
  }
  if (attrs.align !== undefined && attrs.align !== null && (typeof attrs.align !== 'string' || !ALIGNMENTS.includes(attrs.align))) invalid();
  if (type === 'heading' && (!Number.isInteger(attrs.level) || (attrs.level as number) < 1 || (attrs.level as number) > 6)) invalid();
  if (
    type === 'orderedList' &&
    attrs.start !== undefined &&
    (!Number.isInteger(attrs.start) || (attrs.start as number) < 1 || (attrs.start as number) > 100_000)
  ) invalid();
  if (type === 'taskItem' && typeof attrs.checked !== 'boolean') invalid();
  if (type === 'codeBlock' && attrs.language !== undefined && attrs.language !== null) boundedString(attrs.language, 64);
  if (type === 'mention') {
    boundedString(attrs.actor_id, 128);
    boundedString(attrs.label, 200);
  }
  if (type === 'image') {
    // An image names a company file by its identity; the page supplies the address.
    if (typeof attrs.attachment_id !== 'string' || !ATTACHMENT_ID.test(attrs.attachment_id)) invalid();
    if (
      attrs.alt !== undefined && attrs.alt !== null &&
      (typeof attrs.alt !== 'string' || [...attrs.alt].length > 300 || attrs.alt.includes('\0'))
    ) invalid();
    if (attrs.caption !== undefined && attrs.caption !== null) boundedString(attrs.caption, 300);
    if (
      attrs.width !== undefined && attrs.width !== null &&
      (!Number.isInteger(attrs.width) || (attrs.width as number) < 16 || (attrs.width as number) > 4_096)
    ) invalid();
  }
  if (
    (type === 'tableCell' || type === 'tableHeader') &&
    attrs.width !== undefined && attrs.width !== null &&
    (!Number.isInteger(attrs.width) || (attrs.width as number) < 60 || (attrs.width as number) > 1_600)
  ) invalid();
  if (type === 'file') {
    // A file names a company file by its identity, as an image does.
    if (typeof attrs.attachment_id !== 'string' || !ATTACHMENT_ID.test(attrs.attachment_id)) invalid();
    boundedString(attrs.name, 255);
  }
  if (type === 'reference') {
    const kind = boundedString(attrs.kind, 32);
    if (!['work', 'decision', 'attention', 'document', 'artifact', 'actor'].includes(kind)) invalid();
    boundedString(attrs.id, 200);
    boundedString(attrs.label, 300);
  }
}

function validateNode(value: unknown, depth: number, state: ValidationState, topLevel: boolean): void {
  if (depth > MAX_DOCUMENT_DEPTH) invalid();
  state.nodes += 1;
  if (state.nodes > MAX_DOCUMENT_NODES) tooLarge('nodes');
  const node = object(value);
  const type = node.type;
  if (typeof type !== 'string') invalid();
  if (type === 'text') {
    exactKeys(node, ['type', 'text', 'marks']);
    if (typeof node.text !== 'string' || node.text.includes('\0')) invalid();
    state.textCharacters += [...node.text].length;
    if (state.textCharacters > MAX_DOCUMENT_TEXT_CHARACTERS) tooLarge('characters');
    validateMarks(node.marks);
    return;
  }
  exactKeys(node, ['type', 'attrs', 'content']);
  validateAttributes(type, node.attrs, state);
  if (topLevel) {
    const attrs = node.attrs === undefined ? {} : object(node.attrs);
    if (typeof attrs.block_id !== 'string') invalid();
  }
  if (node.content !== undefined && !Array.isArray(node.content)) invalid();
  for (const child of (node.content ?? []) as unknown[]) validateNode(child, depth + 1, state, type === 'doc');
}

function validateProjection(value: unknown): JsonObject {
  const projection = object(value);
  const encoded = JSON.stringify(projection);
  if (Buffer.byteLength(encoded) > MAX_DOCUMENT_BYTES) tooLarge('bytes');
  validateNode(projection, 0, { nodes: 0, textCharacters: 0, blockIds: new Set() }, false);
  try {
    documentSchema.nodeFromJSON(projection).check();
  } catch {
    invalid();
  }
  return projection;
}

export function stateFromProjection(value: unknown): Uint8Array {
  const projection = validateProjection(value);
  try {
    const document = prosemirrorJSONToYDoc(documentSchema, projection, PROSEMIRROR_FRAGMENT_NAME);
    const state = Y.encodeStateAsUpdate(document);
    document.destroy();
    if (state.byteLength < 2) invalid();
    if (state.byteLength > MAX_YJS_STATE_BYTES) tooLarge('bytes');
    return state;
  } catch (error) {
    if (error instanceof DocumentCodecError) throw error;
    invalid();
  }
}

export function projectionFromState(state: Uint8Array): JsonObject {
  if (!(state instanceof Uint8Array) || state.byteLength < 2) invalid();
  if (state.byteLength > MAX_YJS_STATE_BYTES) tooLarge('bytes');
  const document = new Y.Doc();
  try {
    Y.applyUpdate(document, state);
    const sharedTypeNames = [...document.share.keys()];
    if (sharedTypeNames.length !== 1 || sharedTypeNames[0] !== PROSEMIRROR_FRAGMENT_NAME) invalid();
    const fragment = document.getXmlFragment(PROSEMIRROR_FRAGMENT_NAME);
    if (!(fragment instanceof Y.XmlFragment)) invalid();
    return validateProjection(yDocToProsemirrorJSON(document, PROSEMIRROR_FRAGMENT_NAME));
  } catch (error) {
    if (error instanceof DocumentCodecError) throw error;
    invalid();
  } finally {
    document.destroy();
  }
}

/** Compute an incremental update against the existing CRDT identities. Never reseed a live body. */
export function updateForProjection(state: Uint8Array, value: unknown): Uint8Array {
  projectionFromState(state);
  const projection = validateProjection(value);
  const document = new Y.Doc();
  try {
    Y.applyUpdate(document, state);
    const before = Y.encodeStateVector(document);
    const fragment = document.getXmlFragment(PROSEMIRROR_FRAGMENT_NAME);
    const { meta } = initProseMirrorDoc(fragment, documentSchema);
    updateYFragment(document, fragment, documentSchema.nodeFromJSON(projection), meta);
    projectionFromState(Y.encodeStateAsUpdate(document));
    return Y.encodeStateAsUpdate(document, before);
  } finally {
    document.destroy();
  }
}
