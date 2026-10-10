import {
	InputRule,
	Mark,
	Node,
	combineTransactionSteps,
	findChildrenInRange,
	getChangedRanges,
	mergeAttributes,
	type Editor,
	type Extensions,
	type JSONContent
} from '@tiptap/core';
import Collaboration, { isChangeOrigin } from '@tiptap/extension-collaboration';
import CollaborationCaret from '@tiptap/extension-collaboration-caret';
import { OrderedList } from '@tiptap/extension-list';
import TaskItem from '@tiptap/extension-task-item';
import TaskList from '@tiptap/extension-task-list';
import UniqueID from '@tiptap/extension-unique-id';
import StarterKit from '@tiptap/starter-kit';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, TextSelection, type EditorState } from '@tiptap/pm/state';
import type { HocuspocusProvider } from '@hocuspocus/provider';
import type { Doc } from 'yjs';
import type { DocumentContent, DocumentMark, DocumentNode } from './documents';

export const DOCUMENT_COLLABORATION_FRAGMENT = 'default';

const BLOCK_TYPES = [
	'paragraph',
	'heading',
	'bulletList',
	'orderedList',
	'taskList',
	'listItem',
	'taskItem',
	'blockquote',
	'codeBlock',
	'horizontalRule',
	'table',
	'tableRow',
	'tableCell',
	'tableHeader'
] as const;

/* The documents sidecar stores only what the company schema allows and refuses the whole body
 * otherwise, so anything the browser may parse (a paste, an input rule) is held to the same bounds
 * here. These mirror services/native-documents-collaboration/src/document-codec.ts. */
const REFERENCE_KINDS = ['work', 'decision', 'attention', 'document', 'artifact', 'actor'];
const MAX_HREF = 2_048;
const MAX_LINK_TITLE = 300;
const MAX_ORDERED_START = 100_000;

const BLOCK_ID = {
	block_id: {
		default: null,
		parseHTML: (element: HTMLElement) => element.getAttribute('data-block-id'),
		renderHTML: (attributes: Record<string, unknown>) =>
			typeof attributes.block_id === 'string' ? { 'data-block-id': attributes.block_id } : {}
	}
};

interface TableContext {
	table: ProseMirrorNode;
	tablePos: number;
	tableStart: number;
	row: ProseMirrorNode;
	rowPos: number;
	rowIndex: number;
	column: number;
}

/** The table cell the selection is in, innermost first. */
function tableContext(state: EditorState): TableContext | null {
	const { $from } = state.selection;
	for (let depth = $from.depth; depth > 2; depth -= 1) {
		const name = $from.node(depth).type.name;
		if (name !== 'tableCell' && name !== 'tableHeader') continue;
		return {
			table: $from.node(depth - 2),
			tablePos: $from.before(depth - 2),
			tableStart: $from.start(depth - 2),
			row: $from.node(depth - 1),
			rowPos: $from.before(depth - 1),
			rowIndex: $from.index(depth - 2),
			column: $from.index(depth - 1)
		};
	}
	return null;
}

export function inDocumentTable(editor: Editor): boolean {
	return tableContext(editor.state) !== null;
}

export type DocumentTableAction =
	'rowAfter' | 'columnAfter' | 'deleteRow' | 'deleteColumn' | 'deleteTable';

/* Tables here are plain grids with no merged cells, so rows and columns are added and removed
 * cell by cell. Nothing new reaches the schema: cells carry only their block identity. */
