/* A Word document, as the simple HTML mammoth reads from it, turned into a native document body:
 * headings, paragraphs, lists, tables, quotes, code and rules, with bold, italic, struck, code and
 * link marks. Anything else keeps its text and loses its shape; pictures are left out. */
import type { DocumentContent, DocumentMark, DocumentNode } from './documents';

/** The few DOM members this reads, so it runs on a browser's DOM or any compatible one. */
interface HtmlNode {
	nodeType: number;
	nodeName: string;
	textContent: string | null;
	childNodes: ArrayLike<HtmlNode>;
	getAttribute?(name: string): string | null;
}

const TEXT = 3;
const ELEMENT = 1;
const SAFE_LINK = /^(https?:|mailto:)/i;
const MAX_HREF = 2_048;

const id = () => `block-${crypto.randomUUID()}`;
const tag = (node: HtmlNode) => node.nodeName.toLowerCase();
const children = (node: HtmlNode) => Array.from(node.childNodes);

function markFor(node: HtmlNode): DocumentMark | null {
	switch (tag(node)) {
		case 'strong':
		case 'b':
			return { type: 'bold' };
		case 'em':
		case 'i':
			return { type: 'italic' };
		case 'u':
			return { type: 'underline' };
		case 's':
		case 'del':
		case 'strike':
			return { type: 'strike' };
		case 'code':
			return { type: 'code' };
		case 'a': {
			const href = node.getAttribute?.('href')?.trim() ?? '';
			return SAFE_LINK.test(href) && href.length <= MAX_HREF
				? { type: 'link', attrs: { href } }
				: null;
		}
		default:
			return null;
	}
}

function inline(node: HtmlNode, marks: DocumentMark[], out: DocumentNode[]): void {
	if (node.nodeType === TEXT) {
		const text = (node.textContent ?? '').replace(/[\t\n\r]+/g, ' ').replace(/\0/g, '');
		if (!text) return;
		const previous = out.at(-1);
		const same =
			previous?.type === 'text' && JSON.stringify(previous.marks ?? []) === JSON.stringify(marks);
		if (same) previous.text += text;
		else out.push(marks.length ? { type: 'text', text, marks } : { type: 'text', text });
		return;
	}
	if (node.nodeType !== ELEMENT) return;
	const name = tag(node);
	if (name === 'br') {
		out.push({ type: 'hardBreak' });
		return;
	}
	if (name === 'img') return;
	const mark = markFor(node);
	const next = mark && !marks.some((held) => held.type === mark.type) ? [...marks, mark] : marks;
	for (const child of children(node)) inline(child, next, out);
}

function inlineContent(nodes: HtmlNode[]): DocumentNode[] | undefined {
	const out: DocumentNode[] = [];
	for (const node of nodes) inline(node, [], out);
	/* A paragraph does not start or end on a space Word kept, nor hold only spaces. */
	const first = out[0];
	if (first?.type === 'text') first.text = first.text!.replace(/^ +/, '');
	const last = out.at(-1);
	if (last?.type === 'text') last.text = last.text!.replace(/ +$/, '');
	const kept = out.filter((node) => node.type !== 'text' || node.text);
	return kept.length ? kept : undefined;
}

function paragraph(nodes: HtmlNode[]): DocumentNode {
	const content = inlineContent(nodes);
	return content
		? { type: 'paragraph', attrs: { block_id: id() }, content }
		: { type: 'paragraph', attrs: { block_id: id() } };
}

const BLOCK_TAGS = new Set([
	'p',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'ul',
	'ol',
	'table',
	'blockquote',
	'pre',
	'hr',
	'div',
	'section',
	'article',
	'tbody',
	'thead'
]);

/** Block children of an element; runs of inline content between blocks become paragraphs. */
function blocks(parent: HtmlNode): DocumentNode[] {
	const out: DocumentNode[] = [];
	let run: HtmlNode[] = [];
	const flush = () => {
		if (run.some((node) => (node.textContent ?? '').trim() || tag(node) === 'br'))
			out.push(paragraph(run));
		run = [];
	};
	for (const node of children(parent)) {
		if (node.nodeType === ELEMENT && BLOCK_TAGS.has(tag(node))) {
			flush();
			out.push(...block(node));
		} else run.push(node);
	}
	flush();
	return out;
}

