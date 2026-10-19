<script lang="ts">
	import { Section, Row, Toggle, Notice } from '$lib/ui/page';
	import { notificationSettings } from '$lib/model/notification-settings.svelte';
	import { desktopNotify, BLOCKED_HINT } from '$lib/model/desktop-notify.svelte';
	import EmailDeliveryEditor from './editors/EmailDeliveryEditor.svelte';
	let { companyId }: { companyId: string } = $props();
	const settings = notificationSettings(() => companyId);
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
	<EmailDeliveryEditor {companyId} />
</Section>

<style>
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
