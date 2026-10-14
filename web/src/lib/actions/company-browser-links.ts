import type { Action } from 'svelte/action';

const REVIEW_TICKET_HOST = /^[0-9a-f]{32}\.localhost$/;

type CompanyBrowserLinksOptions = {
	open: (url: string) => void;
	/** Whether this viewer chose the company browser; otherwise links open in a new tab here. */
	enabled: () => boolean;
};

/** Routes ordinary external web links through the Company browser when the viewer chose it, and
 * otherwise opens them in a new tab so the cockpit stays put. Native links keep their expected
 * behavior: modified clicks, downloads, mail, and local routes. */
export const companyBrowserLinks: Action<HTMLElement, CompanyBrowserLinksOptions> = (
	node,
	options
) => {
	function onclick(event: MouseEvent) {
		if (
			event.defaultPrevented ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.shiftKey ||
			event.altKey
		)
			return;
		const anchor = (event.target as Element | null)?.closest('a[href]') as HTMLAnchorElement | null;
		if (
			!anchor ||
			!node.contains(anchor) ||
			anchor.hasAttribute('download') ||
			anchor.dataset.openExternally !== undefined
		)
			return;
		let destination: URL;
		try {
			destination = new URL(anchor.href, window.location.href);
		} catch {
			return;
		}
		if (!/^https?:$/.test(destination.protocol) || destination.origin === window.location.origin)
			return;
		/* A file the cockpit opened for the owner lives on its own read-only ticket origin on this
		 * host (`<ticket>.localhost`); the company computer cannot reach it, so it opens here. */
		if (REVIEW_TICKET_HOST.test(destination.hostname)) return;
		event.preventDefault();
		if (options.enabled()) options.open(destination.href);
		else window.open(destination.href, '_blank', 'noopener,noreferrer');
	}
	node.addEventListener('click', onclick);
	return { destroy: () => node.removeEventListener('click', onclick) };
};
