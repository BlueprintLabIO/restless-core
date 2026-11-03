/* Where a picture an agent names in Markdown shows from (3 November 2026, Reno). Agents point at
 * company pictures several ways: a path relative to the document (`magnet/preview.png`), the
 * company path (`/company/outputs/magnet/preview.png`), a `file://` URL, the home folder (`~/…`),
 * a Library link copied from the cockpit, or the address of a preview on the company computer.
 * The cockpit opened none of them, so a send pack's pictures came up blank. Every company picture
 * now reads through one owner route; anything that cannot be shown says why, never a blank box. */

const COMPANY = '/company/';
/* Agents' shell home on the company computer. */
const HOME = '/company/home/';
const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '0.0.0.0', '[::1]']);

export type CompanyImage =
	/** A picture in the company's files, read through the owner route. */
	| { kind: 'company'; src: string; path: string }
	/** A picture on the public web, shown as it is. */
	| { kind: 'web'; src: string }
	/** A reference the owner's view cannot open, and why, in their words. */
	| { kind: 'unavailable'; reason: string; path?: string };

export interface ImageContext {
	/** The company whose files a path names. */
	company: string;
	/** The company path of the document the reference is written in, for relative paths. */
	source?: string;
	/** The cockpit's own origin, so a link copied from it reads as the file it names. */
	origin?: string;
}

/** The owner route a company picture reads through. */
export function companyImageSrc(company: string, path: string): string {
	return `/api/companies/${encodeURIComponent(company)}/library/image?path=${encodeURIComponent(path)}`;
}

/** A path with `.` and `..` worked out, if it stays inside the company home. */
function companyPath(path: string): string | null {
	const parts: string[] = [];
	for (const part of path.split('/')) {
		if (!part || part === '.') continue;
		if (part === '..') {
			if (!parts.length) return null;
			parts.pop();
		} else parts.push(part);
	}
	const joined = `/${parts.join('/')}`;
	return joined.startsWith(COMPANY) && parts.length > 1 ? joined : null;
}

function decoded(value: string): string {
	try {
		return decodeURIComponent(value);
	} catch {
		return value;
	}
}

function fromCompany(path: string, company: string): CompanyImage {
	const resolved = companyPath(path);
	if (!resolved)
		return { kind: 'unavailable', reason: 'This picture is not in the company’s files.', path };
	if (!company)
		return {
			kind: 'unavailable',
			reason: 'This picture opens inside its company.',
			path: resolved
		};
	/* The route serves raster pictures only (an SVG can carry script); the Library opens it apart. */
	if (/\.svgz?$/i.test(resolved))
		return {
			kind: 'unavailable',
			reason: 'This drawing opens in the Library, not in place.',
			path: resolved
		};
	return { kind: 'company', src: companyImageSrc(company, resolved), path: resolved };
}

/** Resolve one Markdown image reference to where the owner's view shows it from. */
export function resolveCompanyImage(href: string | undefined, context: ImageContext): CompanyImage {
	const raw = (href ?? '').trim().replace(/^<|>$/g, '');
	if (!raw) return { kind: 'unavailable', reason: 'This picture names no file.' };
	const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(raw)?.[1]?.toLowerCase();
	if (scheme === 'file') {
		let path: string;
		try {
			path = decoded(new URL(raw).pathname);
		} catch {
			path = decoded(raw.replace(/^file:\/\/[^/]*/i, ''));
		}
		return fromCompany(path, context.company);
	}
	if (scheme === 'http' || scheme === 'https') {
		let url: URL;
		try {
			url = new URL(raw);
		} catch {
			return { kind: 'unavailable', reason: 'This picture’s address is not valid.' };
		}
		if (LOCAL_HOSTS.has(url.hostname) || url.hostname.endsWith('.localhost'))
			return {
				kind: 'unavailable',
				reason: 'This picture is on a preview only the company computer can open.'
			};
		if (context.origin && url.origin === context.origin) return fromCockpit(url, context);
		return { kind: 'web', src: url.href };
	}
	if (scheme === 'data' && /^data:image\/(png|jpeg|gif|webp);base64,/i.test(raw))
		return { kind: 'web', src: raw };
	if (scheme)
		return { kind: 'unavailable', reason: 'This picture’s address cannot be shown here.' };
	const path = decoded(raw.split(/[?#]/)[0]);
	if (path.startsWith('~/')) return fromCompany(`${HOME}${path.slice(2)}`, context.company);
	if (path.startsWith('/')) {
		if (path.startsWith(COMPANY)) return fromCompany(path, context.company);
		return fromCockpit(new URL(raw, 'http://cockpit.invalid'), context);
	}
	/* Relative: beside the document it is written in, or the company home in a message. */
	const folder = context.source?.startsWith(COMPANY)
		? context.source.slice(0, context.source.lastIndexOf('/') + 1)
		: COMPANY;
	return fromCompany(`${folder}${path}`, context.company);
}

/* A Library link copied from the cockpit (`/<company>/library?path=…`) names the file it opens. */
function fromCockpit(url: URL, context: ImageContext): CompanyImage {
	if (url.pathname.startsWith(COMPANY)) return fromCompany(decoded(url.pathname), context.company);
	const named = url.searchParams.get('path');
	if (/^\/[^/]+\/library\/?$/.test(url.pathname) && named?.startsWith(COMPANY))
		return fromCompany(named, context.company);
	return {
		kind: 'unavailable',
		reason: 'This picture is not in the company’s files.',
		path: decoded(url.pathname)
	};
}
