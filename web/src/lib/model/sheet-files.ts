/* Spreadsheet files in and out of native sheets. Loaded only when someone imports or downloads:
 * it carries the spreadsheet engine. An .xlsx is read by the engine's own XLSX reader here, and
 * the server engine loads the result again before it becomes a sheet; a CSV goes as text. */
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { Model } from '@odoo/o-spreadsheet/dist/o_spreadsheet.esm.js';

export type SheetSource = { csv: string } | { workbook: Record<string, unknown> };
export const XLSX_TYPE = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
/** A sheet holds at most 8 MB; the request carries a little more than the workbook. */
const SHEET_BYTES = 8 * 1024 * 1024 - 64 * 1024;
/** Uncompressed workbook XML read from one file, against archives that inflate without end. */
const XML_BYTES = 256 * 1024 * 1024;

const TOO_LARGE = 'This file holds more than a sheet can (8 MB). Split it, or import fewer tabs.';

export async function sheetSource(file: File): Promise<SheetSource> {
	const name = file.name.toLowerCase();
	if (name.endsWith('.csv') || file.type === 'text/csv') {
		if (file.size > SHEET_BYTES) throw new Error(TOO_LARGE);
		return { csv: (await file.text()).replace(/^﻿/, '') };
	}
	if (!name.endsWith('.xlsx') && file.type !== XLSX_TYPE)
		throw new Error('Choose an Excel workbook (.xlsx) or a CSV file.');
	let files: Record<string, string>;
	try {
		let inflated = 0;
		const parts = unzipSync(new Uint8Array(await file.arrayBuffer()), {
			// Only the workbook's XML: images are not carried into sheets.
			filter: (entry) =>
				/\.(xml|rels)$/i.test(entry.name) && (inflated += entry.originalSize) <= XML_BYTES
		});
		files = Object.fromEntries(
			Object.entries(parts).map(([path, bytes]) => [path, strFromU8(bytes)])
		);
	} catch {
		throw new Error(
			'This file is not a readable Excel workbook. If it has a password, remove it in Excel first.'
		);
	}
	if (!files['[Content_Types].xml']) throw new Error('This file is not a readable Excel workbook.');
	let workbook: Record<string, unknown>;
	try {
		workbook = new Model(files, { mode: 'readonly' }).exportData();
	} catch {
		throw new Error(
			'This workbook could not be read. Save it again in Excel, or export it as CSV.'
		);
	}
	if (new Blob([JSON.stringify(workbook)]).size > SHEET_BYTES) throw new Error(TOO_LARGE);
	return { workbook };
}

/** The workbook as an .xlsx file, every tab included. */
export function xlsxBlob(model: any): Blob {
	const { files } = model.exportXLSX() as {
		files: { path: string; content?: string }[];
	};
	// Content types first, as Office writes it. Images are not exported.
	const entries: Record<string, Uint8Array> = {};
	for (const file of [...files].sort((a, b) =>
		a.path === '[Content_Types].xml' ? -1 : b.path === '[Content_Types].xml' ? 1 : 0
	))
		if (typeof file.content === 'string') entries[file.path] = strToU8(file.content);
	return new Blob([zipSync(entries, { level: 6 }) as Uint8Array<ArrayBuffer>], { type: XLSX_TYPE });
}

/** A saved version, which is engine workbook data, as an .xlsx file. */
export function workbookXlsx(workbook: unknown): Blob {
	return xlsxBlob(new Model(workbook, { mode: 'readonly' }));
}
