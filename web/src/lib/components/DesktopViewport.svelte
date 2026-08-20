<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		src = '',
		title = 'Company computer',
		offline = null,
		onload = null
	}: {
		src?: string;
		title?: string;
		offline?: Snippet | null;
		onload?: (() => void) | null;
	} = $props();
</script>

<div class="desktop-viewport">
	{#if src}
		<iframe
			{title}
			{src}
			allow="clipboard-read; clipboard-write"
			referrerpolicy="same-origin"
			onload={() => onload?.()}
		></iframe>
	{:else if offline}
		{@render offline()}
	{:else}
		<div class="desktop-viewport-empty" role="status">No live desktop is attached.</div>
	{/if}
</div>

<style>
	.desktop-viewport {
		position: relative;
		min-width: 0;
		min-height: 0;
		width: 100%;
		height: 100%;
		flex: 1 1 auto;
		display: flex;
		overflow: hidden;
		background: #101217;
	}

	.desktop-viewport iframe {
		display: block;
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		flex: 1 1 auto;
		border: 0;
		background: #101217;
	}

	.desktop-viewport-empty {
		margin: auto;
		color: rgba(255, 255, 255, 0.72);
	}
</style>
