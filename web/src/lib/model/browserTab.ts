/** One controller identity per browsing tab. Session storage keeps it stable
 * through a full reload. BroadcastChannel closes the one browser edge case:
 * a duplicated tab may receive a copy of session storage, but it must not
 * inherit the original tab's valid controller lease. */
const claims = new Map<string, Promise<string>>();

export function browserTabClientId(company: string): Promise<string> {
	let claim = claims.get(company);
	if (!claim) {
		claim = claimIdentity(company);
		claims.set(company, claim);
	}
	return claim;
}

async function claimIdentity(company: string): Promise<string> {
	const key = `restless.browser-tab.${company}`;
	let id = sessionStorage.getItem(key) ?? crypto.randomUUID();
	const channel = new BroadcastChannel(`restless.browser-tab.${company}`);
	const probe = crypto.randomUUID();
	let collision = false;

	channel.onmessage = (event: MessageEvent<{ type?: string; id?: string; probe?: string }>) => {
		const message = event.data;
		if (message.type === 'probe' && message.id === id) {
			channel.postMessage({ type: 'held', id, probe: message.probe });
		} else if (message.type === 'held' && message.id === id && message.probe === probe) {
			collision = true;
		}
	};
	channel.postMessage({ type: 'probe', id, probe });
	await new Promise((resolve) => window.setTimeout(resolve, 60));
	if (collision) id = crypto.randomUUID();
	sessionStorage.setItem(key, id);
	window.addEventListener('pagehide', () => channel.close(), { once: true });
	return id;
}
