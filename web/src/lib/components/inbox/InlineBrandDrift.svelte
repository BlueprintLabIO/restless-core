<script lang="ts">
	/* A brand change made an artifact stale; its fate is chosen in the card (S101) with the same
	 * editor as Company › Brand: revise, keep or retire, with an optional reason for the record. */
	import InlinePanel from './InlinePanel.svelte';
	import DriftDecisionEditor from '../editors/DriftDecisionEditor.svelte';

	let {
		companyId,
		finding,
		owner,
		ondone
	}: {
		companyId: string;
		finding: string;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();
</script>

{#if owner}
	<InlinePanel label="What happens to this artifact" form>
		<DriftDecisionEditor {companyId} {finding} onsaved={ondone} />
	</InlinePanel>
{:else}
	<InlinePanel
		label="Brand decision"
		note="The owner decides what happens to brand-affected work."
	/>
{/if}
