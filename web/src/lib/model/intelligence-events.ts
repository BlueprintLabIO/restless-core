const CHANNEL = 'restless-intelligence-changed';
const ALL_COMPANIES = '*';
const TAB_ID =
	typeof crypto !== 'undefined' && 'randomUUID' in crypto
		? crypto.randomUUID()
		: String(Math.random());

function validCompany(value: unknown): value is string {
	return typeof value === 'string' && value.length > 0 && value.length <= 120;
}

/* When this tab last changed each company's route, so its reads follow the switch closely. */
const changedAt = new Map<string, number>();

/** Whether this tab changed the company's route in the last minute. */
export function changedRecently(company: string, within = 60_000) {
	const at = Math.max(changedAt.get(company) ?? 0, changedAt.get(ALL_COMPANIES) ?? 0);
	return Date.now() - at < within;
}

/** A grant, sign-in, or assignment changed the route seen by open cockpits. */
export function announceIntelligenceChange(company = ALL_COMPANIES) {
	if (typeof window === 'undefined' || !validCompany(company)) return;
	changedAt.set(company, Date.now());
	window.dispatchEvent(new CustomEvent(CHANNEL, { detail: company }));
	if (typeof BroadcastChannel !== 'undefined') {
		const channel = new BroadcastChannel(CHANNEL);
		channel.postMessage({ company, sender: TAB_ID });
		channel.close();
	}
}

export function watchIntelligenceChanges(onChange: (company: string) => void, remoteOnly = false) {
	if (typeof window === 'undefined') return () => {};
	const received = (value: unknown) => {
		if (validCompany(value)) onChange(value);
	};
	const local = (event: Event) => received((event as CustomEvent).detail);
	if (!remoteOnly) window.addEventListener(CHANNEL, local);
	const channel = typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel(CHANNEL);
	if (channel)
		channel.onmessage = (event) => {
			if (event.data?.sender !== TAB_ID) received(event.data?.company);
		};
	return () => {
		if (!remoteOnly) window.removeEventListener(CHANNEL, local);
		channel?.close();
	};
}

export function affectsCompany(changed: string, company: string) {
	return changed === ALL_COMPANIES || changed === company;
}
