import { ownerRequest } from './setting.svelte';

/* Company › Integrations (S109 H2): one status per integration, in the same words
 * `restless capabilities` gives the agents. */
export type EmailState = 'not_connected' | 'key_only' | 'ready' | 'failing';
export type LastSend = {
	at: string;
	outcome: 'accepted' | 'refused' | 'unknown';
	detail: string | null;
	test: boolean;
};
export type EmailStatus = {
	state: EmailState;
	/** The Vault name holding the key; null without one, and always null for a member. */
	key: string | null;
	sender: string | null;
	last_send: LastSend | null;
	waiting_for_owner: number;
	mandate_daily: number | null;
	sent_today: number | null;
	action: string | null;
};
export type IntegrationsView = { email: EmailStatus };

const base = (company: string) => '/api/companies/' + encodeURIComponent(company);
export const integrationsKey = (company: string) => ['integrations', company] as const;

export function readIntegrations(company: string, signal?: AbortSignal) {
	return ownerRequest<IntegrationsView>(`${base(company)}/integrations`, { signal });
}

export function sendEmailTest(company: string, body: { from: string; to: string }) {
	return ownerRequest<{ last_send: LastSend }>(`${base(company)}/integrations/email/test`, {
		method: 'POST',
		json: body
	});
}

export const EMAIL_STATE: Record<
	EmailState,
	{ label: string; tone: 'success' | 'warning' | 'danger' | 'muted' }
> = {
	not_connected: { label: 'Not connected', tone: 'muted' },
	key_only: { label: 'Not tested', tone: 'warning' },
	ready: { label: 'Ready', tone: 'success' },
	failing: { label: 'Not sending', tone: 'danger' }
};

/** The latest email, as one line: when, and whether Resend took it. Why it refused is the
 * status line's to say. */
export function lastSendLine(send: LastSend | null, relative: (at: string) => string): string {
	if (!send) return 'Nothing sent yet';
	const what = send.test ? 'Test' : 'Email';
	if (send.outcome === 'accepted') return `${what} accepted by Resend ${relative(send.at)}`;
	if (send.outcome === 'refused') return `${what} refused by Resend ${relative(send.at)}`;
	return `${what} outcome unknown, ${relative(send.at)}`;
}

/** Who may send, as one line, to the owner or to someone else in the company. */
export function whoSends(
	email: Pick<EmailStatus, 'mandate_daily' | 'sent_today'>,
	owner: boolean
): string {
	if (email.mandate_daily === null)
		return owner
			? 'Each email waits for your approval in the Inbox'
			: 'Each email waits for the owner’s approval';
	return `Exec, up to ${email.mandate_daily} a day under the mandate (${email.sent_today ?? 0} today)`;
}
