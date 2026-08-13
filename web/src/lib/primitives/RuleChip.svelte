<script lang="ts">
	/* Rule provenance chip: a `.badge.badge-rule` button toggling a `.rule-pop`
	 * popover with the rule's provenance detail (falls back to the code). */

	let { code, detail }: { code: string; detail?: string } = $props();

	let open = $state(false);
</script>

<span class="rule-chip-wrap">
	<button
		type="button"
		class="badge badge-rule"
		onclick={(e) => {
			e.stopPropagation();
			open = !open;
		}}
	>
		{code}
	</button>
	{#if open}
		<span class="rule-pop">
			<span class="rule-pop-quote">{detail ?? code}</span>
		</span>
	{/if}
</span>
