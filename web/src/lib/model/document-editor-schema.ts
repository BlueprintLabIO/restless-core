import {
	Extension,
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
import { DragHandlePlugin, normalizeNestedOptions } from '@tiptap/extension-drag-handle';
import { OrderedList } from '@tiptap/extension-list';
import TaskItem from '@tiptap/extension-task-item';
import TaskList from '@tiptap/extension-task-list';
import UniqueID from '@tiptap/extension-unique-id';
import StarterKit from '@tiptap/starter-kit';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import {
	NodeSelection,
	Plugin,
	PluginKey,
	TextSelection,
	type EditorState
} from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import type { HocuspocusProvider } from '@hocuspocus/provider';
import type { Doc } from 'yjs';
import type { DocumentContent, DocumentMark, DocumentNode } from './documents';
import { caretHue } from './page-faces.ts';

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
	'tableHeader',
	'image',
	'file'
] as const;

/* The documents sidecar stores only what the company schema allows and refuses the whole body
 * otherwise, so anything the browser may parse (a paste, an input rule) is held to the same bounds
 * here. These mirror services/native-documents-collaboration/src/document-codec.ts. */
const REFERENCE_KINDS = ['work', 'decision', 'attention', 'document', 'artifact', 'actor'];
const MAX_HREF = 2_048;
const MAX_LINK_TITLE = 300;
const MAX_ORDERED_START = 100_000;

/** The few colours text may be written or highlighted in. A colour is stored by name, so the page
 * draws it to suit a light or dark theme and an export picks its own; anything else is dropped. */
export const DOCUMENT_TEXT_COLORS = ['gray', 'red', 'orange', 'green', 'blue', 'purple'] as const;
export const DOCUMENT_HIGHLIGHTS = ['yellow', 'green', 'blue', 'pink', 'purple'] as const;
/** How a paragraph or heading lines up; left, the default, is no attribute at all. */
export const DOCUMENT_ALIGNMENTS = ['center', 'right', 'justify'] as const;
export type DocumentAlignment = (typeof DOCUMENT_ALIGNMENTS)[number];

/* The colours a pasted span is matched against: a Word or Google Docs colour becomes the nearest
 * named one, and black, white or grey-ish defaults become none. */
const TEXT_COLOR_RGB: Record<(typeof DOCUMENT_TEXT_COLORS)[number], [number, number, number]> = {
	gray: [120, 120, 120],
	red: [220, 38, 38],
	orange: [234, 120, 20],
	green: [22, 150, 70],
	blue: [37, 99, 235],
	purple: [140, 60, 220]
};
const HIGHLIGHT_RGB: Record<(typeof DOCUMENT_HIGHLIGHTS)[number], [number, number, number]> = {
	yellow: [255, 240, 0],
	green: [120, 230, 120],
	blue: [120, 200, 255],
	pink: [255, 150, 200],
	purple: [200, 160, 255]
};

function cssRgb(value: string): [number, number, number] | null {
	const text = value.trim().toLowerCase();
	const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/u.exec(text)?.[1];
	if (hex) {
		const full = hex.length === 3 ? [...hex].map((digit) => digit + digit).join('') : hex;
		return [0, 2, 4].map((at) => parseInt(full.slice(at, at + 2), 16)) as [number, number, number];
	}
	const rgb = /^rgba?\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)(?:[\s,/]+([\d.]+%?))?\s*\)$/u.exec(text);
	if (!rgb) return null;
	if (rgb[4] !== undefined && parseFloat(rgb[4]) === 0) return null;
	return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
}

function nearest<T extends string>(
	rgb: [number, number, number],
	palette: Record<T, [number, number, number]>
): T {
	let best = '' as T;
	let distance = Infinity;
	for (const [name, [r, g, b]] of Object.entries(palette) as [T, [number, number, number]][]) {
		const d = (rgb[0] - r) ** 2 + (rgb[1] - g) ** 2 + (rgb[2] - b) ** 2;
		if (d < distance) [best, distance] = [name, d];
	}
	return best;
}

/** A pasted CSS text colour as a named one, or null for the ordinary black (or white) of a page. */
export function documentTextColorFromCss(value: string | null | undefined): string | null {
	const rgb = value ? cssRgb(value) : null;
	if (!rgb) return null;
	const spread = Math.max(...rgb) - Math.min(...rgb);
	const light = (rgb[0] + rgb[1] + rgb[2]) / 3;
	if (spread < 40 && (light < 70 || light > 200)) return null;
	return spread < 40 ? 'gray' : nearest(rgb, TEXT_COLOR_RGB);
}

/** A pasted CSS background as a named highlight, or null for white, black and transparent. */
export function documentHighlightFromCss(value: string | null | undefined): string | null {
	const rgb = value ? cssRgb(value) : null;
	if (!rgb) return null;
	if (Math.max(...rgb) - Math.min(...rgb) < 40) return null;
	return nearest(rgb, HIGHLIGHT_RGB);
}

function oneOf(value: unknown, allowed: readonly string[]): string | null {
	return typeof value === 'string' && allowed.includes(value) ? value : null;
}

/** A paragraph's or heading's alignment, from its own markup or the style a paste carries. */
const ALIGN = {
	align: {
		default: null,
		parseHTML: (element: HTMLElement) =>
			oneOf(
				element.getAttribute('data-align') ??
					(element.style?.textAlign === 'both' ? 'justify' : element.style?.textAlign),
				DOCUMENT_ALIGNMENTS
			),
		renderHTML: (attributes: Record<string, unknown>) =>
			oneOf(attributes.align, DOCUMENT_ALIGNMENTS) ? { 'data-align': attributes.align } : {}
	}
};

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

