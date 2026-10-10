/* Library's view of what Work produced on the company computer (S64-T5). Library is a projection:
 * these are the Work graph's available artifact references, sorted into the kinds an owner thinks
 * in. Nothing is copied; opening one asks the owner gateway for a read-only origin. */
import { responseFailure } from './failure.ts';

export type LibraryFileKind = 'site' | 'image' | 'deck' | 'pdf' | 'media' | 'text' | 'office';

export interface LibraryArtifact {
	id: string;
	kind: string;
	uri: string;
	label: string;
	note?: string;
	created_by: string;
	work_id: string | null;
	state: string;
	created_at: string;
}

export interface LibraryFile {
	id: string;
	kind: LibraryFileKind;
	label: string;
	uri: string;
	createdBy: string;
	workId: string | null;
	createdAt: string;
	/** The file name, for the row's second line. */
	name: string;
}

export const LIBRARY_KIND_LABEL: Record<LibraryFileKind, string> = {
	site: 'Sites',
	image: 'Images',
	deck: 'Decks',
	pdf: 'PDFs',
	media: 'Media',
	text: 'Notes',
	office: 'Office files'
};

const IMAGE = /\.(png|jpe?g|gif|webp|avif|svg|ico)$/i;
const MEDIA = /\.(mp4|m4v|webm|ogv|mp3|m4a|wav|oga|ogg|flac)$/i;
const TEXT = /\.(md|markdown|txt|csv|json)$/i;
const OFFICE = /\.(docx?|xlsx?|pptx?)$/i;
const DECK_KIND = /^(deck|slides?|presentation)$/i;
const DECK_NAME = /\b(deck|slides?|presentation)\b/i;

function fileName(uri: string): string {
	try {
		const url = new URL(uri);
		return url.host + (url.pathname === '/' ? '' : url.pathname);
	} catch {
		return uri.split('/').filter(Boolean).at(-1) ?? uri;
	}
}

/** Which kind a recorded artifact is, or null when the cockpit cannot show it. A deck is what Staff
 * declared as one (kind `deck`), or an HTML or PDF file named as one. */
export function libraryKind(
	artifact: Pick<LibraryArtifact, 'kind' | 'uri' | 'label'>
): LibraryFileKind | null {
	const uri = artifact.uri.trim();
	const path = uri.split(/[?#]/)[0];
	const declaredDeck =
		DECK_KIND.test(artifact.kind.trim()) || DECK_NAME.test(`${artifact.label} ${fileName(path)}`);
	if (/^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?(\/|$)/.test(uri))
		return declaredDeck ? 'deck' : 'site';
	if (!path.startsWith('/company/')) return null;
	if (/\.(html?)$/i.test(path)) return declaredDeck ? 'deck' : 'site';
	if (/\.pdf$/i.test(path)) return declaredDeck ? 'deck' : 'pdf';
	if (IMAGE.test(path)) return 'image';
	if (MEDIA.test(path)) return 'media';
	if (TEXT.test(path)) return 'text';
	if (OFFICE.test(path)) return /\.pptx?$/i.test(path) ? 'deck' : 'office';
	return null;
}

/** Whether a file of this kind downloads rather than opening in the cockpit. */
export function downloads(file: Pick<LibraryFile, 'uri'>): boolean {
	return OFFICE.test(file.uri.split(/[?#]/)[0]);
}

/** Every available artifact the cockpit can show, newest first, one per locator. */
export function libraryFiles(artifacts: readonly LibraryArtifact[]): LibraryFile[] {
	const seen = new Set<string>();
	const out: LibraryFile[] = [];
	for (const artifact of [...artifacts].sort(
		(a, b) => Date.parse(b.created_at) - Date.parse(a.created_at)
	)) {
		if (artifact.state !== 'available' || !artifact.work_id || seen.has(artifact.uri)) continue;
		const kind = libraryKind(artifact);
		if (!kind) continue;
		seen.add(artifact.uri);
		out.push({
			id: artifact.id,
			kind,
			label: artifact.label.trim() || fileName(artifact.uri),
			uri: artifact.uri,
			createdBy: artifact.created_by,
			workId: artifact.work_id,
			createdAt: artifact.created_at,
			name: fileName(artifact.uri)
		});
	}
	return out;
}

/** Ask the owner gateway for a read-only origin serving this file, probed just now. */
export async function openLibraryFile(company: string, artifactId: string): Promise<string> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/library/open`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ artifact_id: artifactId }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()).review_url;
}

/* Pins are a per-viewer convenience (S64 decision 1): browser storage, never company state. */
const pinKey = (company: string) => `restless:${company}:library-pins`;
export function readPins(company: string): string[] {
	try {
		const raw = JSON.parse(localStorage.getItem(pinKey(company)) ?? '[]');
		return Array.isArray(raw)
			? raw.filter((value): value is string => typeof value === 'string')
			: [];
	} catch {
		return [];
	}
}
export function writePins(company: string, pins: string[]) {
	try {
		localStorage.setItem(pinKey(company), JSON.stringify(pins));
	} catch {
		/* Pins still hold for this visit. */
	}
}
