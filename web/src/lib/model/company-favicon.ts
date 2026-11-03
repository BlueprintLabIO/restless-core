/* Inside a company the browser tab shows that company's mark, as its card, the account rail and
 * the top bar do (3 Nov walkthrough): an uploaded logo, else its pixel figure drawn the way the
 * product's own favicon draws the R, dark cells on a light tile. Leaving the company puts the
 * product's icon back. */
import { companyMarkRows } from '$lib/ui/portfolio';

/** The figure as a favicon: the 5x7 cells on the same 64px tile as static/favicon.svg. */
export function companyFaviconSvg(id: string): string {
	const cells = companyMarkRows(id)
		.flatMap((row, y) =>
			[...row].flatMap((cell, x) =>
				cell === '1'
					? [
							`<rect x="${(14.9 + x * 7).toFixed(1)}" y="${(7.9 + y * 7).toFixed(1)}" width="6.2" height="6.2" rx="1.3"/>`
						]
					: []
			)
		)
		.join('');
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect x="1" y="1" width="62" height="62" rx="14" fill="#f4f5f6" stroke="#c9ccd1" stroke-width="2"/><g fill="#0a0b0d">${cells}</g></svg>`;
}

/** Point the tab's icons at this company's mark until the returned cleanup runs. */
export function keepCompanyFavicon(id: string, logoUrl?: string | null): () => void {
	const links = [...document.head.querySelectorAll<HTMLLinkElement>('link[rel="icon"]')];
	if (!id || !links.length) return () => {};
	const before = links.map((link) => [link.href, link.type, link.getAttribute('sizes')] as const);
	const href = logoUrl
		? logoUrl
		: `data:image/svg+xml,${encodeURIComponent(companyFaviconSvg(id))}`;
	for (const link of links) {
		link.href = href;
		if (logoUrl) link.removeAttribute('type');
		else link.type = 'image/svg+xml';
		link.removeAttribute('sizes');
	}
	return () => {
		links.forEach((link, index) => {
			const [was, type, sizes] = before[index];
			link.href = was;
			if (type) link.type = type;
			if (sizes) link.setAttribute('sizes', sizes);
		});
	};
}