export function runDocumentTableAction(editor: Editor, action: DocumentTableAction): boolean {
	const { state, view } = editor;
	const context = tableContext(state);
	if (!context || !editor.isEditable) return false;
	const { schema } = state;
	const tr = state.tr;
	const removeTable = () =>
		tr.replaceWith(
			context.tablePos,
			context.tablePos + context.table.nodeSize,
			schema.nodes.paragraph.create()
		);
	switch (action) {
		case 'rowAfter': {
			const cells: ProseMirrorNode[] = [];
			context.row.forEach(() => cells.push(schema.nodes.tableCell.createAndFill()!));
			const at = context.rowPos + context.row.nodeSize;
			tr.insert(at, schema.nodes.tableRow.create(null, cells));
			tr.setSelection(TextSelection.near(tr.doc.resolve(at + 2)));
			break;
		}
		case 'columnAfter': {
			const inserts: Array<{ at: number; cell: ProseMirrorNode }> = [];
			context.table.forEach((row, offset) => {
				const index = Math.min(context.column, row.childCount - 1);
				let at = context.tableStart + offset + 1;
				for (let cell = 0; cell <= index; cell += 1) at += row.child(cell).nodeSize;
				inserts.push({ at, cell: row.child(index).type.createAndFill()! });
			});
			for (const insert of inserts.reverse()) tr.insert(insert.at, insert.cell);
			break;
		}
		case 'deleteRow': {
			if (context.table.childCount <= 1) {
				removeTable();
				break;
			}
			tr.delete(context.rowPos, context.rowPos + context.row.nodeSize);
			/* The caret stays in the table: in the row that moved up, or the one above the last. */
			const last = context.rowIndex === context.table.childCount - 1;
			const rowStart = last
				? context.rowPos - context.table.child(context.rowIndex - 1).nodeSize
				: context.rowPos;
			tr.setSelection(TextSelection.near(tr.doc.resolve(rowStart + 2)));
			break;
		}
		case 'deleteColumn': {
			const removals: Array<{ from: number; to: number }> = [];
			context.table.forEach((row, offset) => {
				if (row.childCount <= 1) return;
				const index = Math.min(context.column, row.childCount - 1);
				let from = context.tableStart + offset + 1;
				for (let cell = 0; cell < index; cell += 1) from += row.child(cell).nodeSize;
				removals.push({ from, to: from + row.child(index).nodeSize });
			});
			if (!removals.length) removeTable();
			else for (const removal of removals.reverse()) tr.delete(removal.from, removal.to);
			break;
		}
		case 'deleteTable':
			removeTable();
			break;
	}
	view.dispatch(tr.scrollIntoView());
	view.focus();
	return true;
}

/** Tab and Shift-Tab move between cells; Tab in the last cell adds a row. */
function moveBetweenCells(editor: Editor, direction: 1 | -1): boolean {
	const context = tableContext(editor.state);
	if (!context || !editor.isEditable) return false;
	const starts: number[] = [];
	let current = -1;
	context.table.forEach((row, rowOffset, rowIndex) => {
		row.forEach((_cell, cellOffset, cellIndex) => {
			if (rowIndex === context.rowIndex && cellIndex === context.column) current = starts.length;
			starts.push(context.tableStart + rowOffset + 1 + cellOffset);
		});
	});
	const next = current + direction;
	if (next < 0) return true;
	if (next >= starts.length) return runDocumentTableAction(editor, 'rowAfter');
	const tr = editor.state.tr;
	tr.setSelection(TextSelection.near(tr.doc.resolve(starts[next] + 1)));
	editor.view.dispatch(tr.scrollIntoView());
	return true;
}

const Table = Node.create({
	name: 'table',
	group: 'block',
	content: 'tableRow+',
	isolating: true,
	addAttributes: () => BLOCK_ID,
	parseHTML: () => [{ tag: 'table' }],
	renderHTML: ({ HTMLAttributes }) => ['table', HTMLAttributes, ['tbody', 0]],
	addKeyboardShortcuts() {
		return {
			Tab: () => moveBetweenCells(this.editor, 1),
			'Shift-Tab': () => moveBetweenCells(this.editor, -1)
		};
	}
});

const TableRow = Node.create({
	name: 'tableRow',
	content: '(tableCell | tableHeader)+',
	addAttributes: () => BLOCK_ID,
	parseHTML: () => [{ tag: 'tr' }],
	renderHTML: ({ HTMLAttributes }) => ['tr', HTMLAttributes, 0]
});

const TableCell = Node.create({
	name: 'tableCell',
	content: 'block+',
	isolating: true,
	addAttributes: () => BLOCK_ID,
	parseHTML: () => [{ tag: 'td' }],
	renderHTML: ({ HTMLAttributes }) => ['td', HTMLAttributes, 0]
});

const TableHeader = Node.create({
	name: 'tableHeader',
	content: 'block+',
	isolating: true,
	addAttributes: () => BLOCK_ID,
	parseHTML: () => [{ tag: 'th' }],
	renderHTML: ({ HTMLAttributes }) => ['th', HTMLAttributes, 0]
});

function bounded(value: string | null | undefined, maximum: number): string | null {
	const text = value?.trim() ?? '';
	return text && text.length <= maximum && !text.includes('\0') ? text : null;
}

function atomLabel(element: HTMLElement, mention: boolean): string | null {
	const text = (element.textContent ?? '').trim();
	return bounded(mention ? text.replace(/^@/u, '') : text, mention ? 200 : MAX_LINK_TITLE);
}

