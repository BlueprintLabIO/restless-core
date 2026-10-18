/* What a Library upload is made of: files picked or dropped, the files inside a dropped or
 * picked folder, and the files inside a zip. Each keeps the folder or zip it came from, which
 * becomes its Library collection. A zip opens here in the browser, inside the same limits a
 * single upload has, so nothing it holds can be larger than what the owner could send by hand. */
import { unzip, type UnzipFileInfo } from 'fflate';
import { MAX_UPLOAD_BYTES } from './attachments';

export interface UploadItem {
	file: File;
	/** The folder or zip it came from, used as its collection; null for a loose file. */
	collection: string | null;
}

/** The most files one zip may open into, and the most its contents may weigh. */
export const MAX_ZIP_FILES = 500;
export const MAX_ZIP_BYTES = 256 * 1024 * 1024;

const SKIPPED = /(^|\/)(__MACOSX|\.DS_Store|Thumbs\.db|desktop\.ini)(\/|$)|(^|\/)\._/;

const isZip = (file: File) =>
	/\.zip$/i.test(file.name) ||
	file.type === 'application/zip' ||
	file.type === 'application/x-zip-compressed';

const topFolder = (path: string) => {
	const parts = path.split('/').filter(Boolean);
	return parts.length > 1 ? parts[0] : null;
};

/** Files picked with the folder picker carry their path inside it. */
export function itemsFromPicked(files: readonly File[]): UploadItem[] {
	return files
		.filter((file) => !SKIPPED.test(file.webkitRelativePath || file.name))
		.map((file) => ({ file, collection: topFolder(file.webkitRelativePath || '') }));
}

/** The dropped entries, read before the drop event ends; folders are walked here. */
export function droppedEntries(transfer: DataTransfer | null): FileSystemEntry[] {
	if (!transfer) return [];
	return [...transfer.items]
		.map((item) => (item.kind === 'file' ? item.webkitGetAsEntry?.() : null))
		.filter((entry): entry is FileSystemEntry => !!entry);
}

/** Every file under the dropped entries, a dropped folder's files carrying its name. */
export async function itemsFromEntries(entries: readonly FileSystemEntry[]): Promise<UploadItem[]> {
	const items: UploadItem[] = [];
	const fileOf = (entry: FileSystemFileEntry) =>
		new Promise<File>((resolve, reject) => entry.file(resolve, reject));
	const children = async (directory: FileSystemDirectoryEntry) => {
		const reader = directory.createReader();
		const all: FileSystemEntry[] = [];
		// readEntries answers in pages; an empty page is the end.
		for (;;) {
			const page = await new Promise<FileSystemEntry[]>((resolve, reject) =>
				reader.readEntries(resolve, reject)
			);
			if (!page.length) return all;
			all.push(...page);
		}
	};
	const walk = async (entry: FileSystemEntry, collection: string | null) => {
		if (SKIPPED.test(entry.fullPath)) return;
		if (entry.isFile) {
			items.push({ file: await fileOf(entry as FileSystemFileEntry), collection });
		} else if (entry.isDirectory) {
			for (const child of await children(entry as FileSystemDirectoryEntry))
				await walk(child, collection ?? entry.name);
		}
	};
	for (const entry of entries) await walk(entry, null);
	return items;
}

function openZip(file: File): Promise<{ files: File[]; tooLarge: string[] }> {
	return file.arrayBuffer().then(
		(buffer) =>
			new Promise((resolve, reject) => {
				let count = 0;
				let total = 0;
				const tooLarge: string[] = [];
				unzip(
					new Uint8Array(buffer),
					{
						// The central directory's sizes decide what is opened at all.
						filter: (info: UnzipFileInfo) => {
							if (info.name.endsWith('/') || SKIPPED.test(info.name)) return false;
							if (info.originalSize > MAX_UPLOAD_BYTES) {
								tooLarge.push(info.name.split('/').pop() ?? info.name);
								return false;
							}
							count += 1;
							total += info.originalSize;
							return count <= MAX_ZIP_FILES && total <= MAX_ZIP_BYTES;
						}
					},
					(error, entries) => {
						if (error) return reject(error);
						if (count > MAX_ZIP_FILES || total > MAX_ZIP_BYTES)
							return reject(
								new Error(
									`“${file.name}” holds more than ${MAX_ZIP_FILES} files or ${MAX_ZIP_BYTES / (1024 * 1024)} MB.`
								)
							);
						resolve({
							files: Object.entries(entries).map(
								([path, bytes]) =>
									new File([bytes as Uint8Array<ArrayBuffer>], path.split('/').pop() ?? path)
							),
							tooLarge
						});
					}
				);
			})
	);
}

/** Zips open into their files, named for the zip; a zip that will not open is sent as it is. */
export async function expandZips(
	items: readonly UploadItem[]
): Promise<{ items: UploadItem[]; notes: string[] }> {
	const out: UploadItem[] = [];
	const notes: string[] = [];
	for (const item of items) {
		if (!isZip(item.file)) {
			out.push(item);
			continue;
		}
		try {
			const { files, tooLarge } = await openZip(item.file);
			const collection = item.collection ?? item.file.name.replace(/\.zip$/i, '');
			for (const file of files) out.push({ file, collection });
			if (tooLarge.length)
				notes.push(
					`${tooLarge.length === 1 ? `“${tooLarge[0]}”` : `${tooLarge.length} files`} in “${item.file.name}” ${tooLarge.length === 1 ? 'is' : 'are'} over ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB and stayed in the zip.`
				);
		} catch (cause) {
			out.push(item);
			notes.push(
				cause instanceof Error && cause.message.startsWith('“')
					? `${cause.message} It was added as one file.`
					: `“${item.file.name}” could not be opened, so it was added as one file.`
			);
		}
	}
	return { items: out, notes };
}
