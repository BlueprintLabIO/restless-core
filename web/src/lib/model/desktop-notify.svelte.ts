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

	/** Shows one notification when the cockpit is not in front. */
	show(title: string, body: string, onclick: () => void) {
		if (!this.active || document.visibilityState === 'visible') return;
		const notification = new Notification(title, { body, tag: title });
		notification.onclick = () => {
			window.focus();
			onclick();
			notification.close();
		};
	}
}

export const desktopNotify = new DesktopNotify();