/* A copied mention or reference is parsed back from its own rendering, so both carry everything
 * they need in their markup; one that does not is dropped rather than stored half-formed. */
const Mention = Node.create({
	name: 'mention',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,
	addAttributes: () => ({
		actor_id: {
			default: null,
			parseHTML: (element: HTMLElement) => bounded(element.getAttribute('data-actor-id'), 128),
			renderHTML: (attributes: Record<string, unknown>) =>
				typeof attributes.actor_id === 'string' ? { 'data-actor-id': attributes.actor_id } : {}
		},
		label: {
			default: null,
			parseHTML: (element: HTMLElement) => atomLabel(element, true),
			renderHTML: () => ({})
		}
	}),
	parseHTML: () => [
		{
			tag: 'span[data-native-mention="true"]',
			getAttrs: (element: HTMLElement) =>
				bounded(element.getAttribute('data-actor-id'), 128) && atomLabel(element, true) ? {} : false
		}
	],
	renderHTML: ({ node, HTMLAttributes }) => [
		'span',
		mergeAttributes(HTMLAttributes, { 'data-native-mention': 'true', class: 'native-mention' }),
		`@${String(node.attrs.label ?? '')}`
	],
	renderText: ({ node }) => `@${String(node.attrs.label ?? '')}`
});

/** Where a reference opens in the cockpit, or null when it has no page of its own. */
export function documentReferenceHref(companyId: string, kind: string, id: string): string | null {
	if (!companyId || !id) return null;
	const root = `/${encodeURIComponent(companyId)}`;
	const key = encodeURIComponent(id);
	switch (kind) {
		case 'document':
			return `${root}/library/documents?document=${key}`;
		case 'artifact':
			return `${root}/library?file=${key}`;
		case 'work':
			return `${root}/work/${key}`;
		case 'attention':
			return `${root}?item=${key}`;
		case 'actor':
			return `${root}/people?person=${key}`;
		default:
			return null;
	}
}

function referenceExtension(companyId: string) {
	return Node.create({
		name: 'reference',
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,
		addAttributes: () => ({
			kind: {
				default: null,
				parseHTML: (element: HTMLElement) => element.getAttribute('data-reference-kind'),
				renderHTML: (attributes: Record<string, unknown>) =>
					typeof attributes.kind === 'string' ? { 'data-reference-kind': attributes.kind } : {}
			},
			id: {
				default: null,
				parseHTML: (element: HTMLElement) =>
					bounded(element.getAttribute('data-reference-id'), 200),
				renderHTML: (attributes: Record<string, unknown>) =>
					typeof attributes.id === 'string' ? { 'data-reference-id': attributes.id } : {}
			},
			label: {
				default: null,
				parseHTML: (element: HTMLElement) => atomLabel(element, false),
				renderHTML: () => ({})
			}
		}),
		parseHTML: () => [
			{
				tag: '[data-native-reference="true"]',
				getAttrs: (element: HTMLElement) =>
					REFERENCE_KINDS.includes(element.getAttribute('data-reference-kind') ?? '') &&
					bounded(element.getAttribute('data-reference-id'), 200) &&
					atomLabel(element, false)
						? {}
						: false
			}
		],
		/* A reference is a link to the thing it names: the Library file, the document or the Work. */
		renderHTML: ({ node, HTMLAttributes }) => {
			const label = String(node.attrs.label ?? '');
			const href = documentReferenceHref(
				companyId,
				String(node.attrs.kind ?? ''),
				String(node.attrs.id ?? '')
			);
			return [
				href ? 'a' : 'span',
				mergeAttributes(HTMLAttributes, {
					'data-native-reference': 'true',
					class: 'native-reference',
					...(href ? { href, title: `Open ${label}` } : {})
				}),
				label
			];
		},
		renderText: ({ node }) => String(node.attrs.label ?? '')
	});
}

/** A link the company schema keeps: http(s), mailto, a company path or an in-page anchor. */
export function safeHref(value: string): boolean {
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
	return (
		(value.startsWith('/') && !value.startsWith('//') && !value.includes('\\')) ||
		(value.startsWith('#') && value.length > 1)
	);
}

function storableHref(value: string | null | undefined): string | null {
	const href = value?.trim() ?? '';
	return href && href.length <= MAX_HREF && safeHref(href) ? href : null;
}

