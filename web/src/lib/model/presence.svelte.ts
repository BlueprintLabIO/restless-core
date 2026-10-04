/* Who else is in this company's cockpit. Each open cockpit heartbeats while
 * visible; presence is ephemeral and never stored. */
export type PresentPerson = { actor_id: string; display: string };

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
				if (!stopped) present = body.present ?? [];
			} catch {
				/* Presence is a nicety; the cockpit works without it. */
			}
		}
		void beat();
		const timer = setInterval(beat, 30_000);
		const onVisible = () => void beat();
		const onLeave = () => void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		document.addEventListener('visibilitychange', onVisible);
		window.addEventListener('pagehide', onLeave);
		return () => {
			stopped = true;
			clearInterval(timer);
			document.removeEventListener('visibilitychange', onVisible);
			window.removeEventListener('pagehide', onLeave);
			present = [];
			void fetch(url, { method: 'DELETE', keepalive: true }).catch(() => {});
		};
	});
	return {
		get present() {
			return present;
		}
	};
}
