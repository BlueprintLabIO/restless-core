/* Desktop notifications for this device: when something new needs the person
 * and the cockpit is in the background, the browser taps them on the
 * shoulder. Opt-in per device. Schedules belong to the system's own Focus or
 * Do Not Disturb, which already silences every app at night; a second, fixed
 * set of quiet hours here only hid notices people asked for. */
import { toast } from '$lib/ui/feedback/toast.svelte';

const KEY = 'restless:desktop-notifications';

type Settings = { enabled: boolean };

function read(): Settings {
	try {
		const stored = JSON.parse(localStorage.getItem(KEY) ?? 'null');
		if (stored && typeof stored.enabled === 'boolean') return { enabled: stored.enabled };
	} catch {
		/* Fall through to the default. */
	}
	return { enabled: false };
}

/** What to tell someone whose browser will not show notifications for this site. */
export const BLOCKED_HINT =
	'Notifications are blocked for this site in your browser settings. Allow them there, then turn this on.';

class DesktopNotify {
	settings = $state<Settings>({ enabled: false });
	permission = $state<NotificationPermission | 'unsupported'>('default');

	load() {
		this.settings = read();
		this.permission = typeof Notification === 'undefined' ? 'unsupported' : Notification.permission;
	}

	get active() {
		return this.settings.enabled && this.permission === 'granted';
	}

	get blocked() {
		return this.permission === 'denied';
	}

	async toggle() {
		if (this.permission === 'unsupported') return;
		if (!this.settings.enabled && Notification.permission !== 'granted') {
			this.permission = await Notification.requestPermission();
			if (this.permission !== 'granted') {
				toast(
					this.permission === 'denied'
						? BLOCKED_HINT
						: 'Notifications were not allowed. Try again.',
					{ tone: 'failure' }
				);
				return;
			}
		}
		this.settings = { ...this.settings, enabled: !this.settings.enabled };
		try {
			localStorage.setItem(KEY, JSON.stringify(this.settings));
		} catch {
			/* The toggle still applies for this session. */
		}
	}

	/**
	 * Shows one notification when the cockpit is not in front. `company` names the company it is
	 * about, so an owner of several can tell them apart; `tag` identifies the thing itself, so two
	 * items that share a title do not replace each other while a repeat of one does.
	 */
	show(
		title: string,
		body: string,
		onclick: () => void,
		about: { company?: string; tag?: string } = {}
	) {
		if (!this.active || document.visibilityState === 'visible') return;
		const heading = about.company ? `${about.company}: ${title}` : title;
		const notification = new Notification(heading, { body, tag: about.tag ?? heading });
		notification.onclick = () => {
			window.focus();
			onclick();
			notification.close();
		};
	}
}

export const desktopNotify = new DesktopNotify();

type PortfolioCompany = {
	id: string;
	name: string;
	lifecycle_status: string;
	card?: { decisions_waiting: number };
};
let knownWaiting: Map<string, number> | null = null;

/**
 * Every other company the owner runs taps them on the shoulder too: the portfolio list carries
 * each company's waiting decisions, so a rise in one that is not open in this tab is said, named.
 * The open company speaks for itself, item by item. The first reading only primes.
 */
export function notifyPortfolio(
	companies: PortfolioCompany[],
	openCompany: string,
	open: (companyId: string) => void
) {
	const next = new Map<string, number>();
	for (const company of companies) {
		if (company.lifecycle_status !== 'active' || !company.card) continue;
		const waiting = company.card.decisions_waiting;
		next.set(company.id, waiting);
		const before = knownWaiting?.get(company.id);
		if (before === undefined || waiting <= before || company.id === openCompany) continue;
		desktopNotify.show(
			waiting === 1 ? '1 decision is waiting' : `${waiting} decisions are waiting`,
			'Open the company to decide.',
			() => open(company.id),
			{ company: company.name, tag: `company:${company.id}:waiting` }
		);
	}
	if (companies.length) knownWaiting = next;
}
