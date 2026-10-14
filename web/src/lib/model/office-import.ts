/* A Word or Excel file made into a native document or sheet the company can edit. Excel and CSV go
 * through the sheet import the Sheets page already uses; a Word file is read as Markdown in the
 * browser and becomes the first saved version of a new document. The original file is untouched. */
import { createDocument, emptyDocument, getDocument } from './documents';
import { importSpreadsheet } from './sheets';
import { ownerJson } from './failure';

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

/** A Word document's text as Markdown. Images are left out: a document carries text, and an
 * embedded picture would otherwise arrive as a very long data address. */
export async function docxMarkdown(bytes: ArrayBuffer): Promise<string> {
	const mammoth = (await import('mammoth')).default as unknown as {
		convertToMarkdown(
			input: { arrayBuffer: ArrayBuffer },
			options?: Record<string, unknown>
		): Promise<{ value: string }>;
		images: { imgElement(read: () => Promise<{ src: string }>): unknown };
	};
	const result = await mammoth.convertToMarkdown(
		{ arrayBuffer: bytes },
		{ convertImage: mammoth.images.imgElement(() => Promise.resolve({ src: '' })) }
	);
	return result.value
		.replace(/!\[[^\]]*\]\([^)]*\)/g, '')
		.replace(/<a id="[^"]*"><\/a>/g, '')
		.replace(/\n{3,}/g, '\n\n')
		.trim();
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
	let markdown: string;
	try {
		markdown = await docxMarkdown(await file.arrayBuffer());
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
			content_json: emptyDocument(),
			reason: `Imported from ${file.name}`.slice(0, 200)
		},
		crypto.randomUUID()
	);
	if (markdown) {
		const created = await getDocument(company, receipt.document_id);
		await ownerJson(
			await fetch(
				`/api/companies/${encodeURIComponent(company)}/documents/${encodeURIComponent(receipt.document_id)}/imports/markdown`,
				{
					method: 'POST',
					credentials: 'same-origin',
					headers: { 'content-type': 'application/json', 'Idempotency-Key': crypto.randomUUID() },
					body: JSON.stringify({
						expected_current_version_id: created.current_version.version.id,
						markdown,
						reason: `Imported from ${file.name}`.slice(0, 200)
					})
				}
			)
		);
	}
	return { kind, id: receipt.document_id };
}

/** Where an imported document or sheet opens. */
export function importedHref(company: string, imported: { kind: ImportKind; id: string }): string {
	const root = `/${encodeURIComponent(company)}/library`;
	return imported.kind === 'doc'
		? `${root}/documents?document=${encodeURIComponent(imported.id)}`
		: `${root}/sheets?sheet=${encodeURIComponent(imported.id)}`;
}
