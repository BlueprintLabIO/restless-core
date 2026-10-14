<script lang="ts">
	/* How the company reaches the owner away from the cockpit. Email and this device's
	 * notifications are each person's own, in the profile menu; Telegram belongs to the company. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import TelegramChannel from '$lib/components/TelegramChannel.svelte';
	import AwayMode from '$lib/components/AwayMode.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { Page } from '$lib/ui/page';
	import { page } from '$app/state';

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
</script>

<CompanyTitle title="Notifications" {companyId} />

<Page
	title="Notifications"
	info="How the company reaches you away from the cockpit. Email and desktop notifications are yours alone, in your profile menu."
>
	{#key companyId}<AwayMode {companyId} {owner} /><TelegramChannel {companyId} />{/key}
</Page>
