<script lang="ts">
	import type { Snippet } from 'svelte';
	import Popover from './Popover.svelte';
	let {
		label = 'More actions',
		trigger,
		children: items
	}: { label?: string; trigger?: Snippet; children: Snippet } = $props();
</script>

<Popover {label} {trigger} mode="action">
	{#snippet children(_close)}{@render items()}{/snippet}
</Popover>
