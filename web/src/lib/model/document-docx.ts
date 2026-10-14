/* A native document as a Word file (.docx), written in the browser from its saved body. It keeps
 * what Word can hold of the company schema: headings, paragraphs, bulleted, numbered and task
 * lists, tables, quotes, code, rules and line breaks, with bold, italic, underlined, struck,
 * coloured, highlighted, code and linked text, and each paragraph's alignment. Pictures and comments are not carried. */
import { strToU8, zipSync } from 'fflate';
import type { DocumentContent, DocumentNode } from './documents';

export const DOCX_TYPE = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';

const escape = (text: string) =>
	text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		// Characters XML 1.0 cannot carry at all.
		.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F￾￿]/g, '');

interface Writer {
	links: string[];
	/** One numbering instance per list, so every numbered list starts at its own number. */
	lists: { ordered: boolean; start: number }[];
}

/* The named colours a document keeps, as Word colours. */
const TEXT_COLOURS: Record<string, string> = {
	gray: '6B7280',
	red: 'DC2626',
	orange: 'C2410C',
	green: '15803D',
	blue: '1D4ED8',
	purple: '7E22CE'
};
const HIGHLIGHTS: Record<string, string> = {
	yellow: 'FEF08A',
	green: 'BBF7D0',
	blue: 'BFDBFE',
	pink: 'FBCFE8',
	purple: 'E9D5FF'
};
const JUSTIFY: Record<string, string> = { center: 'center', right: 'right', justify: 'both' };

/** A paragraph's alignment, last of its properties as Word's schema orders them. */
function alignment(node: DocumentNode): string {
	const value = JUSTIFY[String(node.attrs?.align ?? '')];
	return value ? `<w:jc w:val="${value}"/>` : '';
}

function runs(nodes: DocumentNode[] | undefined, writer: Writer): string {
	let out = '';
	for (const node of nodes ?? []) {
		if (node.type === 'hardBreak') {
			out += '<w:r><w:br/></w:r>';
			continue;
		}
		const text =
			node.type === 'text'
				? (node.text ?? '')
				: node.type === 'mention'
					? `@${String(node.attrs?.label ?? '')}`
					: node.type === 'reference'
						? String(node.attrs?.label ?? '')
						: '';
		if (!text) continue;
		const marks = node.marks ?? [];
		const has = (type: string) => marks.some((mark) => mark.type === type);
		const colour = (type: string, palette: Record<string, string>) =>
			palette[String(marks.find((mark) => mark.type === type)?.attrs?.color ?? '')] ?? '';
		const link = marks.find((mark) => mark.type === 'link');
		const href = typeof link?.attrs?.href === 'string' ? link.attrs.href : '';
		/* In the order Word's schema lists run properties. */
		let props = '';
		if (href) props += '<w:rStyle w:val="Hyperlink"/>';
		if (has('code')) props += '<w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/>';
		if (has('bold')) props += '<w:b/>';
		if (has('italic')) props += '<w:i/>';
		if (has('strike')) props += '<w:strike/>';
		const textColour = colour('textColor', TEXT_COLOURS);
		if (textColour) props += `<w:color w:val="${textColour}"/>`;
		if (has('underline')) props += '<w:u w:val="single"/>';
		const highlight = colour('highlight', HIGHLIGHTS);
		if (highlight) props += `<w:shd w:val="clear" w:color="auto" w:fill="${highlight}"/>`;
		const run = `<w:r>${props ? `<w:rPr>${props}</w:rPr>` : ''}<w:t xml:space="preserve">${escape(text)}</w:t></w:r>`;
		if (href && /^(https?:|mailto:)/i.test(href)) {
			writer.links.push(href);
			out += `<w:hyperlink r:id="rIdLink${writer.links.length}" w:history="1">${run}</w:hyperlink>`;
		} else if (href.startsWith('#')) {
			out += `<w:hyperlink w:anchor="${escape(href.slice(1))}">${run}</w:hyperlink>`;
		} else out += run;
	}
	return out;
}

function paragraph(props: string, content: string): string {
	return `<w:p>${props ? `<w:pPr>${props}</w:pPr>` : ''}${content}</w:p>`;
}

/** Paragraph properties a block inherits from where it sits: a quote, a list item, a cell. */
interface Context {
	quote?: boolean;
	list?: { num: number; level: number };
	prefix?: string;
}

