<script lang="ts">
	/* A degraded state (stale data, a refresh that failed) is said once as a toast. This fixed-size
	 * slot keeps it findable after the toast is dismissed: a "!" that says it again. The slot is
	 * always reserved, so the mark appearing never moves anything beside it. */
	let { warning }: { warning: { label: string; show: () => void } | null | undefined } = $props();
</script>

<span class="warning-slot">
	{#if warning}<button
			class="warning-mark"
			type="button"
			aria-label={warning.label}
			title={warning.label}
			onclick={() => warning?.show()}>!</button
		>{/if}
</span>

<style>
	.warning-slot {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
	}
	.warning-mark {
		width: 22px;
		height: 22px;
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: 50%;
		background: var(--surface-raised);
		color: var(--state-danger);
		font-weight: 700;
		line-height: 1;
		cursor: pointer;
	}
	.warning-mark:hover {
		background: var(--wash-hover);
	}
</style>
