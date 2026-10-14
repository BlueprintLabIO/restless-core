/* Library's view of what Work produced on the company computer (S64-T5). Library is a projection:
 * these are the Work graph's available artifact references, sorted into the kinds an owner thinks
 * in. Nothing is copied; opening one asks the owner gateway for a read-only origin. */
import { responseFailure } from './failure.ts';

export type LibraryFileKind =
	'site' | 'image' | 'deck' | 'pdf' | 'media' | 'text' | 'office' | 'other';

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
	/** Set when a newer output replaced this one; it leaves the default views. */
	superseded_at?: string | null;
}

export interface LibraryFile {
	id: string;
	/** Every record of this one file; a link to any of them opens it. */
	ids?: string[];
	kind: LibraryFileKind;
	label: string;
	/** Whether the label is one a recorder gave, not a fallback. */
	named?: boolean;
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
	office: 'Office files',
	other: 'Other files'
};

const IMAGE = /\.(png|jpe?g|gif|webp|avif|svg|ico)$/i;
const MEDIA = /\.(mp4|m4v|webm|ogv|mp3|m4a|wav|oga|ogg|flac)$/i;
const TEXT = /\.(md|markdown|txt|csv|tsv|json|log)$/i;
const OFFICE = /\.(docx?|xlsx?|pptx?)$/i;
/* Kinds the browser itself shows; everything else downloads (an Office file may be read in
 * place by the cockpit, see officeFormat). */
const VIEWABLE =
	/\.(html?|pdf|png|jpe?g|gif|webp|avif|svg|ico|mp4|m4v|webm|ogv|mp3|m4a|wav|oga|ogg|flac|md|markdown|txt|json)$/i;
const DECK_KIND = /^(deck|slides?|presentation)$/i;
const DECK_NAME = /\b(deck|slides?|presentation)\b/i;
/* Labels that name no particular file: recorders' defaults and the words for a kind. */
const GENERIC_LABEL =
	/^(outputs?|review([ _-]?target)?|evidence|artifact|file|result|deliverable|deck|site|page|index(\.html?)?|report|document)$/i;

/* A page whose title was its own address ("127.0.0.1:8650", "http://localhost:5173/"), or any
 * bare host and port, names nothing a person would recognise. */
const ADDRESS_LABEL =
	/^([a-z][a-z0-9+.-]*:\/\/\S*|(localhost|\d{1,3}(\.\d{1,3}){3}|\[[0-9a-f:]+\])(:\d+)?(\/\S*)?|[a-z0-9.-]+:\d{2,5}(\/\S*)?)$/i;
/** Whether a label names nothing in particular: a recorder's default, the word for a kind, or an
 * address. */
function namesNothing(label: string): boolean {
	return GENERIC_LABEL.test(label) || ADDRESS_LABEL.test(label);
}

function fileName(uri: string): string {
	try {
		const url = new URL(uri);
		return url.host + (url.pathname === '/' ? '' : url.pathname);
	} catch {
		return uri.split('/').filter(Boolean).at(-1) ?? uri;
	}
}

