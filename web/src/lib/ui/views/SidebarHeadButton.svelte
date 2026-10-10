<script lang="ts">
	/* The area's main action beside a sidebar title: a real, raised button with its purpose in the
	 * tooltip. */
	import type { Component } from 'svelte';

	let {
		label,
		icon: Icon,
		href,
		onclick
	}: {
		/** What it does, as the tooltip and accessible name. */
		label: string;
		icon: Component<{ size?: number; strokeWidth?: number }>;
		href?: string;
		onclick?: () => void;
	} = $props();
</script>

{#if href}<a class="head-button" {href} title={label} aria-label={label}
		><Icon size={16} strokeWidth={1.9} /></a
	>{:else}<button class="head-button" type="button" title={label} aria-label={label} {onclick}
		><Icon size={16} strokeWidth={1.9} /></button
	>{/if}

<style>
	.head-button {
		display: inline-grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 1px solid var(--edge-control);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: var(--control-depth), var(--bevel);
		color: var(--ink);
		cursor: pointer;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-press) var(--ease-standard);
	}
	.head-button:hover {
		border-color: var(--edge-control-hover);
	}
	.head-button:active {
		box-shadow: var(--control-depth-pressed);
	}
	.head-button:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 1px;
	}
</style>