function list(node: HtmlNode): DocumentNode {
	const ordered = tag(node) === 'ol';
	const items = children(node)
		.filter((child) => child.nodeType === ELEMENT && tag(child) === 'li')
		.map((item): DocumentNode => {
			const content = blocks(item);
			if (content[0]?.type !== 'paragraph')
				content.unshift({ type: 'paragraph', attrs: { block_id: id() } });
			return { type: 'listItem', attrs: { block_id: id() }, content };
		});
	return {
		type: ordered ? 'orderedList' : 'bulletList',
		attrs: { block_id: id() },
		content: items.length
			? items
			: [
					{
						type: 'listItem',
						attrs: { block_id: id() },
						content: [{ type: 'paragraph', attrs: { block_id: id() } }]
					}
				]
	};
}

function table(node: HtmlNode): DocumentNode[] {
	const rows: HtmlNode[] = [];
	const collect = (parent: HtmlNode) => {
		for (const child of children(parent)) {
			if (child.nodeType !== ELEMENT) continue;
			if (tag(child) === 'tr') rows.push(child);
			else if (['thead', 'tbody', 'tfoot'].includes(tag(child))) collect(child);
		}
	};
	collect(node);
	const width = Math.max(
		0,
		...rows.map(
			(row) =>
				children(row).filter(
					(cell) => cell.nodeType === ELEMENT && ['td', 'th'].includes(tag(cell))
				).length
		)
	);
	if (!rows.length || !width) return [];
	return [
		{
			type: 'table',
			attrs: { block_id: id() },
			content: rows.map((row): DocumentNode => {
				const cells = children(row).filter(
					(cell) => cell.nodeType === ELEMENT && ['td', 'th'].includes(tag(cell))
				);
				/* Every row as wide as the widest: merged cells become empty ones. */
				const filled: DocumentNode[] = cells.map((cell) => {
					const content = blocks(cell);
					return {
						type: tag(cell) === 'th' ? 'tableHeader' : 'tableCell',
						attrs: { block_id: id() },
						content: content.length ? content : [paragraph([])]
					};
				});
				while (filled.length < width)
					filled.push({ type: 'tableCell', attrs: { block_id: id() }, content: [paragraph([])] });
				return { type: 'tableRow', attrs: { block_id: id() }, content: filled };
			})
		}
	];
}

function block(node: HtmlNode): DocumentNode[] {
	const name = tag(node);
	const heading = /^h([1-6])$/.exec(name);
	if (heading) {
		const content = inlineContent(children(node));
		const attrs = { block_id: id(), level: Number(heading[1]) };
		return [content ? { type: 'heading', attrs, content } : { type: 'heading', attrs }];
	}
	switch (name) {
		case 'p':
			return [paragraph(children(node))];
		case 'ul':
		case 'ol':
			return [list(node)];
		case 'table':
			return table(node);
		case 'blockquote': {
			const content = blocks(node);
			return [
				{
					type: 'blockquote',
					attrs: { block_id: id() },
					content: content.length ? content : [paragraph([])]
				}
			];
		}
		case 'pre': {
			const text = (node.textContent ?? '').replace(/\0/g, '');
			return [
				text
					? { type: 'codeBlock', attrs: { block_id: id() }, content: [{ type: 'text', text }] }
					: { type: 'codeBlock', attrs: { block_id: id() } }
			];
		}
		case 'hr':
			return [{ type: 'horizontalRule', attrs: { block_id: id() } }];
		default:
			return blocks(node);
	}
}

/** The document body for the HTML inside `body`. Never empty: a blank file is one paragraph. */
export function htmlDocumentContent(body: HtmlNode): DocumentContent {
	const content = blocks(body);
	return {
		type: 'doc',
		content: content.length ? content : [{ type: 'paragraph', attrs: { block_id: id() } }]
	};
}
