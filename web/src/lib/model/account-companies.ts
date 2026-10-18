/* On the one address (Cloud), a plane's session covers only the company it entered, so the company
 * switcher asks the account for the rest. Fleet answers `/account/companies.json` on the same
 * origin. A local appliance has no such account, so it is not asked. */
export interface AccountCompany {
	id: string;
	name: string;
	decisions_waiting: number;
}

const LOOPBACK = new Set(['127.0.0.1', 'localhost', '[::1]']);

export async function readAccountCompanies(signal?: AbortSignal): Promise<AccountCompany[]> {
	if (typeof location === 'undefined' || LOOPBACK.has(location.hostname)) return [];
	try {
		const response = await fetch('/account/companies.json', {
			credentials: 'same-origin',
			headers: { accept: 'application/json' },
			signal
		});
		if (!response.ok || !(response.headers.get('content-type') ?? '').includes('json')) return [];
		const body = (await response.json()) as { companies?: unknown };
		return Array.isArray(body.companies)
			? body.companies.filter(
					(company): company is AccountCompany =>
						typeof company?.id === 'string' &&
						typeof company?.name === 'string' &&
						typeof company?.decisions_waiting === 'number'
				)
			: [];
	} catch {
		return [];
	}
}
