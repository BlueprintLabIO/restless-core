/* Files a person keeps in the company: ones sent in a conversation and ones added straight to the
 * Library (S64). Both are the same retained attachments, under the same per-person and company
 * limits; the gateway lists each person's own (the owner's, everyone's) and serves each one by id. */
import { responseFailure } from './failure.ts';

/** The most files one upload carries and the largest one file may be (the gateway's own caps). */
export const MAX_UPLOAD_FILES = 6;
export const MAX_UPLOAD_BYTES = 5 * 1024 * 1024;

export interface UploadedAttachment {
	id: string;
	name: string;
	media_type: string;
	size_bytes: number;
}

export interface RetainedAttachment {
	attachment_id: string;
	sender_actor_id: string;
	target_actor_id: string;
	canonical_name: string;
	canonical_media_type: string;
	size_bytes: number;
	/** Null for a file added straight to the Library. */
	message_id: number | null;
	content_sha256?: string;
	created_at: string;
	purge_requested_at: string | null;
	purged_at: string | null;
}

/** One row per file a person kept: the same bytes under the same name from the same sender (an
 * upload that also went out in a message, or one sent twice) list once, newest first. */
export function distinctKept(rows: RetainedAttachment[]): RetainedAttachment[] {
	const seen = new Set<string>();
	return rows.filter((row) => {
		if (!row.content_sha256) return true;
		const key = `${row.sender_actor_id}\n${row.canonical_name}\n${row.content_sha256}`;
		if (seen.has(key)) return false;
		seen.add(key);
		return true;
	});
}

export interface AttachmentUsage {
	retained_files: number;
	retained_bytes: number;
	purge_pending_files: number;
	purge_pending_bytes: number;
}

export interface AttachmentLimits {
	retained_files: number;
	retained_bytes: number;
	per_principal_files: number;
	per_principal_bytes: number;
}

export interface AttachmentInventory {
	attachments: RetainedAttachment[];
	usage: AttachmentUsage;
	limits: AttachmentLimits;
}

