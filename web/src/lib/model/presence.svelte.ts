/* Who else is in this company's cockpit. Each open cockpit heartbeats while
 * visible and leaves as soon as it is hidden; presence is ephemeral and never stored. */
export type PresentPerson = { actor_id: string; display: string };

/* The shell's heartbeat fills this, so People and a conversation can say who is here now
 * without a second heartbeat. */
export const presentNow = $state<{ people: PresentPerson[] }>({ people: [] });

export function presence(company: () => string) {
	let present = $state<PresentPerson[]>([]);
	$effect(() => {
		const target = company();
		if (!target) return;
		let stopped = false;
		const url = `/api/companies/${encodeURIComponent(target)}/presence`;
		async function beat() {
			if (stopped || document.visibilityState !== 'visible') return;
			try {
				const response = await fetch(url, { method: 'POST' });
				if (!response.ok) return;
				const body = (await response.json()) as { present: PresentPerson[] };
				if (!stopped) present = presentNow.people = body.present ?? [];
			} catch {
				/* Presence is a nicety; the cockpit works without it. */
			}
		}
		void beat();
		const timer = setInterval(beat, 30_000);
		const onLeave = () => void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		/* A hidden tab is not someone looking: leave at once, and come back when seen again. */
		const onVisible = () => (document.visibilityState === 'visible' ? void beat() : onLeave());
		document.addEventListener('visibilitychange', onVisible);
		window.addEventListener('pagehide', onLeave);
		return () => {
			stopped = true;
			clearInterval(timer);
			document.removeEventListener('visibilitychange', onVisible);
			window.removeEventListener('pagehide', onLeave);
			present = presentNow.people = [];
			void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		};
	});
	return {
		get present() {
			return present;
		}
	};
}
