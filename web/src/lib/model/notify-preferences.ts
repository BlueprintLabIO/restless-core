/* How this person hears about what needs them in a company while the cockpit is closed (Sprint 83):
 * email (on until they turn it off, as it happens or once a day at a time of their choosing) and
 * pushes to the browsers they turned desktop notifications on in. Kept on the server, so email
 * holds on every device; a push reaches only the browser that subscribed. */
import { ownerRequest } from './setting.svelte';

export type NotifyPreferences = {
	email: boolean;
	/** `HH:MM` local time for one email a day, or null for email as things happen. */
	digest: string | null;
	time_zone: string | null;
	/** What browsers subscribe with; null where this plane does not push. */
	push_key: string | null;
};

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

export function readNotifyPreferences(company: string): Promise<NotifyPreferences> {
	return ownerRequest<NotifyPreferences>(`${base(company)}/notification-preferences`);
}

/** Change email on or off, or the digest time (`''` for as it happens), in this browser's zone. */
export function saveNotifyPreferences(
	company: string,
	change: { email?: boolean; digest?: string }
): Promise<NotifyPreferences> {
	return ownerRequest<NotifyPreferences>(`${base(company)}/notification-preferences`, {
		method: 'PUT',
		json: { ...change, time_zone: Intl.DateTimeFormat().resolvedOptions().timeZone }
	});
}

/* The push worker answers no page: its scope is a path nothing lives under. */
const PUSH_SCOPE = '/push-worker/';

export function isPushScope(scope: string): boolean {
	return new URL(scope).pathname === PUSH_SCOPE;
}

export function pushSupported(): boolean {
	return (
		typeof navigator !== 'undefined' && 'serviceWorker' in navigator && 'PushManager' in window
	);
}

function keyBytes(key: string): Uint8Array<ArrayBuffer> {
	const raw = atob(key.replace(/-/g, '+').replace(/_/g, '/'));
	const bytes = new Uint8Array(new ArrayBuffer(raw.length));
	for (let index = 0; index < raw.length; index += 1) bytes[index] = raw.charCodeAt(index);
	return bytes;
}

function sameKey(subscription: PushSubscription, key: string): boolean {
	const current = subscription.options.applicationServerKey;
	if (!current) return false;
	const want = keyBytes(key);
	const have = new Uint8Array(current);
	return have.length === want.length && have.every((byte, index) => byte === want[index]);
}

async function activeRegistration(): Promise<ServiceWorkerRegistration> {
	const registration = await navigator.serviceWorker.register('/push-worker.js', {
		scope: PUSH_SCOPE
	});
	if (registration.active) return registration;
	const worker = registration.installing ?? registration.waiting;
	await new Promise<void>((resolve) => {
		if (!worker) return resolve();
		worker.addEventListener('statechange', () => {
			if (worker.state === 'activated') resolve();
		});
	});
	return registration;
}

/** Subscribe this browser for this person in this company; safe to repeat. */
export async function enablePush(company: string, key: string): Promise<void> {
	const registration = await activeRegistration();
	let subscription = await registration.pushManager.getSubscription();
	if (subscription && !sameKey(subscription, key)) {
		await subscription.unsubscribe();
		subscription = null;
	}
	subscription ??= await registration.pushManager.subscribe({
		userVisibleOnly: true,
		applicationServerKey: keyBytes(key)
	});
	await ownerRequest(`${base(company)}/push-subscription`, {
		method: 'PUT',
		json: subscription.toJSON()
	});
}

/** Stop pushes to this browser: for this company on the server, and for every company here. */
export async function disablePush(company: string): Promise<void> {
	const registration = await navigator.serviceWorker.getRegistration(PUSH_SCOPE);
	const subscription = await registration?.pushManager.getSubscription();
	if (!subscription) return;
	await ownerRequest(`${base(company)}/push-subscription`, {
		method: 'DELETE',
		json: { endpoint: subscription.endpoint }
	}).catch(() => undefined);
	await subscription.unsubscribe();
}
