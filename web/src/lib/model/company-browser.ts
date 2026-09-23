import { browserTabClientId } from './browserTab';

type BrowserIntent = { url: string };

export const COMPANY_BROWSER_OPEN_EVENT = 'restless:company-browser-open';

const intentKey = (company: string) => `restless.company-browser-intent.${company}`;
const destinationKey = (company: string) => `restless.company-browser-destination.${company}`;

/** Keep the destination out of the application URL and history. It is consumed
 * once by the Company computer page, which is the only place that opens it. */
export function prepareCompanyBrowser(company: string, url: string): void {
	const intent = { url } satisfies BrowserIntent;
	sessionStorage.setItem(intentKey(company), JSON.stringify(intent));
	window.dispatchEvent(new CustomEvent<BrowserIntent>(COMPANY_BROWSER_OPEN_EVENT, { detail: intent }));
}

export function consumeCompanyBrowserIntent(company: string): BrowserIntent | null {
	const key = intentKey(company);
	const raw = sessionStorage.getItem(key);
	sessionStorage.removeItem(key);
	if (!raw) return null;
	try {
		const intent = JSON.parse(raw) as Partial<BrowserIntent>;
		const parsed = typeof intent.url === 'string' ? new URL(intent.url) : null;
		return parsed && /^https?:$/.test(parsed.protocol) ? { url: parsed.href } : null;
	} catch {
		return null;
	}
}

export function rememberCompanyBrowserDestination(company: string, url: string): void {
	sessionStorage.setItem(destinationKey(company), url);
}

export function lastCompanyBrowserDestination(company: string): string {
	const url = sessionStorage.getItem(destinationKey(company));
	try {
		return url && /^https?:$/.test(new URL(url).protocol) ? url : '';
	} catch {
		return '';
	}
}

export async function openCompanyBrowser(company: string, url: string): Promise<string> {
	const clientId = await browserTabClientId(company);
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/browser/open`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ url, client_id: clientId }),
		credentials: 'same-origin'
	});
	if (!response.ok) throw await browserOpenError(response);
	const body = (await response.json()) as { desktop_url?: unknown };
	if (typeof body.desktop_url !== 'string' || !body.desktop_url) {
		throw new Error('The Company browser did not return an observation session.');
	}
	return body.desktop_url;
}

export function companyBrowserLabel(url: string): string {
	try {
		const parsed = new URL(url);
		return `${parsed.hostname}${parsed.pathname === '/' ? '' : parsed.pathname}`;
	} catch {
		return 'External page';
	}
}

async function browserOpenError(response: Response): Promise<Error & { status: number }> {
	let message = `${response.status} ${response.statusText}`;
	try {
		const body = (await response.json()) as { message?: unknown };
		if (typeof body.message === 'string') message = body.message;
	} catch {
		// Keep the HTTP result useful if a proxy supplied a non-JSON response.
	}
	if (response.status === 409) {
		message = 'The Company computer is under control. Return control before opening this link here.';
	}
	return Object.assign(new Error(message), { status: response.status });
}
