import { onReturn, untouched } from './activity.ts';

/* Who else is in this company's cockpit. Each open cockpit heartbeats while visible and in
 * use, and leaves as soon as it is hidden; presence is ephemeral and never stored. */
export type PresentPerson = { actor_id: string; display: string };

/* The shell's heartbeat fills this, so People and a conversation can say who is here now
 * without a second heartbeat. */
export const presentNow = $state<{ people: PresentPerson[] }>({ people: [] });

const PRESENCE_BEAT_MS = 60_000;
/* Nobody has touched the page for this long: they have stepped away, as a hidden tab has. */
const AWAY_AFTER_MS = 5 * 60_000;

export function presence(company: () => string) {
	let present = $state<PresentPerson[]>([]);
	$effect(() => {
		const target = company();
		if (!target) return;
		let stopped = false;
		const url = `/api/companies/${encodeURIComponent(target)}/presence`;
		async function beat() {
			if (stopped || document.visibilityState !== 'visible' || untouched(Date.now(), AWAY_AFTER_MS))
				return;
			try {
				const response = await fetch(url, { method: 'POST' });
				if (!response.ok) return;
				const body = (await response.json()) as { present: PresentPerson[] };
				if (!stopped) {
					present = body.present ?? [];
					presentNow.people = present;
				}
			} catch {
				/* Presence is a nicety; the cockpit works without it. */
			}
		}
		void beat();
		/* Once a minute: each beat is a request through the hosted edge. Leaving is said at once
		 * (below), and Core keeps someone present for 75 seconds after a beat (owner_presence.rs). */
		const timer = setInterval(beat, PRESENCE_BEAT_MS);
		/* Back at the page after stepping away: here again at once. */
		const stopReturn = onReturn(() => void beat());
		const onLeave = () => void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		/* A hidden tab is not someone looking: leave at once, and come back when seen again. */
		const onVisible = () => (document.visibilityState === 'visible' ? void beat() : onLeave());
		document.addEventListener('visibilitychange', onVisible);
		window.addEventListener('pagehide', onLeave);
		return () => {
			stopped = true;
			clearInterval(timer);
			stopReturn();
			document.removeEventListener('visibilitychange', onVisible);
			window.removeEventListener('pagehide', onLeave);
			present = [];
			presentNow.people = [];
			void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		};
	});
	return {
		get present() {
			return present;
		}
	};
}
