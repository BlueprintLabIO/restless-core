/** Native surfaces an Inbox item can bring into its middle pane. Navigation to a list or
 * another company keeps its ordinary meaning. No arbitrary page is framed as application UI. */
export type InboxPreviewTarget =
	| { kind: 'file' | 'path' | 'upload' | 'document' | 'sheet'; value: string; href: string }
	| {
			kind: 'limits' | 'identity' | 'charter' | 'provider' | 'notifications';
			value: string;
			href: string;
	  };

export function inboxPreviewTarget(
	company: string,
	href: string | URL,
	origin: string
): InboxPreviewTarget | null {
	let url: URL;
	try {
		url = new URL(href, origin);
	} catch {
		return null;
	}
	if (url.origin !== origin || url.username || url.password) return null;
	const base = '/' + encodeURIComponent(company),
		path = url.pathname;
	const targetHref = path + url.search + url.hash;
	const target = (kind: InboxPreviewTarget['kind'], value: string): InboxPreviewTarget => ({
		kind,
		value,
		href: targetHref
	});
	if (path === base + '/library') {
		const keys = ['file', 'path', 'upload'] as const;
		const selected = keys.filter((key) => url.searchParams.get(key));
		if (selected.length !== 1) return null;
		const kind = selected[0],
			value = url.searchParams.get(kind)!;
		if (kind === 'path' && !value.startsWith('/company/')) return null;
		return target(kind, value);
	}
	for (const [suffix, key] of [
		['/library/documents', 'document'],
		['/work/documents', 'document'],
		['/library/sheets', 'sheet'],
		['/work/sheets', 'sheet']
	] as const) {
		if (path === base + suffix && url.searchParams.get(key))
			return target(key, url.searchParams.get(key)!);
	}
	const legacy = '/documents/' + encodeURIComponent(company) + '/';
	if (
		path.startsWith(legacy) &&
		path.length > legacy.length &&
		!path.slice(legacy.length).includes('/')
	) {
		try {
			const id = decodeURIComponent(path.slice(legacy.length));
			const canonical = new URL(base + '/library/documents' + url.search, origin);
			canonical.searchParams.set('document', id);
			return {
				kind: 'document',
				value: id,
				href: canonical.pathname + canonical.search + url.hash
			};
		} catch {
			return null;
		}
	}
	const settings = {
		resources: 'limits',
		identity: 'identity',
		charter: 'charter',
		provider: 'provider',
		notifications: 'notifications'
	} as const;
	for (const [suffix, kind] of Object.entries(settings))
		if (path === base + '/company/' + suffix) return target(kind, url.hash.slice(1));
	return null;
}

export function inboxPreviewHref(current: URL, href: string): string {
	const url = new URL(current);
	url.searchParams.set('preview', href);
	return url.pathname + url.search + url.hash;
}
export function inboxPreviewBack(current: URL): string {
	const url = new URL(current);
	url.searchParams.delete('preview');
	return url.pathname + url.search + url.hash;
}

/** A message about an embedded surface links to that surface, while the attention/work
 * identifiers independently retain the decision it belongs to. Only the exact Inbox can
 * wrap a preview; other routes and invalid/external wrappers keep their own context. */
export function inboxPreviewContext(
	company: string,
	location: Pick<URL, 'pathname' | 'search'> & Partial<Pick<URL, 'origin'>>
): InboxPreviewTarget | null {
	if (location.pathname !== '/' + encodeURIComponent(company)) return null;
	const href = new URLSearchParams(location.search).get('preview');
	return href
		? inboxPreviewTarget(company, href, location.origin ?? 'http://restless.local')
		: null;
}
