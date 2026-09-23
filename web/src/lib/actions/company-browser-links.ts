import type { Action } from 'svelte/action';

type CompanyBrowserLinksOptions = {
	open: (url: string) => void;
};

/** Routes ordinary external web links through the Company browser. Native links
 * keep their expected behavior: modified clicks, downloads, mail, and local routes. */
export const companyBrowserLinks: Action<HTMLElement, CompanyBrowserLinksOptions> = (node, options) => {
	function onclick(event: MouseEvent) {
		if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)
			return;
		const anchor = (event.target as Element | null)?.closest('a[href]') as HTMLAnchorElement | null;
		if (!anchor || !node.contains(anchor) || anchor.hasAttribute('download') || anchor.dataset.openExternally !== undefined)
			return;
		let destination: URL;
		try {
			destination = new URL(anchor.href, window.location.href);
		} catch {
			return;
		}
		if (!/^https?:$/.test(destination.protocol) || destination.origin === window.location.origin) return;
		event.preventDefault();
		options.open(destination.href);
	}
	node.addEventListener('click', onclick);
	return { destroy: () => node.removeEventListener('click', onclick) };
};
