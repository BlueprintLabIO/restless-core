/* Whether this person is emailed about what needs them in a company (Sprint 83): how the Inbox
 * reaches them with the cockpit closed. On until they turn it off; kept on the server, so it
 * holds on every device. */
import { ownerRequest } from './setting.svelte';

const url = (company: string) =>
	`/api/companies/${encodeURIComponent(company)}/notification-preferences`;

export async function readEmailNotify(company: string): Promise<boolean> {
	return (await ownerRequest<{ email: boolean }>(url(company))).email;
}

export async function setEmailNotify(company: string, email: boolean): Promise<boolean> {
	return (await ownerRequest<{ email: boolean }>(url(company), { method: 'PUT', json: { email } }))
		.email;
}