interface InventoryPage extends AttachmentInventory {
	next_before_created_at: string | null;
	next_before_attachment_id: string | null;
	has_more: boolean;
}

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}/attachments`;

/** Where one kept file is served. */
export function attachmentHref(company: string, id: string): string {
	return `${base(company)}/${encodeURIComponent(id)}`;
}

/** Every file still kept. The company holds at most a few hundred, so this reads them all. */
export async function listRetainedAttachments(
	company: string,
	signal?: AbortSignal
): Promise<AttachmentInventory> {
	const attachments: RetainedAttachment[] = [];
	let cursor: { created: string; id: string } | null = null;
	let last: InventoryPage | null = null;
	for (let pages = 0; pages < 12; pages++) {
		const query = new URLSearchParams({ limit: '100' });
		if (cursor) {
			query.set('before_created_at', cursor.created);
			query.set('before_attachment_id', cursor.id);
		}
		const response = await fetch(`${base(company)}?${query}`, {
			credentials: 'same-origin',
			cache: 'no-store',
			signal
		});
		if (!response.ok) throw await responseFailure(response);
		last = (await response.json()) as InventoryPage;
		attachments.push(...last.attachments);
		if (!last.has_more || !last.next_before_created_at || !last.next_before_attachment_id) break;
		cursor = { created: last.next_before_created_at, id: last.next_before_attachment_id };
	}
	return {
		attachments: attachments.filter((row) => !row.purge_requested_at && !row.purged_at),
		usage: last!.usage,
		limits: last!.limits
	};
}

/* fetch cannot report how much of an upload has gone, so the bytes go through XMLHttpRequest. A
 * lost connection rejects as fetch would, so failures read the same everywhere. */
export function postFiles(
	url: string,
	form: FormData,
	onSent: (bytes: number) => void
): Promise<Response> {
	return new Promise((resolve, reject) => {
		const request = new XMLHttpRequest();
		request.open('POST', url);
		request.withCredentials = true;
		request.upload.onprogress = (event) => onSent(event.loaded);
		request.onload = () =>
			request.status >= 200 && request.status <= 599
				? resolve(
						new Response(request.status === 204 ? null : request.responseText, {
							status: request.status,
							statusText: request.statusText,
							headers: { 'content-type': request.getResponseHeader('content-type') ?? '' }
						})
					)
				: reject(new TypeError('Failed to fetch'));
		request.onerror = () => reject(new TypeError('Failed to fetch'));
		request.onabort = () => reject(new DOMException('The upload was cancelled.', 'AbortError'));
		request.send(form);
	});
}

/** Add files to the company with no conversation message, six at a time. Each batch is whole or
 * nothing; a failure carries the files earlier batches already added as `added`. `onProgress`
 * hears the share of all the bytes sent so far, from 0 to 1. */
export async function uploadAttachments(
	company: string,
	files: readonly File[],
	onProgress?: (fraction: number) => void
): Promise<UploadedAttachment[]> {
	const added: UploadedAttachment[] = [];
	const bytes = (list: readonly File[]) => list.reduce((sum, file) => sum + file.size, 0);
	const total = Math.max(1, bytes(files));
	let done = 0;
	for (let start = 0; start < files.length; start += MAX_UPLOAD_FILES) {
		const batch = files.slice(start, start + MAX_UPLOAD_FILES);
		const form = new FormData();
		for (const file of batch) form.append('file', file, file.name);
		let response: Response;
		try {
			response = await postFiles(base(company), form, (sent) =>
				onProgress?.(Math.min(1, (done + sent) / total))
			);
		} catch (cause) {
			throw Object.assign(cause instanceof Error ? cause : new Error(String(cause)), { added });
		}
		if (!response.ok) throw Object.assign(await responseFailure(response), { added });
		added.push(...((await response.json()) as { attachments: UploadedAttachment[] }).attachments);
		done += bytes(batch);
		onProgress?.(Math.min(1, done / total));
	}
	return added;
}

/** Delete one kept file for good, wherever it was sent. */
export async function deleteAttachment(company: string, id: string): Promise<void> {
	const response = await fetch(attachmentHref(company, id), {
		method: 'DELETE',
		credentials: 'same-origin',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ reason: 'The owner deleted this file from the Library.' })
	});
	if (!response.ok) throw await responseFailure(response);
}

const RASTER = /^image\/(png|jpeg|gif|webp|avif)$/i;
/* No `+` suffix: a browser renders any `…+xml` type as a document. */
const MEDIA = /^(audio|video)\/[a-z0-9.-]+$/i;

/** How the Library can read a kept file in place, or null when it downloads instead. Only types a
 * browser shows without running anything qualify: raster images, PDFs, audio and video. */
export function attachmentReadKind(
	mediaType: string,
	name: string
): 'image' | 'pdf' | 'media' | null {
	const type = mediaType.split(';')[0].trim().toLowerCase();
	if (RASTER.test(type)) return 'image';
	if (type === 'application/pdf' || (type === 'application/octet-stream' && /\.pdf$/i.test(name)))
		return 'pdf';
	if (MEDIA.test(type)) return 'media';
	return null;
}

/** Read a kept file into this page as a typed local URL, so an image or PDF shows in place while
 * the gateway keeps serving every file as a download. The type is forced from the readable kinds,
 * never taken from the file, so nothing active ever runs here. The caller revokes it. */
export async function attachmentObjectUrl(
	company: string,
	id: string,
	mediaType: string,
	name: string,
	signal?: AbortSignal
): Promise<string> {
	const kind = attachmentReadKind(mediaType, name);
	if (!kind) throw new Error('This kind of file downloads rather than opening here.');
	const response = await fetch(attachmentHref(company, id), {
		credentials: 'same-origin',
		signal,
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	const type = kind === 'pdf' ? 'application/pdf' : mediaType.split(';')[0].trim().toLowerCase();
	return URL.createObjectURL(new Blob([await response.arrayBuffer()], { type }));
}

/** A size an owner reads: 840 KB, 3.2 MB. */
export function fileSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
	if (bytes < 1024 * 1024 * 1024)
		return `${(bytes / (1024 * 1024)).toFixed(1).replace(/\.0$/, '')} MB`;
	return `${(bytes / (1024 * 1024 * 1024)).toFixed(1).replace(/\.0$/, '')} GB`;
}

/** A picture found inside pasted content (a data: or web address), as a file to upload; null when
 * the source will not share it or it is not a picture a document shows. */
export async function pastedPictureFile(
	source: string,
	name: string,
	types: readonly string[]
): Promise<File | null> {
	try {
		const response = await fetch(source, { mode: 'cors', credentials: 'omit' });
		const blob = await response.blob();
		if (!response.ok || !types.includes(blob.type)) return null;
		return new File([blob], `${name}.${blob.type.split('/')[1]}`, { type: blob.type });
	} catch {
		return null;
	}
}
