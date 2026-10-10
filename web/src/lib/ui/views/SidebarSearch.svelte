<script lang="ts">
	/* The search field under a sidebar's title: a real control, focused by "/" from anywhere in the
	 * area unless the owner is already typing. */
	import Search from '@lucide/svelte/icons/search';

	let {
		value = $bindable(''),
		placeholder,
		label = placeholder
	}: {
		value?: string;
		placeholder: string;
		/** What is searched, for assistive technology. */
		label?: string;
	} = $props();

	let input = $state<HTMLInputElement | null>(null);

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== '/' || event.metaKey || event.ctrlKey || event.altKey) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
		event.preventDefault();
		input?.focus();
	}
</script>

<svelte:window {onkeydown} />

<label class="sidebar-search">
	<Search size={16} strokeWidth={1.75} aria-hidden="true" />
	<input
		bind:this={input}
		bind:value
		type="search"
		{placeholder}
		aria-label={label}
		onkeydown={(event) => {
			if (event.key === 'Escape') {
				value = '';
				input?.blur();
			}
		}}
	/>
	{#if !value}<kbd aria-hidden="true">/</kbd>{/if}
</label>

<style>
	.sidebar-search {
		display: flex;
		min-width: 0;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 10px;
		border: 1px solid var(--edge-control);
		border-radius: var(--radius-pane);
		background: color-mix(in srgb, var(--surface-raised) 72%, transparent);
		color: var(--text-tertiary);
		cursor: text;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background var(--motion-state) var(--ease-standard);
	}
	.sidebar-search:hover {
		border-color: var(--edge-control-hover);
	}
	.sidebar-search:focus-within {
		border-color: var(--intent-conversation);
		background: var(--surface-raised);
	}
	input {
		flex: 1 1 auto;
		width: 0;
		min-width: 0;
		height: 100%;
		padding: 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
	}
	/* The field's own border shows focus; the global ring on the input would draw a second box. */
	input:focus,
	input:focus-visible {
		outline: none;
		box-shadow: none;
	}
	input::placeholder {
		color: var(--text-tertiary);
	}
	input::-webkit-search-cancel-button {
		display: none;
	}
	kbd {
		flex: none;
		padding: 0 5px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		font: inherit;
		font-size: var(--t-label);
		line-height: 16px;
	}
</style>
