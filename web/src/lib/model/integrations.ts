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
/* Incoming email (ADR 0017): replies reach Exec through the company's own Resend webhook, with a
 * poll behind it. Read from what Restless observed, never a live provider call on read. */
export type IncomingState = 'not_connected' | 'connecting' | 'connected' | 'polling' | 'failing';
export type IncomingStatus = {
	state: IncomingState;
	/** One line in the owner's words. */
	detail: string;
	/** What to do, when there is something to do. */
	action: string | null;
	domains: string[];
	registered_at: string | null;
	last_event_at: string | null;
	last_poll_at: string | null;
	check_sent_at: string | null;
	check_passed_at: string | null;
};
export type IntegrationsView = { email: EmailStatus; incoming?: IncomingStatus };

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

export function connectIncoming(company: string) {
	return ownerRequest<{ incoming: IncomingStatus }>(
		`${base(company)}/integrations/email/incoming/connect`,
		{ method: 'POST', json: {} }
	);
}

export function testIncoming(company: string) {
	return ownerRequest<{ incoming: IncomingStatus }>(
		`${base(company)}/integrations/email/incoming/test`,
		{ method: 'POST', json: {} }
	);
}

export const INCOMING_STATE: Record<
	IncomingState,
	{ label: string; tone: 'success' | 'warning' | 'danger' | 'muted' }
> = {
	not_connected: { label: 'Not connected', tone: 'muted' },
	connecting: { label: 'Connecting', tone: 'warning' },
	connected: { label: 'Connected', tone: 'success' },
	polling: { label: 'Connected', tone: 'success' },
	failing: { label: 'Needs attention', tone: 'danger' }
};

/** The owner's test, as one line: not run, on its way back, or back. */
export function checkLine(
	incoming: Pick<IncomingStatus, 'check_sent_at' | 'check_passed_at'>,
	relative: (at: string) => string
): string {
	if (!incoming.check_sent_at) return 'Not run yet';
	if (checkPassed(incoming)) return `Came back ${relative(incoming.check_passed_at as string)}`;
	return `Sent ${relative(incoming.check_sent_at)}, waiting for it to come back`;
}

/** Whether the owner's test is still on its way back. */
export function checkPending(
	incoming: Pick<IncomingStatus, 'check_sent_at' | 'check_passed_at'> | null | undefined,
	now = Date.now()
): boolean {
	if (!incoming?.check_sent_at) return false;
	if (checkPassed(incoming)) return false;
	return now - Date.parse(incoming.check_sent_at) < 5 * 60_000;
}

function checkPassed(incoming: Pick<IncomingStatus, 'check_sent_at' | 'check_passed_at'>) {
	return (
		!!incoming.check_sent_at &&
		!!incoming.check_passed_at &&
		Date.parse(incoming.check_passed_at) >= Date.parse(incoming.check_sent_at)
	);
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
