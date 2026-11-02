<script lang="ts">
	/* The one way to get more capacity. On a hosted plane that is the account's plan and AI credit,
	 * priced by the billing service itself; Core never shows a price it does not hold. A local
	 * appliance runs on this computer, so its capacity is the computer's. */
	import { Row } from '$lib/ui/page';
	import { planeStatus, type ApplianceStatus } from '$lib/model/appliance';

	let status = $state<ApplianceStatus | null | undefined>(undefined);
	void planeStatus().then((value) => (status = value));
</script>

<Row
	label="More capacity"
	info={status?.hosted
		? 'Your plan sets the account’s compute and the AI credit it includes. Plans, top-ups and their prices are on the billing page, from the payment provider.'
		: 'This Restless runs on your own computer, so its capacity is that computer’s CPU and memory. Give Docker more of either in its settings, then the account pool follows.'}
>
	{#if status?.hosted}<a class="btn small" href="/account/settings/billing">Plans and credit</a>
	{:else if status}<span>Set by this computer</span>
	{:else}<span aria-hidden="true">&nbsp;</span>{/if}
</Row>