/** The last name in a path, decoded for reading. */
export function pathName(path: string): string {
	const name = path.split(/[?#]/)[0].split('/').filter(Boolean).at(-1) ?? path;
	try {
		return decodeURIComponent(name);
	} catch {
		return name;
	}
}

/** Which kind a recorded artifact is, or null when it is not a file or site in the company. A
 * deck is what Staff declared as one (kind `deck`), or an HTML or PDF file named as one, or sitting
 * in a folder named as one (the deck skill writes `…/deck/index.html`). Any other file in the
 * company home is still the owner's: it is listed, and downloads. */
export function libraryKind(
	artifact: Pick<LibraryArtifact, 'kind' | 'uri' | 'label'>
): LibraryFileKind | null {
	const uri = artifact.uri.trim();
	const path = uri.split(/[?#]/)[0];
	const declaredDeck =
		DECK_KIND.test(artifact.kind.trim()) ||
		DECK_NAME.test(`${artifact.label} ${fileName(path)}`) ||
		(path.startsWith('/company/') &&
			path
				.split('/')
				.slice(2, -1)
				.some((segment) => DECK_NAME.test(segment)));
	if (/^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?(\/|$)/.test(uri))
		return declaredDeck ? 'deck' : 'site';
	if (!path.startsWith('/company/')) return null;
	if (/\.(html?)$/i.test(path)) return declaredDeck ? 'deck' : 'site';
	if (/\.pdf$/i.test(path)) return declaredDeck ? 'deck' : 'pdf';
	if (IMAGE.test(path)) return 'image';
	if (MEDIA.test(path)) return 'media';
	if (TEXT.test(path)) return 'text';
	if (OFFICE.test(path)) return /\.pptx?$/i.test(path) ? 'deck' : 'office';
	/* A file names itself with an extension; a bare folder is not something to open. */
	if (/\.[a-z0-9]{1,10}$/i.test(pathName(path))) return 'other';
	return null;
}

/** Whether the browser cannot show this file, so it downloads (an Office file may still be read
 * in the cockpit; see officeFormat). */
export function downloads(file: Pick<LibraryFile, 'uri'>): boolean {
	const path = file.uri.split(/[?#]/)[0];
	return path.startsWith('/company/') && !VIEWABLE.test(path);
}

/** A Word, Excel or PowerPoint file the cockpit reads in place; other Office files only download. */
export function officeFormat(uri: string): 'docx' | 'xlsx' | 'pptx' | null {
	const match = uri.split(/[?#]/)[0].match(/\.(docx|xlsx|pptx)$/i);
	return match ? (match[1].toLowerCase() as 'docx' | 'xlsx' | 'pptx') : null;
}

/** What to call a file: the label its recorder gave, unless that names nothing in particular,
 * then its own file name, then the Work it came from. */
export function fileLabel(label: string, uri: string, workTitle?: string): string {
	const given = label.trim();
	if (given && !namesNothing(given)) return given;
	const path = uri.split(/[?#]/)[0];
	const name = path.startsWith('/company/') ? pathName(path) : fileName(uri);
	if (name && !namesNothing(name)) return name;
	/* A page is its folder: `…/q4-review/deck/index.html` is the Q4 review. */
	if (/^index\.html?$/i.test(name)) {
		const folder = path
			.split('/')
			.slice(2, -1)
			.reverse()
			.find(
				(segment) =>
					!/^(deck|slides|presentation|site|dist|build|public|out|www|outputs)$/i.test(segment)
			);
		if (folder) return pathName(folder);
	}
	return workTitle?.trim() || given || name || uri;
}

/** Every available artifact in the company, newest first, one per locator. Staff often record
 * one file twice (a bare `output` and the skill's named `deck`/`review_target`); those are one
 * row: the real name wins, it is a deck if any record says so, and every record's id still opens
 * it. Work titles name a file whose recorder gave it no real name. */
export function libraryFiles(
	artifacts: readonly LibraryArtifact[],
	works: readonly { id: string; title: string }[] = []
): LibraryFile[] {
	const titles = new Map(works.map((work) => [work.id, work.title]));
	const byUri = new Map<string, LibraryFile>();
	const out: LibraryFile[] = [];
	for (const artifact of [...artifacts].sort(
		(a, b) => Date.parse(b.created_at) - Date.parse(a.created_at)
	)) {
		if (artifact.state !== 'available' || artifact.superseded_at || !artifact.work_id) continue;
		const kind = libraryKind(artifact);
		if (!kind) continue;
		const named = artifact.label.trim() && !namesNothing(artifact.label.trim());
		const merged = byUri.get(artifact.uri);
		if (merged) {
			merged.ids?.push(artifact.id);
			if (kind === 'deck') merged.kind = 'deck';
			if (named && !merged.named) {
				merged.label = artifact.label.trim();
				merged.named = true;
			}
			continue;
		}
		const file: LibraryFile = {
			id: artifact.id,
			ids: [artifact.id],
			kind,
			label: fileLabel(artifact.label, artifact.uri, titles.get(artifact.work_id)),
			named: !!named,
			uri: artifact.uri,
			createdBy: artifact.created_by,
			workId: artifact.work_id,
			createdAt: artifact.created_at,
			name: fileName(artifact.uri)
		};
		byUri.set(artifact.uri, file);
		out.push(file);
	}
	return out;
}

/** The Library row a link names: by any recorded id of the file. */
export function libraryFileById(files: readonly LibraryFile[], id: string): LibraryFile | null {
	return files.find((file) => file.id === id || file.ids?.includes(id)) ?? null;
}

/** What a link to a file that is no longer listed was, and what took its place: the newest file
 * of the same kind from the same Work. A link in Exec's message to a site's first version should
 * lead to the version that replaced it, not to a dead end. */
export function replacedFile(
	artifacts: readonly LibraryArtifact[],
	files: readonly LibraryFile[],
	id: string,
	works: readonly { id: string; title: string }[] = []
): { gone: string; workTitle: string; replacement: LibraryFile | null } | null {
	const record = artifacts.find((artifact) => artifact.id === id);
	if (!record) return null;
	const kind = libraryKind(record);
	const workTitle = works.find((work) => work.id === record.work_id)?.title ?? '';
	const replacement =
		files
			.filter(
				(file) =>
					file.workId === record.work_id &&
					(file.kind === kind || (kind === 'site' && file.kind === 'deck'))
			)
			.toSorted((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))[0] ?? null;
	return { gone: fileLabel(record.label, record.uri, workTitle), workTitle, replacement };
}

/** A file in the company home that no recorded output names: a link in a message or document
 * pointed at it. It opens the same way, by its path. */
export function fileAtPath(path: string): LibraryFile | null {
	const kind = libraryKind({ kind: '', uri: path, label: '' });
	if (!kind) return null;
	const name = pathName(path);
	return {
		id: '',
		kind,
		label: name,
		uri: path,
		createdBy: '',
		workId: null,
		createdAt: '',
		name
	};
}

/** Where a file opens in the Library: by its recorded id, or by its path. */
export function libraryFileHref(company: string, file: { id?: string; uri: string }): string {
	const query = file.id
		? `file=${encodeURIComponent(file.id)}`
		: `path=${encodeURIComponent(file.uri)}`;
	return `/${encodeURIComponent(company)}/library?${query}`;
}

/** The same ticket, asking for the file itself under its own name. */
export function downloadHref(reviewUrl: string): string {
	const [base, hash] = reviewUrl.split('#', 2);
	const joined = `${base}${base.includes('?') ? '&' : '?'}download=1`;
	return hash === undefined ? joined : `${joined}#${hash}`;
}

/** Why a file did not open, in a sentence, with the gateway's own words kept as detail. */
export class LibraryOpenError extends Error {
	readonly sentence: string;
	readonly detail: string;
	constructor(sentence: string, detail: string) {
		super(sentence);
		this.sentence = sentence;
		this.detail = detail;
	}
}

/** An opened file: its read-only origin, and for a small text file its text, to read in place. */
export interface OpenedFile {
	url: string;
	text?: string;
	format?: 'markdown' | 'csv' | 'tsv' | 'json' | 'text';
}

/** Ask the owner gateway for a read-only origin serving this file, probed just now: a recorded
 * output by its id, or any file in the company home by its path. */
export async function openLibraryFile(
	company: string,
	file: { id?: string; uri: string }
): Promise<OpenedFile> {
	let response: Response;
	try {
		response = await fetch(`/api/companies/${encodeURIComponent(company)}/library/open`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(file.id ? { artifact_id: file.id } : { path: file.uri }),
			credentials: 'same-origin'
		});
	} catch (cause) {
		throw new LibraryOpenError(
			'The cockpit could not reach the company. Try again in a moment.',
			cause instanceof Error ? cause.message : String(cause)
		);
	}
	if (!response.ok) {
		const failure = await responseFailure(response);
		const said = failure.serverMessage ?? failure.message;
		/* The gateway's runtime and Library refusals are already written for the owner; a failed
		 * read of the file itself is not. */
		const sentence =
			response.status === 404 && file.id
				? 'This file is no longer one the company’s Work recorded.'
				: response.status === 409 ||
					  response.status === 413 ||
					  (response.status === 503 && /company computer/i.test(said))
					? said
					: response.status === 400
						? 'This is not a file in the company’s home folder.'
						: 'The file could not be read from the company computer. It may have been moved or deleted.';
		throw new LibraryOpenError(sentence, said);
	}
	const body = await response.json();
	return {
		url: body.review_url,
		...(typeof body.text === 'string' ? { text: body.text, format: body.format } : {})
	};
}

/** A Word or Excel file's bytes, for the cockpit to render in place. */
export async function libraryFileBytes(company: string, path: string): Promise<ArrayBuffer> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/library/content?path=${encodeURIComponent(path)}`,
		{ credentials: 'same-origin', cache: 'no-store' }
	);
	if (!response.ok) {
		const failure = await responseFailure(response);
		throw new LibraryOpenError(
			response.status === 413 ||
				(response.status === 503 && /company computer/i.test(failure.serverMessage ?? ''))
				? (failure.serverMessage ?? failure.message)
				: 'The file could not be read from the company computer.',
			failure.serverMessage ?? failure.message
		);
	}
	return response.arrayBuffer();
}

/* Pins are each person's own and follow them to every device (S65): the server keeps them per
 * person, newest first. A pin names an item; the Library shows it only while it can be read. */
const pinsPath = (company: string) => `/api/companies/${encodeURIComponent(company)}/library-pins`;
async function pinsAnswer(response: Response): Promise<string[]> {
	if (!response.ok) throw await responseFailure(response);
	const body = (await response.json()) as { pins?: unknown };
	return Array.isArray(body.pins)
		? body.pins.filter((value): value is string => typeof value === 'string')
		: [];
}
export async function fetchPins(company: string): Promise<string[]> {
	return pinsAnswer(
		await fetch(pinsPath(company), { credentials: 'same-origin', cache: 'no-store' })
	);
}
/** Pin or unpin one item; answers with this person's pins as they now stand. */
export async function setPin(company: string, key: string, pinned: boolean): Promise<string[]> {
	return pinsAnswer(
		await fetch(pinsPath(company), {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ key, pinned })
		})
	);
}
/* Pins made before they moved to the server lived in this browser. They are handed over once,
 * oldest first so the order holds, and the browser copy goes. */
const localPinKey = (company: string) => `restless:${company}:library-pins`;
export function takeLocalPins(company: string): string[] {
	try {
		const raw = JSON.parse(localStorage.getItem(localPinKey(company)) ?? '[]');
		return Array.isArray(raw)
			? raw.filter((value): value is string => typeof value === 'string')
			: [];
	} catch {
		return [];
	}
}
export function forgetLocalPins(company: string) {
	try {
		localStorage.removeItem(localPinKey(company));
	} catch {
		/* Nothing to hand over again; the server copy is the one read. */
	}
}

/** Work whose files have moved on to Archived: stopped Work. A `revises` edge runs from a reviewer
 * to the Work it may send back, not from a newer version, so it retires nothing; a replaced output
 * is marked superseded on the output itself (S85-12). */
export function retiredWorkIds(works: readonly { id: string; status: string }[]): Set<string> {
	return new Set(works.filter((work) => work.status === 'abandoned').map((work) => work.id));
}
