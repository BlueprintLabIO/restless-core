<script lang="ts">
	/* The repairs a banner names, as buttons rather than terminal commands. Each asks first: it
	 * changes the appliance itself. */
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import {
		APPLIANCE_OPERATIONS,
		runApplianceOperation,
		type ApplianceOperation
	} from '$lib/model/appliance';

	let {
		actions,
		ondone
	}: { actions: ApplianceOperation[]; ondone?: (op: ApplianceOperation) => void } = $props();
	let busy = $state(false);

	async function run(op: ApplianceOperation) {
		const meaning = APPLIANCE_OPERATIONS[op];
		if (
			busy ||
			!(await confirmAction({ title: meaning.title, body: meaning.body, action: meaning.label }))
		)
			return;
		busy = true;
		try {
			await runApplianceOperation(op);
			toast(meaning.done);
			ondone?.(op);
		} catch (error) {
			toast(failureSentence(error, 'That did not start. Try again.'), { tone: 'failure' });
		} finally {
			busy = false;
		}
	}
</script>

{#if actions.length}
	<span class="repair">
		{#each actions as op (op)}<button
				class="btn small"
				type="button"
				disabled={busy}
				onclick={() => void run(op)}>{APPLIANCE_OPERATIONS[op].label}…</button
			>{/each}
		<a class="btn small ghost" href="/account/settings/this-computer">Details</a>
	</span>
{/if}

<style>
	.repair {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
</style>