/** What someone typed or pasted into the link field, as a link the schema keeps, or null. */
export function linkFromInput(value: string): string | null {
	const text = value.trim();
	if (!text) return null;
	let href = text;
	if (/^https?:\/\//iu.test(text))
		href = text.replace(/^https?/iu, (scheme) => scheme.toLowerCase());
	else if (/^mailto:/iu.test(text)) href = `mailto:${text.slice('mailto:'.length)}`;
	else if (/^[^\s@/]+@[^\s@/]+\.[^\s@/]+$/u.test(text)) href = `mailto:${text}`;
	else if (!text.startsWith('/') && !text.startsWith('#')) href = `https://${text}`;
	return storableHref(href);
}

const WEB_ADDRESS = /^https?:\/\/\S+$/u;

function linkExtension(requestLink: (() => void) | undefined) {
	return Mark.create({
		name: 'link',
		priority: 1000,
		inclusive: false,
		keepOnSplit: false,
		exitable: true,
		addAttributes: () => ({
			href: {
				default: null,
				parseHTML: (element: HTMLElement) => storableHref(element.getAttribute('href'))
			},
			title: {
				default: null,
				parseHTML: (element: HTMLElement) => bounded(element.getAttribute('title'), MAX_LINK_TITLE)
			}
		}),
		/* A pasted anchor the schema would refuse (javascript:, tel:, a relative path, an empty "#")
		 * keeps its text and loses only the link. A reference renders as an anchor too; it is the
		 * reference node's to parse. */
		parseHTML: () => [
			{
				tag: 'a[href]:not([data-native-reference])',
				getAttrs: (element: HTMLElement) =>
					storableHref(element.getAttribute('href')) ? {} : false
			}
		],
		renderHTML: ({ HTMLAttributes }) => [
			'a',
			{
				href: HTMLAttributes.href,
				title: HTMLAttributes.title,
				rel: 'noopener noreferrer'
			},
			0
		],
		addKeyboardShortcuts: () => ({
			'Mod-k': () => {
				if (!requestLink) return false;
				requestLink();
				return true;
			}
		}),
		addInputRules() {
			const type = this.type;
			/* A web address followed by a space becomes a link, as it does in a mail client. */
			return [
				new InputRule({
					find: /(?:^|\s)(https?:\/\/\S+)(\s)$/u,
					handler: ({ state, range, match }) => {
						const typed = match[1] ?? '';
						const href = storableHref(typed.replace(/[.,;:!?'")\]]+$/u, ''));
						if (!href) return null;
						const from = range.from + match[0].indexOf(typed);
						const to = from + href.length;
						if (state.doc.rangeHasMark(from, to, type)) return null;
						state.tr
							.addMark(from, to, type.create({ href }))
							.insertText(match[2] ?? ' ', range.to)
							.removeStoredMark(type);
					}
				})
			];
		},
		addProseMirrorPlugins() {
			const type = this.type;
			return [
				new Plugin({
					key: new PluginKey('documentLinkPaste'),
					props: {
						/* Pasting a web address links the selected text, or inserts it as a link. */
						handlePaste: (view, event) => {
							const text = event.clipboardData?.getData('text/plain').trim() ?? '';
							const href = WEB_ADDRESS.test(text) ? storableHref(text) : null;
							const { state } = view;
							if (!href || state.selection.$from.parent.type.spec.code) return false;
							const mark = type.create({ href });
							const { from, to, empty } = state.selection;
							view.dispatch(
								empty
									? state.tr.replaceSelectionWith(state.schema.text(href, [mark]), false)
									: state.tr.addMark(from, to, mark)
							);
							return true;
						}
					}
				})
			];
		}
	});
}

/* An ordered list keeps a number to start at and nothing else. Tiptap also records a marker type
 * (from a pasted Word or Google Docs "a." list, or <ol type="i">), which the company schema has
 * no place for; and a list typed as "0. " starts at a number it refuses. */
const DocumentOrderedList = OrderedList.extend({
	addAttributes() {
		return {
			start: {
				default: 1,
				parseHTML: (element: HTMLElement) =>
					orderedStart(Number(element.getAttribute('start') ?? 1))
			}
		};
	},
	addProseMirrorPlugins() {
		const parent = this.parent?.() ?? [];
		const plugin = new Plugin({
			key: new PluginKey('documentOrderedListStart'),
			appendTransaction: (transactions, oldState, newState) => {
				if (!transactions.some((tr) => tr.docChanged && !isChangeOrigin(tr))) return null;
				const tr = newState.tr;
				const transform = combineTransactionSteps(oldState.doc, [...transactions]);
				for (const { newRange } of getChangedRanges(transform)) {
					for (const { node, pos } of findChildrenInRange(
						newState.doc,
						newRange,
						(child) => child.type.name === 'orderedList'
					)) {
						const start = orderedStart(node.attrs.start);
						if (start !== node.attrs.start)
							tr.setNodeMarkup(pos, undefined, { ...node.attrs, start });
					}
				}
				return tr.steps.length ? tr : null;
			}
		});
		return [...parent, plugin];
	}
});

function orderedStart(value: unknown): number {
	const start = Number(value);
	if (!Number.isInteger(start) || start < 1) return 1;
	return Math.min(start, MAX_ORDERED_START);
}

/** The same person always gets the same caret colour. */
export function caretHue(actorId: string): number {
	let hash = 0;
	for (const character of actorId) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
	return hash % 360;
}

/** Other people's carets. The server stamps each with the actor it verified;
 * the name comes from Core's people, never from the other browser. */
export interface DocumentCarets {
	actorId: string;
	nameOf: (actorId: string) => string;
}

/** What the editor asks of the page around it. */
export interface DocumentEditorHooks {
	/** The company whose pages a reference opens. */
	companyId?: string;
	/** Mod-K: the page opens its link field. */
	requestLink?: () => void;
}

function caretExtension(provider: HocuspocusProvider, carets: DocumentCarets) {
	return CollaborationCaret.configure({
		provider,
		user: { actor_id: carets.actorId },
		render: (user) => {
			const actorId = typeof user.actor_id === 'string' ? user.actor_id : '';
			const hue = caretHue(actorId);
			const caret = window.document.createElement('span');
			caret.classList.add('collaboration-carets__caret');
			caret.style.borderColor = `hsl(${hue} 65% 45%)`;
			const label = window.document.createElement('div');
			label.classList.add('collaboration-carets__label');
			label.style.backgroundColor = `hsl(${hue} 65% 45%)`;
			label.textContent = carets.nameOf(actorId);
			caret.append(label);
			return caret;
		},
		selectionRender: (user) => ({
			class: 'collaboration-carets__selection',
			style: `background-color: hsl(${caretHue(typeof user.actor_id === 'string' ? user.actor_id : '')} 65% 45% / 0.18)`
		})
	});
}

export function createDocumentEditorExtensions(
	document: Doc,
	provider: HocuspocusProvider,
	updateBlockIds: boolean,
	carets?: DocumentCarets,
	hooks: DocumentEditorHooks = {}
): Extensions {
	return [
		StarterKit.configure({
			link: false,
			underline: false,
			undoRedo: false,
			orderedList: false,
			heading: { levels: [1, 2, 3, 4, 5, 6] }
		}),
		DocumentOrderedList,
		TaskList,
		TaskItem.configure({ nested: true }),
		Table,
		TableRow,
		TableCell,
		TableHeader,
		Mention,
		referenceExtension(hooks.companyId ?? ''),
		linkExtension(hooks.requestLink),
		UniqueID.configure({
			attributeName: 'block_id',
			types: [...BLOCK_TYPES],
			filterTransaction: (transaction) => !isChangeOrigin(transaction),
			updateDocument: updateBlockIds
		}),
		Collaboration.configure({
			document,
			field: DOCUMENT_COLLABORATION_FRAGMENT,
			provider
		}),
		...(carets ? [caretExtension(provider, carets)] : [])
	];
}

function invalid(message: string): never {
	throw new Error(`The live document cannot become a named version: ${message}`);
}

function object(value: unknown, label: string): Record<string, unknown> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) invalid(`${label} is invalid.`);
	return value as Record<string, unknown>;
}

function string(value: unknown, label: string, maximum: number): string {
	if (
		typeof value !== 'string' ||
		value.length < 1 ||
		value.length > maximum ||
		value.includes('\0')
	) {
		invalid(`${label} is invalid.`);
	}
	return value;
}

function projectMark(value: unknown): DocumentMark {
	const source = object(value, 'text mark');
	const type = string(source.type, 'mark type', 32);
	if (!['bold', 'italic', 'strike', 'code', 'link'].includes(type)) {
		invalid(`mark ${type} is outside the company schema.`);
	}
	if (type !== 'link') return { type };
	const attrs = object(source.attrs, 'link attributes');
	const href = string(attrs.href, 'link href', MAX_HREF);
	if (!safeHref(href)) invalid('link href is unsafe.');
	const projected: Record<string, unknown> = { href };
	if (attrs.title !== null && attrs.title !== undefined) {
		projected.title = string(attrs.title, 'link title', MAX_LINK_TITLE);
	}
	return { type, attrs: projected };
}

interface ProjectionState {
	blockIds: Set<string>;
}

function projectedAttributes(
	type: string,
	value: unknown,
	topLevel: boolean,
	state: ProjectionState
): Record<string, unknown> {
	const source = value === undefined ? {} : object(value, `${type} attributes`);
	const attrs: Record<string, unknown> = {};
	if (BLOCK_TYPES.includes(type as (typeof BLOCK_TYPES)[number])) {
		if (source.block_id !== null && source.block_id !== undefined) {
			const blockId = string(source.block_id, 'block identity', 128);
			if (!/^[A-Za-z0-9_.:-]+$/u.test(blockId)) invalid('block identity is invalid.');
			if (state.blockIds.has(blockId)) invalid('block identity is duplicated.');
			state.blockIds.add(blockId);
			attrs.block_id = blockId;
		}
		if (topLevel && !attrs.block_id) invalid('a top-level block has no stable identity.');
	}
	switch (type) {
		case 'heading': {
			const level = source.level;
			if (!Number.isInteger(level) || Number(level) < 1 || Number(level) > 6) {
				invalid('heading level is invalid.');
			}
			attrs.level = level;
			break;
		}
		case 'orderedList': {
			const start = source.start;
			if (start !== null && start !== undefined) {
				if (!Number.isInteger(start) || Number(start) < 1 || Number(start) > MAX_ORDERED_START) {
					invalid('ordered-list start is invalid.');
				}
				attrs.start = start;
			}
			break;
		}
		case 'taskItem':
			if (typeof source.checked !== 'boolean') invalid('task item state is invalid.');
			attrs.checked = source.checked;
			break;
		case 'codeBlock':
			if (source.language !== null && source.language !== undefined) {
				attrs.language = string(source.language, 'code-block language', 64);
			}
			break;
		case 'mention':
			attrs.actor_id = string(source.actor_id, 'mention actor', 128);
			attrs.label = string(source.label, 'mention label', 200);
			break;
		case 'reference': {
			const kind = string(source.kind, 'reference kind', 32);
			if (!REFERENCE_KINDS.includes(kind)) {
				invalid('reference kind is invalid.');
			}
			attrs.kind = kind;
			attrs.id = string(source.id, 'reference identity', 200);
			attrs.label = string(source.label, 'reference label', MAX_LINK_TITLE);
			break;
		}
	}
	return attrs;
}

const NODE_TYPES = new Set(['doc', ...BLOCK_TYPES, 'text', 'hardBreak', 'mention', 'reference']);

function projectNode(value: unknown, topLevel: boolean, state: ProjectionState): DocumentNode {
	const source = object(value, 'document node');
	const type = string(source.type, 'node type', 64);
	if (!NODE_TYPES.has(type)) invalid(`node ${type} is outside the company schema.`);
	if (type === 'text') {
		if (typeof source.text !== 'string' || source.text.includes('\0')) invalid('text is invalid.');
		const node: DocumentNode = { type, text: source.text };
		if (Array.isArray(source.marks) && source.marks.length) {
			node.marks = source.marks.map(projectMark);
		}
		return node;
	}
	const node: DocumentNode = { type };
	const attrs = projectedAttributes(type, source.attrs, topLevel, state);
	if (Object.keys(attrs).length) node.attrs = attrs;
	if (Array.isArray(source.content)) {
		node.content = source.content.map((child) => projectNode(child, type === 'doc', state));
	}
	return node;
}

/** Convert Tiptap's richer runtime attributes into the exact Core checkpoint schema. */
export function projectDocumentContent(value: JSONContent): DocumentContent {
	const projected = projectNode(value, false, { blockIds: new Set() });
	if (projected.type !== 'doc' || !Array.isArray(projected.content)) {
		invalid('the root is not a document.');
	}
	return projected as DocumentContent;
}

export function selectedDocumentBlockId(editor: Editor): string | null {
	const { $from } = editor.state.selection;
	for (let depth = Math.min(1, $from.depth); depth >= 1; depth -= 1) {
		const value = $from.node(depth).attrs.block_id;
		if (typeof value === 'string' && value) return value;
	}
	return null;
}
