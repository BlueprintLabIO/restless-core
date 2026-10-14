import { getApplianceStatus } from './appliance';
import { isPushScope } from './notify-preferences';

/* The cockpit's service worker keeps a local appliance usable through a flaky connection. On Cloud
 * every page shares one address with Fleet (ADR 0007), so a cockpit worker would answer Fleet's
 * pages too: a hosted plane registers none and removes any it finds, except the push worker, which
 * answers no page. Unknown is not local. */
export async function registerServiceWorkerWhereLocal(): Promise<void> {
	if (!('serviceWorker' in navigator)) return;
	let hosted: boolean;
	try {
		hosted = Boolean((await getApplianceStatus()).hosted);
	} catch {
		return;
	}
	if (hosted) {
		for (const registration of await navigator.serviceWorker.getRegistrations()) {
			if (!isPushScope(registration.scope)) await registration.unregister();
		}
		return;
	}
	await navigator.serviceWorker.register('/service-worker.js');
}
