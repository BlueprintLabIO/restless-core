import { onMount } from 'svelte';
import { settingQuery } from './setting.svelte';
import { desktopNotify } from './desktop-notify.svelte';
import { planeStatus } from './appliance';
import { isLocalHost } from './reenter';
import {
	disablePush,
	enablePush,
	pushSupported,
	readNotifyPreferences,
	saveNotifyPreferences
} from './notify-preferences';
import { failureSentence } from './failure';
import { toast } from '$lib/ui/feedback/toast.svelte';

/** Profile shortcuts and Notifications share the server's saved preferences. */
export function notificationSettings(companyId: () => string, restorePush = false) {
	const query = settingQuery({
		key: () => ['notification-preferences', companyId()],
		read: () => readNotifyPreferences(companyId()),
		enabled: () => !!companyId(),
		poll: false
	});
	let busy = $state(false);
	let emailDelivered = $state<boolean | null>(null);
	onMount(() => {
		desktopNotify.load();
		if (isLocalHost(location.hostname)) emailDelivered = false;
		else
			void planeStatus().then((status) => {
				emailDelivered = status ? Boolean(status.hosted) : true;
			});
	});
	const pushKey = $derived(pushSupported() ? (query.view?.push_key ?? null) : null);
	$effect(() => {
		if (restorePush && desktopNotify.active && pushKey && companyId())
			void enablePush(companyId(), pushKey).catch(() => undefined);
	});
	async function save(change: { email?: boolean; digest?: string }) {
		if (busy || !query.view) return;
		const company = companyId();
		busy = true;
		try {
			const saved = await saveNotifyPreferences(company, change);
			if (company === companyId()) query.accept(saved);
		} catch (cause) {
			toast(failureSentence(cause, 'Your notification choice was not saved.'), { tone: 'failure' });
		} finally {
			busy = false;
		}
	}
	async function toggleDesktop() {
		if (busy) return;
		const company = companyId(),
			key = pushKey;
		busy = true;
		try {
			await desktopNotify.toggle();
			if (!key || !company) return;
			if (desktopNotify.active) await enablePush(company, key);
			else await disablePush(company);
		} catch (cause) {
			toast(failureSentence(cause, 'Closed-browser notifications could not be updated.'), {
				tone: 'failure'
			});
		} finally {
			busy = false;
		}
	}
	return {
		query,
		save,
		toggleDesktop,
		get preferences() {
			return query.view;
		},
		get busy() {
			return busy;
		},
		get emailDelivered() {
			return emailDelivered;
		},
		get pushKey() {
			return pushKey;
		}
	};
}