function blocks(nodes: DocumentNode[] | undefined, writer: Writer, context: Context = {}): string {
	return (nodes ?? []).map((node) => block(node, writer, context)).join('');
}

function contextProps(context: Context): string {
	let props = '';
	if (context.list)
		props += `<w:numPr><w:ilvl w:val="${context.list.level}"/><w:numId w:val="${context.list.num}"/></w:numPr>`;
	if (context.quote) props += '<w:ind w:left="567"/>';
	return props;
}

function block(node: DocumentNode, writer: Writer, context: Context): string {
	switch (node.type) {
		case 'heading': {
			const level = Math.min(6, Math.max(1, Number(node.attrs?.level ?? 1)));
			return paragraph(
				`<w:pStyle w:val="Heading${level}"/>${alignment(node)}`,
				runs(node.content, writer)
			);
		}
		case 'paragraph': {
			const prefix = context.prefix
				? `<w:r><w:t xml:space="preserve">${escape(context.prefix)}</w:t></w:r>`
				: '';
			return paragraph(
				`${context.quote && !context.list ? '<w:pStyle w:val="Quote"/>' : ''}${contextProps(context)}${alignment(node)}`,
				prefix + runs(node.content, writer)
			);
		}
		case 'blockquote':
			return blocks(node.content, writer, { ...context, quote: true });
		case 'codeBlock': {
			const lines = (node.content ?? [])
				.map((child) => child.text ?? '')
				.join('')
				.split('\n');
			return lines
				.map((line) =>
					paragraph(
						'<w:pStyle w:val="Code"/>',
						line ? `<w:r><w:t xml:space="preserve">${escape(line)}</w:t></w:r>` : ''
					)
				)
				.join('');
		}
		case 'horizontalRule':
			return paragraph(
				'<w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="auto"/></w:pBdr>',
				''
			);
		case 'bulletList':
		case 'orderedList': {
			const level = context.list ? Math.min(8, context.list.level + 1) : 0;
			writer.lists.push({
				ordered: node.type === 'orderedList',
				start: Number(node.attrs?.start ?? 1) || 1
			});
			const num = writer.lists.length;
			return (node.content ?? [])
				.map((item) => blocks(item.content, writer, { quote: context.quote, list: { num, level } }))
				.join('');
		}
		case 'taskList':
			return (node.content ?? [])
				.map((item) =>
					blocks(item.content, writer, {
						quote: context.quote,
						prefix: item.attrs?.checked === true ? '☑ ' : '☐ '
					})
				)
				.join('');
		case 'table': {
			const rows = node.content ?? [];
			const width = Math.max(1, ...rows.map((row) => row.content?.length ?? 0));
			const grid = `<w:tblGrid>${'<w:gridCol w:w="2000"/>'.repeat(width)}</w:tblGrid>`;
			const body = rows
				.map(
					(row) =>
						`<w:tr>${(row.content ?? [])
							.map((cell) => {
								const inner = blocks(cell.content, writer);
								/* A header cell reads bold; Word wants at least one paragraph per cell. */
								const content = inner || '<w:p/>';
								return `<w:tc><w:tcPr><w:tcW w:w="0" w:type="auto"/></w:tcPr>${
									cell.type === 'tableHeader'
										? content.replace(/<w:r>(?!<w:rPr>)/g, '<w:r><w:rPr><w:b/></w:rPr>')
										: content
								}</w:tc>`;
							})
							.join('')}</w:tr>`
				)
				.join('');
			return `<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/><w:tblBorders><w:top w:val="single" w:sz="4" w:color="auto"/><w:left w:val="single" w:sz="4" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:color="auto"/><w:right w:val="single" w:sz="4" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:color="auto"/></w:tblBorders></w:tblPr>${grid}${body}</w:tbl><w:p/>`;
		}
		case 'file':
			/* The file is not carried; its name says what it was. */
			return paragraph(
				'',
				`<w:r><w:t xml:space="preserve">${escape(`📎 ${String(node.attrs?.name ?? 'File')}`)}</w:t></w:r>`
			);
		case 'image': {
			/* The picture itself is not carried; its caption is. */
			const caption = String(node.attrs?.caption ?? '').trim();
			return caption
				? paragraph(
						'',
						`<w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve">${escape(caption)}</w:t></w:r>`
					)
				: '';
		}
		default:
			/* Anything else keeps no place in Word, but its text, if any, stays. */
			return node.content ? blocks(node.content, writer, context) : '';
	}
}

