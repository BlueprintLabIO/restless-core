<script lang="ts">
	import { Section, Row, Toggle, Notice } from '$lib/ui/page';
	import { notificationSettings } from '$lib/model/notification-settings.svelte';
	import { desktopNotify, BLOCKED_HINT } from '$lib/model/desktop-notify.svelte';
	let { companyId }: { companyId: string } = $props();
	const settings = notificationSettings(() => companyId);
	const prefs = $derived(settings.preferences);
</script>

<Section
	title="Your notifications"
	info="These choices apply to you. Telegram below is shared by the company."
>
	{#if settings.query.failure}<Notice tone="warning" title="Your preferences could not be read">
			{#snippet actions()}<button class="btn small" onclick={() => settings.query.refresh()}
					>Retry</button
				>{/snippet}
		</Notice>{/if}
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
	<Row
		label="Email"
		info="Email you when something needs you and you have not seen it here. Your choice applies on every device."
	>
		{#if settings.emailDelivered === null}<span class="quiet" role="status"
				>Checking email availability…</span
			>
		{:else if settings.emailDelivered}<Toggle
				label="Email notifications"
				checked={!!prefs?.email}
				disabled={!prefs || settings.busy}
				onchange={(email) => settings.save({ email })}
			/>
		{:else}<span class="quiet">Email delivery is available on the hosted app.</span>{/if}
	</Row>
	{#if settings.emailDelivered && prefs?.email}
		<Row label="Delivery">
			<select
				aria-label="Email delivery"
				value={prefs.digest ? 'digest' : 'immediate'}
				disabled={settings.busy}
				onchange={(event) =>
					settings.save({ digest: event.currentTarget.value === 'digest' ? '08:00' : '' })}
			>
				<option value="immediate">As things happen</option><option value="digest"
					>Daily digest</option
				>
			</select>
			{#if prefs.digest}<input
					aria-label="Daily digest time"
					title="In your time zone"
					type="time"
					value={prefs.digest}
					disabled={settings.busy}
					onchange={(event) =>
						event.currentTarget.value && settings.save({ digest: event.currentTarget.value })}
				/>{/if}
		</Row>
	{/if}
</Section>

<style>
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
