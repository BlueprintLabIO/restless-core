<script lang="ts">
	/* How the company reaches the owner away from the cockpit. Email and this device's
	 * notifications are each person's own, in the profile menu; Telegram belongs to the company. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import TelegramChannel from '$lib/components/TelegramChannel.svelte';
	import { Page } from '$lib/ui/page';
	import { page } from '$app/state';

	const companyId = $derived(page.params.companyId ?? '');
</script>

<CompanyTitle title="Notifications" {companyId} />

<Page
	title="Notifications"
	info="How the company reaches you away from the cockpit. Email and desktop notifications are yours alone, in your profile menu."
>
	{#key companyId}<TelegramChannel {companyId} />{/key}
</Page>