const NS =
	'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"';

function stylesXml(): string {
	const heading = (level: number, size: number) =>
		`<w:style w:type="paragraph" w:styleId="Heading${level}"><w:name w:val="heading ${level}"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="80"/><w:outlineLvl w:val="${level - 1}"/></w:pPr><w:rPr><w:b/><w:sz w:val="${size}"/></w:rPr></w:style>`;
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles ${NS}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>${[40, 32, 28, 24, 22, 22].map((size, index) => heading(index + 1, size)).join('')}<w:style w:type="paragraph" w:styleId="Quote"><w:name w:val="Quote"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="567"/></w:pPr><w:rPr><w:i/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Code"><w:name w:val="Code"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0"/></w:pPr><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="20"/></w:rPr></w:style><w:style w:type="character" w:styleId="Hyperlink"><w:name w:val="Hyperlink"/><w:rPr><w:color w:val="0563C1"/><w:u w:val="single"/></w:rPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/></w:style></w:styles>`;
}

function numberingXml(lists: Writer['lists']): string {
	const level = (ordered: boolean, index: number) =>
		`<w:lvl w:ilvl="${index}"><w:start w:val="1"/><w:numFmt w:val="${ordered ? (['decimal', 'lowerLetter', 'lowerRoman'][index % 3] ?? 'decimal') : 'bullet'}"/><w:lvlText w:val="${ordered ? `%${index + 1}.` : ['•', '◦', '▪'][index % 3]}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="${720 + index * 360}" w:hanging="360"/></w:pPr></w:lvl>`;
	const abstract = (id: number, ordered: boolean) =>
		`<w:abstractNum w:abstractNumId="${id}"><w:multiLevelType w:val="hybridMultilevel"/>${Array.from({ length: 9 }, (_, index) => level(ordered, index)).join('')}</w:abstractNum>`;
	const nums = lists
		.map(
			(list, index) =>
				`<w:num w:numId="${index + 1}"><w:abstractNumId w:val="${list.ordered ? 1 : 0}"/>${
					list.ordered
						? `<w:lvlOverride w:ilvl="0"><w:startOverride w:val="${list.start}"/></w:lvlOverride>`
						: ''
				}</w:num>`
		)
		.join('');
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering ${NS}>${abstract(0, false)}${abstract(1, true)}${nums}</w:numbering>`;
}

/** The document as a .docx file: its title as the first heading unless the body starts with it. */
export function documentDocx(title: string, content: DocumentContent): Blob {
	const writer: Writer = { links: [], lists: [] };
	const first = content.content[0];
	const firstText = (first?.content ?? []).map((node) => node.text ?? '').join('');
	const heading =
		title.trim() && !(first?.type === 'heading' && firstText.trim() === title.trim())
			? paragraph(
					'<w:pStyle w:val="Heading1"/>',
					runs([{ type: 'text', text: title.trim() }], writer)
				)
			: '';
	const body = heading + blocks(content.content, writer);
	const documentXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document ${NS}><w:body>${body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr></w:body></w:document>`;
	const rel = (id: string, type: string, target: string, external = false) =>
		`<Relationship Id="${id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/${type}" Target="${escape(target)}"${external ? ' TargetMode="External"' : ''}/>`;
	const documentRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${rel('rIdStyles', 'styles', 'styles.xml')}${rel('rIdNumbering', 'numbering', 'numbering.xml')}${writer.links.map((href, index) => rel(`rIdLink${index + 1}`, 'hyperlink', href, true)).join('')}</Relationships>`;
	const files: Record<string, Uint8Array> = {
		'[Content_Types].xml': strToU8(
			'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>'
		),
		'_rels/.rels': strToU8(
			`<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${rel('rId1', 'officeDocument', 'word/document.xml')}</Relationships>`
		),
		'word/document.xml': strToU8(documentXml),
		'word/styles.xml': strToU8(stylesXml()),
		'word/numbering.xml': strToU8(numberingXml(writer.lists)),
		'word/_rels/document.xml.rels': strToU8(documentRels)
	};
	return new Blob([zipSync(files, { level: 6 }) as Uint8Array<ArrayBuffer>], { type: DOCX_TYPE });
}
