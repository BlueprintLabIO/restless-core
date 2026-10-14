/* A Word or Excel file made into a native document or sheet the company can edit. Excel and CSV go
 * through the sheet import the Sheets page already uses; a Word file is read in the browser and
 * becomes the body of a new document. The original file is untouched. */
import { createDocument, type DocumentContent } from './documents';
import { htmlDocumentContent } from './docx-content';
import { importSpreadsheet } from './sheets';

export type ImportKind = 'doc' | 'sheet';

/** What a file can become, by its name: a Word file a document, a workbook or CSV a sheet. */
export function importKind(name: string): ImportKind | null {
	const path = name.split(/[?#]/)[0].toLowerCase();
	if (path.endsWith('.docx')) return 'doc';
	if (path.endsWith('.xlsx') || path.endsWith('.csv')) return 'sheet';
	return null;
}

/** The title a converted file gets: its name without the extension. */
export function importTitle(name: string, fallback: string): string {
	const base = name.split('/').at(-1) ?? name;
	return (
		base
			.replace(/\.(docx|xlsx|csv)$/i, '')
			.trim()
			.slice(0, 200) || fallback
	);
}

/** A Word document as a native document body. Pictures are left out: mammoth would carry them as
 * very long data addresses, and a document image is a company file of its own. */
export async function docxContent(bytes: ArrayBuffer): Promise<DocumentContent> {
	const mammoth = (await import('mammoth')).default;
	const result = await mammoth.convertToHtml(
		{ arrayBuffer: bytes },
		{ convertImage: mammoth.images.imgElement(() => Promise.resolve({ src: '' })) }
	);
	return htmlDocumentContent(new DOMParser().parseFromString(result.value, 'text/html').body);
}

/** Make the file an editable document or sheet and return where it opens. */
export async function importOfficeFile(
	company: string,
	file: File
): Promise<{ kind: ImportKind; id: string }> {
	const kind = importKind(file.name);
	if (kind === 'sheet') return { kind, id: (await importSpreadsheet(company, file)).id };
	if (kind !== 'doc')
		throw new Error('Choose a Word document (.docx), an Excel workbook or a CSV.');
	let content: DocumentContent;
	try {
		content = await docxContent(await file.arrayBuffer());
	} catch {
		throw new Error(
			'This Word document could not be read. If it has a password, remove it in Word first.'
		);
	}
	const receipt = await createDocument(
		company,
		{
			title: importTitle(file.name, 'Imported document'),
			kind: 'freeform',
			visibility: 'company',
			linked_room_id: null,
			inherit_room_visibility: false,
			content_json: content,
			reason: `Imported from ${file.name}`.slice(0, 200)
		},
		crypto.randomUUID()
	);
	return { kind, id: receipt.document_id };
}

/** Where an imported document or sheet opens. */
export function importedHref(company: string, imported: { kind: ImportKind; id: string }): string {
	const root = `/${encodeURIComponent(company)}/library`;
	return imported.kind === 'doc'
		? `${root}/documents?document=${encodeURIComponent(imported.id)}`
		: `${root}/sheets?sheet=${encodeURIComponent(imported.id)}`;
}
