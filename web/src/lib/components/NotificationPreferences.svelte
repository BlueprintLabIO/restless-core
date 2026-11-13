<script lang="ts">
	import { toastWhile } from '$lib/ui/feedback/toast.svelte';
	import { Section, Row, Toggle } from '$lib/ui/page';
	import { notificationSettings } from '$lib/model/notification-settings.svelte';
	import { desktopNotify, BLOCKED_HINT } from '$lib/model/desktop-notify.svelte';
	import EmailDeliveryEditor from './editors/EmailDeliveryEditor.svelte';
	let { companyId }: { companyId: string } = $props();
	const settings = notificationSettings(() => companyId);
	/* A read that failed is a toast with Retry, never a banner that pushes the choices down; the
	 * "!" by the section title brings it back. */
	const readWarning = toastWhile(
		() =>
			!!settings.query.failure && {
				text: 'Your preferences could not be read',
				tone: 'failure',
				dedupeKey: `notification-preferences:${companyId}`,
				action: { label: 'Retry', run: () => settings.query.refresh() }
			}
	);
</script>

<Section
	title="Your notifications"
	info="These choices apply to you. Telegram below is shared by the company."
	warning={readWarning.active
		? { label: 'Your preferences could not be read', show: readWarning.show }
		: null}
>
	<Row
		label="Browser"
		info={desktopNotify.blocked
			? BLOCKED_HINT
			: settings.pushKey
				? 'Notify this device when something needs you, including with the cockpit closed.'
				: 'Notify this device while the cockpit is open in the background.'}
	>
		<Toggle
			label="Browser notifications"
			checked={desktopNotify.active}
			disabled={settings.busy || desktopNotify.permission === 'unsupported'}
			onchange={() => settings.toggleDesktop()}
		/>
		{#if desktopNotify.permission === 'unsupported'}<span class="quiet"
				>Unavailable in this browser</span
			>{:else if desktopNotify.blocked}<span class="quiet">Blocked in browser settings</span>{/if}
	</Row>
	<EmailDeliveryEditor {companyId} />
	{#if typeof settings.preferences?.checkins === 'boolean'}
		<Row
			label="Check-ins"
			info="About once an hour while you are around, Exec tells you in a few lines what landed and what is next, and asks what it needs from you. If you do not answer it reminds you once, three hours later, then waits for you."
		>
			<Toggle
				label="Check-ins from Exec"
				checked={settings.preferences.checkins}
				disabled={settings.busy}
				onchange={(on) => settings.save({ checkins: on })}
			/>
		</Row>
	{/if}
</Section>

<style>
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
