<script lang="ts">
	/* The one model list (21 October): every place that picks a model shows the same thing. A
	 * search, the newest releases (a fixed number, so the list never jumps), older ones folded,
	 * and a way to use an ID the list does not show. The model in use always stays in view. */
	import Check from '@lucide/svelte/icons/check';
	import { filterModels, splitModels, type ModelChoice } from '$lib/model/model-groups';

	let {
		models,
		current = '',
		disabled = false,
		framed = false,
		id = '',
		label = 'Model',
		allowCustom = true,
		onchoose
	}: {
		models: ModelChoice[];
		current?: string;
		disabled?: boolean;
		/** Drawn as a bordered field inside a form, rather than inside a menu. */
		framed?: boolean;
		id?: string;
		label?: string;
		/** Offer the typed text as a model ID when the list does not have it. */
		allowCustom?: boolean;
		onchoose: (model: string) => void;
	} = $props();

	let query = $state('');
	let showOlder = $state(false);
	const filtered = $derived(filterModels(models, query));
	const grouped = $derived(splitModels(filtered, current));
	const shown = $derived(
		query.trim() ? filtered : showOlder ? [...grouped.latest, ...grouped.older] : grouped.latest
	);
	const typed = $derived(query.trim());
</script>

<div class="model-picker" class:framed role="group" aria-label={label}>
	<input
		id={id || undefined}
		class="filter"
		type="search"
		placeholder="Find a model"
		aria-label="Find a model"
		autocomplete="off"
		bind:value={query}
		{disabled}
	/>
	<div class="models" role="listbox" aria-label={label}>
		{#each shown as model (model.id)}
			<button
				type="button"
				role="option"
				{disabled}
				aria-selected={model.id === current}
				onclick={() => onchoose(model.id)}
				><span>{model.name || model.id}</span>{#if model.id === current}<Check
						size={14}
						aria-label="Current"
					/>{/if}</button
			>
		{:else}<p class="none">No listed model matches.</p>{/each}
		{#if allowCustom && typed && !models.some((model) => model.id === typed)}<button
				type="button"
				{disabled}
				title="Use a model ID this provider offers that the list does not show"
				onclick={() => onchoose(typed)}>Use “{typed}”</button
			>{/if}
		{#if !typed && grouped.older.length && !showOlder}<button
				type="button"
				class="older"
				onclick={() => (showOlder = true)}>Older models ({grouped.older.length})</button
			>{/if}
	</div>
</div>

<style>
	.model-picker {
		display: grid;
		gap: 2px;
		min-width: 0;
	}
	.model-picker.framed {
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
	}
	.filter {
		width: calc(100% - 8px);
		margin: 2px 4px 4px;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-label);
	}
	.models {
		display: grid;
		max-height: min(232px, 38dvh);
		overflow-y: auto;
	}
	.models button {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		min-height: 30px;
		padding: 5px 8px;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.models button:hover {
		background: var(--surface-hover);
	}
	.models button:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: -2px;
	}
	.models button[aria-selected='true'] {
		font-weight: 600;
	}
	.models button span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.models button :global(svg) {
		flex: none;
		color: var(--fill-primary);
	}
	.models .older {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.none {
		margin: 6px 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
