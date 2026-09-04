import { parsePlatformContext, type PlatformContext } from './contracts';

export async function getPlatformContext(signal?: AbortSignal): Promise<PlatformContext> {
	const response = await fetch('/api/platform', {
		credentials: 'same-origin',
		cache: 'no-store',
		signal
	});
	if (!response.ok) {
		throw Object.assign(new Error(`${response.status} ${response.statusText}`), {
			status: response.status
		});
	}
	return parsePlatformContext(await response.json());
}

export interface CreatedPlatformCompany {
	companyId: string;
	name: string;
}

/** Create one company through the deployment's server-owned platform API. */
export async function createPlatformCompany(name: string): Promise<CreatedPlatformCompany> {
	const response = await fetch('/api/platform/companies', {
		method: 'POST',
		credentials: 'same-origin',
		cache: 'no-store',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ name })
	});
	const document = (await response.json().catch(() => null)) as Record<string, unknown> | null;
	if (!response.ok) {
		throw Object.assign(
			new Error(
				typeof document?.message === 'string'
					? document.message
					: 'The company could not be created.'
			),
			{ status: response.status }
		);
	}
	if (
		!document ||
		typeof document.companyId !== 'string' ||
		!document.companyId ||
		typeof document.name !== 'string' ||
		!document.name
	) {
		throw new Error('The platform returned an invalid company record.');
	}
	return { companyId: document.companyId, name: document.name };
}

/**
 * Start a top-level company entry exchange.
 *
 * This deliberately is not `fetch`: the platform response is allowed to be a
 * no-store, auto-submitting form which carries a short-lived assertion to the
 * selected Core account-plane origin. Fetching that response would either
 * expose the assertion to application JavaScript or turn a cross-origin 303
 * into an API response instead of a browser navigation.
 */
export function enterPlatformCompany(companyId: string): void {
	if (typeof document === 'undefined') {
		throw new Error('Company entry requires a browser document.');
	}
	const form = document.createElement('form');
	form.method = 'POST';
	form.action = `/api/platform/companies/${encodeURIComponent(companyId)}/entry`;
	form.hidden = true;
	document.body.append(form);
	form.submit();
}
