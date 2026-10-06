<script lang="ts">
	/* A named run of sidebar rows. The name is quiet: it groups, it does not shout. */
	import type { Snippet } from 'svelte';

	let { label, children }: { label: string; children: Snippet } = $props();
</script>

<div class="sidebar-group" role="group" aria-label={label}>
	<span class="sidebar-label">{label}</span>
	{@render children()}
</div>

<style>
	.sidebar-group {
		display: grid;
		gap: 1px;
		margin-top: 12px;
	}
	.sidebar-label {
		padding: 0 8px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
</style>
