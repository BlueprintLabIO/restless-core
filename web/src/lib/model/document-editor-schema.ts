import { Mark, Node, type Editor, type Extensions, type JSONContent } from '@tiptap/core';
import Collaboration, { isChangeOrigin } from '@tiptap/extension-collaboration';
import TaskItem from '@tiptap/extension-task-item';
import TaskList from '@tiptap/extension-task-list';
import UniqueID from '@tiptap/extension-unique-id';
import StarterKit from '@tiptap/starter-kit';
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

const BLOCK_ID = {
	block_id: {
		default: null,
		parseHTML: (element: HTMLElement) => element.getAttribute('data-block-id'),
		renderHTML: (attributes: Record<string, unknown>) =>
			typeof attributes.block_id === 'string' ? { 'data-block-id': attributes.block_id } : {}
	}
};

const Table = Node.create({
	name: 'table',
	group: 'block',
	content: 'tableRow+',
	isolating: true,
	addAttributes: () => BLOCK_ID,
	parseHTML: () => [{ tag: 'table' }],
	renderHTML: ({ HTMLAttributes }) => ['table', HTMLAttributes, ['tbody', 0]]
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

const Mention = Node.create({
	name: 'mention',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,
	addAttributes: () => ({
		actor_id: { default: null },
		label: { default: null }
	}),
	parseHTML: () => [{ tag: 'span[data-native-mention="true"]' }],
	renderHTML: ({ HTMLAttributes }) => [
		'span',
		{
			'data-native-mention': 'true',
			'data-actor-id': HTMLAttributes.actor_id,
			class: 'native-mention'
		},
		`@${String(HTMLAttributes.label ?? '')}`
	],
	renderText: ({ node }) => `@${String(node.attrs.label ?? '')}`
});

const Reference = Node.create({
	name: 'reference',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,
	addAttributes: () => ({
		kind: { default: null },
		id: { default: null },
		label: { default: null }
	}),
	parseHTML: () => [{ tag: 'span[data-native-reference="true"]' }],
	renderHTML: ({ HTMLAttributes }) => [
		'span',
		{
			'data-native-reference': 'true',
			'data-reference-kind': HTMLAttributes.kind,
			'data-reference-id': HTMLAttributes.id,
			class: 'native-reference'
		},
		String(HTMLAttributes.label ?? '')
	],
	renderText: ({ node }) => String(node.attrs.label ?? '')
});

const Link = Mark.create({
	name: 'link',
	priority: 1000,
	inclusive: false,
	keepOnSplit: false,
	exitable: true,
	addAttributes: () => ({
		href: { default: null },
		title: { default: null }
	}),
	parseHTML: () => [{ tag: 'a[href]' }],
	renderHTML: ({ HTMLAttributes }) => [
		'a',
		{
			href: HTMLAttributes.href,
			title: HTMLAttributes.title,
			rel: 'noopener noreferrer'
		},
		0
	]
});

export function createDocumentEditorExtensions(
	document: Doc,
	provider: HocuspocusProvider,
	updateBlockIds: boolean
): Extensions {
	return [
		StarterKit.configure({
			link: false,
			underline: false,
			undoRedo: false,
			heading: { levels: [1, 2, 3, 4, 5, 6] }
		}),
		TaskList,
		TaskItem.configure({ nested: true }),
		Table,
		TableRow,
		TableCell,
		TableHeader,
		Mention,
		Reference,
		Link,
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
		})
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
	return (
		(value.startsWith('/') && !value.startsWith('//') && !value.includes('\\')) ||
		(value.startsWith('#') && value.length > 1)
	);
}

function projectMark(value: unknown): DocumentMark {
	const source = object(value, 'text mark');
	const type = string(source.type, 'mark type', 32);
	if (!['bold', 'italic', 'strike', 'code', 'link'].includes(type)) {
		invalid(`mark ${type} is outside the company schema.`);
	}
	if (type !== 'link') return { type };
	const attrs = object(source.attrs, 'link attributes');
	const href = string(attrs.href, 'link href', 2_048);
	if (!safeHref(href)) invalid('link href is unsafe.');
	const projected: Record<string, unknown> = { href };
	if (attrs.title !== null && attrs.title !== undefined) {
		projected.title = string(attrs.title, 'link title', 300);
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
				if (!Number.isInteger(start) || Number(start) < 1 || Number(start) > 100_000) {
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
			if (!['work', 'decision', 'attention', 'document', 'artifact', 'actor'].includes(kind)) {
				invalid('reference kind is invalid.');
			}
			attrs.kind = kind;
			attrs.id = string(source.id, 'reference identity', 200);
			attrs.label = string(source.label, 'reference label', 300);
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
