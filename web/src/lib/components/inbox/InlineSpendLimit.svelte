<script lang="ts">
	/* The spend limit, raised in its card (S101) with the same meter and editor as Company › Limits:
	 * where the budget stands on top, the limit below with the usual raises one tap away. The
	 * footer saves it, which answers the warning or restarts work. */
	import { companyQuery } from '$lib/model/queries.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import SpendMeter from '../editors/SpendMeter.svelte';
	import SpendLimitEditor from '../editors/SpendLimitEditor.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		owner,
		ondone
	}: { companyId: string; owner: boolean; ondone: (receipt: string) => void } = $props();

	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => source.attach());
	const spend = $derived(source.view?.limits.spend ?? null);

	cardActions(() =>
		owner && !spend && source.failure
			? { primary: { label: 'Try again', busyLabel: 'Reading…', run: () => source.refresh() } }
			: {}
	);
</script>

{#if !owner}
	<InlinePanel label="Spend limit" note="The owner sets the spend limit." />
{:else if spend}
	<InlinePanel label="Spend limit" form>
		{#snippet status()}<SpendMeter {spend} />{/snippet}
		<SpendLimitEditor
			{companyId}
			{spend}
			proposed={Math.ceil((Math.max(spend.ceiling_usd, 5) * 2) / 10) * 10}
			onsaved={(company, receipt) => {
				source.accept(company);
				ondone(`${receipt} Work carries on.`);
			}}
		/>
	</InlinePanel>
{:else if source.failure}
	<InlinePanel label="Spend limit" note="The spend limit could not be read." />
{:else}
	<InlinePanel label="Spend limit" note="Reading the spend limit…" />
{/if}
