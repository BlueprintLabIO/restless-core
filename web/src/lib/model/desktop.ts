export interface DesktopWindow {
	id: string;
	title: string;
	app: string;
	active: boolean;
	geometry: { x: number; y: number; width: number; height: number };
}

async function desktopRequest(company: string, path: string, body?: unknown) {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/desktop/windows${path}`,
		{
			method: body === undefined ? 'GET' : 'POST',
			credentials: 'same-origin',
			...(body === undefined
				? {}
				: {
						headers: { 'content-type': 'application/json' },
						body: JSON.stringify(body)
					})
		}
	);
	const result = await response.json();
	if (!response.ok)
		throw new Error(result.message ?? 'The company applications could not be reached.');
	return result;
}

export async function desktopWindows(company: string): Promise<DesktopWindow[]> {
	return (await desktopRequest(company, '')).windows;
}

export async function focusDesktopWindow(
	company: string,
	id: string,
	clientId: string,
	leaseId: string
) {
	await desktopRequest(company, `/${encodeURIComponent(id)}/focus`, {
		client_id: clientId,
		lease_id: leaseId
	});
}