/** What a cockpit path names, when it is the page of something a reference can point at. */
export function referenceFromPath(
	companyId: string,
	path: string
): { kind: string; id: string } | null {
	let url: URL;
	try {
		url = new URL(path, 'https://cockpit.invalid');
	} catch {
		return null;
	}
	const root = `/${encodeURIComponent(companyId)}`;
	const rest = url.pathname.startsWith(`${root}/`) ? url.pathname.slice(root.length) : null;
	const param = (name: string) => url.searchParams.get(name);
	if (url.pathname === root || url.pathname === `${root}/`)
		return param('item') ? { kind: 'attention', id: param('item')! } : null;
	if (rest === '/library/documents' && param('document'))
		return { kind: 'document', id: param('document')! };
	if (rest === '/library' && param('file')) return { kind: 'artifact', id: param('file')! };
	if (rest === '/people' && param('person')) return { kind: 'actor', id: param('person')! };
	const work = rest ? /^\/work\/([^/]+)$/u.exec(rest) : null;
	if (work) return { kind: 'work', id: decodeURIComponent(work[1]) };
	return null;
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

function linkExtension(hooks: DocumentEditorHooks) {
	const requestLink = hooks.requestLink;
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
		/* A live peer's edit reaches this page before the store checks it: an unsafe address is
		 * never put in the page, so no click, middle-click or "open in new tab" can follow it. */
		renderHTML: ({ HTMLAttributes }) => [
			'a',
			{
				href: storableHref(typeof HTMLAttributes.href === 'string' ? HTMLAttributes.href : null),
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
							const { from, to, empty } = state.selection;
							/* A link to a page in this cockpit becomes a chip with that page's title. */
							const reference = empty ? hooks.resolveReference?.(href) : null;
							if (reference) {
								view.dispatch(
									state.tr.replaceSelectionWith(
										state.schema.nodes.reference.create(reference),
										false
									)
								);
								return true;
							}
							const mark = type.create({ href });
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

function colourMark(name: 'highlight' | 'textColor') {
	const palette = name === 'highlight' ? DOCUMENT_HIGHLIGHTS : DOCUMENT_TEXT_COLORS;
	const attribute = name === 'highlight' ? 'data-color' : 'data-text-color';
	const fromCss = name === 'highlight' ? documentHighlightFromCss : documentTextColorFromCss;
	return Mark.create({
		name,
		addAttributes: () => ({
			color: {
				default: null,
				parseHTML: (element: HTMLElement) =>
					oneOf(element.getAttribute(attribute), palette) ??
					fromCss(name === 'highlight' ? element.style?.backgroundColor : element.style?.color) ??
					(name === 'highlight' && element.tagName === 'MARK' ? 'yellow' : null),
				renderHTML: (attributes: Record<string, unknown>) =>
					oneOf(attributes.color, palette) ? { [attribute]: attributes.color } : {}
			}
		}),
		/* Its own markup, a plain <mark>, or a span a word processor coloured; a span in the page's
		 * ordinary colours is not a colour at all. */
		parseHTML: () => [
			{
				tag: name === 'highlight' ? 'mark' : `span[${attribute}]`,
				getAttrs: (element: HTMLElement) =>
					oneOf(element.getAttribute(attribute), palette) || name === 'highlight' ? null : false
			},
			{
				tag: 'span[style]',
				getAttrs: (element: HTMLElement) =>
					fromCss(name === 'highlight' ? element.style?.backgroundColor : element.style?.color)
						? null
						: false
			}
		],
		renderHTML: ({ HTMLAttributes }) => [name === 'highlight' ? 'mark' : 'span', HTMLAttributes, 0]
	});
}

/* A paragraph or heading may be centred, right-aligned or justified (⌘⇧E, ⌘⇧R, ⌘⇧J; ⌘⇧L back to
 * the left), as in every word processor. */
export function alignDocumentBlocks(editor: Editor, align: DocumentAlignment | null): boolean {
	return editor
		.chain()
		.focus()
		.updateAttributes('paragraph', { align })
		.updateAttributes('heading', { align })
		.run();
}

export function documentAlignment(editor: Editor): DocumentAlignment | null {
	const { $from } = editor.state.selection;
	return oneOf($from.parent.attrs.align, DOCUMENT_ALIGNMENTS) as DocumentAlignment | null;
}

const alignmentExtension = Extension.create({
	name: 'documentAlignment',
	addGlobalAttributes: () => [{ types: ['paragraph', 'heading'], attributes: ALIGN }],
	addKeyboardShortcuts() {
		return {
			'Mod-Shift-l': () => alignDocumentBlocks(this.editor, null),
			'Mod-Shift-e': () => alignDocumentBlocks(this.editor, 'center'),
			'Mod-Shift-r': () => alignDocumentBlocks(this.editor, 'right'),
			'Mod-Shift-j': () => alignDocumentBlocks(this.editor, 'justify')
		};
	}
});

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
	/** A web address, when it is a page in this cockpit with a known title, as a reference to it. */
	resolveReference?: (href: string) => { kind: string; id: string; label: string } | null;
	/** Files pasted or dropped at a position: the page uploads them and places them there, a
	 * picture as an image and anything else as a file. */
	uploadImages?: (files: File[], at: number) => void;
	/** After a paste: pictures found in it to upload and place at a position, and what was lost. */
	pasted?: (images: string[], lost: string[], at: number) => void;
	/** The "/" menu opened, changed or closed. */
	slashChange?: (state: SlashMenuState | null) => void;
	/** A key pressed while the "/" menu is open; true when the menu used it. */
	slashKey?: (event: KeyboardEvent) => boolean;
	/** The "@" menu opened, changed or closed. */
	mentionChange?: (state: SlashMenuState | null) => void;
	/** A key pressed while the "@" menu is open; true when the menu used it. */
	mentionKey?: (event: KeyboardEvent) => boolean;
	/** Open the actions of the top-level block at a position, beside an anchor. */
	blockMenu?: (target: { pos: number; anchor: DOMRect; fromKeyboard?: boolean }) => void;
	/** The top-level blocks that have an open comment thread, marked in the text. */
	commentedBlocks?: () => ReadonlySet<string>;
}

/* Blocks with an open comment thread wear a quiet highlight, as commented text does in Notion.
 * The page says which blocks; it dispatches this key when the threads change. */
export const commentMarksKey = new PluginKey<DecorationSet>('documentCommentMarks');

function commentMarks(doc: ProseMirrorNode, blocks: ReadonlySet<string> | undefined) {
	if (!blocks?.size) return DecorationSet.empty;
	const marks: Decoration[] = [];
	doc.forEach((node, offset) => {
		if (blocks.has(node.attrs.block_id))
			marks.push(Decoration.node(offset, offset + node.nodeSize, { class: 'has-comments' }));
	});
	return DecorationSet.create(doc, marks);
}

function commentMarksExtension(hooks: DocumentEditorHooks) {
	return Extension.create({
		name: 'documentCommentMarks',
		addProseMirrorPlugins() {
			return [
				new Plugin<DecorationSet>({
					key: commentMarksKey,
					state: {
						init: (_, state) => commentMarks(state.doc, hooks.commentedBlocks?.()),
						apply: (tr, marks, _old, state) =>
							tr.getMeta(commentMarksKey) || tr.docChanged
								? commentMarks(state.doc, hooks.commentedBlocks?.())
								: marks
					},
					props: { decorations: (state) => commentMarksKey.getState(state) }
				})
			];
		}
	});
}

/** A heading as the outline lists it: where it is, how deep, its words and its anchor. */
export interface OutlineEntry {
	pos: number;
	level: number;
	text: string;
	blockId: string | null;
}

/** The words a heading is linked by in the page: "#pricing-and-terms". */
export function headingSlug(text: string): string {
	return text
		.toLocaleLowerCase()
		.normalize('NFKD')
		.replace(/[\u0300-\u036f]/gu, '')
		.replace(/[^\p{L}\p{N}]+/gu, '-')
		.replace(/^-+|-+$/gu, '')
		.slice(0, 80);
}

export function documentOutline(doc: ProseMirrorNode): OutlineEntry[] {
	const entries: OutlineEntry[] = [];
	doc.descendants((node, pos) => {
		if (node.type.name === 'heading') {
			const text = node.textContent.trim();
			if (text)
				entries.push({
					pos,
					level: Number(node.attrs.level) || 1,
					text,
					blockId: typeof node.attrs.block_id === 'string' ? node.attrs.block_id : null
				});
			return false;
		}
		return !node.isTextblock;
	});
	return entries;
}

/** Where an in-page "#…" link lands: the heading or block with that id, or the heading whose
 * words make that anchor. */
export function documentAnchorTarget(doc: ProseMirrorNode, href: string): number | null {
	let fragment = href.replace(/^#/u, '');
	try {
		fragment = decodeURIComponent(fragment);
	} catch {
		/* Kept as written. */
	}
	if (!fragment) return null;
	let found: number | null = null;
	doc.descendants((node, pos) => {
		if (found !== null) return false;
		if (node.attrs.block_id === fragment) found = pos;
		return found === null;
	});
	if (found !== null) return found;
	const slug = headingSlug(fragment);
	return documentOutline(doc).find((entry) => headingSlug(entry.text) === slug)?.pos ?? null;
}

/** Words and characters in the document, or in the part of it between two positions. */
export function documentCounts(
	doc: ProseMirrorNode,
	from = 0,
	to = doc.content.size
): { words: number; characters: number } {
	const text = doc.textBetween(from, to, '\n', ' ');
	return {
		words: (text.match(/[\p{L}\p{N}][\p{L}\p{N}'’.-]*/gu) ?? []).length,
		characters: [...text.replace(/\n/gu, '')].length
	};
}

/** One occurrence of what Find looks for: a range of document positions. */
export interface FindMatch {
	from: number;
	to: number;
}

/** Every place the words occur, ignoring case, within one paragraph, heading, cell or line of code
 * at a time. A mention or reference is one character, so it is never matched by its label. */
export function findMatches(doc: ProseMirrorNode, query: string, limit = 2_000): FindMatch[] {
	const needle = query.toLocaleLowerCase();
	if (!needle) return [];
	const matches: FindMatch[] = [];
	doc.descendants((node, pos) => {
		if (matches.length >= limit) return false;
		if (!node.isTextblock) return true;
		let text = '';
		const at: number[] = [];
		node.forEach((child, offset) => {
			const piece = child.isText ? (child.text ?? '') : '\uFFFC';
			for (let index = 0; index < piece.length; index += 1)
				at.push(pos + 1 + offset + (child.isText ? index : 0));
			text += piece;
		});
		const lower = text.toLocaleLowerCase();
		/* Lower-casing can change a string's length in rare scripts; then match it as written. */
		const haystack = lower.length === text.length ? lower : text;
		for (
			let index = haystack.indexOf(needle);
			index >= 0;
			index = haystack.indexOf(needle, index + needle.length)
		) {
			matches.push({ from: at[index], to: at[index + needle.length - 1] + 1 });
			if (matches.length >= limit) break;
		}
		return false;
	});
	return matches;
}

export interface FindState {
	query: string;
	matches: FindMatch[];
	current: number;
}

export const findKey = new PluginKey<FindState>('documentFind');

function findDecorations(doc: ProseMirrorNode, state: FindState): DecorationSet {
	if (!state.matches.length) return DecorationSet.empty;
	return DecorationSet.create(
		doc,
		state.matches.map((match, index) =>
			Decoration.inline(match.from, match.to, {
				class: index === state.current ? 'find-match current' : 'find-match'
			})
		)
	);
}

/* Find and replace: the page's find bar says what to look for; matches are marked in the text and
 * a replacement is an ordinary edit, so it reaches everyone editing as any other change does. */
const findExtension = Extension.create({
	name: 'documentFind',
	addProseMirrorPlugins() {
		return [
			new Plugin<FindState>({
				key: findKey,
				state: {
					init: () => ({ query: '', matches: [], current: 0 }),
					apply: (tr, value, _old, state) => {
						const meta = tr.getMeta(findKey) as Partial<FindState> | undefined;
						if (!meta && !tr.docChanged) return value;
						const query = meta?.query ?? value.query;
						const matches =
							meta?.query !== undefined || tr.docChanged
								? findMatches(state.doc, query)
								: value.matches;
						const wanted = meta?.current ?? value.current;
						return {
							query,
							matches,
							current: matches.length ? Math.min(Math.max(0, wanted), matches.length - 1) : 0
						};
					}
				},
				props: {
					decorations: (state) => {
						const value = findKey.getState(state);
						return value ? findDecorations(state.doc, value) : null;
					}
				}
			})
		];
	}
});

/** Look for these words, starting at the match nearest the caret. */
export function setDocumentFind(editor: Editor, query: string): void {
	const { view } = editor;
	const matches = findMatches(view.state.doc, query);
	const from = view.state.selection.from;
	const current = Math.max(
		0,
		matches.findIndex((match) => match.to >= from)
	);
	view.dispatch(view.state.tr.setMeta(findKey, { query, current }));
	revealFind(editor);
}

/** Go to the next or previous match, round from the end to the start. */
export function stepDocumentFind(editor: Editor, step: 1 | -1): void {
	const value = findKey.getState(editor.state);
	if (!value?.matches.length) return;
	const current = (value.current + step + value.matches.length) % value.matches.length;
	editor.view.dispatch(editor.state.tr.setMeta(findKey, { current }));
	revealFind(editor);
}

function revealFind(editor: Editor): void {
	const value = findKey.getState(editor.state);
	const match = value?.matches[value.current];
	if (!match) return;
	const tr = editor.state.tr.setSelection(
		TextSelection.create(editor.state.doc, match.from, match.to)
	);
	editor.view.dispatch(tr.scrollIntoView());
}

/** Replace the current match, or every match, with the replacement. */
export function replaceDocumentFind(editor: Editor, replacement: string, all: boolean): number {
	const value = findKey.getState(editor.state);
	if (!value?.matches.length || !editor.isEditable) return 0;
	const targets = all ? value.matches : [value.matches[value.current]];
	const tr = editor.state.tr;
	for (const match of [...targets].reverse()) {
		if (replacement) tr.insertText(replacement, match.from, match.to);
		else tr.delete(match.from, match.to);
	}
	editor.view.dispatch(tr.setMeta(findKey, { current: value.current }));
	if (!all) revealFind(editor);
	return targets.length;
}

/* The line being written, while it is empty, says how to make anything else: the toolbar is gone,
 * so "/" is the way in. */
function emptyLineHintExtension() {
	return Extension.create({
		name: 'documentEmptyLineHint',
		addProseMirrorPlugins() {
			return [
				new Plugin({
					key: new PluginKey('documentEmptyLineHint'),
					props: {
						decorations: (state) => {
							const { $from, empty } = state.selection;
							const parent = $from.parent;
							if (
								!this.editor.isEditable ||
								!empty ||
								parent.type.name !== 'paragraph' ||
								parent.content.size ||
								$from.depth !== 1
							)
								return null;
							const at = $from.before(1);
							return DecorationSet.create(state.doc, [
								Decoration.node(at, at + parent.nodeSize, {
									class: 'empty-line',
									'data-hint': 'Type / for blocks'
								})
							]);
						}
					}
				})
			];
		}
	});
}

/* A split block keeps its id on the part that is still the same block: the part with content, or
 * the first when both or neither have any; the other part gets a new id. Tiptap's UniqueID gives
 * the id to the second part whenever the first is empty, so splitting an empty line handed that
 * line's Yjs identity to the new one, and every colleague's caret on it followed into the new
 * line (S67). UniqueID still declares the attribute and fills ids when a document opens; this
 * plugin replaces its transaction handling, and gives any block without an id one. */
export function blockIdentityPlugin(): Plugin {
	return new Plugin({
		key: new PluginKey('blockIdentity'),
		appendTransaction: (transactions, oldState, newState) => {
			if (!transactions.some((tr) => tr.docChanged && !isChangeOrigin(tr))) return null;
			const transform = combineTransactionSteps(oldState.doc, [...transactions]);
			const byId = new Map<string, { node: ProseMirrorNode; pos: number }[]>();
			const seen = new Set<number>();
			for (const { newRange } of getChangedRanges(transform)) {
				for (const found of findChildrenInRange(newState.doc, newRange, (node) =>
					(BLOCK_TYPES as readonly string[]).includes(node.type.name)
				)) {
					if (seen.has(found.pos)) continue;
					seen.add(found.pos);
					const id = found.node.attrs.block_id;
					const key = typeof id === 'string' && id ? id : `missing:${found.pos}`;
					byId.set(key, [...(byId.get(key) ?? []), found]);
				}
			}
			const tr = newState.tr;
			for (const [key, parts] of byId) {
				if (key.startsWith('missing:')) {
					tr.setNodeMarkup(parts[0].pos, undefined, {
						...parts[0].node.attrs,
						block_id: crypto.randomUUID()
					});
					continue;
				}
				if (parts.length < 2) continue;
				const filled = parts.filter((part) => part.node.content.size > 0);
				const keeper = filled.length === 1 ? filled[0] : parts[0];
				for (const part of parts)
					if (part !== keeper)
						tr.setNodeMarkup(part.pos, undefined, {
							...part.node.attrs,
							block_id: crypto.randomUUID()
						});
			}
			return tr.steps.length ? tr : null;
		}
	});
}

const blockIdentityExtension = Extension.create({
	name: 'blockIdentity',
	addProseMirrorPlugins: () => [blockIdentityPlugin()]
});

function caretExtension(provider: HocuspocusProvider, carets: DocumentCarets) {
	return CollaborationCaret.configure({
		provider,
		user: { actor_id: carets.actorId },
		render: (user) => {
			const actorId = typeof user.actor_id === 'string' ? user.actor_id : '';
			const hue = caretHue(actorId);
			const caret = window.document.createElement('span');
			caret.classList.add('collaboration-carets__caret');
			caret.dataset.actorId = actorId;
			caret.style.borderColor = `hsl(${hue} 65% 45%)`;
			const label = window.document.createElement('div');
			label.classList.add('collaboration-carets__label');
			label.style.backgroundColor = `hsl(${hue} 65% 45%)`;
			label.textContent = carets.nameOf(actorId);
			/* Never a place to type: the browser's own caret must not step into someone else's.
			 * The word joiners either side keep it moving past, as Tiptap's own caret does. */
			caret.contentEditable = 'false';
			caret.append('⁠', label, '⁠');
			return caret;
		},
		selectionRender: (user) => ({
			class: 'collaboration-carets__selection',
			style: `background-color: hsl(${caretHue(typeof user.actor_id === 'string' ? user.actor_id : '')} 65% 45% / 0.18)`
		})
	});
}

/** Where a company file shows in place: the attachment route, which serves a raster image inline
 * only when its bytes really are one. */
export function documentImageSrc(companyId: string, attachmentId: string): string {
	return `/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachmentId)}?inline=1`;
}

const ATTACHMENT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;
const MAX_IMAGE_ALT = 300;
const MAX_FILE_NAME = 255;
/** The raster types the attachment route shows in place. */
export const DOCUMENT_IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];

function attachmentIdFrom(value: string | null | undefined): string | null {
	return value && ATTACHMENT_ID.test(value) ? value : null;
}

/** A file's name as the words an image is described by: "Q3 chart.png" reads "Q3 chart". */
export function imageAltFromName(name: string): string {
	const alt = name
		.replace(/\.[a-z0-9]{1,5}$/iu, '')
		.replace(/[\p{Cc}]/gu, '')
		.trim();
	return [...alt].slice(0, MAX_IMAGE_ALT).join('');
}

/** Where a file a document carries opens: its preview in the Library. */
export function documentFileHref(companyId: string, attachmentId: string): string {
	return `/${encodeURIComponent(companyId)}/library?upload=${encodeURIComponent(attachmentId)}`;
}

/** A named version's rendered page shows its images from the company that holds them, and links
 * its files to their Library preview. */
export function withDocumentImages(html: string, companyId: string): string {
	return html
		.replace(/<img data-attachment-id="([0-9a-f-]{36})"/gu, (whole, id: string) =>
			ATTACHMENT_ID.test(id)
				? `<img src="${documentImageSrc(companyId, id)}" loading="lazy" data-attachment-id="${id}"`
				: whole
		)
		.replace(
			/(<div[^>]* data-native-file="true" data-attachment-id="([0-9a-f-]{36})">)([^<]*)<\/div>/gu,
			(whole, open: string, id: string, name: string) =>
				ATTACHMENT_ID.test(id)
					? `${open}<a href="${documentFileHref(companyId, id)}">${name}</a></div>`
					: whole
		);
}

/* A file is a block that names a company file other than a picture (a contract, a quote, a sheet)
 * and opens it in the Library's preview. */
function fileExtension(hooks: DocumentEditorHooks) {
	return Node.create({
		name: 'file',
		group: 'block',
		atom: true,
		draggable: true,
		selectable: true,
		addAttributes: () => ({
			...BLOCK_ID,
			attachment_id: {
				default: null,
				parseHTML: (element: HTMLElement) =>
					attachmentIdFrom(element.getAttribute('data-attachment-id')),
				renderHTML: (attributes: Record<string, unknown>) =>
					typeof attributes.attachment_id === 'string'
						? { 'data-attachment-id': attributes.attachment_id }
						: {}
			},
			name: {
				default: null,
				parseHTML: (element: HTMLElement) => bounded(element.textContent, MAX_FILE_NAME),
				renderHTML: () => ({})
			}
		}),
		parseHTML: () => [
			{
				tag: 'div[data-native-file="true"]',
				getAttrs: (element: HTMLElement) =>
					attachmentIdFrom(element.getAttribute('data-attachment-id')) &&
					bounded(element.textContent, MAX_FILE_NAME)
						? {}
						: false
			}
		],
		renderHTML: ({ node, HTMLAttributes }) => {
			const id = attachmentIdFrom(node.attrs.attachment_id);
			const name = String(node.attrs.name ?? '');
			return [
				'div',
				mergeAttributes(HTMLAttributes, { 'data-native-file': 'true', class: 'native-file' }),
				id && hooks.companyId
					? ['a', { href: documentFileHref(hooks.companyId, id), title: `Open ${name}` }, name]
					: ['span', {}, name]
			];
		}
	});
}

/* Any file can go in a document: a picture as an image, anything else as a file block. */
function imageFiles(list: FileList | null | undefined): File[] {
	return [...(list ?? [])];
}

/** What a paste from Word, Google Docs or a web page brings that the schema cannot hold as is. */
export interface PreparedPaste {
	html: string;
	/** Pictures inside the pasted content (data: or web addresses), in order, to upload. */
	images: string[];
	/** Plain words for anything that could not be kept. */
	lost: string[];
}

/* Pasted HTML is made to fit the company schema before it is parsed: merged table cells are
 * split into a plain grid (the text stays in the first cell), pictures are taken out to be
 * uploaded as company files, and what cannot be kept is named rather than silently dropped. */
export function preparePastedHtml(html: string, parser: DOMParser): PreparedPaste {
	const document = parser.parseFromString(html, 'text/html');
	const lost = new Set<string>();
	const images: string[] = [];
	let merged = false;
	for (const table of document.querySelectorAll('table')) {
		const rows = [...table.querySelectorAll(':scope > tr, :scope > * > tr')];
		const carried: number[][] = rows.map(() => []);
		rows.forEach((row, rowIndex) => {
			let column = 0;
			for (const cell of [...row.children]) {
				if (!/^T[DH]$/u.test(cell.tagName)) continue;
				while (carried[rowIndex].includes(column)) column += 1;
				const span = Math.min(20, Math.max(1, Number(cell.getAttribute('colspan')) || 1));
				const down = Math.min(50, Math.max(1, Number(cell.getAttribute('rowspan')) || 1));
				cell.removeAttribute('colspan');
				cell.removeAttribute('rowspan');
				if (span > 1 || down > 1) merged = true;
				for (let extra = 1; extra < span; extra += 1)
					cell.after(document.createElement(cell.tagName.toLowerCase()));
				for (let below = 1; below < down && rowIndex + below < rows.length; below += 1)
					for (let at = column; at < column + span; at += 1) carried[rowIndex + below].push(at);
				column += span;
			}
		});
		/* A cell a merge above covered becomes an empty cell in the grid. */
		rows.forEach((row, rowIndex) => {
			for (const at of carried[rowIndex].toSorted((left, right) => left - right)) {
				const cells = [...row.children].filter((cell) => /^T[DH]$/u.test(cell.tagName));
				const filler = document.createElement('td');
				if (at >= cells.length) row.append(filler);
				else cells[at].before(filler);
			}
		});
	}
	if (merged) lost.add('merged cells (split into a plain grid)');
	for (const image of document.querySelectorAll('img')) {
		if (image.closest('figure[data-attachment-id]')) continue;
		const source = image.getAttribute('src') ?? '';
		if (/^data:image\//iu.test(source) || /^https?:\/\//iu.test(source)) images.push(source);
		else lost.add('pictures that were not inside the copy');
		image.remove();
	}
	for (const list of document.querySelectorAll(
		'ol[type], ol[style*="list-style"], ul[style*="list-style"]'
	)) {
		const type = list.getAttribute('type') ?? '';
		const style = (list as HTMLElement).style?.listStyleType ?? '';
		if ((type && type !== '1') || (style && !['decimal', 'disc', 'none', ''].includes(style)))
			lost.add('list numbering styles');
	}
	if (document.querySelector('iframe, video, audio, object, embed, svg, canvas'))
		lost.add('embedded media');
	return { html: document.body.innerHTML, images, lost: [...lost] };
}

/** Whether a clipboard's HTML carries words, not only a picture of them. */
function htmlHasText(html: string): boolean {
	return /<(p|h[1-6]|li|td|th|span|div)\b[^>]*>[^<\s]/iu.test(html);
}

/* An image is a block that names a company file; the page supplies its address. A copied image
 * pastes back as itself; an image pasted from elsewhere arrives as a file and is uploaded. */
function imageExtension(hooks: DocumentEditorHooks) {
	return Node.create({
		name: 'image',
		group: 'block',
		atom: true,
		draggable: true,
		selectable: true,
		addAttributes: () => ({
			...BLOCK_ID,
			attachment_id: {
				default: null,
				parseHTML: (element: HTMLElement) =>
					attachmentIdFrom(element.getAttribute('data-attachment-id')),
				renderHTML: (attributes: Record<string, unknown>) =>
					typeof attributes.attachment_id === 'string'
						? { 'data-attachment-id': attributes.attachment_id }
						: {}
			},
			alt: {
				default: null,
				parseHTML: (element: HTMLElement) => {
					const alt = element.querySelector('img')?.getAttribute('alt') ?? '';
					return alt.length <= MAX_IMAGE_ALT && !alt.includes('\0') ? alt : null;
				},
				renderHTML: () => ({})
			},
			width: {
				default: null,
				parseHTML: (element: HTMLElement) => {
					const width = Number(element.querySelector('img')?.getAttribute('width'));
					return Number.isInteger(width) && width >= 16 && width <= 4_096 ? width : null;
				},
				renderHTML: () => ({})
			},
			caption: {
				default: null,
				parseHTML: (element: HTMLElement) =>
					bounded(element.querySelector('figcaption')?.textContent, MAX_IMAGE_ALT),
				renderHTML: () => ({})
			}
		}),
		parseHTML: () => [
			{
				tag: 'figure[data-attachment-id]',
				getAttrs: (element: HTMLElement) =>
					attachmentIdFrom(element.getAttribute('data-attachment-id')) ? {} : false
			}
		],
		renderHTML: ({ node, HTMLAttributes }) => {
			const id = attachmentIdFrom(node.attrs.attachment_id);
			const width = node.attrs.width;
			const caption = typeof node.attrs.caption === 'string' ? node.attrs.caption : '';
			const image = [
				'img',
				{
					src: id && hooks.companyId ? documentImageSrc(hooks.companyId, id) : null,
					alt: typeof node.attrs.alt === 'string' ? node.attrs.alt : '',
					...(typeof width === 'number' ? { width: String(width) } : {}),
					loading: 'lazy',
					draggable: 'false'
				}
			] as const;
			return [
				'figure',
				mergeAttributes(HTMLAttributes, { class: 'native-image' }),
				image,
				...(caption ? [['figcaption', {}, caption] as const] : [])
			];
		},
		addProseMirrorPlugins() {
			return [
				new Plugin({
					key: new PluginKey('documentImageFiles'),
					props: {
						/* Word and Excel put a picture of the copied text on the clipboard beside the
						 * text itself: the text wins. */
						handlePaste: (view, event) => {
							const html = event.clipboardData?.getData('text/html') ?? '';
							if (htmlHasText(html)) return false;
							const files = imageFiles(event.clipboardData?.files);
							if (!files.length || !hooks.uploadImages || !view.editable) return false;
							event.preventDefault();
							hooks.uploadImages(files, view.state.selection.from);
							return true;
						},
						transformPastedHTML: (html, view) => {
							if (typeof DOMParser === 'undefined' || !view.editable) return html;
							const prepared = preparePastedHtml(html, new DOMParser());
							if (prepared.images.length || prepared.lost.length)
								/* Once the words are in, the pictures go after them. */
								setTimeout(() =>
									hooks.pasted?.(prepared.images, prepared.lost, view.state.selection.from)
								);
							return prepared.html;
						},
						handleDrop: (view, event, _slice, moved) => {
							const files = moved ? [] : imageFiles(event.dataTransfer?.files);
							if (!files.length || !hooks.uploadImages || !view.editable) return false;
							event.preventDefault();
							const at = view.posAtCoords({ left: event.clientX, top: event.clientY });
							hooks.uploadImages(files, at?.pos ?? view.state.selection.from);
							return true;
						}
					}
				})
			];
		}
	});
}

/** Someone or something the "@" menu can name in place: a person, a document or Work. */
export interface DocumentSuggestion {
	kind: 'actor' | 'document' | 'work';
	id: string;
	label: string;
}

/** The sizes an image can be set to; null is as wide as the page allows. */
export const DOCUMENT_IMAGE_SIZES: { label: string; width: number | null }[] = [
	{ label: 'Small', width: 320 },
	{ label: 'Medium', width: 560 },
	{ label: 'Full width', width: null }
];

/** The image block the selection is on, if it is one. */
export function selectedDocumentImage(
	editor: Editor
): { pos: number; node: ProseMirrorNode } | null {
	const selection = editor.state.selection as { node?: ProseMirrorNode; from: number };
	return selection.node?.type.name === 'image'
		? { pos: selection.from, node: selection.node }
		: null;
}

/** Change a selected image's size, description or caption; an empty text clears it. */
export function updateDocumentImage(
	editor: Editor,
	pos: number,
	attrs: { width?: number | null; alt?: string | null; caption?: string | null }
): void {
	const node = editor.state.doc.nodeAt(pos);
	if (!node || node.type.name !== 'image' || !editor.isEditable) return;
	const tidy = (value: string | null | undefined) =>
		value === undefined
			? undefined
			: (value ?? '')
					.replace(/\p{Cc}/gu, '')
					.slice(0, MAX_IMAGE_ALT)
					.trim() || null;
	const next = { ...node.attrs };
	if (attrs.width !== undefined) next.width = attrs.width;
	if (attrs.alt !== undefined) next.alt = tidy(attrs.alt);
	if (attrs.caption !== undefined) next.caption = tidy(attrs.caption);
	const tr = editor.state.tr.setNodeMarkup(pos, undefined, next);
	tr.setSelection(NodeSelection.create(tr.doc, pos));
	editor.view.dispatch(tr);
}

/** The "/" menu while it is open: where its slash is and what has been typed after it. */
export interface SlashMenuState {
	from: number;
	query: string;
}

export const slashMenuKey = new PluginKey<SlashMenuState | null>('documentSlashMenu');

/** The menu stays open while the caret is after its slash in the same block and the words typed
 * after it could still name a block; anything else closes it. */
export function nextSlashMenuState(
	state: EditorState,
	previous: SlashMenuState | null,
	mapped: number,
	trigger = '/'
): SlashMenuState | null {
	if (!previous) return null;
	const { selection, doc } = state;
	if (!selection.empty || mapped < 0 || mapped >= doc.content.size) return null;
	const head = selection.from;
	if (head <= mapped || doc.textBetween(mapped, mapped + 1) !== trigger) return null;
	if (doc.resolve(mapped).parent !== selection.$from.parent) return null;
	const query = doc.textBetween(mapped + 1, head);
	if (query.length > (trigger === '/' ? 24 : 40) || /^\s|\s\s/u.test(query)) return null;
	return { from: mapped, query };
}

/** Whether a "/" typed here opens the block menu: at the start of a line of text, or after a
 * space, as in Notion. Mid-word ("and/or") and in code it is only a slash. */
export function slashOpensMenu(state: EditorState): boolean {
	const { $from, empty } = state.selection;
	const parent = $from.parent;
	if (!empty || !parent.isTextblock || parent.type.spec.code) return false;
	if ($from.parentOffset === 0) return true;
	const before = parent.textBetween($from.parentOffset - 1, $from.parentOffset, undefined, '￼');
	return /\s/u.test(before);
}

/* Typing "@" the same way opens a menu of people and pages to name in place. */
export const mentionMenuKey = new PluginKey<SlashMenuState | null>('documentMentionMenu');

/* Typing "/" at the start of a line, or after a space, opens a menu of blocks; a slash anywhere
 * else is a slash. "@" works the same way for people and pages. The page draws each menu and
 * handles its keys; Escape, or deleting the character, closes it. */
function triggerMenuPlugin(
	trigger: '/' | '@',
	key: PluginKey<SlashMenuState | null>,
	change: ((state: SlashMenuState | null) => void) | undefined,
	onKey: ((event: KeyboardEvent) => boolean) | undefined
) {
	return new Plugin<SlashMenuState | null>({
		key,
		state: {
			init: () => null,
			apply: (tr, value, _old, state) => {
				const meta = tr.getMeta(key) as SlashMenuState | null | undefined;
				if (meta !== undefined) return meta;
				return value
					? nextSlashMenuState(state, value, tr.mapping.map(value.from, -1), trigger)
					: null;
			}
		},
		props: {
			handleTextInput: (view, from, to, text) => {
				if (text !== trigger || !change || !slashOpensMenu(view.state)) return false;
				view.dispatch(
					view.state.tr.insertText(trigger, from, to).setMeta(key, { from, query: '' })
				);
				return true;
			},
			handleKeyDown: (view, event) => {
				if (!key.getState(view.state)) return false;
				if (event.key === 'Escape') {
					view.dispatch(view.state.tr.setMeta(key, null));
					return true;
				}
				return onKey?.(event) ?? false;
			}
		},
		view: () => ({
			update: (view) => change?.(key.getState(view.state) ?? null),
			destroy: () => change?.(null)
		})
	});
}

function slashMenuExtension(hooks: DocumentEditorHooks) {
	return Extension.create({
		name: 'documentSlashMenu',
		priority: 10_000,
		addProseMirrorPlugins: () => [
			triggerMenuPlugin('/', slashMenuKey, hooks.slashChange, hooks.slashKey),
			triggerMenuPlugin('@', mentionMenuKey, hooks.mentionChange, hooks.mentionKey)
		]
	});
}

const GRIP =
	'<svg width="10" height="16" viewBox="0 0 10 16" aria-hidden="true"><g fill="currentColor"><circle cx="2.5" cy="3" r="1.4"/><circle cx="7.5" cy="3" r="1.4"/><circle cx="2.5" cy="8" r="1.4"/><circle cx="7.5" cy="8" r="1.4"/><circle cx="2.5" cy="13" r="1.4"/><circle cx="7.5" cy="13" r="1.4"/></g></svg>';

/* A grip beside the block under the pointer: drag it to move the block, click it (or press
 * Ctrl+/ in the block) for its actions. Tiptap's own drag handle does the moving, CRDT-safely. */
function blockHandleExtension(hooks: DocumentEditorHooks) {
	return Extension.create({
		name: 'documentBlockHandle',
		addKeyboardShortcuts() {
			return {
				'Mod-/': () => {
					const { $from } = this.editor.state.selection;
					if (!hooks.blockMenu || !this.editor.isEditable || $from.depth < 1) return false;
					const pos = $from.before(1);
					const dom = this.editor.view.nodeDOM(pos);
					if (!(dom instanceof HTMLElement)) return false;
					hooks.blockMenu({ pos, anchor: dom.getBoundingClientRect(), fromKeyboard: true });
					return true;
				}
			};
		},
		addProseMirrorPlugins() {
			if (!hooks.blockMenu || typeof window === 'undefined') return [];
			const element = window.document.createElement('button');
			element.type = 'button';
			element.className = 'block-handle';
			element.setAttribute('aria-label', 'Move or change this block');
			const mod = /Mac|iPhone|iPad/u.test(window.navigator.platform) ? '⌘' : 'Ctrl+';
			element.title = `Drag to move. Click, or press ${mod}/, for more`;
			element.innerHTML = GRIP;
			let current = -1;
			element.addEventListener('click', () => {
				if (current >= 0)
					hooks.blockMenu?.({ pos: current, anchor: element.getBoundingClientRect() });
			});
			return [
				DragHandlePlugin({
					editor: this.editor,
					element,
					nestedOptions: normalizeNestedOptions(false),
					computePositionConfig: { placement: 'left-start', strategy: 'absolute' },
					onNodeChange: ({ node, pos }) => {
						current = node ? pos : -1;
						// Readers move and change nothing, so they see no grip.
						element.style.display = this.editor.isEditable ? '' : 'none';
					}
				}).plugin
			];
		}
	});
}

/** A two-column table with a heading row, as the toolbar and the "/" menu insert it. */
export function documentTable(): JSONContent {
	const row = (type: 'tableHeader' | 'tableCell') => ({
		type: 'tableRow',
		content: [
			{ type, content: [{ type: 'paragraph' }] },
			{ type, content: [{ type: 'paragraph' }] }
		]
	});
	return { type: 'table', content: [row('tableHeader'), row('tableCell')] };
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
		linkExtension(hooks),
		colourMark('highlight'),
		colourMark('textColor'),
		alignmentExtension,
		imageExtension(hooks),
		fileExtension(hooks),
		slashMenuExtension(hooks),
		blockHandleExtension(hooks),
		commentMarksExtension(hooks),
		findExtension,
		emptyLineHintExtension(),
		blockIdentityExtension,
		UniqueID.extend({ addProseMirrorPlugins: () => [] }).configure({
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

const MARK_TYPES = [
	'bold',
	'italic',
	'underline',
	'strike',
	'code',
	'link',
	'highlight',
	'textColor'
];

function projectMark(value: unknown): DocumentMark {
	const source = object(value, 'text mark');
	const type = string(source.type, 'mark type', 32);
	if (!MARK_TYPES.includes(type)) {
		invalid(`mark ${type} is outside the company schema.`);
	}
	if (type === 'highlight' || type === 'textColor') {
		const attrs = object(source.attrs, `${type} attributes`);
		const color = oneOf(
			attrs.color,
			type === 'highlight' ? DOCUMENT_HIGHLIGHTS : DOCUMENT_TEXT_COLORS
		);
		if (!color) invalid(`${type} colour is outside the company palette.`);
		return { type, attrs: { color } };
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
	if ((type === 'paragraph' || type === 'heading') && source.align != null) {
		const align = oneOf(source.align, DOCUMENT_ALIGNMENTS);
		if (!align) invalid('alignment is invalid.');
		attrs.align = align;
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
		case 'image': {
			const id = string(source.attachment_id, 'image file', 36);
			if (!ATTACHMENT_ID.test(id)) invalid('image file is invalid.');
			attrs.attachment_id = id;
			if (source.alt !== null && source.alt !== undefined) {
				if (
					typeof source.alt !== 'string' ||
					[...source.alt].length > MAX_IMAGE_ALT ||
					source.alt.includes('\0')
				) {
					invalid('image description is invalid.');
				}
				attrs.alt = source.alt;
			}
			if (source.width !== null && source.width !== undefined) {
				if (
					!Number.isInteger(source.width) ||
					Number(source.width) < 16 ||
					Number(source.width) > 4_096
				) {
					invalid('image width is invalid.');
				}
				attrs.width = source.width;
			}
			if (source.caption !== null && source.caption !== undefined)
				attrs.caption = string(source.caption, 'image caption', MAX_IMAGE_ALT);
			break;
		}
		case 'file': {
			const id = string(source.attachment_id, 'file', 36);
			if (!ATTACHMENT_ID.test(id)) invalid('file is invalid.');
			attrs.attachment_id = id;
			attrs.name = string(source.name, 'file name', MAX_FILE_NAME);
			break;
		}
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
