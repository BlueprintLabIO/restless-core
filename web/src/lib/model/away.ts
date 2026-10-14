import { ownerJson } from './failure.ts';

/** The owner's away mode: until when, the admin who answers approvals, and what it paused. */
export interface Away {
	until: string;
	deputy?: string | null;
	paused_schedules: string[];
	since?: string | null;
}

const awayApi = (company: string) => `/api/companies/${encodeURIComponent(company)}/company/away`;

export async function fetchAway(company: string): Promise<Away | null> {
	const body = await ownerJson<{ away: Away | null }>(
		await fetch(awayApi(company), { credentials: 'same-origin', cache: 'no-store' })
	);
	return body.away;
}

export async function startAway(
	company: string,
	input: { until: string; deputy?: string; pause_schedules: string[] }
): Promise<Away | null> {
	const body = await ownerJson<{ away: Away | null }>(
		await fetch(awayApi(company), {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(input),
			credentials: 'same-origin'
		})
	);
	return body.away;
}

export async function endAway(company: string): Promise<void> {
	await ownerJson<{ away: null }>(
		await fetch(`${awayApi(company)}/end`, { method: 'POST', credentials: 'same-origin' })
	);
}
