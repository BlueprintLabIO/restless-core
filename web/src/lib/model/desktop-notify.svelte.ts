/* Desktop notifications for this device: when something new needs the owner
 * and the cockpit is in the background, the browser taps them on the
 * shoulder. Opt-in per device, with quiet hours. */
const KEY = 'restless:desktop-notifications';

type Settings = { enabled: boolean; quietFrom: number; quietTo: number };

function read(): Settings {
	try {
		const stored = JSON.parse(localStorage.getItem(KEY) ?? 'null');
		if (stored && typeof stored.enabled === 'boolean') return stored;
	} catch {
		/* Fall through to the default. */
	}
	return { enabled: false, quietFrom: 22, quietTo: 7 };
}

class DesktopNotify {
	settings = $state<Settings>({ enabled: false, quietFrom: 22, quietTo: 7 });
	permission = $state<NotificationPermission | 'unsupported'>('default');

	load() {
		this.settings = read();
		this.permission = typeof Notification === 'undefined' ? 'unsupported' : Notification.permission;
	}

	get active() {
		return this.settings.enabled && this.permission === 'granted';
	}

	async toggle() {
		if (this.permission === 'unsupported') return;
		if (!this.settings.enabled && Notification.permission !== 'granted') {
			this.permission = await Notification.requestPermission();
			if (this.permission !== 'granted') return;
		}
		this.settings = { ...this.settings, enabled: !this.settings.enabled };
		try {
			localStorage.setItem(KEY, JSON.stringify(this.settings));
		} catch {
			/* The toggle still applies for this session. */
		}
	}

	quiet(now = new Date()) {
		const hour = now.getHours();
		const { quietFrom: from, quietTo: to } = this.settings;
		return from > to ? hour >= from || hour < to : hour >= from && hour < to;
	}

	/** Shows one notification when the cockpit is not in front and it is not quiet hours. */
	show(title: string, body: string, onclick: () => void) {
		if (!this.active || document.visibilityState === 'visible' || this.quiet()) return;
		const notification = new Notification(title, { body, tag: title });
		notification.onclick = () => {
			window.focus();
			onclick();
			notification.close();
		};
	}
}

export const desktopNotify = new DesktopNotify();
